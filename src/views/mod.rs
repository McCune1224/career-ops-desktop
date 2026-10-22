//! The screens.

pub mod config;
pub mod cv;
pub mod documents;
pub mod followups;
pub mod interview;
pub mod maintenance;
pub mod outcomes;
pub mod pdfview;
pub mod pipeline;
pub mod portals;
pub mod report;
pub mod run;
pub mod today;

use egui::{Align, Color32, CornerRadius, Layout, Margin, RichText, Stroke, TextStyle};

use crate::app::{CareerOpsApp, SortKey};
use crate::model::Application;
use crate::theme::{self, Palette};

/// Sort the tracker rows the way the table header says.
///
/// The web app sorts with two passes: the chosen key, then the tracker number
/// as the tie-break. Same rule here, so the same rows land in the same order.
pub fn sort_rows(rows: &mut [Application], key: SortKey, descending: bool) {
    rows.sort_by(|a, b| {
        let ordering = match key {
            SortKey::Tracker => a.number().cmp(&b.number()),
            SortKey::Company => a.company.to_lowercase().cmp(&b.company.to_lowercase()),
            SortKey::Role => a.role.to_lowercase().cmp(&b.role.to_lowercase()),
            SortKey::Score => a
                .score_value()
                .partial_cmp(&b.score_value())
                .unwrap_or(std::cmp::Ordering::Equal),
            SortKey::Status => a.status.cmp(&b.status),
            SortKey::Date => a.date.cmp(&b.date),
        };
        let ordering = if descending { ordering.reverse() } else { ordering };
        ordering.then_with(|| a.number().cmp(&b.number()))
    });
}

/// Apply the status filter, the score floor and the free-text search.
pub fn filter_rows(
    rows: Vec<Application>,
    status_filter: &str,
    min_score: f32,
    search: &str,
) -> Vec<Application> {
    let needle = search.trim().to_lowercase();
    rows.into_iter()
        .filter(|row| {
            if status_filter != "All" && row.status != status_filter {
                return false;
            }
            if min_score > 0.0 && row.score_value() < min_score {
                return false;
            }
            if needle.is_empty() {
                return true;
            }
            row.company.to_lowercase().contains(&needle)
                || row.role.to_lowercase().contains(&needle)
                || row.notes.to_lowercase().contains(&needle)
                || row.n == needle
        })
        .collect()
}

/// A short, human label for a tracker row.
pub fn row_title(app: &Application) -> String {
    if app.role.is_empty() {
        app.company.clone()
    } else {
        format!("{} — {}", app.company, app.role)
    }
}

/// The screen frame: a title, an optional subtitle, and a hairline under them.
pub fn frame(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: Option<&str>,
    app: &mut CareerOpsApp,
    body: impl FnOnce(&mut egui::Ui, &mut CareerOpsApp),
) {
    // A `Ui` captures the context's style when it is created, so a style
    // installed by `ensure` a moment ago would not reach this frame's widgets.
    // `theme::adopt` refreshes it: then a screen drawn by a host that never
    // called `theme::apply` still renders with the app's type scale instead of
    // panicking on a named text style.
    let t = theme::adopt(ui);
    ui.add_space(4.0);
    ui.heading(RichText::new(title).color(t.fg));
    if let Some(sub) = subtitle {
        if !sub.is_empty() {
            ui.label(RichText::new(sub).text_style(theme::meta()).color(t.fg_muted));
        }
    }
    ui.add_space(10.0);
    hairline(ui, t);
    ui.add_space(12.0);
    body(ui, app);
}

/// A section header: a small, muted label over a hairline, so a long screen is
/// scannable without spending another weight of title on it.
///
/// Deliberately not upper-cased. Capitals buy a little hierarchy and cost
/// legibility on every label, and they also make the text harder to find by
/// name — a section called "Logged follow-ups" should say exactly that.
pub fn section(ui: &mut egui::Ui, title: &str) {
    let t = theme::palette(ui.ctx());
    ui.add_space(6.0);
    ui.label(
        RichText::new(title)
            .text_style(theme::meta())
            .color(t.fg_muted)
            .strong(),
    );
    ui.add_space(4.0);
    hairline(ui, t);
    ui.add_space(8.0);
}

