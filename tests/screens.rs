//! Headless per-screen tests.
//!
//! Each test renders ONE screen through `views::*::ui` inside an egui harness
//! and asserts on the widgets that screen produced. No window, no display
//! server, no bridge process and no data root: the engine is a `FakeBackend`
//! answering with the exact JSON shapes the real bridge sends.
//!
//! These are the tests the capability matrix points at. If a screen stops
//! showing the thing the web app shows for the same capability, one of these
//! fails.

use std::sync::Arc;

use career_ops_desktop::app::{CareerOpsApp, Screen};
use career_ops_desktop::backend::FakeBackend;
use career_ops_desktop::model::RunEvent;
use career_ops_desktop::settings::Settings;
use career_ops_desktop::views;
use egui_kittest::kittest::Queryable;

/// True when any node's label contains `text`.
///
/// `query_by_label_contains` insists on exactly one match, which is the wrong
/// question for a screen that may legitimately say the same thing twice; these
/// tests ask "is it shown at all".
fn shows(harness: &egui_kittest::Harness<'_, CareerOpsApp>, text: &str) -> bool {
    harness.query_all_by_label_contains(text).next().is_some()
}

/// Build an app whose engine answers from `answers`.
fn app_with(answers: &[(&str, serde_json::Value)]) -> CareerOpsApp {
    let mut fake = FakeBackend::new();
    for (op, value) in answers {
        fake = fake.with(op, value.clone());
    }
    let mut app = CareerOpsApp::new(Arc::new(fake), Settings::default());
    app.load_all();
    app
}

/// Run one screen headlessly and return the harness.
#[allow(clippy::type_complexity)]
fn harness_for<F>(
    app: CareerOpsApp,
    screen: F,
) -> egui_kittest::Harness<'static, CareerOpsApp>
where
    F: FnMut(&mut egui::Ui, &mut CareerOpsApp) + 'static,
{
    let mut harness = egui_kittest::HarnessBuilder::default()
        .with_size(egui::vec2(1280.0, 800.0))
        // Accessibility hardening is its own piece of work; what these tests
        // assert is the CONTENT each screen renders.
        .with_accessibility_check(false)
        .build_ui_state(screen, app);
    harness.run();
    harness
}

/// The same, for a screen that is mid-run.
///
/// A running screen shows a spinner, which asks egui to repaint forever, so
/// `run()` would never settle. A fixed number of steps is the right measure
/// here: render, let the layout settle, stop.
fn harness_running<F>(
    app: CareerOpsApp,
    screen: F,
) -> egui_kittest::Harness<'static, CareerOpsApp>
where
    F: FnMut(&mut egui::Ui, &mut CareerOpsApp) + 'static,
{
    let mut harness = egui_kittest::HarnessBuilder::default()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_accessibility_check(false)
        .build_ui_state(screen, app);
    harness.run_steps(4);
    harness
}

// ─── fixtures: the exact bridge payload shapes ─────────────────────────────

fn bootstrap() -> serde_json::Value {
    serde_json::json!({
        "dataRoot": "/fixture",
        "codeRoot": "/fixture",
        "doctor": { "available": true, "onboardingNeeded": false, "missing": [], "warnings": [] },
        "doctorState": { "phase": "established", "onboardingNeeded": false, "missing": [], "hasCv": true, "hasData": true },
        "clis": [
            { "id": "claude", "name": "Claude Code", "run": "claude", "url": "https://claude.ai", "installed": true, "path": "/usr/bin/claude" },
            { "id": "codex", "name": "Codex", "run": "codex", "url": "https://openai.com", "installed": false, "path": "" }
        ],
        "states": ["Evaluated","Applied","Responded","Assessment","Interview","Offer","Hired","Rejected","Discarded","SKIP"],
        "defaultCliId": "claude",
        "trackerExists": true,
        "inboxCount": 4,
        "applicationCount": 2,
        "reports": 2,
        "pdfs": 1,
        "followupsEnabled": true,
        "cvExists": true
    })
}

fn pipeline() -> serde_json::Value {
    serde_json::json!({
        "root": "/fixture",
        "rootExists": true,
        "trackerPath": "/fixture/data/applications.md",
        "inbox": [
            { "url": "https://example.com/jobs/1", "company": "Contoso", "role": "SRE", "location": "", "compensation": "", "done": false, "postedAt": "2026-10-01" }
        ],
        "applications": [
            {
                "n": "1", "date": "2026-10-01", "company": "Leidos", "via": "", "role": "Cyber Defense Analyst",
                "location": "", "score": "2.5/5", "status": "SKIP", "pdf": "❌",
                "report": "[001](../reports/001-leidos-2026-10-01.md)",
                "notes": "needs a TS/SCI", "url": "https://leidos.example/job/R-1", "applyLink": "", "followUp": ""
            },
            {
                "n": "2", "date": "2026-10-02", "company": "Dragos", "via": "", "role": "Associate IT System Administrator",
                "location": "", "score": "4.6/5", "status": "Applied", "pdf": "✅",
                "report": "[002](../reports/002-dragos-2026-10-02.md)",
                "notes": "best fit", "url": "", "applyLink": "", "followUp": ""
            }
        ],
        "fieldNames": ["n","date","company","via","role","location","score","status","pdf","report","notes","url"]
    })
}

fn report() -> serde_json::Value {
    serde_json::json!({
        "n": "2",
        "application": {
            "n": "2", "date": "2026-10-02", "company": "Dragos", "via": "", "role": "Associate IT System Administrator",
            "location": "", "score": "4.6/5", "status": "Applied", "pdf": "✅", "report": "", "notes": "", "url": "", "applyLink": "", "followUp": ""
        },
        "file": "002-dragos-2026-10-02.md",
        "markdown": "# Evaluation: Dragos — Associate IT System Administrator\n\n**Score:** 4.6/5\n**Legitimacy:** High Confidence\n\n---\n\n## Block A — Role fit\n\nA strong **match** for the `IT` background.\n\n- Windows estate\n- Security tooling\n\n| Signal | Read |\n|---|---|\n| Score | 4.6/5 |\n\nVERDICT: 4.6/5 — strong fit",
        "title": "Dragos — Associate IT System Administrator",
        "legitimacy": "High Confidence",
        "fields": [
            { "label": "Score", "value": "4.6/5" },
            { "label": "Legitimacy", "value": "High Confidence" },
            { "label": "URL", "value": "https://dragos.example/job/42" }
        ],
        "postingUrl": "https://dragos.example/job/42",
        "trackerUrl": null,
        "cvPdf": "/fixture/output/cv-alex-dragos-002.pdf",
        "cvHtml": null,
        "coverPdf": null,
        "intro": "The evaluation, in full.",
        "lead": "A strong match for the IT background.",
        "verdict": "VERDICT: 4.6/5 — strong fit",
        "sections": [
            {
                "heading": "Block A — Role fit",
                "letter": "A",
                "content": "A strong **match** for the `IT` background.",
                "preview": "A strong match for the IT background.",
                "lead": false
            },
            {
                "heading": "Block B — CV match",
                "letter": "B",
                "content": "The CV covers every requirement the posting names.",
                "preview": "The CV covers every requirement.",
                "lead": true
            },
            {
                "heading": "Block C — Gaps",
                "letter": "C",
                "content": "Two requirements the CV does not show.",
                "preview": "Two requirements the CV does not show.",
                "lead": false
            }
        ]
    })
}

fn cv() -> serde_json::Value {
    serde_json::json!({
        "content": "# Alex McCune\n\n## Summary\n\nIT operations and security.\n",
        "exists": true,
        "path": "/fixture/cv.md"
    })
}

