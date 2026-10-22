//! The follow-ups screen: what is due, what was logged, and the log dialog.

use egui::{Align, Layout, Margin, RichText};

use crate::app::CareerOpsApp;
use crate::model::FollowupEntry;
use crate::theme::{self, Palette};
use crate::views::{frame, ghost_button, pill, primary_button, section};

/// Draw the follow-ups screen.
pub fn ui(ui: &mut egui::Ui, app: &mut CareerOpsApp) {
    let t = theme::palette(ui.ctx());
    let Some(followups) = app.state.followups.clone() else {
        ui.spinner();
        ui.label("Reading the follow-ups…");
        return;
    };

    if !followups.followups_enabled {
        frame(
            ui,
            "Follow-ups",
            Some("Follow-ups are turned off in config/profile.yml."),
            app,
            |_ui, _app| {},
        );
        return;
    }

    frame(
        ui,
        "Follow-ups",
        Some("The cadence verdict comes from the core's own calculator."),
        app,
        |ui, app| {
            toolbar(ui, app, t);
            ui.add_space(10.0);

            if !followups.available {
                ui.horizontal_wrapped(|ui| {
                    pill(ui, "unavailable", t.warn);
                    ui.label(
                        RichText::new(
                            "The cadence calculator is not available in this checkout.",
                        )
                        .color(t.warn),
                    );
                });
                return;
            }

            let needle = app.followup_view.search.trim().to_lowercase();
            let mut shown = 0usize;
            for entry in &followups.entries {
                if app.followup_view.only_due
                    && !matches!(entry.urgency.as_str(), "urgent" | "overdue")
                {
                    continue;
                }
                if !needle.is_empty()
                    && !entry.company.to_lowercase().contains(&needle)
                    && !entry.role.to_lowercase().contains(&needle)
                {
                    continue;
                }
                shown += 1;
                entry_row(ui, app, entry, t);
            }
            if shown == 0 {
                ui.label(RichText::new("Nothing to follow up on.").color(t.fg_muted));
            }

            ui.add_space(16.0);
            section(ui, "Logged follow-ups");
            if followups.history.is_empty() {
                ui.label(
                    RichText::new("No follow-up has been logged yet.").color(t.fg_muted),
                );
            }
            for item in &followups.history {
                let num = item.num.map(|n| n.to_string());
                theme::card(ui, t)
                    .inner_margin(Margin::symmetric(14, 9))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "#{}",
                                    item.num.map(|n| n.to_string()).unwrap_or_else(|| "—".into())
                                ))
                                .monospace()
                                .color(t.fg_muted),
                            );
                            ui.label(
                                RichText::new(&item.date)
                                    .monospace()
                                    .text_style(theme::meta())
                                    .color(t.fg_muted),
                            );
                            ui.label(RichText::new(&item.company).strong().color(t.fg));
                            pill(ui, &item.channel, t.accent);
                            if !item.notes.is_empty() {
                                ui.label(RichText::new(&item.notes).color(t.fg_muted));
                            }
                            if let Some(num) = &num {
                                if ghost_button(ui, "Remove").clicked() {
                                    app.delete_followup(num);
                                }
                            }
                        });
                    });
                ui.add_space(6.0);
            }
        },
    );
}

/// Search, the due-only switch, and what the cadence calculator reported.
fn toolbar(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).inner_margin(Margin::symmetric(14, 10)).show(
        ui,
        |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut app.followup_view.only_due, "Only due");
                ui.add_space(12.0);
                ui.label(
                    RichText::new("Search")
                        .text_style(theme::meta())
                        .color(t.fg_muted),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut app.followup_view.search)
                        .desired_width(240.0)
                        .hint_text("company or role"),
                );
            });
        },
    );
}

