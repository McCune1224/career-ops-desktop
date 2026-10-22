//! The run screen: evaluate a pasted posting, tailor the CV, draft a cover
//! letter — with the same streamed progress the web app shows.

use egui::{Color32, Margin, RichText};

use crate::app::{CareerOpsApp, LogKind, RunKind, RUN_KINDS};
use crate::theme::{self, Palette};
use crate::views::{frame, pill, section};

/// Draw the run screen.
pub fn ui(ui: &mut egui::Ui, app: &mut CareerOpsApp) {
    let t = theme::palette(ui.ctx());
    frame(
        ui,
        "Run",
        Some("The agent CLI does the work; the app never writes what the agent produced."),
        app,
        |ui, app| {
            form_card(ui, app, t);
            ui.add_space(14.0);
            sensor_card(ui, app, t);
            ui.add_space(14.0);
            progress_card(ui, app, t);
        },
    );
}

/// The kind picker and the input: the part of the screen you act on.
fn form_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for kind in RUN_KINDS {
                let selected = app.run.kind == Some(kind);
                let (fill, stroke, color) = if selected {
                    (t.tint(t.accent).gamma_multiply(2.4), t.accent, t.fg)
                } else {
                    (Color32::TRANSPARENT, t.border, t.fg_muted)
                };
                let button = egui::Button::new(
                    RichText::new(kind.label())
                        .text_style(theme::meta())
                        .color(color),
                )
                .fill(fill)
                .stroke(egui::Stroke::new(1.0, stroke))
                .corner_radius(egui::CornerRadius::same(6))
                .min_size(egui::vec2(0.0, 26.0));
                if ui.add(button).clicked() {
                    app.run.kind = Some(kind);
                    // The cost of THIS kind, from this client's own history.
                    app.estimate_cost(kind.id(), 1);
                }
            }
        });

        ui.add_space(8.0);
        if let Some(kind) = app.run.kind {
            ui.label(RichText::new(kind.hint()).color(t.fg_muted));
        }
        ui.add_space(10.0);

        ui.label(
            RichText::new(match app.run.kind {
                Some(RunKind::Evaluate) => "Job URL or posting text",
                Some(RunKind::Pdf) | Some(RunKind::Cover) => "Report number or company",
                Some(RunKind::Email) | Some(RunKind::Prep) | Some(RunKind::Plan) => {
                    "Report number or company"
                }
                Some(RunKind::Practice) => "A question, or a report number",
                Some(RunKind::Debrief) => "What happened in the interview",
                Some(RunKind::RedFlag) | Some(RunKind::Contacto) => "The company name",
                Some(RunKind::OfferPrep) => "The offer text",
                Some(RunKind::FixPortal) => "The company name",
                None => "Input",
            })
            .text_style(theme::meta())
            .color(t.fg_muted),
        );
        ui.add(
            egui::TextEdit::singleline(&mut app.run.input)
                .desired_width(f32::INFINITY)
                .hint_text("https://…"),
        );

        if matches!(app.run.kind, Some(RunKind::Cover)) {
            ui.add_space(8.0);
            ui.label(
                RichText::new("The letter's four answers — the mode asks for these")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.run.cover_why)
                    .desired_width(f32::INFINITY)
                    .hint_text("why this role"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.run.cover_problem)
                    .desired_width(f32::INFINITY)
                    .hint_text("the problem to solve"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.run.cover_approach)
                    .desired_width(f32::INFINITY)
                    .hint_text("your approach"),
            );
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Tone")
                        .text_style(theme::meta())
                        .color(t.fg_muted),
                );
                egui::ComboBox::from_id_salt("cover-tone")
                    .selected_text(RichText::new(&app.cover_tone).color(t.fg))
                    .show_ui(ui, |ui| {
                        for tone in COVER_TONES {
                            if ui
                                .selectable_label(&app.cover_tone == tone, tone)
                                .clicked()
                            {
                                app.cover_tone = tone.to_string();
                            }
                        }
                    });
            });
        }

        if let Some(estimate) = app.run.estimate.clone() {
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!("This kind of run: {}", estimate.text))
                        .text_style(theme::meta())
                        .color(t.fg_muted),
                );
                if estimate.requires_confirmation {
                    if let Some(confirmation) = &estimate.confirmation {
                        pill(ui, "spend", t.warn);
                        ui.label(
                            RichText::new(confirmation)
                                .text_style(theme::meta())
                                .color(t.warn),
                        );
                    }
                }
            });
        }

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            let cli = app
                .state
                .bootstrap
                .as_ref()
                .and_then(|b| b.clis.iter().find(|c| c.id == app.settings.cli_id).cloned());
            let ready = cli.as_ref().map(|c| c.installed).unwrap_or(false);
            let can_start = app.run_ready();

            if crate::views::primary_button(ui, "Start", can_start).clicked() {
                start(app);
            }
            if app.run.running {
                ui.spinner();
                ui.label(RichText::new("Running…").color(t.accent));
                if let Some((elapsed, silent)) = app.run_clock(ui.input(|i| i.time)) {
                    ui.label(
                        RichText::new(format!("elapsed {}", short_duration(elapsed)))
                            .monospace()
                            .text_style(theme::meta())
                            .color(t.fg_muted),
                    );
                    if silent >= crate::app::SILENCE_NOTICE_AFTER_SECS {
                        // A silent CLI and a hung one look the same from here.
                        // Say which it is: the transport is answering, the
                        // agent is not — and that is normal for a CLI that
                        // buffers until it is finished.
                        pill(ui, &format!("no output for {}", short_duration(silent)), t.warn);
                    }
                }
            }
            if !app.run.running
                && !app.run.log.is_empty()
                && crate::views::ghost_button(ui, "Clear the log").clicked()
            {
                app.run.log.clear();
                app.run.finished = false;
                app.run.error = None;
                app.run.report_n = None;
                app.run.started_at = None;
                app.run.last_output_at = None;
            }

            ui.label(
                RichText::new(match &cli {
                    Some(cli) if cli.installed => format!("CLI: {}", cli.name),
                    _ => "No usable CLI is selected — pick one in Config.".to_string(),
                })
                .text_style(theme::meta())
                .color(if ready { t.fg_muted } else { t.warn }),
            );
        });

        if let Some(error) = &app.run.error {
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                pill(ui, "error", t.bad);
                ui.label(RichText::new(error).color(t.bad));
            });
        }
        if let Some(report_n) = app.run.report_n.clone() {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                pill(ui, "saved", t.good);
                ui.label(
                    RichText::new(format!("report #{report_n}"))
                        .monospace()
                        .color(t.fg),
                );
                if crate::views::ghost_button(ui, "Open it").clicked() {
                    app.open_report(&report_n);
                }
            });
        }
        if app.run.finished && !app.run.running {
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                // A finished run leaves a readable record, so the CLI assistant
                // can be asked "what did we find on that role?" later.
                if crate::views::ghost_button(ui, "Save the run log").clicked() {
                    save_run_log(app);
                }
                if let Some(tokens) = app.run.tokens {
                    ui.label(
                        RichText::new(format!("{tokens} tokens"))
                            .monospace()
                            .text_style(theme::meta())
                            .color(t.fg_muted),
                    );
                }
                if let Some(cost) = app.run.cost_usd {
                    ui.label(
                        RichText::new(format!("${cost:.4}"))
                            .monospace()
                            .text_style(theme::meta())
                            .color(t.fg_muted),
                    );
                }
            });
        }
    });
}

