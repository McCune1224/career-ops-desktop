//! The visual system: colours, type scale, spacing and widget styling.
//!
//! One module so a screen never invents a colour or a size. The old shell set
//! two fills inline and let egui's 14pt default carry everything else, which on
//! a 2560x1440 panel read as a small window with smaller text. Everything a
//! screen draws now comes from a token here.
//!
//! ## Why these numbers
//!
//! Measured on the machine this was built against: a 2560x1440 primary display
//! at 597x336 mm is about 109 PPI, with a desktop scale factor of 1.0. egui's
//! default body size is 14 points, so text landed near 10.5 physical points —
//! legible, but only just, and the window opened at 1280x820 (half the screen)
//! which made the whole thing look shrunken rather than merely dense.
//!
//! So: body 16.5, five deliberate steps of type, a 4/8/12/16/24 spacing rhythm,
//! and 36pt table rows. Plus a persisted zoom (Ctrl +/-/0) for taste and for
//! displays this measurement does not describe.
//!
//! ## Palette
//!
//! The dark palette is a developer-tool slate (`#0F172A` ground, `#1B2336`
//! card) rather than pure black: the tracker is read for long stretches and a
//! true-black ground with pure-white text is harsher, not more legible.
//!
//! One deliberate departure from the theme this was seeded from: its accent was
//! green, and green is already this app's *success* semantic — a good score, an
//! applied row. An accent that means "clickable" competing with one that means
//! "went well" makes both weaker, so interactive is blue and green keeps its
//! one meaning.

use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Shadow, Stroke, TextStyle, Vec2};

/// Screen title.
pub const HEADING: f32 = 26.0;
/// Section header inside a screen.
pub const H2: f32 = 19.0;
/// Body copy: the reading size for a report or a CV.
pub const BODY: f32 = 16.5;
/// Controls, buttons, table cells.
pub const BUTTON: f32 = 15.0;
/// Monospace: numbers, dates, tracker ids.
pub const MONO: f32 = 15.0;
/// Metadata, hints, captions.
pub const SMALL: f32 = 13.5;

/// A table row's height. 36 rather than 26: a row is a thing you read and click.
pub const ROW_HEIGHT: f32 = 36.0;
/// The sidebar's width.
///
/// Chosen to leave the tracker table room on a small canvas: this window
/// manager tiles windows to a quarter of the display, which measures 1270x685
/// pixels here, so the table has about 880 points to work with after the
/// sidebar and the page margins.
pub const SIDEBAR: f32 = 196.0;
/// Small controls.
pub const RADIUS: u8 = 8;
/// Cards and framed regions.
pub const CARD_RADIUS: u8 = 10;

/// A named text style for section headers.
pub fn h2() -> TextStyle {
    TextStyle::Name("h2".into())
}

/// A named text style for metadata.
pub fn meta() -> TextStyle {
    TextStyle::Name("meta".into())
}

/// The colours one theme uses. Semantic slots only: a screen asks for `good`,
/// never for a specific green.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// The window's ground.
    pub bg: Color32,
    /// Sidebar and top bar: one step off the ground.
    pub panel: Color32,
    /// Cards and rows.
    pub card: Color32,
    /// A card under the pointer.
    pub card_hover: Color32,
    /// The zebra stripe on alternate table rows.
    pub zebra: Color32,
    /// Hairlines: separators, card edges.
    pub border: Color32,
    /// Edges that need to be seen: a focused field, a hovered card.
    pub border_strong: Color32,
    /// Primary text.
    pub fg: Color32,
    /// Secondary text: hints, captions, units.
    pub fg_muted: Color32,
    /// Interactive: links, the selected nav item, a focused control.
    pub accent: Color32,
    /// Success: a strong score, an applied row, a finished run.
    pub good: Color32,
    /// Attention: a stale follow-up, an unpersonalized profile.
    pub warn: Color32,
    /// Failure: a rejected status, a crashed run.
    pub bad: Color32,
    /// Neutral information.
    pub info: Color32,
}

impl Palette {
    /// The default ground: a slate dark, not pure black.
    pub const fn dark() -> Palette {
        Palette {
            bg: Color32::from_rgb(15, 23, 42),
            panel: Color32::from_rgb(19, 28, 46),
            card: Color32::from_rgb(27, 35, 54),
            card_hover: Color32::from_rgb(34, 44, 67),
            zebra: Color32::from_rgb(23, 31, 48),
            border: Color32::from_rgb(42, 53, 72),
            border_strong: Color32::from_rgb(71, 85, 105),
            fg: Color32::from_rgb(248, 250, 252),
            fg_muted: Color32::from_rgb(148, 163, 184),
            accent: Color32::from_rgb(110, 168, 254),
            good: Color32::from_rgb(34, 197, 94),
            warn: Color32::from_rgb(224, 166, 58),
            bad: Color32::from_rgb(239, 68, 68),
            info: Color32::from_rgb(56, 189, 248),
        }
    }

