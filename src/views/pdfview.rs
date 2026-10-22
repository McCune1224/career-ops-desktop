//! The PDF preview: a page of a PDF, painted in the window.
//!
//! The page is rasterised by `src/pdf.rs` (poppler's `pdftoppm`, no PDF library
//! and no webview) and drawn here as a texture. egui scales it, so one render
//! at 110 DPI serves every pane width the window can have.
//!
//! What this is NOT: a PDF *viewer*. There is no text selection, no search, no
//! annotation. What it is for is the question the user actually has — does this
//! page look right before I send it — and for anything past that, "Open in your
//! viewer" is one click away and opens the very same file.

use egui::RichText;

use crate::app::CareerOpsApp;
use crate::theme::{self, Palette};
use crate::views::pill;

/// Draw the preview card, when a preview is open.
pub fn card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    let Some(preview) = app.preview.clone() else {
        return;
    };
    theme::card(ui, t).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(
                    preview
                        .path
                        .rsplit('/')
                        .next()
                        .unwrap_or(preview.path.as_str()),
                )
                .strong()
                .color(t.fg),
            );
            pill(
                ui,
                &format!("page {} of {}", preview.page, preview.pages.max(1)),
                t.accent,
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::views::ghost_button(ui, "Close").clicked() {
                    app.close_preview();
                }
                if crate::views::ghost_button(ui, "Open in your viewer").clicked() {
                    crate::views::open_path(&preview.path);
                }
                if preview.page < preview.pages && crate::views::ghost_button(ui, "Next").clicked()
                {
                    app.preview_page(1);
                }
                if preview.page > 1 && crate::views::ghost_button(ui, "Previous").clicked() {
                    app.preview_page(-1);
                }
            });
        });
        ui.add_space(8.0);
        if let Some(error) = &preview.error {
            // ONE actionable sentence. A preview that silently shows nothing is
            // the failure mode the user cannot act on.
            ui.horizontal_wrapped(|ui| {
                pill(ui, "no preview", t.warn);
                ui.label(RichText::new(error).color(t.warn));
            });
            return;
        }
        let Some(texture) = &preview.texture else {
            ui.label(
                RichText::new("Rendering the page…")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            return;
        };
        let available = ui.available_width().min(760.0);
        let size = texture.size_vec2();
        let scale = if size.x > 0.0 {
            (available / size.x).min(1.6)
        } else {
            1.0
        };
        egui::ScrollArea::vertical()
            .id_salt("pdf-preview")
            .max_height(520.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::Image::new(texture)
                        .fit_to_exact_size(size * scale)
                        .corner_radius(egui::CornerRadius::same(theme::RADIUS)),
                );
            });
    });
}
