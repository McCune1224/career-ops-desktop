//! A small markdown renderer for egui.
//!
//! The reports are markdown documents a person reads, so the screen needs
//! headings, emphasis, lists, tables and code — not a full CommonMark
//! implementation. Anything this does not understand is rendered as plain
//! text, which is the safe direction: a report never loses content because
//! the renderer did not know the syntax.
//!
//! Inline emphasis is carried by a `LayoutJob` rather than by concatenating
//! styled strings, because that is the only way egui wraps a paragraph at the
//! window edge *inside* a run of mixed styling.
//!
//! Type comes from `theme`: body copy at the reading size, headings on a
//! deliberate scale, and a 1.5 line height — a report is the thing on this
//! screen a person actually reads for minutes at a time.

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontFamily, FontId, RichText, TextStyle};

use crate::theme::{self, Palette};

/// One block of a rendered document.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph(String),
    Bullet(String),
    Numbered {
        number: usize,
        text: String,
    },
    Code(String),
    Quote(String),
    Rule,
    Table {
        header: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

/// Split a markdown document into blocks.
pub fn parse(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<String> = Vec::new();
    let mut in_code = false;
    let mut code: Vec<String> = Vec::new();
    let mut list: Vec<String> = Vec::new();

    fn flush_paragraph(paragraph: &mut Vec<String>, blocks: &mut Vec<Block>) {
        if !paragraph.is_empty() {
            blocks.push(Block::Paragraph(paragraph.join(" ")));
            paragraph.clear();
        }
    }
    fn flush_list(list: &mut Vec<String>, blocks: &mut Vec<Block>) {
        for item in list.drain(..) {
            blocks.push(Block::Bullet(item));
        }
    }

    for raw in markdown.lines() {
        let line = raw.trim_end();

        if line.trim_start().starts_with("```") {
            if in_code {
                blocks.push(Block::Code(code.join("\n")));
                code.clear();
                in_code = false;
            } else {
                flush_paragraph(&mut paragraph, &mut blocks);
                flush_list(&mut list, &mut blocks);
                in_code = true;
            }
            continue;
        }
        if in_code {
            code.push(raw.to_string());
            continue;
        }

        let trimmed = line.trim_start();

        if trimmed.is_empty() {
            flush_paragraph(&mut paragraph, &mut blocks);
            flush_list(&mut list, &mut blocks);
            continue;
        }

        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            flush_paragraph(&mut paragraph, &mut blocks);
            flush_list(&mut list, &mut blocks);
            blocks.push(Block::Rule);
            continue;
        }

        if let Some(rest) = heading(trimmed) {
            flush_paragraph(&mut paragraph, &mut blocks);
            flush_list(&mut list, &mut blocks);
            blocks.push(rest);
            continue;
        }

        if is_table_row(trimmed) {
            flush_paragraph(&mut paragraph, &mut blocks);
            flush_list(&mut list, &mut blocks);
            let cells = split_row(trimmed);
            if cells
                .iter()
                .all(|c| c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' '))
            {
                continue; // the header separator
            }
            match blocks.last_mut() {
                Some(Block::Table { header, .. }) if header.is_empty() => {
                    *header = cells;
                }
                Some(Block::Table { rows, .. }) => rows.push(cells),
                _ => blocks.push(Block::Table {
                    header: cells,
                    rows: Vec::new(),
                }),
            }
            continue;
        }

        if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            flush_paragraph(&mut paragraph, &mut blocks);
            list.push(item.trim().to_string());
            continue;
        }
        if let Some(item) = numbered(trimmed) {
            flush_paragraph(&mut paragraph, &mut blocks);
            flush_list(&mut list, &mut blocks);
            blocks.push(item);
            continue;
        }
        if let Some(quote) = trimmed.strip_prefix("> ") {
            flush_paragraph(&mut paragraph, &mut blocks);
            flush_list(&mut list, &mut blocks);
            blocks.push(Block::Quote(quote.trim().to_string()));
            continue;
        }

        flush_list(&mut list, &mut blocks);
        paragraph.push(trimmed.to_string());
    }

    if in_code && !code.is_empty() {
        blocks.push(Block::Code(code.join("\n")));
    }
    flush_paragraph(&mut paragraph, &mut blocks);
    flush_list(&mut list, &mut blocks);
    blocks
}

