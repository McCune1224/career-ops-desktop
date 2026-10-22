//! The client for the Node bridge.
//!
//! The bridge is a child process: `node <checkout>/desktop/bridge/server.mjs`.
//! It speaks newline-delimited JSON on stdio. This module owns the process,
//! the request/response matching, and the event queue.
//!
//! No socket is opened and no port is bound, in either direction: the app
//! reaches the engine only through the pipes of a process it started.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::code_root;
use crate::model::RunEvent;

/// A failure the bridge reported, or the bridge could not be reached.
#[derive(Debug, Clone)]
pub struct BridgeError {
    /// A stable code: "busy", "not-found", "bad-request", …
    pub code: String,
    /// The message, safe to show on screen.
    pub message: String,
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for BridgeError {}

/// An event of a running job, tagged with the request it belongs to.
#[derive(Debug, Clone)]
pub struct StreamEvent {
    pub request_id: u64,
    pub event: RunEvent,
}

/// Where the app finds the two trees it needs.
///
/// They are different trees in different repositories: this repository holds
/// the bridge, and the career-ops checkout holds the engine (the root scripts
/// and `web/src/lib`). Neither is inferred from the other's shape.
#[derive(Debug, Clone)]
pub struct BridgePaths {
    /// This repository's root: the directory holding `bridge/`.
    pub client_root: PathBuf,
    /// The career-ops checkout that holds the engine.
    pub code_root: PathBuf,
    /// The bridge entry point.
    pub server: PathBuf,
    /// The node binary to run it with.
    pub node: String,
}

impl BridgePaths {
    /// Locate both trees.
    ///
    /// The client root is found from the running executable, then the working
    /// directory, then the directory the build happened in — a release build
    /// usually lives outside the checkout, so the path is resolved in that
    /// order and the first hit wins.
    ///
    /// The engine checkout is NAMED, never inferred: `CAREER_OPS_CODE_ROOT`
    /// wins, then the value this app remembered in its own settings, then a
    /// sibling checkout of the same parent directory, and otherwise one
    /// actionable sentence. See `code_root.rs`.
    ///
    /// `remembered` is the code root this app stored on its last run. It is
    /// what makes the choice survive a restart.
    pub fn discover(remembered: Option<&str>) -> Result<BridgePaths, BridgeError> {
        let node = std::env::var("CAREER_OPS_NODE").unwrap_or_else(|_| "node".to_string());
        let client_root = find_client_root().ok_or_else(|| BridgeError {
            code: "client-root-missing".into(),
            message: format!(
                "could not find this client's own bridge directory; run the app from its checkout or set CAREER_OPS_CODE_ROOT"
            ),
        })?;
        let server = client_root.join("bridge").join("server.mjs");

        let explicit = std::env::var(code_root::CODE_ROOT_ENV)
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| remembered.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()));

        let resolved = code_root::resolve(explicit.as_deref(), &client_root).map_err(|message| {
            BridgeError {
                code: "code-root-missing".into(),
                message,
            }
        })?;

        Ok(BridgePaths {
            client_root,
            code_root: resolved.path,
            server,
            node,
        })
    }
}

/// This repository's root: the directory holding `bridge/server.mjs`.
fn find_client_root() -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        while let Some(d) = dir {
            roots.push(d.clone());
            dir = d.parent().map(Path::to_path_buf);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = Some(cwd);
        while let Some(d) = dir {
            roots.push(d.clone());
            dir = d.parent().map(Path::to_path_buf);
        }
    }
    roots.push(code_root::client_root());
    for root in roots {
        if root.join("bridge").join("server.mjs").is_file() {
            return Some(root);
        }
    }
    None
}

/// The live connection to the bridge.
pub struct Bridge {
    child: Mutex<Option<Child>>,
    stdin: Mutex<Option<ChildStdin>>,
    pending: Arc<Mutex<HashMap<u64, Sender<Result<Value, BridgeError>>>>>,
    events: Arc<Mutex<Vec<StreamEvent>>>,
    next_id: AtomicU64,
    paths: BridgePaths,
}

