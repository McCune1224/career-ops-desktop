//! The data shapes the Node bridge sends.
//!
//! These mirror the bridge's JSON exactly. The bridge is the only thing that
//! speaks to the career-ops engine, so these types are the app's view of it —
//! there is no second model of a tracker row, a report or a follow-up anywhere
//! else in the app.

use serde::{Deserialize, Serialize};

/// A tracker row, as the shared tracker parser produces it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Application {
    pub n: String,
    pub date: String,
    pub company: String,
    pub via: String,
    pub role: String,
    pub location: String,
    pub score: String,
    pub status: String,
    pub pdf: String,
    pub report: String,
    pub notes: String,
    pub url: String,
    pub apply_link: String,
    pub follow_up: String,
}

impl Application {
    /// The numeric part of the tracker `#`, for sorting.
    pub fn number(&self) -> i64 {
        self.n.trim().parse::<i64>().unwrap_or(i64::MAX)
    }

    /// The score as a number, for the min-score filter and for sorting.
    /// An unrated row sorts last rather than as zero.
    pub fn score_value(&self) -> f32 {
        parse_score(&self.score)
    }
}

/// "4.2/5" -> 4.2. Anything else -> 0.0.
pub fn parse_score(raw: &str) -> f32 {
    let head = raw.split('/').next().unwrap_or("").trim();
    head.parse::<f32>().unwrap_or(0.0)
}

/// A pending posting from data/pipeline.md.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InboxJob {
    pub url: String,
    pub company: String,
    pub role: String,
    pub location: String,
    pub compensation: String,
    pub done: bool,
    pub posted_at: Option<String>,
}

/// A CLI the core knows how to drive.
///
/// `path` is null when the CLI is not installed — `findBin()` in the web app's
/// clis.ts answers `string | null`, and the bridge passes that through. Typing
/// it as a `String` made the whole bootstrap payload undecodable on any machine
/// missing even one CLI, which is the common case.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Cli {
    pub id: String,
    pub name: String,
    pub run: String,
    pub url: String,
    pub installed: bool,
    pub path: Option<String>,
}

/// What the install looks like right now.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Bootstrap {
    pub data_root: String,
    pub code_root: String,
    pub doctor: Doctor,
    pub doctor_state: DoctorState,
    pub clis: Vec<Cli>,
    /// The CLI the engine would drive when the app has no usable preference.
    pub default_cli_id: Option<String>,
    pub states: Vec<String>,
    pub tracker_exists: bool,
    pub inbox_count: usize,
    pub application_count: usize,
    pub reports: usize,
    pub pdfs: usize,
    pub followups_enabled: bool,
    pub cv_exists: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Doctor {
    pub available: bool,
    pub onboarding_needed: bool,
    pub missing: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DoctorState {
    pub phase: String,
    pub onboarding_needed: bool,
    pub missing: Vec<String>,
    pub has_cv: bool,
    pub has_data: bool,
}

/// The tracker and the inbox, as the pipeline screen needs them.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Pipeline {
    pub root: String,
    pub root_exists: bool,
    pub tracker_path: String,
    pub inbox: Vec<InboxJob>,
    pub applications: Vec<Application>,
}

/// One application, with its report.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Report {
    pub n: String,
    pub application: Option<Application>,
    pub file: Option<String>,
    pub markdown: Option<String>,
    pub title: Option<String>,
    pub legitimacy: Option<String>,
    pub fields: Vec<ReportField>,
    /// The report's introduction, before its first block.
    pub intro: String,
    /// The one-line "why this score".
    pub lead: String,
    /// The report's VERDICT line, lifted out of the blocks so folding the
    /// body never hides the answer.
    pub verdict: Option<String>,
    /// The report's blocks, split by the engine's own splitter.
    pub sections: Vec<ReportSection>,
    pub posting_url: Option<String>,
    pub tracker_url: Option<String>,
    pub cv_pdf: Option<String>,
    pub cv_html: Option<String>,
    pub cover_pdf: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReportField {
    pub label: String,
    pub value: String,
}

/// One block of a report, as the engine's splitter produced it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReportSection {
    pub heading: String,
    pub letter: Option<String>,
    pub content: String,
    /// The one-line summary shown while the block is folded.
    pub preview: String,
    /// True for the block that carries the CV match.
    pub lead: bool,
}

/// A follow-up, as the core's cadence calculator reports it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FollowupEntry {
    pub num: i64,
    pub date: String,
    pub company: String,
    pub role: String,
    pub status: String,
    pub score: String,
    pub notes: String,
    pub urgency: String,
    pub next_followup_date: Option<String>,
    pub days_until_next: Option<i64>,
    pub days_since_application: Option<i64>,
    pub days_since_last_followup: Option<i64>,
    pub followup_count: i64,
    pub report_path: Option<String>,
    pub contacts: Vec<String>,
}