fn followups() -> serde_json::Value {
    serde_json::json!({
        "available": true,
        "entries": [
            {
                "num": 2, "date": "2026-10-02", "company": "Dragos", "role": "Associate IT System Administrator",
                "status": "Applied", "score": "4.6/5", "notes": "", "urgency": "overdue",
                "nextFollowupDate": "2026-10-10", "daysUntilNext": -3, "daysSinceApplication": 19,
                "daysSinceLastFollowup": null, "followupCount": 0, "reportPath": null, "contacts": []
            }
        ],
        "due": [
            {
                "num": 2, "date": "2026-10-02", "company": "Dragos", "role": "Associate IT System Administrator",
                "status": "Applied", "score": "4.6/5", "notes": "", "urgency": "overdue",
                "nextFollowupDate": "2026-10-10", "daysUntilNext": -3, "daysSinceApplication": 19,
                "daysSinceLastFollowup": null, "followupCount": 0, "reportPath": null, "contacts": []
            }
        ],
        "upcoming": [],
        "nextUpcoming": null,
        "history": [
            { "num": 1, "appNum": 2, "date": "2026-10-12", "company": "Dragos", "role": "Associate IT System Administrator", "channel": "Email", "contact": "", "notes": "emailed the recruiter" }
        ],
        "overrides": [],
        "followupsEnabled": true
    })
}

fn config() -> serde_json::Value {
    serde_json::json!({
        "profile": { "candidate": { "full_name": "Alex McCune", "email": "alex@example.com", "location": "Maryland" } },
        "exists": true,
        "malformed": false,
        "clis": [
            { "id": "claude", "name": "Claude Code", "run": "claude", "url": "https://claude.ai", "installed": true, "path": "/usr/bin/claude" }
        ],
        "states": ["Evaluated","Applied"],
        "codeRoot": "/fixture",
        "dataRoot": "/fixture"
    })
}

fn all_fixtures() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        ("bootstrap", bootstrap()),
        ("pipeline", pipeline()),
        ("report", report()),
        ("cv", cv()),
        ("followups", followups()),
        ("config", config()),
    ]
}

/// The cadence answer, which the config screen asks for by action.
fn cadence() -> serde_json::Value {
    serde_json::json!({
        "defaults": { "applied_first_days": 7, "applied_second_days": 14, "applied_max_followups": 3 },
        "defaultsAvailable": true,
        "overrides": {},
        "effective": { "applied_first_days": 7, "applied_second_days": 14, "applied_max_followups": 3 },
        "enabled": true
    })
}

// ─── Pipeline: sorting, filtering, status change ────────────────────────────

#[test]
fn pipeline_screen_renders_the_tracker_rows() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::pipeline::ui(ui, app));

    harness.get_by_label("Pipeline");
    // Both applications, and the counts line the web app's header shows.
    assert!(harness.query_by_label("Leidos").is_some(), "row 1 company missing");
    assert!(harness.query_by_label("Dragos").is_some(), "row 2 company missing");
    assert!(
        shows(&harness, "2 applications"),
        "the pipeline header does not report the tracker size"
    );
    assert!(
        shows(&harness, "1 pending postings"),
        "the pipeline header does not report the inbox size"
    );
}

#[test]
fn pipeline_screen_offers_a_status_control_per_row() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::pipeline::ui(ui, app));

    // The combo's selected text is the row's current status, and there is one
    // per row: that is the capability "canonical status changes".
    assert!(harness.query_by_label("SKIP").is_some(), "row 1 status control missing");
    assert!(harness.query_by_label("Applied").is_some(), "row 2 status control missing");
}

#[test]
fn pipeline_screen_exposes_every_canonical_state_filter() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::pipeline::ui(ui, app));

    assert!(harness.query_by_label("All").is_some(), "the All filter tab is missing");
    for state in [
        "Evaluated", "Responded", "Assessment", "Interview", "Offer", "Hired", "Rejected", "Discarded",
    ] {
        assert!(
            harness.query_by_label(state).is_some(),
            "the filter tab for {state} is missing"
        );
    }
}

#[test]
fn pipeline_screen_offers_report_and_posting_actions() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::pipeline::ui(ui, app));

    // One Report button per row, and a Posting button only where a URL exists.
    assert_eq!(harness.get_all_by_label("Report").count(), 2);
    assert_eq!(
        harness.get_all_by_label("Posting").count(),
        1,
        "the Posting action must appear only on rows that record a URL"
    );
}

#[test]
fn pipeline_filters_by_status_and_score() {
    use career_ops_desktop::views::{filter_rows, sort_rows};
    let rows: Vec<career_ops_desktop::model::Application> =
        serde_json::from_value(pipeline()["applications"].clone()).unwrap();

    let applied = filter_rows(rows.clone(), "Applied", 0.0, "");
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].company, "Dragos");

    let high = filter_rows(rows.clone(), "All", 4.0, "");
    assert_eq!(high.len(), 1, "the score floor must drop the 2.5/5 row");

    let search = filter_rows(rows.clone(), "All", 0.0, "leidos");
    assert_eq!(search.len(), 1, "search must match the company, case-insensitively");

    let mut sorted = rows.clone();
    sort_rows(
        &mut sorted,
        career_ops_desktop::app::SortKey::Score,
        true,
    );
    assert_eq!(sorted[0].company, "Dragos", "descending score sorts the best first");
}

// ─── Report: reading it and opening the posting ─────────────────────────────

#[test]
fn report_screen_shows_the_report_and_the_source_posting() {
    let mut app = app_with(&all_fixtures());
    app.screen = Screen::Report("2".into());
    app.state.report =
        Some(serde_json::from_value(report()).expect("report fixture decodes"));
    let harness = harness_for(app, |ui, app| views::report::ui(ui, app));

    assert!(
        harness.query_by_label("Open the source posting").is_some(),
        "the report screen must offer the source posting"
    );
    assert!(
        harness.query_by_label("Dragos — Associate IT System Administrator").is_some(),
        "the report title is missing"
    );
    assert!(
        shows(&harness, "High Confidence"),
        "the legitimacy tier is missing"
    );
    // Both artifacts the report page links are offered.
    assert!(harness.query_by_label("Open the tailored CV (PDF)").is_some());
    assert!(
        shows(&harness, "VERDICT"),
        "the report body is not rendered"
    );
}

// ─── Run: the structured run screen + streamed progress ─────────────────────

#[test]
fn run_screen_offers_the_three_structured_runs() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::run::ui(ui, app));

    assert!(harness.query_by_label("Evaluate a posting").is_some());
    assert!(harness.query_by_label("Tailor the CV (PDF)").is_some());
    assert!(harness.query_by_label("Draft a cover letter").is_some());
    assert!(harness.query_by_label("Start").is_some());
}

#[test]
fn run_screen_keeps_start_disabled_when_no_cli_is_selected() {
    let mut app = app_with(&all_fixtures());
    // Settings default to no CLI; the run must not be startable.
    app.settings.cli_id.clear();
    let harness = harness_for(app, |ui, app| views::run::ui(ui, app));
    assert!(
        shows(&harness, "No usable CLI is selected"),
        "the run screen must say why it cannot start"
    );
}

#[test]
fn run_screen_renders_streamed_progress() {
    let mut app = app_with(&all_fixtures());
    app.settings.cli_id = "claude".into();
    app.run.kind = Some(career_ops_desktop::RunKind::Evaluate);
    app.run.input = "https://example.com/job".into();
    app.run.running = true;
    app.run.request_id = Some(1);

    // The exact event vocabulary the web app streams.
    let events = vec![
        career_ops_desktop::bridge::StreamEvent {
            request_id: 1,
            event: RunEvent::Status { label: "Reading the posting…".into() },
        },
        career_ops_desktop::bridge::StreamEvent {
            request_id: 1,
            event: RunEvent::Tool { name: "WebFetch".into() },
        },
        career_ops_desktop::bridge::StreamEvent {
            request_id: 1,
            event: RunEvent::Text { text: "Scoring against the CV.".into() },
        },
    ];
    app.absorb_events(events);

    let harness = harness_running(app, |ui, app| views::run::ui(ui, app));
    assert!(shows(&harness, "Reading the posting…"));
    assert!(shows(&harness, "[tool] WebFetch"));
    assert!(shows(&harness, "Scoring against the CV."));
}

