//! The window shell: the frame, the sidebar and the screen router.
//!
//! This lives in the library rather than in `main.rs` because `eframe::App` is
//! a foreign trait: the orphan rule requires the implementation to sit in the
//! crate that defines `CareerOpsApp`.
//!
//! The window opens at most of the primary monitor rather than at a fixed
//! 1280x820. That number was the single largest reason the app looked small:
//! on the machine it was built against it filled half the screen, so even
//! correctly sized text read as a shrunken window.

use std::sync::Arc;

use eframe::egui;
use egui::{Align, Color32, CornerRadius, Layout, Margin, RichText, Stroke};

use crate::app::{CareerOpsApp, Screen};
use crate::backend::BridgeBackend;
use crate::bridge::{Bridge, BridgePaths};
use crate::settings::{Settings, Theme};
use crate::theme::{self, Palette};

/// The screens in the sidebar, in the order the web app lists them.
pub const NAV: &[(&str, Screen)] = &[
    ("Today", Screen::Today),
    ("Pipeline", Screen::Pipeline),
    ("Run", Screen::Run),
    ("CV", Screen::Cv),
    ("Follow-ups", Screen::Followups),
    ("Config", Screen::Config),
];

/// Open the window and run the app.
pub fn run() -> eframe::Result<()> {
    let mut settings = Settings::load();
    // The engine checkout is named, not inferred: CAREER_OPS_CODE_ROOT, then
    // the value this app remembered, then a sibling checkout. The resolved
    // value is written back to the settings so the choice survives a restart.
    let paths = match BridgePaths::discover(Some(settings.code_root.as_str())) {
        Ok(paths) => paths,
        Err(e) => {
            eprintln!("career-ops: {e}");
            std::process::exit(1);
        }
    };
    if settings.code_root != paths.code_root.display().to_string() {
        settings.code_root = paths.code_root.display().to_string();
        // A failure to remember the choice is not fatal: the app runs with the
        // checkout it resolved this time.
        let _ = settings.save();
    }
    let data_root = if settings.data_root.trim().is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(settings.data_root.trim()))
    };

    let bridge = match Bridge::start(paths, data_root.as_deref()) {
        Ok(bridge) => bridge,
        Err(e) => {
            eprintln!("career-ops: {e}");
            std::process::exit(1);
        }
    };
    let backend = Arc::new(BridgeBackend::new(bridge));

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("career-ops")
            .with_app_id("career-ops")
            // Open large.
            //
            // The first version asked for 1280x820, which on a 2560x1440 panel
            // is half the screen — the single largest reason the app read as
            // "very small". Now 2000x1240: 78% x 86% of that panel, and a window
            // manager clamps an oversized request to its own work area, so a
            // smaller display gets a fitting window rather than an overflowing
            // one.
            //
            // No `with_maximized`: on the window manager this was developed
            // against, a maximise request quarter-tiled the window to
            // 1275x700 — smaller than either fixed size. A plain size is
            // honoured; the maximise key is the user's.
            .with_inner_size([2000.0, 1240.0])
            .with_min_inner_size([1040.0, 680.0]),
        ..Default::default()
    };

    eframe::run_native(
        "career-ops",
        native_options,
        Box::new(move |cc| {
            let mut app = CareerOpsApp::new(backend.clone(), settings.clone());
            theme::apply(&cc.egui_ctx, settings.theme.clone(), settings.zoom);
            app.load_all();
            Ok(Box::new(app))
        }),
    )
}

impl eframe::App for CareerOpsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Ctrl +/-/0 and Ctrl+Shift+Z for zoom, the shortcuts every desktop app
        // that can be too small has.
        self.handle_zoom_input(&ctx);

        // Drain the run's streamed events before drawing, so the log shows the
        // newest line in the same frame it arrives.
        self.run_now = ctx.input(|i| i.time);
        let events = self.backend.poll_events();
        if !events.is_empty() {
            self.absorb_events(events);
            ctx.request_repaint();
        }

        let t = theme::palette(&ctx);
        let mut root_visuals = ui.visuals().clone();
        root_visuals.panel_fill = t.bg;
        ui.ctx().set_visuals(root_visuals);

        top_bar(ui, self, t);
        egui::Panel::left("nav")
            .resizable(false)
            .default_size(theme::SIDEBAR)
            .frame(
                egui::Frame::NONE
                    .fill(t.panel)
                    .inner_margin(Margin::symmetric(12, 12)),
            )
            .show(ui, |ui| sidebar(ui, self, t));

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(t.bg)
                    .inner_margin(Margin::symmetric(22, 14)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.screen_ui(ui));
            });
    }
}

impl CareerOpsApp {
    /// Draw the current screen.
    pub fn screen_ui(&mut self, ui: &mut egui::Ui) {
        match self.screen.clone() {
            Screen::Today => crate::views::today::ui(ui, self),
            Screen::Pipeline => crate::views::pipeline::ui(ui, self),
            Screen::Report(_) => crate::views::report::ui(ui, self),
            Screen::Run => crate::views::run::ui(ui, self),
            Screen::Cv => crate::views::cv::ui(ui, self),
            Screen::Followups => crate::views::followups::ui(ui, self),
            Screen::Config => crate::views::config::ui(ui, self),
        }
    }
}