/// What this client has spent, in the windows the account meters.
fn sensor_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "What you have spent", |ui| {
            if crate::views::ghost_button(ui, "Refresh").clicked() {
                app.load_usage_sensor();
            }
        });
        ui.add_space(8.0);
        let Some(usage) = app.state.usage.clone() else {
            ui.label(
                RichText::new("No run has been recorded yet.")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            return;
        };
        ui.horizontal_wrapped(|ui| {
            for window in &usage.windows {
                crate::views::pill(
                    ui,
                    &format!(
                        "{}h: {} tokens · ${:.4} over {} run(s)",
                        window.hours, window.tokens as i64, window.usd, window.runs
                    ),
                    t.accent,
                );
            }
        });
    });
}

/// The streamed log: the other half of the screen, and the reason it exists.
fn progress_card(ui: &mut egui::Ui, app: &CareerOpsApp, t: Palette) {
    section(ui, "Progress");
    theme::card(ui, t).inner_margin(Margin::symmetric(14, 12)).show(
        ui,
        |ui| {
            if app.run.log.is_empty() {
                ui.label(
                    RichText::new("Nothing yet. Start a run and its steps appear here.")
                        .color(t.fg_muted),
                );
                return;
            }
            egui::ScrollArea::vertical()
                .max_height(280.0)
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in &app.run.log {
                        let color = match line.kind {
                            LogKind::Tool => t.info,
                            LogKind::Status => t.fg,
                            LogKind::Error => t.bad,
                            LogKind::Done => t.good,
                            LogKind::Text => t.fg_muted,
                        };
                        let text = RichText::new(&line.text).color(color);
                        let text = match line.kind {
                            LogKind::Status | LogKind::Done => text.strong(),
                            LogKind::Tool => text.monospace(),
                            _ => text,
                        };
                        ui.label(text);
                    }
                });
        },
    );
}

