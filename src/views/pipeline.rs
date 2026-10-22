//! The tracker screen: the pipeline table with sorting, filtering, the
//! canonical status change, and the bulk changes that go with it.
//!
//! The status change goes through the core's set-status.mjs (through the
//! bridge), never through a write of the tracker's own. A bulk change is the
//! SAME writer, once per row, so each change gets its own ledger row.

use egui::{Align, Color32, CornerRadius, Layout, Margin, RichText, Stroke};
use egui_extras::{Column, TableBuilder};

use crate::app::{CareerOpsApp, SortKey};
use crate::model::Application;
use crate::theme::{self, Palette};
use crate::views::{filter_rows, frame, score_text, sort_rows};

/// The canonical states, in the order the core lists them. Kept here only as
/// the fallback when the engine has not answered yet; the engine's own
/// templates/states.yml is what the app renders once it has.
pub const FALLBACK_STATES: &[&str] = &[
    "Evaluated",
    "Applied",
    "Responded",
    "Assessment",
    "Interview",
    "Offer",
    "Hired",
    "Rejected",
    "Discarded",
    "SKIP",
];

/// The minimum width of each table column, in points, in column order.
///
/// A constant, and covered by a test, because the table is the screen most
/// likely to be squeezed and the least obvious when it is: an over-wide table
/// does not error, it clips the Actions column off the right edge. This is the
/// budget the layout is designed against — a window the window manager tiles to
/// a quarter of a 2560x1440 display is about 1104 points wide, minus the
/// sidebar and the page margins, which leaves roughly 840 for the table.
pub const TABLE_MIN_WIDTHS: [f32; 8] = [28.0, 34.0, 120.0, 110.0, 62.0, 120.0, 84.0, 130.0];

/// The narrowest canvas the tracker table is designed to fit.
pub const TABLE_CANVAS_MIN: f32 = 840.0;

/// Draw the tracker screen.
pub fn ui(ui: &mut egui::Ui, app: &mut CareerOpsApp) {
    let t = theme::palette(ui.ctx());
    let applications = match &app.state.pipeline {
        Some(pipeline) => pipeline.applications.clone(),
        None => {
            ui.spinner();
            ui.label("Reading the tracker…");
            return;
        }
    };
    let inbox = app
        .state
        .pipeline
        .as_ref()
        .map(|p| p.inbox.clone())
        .unwrap_or_default();
    let states: Vec<String> = app
        .state
        .bootstrap
        .as_ref()
        .map(|b| b.states.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| FALLBACK_STATES.iter().map(|s| s.to_string()).collect());
    let subtitle = format!(
        "{} applications · {} pending postings",
        applications.len(),
        inbox.len()
    );

    frame(ui, "Pipeline", Some(&subtitle), app, |ui, app| {
        filter_bar(ui, app, &states, t);
        ui.add_space(10.0);
        bulk_bar(ui, app, &states, t);

        let mut rows = filter_rows(
            applications,
            &app.pipeline_view.status_filter,
            app.pipeline_view.min_score,
            &app.pipeline_view.search,
        );
        sort_rows(&mut rows, app.pipeline_view.sort, app.pipeline_view.descending);

        if rows.is_empty() {
            ui.label(
                RichText::new("No application matches this filter.")
                    .color(t.fg_muted),
            );
        } else {
            table(ui, app, &rows, &states, t);
        }

        ui.add_space(14.0);
        compare_card(ui, app, t);
        ui.add_space(14.0);
        add_card(ui, app, t);
        ui.add_space(14.0);
        find_card(ui, app, t);
        ui.add_space(14.0);
        inbox_card(ui, app, &inbox, t);
    });
}

