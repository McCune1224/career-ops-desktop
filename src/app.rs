//! The app's state and navigation.

use std::sync::Arc;

use serde_json::json;

use crate::backend::{call_typed, Backend};
use crate::bridge::BridgeError;
use crate::model::{Bootstrap, Config, Cv, Followups, Pipeline, Report, RunEvent};
use crate::settings::Settings;

/// Where the app is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Today,
    Pipeline,
    Report(String),
    Run,
    Cv,
    Followups,
    Config,
}

/// What the app knows, loaded from the engine.
#[derive(Debug, Default)]
pub struct AppState {
    pub bootstrap: Option<Bootstrap>,
    pub pipeline: Option<Pipeline>,
    pub report: Option<Report>,
    pub cv: Option<Cv>,
    pub followups: Option<Followups>,
    pub config: Option<Config>,
    /// The last message for the user, and whether it is an error.
    pub message: Option<(String, bool)>,
    /// True while a request is in flight.
    pub busy: bool,
}

impl AppState {
    pub fn say(&mut self, text: impl Into<String>) {
        self.message = Some((text.into(), false));
    }

    pub fn fail(&mut self, text: impl Into<String>) {
        self.message = Some((text.into(), true));
    }
}

/// The sort keys the tracker table offers, matching the web app's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Tracker,
    Company,
    Role,
    Score,
    Status,
    Date,
}

impl SortKey {
    pub fn label(&self) -> &'static str {
        match self {
            SortKey::Tracker => "#",
            SortKey::Company => "Company",
            SortKey::Role => "Role",
            SortKey::Score => "Score",
            SortKey::Status => "Status",
            SortKey::Date => "Date",
        }
    }
}

/// The tracker table's sort and filter state.
#[derive(Debug, Clone)]
pub struct PipelineView {
    pub sort: SortKey,
    pub descending: bool,
    /// A canonical status, or "All".
    pub status_filter: String,
    pub min_score: f32,
    pub search: String,
}

impl Default for PipelineView {
    fn default() -> Self {
        PipelineView {
            sort: SortKey::Tracker,
            descending: false,
            status_filter: "All".to_string(),
            min_score: 0.0,
            search: String::new(),
        }
    }
}

/// The kinds of run the run screen can start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunKind {
    Evaluate,
    Pdf,
    Cover,
}

impl RunKind {
    pub fn id(&self) -> &'static str {
        match self {
            RunKind::Evaluate => "evaluate",
            RunKind::Pdf => "pdf",
            RunKind::Cover => "cover",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            RunKind::Evaluate => "Evaluate a posting",
            RunKind::Pdf => "Tailor the CV (PDF)",
            RunKind::Cover => "Draft a cover letter",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            RunKind::Evaluate => "Paste a job URL or the posting text. The agent scores it and the report is saved.",
            RunKind::Pdf => "Paste the report number or the company. The agent tailors your CV and it is rendered to PDF.",
            RunKind::Cover => "Paste the report number or the company, plus a short brief of what to say.",
        }
    }
}

/// One line of a run's log.
#[derive(Debug, Clone)]
pub struct LogLine {
    pub text: String,
    pub kind: LogKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Text,
    Tool,
    Status,
    Error,
    Done,
}

/// The run screen's state.
#[derive(Debug, Clone, Default)]
pub struct RunState {
    pub kind: Option<RunKind>,
    pub input: String,
    pub brief: String,
    pub log: Vec<LogLine>,
    pub running: bool,
    pub finished: bool,
    pub error: Option<String>,
    pub report_n: Option<String>,
    pub request_id: Option<u64>,
    /// When the run started, on egui's clock.
    pub started_at: Option<f64>,
    /// When the last *informative* event arrived. A keepalive is transport
    /// liveness, not the agent, so it deliberately does not move this.
    pub last_output_at: Option<f64>,
}

/// A CLI that has said nothing for this long gets a note saying so.
///
/// Measured, and the reason this exists: a real `hermes` evaluation streamed
/// NOTHING for its whole 740-second budget — `-Q` buffers stdout until the end —
/// and a real `opencode2` evaluation went five minutes with two text events
/// before the CLI quit. Both were working. Both looked identical to a hang,
/// because the only thing the screen could say was "Running…".
pub const SILENCE_NOTICE_AFTER_SECS: f64 = 15.0;

