//! The portals screen: what is new, which boards are healthy, why a posting was
//! missed, and which roles keep being re-listed.
//!
//! Every answer is the core's own: coverage-check.mjs explains a miss,
//! audit-portals.mjs judges a board, detect-reposts.mjs finds a repeated role,
//! and the web app's whats-new module lists what the last scan found.

use egui::RichText;

use crate::app::CareerOpsApp;
use crate::theme::{self, Palette};
use crate::views::frame;

/// The coverage-check form.
#[derive(Debug, Clone, Default)]
pub struct CoverageForm {
    pub url: String,
    pub company: String,
    pub title: String,
    pub gap: String,
    pub meaning: String,
    pub fix: String,
    pub evidence: Vec<String>,
}

/// The discover form.
#[derive(Debug, Clone, Default)]
pub struct DiscoverForm {
    pub companies: String,
    pub output: String,
}

/// Draw the portals screen.
pub fn ui(ui: &mut egui::Ui, app: &mut CareerOpsApp) {
    let t = theme::palette(ui.ctx());
    frame(
        ui,
        "Portals",
        Some("What the last scan found, and the boards it found it on."),
        app,
        |ui, app| {
            whats_new_card(ui, app, t);
            ui.add_space(14.0);
            boards_card(ui, app, t);
            ui.add_space(14.0);
            coverage_card(ui, app, t);
            ui.add_space(14.0);
            checks_card(ui, app, t);
            ui.add_space(14.0);
            discover_card(ui, app, t);
        },
    );
}

/// What is new this week.
fn whats_new_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "New this week", |ui| {
            if crate::views::ghost_button(ui, "Reload").clicked() {
                app.load_whats_new();
            }
        });
        ui.add_space(8.0);
        let Some(whats) = app.state.whats_new.clone() else {
            ui.label(
                RichText::new("The scan history has not been read yet.")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            return;
        };
        ui.label(
            RichText::new(format!(
                "{} posting(s) surfaced in the last {} day(s)",
                whats.count, whats.days
            ))
            .text_style(theme::meta())
            .color(t.fg_muted),
        );
        if whats.offers.is_empty() {
            ui.label(
                RichText::new("Nothing new, or nothing left to look at.")
                    .color(t.fg_muted),
            );
            return;
        }
        for offer in whats.offers.iter().take(20) {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&offer.company).strong().color(t.fg));
                ui.label(RichText::new(&offer.title).color(t.fg_muted));
                if !offer.location.is_empty() {
                    ui.label(
                        RichText::new(&offer.location)
                            .text_style(theme::meta())
                            .color(t.fg_muted),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::views::ghost_button(ui, "Open").clicked() {
                        crate::views::pipeline::open_url(ui, &offer.url);
                    }
                });
            });
            ui.add_space(3.0);
        }
    });
}

/// The watched boards, as the audit judged them.
fn boards_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Watched boards", |ui| {
            if crate::views::ghost_button(ui, "Audit").clicked() {
                app.load_portals();
            }
        });
        ui.add_space(8.0);
        let Some(boards) = app.state.portals.clone() else {
            ui.label(
                RichText::new("The audit has not run yet.")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            return;
        };
        if boards.is_empty() {
            ui.label(
                RichText::new("No boards are watched yet.").color(t.fg_muted),
            );
            return;
        }
        for board in &boards {
            let tone = match board.verdict.as_str() {
                "ok" => t.good,
                "small" => t.warn,
                _ => t.bad,
            };
            ui.horizontal(|ui| {
                crate::views::pill(ui, &board.verdict, tone);
                ui.label(RichText::new(&board.name).strong().color(t.fg));
                if !board.provider.is_empty() && board.provider != "—" {
                    ui.label(
                        RichText::new(&board.provider)
                            .text_style(theme::meta())
                            .color(t.fg_muted),
                    );
                }
                if let Some(count) = board.count {
                    ui.label(
                        RichText::new(format!("{count} posting(s)"))
                            .monospace()
                            .text_style(theme::meta())
                            .color(t.fg_muted),
                    );
                }
            });
            if let Some(detail) = &board.detail {
                ui.label(
                    RichText::new(detail)
                        .text_style(theme::meta())
                        .color(t.fg_muted),
                );
            }
            ui.add_space(4.0);
        }
    });
}