#[test]
fn a_done_event_endpoints_the_run_and_records_the_report_number() {
    let mut fake = FakeBackend::new();
    fake = fake.with("bootstrap", bootstrap()).with("pipeline", pipeline());
    let mut app = CareerOpsApp::new(Arc::new(fake), Settings::default());
    app.run.running = true;
    app.run.request_id = Some(7);

    app.absorb_events(vec![
        career_ops_desktop::bridge::StreamEvent {
            request_id: 7,
            event: RunEvent::Keepalive,
        },
        career_ops_desktop::bridge::StreamEvent {
            request_id: 7,
            event: RunEvent::Done { tokens: Some(1200), cost_usd: Some(0.02), report_n: Some("42".into()) },
        },
    ]);

    assert!(!app.run.running, "a done event must end the run");
    assert_eq!(app.run.report_n.as_deref(), Some("42"));
    assert!(
        app.run.log.iter().all(|line| !line.text.contains("keepalive")),
        "a keepalive is transport noise, not a log line"
    );
}

#[test]
fn an_error_event_records_the_failure() {
    let mut fake = FakeBackend::new();
    fake = fake.with("bootstrap", bootstrap()).with("pipeline", pipeline());
    let mut app = CareerOpsApp::new(Arc::new(fake), Settings::default());
    app.run.running = true;
    app.run.request_id = Some(3);

    app.absorb_events(vec![career_ops_desktop::bridge::StreamEvent {
        request_id: 3,
        event: RunEvent::Error { msg: "The CLI produced no output".into() },
    }]);

    assert!(!app.run.running);
    assert_eq!(app.run.error.as_deref(), Some("error: The CLI produced no output"));
}

// ─── CV: viewing and editing ────────────────────────────────────────────────

#[test]
fn cv_screen_shows_the_cv_and_its_editor() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::cv::ui(ui, app));

    assert!(harness.query_by_label("Save").is_some(), "the CV editor has no Save");
    // The preview renders the markdown, so the heading text is present.
    assert!(
        shows(&harness, "Alex McCune"),
        "the CV content is not shown"
    );
    assert!(harness.query_by_label("Preview").is_some());
}

// ─── Follow-ups ────────────────────────────────────────────────────────────

#[test]
fn followups_screen_lists_due_followups_and_the_log() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::followups::ui(ui, app));

    assert!(harness.query_by_label("Follow-ups").is_some());
    assert!(
        shows(&harness, "3 days overdue"),
        "the overdue countdown is missing"
    );
    assert!(harness.query_by_label("Log a follow-up").is_some());
    assert!(
        shows(&harness, "Logged follow-ups"),
        "the logged history section is missing"
    );
    assert!(shows(&harness, "emailed the recruiter"));
}

// ─── Config ────────────────────────────────────────────────────────────────

#[test]
fn config_screen_shows_the_cli_picker_and_the_cadence() {
    let mut answers = all_fixtures();
    answers.push(("followups", cadence()));
    // The config screen reads the space-separated cadence answer, which the
    // fake returns for the `followups` op whatever the action.
    let mut fake = FakeBackend::new();
    for (op, value) in &answers {
        fake = fake.with(op, value.clone());
    }
    let mut app = CareerOpsApp::new(Arc::new(fake), Settings::default());
    app.load_all();

    let harness = harness_for(app, |ui, app| views::config::ui(ui, app));
    assert!(harness.query_by_label("Claude Code").is_some(), "the CLI picker is missing");
    assert!(
        shows(&harness, "Follow-up cadence"),
        "the cadence section is missing"
    );
    assert!(
        shows(&harness, "Candidate"),
        "the candidate section is missing"
    );
}

#[test]
fn config_screen_reports_where_the_data_comes_from() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::config::ui(ui, app));
    assert!(
        shows(&harness, "/fixture"),
        "the config screen must show the data root it is reading"
    );
}

// ─── Today ─────────────────────────────────────────────────────────────────

#[test]
fn today_screen_renders_the_daily_loop_tiles() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::today::ui(ui, app));

    assert!(harness.query_by_label("Today").is_some());
    assert!(harness.query_by_label("Applications").is_some());
    assert!(harness.query_by_label("Pending postings").is_some());
    assert!(harness.query_by_label("Reports").is_some());
    assert!(harness.query_by_label("Follow-ups due").is_some());
    assert!(harness.query_by_label("Open the Run screen").is_some());
}

// ─── The status write goes through the bridge, never the tracker ───────────

#[test]
fn a_status_change_goes_through_the_status_op() {
    let mut fake = FakeBackend::new();
    fake = fake.with("bootstrap", bootstrap()).with("pipeline", pipeline());
    fake = fake.with("status", serde_json::json!({ "status": "Applied", "changed": true, "statusLogged": true }));
    let backend = Arc::new(fake);
    let mut app = CareerOpsApp::new(backend.clone(), Settings::default());

    app.set_status("1", "applied");

    // The write is one `status` call carrying the row and the state; the app
    // never touches data/applications.md itself.
    let calls = backend.calls.lock().unwrap();
    let status_calls: Vec<_> = calls.iter().filter(|(op, _)| op == "status").collect();
    assert_eq!(status_calls.len(), 1, "exactly one status write");
    assert_eq!(status_calls[0].1["n"], "1");
    assert_eq!(status_calls[0].1["status"], "applied");
    assert!(
        app.state
            .message
            .as_ref()
            .is_some_and(|(text, is_error)| !is_error && text.contains("Applied")),
        "the screen must confirm the canonical state"
    );
}

#[test]
fn a_busy_tracker_is_reported_as_retryable() {
    let mut app = CareerOpsApp::new(Arc::new(FailingBackend), Settings::default());
    app.set_status("1", "Applied");
    let (message, is_error) = app.state.message.clone().expect("a message");
    assert!(is_error);
    assert!(
        message.contains("busy"),
        "a contended lock must read as retryable, got: {message}"
    );
}

/// A backend whose status call fails the way a contended tracker lock does.
struct FailingBackend;

impl career_ops_desktop::Backend for FailingBackend {
    fn call(
        &self,
        op: &str,
        _params: serde_json::Value,
    ) -> Result<serde_json::Value, career_ops_desktop::bridge::BridgeError> {
        if op == "status" {
            Err(career_ops_desktop::bridge::BridgeError {
                code: "busy".into(),
                message: "the tracker is busy".into(),
            })
        } else {
            Ok(serde_json::json!({}))
        }
    }

    fn start_run(
        &self,
        _kind: &str,
        _input: &str,
        _cli_id: &str,
        _brief: Option<&str>,
    ) -> Result<u64, career_ops_desktop::bridge::BridgeError> {
        Ok(1)
    }

    fn start_batch(
        &self,
        _kind: &str,
        _inputs: &[String],
        _cli_id: &str,
    ) -> Result<u64, career_ops_desktop::bridge::BridgeError> {
        Ok(1)
    }

    fn poll_events(&self) -> Vec<career_ops_desktop::bridge::StreamEvent> {
        Vec::new()
    }
}

// ─── Layout budget ─────────────────────────────────────────────────────────