/// The CV screen's state.
#[derive(Debug, Clone, Default)]
pub struct CvEditor {
    pub content: String,
    pub original: String,
    pub dirty: bool,
}

/// The follow-ups screen's state.
#[derive(Debug, Clone, Default)]
pub struct FollowupView {
    /// Which application the log dialog is open for.
    pub logging_for: Option<String>,
    pub log_channel: String,
    pub log_notes: String,
    pub log_contact: String,
    pub search: String,
    pub only_due: bool,
}

/// The whole app.
pub struct CareerOpsApp {
    pub backend: Arc<dyn Backend>,
    pub state: AppState,
    pub screen: Screen,
    pub settings: Settings,
    pub run: RunState,
    pub pipeline_view: PipelineView,
    pub followup_view: FollowupView,
    pub cv_editor: CvEditor,
    /// egui's clock at the last frame, so an event folded outside a `Ui` still
    /// gets a timestamp.
    pub run_now: f64,
}

impl CareerOpsApp {
    pub fn new(backend: Arc<dyn Backend>, settings: Settings) -> Self {
        CareerOpsApp {
            backend,
            state: AppState::default(),
            screen: Screen::Today,
            settings,
            run: RunState::default(),
            pipeline_view: PipelineView::default(),
            followup_view: FollowupView::default(),
            cv_editor: CvEditor::default(),
            run_now: 0.0,
        }
    }

    /// Re-apply the visual system after a theme or zoom change.
    pub fn apply_theme(&mut self, ctx: &egui::Context) {
        crate::theme::apply(ctx, self.settings.theme.clone(), self.settings.zoom);
        let _ = self.settings.save();
    }

    /// Step the zoom up or down, and remember it.
    pub fn zoom_by(&mut self, ctx: &egui::Context, steps: i32) {
        let next = (self.settings.zoom + steps as f32 * 0.05).clamp(0.75, 2.5);
        self.settings.zoom = (next * 100.0).round() / 100.0;
        self.apply_theme(ctx);
    }

