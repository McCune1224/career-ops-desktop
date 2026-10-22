//! The app's state and navigation.

use std::sync::Arc;

use serde_json::json;

use crate::backend::{call_typed, Backend};
use crate::bridge::BridgeError;
use crate::model::{
    Analysis, Bootstrap, Config, Contacts, CostEstimate, Cv, Document, Followups, FollowupSeed,
    Liveness, Maintenance, Pipeline, Plugin, Portal, ProfileValidation, Report, RunEvent,
    SearchResults, StoryBank, Usage, Version, WhatsNew,
};
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
    Documents,
    Portals,
    Interview,
    Outcomes,
    Config,
}

impl Screen {
    /// The screen's name in the sidebar and in the settings file.
    pub fn id(&self) -> &'static str {
        match self {
            Screen::Today => "today",
            Screen::Pipeline => "pipeline",
            Screen::Report(_) => "report",
            Screen::Run => "run",
            Screen::Cv => "cv",
            Screen::Followups => "followups",
            Screen::Documents => "documents",
            Screen::Portals => "portals",
            Screen::Interview => "interview",
            Screen::Outcomes => "outcomes",
            Screen::Config => "config",
        }
    }

    /// The screen a saved id names, or Today when the id names none.
    ///
    /// A saved id that no longer matches a screen is not an error: the app
    /// opens where it can and the user moves on.
    pub fn from_id(id: &str) -> Screen {
        match id {
            "pipeline" => Screen::Pipeline,
            "run" => Screen::Run,
            "cv" => Screen::Cv,
            "followups" => Screen::Followups,
            "documents" => Screen::Documents,
            "portals" => Screen::Portals,
            "interview" => Screen::Interview,
            "outcomes" => Screen::Outcomes,
            "config" => Screen::Config,
            _ => Screen::Today,
        }
    }
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
    /// What the core's profile validator found.
    pub profile_validation: Option<ProfileValidation>,
    /// The last maintenance tool that ran, and what it said.
    pub maintenance: Option<Maintenance>,
    /// The client's own spend, in windows.
    pub usage: Option<Usage>,
    /// The last free-text search.
    pub search: Option<SearchResults>,
    /// The generated documents.
    pub documents: Option<Vec<Document>>,
    /// The watched boards, as the audit saw them.
    pub portals: Option<Vec<Portal>>,
    /// What is new this week.
    pub whats_new: Option<WhatsNew>,
    /// The story bank, with its provenance verdict.
    pub story_bank: Option<StoryBank>,
    /// The phonebook.
    pub contacts: Option<Contacts>,
    /// The plugins the core knows.
    pub plugins: Option<Vec<Plugin>>,
    /// The last liveness verdict.
    pub liveness: Option<Liveness>,
    /// The last analysis the report screen asked for.
    pub analysis: Option<Analysis>,
    /// What is running: the client, the bridge protocol and the engine release.
    pub version: Option<Version>,
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
    /// The rows picked for a bulk change.
    pub selected: Vec<String>,
    /// True while the "add an application by hand" form is open.
    pub adding: bool,
    /// The hand-added row's fields.
    pub new_company: String,
    pub new_role: String,
    pub new_date: String,
    pub new_status: String,
    pub new_url: String,
}

impl Default for PipelineView {
    fn default() -> Self {
        PipelineView {
            sort: SortKey::Tracker,
            descending: false,
            status_filter: "All".to_string(),
            min_score: 0.0,
            search: String::new(),
            selected: Vec::new(),
            adding: false,
            new_company: String::new(),
            new_role: String::new(),
            new_date: String::new(),
            new_status: "Evaluated".to_string(),
            new_url: String::new(),
        }
    }
}

/// The kinds of run the run screen can start.
///
/// The first three are the engine's own kinds: they persist an artifact
/// (a report, a PDF, a letter). The rest are career-ops MODES: the agent
/// follows the mode file and the reply is the deliverable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunKind {
    Evaluate,
    Pdf,
    Cover,
    Email,
    OfferPrep,
    Prep,
    Plan,
    Practice,
    Debrief,
    RedFlag,
    FixPortal,
}