/// The bar across the top: what this is, what it is reading, and the controls
/// that are global rather than screen-specific.
fn top_bar(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    egui::Panel::top("top-bar")
        .frame(
            egui::Frame::NONE
                .fill(t.panel)
                .inner_margin(Margin::symmetric(16, 10)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("career-ops")
                        .size(18.0)
                        .strong()
                        .color(t.fg),
                );
                ui.add_space(8.0);
                if let Some(state) = &app.state.bootstrap {
                    ui.label(
                        RichText::new(&state.data_root)
                            .text_style(theme::meta())
                            .monospace()
                            .color(t.fg_muted),
                    );
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if theme_button(ui, app, t) {
                        app.apply_theme(ui.ctx());
                    }
                    if crate::views::ghost_button(ui, "Reload").clicked() {
                        app.load_all();
                    }
                    if app.state.busy {
                        ui.spinner();
                    }
                    if let Some((message, is_error)) = app.state.message.clone() {
                        let color = if is_error { t.bad } else { t.good };
                        ui.label(RichText::new(message).color(color));
                    }
                    if let Some(due) = app.due_followups() {
                        if due > 0 {
                            crate::views::pill(ui, &format!("{due} due"), t.warn);
                        }
                    }
                });
            });
        });
}

/// The theme toggle. Returns true when it was clicked, having already flipped
/// the setting, so the caller only has to re-apply.
fn theme_button(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) -> bool {
    let (label, next) = match app.settings.theme {
        Theme::Dark => ("Light mode", Theme::Light),
        Theme::Light => ("Dark mode", Theme::Dark),
    };
    let response = ui.add(
        egui::Button::new(RichText::new(label).color(t.fg_muted))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, t.border))
            .corner_radius(CornerRadius::same(theme::RADIUS)),
    );
    if response.clicked() {
        app.settings.theme = next;
        return true;
    }
    false
}

/// The sidebar: navigation, then what the app is working on.
fn sidebar(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    ui.add_space(2.0);
    ui.label(
        RichText::new("WORKSPACE")
            .text_style(theme::meta())
            .color(t.fg_muted),
    );
    ui.add_space(6.0);

    let mut goto: Option<Screen> = None;
    for (label, screen) in NAV {
        let selected = matches!(
            (screen, &app.screen),
            (Screen::Today, Screen::Today)
                | (Screen::Pipeline, Screen::Pipeline)
                | (Screen::Run, Screen::Run)
                | (Screen::Cv, Screen::Cv)
                | (Screen::Followups, Screen::Followups)
                | (Screen::Config, Screen::Config)
        );
        // A report is a view of one pipeline row, so Pipeline stays lit.
        let selected = selected || matches!((screen, &app.screen), (Screen::Pipeline, Screen::Report(_)));
        if nav_item(ui, label, selected, t) {
            goto = Some(screen.clone());
        }
    }
    if let Some(screen) = goto {
        app.screen = screen;
    }

    ui.add_space(12.0);
    crate::views::hairline(ui, t);
    ui.add_space(10.0);

    if let Some(state) = &app.state.bootstrap {
        ui.label(
            RichText::new(format!("{} applications", state.application_count))
                .text_style(theme::meta())
                .color(t.fg_muted),
        );
        ui.label(
            RichText::new(format!("{} pending", state.inbox_count))
                .text_style(theme::meta())
                .color(t.fg_muted),
        );
        if let Some(due) = app.due_followups() {
            if due > 0 {
                ui.add_space(4.0);
                crate::views::pill(ui, &format!("{due} follow-up(s) due"), t.warn);
            }
        }
    }

    // The zoom control sits here because "make it bigger" is a judgement the
    // fixed token scale cannot make for a display it has never seen.
    ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if crate::views::ghost_button(ui, "A-").clicked() {
                app.zoom_by(ui.ctx(), -1);
            }
            if crate::views::ghost_button(ui, "A+").clicked() {
                app.zoom_by(ui.ctx(), 1);
            }
            ui.label(
                RichText::new(format!("{:.0}%", app.settings.zoom * 100.0))
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        });
        ui.label(
            RichText::new("Ctrl +/- to zoom")
                .text_style(theme::meta())
                .color(t.fg_muted),
        );
    });
}

/// One navigation row: a card that lights up, with an accent bar when selected.
fn nav_item(ui: &mut egui::Ui, label: &str, selected: bool, t: Palette) -> bool {
    let height = 34.0;
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    let hovered = response.hovered();

    let fill = if selected {
        t.tint(t.accent).gamma_multiply(2.2)
    } else if hovered {
        t.card
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, CornerRadius::same(theme::RADIUS), fill);
    if selected {
        // The bar is what makes the current location readable at a glance.
        let bar = egui::Rect::from_min_size(
            rect.left_top() + egui::vec2(0.0, 6.0),
            egui::vec2(3.0, height - 12.0),
        );
        ui.painter().rect_filled(bar, CornerRadius::same(2), t.accent);
    }
    let text_color = if selected { t.fg } else { t.fg_muted };
    ui.painter().text(
        rect.left_center() + egui::vec2(14.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(theme::BUTTON + 0.5),
        if hovered || selected { text_color } else { text_color },
    );
    // The label has to reach the accessibility tree, or a screen reader (and
    // the headless screen tests) cannot see it. A painted rect is invisible.
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::Button, true, selected, label)
    });
    response.clicked()
}