    /// Ctrl +/-/0. The keyboard route to the same two buttons in the sidebar.
    pub fn handle_zoom_input(&mut self, ctx: &egui::Context) {
        use egui::Key;
        let (bigger, smaller, reset) = ctx.input(|i| {
            let command = i.modifiers.command;
            (
                command && (i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals)),
                command && i.key_pressed(Key::Minus),
                command && i.key_pressed(Key::Num0),
            )
        });
        if bigger {
            self.zoom_by(ctx, 1);
        } else if smaller {
            self.zoom_by(ctx, -1);
        } else if reset {
            self.settings.zoom = crate::settings::DEFAULT_ZOOM;
            self.apply_theme(ctx);
        }
    }

    /// Load everything the screens need.
    pub fn load_all(&mut self) {
        self.state.busy = true;
        self.load_bootstrap();
        self.load_pipeline();
        self.load_followups();
        self.load_config();
        self.load_cv();
        self.resolve_cli();
        self.state.busy = false;
    }

    pub fn load_bootstrap(&mut self) {
        match call_typed(self.backend.as_ref(), "bootstrap", json!({})) {
            Ok(value) => self.state.bootstrap = Some(value),
            Err(e) => self.state.fail(format!("could not read the install state: {e}")),
        }
    }

    pub fn load_pipeline(&mut self) {
        match call_typed(self.backend.as_ref(), "pipeline", json!({})) {
            Ok(value) => self.state.pipeline = Some(value),
            Err(e) => self.state.fail(format!("could not read the tracker: {e}")),
        }
    }

    pub fn load_followups(&mut self) {
        match call_typed::<_, Followups>(
            self.backend.as_ref(),
            "followups",
            json!({ "action": "list" }),
        ) {
            Ok(value) => self.state.followups = Some(value),
            Err(e) => self.state.fail(format!("could not read the follow-ups: {e}")),
        }
    }

    pub fn load_config(&mut self) {
        match call_typed(self.backend.as_ref(), "config", json!({ "action": "get" })) {
            Ok(value) => self.state.config = Some(value),
            Err(e) => self.state.fail(format!("could not read the config: {e}")),
        }
    }

    pub fn load_cv(&mut self) {
        match call_typed::<_, Cv>(self.backend.as_ref(), "cv", json!({ "action": "read" })) {
            Ok(value) => {
                self.cv_editor.original = value.content.clone();
                if !self.cv_editor.dirty {
                    self.cv_editor.content = value.content.clone();
                }
                self.state.cv = Some(value);
            }
            Err(e) => self.state.fail(format!("could not read the CV: {e}")),
        }
    }

    /// Open one application's report.
    pub fn open_report(&mut self, n: &str) {
        match call_typed::<_, Report>(self.backend.as_ref(), "report", json!({ "n": n })) {
            Ok(value) => {
                self.state.report = Some(value);
                self.screen = Screen::Report(n.to_string());
            }
            Err(e) => self.state.fail(format!("could not open report {n}: {e}")),
        }
    }

    /// Change a tracker row's status through the canonical writer.
    pub fn set_status(&mut self, n: &str, status: &str) {
        self.state.busy = true;
        let result = self
            .backend
            .call("status", json!({ "n": n, "status": status }));
        self.state.busy = false;
        match result {
            Ok(value) => {
                let changed = value
                    .get("changed")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let canonical = value
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(status);
                self.load_pipeline();
                self.load_followups();
                if changed {
                    self.state.say(format!("#{n} is now {canonical}"));
                } else {
                    self.state.say(format!("#{n} was already {canonical}"));
                }
            }
            Err(e) => self.state.fail(describe_status_error(&e)),
        }
    }

    /// Log a follow-up.
    pub fn log_followup(&mut self, app_num: &str, company: &str, role: &str) {
        let params = json!({
            "action": "log",
            "appNum": app_num,
            "company": company,
            "role": role,
            "channel": self.followup_view.log_channel,
            "contact": self.followup_view.log_contact,
            "notes": self.followup_view.log_notes,
        });
        match self.backend.call("followups", params) {
            Ok(_) => {
                self.state.say(format!("logged a follow-up for #{app_num}"));
                self.followup_view.logging_for = None;
                self.followup_view.log_notes.clear();
                self.followup_view.log_contact.clear();
                self.load_followups();
            }
            Err(e) => self.state.fail(format!("could not log the follow-up: {e}")),
        }
    }

    /// Save the CV editor's content.
    pub fn save_cv(&mut self) {
        match self.backend.call(
            "cv",
            json!({ "action": "write", "content": self.cv_editor.content }),
        ) {
            Ok(value) => {
                let backup = value.get("backedUp").and_then(serde_json::Value::as_str);
                self.cv_editor.dirty = false;
                self.cv_editor.original = self.cv_editor.content.clone();
                match backup {
                    Some(path) => self.state.say(format!("saved the CV (backup at {path})")),
                    None => self.state.say("saved the CV"),
                }
                self.load_cv();
            }
            Err(e) => self.state.fail(format!("could not save the CV: {e}")),
        }
    }
}

/// A status error, said in the user's terms.
fn describe_status_error(error: &BridgeError) -> String {
    match error.code.as_str() {
        "busy" => "the tracker is busy — another writer holds the lock. Try again in a moment.".into(),
        "not-found" => format!("no tracker row matches: {error}"),
        "core-script-missing" => {
            "this data root has no career-ops scripts, so the tracker cannot be written.".into()
        }
        _ => format!("could not change the status: {error}"),
    }
}

