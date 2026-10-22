//! The bridge's JSON, decoded as it actually arrives.
//!
//! The screen tests use hand-written fixtures, which is right for asserting
//! what a screen shows but wrong for discovering what the engine actually
//! sends: a hand-written fixture drifts toward the shape the Rust types expect.
//! This suite is the counterweight. Its payloads are copied from a real bridge
//! response, nulls and all, so a model that cannot decode the real thing fails
//! here instead of in the window.
//!
//! The bug this exists for: `Cli.path` was typed `String`, but the engine sends
//! `null` for a CLI that is not installed. The app launched, could not decode
//! `bootstrap`, and showed "could not read the install state: invalid type:
//! null, expected a string" with an empty window. No screen test caught it,
//! because every fixture said `""`.

use career_ops_desktop::model::{
    Bootstrap, Config, Cv, FollowupEntry, Followups, Pipeline, Report, RunEvent, StatusResult,
};

/// A real `bootstrap` payload: the installed CLIs carry a path, the rest carry
/// null. An install with every CLI present would never exercise the null.
const BOOTSTRAP: &str = r##"{
  "dataRoot": "/home/candidate/career-ops",
  "codeRoot": "/home/candidate/career-ops",
  "doctor": { "available": true, "onboardingNeeded": false, "missing": [], "warnings": [],
              "unpersonalized": [{ "path": "modes/_profile.md", "reason": "r", "impact": "i" }] },
  "doctorState": { "phase": "established", "onboardingNeeded": false, "missing": [], "hasCv": true, "hasData": true },
  "clis": [
    { "id": "claude", "name": "Claude Code", "run": "claude", "url": "https://claude.ai", "installed": true, "path": "/usr/bin/claude" },
    { "id": "codex", "name": "Codex CLI", "run": "codex", "url": "https://openai.com", "installed": false, "path": null },
    { "id": "pi", "name": "Pi", "run": "pi", "url": "", "installed": true, "path": "/usr/bin/pi" }
  ],
  "states": ["Evaluated", "Applied", "Responded", "Assessment", "Interview", "Offer", "Hired", "Rejected", "Discarded", "SKIP"],
  "trackerExists": true,
  "inboxCount": 59,
  "applicationCount": 15,
  "reports": 15,
  "pdfs": 6,
  "followupsEnabled": true,
  "cvExists": true
}"##;

const PIPELINE: &str = r##"{
  "root": "/home/candidate/career-ops",
  "rootExists": true,
  "trackerPath": "/home/candidate/career-ops/data/applications.md",
  "inbox": [
    { "url": "https://example.com/job/1", "company": "Contoso", "role": "SRE", "done": false, "postedAt": null },
    { "url": "https://example.com/job/2", "company": "Fabrikam", "role": "DevOps", "done": true }
  ],
  "applications": [
    { "n": "1", "date": "2026-10-19", "company": "Leidos", "via": "", "role": "Cyber Defense Analyst",
      "location": "", "score": "2.5/5", "status": "SKIP", "pdf": "❌",
      "report": "[001](../reports/001-leidos-2026-10-19.md)", "notes": "n", "url": "", "applyLink": "", "followUp": "" }
  ],
  "fieldNames": ["n", "date", "company"]
}"##;

const REPORT: &str = r##"{
  "n": "2",
  "application": { "n": "2", "date": "2026-10-02", "company": "Dragos", "via": "", "role": "IT Admin",
    "location": "", "score": "4.6/5", "status": "Applied", "pdf": "✅", "report": "", "notes": "", "url": "", "applyLink": "", "followUp": "" },
  "file": "002-dragos-2026-10-02.md",
  "markdown": "# Evaluation: Dragos — IT Admin",
  "title": "Dragos — IT Admin",
  "legitimacy": "High Confidence",
  "fields": [{ "label": "Score", "value": "4.6/5" }],
  "postingUrl": "https://dragos.example/job/42",
  "trackerUrl": null,
  "cvPdf": null,
  "cvHtml": null,
  "coverPdf": null
}"##;

const FOLLOWUPS: &str = r##"{
  "available": true,
  "entries": [{ "num": 2, "date": "2026-10-02", "appliedDate": "2026-10-02", "appDateSource": "notes",
    "company": "Dragos", "via": null, "role": "IT Admin", "status": "Applied", "score": "4.6/5", "notes": "",
    "reportPath": "/x", "contacts": [], "daysSinceApplication": 19, "daysSinceLastFollowup": null,
    "followupCount": 0, "followups": [], "urgency": "overdue", "nextFollowupDate": "2026-10-10",
    "nextOverride": null, "daysUntilNext": -3 }],
  "due": [], "upcoming": [], "nextUpcoming": null,
  "history": [{ "num": 1, "appNum": 2, "date": "2026-10-12", "company": "Dragos", "role": "IT Admin",
    "channel": "Email", "contact": "", "notes": "n" },
    { "num": null, "appNum": 3, "date": "2026-10-13", "company": "Legacy", "role": "", "channel": "Other", "contact": "", "notes": "" }],
  "overrides": [{ "appNum": 2, "date": "2026-10-10" }],
  "followupsEnabled": true
}"##;

