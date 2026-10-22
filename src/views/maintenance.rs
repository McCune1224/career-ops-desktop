//! The maintenance screen: the tracker's own health, and the repairs the core
//! already knows how to make.
//!
//! Every button here runs a CORE script and shows what the script said. The
//! client adds no rule of its own, and every repair is a proposal until the
//! user asks for it to be applied.

use egui::RichText;

use crate::app::CareerOpsApp;
use crate::theme::{self, Palette};
use crate::views::frame;

/// The tools the core ships, and what each one is for.
pub const TOOLS: [(&str, &str); 8] = [
    ("verify", "Health check — every broken link, orphan and inconsistent row"),
    ("normalize", "Rewrite non-canonical statuses"),
    ("dedup", "Merge duplicate tracker rows"),
    ("fixLinks", "Repair dead report links"),
    ("reconcile", "Reconcile the inbox against the batch state"),
    ("jdArchive", "Check that every report kept its JD"),
    ("pdfFlags", "Make the PDF column match the disk"),
    ("fixSlugs", "Repair malformed portal slugs"),
];

/// Draw the maintenance screen.
pub fn ui(ui: &mut egui::Ui, app: &mut CareerOpsApp) {
    let t = theme::palette(ui.ctx());
    frame(
        ui,
        "Maintenance",
        Some("The core's own repair tools. A proposal first, an action second."),
        app,
        |ui, app| {
            tools_card(ui, app, t);
            ui.add_space(14.0);
            output_card(ui, app, t);
        },
    );
}

/// The tools.
fn tools_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Tools", |ui| {
            ui.label(
                RichText::new("the first click proposes, the second applies")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        });
        ui.add_space(8.0);
        for (tool, description) in TOOLS {
            ui.horizontal(|ui| {
                ui.label(RichText::new(description).color(t.fg));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::views::ghost_button(ui, "Apply").clicked() {
                        app.run_maintenance(tool, true);
                    }
                    if crate::views::ghost_button(ui, "Preview").clicked() {
                        app.run_maintenance(tool, false);
                    }
                });
            });
            ui.add_space(4.0);
        }
    });
}

/// What the last tool said.
fn output_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Result", |ui| {
            if let Some(result) = &app.state.maintenance {
                crate::views::pill(
                    ui,
                    if result.apply { "applied" } else { "proposal" },
                    if result.ok { t.good } else { t.warn },
                );
            }
        });
        ui.add_space(8.0);
        let Some(result) = app.state.maintenance.clone() else {
            ui.label(
                RichText::new("Nothing has been run yet.")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            return;
        };
        if result.output.is_empty() {
            ui.label(
                RichText::new("The tool said nothing.")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            return;
        }
        for line in result.output.lines().take(40) {
            ui.label(
                RichText::new(line)
                    .monospace()
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        }
    });
}