fn is_table_row(line: &str) -> bool {
    line.starts_with('|') && line.ends_with('|') && line.matches('|').count() >= 2
}

fn heading(line: &str) -> Option<Block> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = line[hashes..].trim();
    if rest.is_empty() {
        return None;
    }
    Some(Block::Heading {
        level: hashes as u8,
        text: rest.to_string(),
    })
}

fn numbered(line: &str) -> Option<Block> {
    let digits: String = line.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let rest = line[digits.len()..].strip_prefix(". ")?;
    Some(Block::Numbered {
        number: digits.parse().unwrap_or(1),
        text: rest.trim().to_string(),
    })
}

fn split_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}

/// How the renderer colours a document.
#[derive(Debug, Clone)]
pub struct Style {
    pub body: TextStyle,
    pub code: Color32,
    pub muted: Color32,
    pub link: Color32,
    pub heading: Color32,
    pub rule: Color32,
}

impl Style {
    /// Take the colours from the active palette, so a theme switch reaches the
    /// report body too.
    pub fn from_palette(t: Palette) -> Style {
        Style {
            body: TextStyle::Body,
            code: t.info,
            muted: t.fg_muted,
            link: t.accent,
            heading: t.fg,
            rule: t.border,
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Style::from_palette(Palette::dark())
    }
}

/// The inline styles a run of text can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Inline {
    Plain,
    Strong,
    Italic,
    Code,
    Link,
    Url,
}

/// Turn inline markdown into a layout job.
///
/// `**bold**`, `*italic*`, `` `code` `` and `[label](url)` are understood; the
/// URL of a link is shown after its label, greyed, because the native client
/// has no tooltip that survives a screenshot.
pub fn inline_job(ui: &egui::Ui, text: &str, style: &Style) -> LayoutJob {
    let font = style.body.resolve(ui.style());
    let mut job = LayoutJob::default();
    // 1.5 line height: a report is read, not scanned. The default single-height
    // leading is what makes a wall of prose feel cramped.
    job.wrap.max_width = f32::INFINITY;
    let push = |chunk: &str, kind: Inline, job: &mut LayoutJob| {
        let (color, italics, font_id, underline) = match kind {
            Inline::Plain => (ui.visuals().text_color(), false, font.clone(), false),
            Inline::Strong => (style.heading, false, font.clone(), false),
            Inline::Italic => (ui.visuals().text_color(), true, font.clone(), false),
            Inline::Code => (style.code, false, FontId::monospace(font.size * 0.95), false),
            Inline::Link => (style.link, false, font.clone(), true),
            Inline::Url => (style.muted, false, font.clone(), false),
        };
        let underline = if underline {
            egui::Stroke::new(1.0, color)
        } else {
            egui::Stroke::NONE
        };
        job.append(
            chunk,
            0.0,
            TextFormat {
                font_id,
                color,
                italics,
                underline,
                line_height: Some(font.size * 1.5),
                ..Default::default()
            },
        );
    };

    let chars: Vec<char> = text.chars().collect();
    let mut buf = String::new();
    let mut i = 0;
    while i < chars.len() {
        let rest: String = chars[i..].iter().collect();
        if let Some(after) = rest.strip_prefix("**") {
            if let Some(end) = after.find("**") {
                if !buf.is_empty() {
                    push(&buf, Inline::Plain, &mut job);
                    buf.clear();
                }
                push(&after[..end], Inline::Strong, &mut job);
                i += 2 + end + 2;
                continue;
            }
        }
        if let Some(after) = rest.strip_prefix('`') {
            if let Some(end) = after.find('`') {
                if !buf.is_empty() {
                    push(&buf, Inline::Plain, &mut job);
                    buf.clear();
                }
                push(&after[..end], Inline::Code, &mut job);
                i += 1 + end + 1;
                continue;
            }
        }
        if let Some(after) = rest.strip_prefix('*') {
            if let Some(end) = after.find('*') {
                if !buf.is_empty() {
                    push(&buf, Inline::Plain, &mut job);
                    buf.clear();
                }
                push(&after[..end], Inline::Italic, &mut job);
                i += 1 + end + 1;
                continue;
            }
        }
        if let Some(after) = rest.strip_prefix('[') {
            if let Some(close) = after.find("](") {
                if let Some(url_end) = after[close + 2..].find(')') {
                    if !buf.is_empty() {
                        push(&buf, Inline::Plain, &mut job);
                        buf.clear();
                    }
                    let label = &after[..close];
                    let url = &after[close + 2..close + 2 + url_end];
                    push(label, Inline::Link, &mut job);
                    push(&format!(" ({url})"), Inline::Url, &mut job);
                    i += 1 + close + 2 + url_end + 1;
                    continue;
                }
            }
        }
        buf.push(chars[i]);
        i += 1;
    }
    if !buf.is_empty() {
        push(&buf, Inline::Plain, &mut job);
    }
    job
}