/// The tones a cover letter can be written in, from the engine's own set.
pub const COVER_TONES: [&str; 4] = ["formal", "direct", "conversational", "mirror"];

/// "45s" / "2m 14s", for a clock a person reads at a glance.
pub fn short_duration(seconds: f64) -> String {
    let total = seconds.max(0.0).round() as u64;
    if total < 60 {
        format!("{total}s")
    } else if total < 3600 {
        format!("{}m {:02}s", total / 60, total % 60)
    } else {
        format!("{}h {:02}m", total / 3600, (total % 3600) / 60)
    }
}

/// Save the finished run, so it can be read back and costed.
fn save_run_log(app: &mut CareerOpsApp) {
    let kind = app.run.kind.map(|k| k.id()).unwrap_or("evaluate");
    let output = app
        .run
        .log
        .iter()
        .filter(|line| line.kind == LogKind::Text)
        .map(|line| line.text.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let id = format!("{kind}-{}", app.run.report_n.clone().unwrap_or_else(|| "run".into()));
    app.save_run(
        &id,
        kind,
        &app.run.input.clone(),
        &output,
        app.run.tokens,
        app.run.cost_usd,
    );
}

/// Hand the run to the engine.
pub fn start(app: &mut CareerOpsApp) {
    let Some(kind) = app.run.kind else { return };
    let input = app.run.input.trim().to_string();
    if input.is_empty() {
        return;
    }
    app.run.log.clear();
    app.run.error = None;
    app.run.finished = false;
    app.run.report_n = None;
    // The cover brief is the engine's OBJECT of four answers and a tone; the
    // client sends it as JSON text, which the bridge parses back.
    let brief = if kind == RunKind::Cover {
        let payload = serde_json::json!({
            "why": app.run.cover_why,
            "problem": app.run.cover_problem,
            "approach": app.run.cover_approach,
            "tone": app.cover_tone,
        });
        Some(payload.to_string())
    } else if app.run.brief.trim().is_empty() {
        None
    } else {
        Some(app.run.brief.clone())
    };
    match app
        .backend
        .start_run(kind.id(), &input, &app.settings.cli_id, brief.as_deref())
    {
        Ok(id) => {
            let now = app.run_now;
            app.run.running = true;
            app.run.request_id = Some(id);
            app.run.started_at = Some(now);
            app.run.last_output_at = Some(now);
            app.run.log.push(crate::app::LogLine {
                text: format!("Started the {} run…", kind.label()),
                kind: LogKind::Status,
            });
        }
        Err(e) => {
            app.run.running = false;
            app.run.error = Some(format!("could not start the run: {e}"));
        }
    }
}
