//! The report screen: one application's evaluation, and the things that hang
//! off it (the source posting, the tailored CV, the cover letter).
//!
//! The body is FOLDED: the lead and the verdict are open, every other block is
//! one click away. A report you cannot scan is a report you avoid, and the
//! blocks are split by the web app's own splitter, so "block B" means the same
//! thing here as it does there.

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

        body_card(ui, &report, t);
        ui.add_space(12.0);
        checks_card(ui, app, &report, t);
        ui.add_space(12.0);

        ui.horizontal_wrapped(|ui| {
            if crate::views::ghost_button(ui, "Back to the pipeline").clicked() {
                app.go(crate::app::Screen::Pipeline);
            }
            // Re-running is the real action here, and it is worth prefilling:
            // a re-evaluation of the same posting should not need a retyped URL.
            if let Some(url) = report.posting_url.clone() {
                if crate::views::ghost_button(ui, "Re-run the evaluation").clicked() {
                    app.run.kind = Some(crate::app::RunKind::Evaluate);
                    app.run.input = url;
                    app.go(crate::app::Screen::Run);
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

        // The one-line "why this score", the same prose the web app's verdict
        // card uses.
        if !report.lead.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new(&report.lead).color(t.fg));
        }
        // And the verdict itself: the blocks fold, but the answer never does.
        if let Some(verdict) = &report.verdict {
            ui.add_space(4.0);
            ui.label(
                RichText::new(verdict)
                    .monospace()
                    .strong()
                    .color(t.score(report.application.as_ref().map(|a| a.score_value()).unwrap_or(0.0))),
            );
        }

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

/// The body: the introduction, the lead block open, the rest folded.
fn body_card(ui: &mut egui::Ui, report: &crate::model::Report, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        if report.sections.is_empty() {
            if let Some(markdown_text) = report.markdown.as_deref() {
                markdown::render(ui, markdown_text);
            } else {
                ui.label(
                    RichText::new("This application has no report yet.")
                        .color(t.fg_muted),
                );
            }
            return;
        }
        if !report.intro.is_empty() {
            markdown::render(ui, &report.intro);
            ui.add_space(8.0);
        }
        for section in &report.sections {
            if section.lead {
                // The lead block is OPEN: it is the one that says whether this
                // is worth the user's time.
                ui.label(
                    RichText::new(&section.heading)
                        .text_style(theme::h2())
                        .strong()
                        .color(t.fg),
                );
                markdown::render(ui, &section.content);
                ui.add_space(8.0);
            } else {
                egui::CollapsingHeader::new(
                    RichText::new(&section.heading)
                        .text_style(theme::meta())
                        .strong()
                        .color(t.fg),
                )
                .id_salt(("report-section", section.heading.clone()))
                .show(ui, |ui| {
                    if !section.preview.is_empty() {
                        ui.label(
                            RichText::new(&section.preview)
                                .text_style(theme::meta())
                                .color(t.fg_muted),
                        );
                    }
                    markdown::render(ui, &section.content);
                });
            }
        }
    });
}

/// The checks: keyword coverage, score levers and the company's history.
///
/// Each is the core's own analysis, asked for on demand, and none of them ever
/// writes to the CV.
fn checks_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, report: &crate::model::Report, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Checks", |ui| {
            ui.label(
                RichText::new("read-only — none of these changes your CV")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if crate::views::ghost_button(ui, "Keyword coverage").clicked() {
                app.analysis_action("keywordMatch", &report.n);
            }
            if crate::views::ghost_button(ui, "Score levers").clicked() {
                app.analysis_action("scoreLevers", &report.n);
            }
            if let Some(company) = report.application.as_ref().map(|a| a.company.clone()) {
                if crate::views::ghost_button(ui, "Company history").clicked() {
                    app.analysis_action("companyHistory", &company);
                }
            }
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            if crate::views::ghost_button(ui, "Screening pack").clicked() {
                app.load_questions(&report.n);
            }
            if let Some(url) = report.posting_url.clone() {
                if crate::views::ghost_button(ui, "Is it still live?").clicked() {
                    app.check_liveness(&url);
                }
            }
        });
        if let Some(pack) = app.state.questions.clone() {
            ui.add_space(6.0);
            ui.label(
                RichText::new(pack)
                    .monospace()
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        }
        if let Some(liveness) = app.state.liveness.clone() {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                crate::views::pill(
                    ui,
                    &liveness.verdict,
                    if liveness.verdict == "active" { t.good } else { t.warn },
                );
                ui.label(
                    RichText::new(
                        "an unreadable posting is unconfirmed, never closed",
                    )
                    .text_style(theme::meta())
                    .color(t.fg_muted),
                );
            });
        }
        if let Some(result) = app.state.analysis.clone() {
            ui.add_space(8.0);
            ui.label(
                RichText::new(&result.title)
                    .text_style(theme::meta())
                    .strong()
                    .color(t.fg),
            );
            if !result.body.is_empty() {
                ui.label(RichText::new(&result.body).color(t.fg_muted));
            }
        }
    });
}

/// Open a file in the user's desktop environment.
pub fn open_file(path: &str) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}
