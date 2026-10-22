//! The outcomes screen: what happened to an application, who is still silent,
//! where the recruiting process dragged, what the salary gap looks like, and
//! the one thing a hire may share about it.
//!
//! Every answer is the core's own: outcome.mjs, rejection-latency.mjs,
//! process-quality.mjs, salary-gap.mjs, negotiation-roi.mjs and
//! hired-share.mjs.

use egui::RichText;

use crate::app::CareerOpsApp;
use crate::theme::{self, Palette};
use crate::views::frame;

/// Draw the outcomes screen.
pub fn ui(ui: &mut egui::Ui, app: &mut CareerOpsApp) {
    let t = theme::palette(ui.ctx());
    frame(
        ui,
        "Outcomes",
        Some("What happened, what is still quiet, and what it cost."),
        app,
        |ui, app| {
            record_card(ui, app, t);
            ui.add_space(14.0);
            quiet_card(ui, app, t);
            ui.add_space(14.0);
            salary_card(ui, app, t);
            ui.add_space(14.0);
            contacts_card(ui, app, t);
            ui.add_space(14.0);
            share_card(ui, app, t);
        },
    );
}

/// Record an outcome for one application.
fn record_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Record an outcome", |ui| {
            ui.label(
                RichText::new("the core archives the posting and syncs the tracker")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.outcome.target)
                    .desired_width(120.0)
                    .hint_text("report # or company"),
            );
            egui::ComboBox::from_id_salt("outcome-type")
                .selected_text(RichText::new(&app.outcome.kind).color(t.fg))
                .show_ui(ui, |ui| {
                    for kind in OUTCOME_KINDS {
                        if ui
                            .selectable_label(app.outcome.kind == kind, kind)
                            .clicked()
                        {
                            app.outcome.kind = kind.to_string();
                        }
                    }
                });
            ui.add(
                egui::TextEdit::singleline(&mut app.outcome.note)
                    .desired_width(200.0)
                    .hint_text("note"),
            );
            if crate::views::primary_button(ui, "Record", !app.outcome.target.trim().is_empty()).clicked() {
                record(app);
            }
        });
        if !app.outcome.output.is_empty() {
            ui.add_space(6.0);
            ui.label(
                RichText::new(&app.outcome.output)
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        }
    });
}

/// The outcome kinds the core's outcome.mjs accepts.
pub const OUTCOME_KINDS: [&str; 7] = [
    "interview_progress",
    "offer_received",
    "hired",
    "offer_declined",
    "rejected",
    "no_response",
    "interview_only",
];

/// Record the outcome, through the core's own script.
fn record(app: &mut CareerOpsApp) {
    let params = serde_json::json!({
        "action": "record",
        "n": app.outcome.target,
        "type": app.outcome.kind,
        "note": app.outcome.note,
        "dryRun": true,
    });
    match app.backend.call("outcomes", params) {
        Ok(value) => {
            app.outcome.output = value
                .get("output")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();
            app.state.say("previewed the outcome; nothing was written");
        }
        Err(e) => app.state.fail(format!("could not record the outcome: {e}")),
    }
}

/// Companies still silent after an interview.
fn quiet_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Still silent", |ui| {
            if crate::views::ghost_button(ui, "Check").clicked() {
                load_json(app, "analysis", "quietCompanies");
            }
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new("a suggestion, never a decision — the copy is yours to make")
                .text_style(theme::meta())
                .color(t.fg_muted),
        );
    });
}

/// The salary gap, wanted against advertised against actual.
fn salary_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Salary gap", |ui| {
            if crate::views::ghost_button(ui, "Check").clicked() {
                load_json(app, "analysis", "salaryGap");
            }
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new("wanted, advertised and actual, per role")
                .text_style(theme::meta())
                .color(t.fg_muted),
        );
    });
}

/// The phonebook: who you know, and where they are.
fn contacts_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Contacts", |ui| {
            if crate::views::ghost_button(ui, "Reload").clicked() {
                app.load_contacts();
            }
        });
        ui.add_space(8.0);
        let Some(contacts) = app.state.contacts.clone() else {
            ui.label(
                RichText::new("The phonebook has not been read yet.")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            return;
        };
        ui.label(
            RichText::new(format!("{} contact(s)", contacts.total))
                .text_style(theme::meta())
                .color(t.fg_muted),
        );
        for contact in contacts.contacts.iter().take(20) {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&contact.name).strong().color(t.fg));
                if !contact.company.is_empty() {
                    ui.label(RichText::new(&contact.company).color(t.fg_muted));
                }
                if !contact.r#type.is_empty() {
                    crate::views::pill(ui, &contact.r#type, t.accent);
                }
                if !contact.email.is_empty() {
                    ui.label(
                        RichText::new(&contact.email)
                            .text_style(theme::meta())
                            .color(t.fg_muted),
                    );
                }
            });
            ui.add_space(3.0);
        }
    });
}

/// The outreach half: the warm introduction, the invite, the export.
fn share_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Outreach", |ui| {
            ui.label(
                RichText::new("offline; nothing is ever sent")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.outreach_company)
                    .desired_width(180.0)
                    .hint_text("company"),
            );
            if crate::views::ghost_button(ui, "Do I know anyone?").clicked() {
                let company = app.outreach_company.clone();
                app.run_op(
                    "contacts",
                    serde_json::json!({ "action": "linkedin", "company": company }),
                );
            }
            if crate::views::ghost_button(ui, "Export the phonebook").clicked() {
                app.run_op("contacts", serde_json::json!({ "action": "export" }));
            }
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.outreach_invite)
                    .desired_width(320.0)
                    .hint_text("paste an interview invite"),
            );
            if crate::views::ghost_button(ui, "Match it").clicked() {
                let invite = app.outreach_invite.clone();
                app.run_op(
                    "contacts",
                    serde_json::json!({ "action": "inviteMatch", "invite": invite }),
                );
            }
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Share a hire")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.outreach_report)
                    .desired_width(60.0)
                    .hint_text("report #"),
            );
            if crate::views::ghost_button(ui, "Draft it").clicked() {
                let report = app.outreach_report.trim().parse::<i64>().unwrap_or(0);
                app.run_op("outcomes", serde_json::json!({ "action": "hiredShare", "report": report }));
            }
        });
        if !app.state.notice.is_empty() {
            ui.add_space(6.0);
            ui.label(
                RichText::new(&app.state.notice)
                    .monospace()
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        }
    });
}

/// Ask the bridge for a JSON answer and show it.
fn load_json(app: &mut CareerOpsApp, op: &str, action: &str) {
    let params = serde_json::json!({ "action": action });
    match app.backend.call(op, params) {
        Ok(value) => {
            let text = value
                .get("output")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();
            if !text.is_empty() {
                app.state.say(text.lines().next().unwrap_or("done").to_string());
            }
        }
        Err(e) => app.state.fail(format!("could not run {action}: {e}")),
    }
}
