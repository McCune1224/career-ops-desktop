//! The CV screen: read and edit cv.md, with a live preview.

use egui::{Margin, RichText};

use crate::app::CareerOpsApp;
use crate::theme::{self, Palette};
use crate::views::{frame, pill};

/// The checks: the CV's structure, and the gaps the pipeline shows.
fn checks_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Checks", |ui| {
            ui.label(
                RichText::new("read-only — neither of these changes your CV")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if crate::views::ghost_button(ui, "Check the structure").clicked() {
                app.check_cv_structure();
            }
            if crate::views::ghost_button(ui, "Skill gaps").clicked() {
                app.load_skill_gaps();
            }
            if crate::views::ghost_button(ui, "Import from documents/").clicked() {
                app.run_op("profile", serde_json::json!({ "action": "intake" }));
            }
            if crate::views::ghost_button(ui, "Master career profile").clicked() {
                app.run_op("profile", serde_json::json!({ "action": "masterProfile" }));
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
        if let Some(check) = app.state.cv_check.clone() {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                pill(
                    ui,
                    &check.verdict,
                    if check.verdict == "pass" { t.good } else { t.warn },
                );
            });
            for violation in &check.violations {
                ui.label(
                    RichText::new(format!("— {violation}"))
                        .text_style(theme::meta())
                        .color(t.warn),
                );
            }
        }
        if let Some(analysis) = app.state.analysis.clone() {
            if analysis.action == "upskill" {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(&analysis.title)
                        .text_style(theme::meta())
                        .strong()
                        .color(t.fg),
                );
                ui.label(RichText::new(&analysis.body).color(t.fg_muted));
            }
        }
    });
}

/// The template the `pdf` run fills.
fn template_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Template", |ui| {
            if crate::views::ghost_button(ui, "Reload").clicked() {
                app.load_cv_templates();
            }
        });
        ui.add_space(8.0);
        if app.state.cv_templates.is_empty() {
            ui.label(
                RichText::new("The template list has not been read yet.")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        }
        for name in app.state.cv_templates.clone() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&name).monospace().color(t.fg));
                if crate::views::ghost_button(ui, "Use this one").clicked() {
                    app.set_cv_template(&name);
                }
            });
        }
    });
}

/// Draw the CV screen.
pub fn ui(ui: &mut egui::Ui, app: &mut CareerOpsApp) {
    let t = theme::palette(ui.ctx());
    frame(
        ui,
        "CV",
        Some("cv.md is the source of truth. Every tailored CV is generated from it."),
        app,
        |ui, app| {
            ui.horizontal(|ui| {
                if crate::views::primary_button(ui, "Save", app.cv_editor.dirty).clicked() {
                    app.save_cv();
                }
                if crate::views::ghost_button(ui, "Reload").clicked() {
                    app.cv_editor.dirty = false;
                    app.load_cv();
                }
                if app.cv_editor.dirty {
                    pill(ui, "Unsaved changes", t.warn);
                }
                if let Some(state) = &app.state.bootstrap {
                    ui.label(
                        RichText::new(format!(
                            "{} report(s) · {} PDF(s) in output/",
                            state.reports, state.pdfs
                        ))
                        .text_style(theme::meta())
                        .color(t.fg_muted),
                    );
                }
            });
            ui.add_space(10.0);

            // Two panes of equal weight: the source and what it will look like.
            ui.columns(2, |cols| {
                let source_frame = theme::card(&cols[0], t).inner_margin(Margin::symmetric(12, 10));
                source_frame.show(&mut cols[0], |ui| {
                    ui.label(
                        RichText::new("SOURCE")
                            .text_style(theme::meta())
                            .color(t.fg_muted),
                    );
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .id_salt("cv-source")
                        .max_height(430.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let response = ui.add(
                                egui::TextEdit::multiline(&mut app.cv_editor.content)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(34)
                                    .code_editor(),
                            );
                            if response.changed() {
                                app.cv_editor.dirty = true;
                            }
                        });
                });

                let preview_frame = theme::card(&cols[1], t).inner_margin(Margin::symmetric(12, 10));
                preview_frame.show(&mut cols[1], |ui| {
                    ui.label(
                        RichText::new("Preview")
                            .text_style(theme::meta())
                            .color(t.fg_muted),
                    );
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .id_salt("cv-preview")
                        .max_height(430.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            crate::markdown::render(ui, &app.cv_editor.content);
                        });
                });
            });

            ui.add_space(8.0);
            if let Some(path) = app.state.cv.as_ref().map(|c| c.path.clone()) {
                ui.label(
                    RichText::new(path)
                        .monospace()
                        .text_style(theme::meta())
                        .color(t.fg_muted),
                );
            }

            ui.add_space(14.0);
            checks_card(ui, app, t);
            ui.add_space(14.0);
            template_card(ui, app, t);
        },
    );
}