/// The tracker table.
fn table(ui: &mut egui::Ui, app: &mut CareerOpsApp, rows: &[Application], states: &[String], t: Palette) {
    let mut open: Option<String> = None;
    let mut status_change: Option<(String, String)> = None;
    let mut delete: Option<String> = None;

    theme::card(ui, t).inner_margin(Margin::symmetric(10, 8)).show(ui, |ui| {
        let table = TableBuilder::new(ui)
            .striped(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::auto().at_least(TABLE_MIN_WIDTHS[0]))
            .column(Column::auto().at_least(TABLE_MIN_WIDTHS[1]))
            .column(Column::auto().at_least(TABLE_MIN_WIDTHS[2]))
            .column(Column::initial(190.0).at_least(TABLE_MIN_WIDTHS[3]).clip(true))
            .column(Column::auto().at_least(TABLE_MIN_WIDTHS[4]))
            .column(Column::auto().at_least(TABLE_MIN_WIDTHS[5]))
            .column(Column::auto().at_least(TABLE_MIN_WIDTHS[6]))
            .column(Column::remainder().at_least(TABLE_MIN_WIDTHS[7]))
            .min_scrolled_height(160.0)
            .header(26.0, |mut header| {
                header.col(|ui| {
                    ui.label(
                        RichText::new("")
                            .text_style(theme::meta())
                            .color(t.fg_muted)
                            .strong(),
                    );
                });
                for key in [
                    SortKey::Tracker,
                    SortKey::Company,
                    SortKey::Role,
                    SortKey::Score,
                    SortKey::Status,
                    SortKey::Date,
                ] {
                    let label = key.label();
                    header.col(|ui| {
                        let active = app.pipeline_view.sort == key;
                        let arrow = if active {
                            if app.pipeline_view.descending {
                                " (desc)"
                            } else {
                                " (asc)"
                            }
                        } else {
                            ""
                        };
                        let color = if active { t.accent } else { t.fg_muted };
                        let button = egui::Button::new(
                            RichText::new(format!("{label}{arrow}"))
                                .text_style(theme::meta())
                                .color(color)
                                .strong(),
                        )
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE);
                        if ui.add(button).on_hover_text("Sort by this column").clicked() {
                            if app.pipeline_view.sort == key {
                                app.pipeline_view.descending = !app.pipeline_view.descending;
                            } else {
                                app.pipeline_view.sort = key;
                                app.pipeline_view.descending = false;
                            }
                        }
                    });
                }
                header.col(|ui| {
                    ui.label(
                        RichText::new("Actions")
                            .text_style(theme::meta())
                            .color(t.fg_muted)
                            .strong(),
                    );
                });
            });

        table.body(|body| {
            body.rows(theme::ROW_HEIGHT, rows.len(), |mut table_row| {
                let row = &rows[table_row.index()];
                let n = row.n.clone();
                let zebra = table_row.index() % 2 == 1;
                table_row.col(|ui| {
                    let mut picked = app.is_selected(&n);
                    let checkbox = egui::Checkbox::new(&mut picked, "");
                    if ui
                        .add(checkbox)
                        .on_hover_text("Pick this row for a bulk change")
                        .changed()
                    {
                        app.toggle_selection(&n);
                    }
                });
                table_row.col(|ui| {
                    ui.label(RichText::new(&row.n).monospace().color(t.fg_muted));
                });
                table_row.col(|ui| {
                    ui.label(RichText::new(&row.company).strong().color(t.fg));
                });
                table_row.col(|ui| {
                    ui.label(RichText::new(&row.role).color(t.fg_muted));
                });
                table_row.col(|ui| {
                    ui.label(score_text(t, &row.score, row.score_value()));
                });
                table_row.col(|ui| {
                    status_combo(ui, row, states, &mut status_change, t, zebra);
                });
                table_row.col(|ui| {
                    ui.label(
                        RichText::new(&row.date)
                            .monospace()
                            .text_style(theme::meta())
                            .color(t.fg_muted),
                    );
                });
                table_row.col(|ui| {
                    ui.horizontal(|ui| {
                        if row_action(ui, "Report", t).clicked() {
                            open = Some(n.clone());
                        }
                        if let Some(url) = posting_url(row) {
                            if row_action(ui, "Posting", t).on_hover_text(&url).clicked() {
                                open_url(ui, &url);
                            }
                        }
                        if row_action(ui, "Delete", t)
                            .on_hover_text("Remove this row; the report it had is left behind")
                            .clicked()
                        {
                            delete = Some(n.clone());
                        }
                    });
                });
            });
        });
    });

    if let Some(n) = open {
        app.open_report(&n);
    }
    if let Some((n, status)) = status_change {
        app.set_status(&n, &status);
    }
    if let Some(n) = delete {
        app.delete_row(&n);
    }
}

