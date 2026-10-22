//! The engine, behind a trait.
//!
//! The app never talks to the career-ops engine directly; it talks to the Node
//! bridge through this trait. The trait exists so a screen can be rendered
//! headlessly in a test against a canned engine, with no bridge process and no
//! data root.

use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::bridge::{Bridge, BridgeError, StreamEvent};
use crate::model::RunEvent;

/// What a screen needs from the career-ops engine.
pub trait Backend: Send + Sync {
    /// Call one operation and get the raw answer.
    fn call(&self, op: &str, params: Value) -> Result<Value, BridgeError>;

    /// Start a streaming run. The events arrive through `poll_events`.
    fn start_run(
        &self,
        kind: &str,
        input: &str,
        cli_id: &str,
        brief: Option<&str>,
    ) -> Result<u64, BridgeError>;

    /// Take the events queued since the last call.
    fn poll_events(&self) -> Vec<StreamEvent>;
}

/// Call one operation and decode the answer.
///
/// A free function rather than a trait method: a generic method would make the
/// trait not object-safe, and the app holds its backend behind `Arc<dyn
/// Backend>`.
pub fn call_typed<B, T>(backend: &B, op: &str, params: Value) -> Result<T, BridgeError>
where
    B: Backend + ?Sized,
    T: DeserializeOwned,
{
    let raw = backend.call(op, params)?;
    serde_json::from_value(raw).map_err(|e| BridgeError {
        code: "bad-payload".into(),
        message: format!("the career-ops engine sent something unreadable: {e}"),
    })
}

/// The real backend: the Node bridge.
pub struct BridgeBackend {
    bridge: std::sync::Arc<Bridge>,
}

impl BridgeBackend {
    pub fn new(bridge: std::sync::Arc<Bridge>) -> Self {
        BridgeBackend { bridge }
    }
}

impl Backend for BridgeBackend {
    fn call(&self, op: &str, params: Value) -> Result<Value, BridgeError> {
        self.bridge.call(op, params)
    }

    fn start_run(
        &self,
        kind: &str,
        input: &str,
        cli_id: &str,
        brief: Option<&str>,
    ) -> Result<u64, BridgeError> {
        let params = json!({
            "kind": kind,
            "input": input,
            "cliId": cli_id,
            "brief": brief,
        });
        self.bridge.start_stream("run", params)
    }

    fn poll_events(&self) -> Vec<StreamEvent> {
        self.bridge.poll_events()
    }
}

/// A backend that answers from a script, for headless screen tests.
///
/// The answers are the shapes the real bridge sends, so a screen that renders
/// correctly here renders correctly against the engine.
pub struct FakeBackend {
    answers: std::collections::HashMap<String, Value>,
    pub calls: std::sync::Mutex<Vec<(String, Value)>>,
    pub events: std::sync::Mutex<Vec<StreamEvent>>,
}

impl FakeBackend {
    pub fn new() -> Self {
        FakeBackend {
            answers: std::collections::HashMap::new(),
            calls: std::sync::Mutex::new(Vec::new()),
            events: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Register the answer for one op.
    pub fn with(mut self, op: &str, value: Value) -> Self {
        self.answers.insert(op.to_string(), value);
        self
    }

    /// Queue an event the next `poll_events` will return.
    pub fn push_event(&self, event: RunEvent) {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(StreamEvent {
                request_id: 1,
                event,
            });
    }
}

impl Default for FakeBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for FakeBackend {
    fn call(&self, op: &str, params: Value) -> Result<Value, BridgeError> {
        self.calls
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((op.to_string(), params));
        self.answers.get(op).cloned().ok_or_else(|| BridgeError {
            code: "no-answer".into(),
            message: format!("the fake engine has no answer for {op}"),
        })
    }

    fn start_run(
        &self,
        kind: &str,
        input: &str,
        cli_id: &str,
        brief: Option<&str>,
    ) -> Result<u64, BridgeError> {
        self.calls
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((
                "run".to_string(),
                json!({ "kind": kind, "input": input, "cliId": cli_id, "brief": brief }),
            ));
        Ok(1)
    }

    fn poll_events(&self) -> Vec<StreamEvent> {
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *events)
    }
}