/// Draw a markdown document with the palette's styling.
pub fn render(ui: &mut egui::Ui, markdown: &str) {
    let style = Style::from_palette(theme::palette(ui.ctx()));
    render_style(ui, markdown, style);
}

/// Draw a markdown document with explicit styling.
pub fn render_style(ui: &mut egui::Ui, markdown: &str, style: Style) {
    let style = &style;
    let base = ui.text_style_height(&TextStyle::Body);
    for block in parse(markdown) {
        match block {
            Block::Heading { level, text } => {
                let size = match level {
                    1 => theme::HEADING,
                    2 => theme::H2,
                    3 => base * 1.12,
                    _ => base,
                };
                ui.add_space(if level <= 2 { 12.0 } else { 8.0 });
                ui.label(
                    RichText::new(&text)
                        .size(size)
                        .strong()
                        .color(style.heading),
                );
                if level == 1 {
                    ui.add_space(2.0);
                }
            }
            Block::Paragraph(text) => {
                let job = inline_job(ui, &text, style);
                ui.label(job);
                ui.add_space(6.0);
            }
            Block::Bullet(text) => {
                let job = inline_job(ui, &text, style);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("-").color(style.muted));
                    ui.label(job);
                });
            }
            Block::Numbered { number, text } => {
                let job = inline_job(ui, &text, style);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("{number}.")).color(style.muted));
                    ui.label(job);
                });
            }
            Block::Code(code) => {
                ui.add_space(4.0);
                egui::Frame::NONE
                    .fill(ui.visuals().extreme_bg_color)
                    .stroke(egui::Stroke::new(1.0, style.rule))
                    .corner_radius(egui::CornerRadius::same(theme::RADIUS))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.label(RichText::new(code).monospace().color(style.code));
                    });
                ui.add_space(6.0);
            }
            Block::Quote(text) => {
                let job = inline_job(ui, &text, style);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("|").color(style.muted));
                    ui.label(job);
                });
            }
            Block::Rule => {
                ui.add_space(6.0);
                crate::views::hairline(ui, style.rule_palette());
                ui.add_space(6.0);
            }
            Block::Table { header, rows } => {
                ui.add_space(4.0);
                egui::Grid::new(format!("md-table-{:x}", hash(&header, &rows)))
                    .num_columns(header.len().max(1))
                    .striped(true)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        for cell in &header {
                            ui.label(RichText::new(cell).strong().color(style.heading));
                        }
                        ui.end_row();
                        for row in &rows {
                            for cell in row {
                                let job = inline_job(ui, cell, style);
                                ui.label(job);
                            }
                            ui.end_row();
                        }
                    });
                ui.add_space(6.0);
            }
        }
    }
}

impl Style {
    /// The hairline colour, as a palette (a rule is one border).
    fn rule_palette(&self) -> Palette {
        Palette {
            border: self.rule,
            ..Palette::dark()
        }
    }
}

/// A stable id for a table, so two tables in one document do not share state.
fn hash(header: &[String], rows: &[Vec<String>]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    header.hash(&mut hasher);
    rows.hash(&mut hasher);
    hasher.finish()
}

/// Unused import guard for `FontFamily`, kept for an explicit family choice.
#[allow(dead_code)]
fn _family() -> FontFamily {
    FontFamily::Proportional
}
