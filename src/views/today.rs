//! The landing screen: the day's state at a glance, and the way into a run.

use egui::{Align, Layout, RichText};

use crate::app::CareerOpsApp;
use crate::theme::{self, Palette};
use crate::views::{frame, metric, pill, section};

/// Draw the Today screen.
pub fn ui(ui: &mut egui::Ui, app: &mut CareerOpsApp) {
    let t = theme::palette(ui.ctx());
    frame(
        ui,
        "Today",
        Some("The daily loop: what is due, what is new, and what to run next."),
        app,
        |ui, app| {
            let Some(state) = app.state.bootstrap.clone() else {
                ui.spinner();
                ui.label("Reading the install…");
                return;
            };

            if !state.doctor.missing.is_empty() {
                theme::card(ui, t).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        pill(ui, "setup", t.warn);
                        ui.label(
                            RichText::new("This install is missing a prerequisite:")
                                .color(t.fg),
                        );
                    });
                    for item in &state.doctor.missing {
                        ui.label(RichText::new(format!("— {item}")).color(t.fg_muted));
                    }
                });
                ui.add_space(12.0);
            }

            let due = app
                .state
                .followups
                .as_ref()
                .map(|f| f.due.clone())
                .unwrap_or_default();
            let due_tint = if due.is_empty() { None } else { Some(t.warn) };

            ui.columns(4, |columns| {
                metric(
                    &mut columns[0],
                    &state.application_count.to_string(),
                    "Applications",
                    "in the tracker",
                    None,
                );
                metric(
                    &mut columns[1],
                    &state.inbox_count.to_string(),
                    "Pending postings",
                    "waiting in the inbox",
                    None,
                );
                metric(
                    &mut columns[2],
                    &state.reports.to_string(),
                    "Reports",
                    &format!("{} PDF(s) in output/", state.pdfs),
                    None,
                );
                metric(
                    &mut columns[3],
                    &due.len().to_string(),
                    "Follow-ups due",
                    if due.is_empty() { "nothing overdue" } else { "needs a nudge" },
                    due_tint,
                );
            });

            ui.add_space(16.0);
            section(ui, "Due now");
            if due.is_empty() {
                ui.label(RichText::new("Nothing is due.").color(t.fg_muted));
            }
            for entry in due.iter().take(8) {
                due_row(ui, t, entry);
            }

            ui.add_space(16.0);
            section(ui, "Next step");
            theme::hero_card(ui, t).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Evaluate a posting")
                                .text_style(theme::h2())
                                .strong()
                                .color(t.fg),
                        );
                        ui.label(
                            RichText::new(
                                "Paste a URL on the Run screen to score a posting against your CV.",
                            )
                            .color(t.fg_muted),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if crate::views::primary_button(ui, "Open the Run screen", true).clicked() {
                            app.screen = crate::app::Screen::Run;
                        }
                    });
                });
            });
        },
    );
}

/// One due follow-up, as a row you can act on.
fn due_row(ui: &mut egui::Ui, t: Palette, entry: &crate::model::FollowupEntry) {
    theme::card(ui, t).inner_margin(egui::Margin::symmetric(14, 10)).show(
        ui,
        |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("#{}", entry.num))
                        .monospace()
                        .color(t.fg_muted),
                );
                ui.label(RichText::new(&entry.company).strong().color(t.fg));
                ui.label(RichText::new(&entry.role).color(t.fg_muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let label = match entry.days_until_next {
                        Some(days) if days < 0 => format!("{} days overdue", -days),
                        Some(0) => "due today".to_string(),
                        Some(days) => format!("in {days} days"),
                        None => "no date".to_string(),
                    };
                    pill(ui, &label, t.urgency(&entry.urgency));
                });
            });
        },
    );
    ui.add_space(6.0);
}