/// One logged follow-up, from data/follow-ups.md.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FollowupLogEntry {
    pub num: Option<i64>,
    pub app_num: Option<i64>,
    pub date: String,
    pub company: String,
    pub role: String,
    pub channel: String,
    pub contact: String,
    pub notes: String,
}

/// The follow-ups screen's data.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Followups {
    pub available: bool,
    pub entries: Vec<FollowupEntry>,
    pub due: Vec<FollowupEntry>,
    pub upcoming: Vec<FollowupEntry>,
    pub next_upcoming: Option<FollowupEntry>,
    pub history: Vec<FollowupLogEntry>,
    pub overrides: Vec<FollowupOverride>,
    pub followups_enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FollowupOverride {
    pub app_num: i64,
    pub date: Option<String>,
}

/// The cadence knobs, from config/profile.yml and the core's defaults.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Cadence {
    pub defaults: std::collections::BTreeMap<String, i64>,
    pub defaults_available: bool,
    pub overrides: std::collections::BTreeMap<String, i64>,
    pub effective: std::collections::BTreeMap<String, i64>,
    pub enabled: bool,
}

/// The config screen's data.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub profile: serde_json::Value,
    pub exists: bool,
    pub malformed: bool,
    pub clis: Vec<Cli>,
    pub states: Vec<String>,
    pub code_root: String,
    pub data_root: String,
}

/// A CV, read or written.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Cv {
    pub content: String,
    pub exists: bool,
    pub path: String,
    pub ok: bool,
    pub backed_up: Option<String>,
}

/// What a status change returned.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StatusResult {
    pub status: String,
    pub changed: bool,
    pub status_logged: bool,
}

/// One row's outcome in a bulk status change.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BulkRow {
    pub n: String,
    pub ok: bool,
    pub status: String,
    pub changed: bool,
    pub status_logged: bool,
    pub error: Option<String>,
    pub code: Option<String>,
}

/// A bulk status change: one writer call per row, one outcome per row.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BulkStatus {
    pub status: String,
    pub applied: usize,
    pub failed: usize,
    pub ledger_rows: usize,
    pub results: Vec<BulkRow>,
}

/// A tracker row that was removed, or proposed for removal.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TrackerDelete {
    pub ok: bool,
    pub n: String,
    /// The report the removal orphans, when the row had one.
    pub orphaned_report: Option<String>,
    pub output: String,
    pub dry_run: bool,
}

/// A row the user added by hand.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TrackerAdd {
    pub ok: bool,
    pub num: Option<i64>,
    pub company: String,
    pub role: String,
    pub status: String,
    pub score: String,
}

/// One finding from the profile validator.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProfileFinding {
    pub level: String,
    pub code: String,
    pub key: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

/// What the core's profile validator found.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProfileValidation {
    pub ok: bool,
    pub exit_code: i64,
    pub findings: Vec<ProfileFinding>,
    pub errors: Vec<ProfileFinding>,
    pub warnings: Vec<ProfileFinding>,
    pub message: String,
}

/// One maintenance tool's outcome.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Maintenance {
    pub tool: String,
    pub apply: bool,
    pub ok: bool,
    pub exit_code: i64,
    /// The script's own words. The client adds no rule of its own.
    pub output: String,
    /// The script's JSON, when it has any.
    pub data: Option<serde_json::Value>,
}

/// The estimate for one run of a kind.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CostEstimate {
    pub kind: String,
    pub count: usize,
    pub tokens: Option<f64>,
    pub usd: Option<f64>,
    pub samples: usize,
    pub text: String,
    pub requires_confirmation: bool,
    pub confirmation: Option<String>,
}

/// One spend window: the client's own runs inside it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UsageWindow {
    pub name: String,
    pub hours: i64,
    pub runs: usize,
    pub tokens: f64,
    pub usd: f64,
}

/// The client's own spend, in windows.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Usage {
    pub windows: Vec<UsageWindow>,
    pub computed_at: f64,
    pub runs: usize,
}

/// A run this client recorded.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RunRecord {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub status: String,
    pub tokens: f64,
    pub usd: f64,
    pub started_at: f64,
}

/// What the follow-up seeder did.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FollowupSeed {
    pub ok: bool,
    pub seeded: bool,
    pub app_num: Option<i64>,
    pub next_date: Option<String>,
    pub pin: Option<String>,
    pub reason: Option<String>,
}

/// A document the client generated.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Document {
    pub filename: String,
    pub kind: String,
    pub date: Option<String>,
    pub report_id: Option<String>,
    pub format: Option<String>,
    pub id: String,
}

/// A posting's liveness verdict.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Liveness {
    pub ok: bool,
    pub verdict: String,
    pub output: String,
}

