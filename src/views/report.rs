//! The report screen: one application's evaluation, and the things that hang
//! off it (the source posting, the tailored CV, the cover letter).

use egui::{RichText};

use crate::app::CareerOpsApp;
use crate::markdown;
use crate::theme::{self, Palette};
use crate::views::{frame, pill};

/// Draw the report screen.
pub fn ui(ui: &mut egui::Ui, app: &mut CareerOpsApp) {
    let t = theme::palette(ui.ctx());
    let Some(report) = app.state.report.clone() else {
        ui.spinner();
        ui.label("Opening the report…");
        return;
    };

    let title = report
        .title
        .clone()
        .or_else(|| report.application.as_ref().map(|a| a.company.clone()))
        .unwrap_or_else(|| format!("Report {}", report.n));

    frame(ui, &title, report.file.as_deref(), app, |ui, app| {
        verdict_card(ui, &report, t);
        ui.add_space(12.0);

        theme::card(ui, t).show(ui, |ui| {
            if let Some(markdown_text) = report.markdown.as_deref() {
                markdown::render(ui, markdown_text);
            } else {
                ui.label(RichText::new("This application has no report yet.").color(t.fg_muted));
            }
        });

        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            if crate::views::ghost_button(ui, "Back to the pipeline").clicked() {
                app.screen = crate::app::Screen::Pipeline;
            }
            // Re-running is the real action here, and it is worth prefilling:
            // a re-evaluation of the same posting should not need a retyped URL.
            if let Some(url) = report.posting_url.clone() {
                if crate::views::ghost_button(ui, "Re-run the evaluation").clicked() {
                    app.run.kind = Some(crate::app::RunKind::Evaluate);
                    app.run.input = url;
                    app.screen = crate::app::Screen::Run;
                }
            }
        });
    });
}

/// The verdict at the top: how it scored, what tier, and every way out.
fn verdict_card(ui: &mut egui::Ui, report: &crate::model::Report, t: Palette) {
    theme::hero_card(ui, t).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            if let Some(app_row) = &report.application {
                if !app_row.score.is_empty() {
                    ui.label(
                        RichText::new(&app_row.score)
                            .size(30.0)
                            .monospace()
                            .color(t.score(app_row.score_value())),
                    );
                    ui.add_space(8.0);
                }
                if !app_row.status.is_empty() {
                    pill(ui, &app_row.status, t.accent);
                }
                if !app_row.date.is_empty() {
                    ui.label(
                        RichText::new(&app_row.date)
                            .monospace()
                            .text_style(theme::meta())
                            .color(t.fg_muted),
                    );
                }
            }
            if let Some(legitimacy) = &report.legitimacy {
                pill(
                    ui,
                    &format!("Legitimacy: {legitimacy}"),
                    t.legitimacy(legitimacy),
                );
            }
        });

        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            if let Some(url) = report.posting_url.as_deref() {
                if crate::views::primary_button(ui, "Open the source posting", true).clicked() {
                    crate::views::pipeline::open_url(ui, url);
                }
                if crate::views::ghost_button(ui, "Copy the posting URL").clicked() {
                    ui.ctx().copy_text(url.to_string());
                }
            }
            if let Some(path) = report.cv_pdf.as_deref() {
                if crate::views::ghost_button(ui, "Open the tailored CV (PDF)").clicked() {
                    open_file(path);
                }
            }
            if let Some(path) = report.cover_pdf.as_deref() {
                if crate::views::ghost_button(ui, "Open the cover letter (PDF)").clicked() {
                    open_file(path);
                }
            }
        });

        if report.posting_url.is_none() {
            ui.add_space(6.0);
            ui.label(
                RichText::new("No source posting URL is recorded for this application.")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        }
    });
}

/// Open a file in the user's desktop environment.
pub fn open_file(path: &str) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}