/// The bulk bar: what is picked, and the state to apply to it.
fn bulk_bar(ui: &mut egui::Ui, app: &mut CareerOpsApp, states: &[String], t: Palette) {
    let picked = app.pipeline_view.selected.len();
    if picked == 0 {
        return;
    }
    theme::card(ui, t).inner_margin(Margin::symmetric(14, 10)).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!("{picked} row(s) picked"))
                    .strong()
                    .color(t.fg),
            );
            ui.label(
                RichText::new("set them all to")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            let mut chosen = app.bulk_state.clone();
            egui::ComboBox::from_id_salt("bulk-status")
                .selected_text(RichText::new(&chosen).color(t.fg))
                .show_ui(ui, |ui| {
                    for state in states {
                        if ui.selectable_label(&chosen == state, state.as_str()).clicked() {
                            chosen = state.clone();
                        }
                    }
                });
            app.bulk_state = chosen.clone();
            if crate::views::primary_button(ui, "Apply", !chosen.is_empty()).clicked() {
                app.bulk_status(&chosen);
            }
            if crate::views::ghost_button(ui, "Clear").clicked() {
                app.pipeline_view.selected.clear();
            }
        });
    });
    ui.add_space(8.0);
}

/// Compare two or three applications side by side.
fn compare_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Compare", |ui| {
            ui.label(
                RichText::new("pick two or three rows above, then compare them")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if crate::views::primary_button(ui, "Compare the picked rows", app.pipeline_view.selected.len() >= 2)
                .clicked()
            {
                let rows = app.pipeline_view.selected.clone();
                app.analysis_action_ids("compare", &rows);
            }
        });
        if let Some(result) = app.state.analysis.clone() {
            if result.action == "compare" {
                ui.add_space(8.0);
                ui.label(RichText::new(&result.body).color(t.fg_muted));
            }
        }
    });
}

/// The "add an application by hand" form.
fn add_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Add an application by hand", |ui| {
            if !app.pipeline_view.adding && crate::views::ghost_button(ui, "Add").clicked() {
                app.pipeline_view.adding = true;
                if app.pipeline_view.new_date.is_empty() {
                    app.pipeline_view.new_date = today();
                }
            }
        });
        if !app.pipeline_view.adding {
            return;
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.pipeline_view.new_company)
                    .desired_width(160.0)
                    .hint_text("company"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.pipeline_view.new_role)
                    .desired_width(160.0)
                    .hint_text("role"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.pipeline_view.new_date)
                    .desired_width(100.0)
                    .hint_text("YYYY-MM-DD"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.pipeline_view.new_url)
                    .desired_width(200.0)
                    .hint_text("posting URL (optional)"),
            );
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("status")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            egui::ComboBox::from_id_salt("new-row-status")
                .selected_text(RichText::new(&app.pipeline_view.new_status).color(t.fg))
                .show_ui(ui, |ui| {
                    for state in FALLBACK_STATES {
                        if ui
                            .selectable_label(app.pipeline_view.new_status == *state, *state)
                            .clicked()
                        {
                            app.pipeline_view.new_status = state.to_string();
                        }
                    }
                });
            ui.label(
                RichText::new("the score is N/A until it is evaluated")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            if crate::views::primary_button(ui, "Save", !app.pipeline_view.new_company.trim().is_empty())
                .clicked()
            {
                app.add_application();
            }
            if crate::views::ghost_button(ui, "Cancel").clicked() {
                app.pipeline_view.adding = false;
            }
        });
    });
}