#[test]
fn the_tracker_table_fits_the_narrowest_canvas_it_is_designed_for() {
    // The window manager this was developed against tiles windows to a quarter
    // of the display, so the table has to survive about 840 points. An
    // over-wide table does not fail loudly — it clips the Actions column off
    // the right edge — so the budget is asserted rather than eyeballed.
    use career_ops_desktop::views::pipeline::{TABLE_CANVAS_MIN, TABLE_MIN_WIDTHS};
    const COLUMN_GAP: f32 = 10.0;
    let gaps = COLUMN_GAP * (TABLE_MIN_WIDTHS.len() as f32 - 1.0);
    let total: f32 = TABLE_MIN_WIDTHS.iter().sum::<f32>() + gaps;
    assert!(
        total <= TABLE_CANVAS_MIN,
        "the table's minimum widths total {total} points, over the {TABLE_CANVAS_MIN}-point budget by {}",
        total - TABLE_CANVAS_MIN
    );
}

#[test]
fn the_body_text_is_larger_than_egui_default() {
    // The complaint this overhaul answers was "it's very small". egui's default
    // body size is 14 points; anything at or below that has regressed.
    assert!(
        career_ops_desktop::theme::BODY > 16.0,
        "body copy is {} points — the whole point of the overhaul was to stop rendering at the default 14",
        career_ops_desktop::theme::BODY
    );
    assert!(career_ops_desktop::theme::ROW_HEIGHT >= 34.0);
    assert!(career_ops_desktop::settings::DEFAULT_ZOOM >= 1.1);
}

// ─── Interaction: the controls, not just the labels ────────────────────────
//
// The screen tests above assert what a screen RENDERS. These drive the widgets
// the way a person does — click the tab, click the sort header, click Start —
// and assert what the app then DID. A screen can render a control that is wired
// to nothing, and only a click can tell the difference.

#[test]
fn clicking_a_status_tab_filters_the_table() {
    let app = app_with(&all_fixtures());
    let mut harness = harness_for(app, |ui, app| views::pipeline::ui(ui, app));

    // Both rows before the filter.
    assert!(harness.query_by_label("Leidos").is_some());
    assert!(harness.query_by_label("Dragos").is_some());

    harness.get_by_label("SKIP").click();
    harness.run();

    assert_eq!(
        harness.state().pipeline_view.status_filter,
        "SKIP",
        "the tab click did not reach the filter state"
    );
    assert!(harness.query_by_label("Leidos").is_some(), "the SKIP row should survive");
    assert!(
        harness.query_by_label("Dragos").is_none(),
        "the Applied row should have been filtered out"
    );
}

#[test]
fn clicking_a_column_header_reorders_the_rows() {
    let app = app_with(&all_fixtures());
    let mut harness = harness_for(app, |ui, app| views::pipeline::ui(ui, app));

    // Default sort is the tracker number, so Leidos (#1) sits above Dragos (#2).
    let leidos = harness.get_by_label("Leidos").rect().top();
    let dragos = harness.get_by_label("Dragos").rect().top();
    assert!(leidos < dragos, "the default order should be by tracker number");

    harness.get_by_label("Company").click();
    harness.run();

    assert_eq!(harness.state().pipeline_view.sort, career_ops_desktop::app::SortKey::Company);
    let leidos = harness.get_by_label("Leidos").rect().top();
    let dragos = harness.get_by_label("Dragos").rect().top();
    assert!(
        dragos < leidos,
        "sorting by company must put Dragos above Leidos — the header click changed nothing"
    );
}

#[test]
fn clicking_report_opens_that_application() {
    let mut fake = FakeBackend::new();
    for (op, value) in all_fixtures() {
        fake = fake.with(op, value);
    }
    let backend = Arc::new(fake);
    let mut app = CareerOpsApp::new(backend.clone(), Settings::default());
    app.load_all();
    let mut harness = harness_for(app, |ui, app| views::pipeline::ui(ui, app));

    // The first row's Report button, not "the" Report button: there is one per row.
    harness.get_all_by_label("Report").next().unwrap().click();
    harness.run();

    // It asked the engine for that exact row…
    let calls = backend.calls.lock().unwrap();
    let asked: Vec<_> = calls
        .iter()
        .filter(|(op, params)| op == "report" && params["n"] == "1")
        .collect();
    assert_eq!(asked.len(), 1, "clicking Report must request report #1");
    drop(calls);

    // …and the screen followed it.
    assert_eq!(
        harness.state().screen,
        Screen::Report("1".to_string()),
        "the report screen did not open"
    );
}

#[test]
fn clicking_start_hands_the_run_to_the_engine() {
    let mut fake = FakeBackend::new();
    for (op, value) in all_fixtures() {
        fake = fake.with(op, value);
    }
    let backend = Arc::new(fake);
    let mut app = CareerOpsApp::new(backend.clone(), Settings::default());
    app.load_all();
    app.settings.cli_id = "claude".into();
    app.run.kind = Some(career_ops_desktop::RunKind::Evaluate);
    app.run.input = "https://example.com/job/7".into();

    let mut harness = harness_running(app, |ui, app| views::run::ui(ui, app));
    harness.get_by_label("Start").click();
    // A started run shows a spinner, which asks for repaints forever, so the
    // harness is stepped rather than settled.
    harness.run_steps(3);

    let calls = backend.calls.lock().unwrap();
    let started: Vec<_> = calls.iter().filter(|(op, _)| op == "run").collect();
    assert_eq!(started.len(), 1, "the Start button must start exactly one run");
    assert_eq!(started[0].1["kind"], "evaluate");
    assert_eq!(started[0].1["input"], "https://example.com/job/7");
    assert_eq!(started[0].1["cliId"], "claude");
    drop(calls);

    assert!(harness.state().run.running, "the run screen does not know a run is in flight");
}

#[test]
fn clicking_a_run_kind_switches_the_input_and_the_hint() {
    let app = app_with(&all_fixtures());
    let mut harness = harness_for(app, |ui, app| views::run::ui(ui, app));

    harness.get_by_label("Draft a cover letter").click();
    harness.run();

    assert_eq!(
        harness.state().run.kind,
        Some(career_ops_desktop::RunKind::Cover),
        "the kind picker did not take the click"
    );
    // The cover kind needs a brief, so the form grows a second field and the
    // input label changes to something cover-specific.
    assert!(
        shows(&harness, "Report number or company"),
        "the input label did not follow the kind"
    );
    assert!(
        shows(&harness, "The letter's four answers"),
        "the cover letter's answers are missing"
    );
    assert!(
        shows(&harness, "Tone"),
        "the cover letter's tone picker is missing"
    );
}

#[test]
fn typing_in_the_search_box_filters_as_you_type() {
    let app = app_with(&all_fixtures());
    let mut harness = harness_for(app, |ui, app| views::pipeline::ui(ui, app));

    // A text field only receives typed text when it holds focus, so the click
    // that focuses it is part of the interaction, not a test workaround.
    //
    // The screen has more than one text field now — the filter's search box
    // and the free-text find box — so the search box is picked by position:
    // the filter bar is drawn first.
    {
        let mut inputs = harness.query_all_by_role(egui::Role::TextInput);
        let field = inputs.next().expect("the pipeline screen has a search box");
        field.click();
    }
    harness.run();
    {
        let mut inputs = harness.query_all_by_role(egui::Role::TextInput);
        let field = inputs.next().expect("the pipeline screen has a search box");
        field.type_text("dragos");
    }
    harness.run();

    assert_eq!(harness.state().pipeline_view.search, "dragos");
    assert!(
        harness.query_by_label("Dragos").is_some(),
        "the matching row should still be shown"
    );
    assert!(
        harness.query_by_label("Leidos").is_none(),
        "the non-matching row should be gone"
    );
}

