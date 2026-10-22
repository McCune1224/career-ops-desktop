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