/// The free-text search, over the tracker and the reports.
fn find_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Find", |ui| {
            ui.label(
                RichText::new("offline")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.find_query)
                    .desired_width(280.0)
                    .hint_text("a report number, a tracker number, or a name"),
            );
            if crate::views::primary_button(ui, "Search", !app.find_query.trim().is_empty()).clicked() {
                app.search_tracker(&app.find_query.clone());
            }
        });
        if let Some(results) = app.state.search.clone() {
            ui.add_space(8.0);
            if results.matches.is_empty() {
                ui.label(
                    RichText::new(format!("nothing matches '{}'", results.query))
                        .text_style(theme::meta())
                        .color(t.fg_muted),
                );
            }
            for hit in results.matches.iter().take(20) {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("#{}", hit.tracker_num))
                            .monospace()
                            .color(t.fg_muted),
                    );
                    ui.label(RichText::new(&hit.company).strong().color(t.fg));
                    ui.label(RichText::new(&hit.role).color(t.fg_muted));
                    if !hit.status.is_empty() {
                        crate::views::pill(ui, &hit.status, t.accent);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if crate::views::ghost_button(ui, "Open").clicked() {
                            app.open_report(&hit.tracker_num);
                        }
                    });
                });
                ui.add_space(3.0);
            }
        }
    });
}

/// The pending-posting inbox, with the skip control and the shortlist.
fn inbox_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, inbox: &[crate::model::InboxJob], t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Pending postings", |ui| {
            ui.label(
                RichText::new(format!("{} in the inbox", inbox.len()))
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        });
        ui.add_space(8.0);
        if !app.shortlist.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!("{} shortlisted", app.shortlist.len()))
                        .strong()
                        .color(t.fg),
                );
                if crate::views::ghost_button(ui, "Score the shortlist").clicked() {
                    app.estimate_shortlist();
                }
                if crate::views::ghost_button(ui, "Clear the shortlist").clicked() {
                    app.shortlist.clear();
                    app.settings.shortlist.clear();
                    let _ = app.settings.save();
                }
            });
            // The spend is confirmed BEFORE the first run, not after it.
            if let Some(estimate) = app.run.estimate.clone() {
                if estimate.requires_confirmation {
                    ui.horizontal_wrapped(|ui| {
                        crate::views::pill(ui, "spend", t.warn);
                        ui.label(
                            RichText::new(
                                estimate
                                    .confirmation
                                    .clone()
                                    .unwrap_or_else(|| estimate.text.clone()),
                            )
                            .text_style(theme::meta())
                            .color(t.warn),
                        );
                        if crate::views::primary_button(ui, "Yes, score them", true).clicked() {
                            app.start_shortlist_batch();
                        }
                        if crate::views::ghost_button(ui, "Not now").clicked() {
                            app.run.estimate = None;
                        }
                    });
                }
            }
            ui.add_space(6.0);
        }
        if inbox.is_empty() {
            ui.label(RichText::new("The inbox is empty.").color(t.fg_muted));
            return;
        }
        let mut shown = 0;
        for job in inbox.iter().take(30) {
            // A shortlisted posting is HIDDEN from the pending list: it has
            // been dealt with, and the shortlist is where it now lives.
            if app.is_shortlisted(&job.url) {
                continue;
            }
            shown += 1;
            ui.horizontal(|ui| {
                let mark = if job.done { "skip" } else { "pending" };
                crate::views::pill(ui, mark, if job.done { t.fg_muted } else { t.accent });
                ui.label(RichText::new(&job.company).strong().color(t.fg));
                ui.label(RichText::new(&job.role).color(t.fg_muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let label = if job.done { "Unskip" } else { "Skip" };
                    if row_action(ui, label, t).clicked() {
                        app.skip_inbox(&job.url, !job.done);
                    }
                    if row_action(ui, "Shortlist", t).clicked() {
                        app.toggle_shortlist(&job.url);
                    }
                    if row_action(ui, "Open", t).clicked() {
                        open_url(ui, &job.url);
                    }
                });
            });
            ui.add_space(3.0);
        }
        if shown == 0 {
            ui.label(
                RichText::new("Every pending posting is on the shortlist.")
                    .color(t.fg_muted),
            );
        }
    });
}

/// A quiet, compact action for a table row.
fn row_action(ui: &mut egui::Ui, text: &str, t: Palette) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(text)
                .text_style(theme::meta())
                .color(t.fg),
        )
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(CornerRadius::same(6))
        .min_size(egui::vec2(0.0, 24.0)),
    )
}