    /// A light ground, for a bright room or a projector.
    pub const fn light() -> Palette {
        Palette {
            bg: Color32::from_rgb(246, 248, 251),
            panel: Color32::from_rgb(255, 255, 255),
            card: Color32::from_rgb(255, 255, 255),
            card_hover: Color32::from_rgb(241, 245, 249),
            zebra: Color32::from_rgb(248, 250, 252),
            border: Color32::from_rgb(226, 232, 240),
            border_strong: Color32::from_rgb(203, 213, 225),
            fg: Color32::from_rgb(15, 23, 42),
            fg_muted: Color32::from_rgb(100, 116, 139),
            accent: Color32::from_rgb(37, 99, 235),
            good: Color32::from_rgb(21, 128, 61),
            warn: Color32::from_rgb(180, 83, 9),
            bad: Color32::from_rgb(220, 38, 38),
            info: Color32::from_rgb(3, 105, 161),
        }
    }

    /// The colour a score gets: the same thresholds the web UI grades on.
    pub fn score(&self, value: f32) -> Color32 {
        if value >= 4.0 {
            self.good
        } else if value >= 3.0 {
            self.warn
        } else if value > 0.0 {
            self.bad
        } else {
            self.fg_muted
        }
    }

    /// The colour a legitimacy tier gets.
    pub fn legitimacy(&self, tier: &str) -> Color32 {
        let lower = tier.to_lowercase();
        if lower.contains("high") || lower.contains("confian") {
            self.good
        } else if lower.contains("caution") || lower.contains("precaution") {
            self.warn
        } else if lower.contains("suspic") || lower.contains("scam") || lower.contains("fake") {
            self.bad
        } else {
            self.fg_muted
        }
    }

    /// The colour an urgency gets.
    pub fn urgency(&self, urgency: &str) -> Color32 {
        match urgency {
            "urgent" => self.bad,
            "overdue" => self.warn,
            "waiting" => self.info,
            _ => self.fg_muted,
        }
    }

    /// A translucent badge fill from a semantic colour.
    pub fn tint(&self, color: Color32) -> Color32 {
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 38)
    }
}

/// The token store, so any `Ui` can ask for the palette without threading it
/// through every function signature.
/// A stable id for the palette slot. A constant keeps the lookup independent of
/// any widget's own id scope, so every screen sees the same palette.
const PALETTE_ID: &str = "career-ops-palette";

fn palette_id() -> egui::Id {
    egui::Id::unique(PALETTE_ID)
}

/// The active palette.
pub fn palette(ctx: &egui::Context) -> Palette {
    ctx.data(|d| d.get_temp::<Palette>(palette_id()))
        .unwrap_or_else(Palette::dark)
}

/// Publish the active palette to the context.
fn publish(ctx: &egui::Context, palette: Palette) {
    ctx.data_mut(|d| d.insert_temp(palette_id(), palette));
}

/// Make sure this context has the app's named styles and palette.
///
/// `shell::run` calls `apply()` with the user's theme and zoom. A screen
/// rendered on its own — the headless tests do exactly that — never goes
/// through the shell, and egui panics when a `TextStyle::Name` is resolved
/// against a style that does not define it. So every screen frame calls this
/// first; it is a no-op once the styles are installed.
/// Make `ui` draw with the app's own token styles.
///
/// Installing the styles on the context is not enough for the frame that is
/// being drawn right now: a `Ui` captures the context's style when it is
/// created, so a style installed a moment ago never reaches it. Copies the live
/// style onto the `Ui` and returns the palette in one step, so a caller cannot
/// do one and forget the other.
pub fn adopt(ui: &mut egui::Ui) -> Palette {
    ensure(ui.ctx());
    let live = ui.ctx().style_of(ui.ctx().theme()).as_ref().clone();
    let style = ui.style_mut();
    style.text_styles = live.text_styles;
    style.spacing = live.spacing;
    style.visuals = live.visuals;
    palette(ui.ctx())
}

pub fn ensure(ctx: &egui::Context) {
    // Whichever theme egui currently resolves is the one a screen will draw
    // with, so asking it is the right question either way.
    if ctx.style_of(ctx.theme()).text_styles.contains_key(&meta()) {
        return;
    }
    apply(
        ctx,
        crate::settings::Theme::default(),
        crate::settings::DEFAULT_ZOOM,
    );
}

/// Apply the whole visual system.
///
/// `zoom` multiplies every point the app draws, which is the knob that actually
/// answers "this is too small" on a display whose density the fixed token scale
/// cannot know about.
pub fn apply(ctx: &egui::Context, theme: crate::settings::Theme, zoom: f32) {
    let zoom = zoom.clamp(0.75, 2.5);
    ctx.set_zoom_factor(zoom);

    for (mode, tokens) in [
        (egui::Theme::Dark, Palette::dark()),
        (egui::Theme::Light, Palette::light()),
    ] {
        let mut style = (*ctx.style_of(mode)).clone();
        style.text_styles = text_styles();
        style.spacing = spacing();
        style.visuals = visuals(tokens, mode == egui::Theme::Dark);
        ctx.set_style_of(mode, style);
    }

    let active = match theme {
        crate::settings::Theme::Dark => Palette::dark(),
        crate::settings::Theme::Light => Palette::light(),
    };
    publish(ctx, active);
    ctx.set_theme(match theme {
        crate::settings::Theme::Dark => egui::ThemePreference::Dark,
        crate::settings::Theme::Light => egui::ThemePreference::Light,
    });
}