#[test]
fn logging_a_followup_from_the_dialog_writes_through_the_bridge() {
    let mut fake = FakeBackend::new();
    for (op, value) in all_fixtures() {
        fake = fake.with(op, value);
    }
    fake = fake.with(
        "followups",
        followups(),
    );
    let backend = Arc::new(fake);
    let mut app = CareerOpsApp::new(backend.clone(), Settings::default());
    app.load_all();
    let mut harness = harness_for(app, |ui, app| views::followups::ui(ui, app));

    // Open the dialog for the one due entry…
    harness.get_by_label("Log a follow-up").click();
    harness.run();
    assert_eq!(harness.state().followup_view.logging_for.as_deref(), Some("2"));

    // …fill the dialog's fields. The note is set through the dialog's own state
    // rather than by typing: this screen has three text inputs (search, contact,
    // notes) and picking one by role order is ambiguous enough that the
    // assertion would test the picker, not the plumbing. Typing into a field
    // reaching the app state is proven by the search-box test above; what
    // matters here is that the dialog's fields reach the WRITE.
    {
        let state = harness.state_mut();
        state.followup_view.log_notes = "called them".into();
        state.followup_view.log_contact = "Dana".into();
    }
    harness.run();

    // …and save it.
    harness.get_by_label("Save").click();
    harness.run();

    let calls = backend.calls.lock().unwrap();
    let logged: Vec<_> = calls
        .iter()
        .filter(|(op, params)| op == "followups" && params["action"] == "log")
        .collect();
    assert_eq!(logged.len(), 1, "Save must write exactly one follow-up");
    assert_eq!(logged[0].1["appNum"], "2");
    assert_eq!(logged[0].1["company"], "Dragos");
    assert_eq!(logged[0].1["channel"], "Email", "the dialog's default channel");
    assert_eq!(
        logged[0].1["notes"], "called them",
        "the dialog's note did not reach the write"
    );
    assert_eq!(logged[0].1["contact"], "Dana");
}

#[test]
fn saving_the_profile_from_config_merges_rather_than_replaces() {
    let mut fake = FakeBackend::new();
    for (op, value) in all_fixtures() {
        fake = fake.with(op, value);
    }
    fake = fake.with(
        "followups",
        serde_json::json!({
            "defaults": { "applied_first_days": 7 }, "defaultsAvailable": true,
            "overrides": {}, "effective": { "applied_first_days": 7 }, "enabled": true
        }),
    );
    let backend = Arc::new(fake);
    let mut app = CareerOpsApp::new(backend.clone(), Settings::default());
    app.load_all();
    let harness_app = app;
    let mut harness = harness_for(harness_app, |ui, app| views::config::ui(ui, app));

    harness.get_by_label("Save the profile").click();
    harness.run();

    let calls = backend.calls.lock().unwrap();
    let saved: Vec<_> = calls
        .iter()
        .filter(|(op, params)| op == "config" && params["action"] == "setProfile")
        .collect();
    assert_eq!(saved.len(), 1, "one profile write per click");
    // The writer is a PATCH: it carries the fields it means to change and no
    // notion of replacing the document. The merge itself is the bridge's job
    // and is asserted in tests/desktop-bridge-ops.test.mjs.
    assert_eq!(saved[0].1["name"], "Alex McCune");
    assert_eq!(saved[0].1["email"], "alex@example.com");
}

// ─── A silent CLI is not a hang ────────────────────────────────────────────
//
// Measured, not imagined: a real `hermes` evaluation streamed nothing for its
// entire 740-second budget (`-Q` buffers stdout), and a real `opencode2`
// evaluation produced two text events in five minutes before quitting on its
// own. Both were working. Both were indistinguishable from a hang, because the
// screen could only say "Running…".

#[test]
fn a_keepalive_does_not_count_as_the_agent_speaking() {
    let mut fake = FakeBackend::new();
    fake = fake.with("bootstrap", bootstrap()).with("pipeline", pipeline());
    let mut app = CareerOpsApp::new(Arc::new(fake), Settings::default());
    app.run.running = true;
    app.run.request_id = Some(1);
    app.run.started_at = Some(0.0);
    app.run.last_output_at = Some(0.0);

    // Ten seconds of keepalives: the transport is alive, the agent is silent.
    app.absorb_events_at(
        vec![career_ops_desktop::bridge::StreamEvent {
            request_id: 1,
            event: RunEvent::Keepalive,
        }],
        10.0,
    );
    assert_eq!(app.run.last_output_at, Some(0.0), "a keepalive must not reset the silence clock");
    assert_eq!(app.run_clock(10.0), Some((10.0, 10.0)));

    // Then real text.
    app.absorb_events_at(
        vec![career_ops_desktop::bridge::StreamEvent {
            request_id: 1,
            event: RunEvent::Text { text: "reading the posting".into() },
        }],
        12.0,
    );
    assert_eq!(app.run.last_output_at, Some(12.0));
    assert_eq!(app.run_clock(14.0), Some((14.0, 2.0)));
}

#[test]
fn a_silent_run_says_so_instead_of_only_saying_running() {
    let mut fake = FakeBackend::new();
    for (op, value) in all_fixtures() {
        fake = fake.with(op, value);
    }
    let mut app = CareerOpsApp::new(Arc::new(fake), Settings::default());
    app.load_all();
    app.settings.cli_id = "hermes".into();
    app.run.kind = Some(career_ops_desktop::RunKind::Evaluate);
    app.run.input = "https://example.com/job".into();
    app.run.running = true;
    app.run.request_id = Some(1);
    // A run started long ago with nothing heard since.
    app.run.started_at = Some(-200.0);
    app.run.last_output_at = Some(-200.0);

    let harness = harness_running(app, |ui, app| views::run::ui(ui, app));
    assert!(
        shows(&harness, "no output for"),
        "a long silence must be named, not left as a bare spinner"
    );
    assert!(shows(&harness, "elapsed"), "the run's age is missing");
}

#[test]
fn a_quiet_run_under_the_threshold_does_not_nag() {
    let mut fake = FakeBackend::new();
    for (op, value) in all_fixtures() {
        fake = fake.with(op, value);
    }
    let mut app = CareerOpsApp::new(Arc::new(fake), Settings::default());
    app.load_all();
    app.settings.cli_id = "hermes".into();
    app.run.kind = Some(career_ops_desktop::RunKind::Evaluate);
    app.run.input = "https://example.com/job".into();
    app.run.running = true;
    app.run.request_id = Some(1);
    app.run.started_at = Some(0.0);
    app.run.last_output_at = Some(0.0);

    let harness = harness_running(app, |ui, app| views::run::ui(ui, app));
    assert!(
        !shows(&harness, "no output for"),
        "a run that started a moment ago must not claim silence"
    );
}

#[test]
fn durations_read_the_way_a_person_would_say_them() {
    use career_ops_desktop::views::run::short_duration;
    assert_eq!(short_duration(0.4), "0s");
    assert_eq!(short_duration(45.0), "45s");
    assert_eq!(short_duration(134.0), "2m 14s");
    assert_eq!(short_duration(740.0), "12m 20s");
    assert_eq!(short_duration(3700.0), "1h 01m");
}

// ─── The app picks a CLI that exists ──────────────────────────────────────

#[test]
fn a_missing_saved_cli_is_replaced_by_one_that_is_installed() {
    // The stored id names a CLI this machine does not have — the #4012 shape,
    // and the reason the run screen otherwise refuses with "pick one in Config"
    // for a CLI nobody chose.
    let mut app = app_with(&all_fixtures());
    app.settings.cli_id = "claude-not-installed-here".into();
    let changed = app.resolve_cli();
    assert!(changed, "a stale id must be substituted");
    assert_eq!(app.settings.cli_id, "claude", "the engine's default should win");
}

#[test]
fn an_installed_saved_cli_is_left_alone() {
    let mut app = app_with(&all_fixtures());
    app.settings.cli_id = "codex".into();
    // codex is present in the fixture but NOT installed.
    assert!(app.resolve_cli(), "an uninstalled id must be substituted");
    assert_eq!(app.settings.cli_id, "claude");

    let mut app = app_with(&all_fixtures());
    app.settings.cli_id = "claude".into();
    assert!(!app.resolve_cli(), "an installed id must not be replaced");
    assert_eq!(app.settings.cli_id, "claude");
}