/// A match found by the free-text search.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchMatch {
    pub tracker_num: String,
    pub company: String,
    pub role: String,
    pub score: String,
    pub status: String,
    pub report_num: Option<String>,
}

/// The result of a free-text search across the tracker.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchResults {
    pub query: String,
    pub found: usize,
    pub matches: Vec<SearchMatch>,
}

/// One company of the watched boards.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Portal {
    pub name: String,
    pub provider: String,
    pub careers_url: String,
    pub verdict: String,
    pub count: Option<i64>,
    pub detail: Option<String>,
    pub samples: Vec<String>,
}

/// One posting the scan history knows and the user has not evaluated.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Offer {
    pub url: String,
    pub company: String,
    pub title: String,
    pub location: String,
    pub posted_at: String,
    pub ats: String,
}

/// What is new this week.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WhatsNew {
    pub available: bool,
    pub count: usize,
    pub days: i64,
    pub offers: Vec<Offer>,
}

/// A story in the bank, with its provenance verdict.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StoryProvenance {
    pub story: String,
    pub claim: String,
    pub pattern: Option<String>,
    pub reason: Option<String>,
}

/// The provenance of every claim in the story bank.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StoryBank {
    pub available: bool,
    pub exists: bool,
    pub text: String,
    pub existing: Vec<StoryProvenance>,
    pub supported_by_resume: Vec<StoryProvenance>,
    pub derived_unverified: Vec<StoryProvenance>,
    pub user_cannot_confirm: Vec<StoryProvenance>,
}

/// A contact in the phonebook.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Contact {
    pub name: String,
    pub company: String,
    pub r#type: String,
    pub title: String,
    pub phone: String,
    pub email: String,
    pub linkedin: String,
    pub notes: String,
}

/// The phonebook.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Contacts {
    pub available: bool,
    pub total: usize,
    pub contacts: Vec<Contact>,
}

/// One candidate an invite could match.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InviteCandidate {
    pub app_number: Option<i64>,
    pub company: String,
    pub role: String,
    pub status: String,
    pub match_confidence: Option<String>,
}

/// What the invite matcher found.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InviteMatch {
    pub available: bool,
    pub classification: Option<String>,
    pub candidates: Vec<InviteCandidate>,
    pub output: String,
}

/// A plugin the core can load.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub hooks: Vec<String>,
    pub missing_env: Vec<String>,
}

/// What is running: the client, the bridge protocol and the engine release.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Version {
    /// The bridge protocol version.
    pub protocol: i64,
    /// The career-ops release the engine checkout carries.
    pub engine: String,
    /// The node runtime the bridge runs on.
    pub node: String,
}

/// What the CV's structure check found.
#[derive(Debug, Clone, Default)]
pub struct CvCheck {
    pub verdict: String,
    pub violations: Vec<String>,
}

/// One analysis result, as the screen shows it.
#[derive(Debug, Clone, Default)]
pub struct Analysis {
    pub action: String,
    pub title: String,
    pub body: String,
}

/// One event of a running job, in the web app's NDJSON shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
// `rename_all` renames the VARIANTS; the fields inside each variant need
// `rename_all_fields`, or the bridge's `reportN`/`costUsd` never reach
// `report_n`/`cost_usd` and a finished run reports no report number.
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RunEvent {
    Text { text: String },
    Tool { name: String },
    Status { label: String },
    Keepalive,
    Error { msg: String },
    Done {
        #[serde(default)]
        tokens: Option<u64>,
        #[serde(default)]
        cost_usd: Option<f64>,
        #[serde(default)]
        report_n: Option<String>,
    },
    #[serde(other)]
    Unknown,
}

impl RunEvent {
    /// A short line for the run log.
    pub fn describe(&self) -> String {
        match self {
            RunEvent::Text { text } => text.trim_end().to_string(),
            RunEvent::Tool { name } => format!("🔧 {name}"),
            RunEvent::Status { label } => label.clone(),
            RunEvent::Keepalive => String::new(),
            RunEvent::Error { msg } => format!("⚠ {msg}"),
            RunEvent::Done { tokens, cost_usd, report_n } => {
                let mut parts = Vec::new();
                if let Some(n) = report_n {
                    parts.push(format!("report #{n}"));
                }
                if let Some(t) = tokens {
                    parts.push(format!("{t} tokens"));
                }
                if let Some(c) = cost_usd {
                    parts.push(format!("${c:.4}"));
                }
                if parts.is_empty() {
                    "done".to_string()
                } else {
                    parts.join(" · ")
                }
            }
            RunEvent::Unknown => String::new(),
        }
    }

    /// True when this event ends the run.
    pub fn is_terminal(&self) -> bool {
        matches!(self, RunEvent::Done { .. } | RunEvent::Error { .. })
    }
}
