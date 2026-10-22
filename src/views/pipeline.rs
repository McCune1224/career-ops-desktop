//! The tracker screen: the pipeline table with sorting, filtering and the
//! canonical status change.
//!
//! The status change goes through the core's set-status.mjs (through the
//! bridge), never through a write of the tracker's own.

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
pub const TABLE_MIN_WIDTHS: [f32; 7] = [34.0, 120.0, 110.0, 62.0, 120.0, 84.0, 130.0];

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
    let inbox_count = app
        .state
        .pipeline
        .as_ref()
        .map(|p| p.inbox.len())
        .unwrap_or(0);
    let states: Vec<String> = app
        .state
        .bootstrap
        .as_ref()
        .map(|b| b.states.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| FALLBACK_STATES.iter().map(|s| s.to_string()).collect());
    let subtitle = format!(
        "{} applications · {inbox_count} pending postings",
        applications.len()
    );

    frame(ui, "Pipeline", Some(&subtitle), app, |ui, app| {
        filter_bar(ui, app, &states, t);
        ui.add_space(10.0);

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
            return;
        }

        let mut open: Option<String> = None;
        let mut status_change: Option<(String, String)> = None;

        theme::card(ui, t).inner_margin(Margin::symmetric(10, 8)).show(
            ui,
            |ui| {
                let table = TableBuilder::new(ui)
                    .striped(true)
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .column(Column::auto().at_least(TABLE_MIN_WIDTHS[0]))
                    .column(Column::auto().at_least(TABLE_MIN_WIDTHS[1]))
                    .column(
                        Column::initial(190.0)
                            .at_least(TABLE_MIN_WIDTHS[2])
                            .clip(true),
                    )
                    .column(Column::auto().at_least(TABLE_MIN_WIDTHS[3]))
                    .column(Column::auto().at_least(TABLE_MIN_WIDTHS[4]))
                    .column(Column::auto().at_least(TABLE_MIN_WIDTHS[5]))
                    .column(Column::remainder().at_least(TABLE_MIN_WIDTHS[6]))
                    .min_scrolled_height(160.0)
                    .header(26.0, |mut header| {
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
                                        app.pipeline_view.descending =
                                            !app.pipeline_view.descending;
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
                            status_combo(ui, row, &states, &mut status_change, t, zebra);
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
                                    if row_action(ui, "Posting", t)
                                        .on_hover_text(&url)
                                        .clicked()
                                    {
                                        open_url(ui, &url);
                                    }
                                }
                            });
                        });
                    });
                });
            },
        );

        if let Some(n) = open {
            app.open_report(&n);
        }
        if let Some((n, status)) = status_change {
            app.set_status(&n, &status);
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