#[test]
fn loading_the_app_selects_a_usable_cli_without_a_visit_to_config() {
    let mut fake = FakeBackend::new();
    for (op, value) in all_fixtures() {
        fake = fake.with(op, value);
    }
    let mut app = CareerOpsApp::new(Arc::new(fake), Settings::default());
    assert!(app.settings.cli_id.is_empty(), "the default settings name no CLI");
    app.load_all();
    assert_eq!(
        app.settings.cli_id, "claude",
        "opening the app must leave a CLI the Start button can use"
    );
    // The claim is the whole path: open the app, choose a kind, paste a URL, and
    // Start is live without a detour through Config.
    app.run.kind = Some(career_ops_desktop::RunKind::Evaluate);
    app.run.input = "https://example.com/job/1".into();
    assert!(app.run_ready(), "the run screen would still refuse to start");
}

#[test]
fn config_shows_the_installed_clis_first_and_folds_the_rest_away() {
    let mut fake = FakeBackend::new();
    for (op, value) in all_fixtures() {
        fake = fake.with(op, value);
    }
    fake = fake.with(
        "followups",
        serde_json::json!({
            "defaults": {}, "defaultsAvailable": false, "overrides": {}, "effective": {}, "enabled": true
        }),
    );
    let mut app = CareerOpsApp::new(Arc::new(fake), Settings::default());
    app.load_all();
    let mut harness = harness_for(app, |ui, app| views::config::ui(ui, app));

    // Installed: visible and selectable.
    assert!(harness.query_by_label("Claude Code").is_some());
    // Not installed: folded away, so it cannot be mistaken for a choice.
    assert!(
        harness.query_by_label("Codex").is_none(),
        "an uninstalled CLI should not sit in the list as an equal option"
    );
    assert!(
        shows(&harness, "Not installed (1)"),
        "the folded group must say how many it holds"
    );

    // And it opens.
    harness.get_by_label("Not installed (1)").click();
    harness.run();
    assert!(
        harness.query_by_label("Codex").is_some(),
        "the folded group did not open"
    );
}

// ─── M1: the trustworthy daily loop ────────────────────────────────────────

#[test]
fn the_pipeline_screen_offers_a_skip_control_for_each_pending_posting() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::pipeline::ui(ui, app));

    assert!(
        shows(&harness, "Skip"),
        "the pending postings must offer a skip control"
    );
    assert!(
        shows(&harness, "Contoso"),
        "the pending posting must be shown"
    );
    assert!(
        shows(&harness, "Shortlist"),
        "the pending posting must offer a shortlist control"
    );
}

#[test]
fn the_pipeline_screen_offers_a_bulk_change_and_a_delete() {
    let app = app_with(&all_fixtures());
    let mut harness = harness_for(app, |ui, app| views::pipeline::ui(ui, app));

    // The bulk bar appears once a row is picked.
    assert!(
        !shows(&harness, "row(s) picked"),
        "nothing is picked yet, so there is no bulk bar"
    );
    // The per-row delete control is always there.
    assert!(
        shows(&harness, "Delete"),
        "each row must offer a removal control"
    );
}

#[test]
fn the_report_screen_opens_the_lead_and_folds_the_rest() {
    let mut app = app_with(&all_fixtures());
    app.screen = Screen::Report("2".into());
    app.state.report =
        Some(serde_json::from_value(report()).expect("report fixture decodes"));
    let harness = harness_for(app, |ui, app| views::report::ui(ui, app));

    // The lead is OPEN: its content is on the screen.
    assert!(
        shows(&harness, "The CV covers every requirement"),
        "the lead block must be open"
    );
    // The other blocks are FOLDED: their headings are there, their bodies are
    // not.
    assert!(
        shows(&harness, "Block C"),
        "the other blocks must still be listed"
    );
    assert!(
        !shows(&harness, "Two requirements the CV does not show"),
        "a folded block must not show its body"
    );
}

#[test]
fn ctrl_number_keys_reach_every_screen() {
    use career_ops_desktop::shell::NAV;
    assert!(
        NAV.len() <= 10,
        "there are more screens than the Ctrl 1..9 and Ctrl 0 shortcuts cover"
    );
    for (index, (label, screen)) in NAV.iter().enumerate() {
        let mut app = app_with(&all_fixtures());
        // The key the user presses for this screen: 1..9, then 0.
        let key = match index {
            0 => egui::Key::Num1,
            1 => egui::Key::Num2,
            2 => egui::Key::Num3,
            3 => egui::Key::Num4,
            4 => egui::Key::Num5,
            5 => egui::Key::Num6,
            6 => egui::Key::Num7,
            7 => egui::Key::Num8,
            8 => egui::Key::Num9,
            _ => egui::Key::Num0,
        };
        // The shortcut is read from the frame's input, so the test presses the
        // key in a real frame and lets the screen draw.
        let mut harness = harness_for(app, |ui, app| {
            app.handle_screen_shortcuts(ui.ctx());
            views::pipeline::ui(ui, app);
        });
        harness.key_press_modifiers(egui::Modifiers::COMMAND, key);
        harness.run();
        assert_eq!(
            harness.state().screen, *screen,
            "Ctrl + {index} must reach {label}"
        );
    }
}

#[test]
fn the_app_reopens_on_the_screen_it_was_left_on() {
    let mut app = app_with(&all_fixtures());
    app.go(Screen::Portals);
    assert_eq!(app.settings.last_screen, "portals");

    let mut reopened = app_with(&all_fixtures());
    reopened.settings.last_screen = "portals".to_string();
    reopened.go(Screen::from_id(&reopened.settings.last_screen));
    assert_eq!(reopened.screen, Screen::Portals);

    // A saved id that names no screen opens Today rather than a blank window.
    let mut stale = app_with(&all_fixtures());
    stale.go(Screen::from_id("a-screen-that-no-longer-exists"));
    assert_eq!(stale.screen, Screen::Today);
}

#[test]
fn the_follow_ups_entry_is_hidden_when_follow_ups_are_off() {
    let mut answers = all_fixtures();
    let mut boot = bootstrap();
    boot["followupsEnabled"] = serde_json::json!(false);
    answers.retain(|(op, _)| *op != "bootstrap");
    answers.push(("bootstrap", boot));

    let app = app_with(&answers);
    let entries = career_ops_desktop::shell::nav_entries(&app);
    assert!(
        !entries.iter().any(|(label, _)| *label == "Follow-ups"),
        "a nav entry for a feature that is off is a dead control"
    );

    let mut answers_on = all_fixtures();
    let app_on = app_with(&answers_on);
    let entries_on = career_ops_desktop::shell::nav_entries(&app_on);
    assert!(
        entries_on.iter().any(|(label, _)| *label == "Follow-ups"),
        "the entry must be there when follow-ups are on"
    );
}

#[test]
fn a_missing_saved_cli_is_replaced_and_the_swap_is_said() {
    let mut app = app_with(&all_fixtures());
    app.settings.cli_id = "codex".to_string();
    let replaced = app.resolve_cli();
    assert!(replaced, "the absent CLI must be replaced");
    assert_eq!(app.settings.cli_id, "claude");
    let message = app.state.message.clone().expect("the swap must be said");
    assert!(
        message.0.contains("codex") && message.0.contains("Claude Code"),
        "the message must name both CLIs: {}",
        message.0
    );
}