/// Why a posting you found by hand was missed.
fn coverage_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Why was this missed?", |ui| {
            ui.label(
                RichText::new("offline — no tokens, no network")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.coverage.url)
                    .desired_width(240.0)
                    .hint_text("posting URL"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.coverage.company)
                    .desired_width(140.0)
                    .hint_text("company"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.coverage.title)
                    .desired_width(140.0)
                    .hint_text("title"),
            );
            if crate::views::primary_button(ui, "Explain", !app.coverage.url.trim().is_empty()).clicked() {
                explain_coverage(app);
            }
        });
        if !app.coverage.gap.is_empty() {
            ui.add_space(8.0);
            crate::views::pill(ui, &app.coverage.gap, t.accent);
            ui.label(RichText::new(&app.coverage.meaning).color(t.fg));
            if !app.coverage.fix.is_empty() {
                ui.label(
                    RichText::new(format!("fix: {}", app.coverage.fix))
                        .text_style(theme::meta())
                        .color(t.fg_muted),
                );
            }
            for line in &app.coverage.evidence {
                ui.label(
                    RichText::new(format!("— {line}"))
                        .text_style(theme::meta())
                        .color(t.fg_muted),
                );
            }
        }
    });
}

/// Ask the core why a posting was missed.
fn explain_coverage(app: &mut CareerOpsApp) {
    let params = serde_json::json!({
        "action": "coverage",
        "url": app.coverage.url,
        "company": app.coverage.company,
        "title": app.coverage.title,
    });
    match app.backend.call("portals", params) {
        Ok(value) => {
            app.coverage.gap = value
                .get("gap")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();
            app.coverage.meaning = value
                .get("meaning")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();
            app.coverage.fix = value
                .get("fix")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();
            app.coverage.evidence = value
                .get("evidence")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .map(|v| v.as_str().unwrap_or("").to_string())
                        .collect()
                })
                .unwrap_or_default();
        }
        Err(e) => app.state.fail(format!("could not explain the miss: {e}")),
    }
}

/// The checks: a re-listed role, a posting's liveness, a JD without a browser.
fn checks_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Checks", |ui| {
            ui.label(
                RichText::new("the network is only used by the liveness check")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if crate::views::ghost_button(ui, "Re-listed roles").clicked() {
                match app
                    .backend
                    .call("portals", serde_json::json!({ "action": "reposts" }))
                {
                    Ok(value) => {
                        let count = value
                            .get("clusters")
                            .and_then(serde_json::Value::as_array)
                            .map(|c| c.len())
                            .unwrap_or(0);
                        app.state.say(format!("{count} re-listed role(s)"));
                    }
                    Err(e) => app.state.fail(format!("could not check the history: {e}")),
                }
            }
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.coverage.url)
                    .desired_width(260.0)
                    .hint_text("a posting URL to check"),
            );
            if crate::views::ghost_button(ui, "Live?").clicked() {
                let url = app.coverage.url.clone();
                app.check_liveness(&url);
            }
            if crate::views::ghost_button(ui, "JD without a browser").clicked() {
                let url = app.coverage.url.clone();
                app.run_op("liveness", serde_json::json!({ "action": "fetchJd", "url": url }));
            }
        });
        if let Some(liveness) = app.state.liveness.clone() {
            ui.add_space(6.0);
            crate::views::pill(
                ui,
                &liveness.verdict,
                if liveness.verdict == "active" { t.good } else { t.warn },
            );
        }
        if !app.state.notice.is_empty() {
            ui.add_space(6.0);
            ui.label(
                RichText::new(&app.state.notice)
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        }
    });
}

/// Find a company's board from its name.
fn discover_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Find a company's board", |ui| {
            ui.label(
                RichText::new("needs the network")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.discover.companies)
                    .desired_width(300.0)
                    .hint_text("company names, comma separated"),
            );
            if crate::views::primary_button(ui, "Find", !app.discover.companies.trim().is_empty()).clicked() {
                discover(app);
            }
        });
        if !app.discover.output.is_empty() {
            ui.add_space(6.0);
            ui.label(
                RichText::new(&app.discover.output)
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        }
    });
}

/// Ask the core to resolve company names into boards.
fn discover(app: &mut CareerOpsApp) {
    let names: Vec<String> = app
        .discover
        .companies
        .split(',')
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .collect();
    let params = serde_json::json!({ "action": "discover", "names": names });
    match app.backend.call("portals", params) {
        Ok(value) => {
            app.discover.output = value
                .get("output")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();
        }
        Err(e) => app.state.fail(format!("could not search for boards: {e}")),
    }
}