/// One follow-up row.
fn entry_row(ui: &mut egui::Ui, app: &mut CareerOpsApp, entry: &FollowupEntry, t: Palette) {
    theme::card(ui, t)
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!("#{}", entry.num))
                        .monospace()
                        .color(t.fg_muted),
                );
                ui.label(RichText::new(&entry.company).strong().color(t.fg));
                if !entry.role.is_empty() {
                    ui.label(RichText::new(&entry.role).color(t.fg_muted));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ghost_button(ui, "Pin the next date").clicked() {
                        app.followup_view.pinning_for = Some(entry.num.to_string());
                        app.followup_view.pin_date = entry
                            .next_followup_date
                            .clone()
                            .unwrap_or_else(|| String::from("2026-01-01"));
                    }
                    if ghost_button(ui, "Seed the first follow-up").clicked() {
                        app.seed_followup(&entry.num.to_string());
                    }
                    if ghost_button(ui, "Log a follow-up").clicked() {
                        app.followup_view.logging_for = Some(entry.num.to_string());
                        app.followup_view.log_channel = "Email".to_string();
                        app.followup_view.log_notes.clear();
                        app.followup_view.log_contact.clear();
                    }
                    let due = match entry.days_until_next {
                        Some(days) if days < 0 => format!("{} days overdue", -days),
                        Some(0) => "due today".to_string(),
                        Some(days) => format!("in {days} days"),
                        None => "no date".to_string(),
                    };
                    pill(ui, &due, t.urgency(&entry.urgency));
                });
            });
            if entry.followup_count > 0 {
                ui.label(
                    RichText::new(format!(
                        "{} follow-up(s) logged{}",
                        entry.followup_count,
                        entry
                            .days_since_last_followup
                            .map(|d| format!(", last {d} days ago"))
                            .unwrap_or_default()
                    ))
                    .text_style(theme::meta())
                    .color(t.fg_muted),
                );
            }
        });
    ui.add_space(6.0);

    let target = entry.num.to_string();
    if app.followup_view.logging_for.as_deref() == Some(target.as_str()) {
        log_dialog(ui, app, entry, t);
    }
    if app.followup_view.pinning_for.as_deref() == Some(target.as_str()) {
        pin_dialog(ui, app, entry, t);
    }
}

/// The dialog that pins (or clears) the next follow-up date.
fn pin_dialog(ui: &mut egui::Ui, app: &mut CareerOpsApp, entry: &FollowupEntry, t: Palette) {
    theme::hero_card(ui, t).show(ui, |ui| {
        ui.label(
            RichText::new(format!(
                "Next follow-up for #{} — {}",
                entry.num, entry.company
            ))
            .strong()
            .color(t.fg),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Date (YYYY-MM-DD)")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.followup_view.pin_date)
                    .desired_width(140.0),
            );
            if primary_button(ui, "Pin", true).clicked() {
                let date = app.followup_view.pin_date.trim().to_string();
                app.pin_followup(&entry.num.to_string(), &date);
            }
            if ghost_button(ui, "Clear the pin").clicked() {
                app.unpin_followup(&entry.num.to_string());
            }
            if ghost_button(ui, "Cancel").clicked() {
                app.followup_view.pinning_for = None;
            }
        });
    });
    ui.add_space(6.0);
}

/// The dialog that appends a row to data/follow-ups.md.
fn log_dialog(ui: &mut egui::Ui, app: &mut CareerOpsApp, entry: &FollowupEntry, t: Palette) {
    theme::hero_card(ui, t).show(ui, |ui| {
        ui.label(
            RichText::new(format!("Log a follow-up for #{} — {}", entry.num, entry.company))
                .strong()
                .color(t.fg),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Channel").text_style(theme::meta()).color(t.fg_muted));
            egui::ComboBox::from_id_salt("followup-channel")
                .selected_text(app.followup_view.log_channel.clone())
                .width(120.0)
                .show_ui(ui, |ui| {
                    for channel in ["Email", "LinkedIn", "Phone", "Other"] {
                        ui.selectable_value(
                            &mut app.followup_view.log_channel,
                            channel.to_string(),
                            channel,
                        );
                    }
                });
            ui.add_space(10.0);
            ui.label(RichText::new("Contact").text_style(theme::meta()).color(t.fg_muted));
            ui.add(
                egui::TextEdit::singleline(&mut app.followup_view.log_contact)
                    .desired_width(200.0),
            );
        });
        ui.add_space(6.0);
        ui.label(RichText::new("Notes").text_style(theme::meta()).color(t.fg_muted));
        ui.add(
            egui::TextEdit::multiline(&mut app.followup_view.log_notes)
                .desired_width(f32::INFINITY)
                .desired_rows(2),
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if primary_button(ui, "Save", true).clicked() {
                app.log_followup(&entry.num.to_string(), &entry.company, &entry.role);
            }
            if ghost_button(ui, "Cancel").clicked() {
                app.followup_view.logging_for = None;
            }
        });
    });
    ui.add_space(6.0);
}