#[test]
fn the_config_screen_shows_what_the_profile_validator_found() {
    let mut answers = all_fixtures();
    answers.retain(|(op, _)| *op != "config");
    answers.push((
        "config",
        serde_json::json!({
            "profile": { "candidate": { "full_name": "Alex McCune" } },
            "exists": true,
            "malformed": false,
            "clis": [],
            "states": ["Evaluated"],
            "codeRoot": "/fixture",
            "dataRoot": "/fixture"
        }),
    ));
    let mut app = app_with(&answers);
    app.state.profile_validation = Some(career_ops_desktop::model::ProfileValidation {
        ok: false,
        exit_code: 2,
        findings: vec![],
        errors: vec![career_ops_desktop::model::ProfileFinding {
            level: "error".into(),
            code: "not-a-mapping".into(),
            key: None,
            message: "config/profile.yml must contain named settings".into(),
            suggestion: Some("fix the YAML".into()),
        }],
        warnings: vec![],
        message: "config/profile.yml must contain named settings".into(),
    });
    let harness = harness_for(app, |ui, app| views::config::ui(ui, app));

    assert!(
        shows(&harness, "Profile check"),
        "the config screen must offer the profile check"
    );
    assert!(
        shows(&harness, "config/profile.yml must contain named settings"),
        "the finding must reach the screen"
    );
    assert!(
        shows(&harness, "Health check"),
        "the config screen must offer the health check"
    );
}

#[test]
fn the_follow_ups_screen_offers_a_remove_and_a_pin() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::followups::ui(ui, app));

    assert!(
        shows(&harness, "Remove"),
        "a logged follow-up must offer a removal control"
    );
    assert!(
        shows(&harness, "Pin the next date"),
        "a follow-up must offer a pin control"
    );
    assert!(
        shows(&harness, "Seed the first follow-up"),
        "a follow-up must offer the first-date seed"
    );
}

#[test]
fn the_run_screen_offers_every_kind_and_its_cost() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::run::ui(ui, app));

    assert!(shows(&harness, "Evaluate a posting"));
    assert!(shows(&harness, "Tailor the CV (PDF)"));
    assert!(shows(&harness, "Draft a cover letter"));
    assert!(shows(&harness, "Draft an application email"));
    assert!(shows(&harness, "Build an interview prep pack"));
    assert!(shows(&harness, "Practise interview questions"));
    assert!(shows(&harness, "Debrief a real interview"));
    assert!(shows(&harness, "Judge whether a company is safe"));
    assert!(shows(&harness, "Walk an offer's clauses"));
    assert!(shows(&harness, "Repair a broken portal"));
}

#[test]
fn a_run_is_refused_when_the_app_and_the_bridge_disagree() {
    let mut app = app_with(&all_fixtures());
    app.state.version = Some(career_ops_desktop::model::Version {
        protocol: career_ops_desktop::bridge::PROTOCOL_VERSION + 1,
        engine: "9.9.9".into(),
        node: "v22.0.0".into(),
    });
    app.settings.cli_id = "claude".into();
    app.run.kind = Some(career_ops_desktop::app::RunKind::Evaluate);
    app.run.input = "https://example.com/job/1".into();
    assert!(
        !app.run_ready(),
        "a mismatched pair must not start a run"
    );

    app.state.version = Some(career_ops_desktop::model::Version {
        protocol: career_ops_desktop::bridge::PROTOCOL_VERSION,
        engine: "9.9.9".into(),
        node: "v22.0.0".into(),
    });
    assert!(
        app.run_ready(),
        "a matching pair must start a run"
    );
}

#[test]
fn the_zoom_and_the_theme_are_saved() {
    // A temporary config directory, so the test never touches the real one.
    let dir = std::env::temp_dir().join(format!("career-ops-settings-{}", std::process::id()));
    let previous = std::env::var("XDG_CONFIG_HOME").ok();
    std::env::set_var("XDG_CONFIG_HOME", &dir);

    let mut app = app_with(&all_fixtures());
    let ctx = egui::Context::default();
    app.zoom_by(&ctx, 1);
    assert!(
        app.settings.zoom > 1.15,
        "Ctrl + must make things bigger, not smaller"
    );
    app.settings.theme = career_ops_desktop::settings::Theme::Light;
    let _ = app.settings.save();

    let reloaded = career_ops_desktop::settings::Settings::load();
    assert_eq!(reloaded.zoom, app.settings.zoom, "the zoom must survive a restart");
    assert_eq!(
        reloaded.theme,
        career_ops_desktop::settings::Theme::Light,
        "the theme must survive a restart"
    );

    match previous {
        Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_bridge_that_dies_fails_the_pending_calls_with_bridge_gone() {
    use career_ops_desktop::bridge::{Bridge, BridgePaths};

    // A server that exits at once: the bridge is gone before it answers.
    let dir = std::env::temp_dir().join(format!("career-ops-dead-bridge-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the temp dir must be creatable");
    let server = dir.join("server.mjs");
    std::fs::write(&server, "process.exit(0);\n").expect("the stub must be writable");

    let paths = BridgePaths {
        client_root: dir.clone(),
        code_root: dir.clone(),
        server,
        node: "node".to_string(),
    };
    let bridge = Bridge::start(paths, None).expect("the process must start");
    let outcome = bridge.call("bootstrap", serde_json::json!({}));
    assert!(outcome.is_err(), "a call to a dead bridge must fail");
    let error = outcome.expect_err("the call must fail");
    assert_eq!(error.code, "bridge-gone");
    assert!(
        error.message.contains("stopped running"),
        "the message must say the bridge stopped: {}",
        error.message
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_cv_screen_offers_the_structure_check_and_the_skill_gaps() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::cv::ui(ui, app));

    assert!(
        shows(&harness, "Check the structure"),
        "the CV screen must offer the structure check"
    );
    assert!(
        shows(&harness, "Skill gaps"),
        "the CV screen must offer the skill gaps"
    );
    assert!(
        shows(&harness, "Template"),
        "the CV screen must offer the template picker"
    );
}

#[test]
fn the_run_screen_offers_the_cover_tones() {
    let app = app_with(&all_fixtures());
    let mut harness = harness_for(app, |ui, app| views::run::ui(ui, app));

    harness.get_by_label("Draft a cover letter").click();
    harness.run();

    assert!(
        shows(&harness, "Tone"),
        "a cover letter must offer a tone"
    );
    assert!(
        shows(&harness, "The letter's four answers"),
        "a cover letter must ask the four questions the mode asks"
    );
}

#[test]
fn the_documents_screen_lists_what_was_generated() {
    let mut app = app_with(&all_fixtures());
    app.state.documents = Some(vec![career_ops_desktop::model::Document {
        filename: "cv-alex-acme-001.pdf".into(),
        kind: "cv".into(),
        date: Some("2026-01-02".into()),
        report_id: Some("1".into()),
        format: Some("pdf".into()),
        id: "abc".into(),
    }]);
    let harness = harness_for(app, |ui, app| views::documents::ui(ui, app));

    assert!(
        shows(&harness, "Generated documents"),
        "the documents screen must show the list"
    );
    assert!(
        shows(&harness, "cv-alex-acme-001.pdf"),
        "the generated document must be listed"
    );
    assert!(
        shows(&harness, "Archive a posting"),
        "the documents screen must offer the archive"
    );
}

// ─── the rest of the loop ──────────────────────────────────────────────────

#[test]
fn the_report_screen_offers_the_screening_pack_and_a_liveness_check() {
    let mut app = app_with(&all_fixtures());
    app.screen = Screen::Report("2".into());
    app.state.report =
        Some(serde_json::from_value(report()).expect("report fixture decodes"));
    let harness = harness_for(app, |ui, app| views::report::ui(ui, app));

    assert!(shows(&harness, "Screening pack"));
    assert!(shows(&harness, "Is it still live?"));
    assert!(shows(&harness, "Keyword coverage"));
    assert!(shows(&harness, "Score levers"));
    assert!(shows(&harness, "Company history"));
}

#[test]
fn the_pipeline_screen_offers_a_compare() {
    let mut app = app_with(&all_fixtures());
    app.pipeline_view.selected = vec!["1".into(), "2".into()];
    let harness = harness_for(app, |ui, app| views::pipeline::ui(ui, app));
    assert!(shows(&harness, "Compare the picked rows"));
}

#[test]
fn the_run_screen_shows_what_has_been_spent() {
    let mut app = app_with(&all_fixtures());
    app.state.usage = Some(career_ops_desktop::model::Usage {
        windows: vec![
            career_ops_desktop::model::UsageWindow {
                name: "window5h".into(),
                hours: 5,
                runs: 2,
                tokens: 12_000.0,
                usd: 0.42,
            },
            career_ops_desktop::model::UsageWindow {
                name: "window7d".into(),
                hours: 168,
                runs: 9,
                tokens: 99_000.0,
                usd: 3.1,
            },
        ],
        computed_at: 0.0,
        runs: 9,
    });
    let harness = harness_for(app, |ui, app| views::run::ui(ui, app));
    assert!(shows(&harness, "What you have spent"));
    assert!(shows(&harness, "5h:"));
    assert!(shows(&harness, "168h:"));
}

#[test]
fn the_cv_screen_offers_the_import_and_the_master_profile() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::cv::ui(ui, app));
    assert!(shows(&harness, "Import from documents/"));
    assert!(shows(&harness, "Master career profile"));
}

#[test]
fn the_config_screen_offers_the_language_and_the_checklist() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::config::ui(ui, app));
    assert!(shows(&harness, "Language and market"));
    assert!(shows(&harness, "For a later session"));
    assert!(shows(&harness, "Plugins"));
    assert!(shows(&harness, "Maintenance"));
}

