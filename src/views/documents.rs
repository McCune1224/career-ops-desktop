//! The documents screen: every CV and cover letter this search has produced,
//! and the posting archive that keeps them honest.
//!
//! The list is the web app's own: discoverDocuments() reads output/ and the
//! pdf-index manifest. The archive is the core's own archive-posting.mjs,
//! which needs a browser, so the app asks before it spends one.

use egui::RichText;

use crate::app::CareerOpsApp;
use crate::theme::{self, Palette};
use crate::views::frame;

/// Draw the documents screen.
pub fn ui(ui: &mut egui::Ui, app: &mut CareerOpsApp) {
    let t = theme::palette(ui.ctx());
    frame(
        ui,
        "Documents",
        Some("Every tailored CV and cover letter, and the postings kept with them."),
        app,
        |ui, app| {
            crate::views::pdfview::card(ui, app, t);
            if app.preview.is_some() {
                ui.add_space(14.0);
            }
            list_card(ui, app, t);
            ui.add_space(14.0);
            archive_card(ui, app, t);
        },
    );
}

/// The list of generated documents.
fn list_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Generated documents", |ui| {
            if crate::views::ghost_button(ui, "Reload").clicked() {
                app.load_documents();
            }
        });
        ui.add_space(8.0);
        let Some(documents) = app.state.documents.clone() else {
            ui.label(
                RichText::new("output/ has not been read yet.")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            return;
        };
        if documents.is_empty() {
            ui.label(
                RichText::new("Nothing has been generated yet.")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            return;
        }
        for doc in &documents {
            ui.horizontal(|ui| {
                let kind = match doc.kind.as_str() {
                    "cv" => "CV",
                    "cover" => "Cover",
                    _ => "Document",
                };
                crate::views::pill(ui, kind, t.accent);
                ui.label(RichText::new(&doc.filename).monospace().color(t.fg));
                if let Some(date) = &doc.date {
                    ui.label(
                        RichText::new(date)
                            .monospace()
                            .text_style(theme::meta())
                            .color(t.fg_muted),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::views::ghost_button(ui, "Preview").clicked() {
                        preview_document(app, &doc.filename);
                    }
                    if crate::views::ghost_button(ui, "Open").clicked() {
                        open_document(app, &doc.filename);
                    }
                    if let Some(report) = &doc.report_id {
                        if crate::views::ghost_button(ui, "Open the report").clicked() {
                            app.open_report(report);
                        }
                    }
                });
            });
            ui.add_space(4.0);
        }
    });
}

/// The archive form: keep a posting with the report it belongs to.
fn archive_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Archive a posting", |ui| {
            ui.label(
                RichText::new("needs a browser and the network")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new("Posting URL")
                .text_style(theme::meta())
                .color(t.fg_muted),
        );
        ui.add(
            egui::TextEdit::singleline(&mut app.archive.url)
                .desired_width(f32::INFINITY)
                .hint_text("https://…"),
        );
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Company")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.archive.company)
                    .desired_width(160.0),
            );
            ui.label(
                RichText::new("Role")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            ui.add(egui::TextEdit::singleline(&mut app.archive.role).desired_width(160.0));
            ui.label(
                RichText::new("Report #")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.archive.report)
                    .desired_width(60.0),
            );
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let mut dry = app.archive.dry_run;
            ui.checkbox(&mut dry, RichText::new("preview only (no capture)").color(t.fg));
            app.archive.dry_run = dry;
            if crate::views::primary_button(ui, "Archive", !app.archive.url.trim().is_empty()).clicked() {
                archive(app);
            }
        });
        if !app.archive.output.is_empty() {
            ui.add_space(6.0);
            ui.label(
                RichText::new(&app.archive.output)
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        }
    });
}

/// Ask the bridge to archive the posting.
fn archive(app: &mut CareerOpsApp) {
    let params = serde_json::json!({
        "action": "archive",
        "url": app.archive.url,
        "company": app.archive.company,
        "role": app.archive.role,
        "report": app.archive.report,
        "dryRun": app.archive.dry_run,
    });
    match app.backend.call("documents", params) {
        Ok(value) => {
            let output = value
                .get("output")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();
            app.archive.output = output;
            app.state.say(if app.archive.dry_run {
                "previewed the archive; nothing was captured"
            } else {
                "archived the posting"
            });
        }
        Err(e) => app.state.fail(format!("could not archive the posting: {e}")),
    }
}

/// Show a generated document's first page in the window.
fn preview_document(app: &mut CareerOpsApp, filename: &str) {
    let params = serde_json::json!({ "action": "resolve", "filename": filename });
    match app.backend.call("documents", params) {
        Ok(value) => {
            let path = value
                .get("path")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();
            if path.is_empty() {
                app.state.fail("that document could not be found");
            } else {
                app.open_preview(&path);
            }
        }
        Err(e) => app.state.fail(format!("could not open the document: {e}")),
    }
}

/// Open a generated document in the user's own viewer.
fn open_document(app: &mut CareerOpsApp, filename: &str) {
    let params = serde_json::json!({ "action": "resolve", "filename": filename });
    match app.backend.call("documents", params) {
        Ok(value) => {
            let path = value
                .get("path")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();
            if path.is_empty() {
                app.state.fail("that document could not be found");
            } else {
                crate::views::open_path(&path);
            }
        }
        Err(e) => app.state.fail(format!("could not open the document: {e}")),
    }
}

/// The sections this screen owns, for the tests that look for them.
pub const SECTIONS: [&str; 2] = ["Generated documents", "Archive a posting"];