/// The filter bar: status tabs, a score floor and a search box.
fn filter_bar(ui: &mut egui::Ui, app: &mut CareerOpsApp, states: &[String], t: Palette) {
    theme::card(ui, t).inner_margin(Margin::symmetric(14, 12)).show(
        ui,
        |ui| {
            ui.horizontal_wrapped(|ui| {
                let mut tabs: Vec<String> = vec!["All".to_string()];
                tabs.extend(states.iter().cloned());
                for tab in tabs {
                    let selected = app.pipeline_view.status_filter == tab;
                    // A tab is a pill, not a text link: the selected one has to
                    // be findable in peripheral vision.
                    let (fill, stroke, color) = if selected {
                        (t.tint(t.accent).gamma_multiply(2.4), t.accent, t.fg)
                    } else {
                        (Color32::TRANSPARENT, t.border, t.fg_muted)
                    };
                    let button = egui::Button::new(
                        RichText::new(&tab).text_style(theme::meta()).color(color),
                    )
                    .fill(fill)
                    .stroke(Stroke::new(1.0, stroke))
                    .corner_radius(CornerRadius::same(6))
                    .min_size(egui::vec2(0.0, 24.0));
                    if ui.add(button).on_hover_text("Filter by status").clicked() {
                        app.pipeline_view.status_filter = tab.clone();
                    }
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Min score").text_style(theme::meta()).color(t.fg_muted));
                ui.spacing_mut().slider_width = 150.0;
                ui.add(
                    egui::Slider::new(&mut app.pipeline_view.min_score, 0.0..=5.0)
                        .step_by(0.5)
                        .suffix("/5"),
                );
                ui.add_space(12.0);
                ui.label(RichText::new("Search").text_style(theme::meta()).color(t.fg_muted));
                ui.add(
                    egui::TextEdit::singleline(&mut app.pipeline_view.search)
                        .desired_width(260.0)
                        .hint_text("company, role or notes"),
                );
                if !app.pipeline_view.search.is_empty() && ui.small_button("clear").clicked() {
                    app.pipeline_view.search.clear();
                }
            });
        },
    );
}

/// The status cell: a combo that writes through set-status.mjs.
///
/// Deliberately still a ComboBox. Its selected text reaches the accessibility
/// tree as the control's *value* rather than as a label, which is what lets the
/// filter tabs above own those words: a pill here would put two nodes named
/// "Applied" on the screen, and the screen tests — correctly — refuse an
/// ambiguous query. The tint is carried on the text instead.
fn status_combo(
    ui: &mut egui::Ui,
    row: &Application,
    states: &[String],
    change: &mut Option<(String, String)>,
    t: Palette,
    zebra: bool,
) {
    let tint = if zebra { t.zebra } else { t.card };
    egui::ComboBox::from_id_salt(("status", row.n.clone()))
        .selected_text(
            RichText::new(&row.status)
                .text_style(theme::meta())
                .color(t.fg)
                .background_color(tint),
        )
        .width(120.0)
        .show_ui(ui, |ui| {
            for state in states {
                let selected = &row.status == state;
                if ui.selectable_label(selected, state.as_str()).clicked() {
                    *change = Some((row.n.clone(), state.clone()));
                }
            }
        });
}

/// The posting URL of a tracker row, when it has one.
pub fn posting_url(row: &Application) -> Option<String> {
    let raw = row.url.trim();
    if raw.is_empty() || raw == "—" {
        return None;
    }
    // The tracker stores the URL as a markdown link in some installs.
    if let Some(start) = raw.find("](") {
        if let Some(end) = raw[start + 2..].find(')') {
            return Some(raw[start + 2..start + 2 + end].to_string());
        }
    }
    Some(raw.to_string())
}

/// Open a URL in the user's browser.
///
/// The URL comes from a tracker row or a report header, so it is checked for a
/// web scheme first: opening it hands the string to the desktop, and a
/// `file://` there would open something the user did not ask for.
pub fn open_url(ui: &egui::Ui, url: &str) {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return;
    }
    ui.ctx().open_url(egui::OpenUrl::new_tab(url));
}

/// Today's date, for the hand-added row's date field.
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let days = secs / 86_400;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The civil date from a day count since the epoch.
///
/// Howard Hinnant's civil_from_days, the same one the core uses.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