impl CareerOpsApp {
    /// Point the run screens at a CLI that actually exists.
    ///
    /// A saved id outlives the CLI it names — swap one install for another and
    /// the stored id still says `claude`, which is the bug the web app's
    /// keepIfInstalled() exists to prevent (#4012). Here it is worse than a 404:
    /// the run screen disables Start and the only explanation is "pick one in
    /// Config", for a CLI the user never chose.
    ///
    /// The substitute comes from the engine's own picker (`defaultCliId` on the
    /// bootstrap answer), not from a second rule invented on this side.
    pub fn resolve_cli(&mut self) -> bool {
        let Some(state) = self.state.bootstrap.as_ref() else {
            return false;
        };
        let clis = state.clis.clone();
        let default_id = state.default_cli_id.clone();
        let installed = |id: &str| clis.iter().any(|c| c.id == id && c.installed);
        if installed(&self.settings.cli_id) {
            return false;
        }
        let substitute = default_id
            .or_else(|| clis.iter().find(|c| c.installed).map(|c| c.id.clone()));
        let Some(substitute) = substitute else {
            return false;
        };
        if substitute == self.settings.cli_id {
            return false;
        }
        self.settings.cli_id = substitute;
        let _ = self.settings.save();
        true
    }

    /// Fold the bridge's streamed run events into the run log.
    ///
    /// The event vocabulary is the web app's: text / tool / status / keepalive
    /// / error / done. `keepalive` is a transport liveness signal and never
    /// becomes a log line; `done` and `error` end the run.
    pub fn absorb_events(&mut self, events: Vec<crate::bridge::StreamEvent>) {
        let now = self.run_now;
        self.absorb_events_at(events, now);
    }

    /// Fold events, with an explicit clock so the silence rule is testable.
    pub fn absorb_events_at(&mut self, events: Vec<crate::bridge::StreamEvent>, now: f64) {
        let request_id = self.run.request_id;
        for stream in events {
            if let Some(request_id) = request_id {
                if stream.request_id != request_id {
                    continue;
                }
            }
            let (kind, text) = match &stream.event {
                RunEvent::Text { text } => (LogKind::Text, text.trim_end().to_string()),
                RunEvent::Tool { name } => (LogKind::Tool, format!("[tool] {name}")),
                RunEvent::Status { label } => (LogKind::Status, label.clone()),
                RunEvent::Keepalive | RunEvent::Unknown => continue,
                RunEvent::Error { msg } => (LogKind::Error, format!("error: {msg}")),
                RunEvent::Done { .. } => (LogKind::Done, stream.event.describe()),
            };
            if text.is_empty() {
                continue;
            }
            // The agent said something. A keepalive never reaches here, which is
            // the point: it proves the transport is alive, never that the model is.
            self.run.last_output_at = Some(now);
            if kind == LogKind::Error {
                self.run.error = Some(text.clone());
            }
            if let RunEvent::Done { report_n, .. } = &stream.event {
                self.run.report_n = report_n.clone();
            }
            self.run.log.push(LogLine { text, kind });
            if stream.event.is_terminal() {
                self.run.running = false;
                self.run.finished = true;
                if kind == LogKind::Done {
                    self.load_pipeline();
                    self.load_bootstrap();
                }
            }
        }
    }

    /// Whether the Start button may be pressed.
    ///
    /// One rule, asked by the button and by the tests, rather than the same four
    /// conditions written inline in a view where nothing can check them.
    pub fn run_ready(&self) -> bool {
        if self.run.running || self.run.kind.is_none() || self.run.input.trim().is_empty() {
            return false;
        }
        self.state
            .bootstrap
            .as_ref()
            .and_then(|b| b.clis.iter().find(|c| c.id == self.settings.cli_id))
            .map(|c| c.installed)
            .unwrap_or(false)
    }

    /// How long the current run has been going, and how long it has been silent.
    ///
    /// `None` when nothing is running, so the screen can stay quiet.
    pub fn run_clock(&self, now: f64) -> Option<(f64, f64)> {
        if !self.run.running {
            return None;
        }
        let started = self.run.started_at.unwrap_or(now);
        let heard = self.run.last_output_at.unwrap_or(started);
        Some(((now - started).max(0.0), (now - heard).max(0.0)))
    }

    /// How many follow-ups are due, when the follow-ups are on.
    pub fn due_followups(&self) -> Option<usize> {
        let state = self.state.bootstrap.as_ref()?;
        if !state.followups_enabled {
            return None;
        }
        Some(self.state.followups.as_ref().map(|f| f.due.len()).unwrap_or(0))
    }
}