/// The five steps of type the app uses, plus two named ones.
fn text_styles() -> std::collections::BTreeMap<TextStyle, FontId> {
    let mut styles = std::collections::BTreeMap::new();
    styles.insert(TextStyle::Heading, FontId::new(HEADING, FontFamily::Proportional));
    styles.insert(TextStyle::Body, FontId::new(BODY, FontFamily::Proportional));
    styles.insert(TextStyle::Button, FontId::new(BUTTON, FontFamily::Proportional));
    styles.insert(TextStyle::Small, FontId::new(SMALL, FontFamily::Proportional));
    styles.insert(TextStyle::Monospace, FontId::new(MONO, FontFamily::Monospace));
    styles.insert(h2(), FontId::new(H2, FontFamily::Proportional));
    styles.insert(meta(), FontId::new(SMALL, FontFamily::Proportional));
    styles
}

/// A 4/8/12/16/24 rhythm, with controls sized for a pointer rather than a finger.
fn spacing() -> egui::Spacing {
    egui::Spacing {
        item_spacing: Vec2::new(10.0, 9.0),
        button_padding: Vec2::new(13.0, 8.0),
        interact_size: Vec2::new(30.0, 27.0),
        slider_width: 150.0,
        combo_width: 150.0,
        indent: 18.0,
        window_margin: Margin::same(14),
        menu_margin: Margin::same(8),
        ..Default::default()
    }
}

fn visuals(t: Palette, dark: bool) -> egui::Visuals {
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    v.dark_mode = dark;
    v.panel_fill = t.panel;
    v.window_fill = t.card;
    // Text fields and code blocks sit *below* the card, not above it.
    v.extreme_bg_color = t.bg;
    // Alternate table rows.
    v.faint_bg_color = t.zebra;
    v.override_text_color = Some(t.fg);
    v.hyperlink_color = t.accent;
    v.window_corner_radius = CornerRadius::same(CARD_RADIUS);
    v.menu_corner_radius = CornerRadius::same(RADIUS);
    v.window_shadow = Shadow {
        offset: [0, 12],
        blur: 32,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 120 } else { 40 }),
    };
    v.popup_shadow = Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 120 } else { 40 }),
    };
    v.selection.bg_fill = t.tint(t.accent).gamma_multiply(2.2);
    v.selection.stroke = Stroke::new(1.0, t.accent);

    // Non-interactive surfaces: panels, separators, group frames.
    v.widgets.noninteractive.bg_fill = t.panel;
    v.widgets.noninteractive.weak_bg_fill = t.panel;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, t.fg);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(RADIUS);

    // Resting controls: a button is a card with an edge, not a filled block.
    v.widgets.inactive.bg_fill = t.card;
    v.widgets.inactive.weak_bg_fill = t.card;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, t.border);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, t.fg);
    v.widgets.inactive.corner_radius = CornerRadius::same(RADIUS);

    v.widgets.hovered.bg_fill = t.card_hover;
    v.widgets.hovered.weak_bg_fill = t.card_hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, t.border_strong);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, t.fg);
    v.widgets.hovered.corner_radius = CornerRadius::same(RADIUS);
    v.widgets.hovered.expansion = 0.0;

    v.widgets.active.bg_fill = t.tint(t.accent).gamma_multiply(2.6);
    v.widgets.active.weak_bg_fill = t.tint(t.accent).gamma_multiply(2.6);
    v.widgets.active.bg_stroke = Stroke::new(1.0, t.accent);
    v.widgets.active.fg_stroke = Stroke::new(1.0, t.fg);
    v.widgets.active.corner_radius = CornerRadius::same(RADIUS);
    v.widgets.active.expansion = 0.0;

    v.widgets.open.bg_fill = t.card_hover;
    v.widgets.open.weak_bg_fill = t.card_hover;
    v.widgets.open.bg_stroke = Stroke::new(1.0, t.border_strong);
    v.widgets.open.fg_stroke = Stroke::new(1.0, t.fg);
    v.widgets.open.corner_radius = CornerRadius::same(RADIUS);

    v
}

/// A card: the standard container for a grouping of related things.
pub fn card(ui: &egui::Ui, t: Palette) -> egui::Frame {
    egui::Frame::NONE
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(CornerRadius::same(CARD_RADIUS))
        .inner_margin(Margin::symmetric(16, 14))
        .shadow(Shadow {
            offset: [0, 1],
            blur: 2,
            spread: 0,
            color: Color32::from_black_alpha(if ui.visuals().dark_mode { 60 } else { 12 }),
        })
}

/// A raised card, for the one thing on a screen that is the point of it.
pub fn hero_card(ui: &egui::Ui, t: Palette) -> egui::Frame {
    card(ui, t)
        .fill(t.card_hover)
        .stroke(Stroke::new(1.0, t.tint(t.accent).gamma_multiply(2.6)))
}