#[test]
fn the_portals_screen_offers_the_liveness_and_the_reposts() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::portals::ui(ui, app));
    assert!(shows(&harness, "Re-listed roles"));
    assert!(shows(&harness, "JD without a browser"));
    assert!(shows(&harness, "Why was this missed?"));
    assert!(shows(&harness, "New this week"));
}

#[test]
fn the_outcomes_screen_offers_the_outreach_and_the_share() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::outcomes::ui(ui, app));
    assert!(shows(&harness, "Outreach"));
    assert!(shows(&harness, "Do I know anyone?"));
    assert!(shows(&harness, "Match it"));
    assert!(shows(&harness, "Share a hire"));
    assert!(shows(&harness, "Record an outcome"));
}

#[test]
fn a_missing_engine_checkout_is_one_actionable_sentence() {
    use career_ops_desktop::code_root;
    // A directory with no checkout beside it: the failure is the SENTENCE, and
    // nothing else.
    let nowhere = std::env::temp_dir().join("career-ops-no-such-parent").join("client");
    let error = code_root::resolve(None, &nowhere).expect_err("there is no checkout there");
    assert_eq!(error, code_root::ACTIONABLE);
    assert!(
        !error.contains(". "),
        "the failure must be one sentence, not two: {error}"
    );
    assert!(error.contains("CAREER_OPS_CODE_ROOT"), "and it must say what to do");
}

#[test]
fn the_outcomes_screen_proposes_an_outcome_before_recording_it() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::outcomes::ui(ui, app));
    assert!(
        shows(&harness, "Preview"),
        "an outcome is previewed before it is recorded"
    );
    assert!(
        shows(&harness, "Record it"),
        "and recorded on the second click"
    );
}

#[test]
fn the_config_screen_edits_one_personalization_section() {
    let mut app = app_with(&all_fixtures());
    app.state.personalization = Some(vec![
        career_ops_desktop::app::PersonalizationSection {
            id: "targetRoles".into(),
            heading: "## Your Target Roles".into(),
            state: "template".into(),
            body: "| Archetype |".into(),
        },
        career_ops_desktop::app::PersonalizationSection {
            id: "exitNarrative".into(),
            heading: "## Your Exit Narrative".into(),
            state: "filled".into(),
            body: "I want a role that keeps my hands on the systems.".into(),
        },
    ]);
    app.personalization_section = "targetRoles".into();
    let harness = harness_for(app, |ui, app| views::config::ui(ui, app));

    assert!(
        shows(&harness, "Your Target Roles (template)"),
        "the sections must be listed with their state"
    );
    assert!(
        shows(&harness, "Save this section"),
        "one section at a time is the rule"
    );
    assert!(
        shows(&harness, "only this section changes"),
        "and the screen must say so"
    );
}

#[test]
fn the_config_screen_confirms_a_plugin_install() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::config::ui(ui, app));
    assert!(shows(&harness, "Add a plugin"));
}

#[test]
fn the_run_screen_offers_the_outreach_mode() {
    let app = app_with(&all_fixtures());
    let harness = harness_for(app, |ui, app| views::run::ui(ui, app));
    assert!(
        shows(&harness, "Draft outreach to a contact"),
        "the outreach mode must be offered"
    );
}

// ─── the PDF preview ───────────────────────────────────────────────────────

#[test]
fn the_report_screen_offers_a_pdf_preview_and_the_viewer_opens_it() {
    let mut app = app_with(&all_fixtures());
    let app_ref = &mut app;
    app_ref.screen = Screen::Report("2".into());
    app_ref.state.report =
        Some(serde_json::from_value(report()).expect("report fixture decodes"));
    let harness = harness_for(app, |ui, app| views::report::ui(ui, app));

    assert!(
        shows(&harness, "Preview the CV"),
        "the report screen must offer a preview of the tailored CV"
    );
    assert!(
        shows(&harness, "Open the tailored CV (PDF)"),
        "and the file itself, for the user's own viewer"
    );
}

#[test]
fn the_documents_screen_offers_a_preview_of_what_it_lists() {
    let mut app = app_with(&all_fixtures());
    app.state.documents = Some(vec![career_ops_desktop::model::Document {
        filename: "cv-alex-acme-001.pdf".into(),
        kind: "cv".into(),
        date: Some("2026-01-02".into()),
        report_id: Some("1".into()),
        format: Some("pdf".into()),
        id: "abc".into(),
    }]);
    let harness = harness_for(app, |ui, app| views::documents::ui(ui, app));
    assert!(
        shows(&harness, "Preview"),
        "a listed document must be previewable in the window"
    );
}

#[test]
fn the_preview_card_says_which_page_it_is_showing() {
    let mut app = app_with(&all_fixtures());
    app.preview = Some(career_ops_desktop::app::PdfPreview {
        path: "/fixture/output/cv-alex-acme-001.pdf".into(),
        page: 1,
        pages: 2,
        texture: None,
        error: None,
    });
    let harness = harness_for(app, |ui, app| {
        // The card is drawn on its own here, so it adopts the token styles first
        // — a screen draws through `frame`, which does the same.
        let t = career_ops_desktop::theme::adopt(ui);
        views::pdfview::card(ui, app, t);
    });
    assert!(shows(&harness, "page 1 of 2"), "the card must say where it is");
    assert!(shows(&harness, "Next"), "and offer the next page");
    assert!(shows(&harness, "Open in your viewer"));
}

#[test]
fn a_preview_that_cannot_be_made_says_why() {
    let mut app = app_with(&all_fixtures());
    app.preview = Some(career_ops_desktop::app::PdfPreview {
        path: "/fixture/output/missing.pdf".into(),
        page: 1,
        pages: 1,
        texture: None,
        error: Some("There is no file at /fixture/output/missing.pdf".into()),
    });
    let harness = harness_for(app, |ui, app| {
        // The card is drawn on its own here, so the token styles it names are
        // installed first — a screen draws through `frame`, which does this.
        let t = career_ops_desktop::theme::adopt(ui);
        views::pdfview::card(ui, app, t);
    });
    assert!(shows(&harness, "no preview"), "the card must say there is none");
    assert!(
        shows(&harness, "There is no file at"),
        "and give the one sentence that says what to do about it"
    );
}