/// Every kind, in the order the screen offers them.
pub const RUN_KINDS: [RunKind; 11] = [
    RunKind::Evaluate,
    RunKind::Pdf,
    RunKind::Cover,
    RunKind::Email,
    RunKind::OfferPrep,
    RunKind::Prep,
    RunKind::Plan,
    RunKind::Practice,
    RunKind::Debrief,
    RunKind::RedFlag,
    RunKind::FixPortal,
];

impl RunKind {
    pub fn id(&self) -> &'static str {
        match self {
            RunKind::Evaluate => "evaluate",
            RunKind::Pdf => "pdf",
            RunKind::Cover => "cover",
            RunKind::Email => "email",
            RunKind::OfferPrep => "offer-prep",
            RunKind::Prep => "prep",
            RunKind::Plan => "plan",
            RunKind::Practice => "practice",
            RunKind::Debrief => "debrief",
            RunKind::RedFlag => "redflag",
            RunKind::FixPortal => "fix-portal",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            RunKind::Evaluate => "Evaluate a posting",
            RunKind::Pdf => "Tailor the CV (PDF)",
            RunKind::Cover => "Draft a cover letter",
            RunKind::Email => "Draft an application email",
            RunKind::OfferPrep => "Walk an offer's clauses",
            RunKind::Prep => "Build an interview prep pack",
            RunKind::Plan => "Plan interview preparation",
            RunKind::Practice => "Practise interview questions",
            RunKind::Debrief => "Debrief a real interview",
            RunKind::RedFlag => "Judge whether a company is safe",
            RunKind::FixPortal => "Repair a broken portal",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            RunKind::Evaluate => "Paste a job URL or the posting text. The agent scores it and the report is saved.",
            RunKind::Pdf => "Paste the report number or the company. The agent tailors your CV and it is rendered to PDF.",
            RunKind::Cover => "Paste the report number or the company, plus a short brief of what to say.",
            RunKind::Email => "Paste the report number or the company. The agent drafts the email; nothing is ever sent.",
            RunKind::OfferPrep => "Paste the offer text. The agent walks the clauses and lists the questions for your lawyer.",
            RunKind::Prep => "Paste the report number or the company. The agent builds the prep pack and saves it for you.",
            RunKind::Plan => "Paste the report number or the company. The agent allocates preparation time to named topics.",
            RunKind::Practice => "Paste the question, or a report number. The agent asks and gives specific feedback.",
            RunKind::Debrief => "Paste what happened. The agent turns it into gaps that carry into the next pack.",
            RunKind::RedFlag => "Paste the company. The agent cites what it found and says what it could not check.",
            RunKind::FixPortal => "Paste the company name. The agent finds the working board and repairs that one entry.",
        }
    }

    /// True for the kinds the engine persists an artifact for.
    pub fn persists(&self) -> bool {
        matches!(self, RunKind::Evaluate | RunKind::Pdf | RunKind::Cover)
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

impl LogKind {
    /// The name the web app's run log uses for this kind of step.
    pub fn kind_name(&self) -> &'static str {
        match self {
            LogKind::Text => "text",
            LogKind::Tool => "tool",
            LogKind::Status => "status",
            LogKind::Error => "error",
            LogKind::Done => "done",
        }
    }
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
    /// What the last run of this kind is expected to cost.
    pub estimate: Option<CostEstimate>,
    /// The tokens the last run reported.
    pub tokens: Option<f64>,
    /// The dollar cost the last run reported.
    pub cost_usd: Option<f64>,
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

/// The outcome form's state.
#[derive(Debug, Clone)]
pub struct OutcomeForm {
    pub target: String,
    pub kind: String,
    pub note: String,
    pub output: String,
}

impl Default for OutcomeForm {
    fn default() -> Self {
        OutcomeForm {
            target: String::new(),
            kind: "interview_progress".to_string(),
            note: String::new(),
            output: String::new(),
        }
    }
}

/// The archive form's state, kept in the app so a screen switch does not lose
/// what the user typed.
#[derive(Debug, Clone, Default)]
pub struct ArchiveForm {
    pub url: String,
    pub company: String,
    pub role: String,
    pub report: String,
    pub dry_run: bool,
    pub output: String,
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
    /// Which application the "pin the next date" form is open for.
    pub pinning_for: Option<String>,
    pub pin_date: String,
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
    /// The posting-archive form on the documents screen.
    pub archive: ArchiveForm,
    /// The coverage-check form on the portals screen.
    pub coverage: crate::views::portals::CoverageForm,
    /// The board-discovery form on the portals screen.
    pub discover: crate::views::portals::DiscoverForm,
    /// The question and its matched story, on the interview screen.
    pub story_match: crate::views::interview::StoryMatch,
    /// The outcome form on the outcomes screen.
    pub outcome: OutcomeForm,
    /// The rows the user shortlisted for a batch, by inbox URL.
    pub shortlist: Vec<String>,
    /// The state a bulk change applies.
    pub bulk_state: String,
    /// The free-text search query.
    pub find_query: String,
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
            archive: ArchiveForm::default(),
            coverage: crate::views::portals::CoverageForm::default(),
            discover: crate::views::portals::DiscoverForm::default(),
            story_match: crate::views::interview::StoryMatch::default(),
            outcome: OutcomeForm::default(),
            shortlist: Vec::new(),
            bulk_state: "Applied".to_string(),
            find_query: String::new(),
            run_now: 0.0,
        }
    }

    /// Go to a screen, and remember it.
    ///
    /// The remembered screen is what the app reopens on, so it is written
    /// whenever the user moves — a screen reached by a click and a screen
    /// reached by a key are the same event.
    pub fn go(&mut self, screen: Screen) {
        let id = screen.id();
        self.screen = screen;
        if self.settings.last_screen != id {
            self.settings.last_screen = id.to_string();
            let _ = self.settings.save();
        }
    }

    /// Ctrl 1..9 and Ctrl 0: the screen shortcuts.
    ///
    /// Ten screens and ten shortcuts, so every screen is reachable from the
    /// keyboard alone. The sidebar is the other route and is focusable.
    pub fn handle_screen_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::Key;
        let index = ctx.input(|i| {
            if !i.modifiers.command {
                return None;
            }
            let keys = [
                Key::Num1,
                Key::Num2,
                Key::Num3,
                Key::Num4,
                Key::Num5,
                Key::Num6,
                Key::Num7,
                Key::Num8,
                Key::Num9,
            ];
            for (at, key) in keys.iter().enumerate() {
                if i.key_pressed(*key) {
                    return Some(at);
                }
            }
            if i.key_pressed(Key::Num0) {
                return Some(9);
            }
            None
        });
        let Some(index) = index else { return };
        let Some((_, screen)) = crate::shell::NAV.get(index) else {
            return;
        };
        self.go(screen.clone());
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

    /// Pick a row for a bulk change.
    pub fn toggle_selection(&mut self, n: &str) {
        if let Some(at) = self.pipeline_view.selected.iter().position(|row| row == n) {
            self.pipeline_view.selected.remove(at);
        } else {
            self.pipeline_view.selected.push(n.to_string());
        }
    }

    /// Whether a row is picked for a bulk change.
    pub fn is_selected(&self, n: &str) -> bool {
        self.pipeline_view.selected.iter().any(|row| row == n)
    }

    /// Apply one state to every picked row, through the canonical writer.
    ///
    /// One call per row, so each change gets its own ledger row and a row that
    /// refuses does not take the others with it.
    pub fn bulk_status(&mut self, status: &str) {
        if self.pipeline_view.selected.is_empty() {
            self.state.fail("pick at least one row first");
            return;
        }
        let rows = self.pipeline_view.selected.clone();
        self.state.busy = true;
        let result = self
            .backend
            .call("status", json!({ "rows": rows, "status": status }));
        self.state.busy = false;
        match result {
            Ok(value) => {
                let applied = value.get("applied").and_then(serde_json::Value::as_u64).unwrap_or(0);
                let failed = value.get("failed").and_then(serde_json::Value::as_u64).unwrap_or(0);
                self.pipeline_view.selected.clear();
                self.load_pipeline();
                self.load_followups();
                if failed == 0 {
                    self.state.say(format!("{applied} row(s) are now {status}"));
                } else {
                    self.state
                        .fail(format!("{applied} row(s) changed, {failed} did not"));
                }
            }
            Err(e) => self.state.fail(format!("could not change the status: {e}")),
        }
    }

    /// Skip an inbox posting, or un-skip it.
    pub fn skip_inbox(&mut self, url: &str, done: bool) {
        match self.backend.call("inbox", json!({ "url": url, "done": done })) {
            Ok(_) => {
                self.load_pipeline();
                self.state.say(if done { "skipped the posting" } else { "un-skipped the posting" });
            }
            Err(e) => self.state.fail(format!("could not skip the posting: {e}")),
        }
    }

    /// Propose removing a tracker row, and name the report it orphans.
    pub fn preview_delete(&mut self, n: &str) {
        match self.backend.call("tracker", json!({ "action": "deletePreview", "n": n })) {
            Ok(value) => {
                let orphaned = value
                    .get("orphanedReport")
                    .and_then(serde_json::Value::as_str)
                    .map(|s| s.to_string());
                let message = match orphaned {
                    Some(report) => format!("removing #{n} orphans the report {report}"),
                    None => format!("removing #{n} orphans no report"),
                };
                self.state.say(message);
            }
            Err(e) => self.state.fail(format!("could not check the removal: {e}")),
        }
    }

    /// Remove a tracker row, through the core's own remover.
    pub fn delete_row(&mut self, n: &str) {
        match self.backend.call("tracker", json!({ "action": "delete", "n": n })) {
            Ok(value) => {
                let orphaned = value
                    .get("orphanedReport")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                self.load_pipeline();
                self.load_bootstrap();
                if orphaned.is_empty() {
                    self.state.say(format!("removed #{n}"));
                } else {
                    self.state.say(format!("removed #{n}; the report {orphaned} is now orphaned"));
                }
            }
            Err(e) => self.state.fail(format!("could not remove the row: {e}")),
        }
    }

    /// Add an application by hand, through the core's merge.
    pub fn add_application(&mut self) {
        let row = json!({
            "company": self.pipeline_view.new_company,
            "role": self.pipeline_view.new_role,
            "date": self.pipeline_view.new_date,
            "status": self.pipeline_view.new_status,
            "url": self.pipeline_view.new_url,
        });
        match self.backend.call("tracker", json!({ "action": "add", "row": row })) {
            Ok(value) => {
                let num = value.get("num").and_then(serde_json::Value::as_i64);
                self.pipeline_view.adding = false;
                self.pipeline_view.new_company.clear();
                self.pipeline_view.new_role.clear();
                self.pipeline_view.new_date.clear();
                self.pipeline_view.new_url.clear();
                self.load_pipeline();
                match num {
                    Some(n) => self.state.say(format!("added the application as #{n}")),
                    None => self.state.say("added the application"),
                }
            }
            Err(e) => self.state.fail(format!("could not add the application: {e}")),
        }
    }

    /// Find a tracker row or a report by free text.
    pub fn search_tracker(&mut self, query: &str) {
        match self.backend.call("tracker", json!({ "action": "find", "query": query })) {
            Ok(value) => match serde_json::from_value::<SearchResults>(value) {
                Ok(results) => {
                    let count = results.found;
                    self.state.search = Some(results);
                    if count == 0 {
                        self.state.say(format!("nothing matches '{query}'"));
                    } else {
                        self.state.say(format!("{count} match(es) for '{query}'"));
                    }
                }
                Err(e) => self.state.fail(format!("the search answer did not decode: {e}")),
            },
            Err(e) => self.state.fail(format!("could not search: {e}")),
        }
    }

    /// Put a posting on (or off) the shortlist.
    ///
    /// The shortlist is the client's own list and it is saved, so it survives a
    /// restart: a shortlist you cannot come back to is one you rebuild.
    pub fn toggle_shortlist(&mut self, url: &str) {
        if let Some(at) = self.shortlist.iter().position(|u| u == url) {
            self.shortlist.remove(at);
        } else {
            self.shortlist.push(url.to_string());
        }
        self.settings.shortlist = self.shortlist.clone();
        let _ = self.settings.save();
    }

    /// Whether a posting is on the shortlist.
    pub fn is_shortlisted(&self, url: &str) -> bool {
        self.shortlist.iter().any(|u| u == url)
    }

    /// The spend estimate for scoring the whole shortlist.
    pub fn estimate_shortlist(&mut self) {
        let count = self.shortlist.len();
        if count == 0 {
            self.state.fail("the shortlist is empty");
            return;
        }
        self.estimate_cost("evaluate", count);
    }

    /// Start scoring the whole shortlist, one posting at a time.
    ///
    /// The caller confirmed the spend first; this is the second half of that
    /// promise, and it is the only place a batch starts.
    pub fn start_shortlist_batch(&mut self) {
        let inputs = self.shortlist.clone();
        if inputs.is_empty() {
            self.state.fail("the shortlist is empty");
            return;
        }
        self.run.log.clear();
        self.run.error = None;
        self.run.finished = false;
        self.run.report_n = None;
        match self
            .backend
            .start_batch("evaluate", &inputs, &self.settings.cli_id)
        {
            Ok(id) => {
                let now = self.run_now;
                self.run.running = true;
                self.run.request_id = Some(id);
                self.run.started_at = Some(now);
                self.run.last_output_at = Some(now);
                self.run.log.push(crate::app::LogLine {
                    text: format!("Scoring {} shortlisted posting(s)…", inputs.len()),
                    kind: LogKind::Status,
                });
                self.go(Screen::Run);
            }
            Err(e) => {
                self.run.running = false;
                self.run.error = Some(format!("could not start the batch: {e}"));
            }
        }
    }

    /// Read what is running, and refuse a mismatched pair.
    pub fn load_version(&mut self) {
        match call_typed::<_, Version>(self.backend.as_ref(), "version", json!({ "action": "get" })) {
            Ok(value) => {
                let mismatch = value.protocol != crate::bridge::PROTOCOL_VERSION;
                let behind = if mismatch {
                    Some(if value.protocol < crate::bridge::PROTOCOL_VERSION {
                        "the bridge is behind the app"
                    } else {
                        "the app is behind the bridge"
                    })
                } else {
                    None
                };
                self.state.version = Some(value);
                if let Some(behind) = behind {
                    self.state
                        .fail(format!("{behind} — the versions do not match, so no run will start"));
                }
            }
            Err(e) => self.state.fail(format!("could not read the version: {e}")),
        }
    }

    /// Whether a run may start at all.
    ///
    /// A mismatched pair is refused: an app talking to a bridge it does not
    /// understand would show a screen that is wrong in ways nobody can see.
    pub fn version_matches(&self) -> bool {
        match self.state.version.as_ref() {
            Some(version) => version.protocol == crate::bridge::PROTOCOL_VERSION,
            None => true,
        }
    }

    /// Ask for one analysis and keep what came back.
    ///
    /// The argument is the report number for a per-report analysis and the
    /// company name for a per-company one; the core's scripts decide what it
    /// means.
    pub fn analysis_action(&mut self, action: &str, argument: &str) {
        let params = match action {
            "keywordMatch" | "scoreLevers" => serde_json::json!({ "action": action, "n": argument }),
            "companyHistory" => serde_json::json!({ "action": action, "company": argument }),
            _ => serde_json::json!({ "action": action }),
        };
        match self.backend.call("analysis", params) {
            Ok(value) => {
                let title = match action {
                    "keywordMatch" => "Keyword coverage".to_string(),
                    "scoreLevers" => "What moved the score".to_string(),
                    "companyHistory" => format!("Company history: {argument}"),
                    other => other.to_string(),
                };
                let body = render_analysis(action, &value);
                self.state.analysis = Some(Analysis {
                    action: action.to_string(),
                    title,
                    body,
                });
            }
            Err(e) => self.state.fail(format!("could not run {action}: {e}")),
        }
    }

    /// Ask the core's validator what is wrong with the profile.
    pub fn validate_profile(&mut self) {
        match call_typed::<_, ProfileValidation>(
            self.backend.as_ref(),
            "config",
            json!({ "action": "validate" }),
        ) {
            Ok(value) => {
                let ok = value.ok;
                let message = value.message.clone();
                self.state.profile_validation = Some(value);
                if ok {
                    self.state.say("the profile is fine");
                } else {
                    self.state.fail(message);
                }
            }
            Err(e) => self.state.fail(format!("could not validate the profile: {e}")),
        }
    }

    /// Run one maintenance tool, or propose its changes.
    pub fn run_maintenance(&mut self, tool: &str, apply: bool) {
        self.state.busy = true;
        let result = self
            .backend
            .call("maintenance", json!({ "tool": tool, "apply": apply }));
        self.state.busy = false;
        match result {
            Ok(value) => match serde_json::from_value::<Maintenance>(value) {
                Ok(result) => {
                    let ok = result.ok;
                    self.state.maintenance = Some(result);
                    if ok {
                        self.state.say(if apply { "done" } else { "nothing was changed — this is the proposal" });
                    } else {
                        self.state.fail("the tool reported problems");
                    }
                    self.load_pipeline();
                }
                Err(e) => self.state.fail(format!("the tool's answer did not decode: {e}")),
            },
            Err(e) => self.state.fail(format!("could not run the tool: {e}")),
        }
    }

    /// Remove a logged follow-up.
    pub fn delete_followup(&mut self, num: &str) {
        match self.backend.call("followups", json!({ "action": "delete", "num": num })) {
            Ok(_) => {
                self.load_followups();
                self.state.say(format!("removed the logged follow-up #{num}"));
            }
            Err(e) => self.state.fail(format!("could not remove the follow-up: {e}")),
        }
    }

    /// Pin the next follow-up date for one application.
    pub fn pin_followup(&mut self, app_num: &str, date: &str) {
        let params = json!({ "action": "override", "appNum": app_num, "date": date });
        match self.backend.call("followups", params) {
            Ok(_) => {
                self.followup_view.pinning_for = None;
                self.load_followups();
                self.state.say(format!("pinned the next follow-up for #{app_num} on {date}"));
            }
            Err(e) => self.state.fail(format!("could not pin the follow-up: {e}")),
        }
    }

    /// Clear a pinned follow-up date.
    pub fn unpin_followup(&mut self, app_num: &str) {
        let params = json!({ "action": "unoverride", "appNum": app_num });
        match self.backend.call("followups", params) {
            Ok(_) => {
                self.load_followups();
                self.state.say(format!("cleared the pinned date for #{app_num}"));
            }
            Err(e) => self.state.fail(format!("could not clear the pin: {e}")),
        }
    }

    /// Ask what a run of this kind costs, from this client's own history.
    pub fn estimate_cost(&mut self, kind: &str, count: usize) {
        match call_typed::<_, CostEstimate>(
            self.backend.as_ref(),
            "cost",
            json!({ "action": "estimate", "kind": kind, "count": count }),
        ) {
            Ok(value) => {
                self.run.estimate = Some(value);
            }
            Err(e) => self.state.fail(format!("could not estimate the cost: {e}")),
        }
    }

    /// Load the client's own spend.
    pub fn load_usage(&mut self) {
        match call_typed::<_, Usage>(self.backend.as_ref(), "cost", json!({ "action": "usage" })) {
            Ok(value) => self.state.usage = Some(value),
            Err(e) => self.state.fail(format!("could not read the usage: {e}")),
        }
    }

    /// Record a finished run, so it can be read back and costed.
    pub fn save_run(&mut self, id: &str, kind: &str, input: &str, output: &str, tokens: Option<f64>, usd: Option<f64>) {
        let run = json!({
            "id": id,
            "kind": kind,
            "title": id,
            "input": input,
            "status": "done",
            "tokens": tokens.unwrap_or(0.0),
            "usd": usd.unwrap_or(0.0),
            "startedAt": now_millis(),
            "steps": self.run.log.iter().map(|line| json!({ "kind": line.kind.kind_name(), "label": line.text })).collect::<Vec<_>>(),
            "output": output,
        });
        match self.backend.call("runs", json!({ "action": "save", "run": run })) {
            Ok(_) => self.state.say("saved the run log"),
            Err(e) => self.state.fail(format!("could not save the run log: {e}")),
        }
    }

    /// Seed the first follow-up date for a row that turned Applied.
    pub fn seed_followup(&mut self, n: &str) {
        match call_typed::<_, FollowupSeed>(
            self.backend.as_ref(),
            "followups",
            json!({ "action": "seed", "appNum": n }),
        ) {
            Ok(value) => {
                self.load_followups();
                if value.seeded {
                    let date = value.next_date.unwrap_or_default();
                    self.state.say(format!("pinned the first follow-up for #{n} on {date}"));
                } else {
                    self.state.say(format!("no first follow-up was pinned for #{n}"));
                }
            }
            Err(e) => self.state.fail(format!("could not seed the follow-up: {e}")),
        }
    }

    /// Load the generated documents.
    pub fn load_documents(&mut self) {
        match self.backend.call("documents", json!({ "action": "list" })) {
            Ok(value) => match serde_json::from_value::<Vec<Document>>(value.get("documents").cloned().unwrap_or(serde_json::Value::Null)) {
                Ok(documents) => self.state.documents = Some(documents),
                Err(e) => self.state.fail(format!("the documents did not decode: {e}")),
            },
            Err(e) => self.state.fail(format!("could not read the documents: {e}")),
        }
    }

    /// Load the watched boards, as the audit saw them.
    pub fn load_portals(&mut self) {
        match self.backend.call("portals", json!({ "action": "audit" })) {
            Ok(value) => match serde_json::from_value::<Vec<Portal>>(value.get("rows").cloned().unwrap_or(serde_json::Value::Null)) {
                Ok(rows) => self.state.portals = Some(rows),
                Err(e) => self.state.fail(format!("the portal audit did not decode: {e}")),
            },
            Err(e) => self.state.fail(format!("could not audit the boards: {e}")),
        }
    }

    /// Load what is new this week.
    pub fn load_whats_new(&mut self) {
        match call_typed::<_, WhatsNew>(
            self.backend.as_ref(),
            "portals",
            json!({ "action": "whatsNew" }),
        ) {
            Ok(value) => self.state.whats_new = Some(value),
            Err(e) => self.state.fail(format!("could not read the new postings: {e}")),
        }
    }

    /// Load the story bank and its provenance verdict.
    pub fn load_story_bank(&mut self) {
        match call_typed::<_, StoryBank>(
            self.backend.as_ref(),
            "interview",
            json!({ "action": "storyBank" }),
        ) {
            Ok(value) => self.state.story_bank = Some(value),
            Err(e) => self.state.fail(format!("could not read the story bank: {e}")),
        }
    }

    /// Load the phonebook.
    pub fn load_contacts(&mut self) {
        match call_typed::<_, Contacts>(
            self.backend.as_ref(),
            "contacts",
            json!({ "action": "list" }),
        ) {
            Ok(value) => self.state.contacts = Some(value),
            Err(e) => self.state.fail(format!("could not read the contacts: {e}")),
        }
    }

    /// Load the plugins the core knows.
    pub fn load_plugins(&mut self) {
        match self.backend.call("plugins", json!({ "action": "list" })) {
            Ok(value) => {
                let text = value.get("output").and_then(serde_json::Value::as_str).unwrap_or("");
                self.state.plugins = Some(parse_plugin_list(text));
            }
            Err(e) => self.state.fail(format!("could not read the plugins: {e}")),
        }
    }

    /// Check whether a posting is still live.
    pub fn check_liveness(&mut self, url: &str) {
        match call_typed::<_, Liveness>(
            self.backend.as_ref(),
            "liveness",
            json!({ "action": "check", "url": url }),
        ) {
            Ok(value) => {
                let verdict = value.verdict.clone();
                self.state.liveness = Some(value);
                self.state.say(format!("the posting is {verdict}"));
            }
            Err(e) => self.state.fail(format!("could not check the posting: {e}")),
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

/// An analysis answer, as the screen shows it.
///
/// The core's answers are structured; the screen shows the parts a person
/// reads. Nothing here decides what the analysis means — the verdict, the
/// counts and the lists are the core's.
fn render_analysis(action: &str, value: &serde_json::Value) -> String {
    let get = |key: &str| value.get(key).and_then(serde_json::Value::as_str).unwrap_or("");
    let count = |key: &str| value.get(key).and_then(serde_json::Value::as_u64).unwrap_or(0);
    match action {
        "keywordMatch" => {
            let mut lines = vec![format!(
                "{} of {} keyword(s) covered ({:.0}%)",
                count("presentCount"),
                count("total"),
                value.get("coveragePct").and_then(serde_json::Value::as_f64).unwrap_or(0.0)
            )];
            for tier in ["present", "thin", "missing"] {
                let items = value
                    .get(tier)
                    .and_then(serde_json::Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .map(|v| v.as_str().unwrap_or("").to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if !items.is_empty() {
                    lines.push(format!("{tier}: {}", items.join(", ")));
                }
            }
            lines.join("\n")
        }
        "scoreLevers" => {
            let levers = value.get("levers").cloned().unwrap_or(serde_json::Value::Null);
            let mut lines = Vec::new();
            for (key, label) in [
                ("holding", "held the score"),
                ("close", "close to moving"),
            ] {
                if let Some(items) = levers.get(key).and_then(serde_json::Value::as_array) {
                    for item in items {
                        let text = item.get("text").and_then(serde_json::Value::as_str).unwrap_or("");
                        if !text.is_empty() {
                            lines.push(format!("{label}: {text}"));
                        }
                    }
                }
            }
            if lines.is_empty() {
                lines.push("this report has no machine summary".to_string());
            }
            lines.join("\n")
        }
        "companyHistory" => {
            let mut lines = Vec::new();
            if let Some(items) = value.get("rows").and_then(serde_json::Value::as_array) {
                for item in items {
                    let company = item.get("company").and_then(serde_json::Value::as_str).unwrap_or("");
                    let role = item.get("role").and_then(serde_json::Value::as_str).unwrap_or("");
                    let status = item.get("status").and_then(serde_json::Value::as_str).unwrap_or("");
                    if !company.is_empty() || !role.is_empty() {
                        lines.push(format!("{company} — {role} ({status})"));
                    }
                }
            }
            if lines.is_empty() {
                lines.push(format!("nothing recorded for {}", get("company")));
            }
            lines.join("\n")
        }
        _ => get("output").to_string(),
    }
}

/// The current time in milliseconds, for a run record.
fn now_millis() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// The plugin table the core printed, read back into the model.
///
/// `plugins.mjs list` prints a table, not JSON, so the table is read here.
/// The columns are the core's own; nothing is inferred from them.
fn parse_plugin_list(text: &str) -> Vec<Plugin> {
    let mut plugins = Vec::new();
    for line in text.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("ID") || trimmed.starts_with("---") {
            continue;
        }
        let mut cells = trimmed.split_whitespace();
        let Some(id) = cells.next() else { continue };
        let name = cells.next().unwrap_or("").to_string();
        let enabled = trimmed.contains("enabled");
        plugins.push(Plugin {
            id: id.to_string(),
            name,
            enabled,
            hooks: Vec::new(),
            missing_env: Vec::new(),
        });
    }
    plugins
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
        // SAY IT. A swap that happens quietly is a wrong result the user never
        // learns about: the run was made by a CLI they did not choose, and the
        // only honest thing is to name the swap on the screen.
        let previous = self.settings.cli_id.clone();
        self.settings.cli_id = substitute.clone();
        let _ = self.settings.save();
        let name = clis
            .iter()
            .find(|c| c.id == substitute)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| substitute.clone());
        if previous.trim().is_empty() {
            self.state.say(format!("using {name}"));
        } else {
            self.state
                .say(format!("{previous} is not installed — using {name} instead"));
        }
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
            if let RunEvent::Done { report_n, tokens, cost_usd } = &stream.event {
                self.run.report_n = report_n.clone();
                self.run.tokens = tokens.map(|t| t as f64);
                self.run.cost_usd = cost_usd.map(|c| c as f64);
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
        // A mismatched pair is refused: the screen would be wrong in ways
        // nobody can see.
        if !self.version_matches() {
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