const CONFIG: &str = r##"{
  "profile": { "candidate": { "full_name": "Candidate", "email": "c@example.com", "location": "Remote" },
               "target_roles": { "primary": ["SRE"] }, "compensation": { "currency": "USD" } },
  "exists": true, "malformed": false,
  "clis": [{ "id": "claude", "name": "Claude Code", "run": "claude", "url": "", "installed": false, "path": null }],
  "states": ["Evaluated"],
  "codeRoot": "/home/candidate/career-ops",
  "dataRoot": "/home/candidate/career-ops"
}"##;

#[test]
fn the_real_bootstrap_payload_decodes() {
    let value: Bootstrap = serde_json::from_str(BOOTSTRAP).expect("bootstrap must decode");
    assert_eq!(value.application_count, 15);
    assert_eq!(value.inbox_count, 59);
    assert_eq!(value.states.len(), 10);
    // The null path is the one that broke the app: an install missing a CLI
    // could not open at all.
    let codex = value.clis.iter().find(|c| c.id == "codex").expect("codex row");
    assert!(!codex.installed);
    assert_eq!(codex.path, None);
    let claude = value.clis.iter().find(|c| c.id == "claude").expect("claude row");
    assert_eq!(claude.path.as_deref(), Some("/usr/bin/claude"));
}

#[test]
fn the_real_pipeline_payload_decodes() {
    let value: Pipeline = serde_json::from_str(PIPELINE).expect("pipeline must decode");
    assert_eq!(value.applications.len(), 1);
    assert_eq!(value.applications[0].score_value(), 2.5);
    assert_eq!(value.inbox.len(), 2);
    // A posting with no recorded date, and one with no `postedAt` key at all.
    assert_eq!(value.inbox[0].posted_at, None);
    assert_eq!(value.inbox[1].posted_at, None);
    assert!(value.inbox[1].done);
}

#[test]
fn the_real_report_payload_decodes() {
    let value: Report = serde_json::from_str(REPORT).expect("report must decode");
    assert_eq!(value.posting_url.as_deref(), Some("https://dragos.example/job/42"));
    assert_eq!(value.cv_pdf, None, "a report with no tailored CV has null, not an empty string");
    assert_eq!(value.legitimacy.as_deref(), Some("High Confidence"));
}

#[test]
fn the_real_followups_payload_decodes() {
    let value: Followups = serde_json::from_str(FOLLOWUPS).expect("followups must decode");
    assert!(value.available);
    assert_eq!(value.entries.len(), 1);
    let entry: &FollowupEntry = &value.entries[0];
    assert_eq!(entry.days_until_next, Some(-3));
    assert_eq!(entry.days_since_last_followup, None);
    // A legacy bullet row in the log carries no num.
    assert_eq!(value.history.len(), 2);
    assert_eq!(value.history[1].num, None);
}

#[test]
fn the_real_config_payload_decodes() {
    let value: Config = serde_json::from_str(CONFIG).expect("config must decode");
    assert_eq!(value.clis[0].path, None);
    assert!(!value.malformed);
    assert_eq!(
        value.profile["candidate"]["full_name"].as_str(),
        Some("Candidate")
    );
}

#[test]
fn a_cv_that_does_not_exist_yet_decodes() {
    // The first run on a fresh install: read returns an empty document and no
    // backup.
    let value: Cv = serde_json::from_str(r##"{ "content": "", "exists": false, "path": "/x/cv.md" }"##)
        .expect("cv must decode");
    assert!(!value.exists);
    assert!(value.content.is_empty());
}

#[test]
fn a_status_result_decodes() {
    let value: StatusResult = serde_json::from_str(
        r##"{ "status": "Applied", "changed": true, "statusLogged": true }"##,
    )
    .expect("status must decode");
    assert!(value.changed);
    assert!(value.status_logged);
}

#[test]
fn every_run_event_the_bridge_emits_decodes() {
    // The vocabulary is the web app's NDJSON stream. A new event type arrives
    // as Unknown rather than as a decode error, so an older client degrades.
    let events: Vec<RunEvent> = [
        r##"{"type":"text","text":"hello"}"##,
        r##"{"type":"tool","name":"WebFetch"}"##,
        r##"{"type":"status","label":"Saving report…"}"##,
        r##"{"type":"keepalive"}"##,
        r##"{"type":"error","msg":"boom"}"##,
        r##"{"type":"done","tokens":1200,"costUsd":0.0123,"reportN":"42"}"##,
        r##"{"type":"done"}"##,
        r##"{"type":"something-new-in-a-later-release"}"##,
    ]
    .iter()
    .map(|raw| serde_json::from_str(raw).unwrap_or_else(|e| panic!("{raw}: {e}")))
    .collect();

    assert!(matches!(events[0], RunEvent::Text { .. }));
    assert!(matches!(events[1], RunEvent::Tool { .. }));
    assert!(matches!(events[2], RunEvent::Status { .. }));
    assert!(matches!(events[3], RunEvent::Keepalive));
    assert!(matches!(events[4], RunEvent::Error { .. }));
    assert!(matches!(&events[5], RunEvent::Done { report_n: Some(n), .. } if n == "42"));
    assert!(matches!(&events[6], RunEvent::Done { report_n: None, .. }));
    assert!(matches!(events[7], RunEvent::Unknown));
    assert!(events[5].is_terminal());
    assert!(!events[0].is_terminal());
}