impl Bridge {
    /// Start the bridge process.
    ///
    /// `data_root` is the career-ops data root (CAREER_OPS_ROOT). When it is
    /// None the bridge resolves it the way the core does: the marker file, or
    /// the checkout itself.
    pub fn start(paths: BridgePaths, data_root: Option<&Path>) -> Result<Arc<Bridge>, BridgeError> {
        let mut command = Command::new(&paths.node);
        command
            .arg(&paths.server)
            .current_dir(&paths.code_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        command.env("CAREER_OPS_CODE_ROOT", &paths.code_root);
        if let Some(root) = data_root {
            command.env("CAREER_OPS_ROOT", root);
        }

        let mut child = command.spawn().map_err(|e| BridgeError {
            code: "spawn-failed".into(),
            message: format!("could not start the career-ops bridge: {e}"),
        })?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().ok_or_else(|| BridgeError {
            code: "spawn-failed".into(),
            message: "the career-ops bridge produced no output channel".into(),
        })?;

        let pending: Arc<Mutex<HashMap<u64, Sender<Result<Value, BridgeError>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let events: Arc<Mutex<Vec<StreamEvent>>> = Arc::new(Mutex::new(Vec::new()));

        let reader_pending = Arc::clone(&pending);
        let reader_events = Arc::clone(&events);
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if line.trim().is_empty() {
                    continue;
                }
                let Ok(message) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                let id = message.get("id").and_then(Value::as_u64).unwrap_or(0);
                if let Some(event) = message.get("event") {
                    if let Ok(parsed) = serde_json::from_value::<RunEvent>(event.clone()) {
                        reader_events
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .push(StreamEvent {
                                request_id: id,
                                event: parsed,
                            });
                    }
                    continue;
                }
                let outcome = if message.get("ok").and_then(Value::as_bool) == Some(true) {
                    Ok(message.get("data").cloned().unwrap_or(Value::Null))
                } else {
                    let error = message.get("error");
                    Err(BridgeError {
                        code: error
                            .and_then(|e| e.get("code"))
                            .and_then(Value::as_str)
                            .unwrap_or("failed")
                            .to_string(),
                        message: error
                            .and_then(|e| e.get("message"))
                            .and_then(Value::as_str)
                            .unwrap_or("the career-ops bridge reported a failure")
                            .to_string(),
                    })
                };
                let mut waiters = reader_pending.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(tx) = waiters.get(&id) {
                    let _ = tx.send(outcome);
                    waiters.remove(&id);
                }
            }
            // The bridge died: fail every waiting request rather than leaving
            // the screen waiting forever.
            let mut waiters = reader_pending.lock().unwrap_or_else(|e| e.into_inner());
            for (_, tx) in waiters.drain() {
                let _ = tx.send(Err(BridgeError {
                    code: "bridge-gone".into(),
                    message: "the career-ops bridge stopped running".into(),
                }));
            }
        });

        Ok(Arc::new(Bridge {
            child: Mutex::new(Some(child)),
            stdin: Mutex::new(stdin),
            pending,
            events,
            next_id: AtomicU64::new(1),
            paths,
        }))
    }

    /// The checkout this bridge is running from.
    pub fn code_root(&self) -> &Path {
        &self.paths.code_root
    }

    /// Call one bridge operation and wait for its answer.
    pub fn call(&self, op: &str, params: Value) -> Result<Value, BridgeError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx): (Sender<Result<Value, BridgeError>>, Receiver<Result<Value, BridgeError>>) =
            channel();
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, tx);
        self.send(id, op, params)?;
        rx.recv().unwrap_or_else(|_| {
            Err(BridgeError {
                code: "bridge-gone".into(),
                message: "the career-ops bridge stopped running".into(),
            })
        })
    }

    /// Call one bridge operation and decode the answer.
    pub fn call_typed<T: serde::de::DeserializeOwned>(
        &self,
        op: &str,
        params: Value,
    ) -> Result<T, BridgeError> {
        let raw = self.call(op, params)?;
        serde_json::from_value(raw).map_err(|e| BridgeError {
            code: "bad-payload".into(),
            message: format!("the career-ops bridge sent something unreadable: {e}"),
        })
    }

    /// Start a streaming operation. The events arrive through `poll_events`.
    pub fn start_stream(&self, op: &str, params: Value) -> Result<u64, BridgeError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.send(id, op, params)?;
        Ok(id)
    }

    /// Take every event queued since the last call.
    pub fn poll_events(&self) -> Vec<StreamEvent> {
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *events)
    }

    fn send(&self, id: u64, op: &str, params: Value) -> Result<(), BridgeError> {
        let request = json!({ "id": id, "op": op, "params": params });
        let mut guard = self.stdin.lock().unwrap_or_else(|e| e.into_inner());
        let stdin = guard.as_mut().ok_or_else(|| BridgeError {
            code: "bridge-gone".into(),
            message: "the career-ops bridge is no longer accepting requests".into(),
        })?;
        let mut line = serde_json::to_string(&request).unwrap_or_default();
        line.push('\n');
        stdin.write_all(line.as_bytes()).map_err(|e| BridgeError {
            code: "bridge-gone".into(),
            message: format!("could not reach the career-ops bridge: {e}"),
        })?;
        stdin.flush().map_err(|e| BridgeError {
            code: "bridge-gone".into(),
            message: format!("could not reach the career-ops bridge: {e}"),
        })
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
