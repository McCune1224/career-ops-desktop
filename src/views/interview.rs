//! The interview screen: the story bank with its provenance verdict, the match
//! of a story to a question, and the week's digest.
//!
//! The provenance verdict is the point of this screen. A number in the story
//! bank that no primary file supports is marked, and the app shows that mark
//! rather than repeating the number as fact.

use egui::RichText;

use crate::app::CareerOpsApp;
use crate::theme::{self, Palette};
use crate::views::frame;

/// The question and its answer.
#[derive(Debug, Clone, Default)]
pub struct StoryMatch {
    pub question: String,
    pub story: String,
}

/// Draw the interview screen.
pub fn ui(ui: &mut egui::Ui, app: &mut CareerOpsApp) {
    let t = theme::palette(ui.ctx());
    frame(
        ui,
        "Interview",
        Some("Your stories, what may be said about them, and the week's gaps."),
        app,
        |ui, app| {
            bank_card(ui, app, t);
            ui.add_space(14.0);
            match_card(ui, app, t);
        },
    );
}

/// The story bank and what may be said about it.
fn bank_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Story bank", |ui| {
            if crate::views::ghost_button(ui, "Reload").clicked() {
                app.load_story_bank();
            }
        });
        ui.add_space(8.0);
        let Some(bank) = app.state.story_bank.clone() else {
            ui.spinner();
            ui.label("Reading the story bank…");
            return;
        };
        if !bank.exists {
            ui.label(
                RichText::new("There is no story bank yet.")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            return;
        }
        crate::views::key_values(
            ui,
            &[
                (
                    "supported by your CV",
                    bank.supported_by_resume.len().to_string(),
                ),
                ("not verified", bank.derived_unverified.len().to_string()),
                (
                    "you cannot confirm",
                    bank.user_cannot_confirm.len().to_string(),
                ),
            ],
        );
        ui.add_space(10.0);
        if !bank.user_cannot_confirm.is_empty() {
            ui.label(
                RichText::new("Never restated as fact")
                    .text_style(theme::meta())
                    .strong()
                    .color(t.warn),
            );
            for story in &bank.user_cannot_confirm {
                ui.label(
                    RichText::new(format!("— {}", story.story))
                        .text_style(theme::meta())
                        .color(t.fg_muted),
                );
            }
        }
        if !bank.derived_unverified.is_empty() {
            ui.add_space(8.0);
            ui.label(
                RichText::new("Not verified — say them as narrative, not as numbers")
                    .text_style(theme::meta())
                    .strong()
                    .color(t.warn),
            );
            for story in &bank.derived_unverified {
                ui.label(
                    RichText::new(format!("— {}", story.story))
                        .text_style(theme::meta())
                        .color(t.fg_muted),
                );
            }
        }
        if !bank.supported_by_resume.is_empty() {
            ui.add_space(8.0);
            ui.label(
                RichText::new("Backed by your CV")
                    .text_style(theme::meta())
                    .strong()
                    .color(t.good),
            );
            for story in &bank.supported_by_resume {
                ui.label(
                    RichText::new(format!("— {}", story.story))
                        .text_style(theme::meta())
                        .color(t.fg_muted),
                );
            }
        }
    });
}

/// Match a question to a story.
fn match_card(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    theme::card(ui, t).show(ui, |ui| {
        crate::views::card_title(ui, "Match a story to a question", |ui| {
            ui.label(
                RichText::new("offline")
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.story_match.question)
                    .desired_width(f32::INFINITY)
                    .hint_text("the question you were asked"),
            );
            if crate::views::primary_button(ui, "Match", !app.story_match.question.trim().is_empty()).clicked() {
                match_story(app);
            }
        });
        if !app.story_match.story.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new(&app.story_match.story).color(t.fg));
        }
    });
}

/// Ask the core to match the question.
fn match_story(app: &mut CareerOpsApp) {
    let params = serde_json::json!({
        "action": "matchStar",
        "question": app.story_match.question,
    });
    match app.backend.call("interview", params) {
        Ok(value) => {
            app.story_match.story = value
                .get("story")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();
        }
        Err(e) => {
            app.story_match.story.clear();
            app.state.fail(format!("could not match a story: {e}"));
        }
    }
}