/// A one-pixel rule.
pub fn hairline(ui: &mut egui::Ui, t: Palette) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), egui::Sense::hover());
    ui.painter().hline(rect.x_range(), rect.center().y, Stroke::new(1.0, t.border));
}

/// A rounded, tinted badge. The colour carries the meaning; the text says it.
pub fn pill(ui: &mut egui::Ui, text: &str, color: Color32) {
    let t = theme::palette(ui.ctx());
    egui::Frame::NONE
        .fill(t.tint(color).gamma_multiply(2.2))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.55)))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).text_style(theme::meta()).color(color).strong());
        });
}

/// A big number with a caption: the shape a dashboard tile wants.
pub fn metric(ui: &mut egui::Ui, value: &str, label: &str, hint: &str, accent: Option<Color32>) {
    let t = theme::palette(ui.ctx());
    theme::card(ui, t).show(ui, |ui| {
        ui.set_min_width(140.0);
        ui.vertical(|ui| {
            ui.label(
                RichText::new(value)
                    .size(30.0)
                    .monospace()
                    .color(accent.unwrap_or(t.fg)),
            );
            ui.label(RichText::new(label).color(t.fg).strong());
            ui.label(
                RichText::new(hint)
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        });
    });
}

/// A primary action, drawn in the accent rather than as another grey button.
pub fn primary_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let t = theme::palette(ui.ctx());
    let button = egui::Button::new(RichText::new(text).color(if enabled { t.fg } else { t.fg_muted }))
        .fill(if enabled {
            t.tint(t.accent).gamma_multiply(2.8)
        } else {
            t.card
        })
        .stroke(Stroke::new(
            1.0,
            if enabled { t.accent } else { t.border },
        ))
        .corner_radius(CornerRadius::same(theme::RADIUS));
    ui.add_enabled(enabled, button)
}

/// A quiet action: still a button, but nothing shouts.
pub fn ghost_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let t = theme::palette(ui.ctx());
    ui.add(
        egui::Button::new(RichText::new(text).color(t.fg_muted))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, t.border))
            .corner_radius(CornerRadius::same(theme::RADIUS)),
    )
}

/// The score, as a monospace badge coloured by how good it is.
pub fn score_text(t: Palette, score: &str, value: f32) -> RichText {
    RichText::new(if score.is_empty() { "—" } else { score })
        .monospace()
        .color(t.score(value))
}

/// A title row for a card: the heading plus a slot on the right for actions.
pub fn card_title(ui: &mut egui::Ui, title: &str, right: impl FnOnce(&mut egui::Ui)) {
    let t = theme::palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).text_style(TextStyle::Body).strong().color(t.fg));
        ui.with_layout(Layout::right_to_left(Align::Center), right);
    });
}

/// A list of label and value pairs, the shape a data card wants.
pub fn key_values(ui: &mut egui::Ui, pairs: &[(&str, String)]) {
    let t = theme::palette(ui.ctx());
    egui::Grid::new("key-values")
        .num_columns(2)
        .striped(false)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            for (label, value) in pairs {
                ui.label(
                    RichText::new(*label)
                        .text_style(theme::meta())
                        .color(t.fg_muted),
                );
                ui.label(RichText::new(value).color(t.fg));
                ui.end_row();
            }
        });
}

/// Open a path in the user's desktop environment.
///
/// The path comes from the engine's own resolver and is already containment
/// checked there; this hands it to the desktop the same way the report screen
/// hands over a PDF.
pub fn open_path(path: &str) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

// re-exported for the screens that draw their own table chrome
pub use theme::Palette as Tokens;
