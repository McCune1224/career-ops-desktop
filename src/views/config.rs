//! The config screen: the settings the other screens depend on.
//!
//! Which CLI to drive, the follow-up cadence, and the candidate details the
//! profile holds. The profile is a user-layer file; it is written through the
//! bridge's merge-safe writer, never clobbered.

use egui::{Margin, RichText};

use crate::app::CareerOpsApp;
use crate::theme::{self, Palette};
use crate::views::{frame, ghost_button, pill, primary_button};

/// Draw the config screen.
pub fn ui(ui: &mut egui::Ui, app: &mut CareerOpsApp) {
    let t = theme::palette(ui.ctx());
    frame(
        ui,
        "Config",
        Some("Engine mode: CLI. The app drives the same AI CLI the core does."),
        app,
        |ui, app| {
            theme::card(ui, t).show(ui, |ui| cli_section(ui, app, t));
            ui.add_space(12.0);
            theme::card(ui, t).show(ui, |ui| cadence_section(ui, app, t));
            ui.add_space(12.0);
            theme::card(ui, t).show(ui, |ui| profile_section(ui, app, t));
            ui.add_space(12.0);
            theme::card(ui, t).show(ui, |ui| profile_check_section(ui, app, t));
            ui.add_space(12.0);
            theme::card(ui, t).show(ui, |ui| plugins_section(ui, app, t));
            ui.add_space(12.0);
            theme::card(ui, t).show(ui, |ui| maintenance_section(ui, app, t));
            ui.add_space(12.0);
            theme::card(ui, t).show(ui, |ui| language_section(ui, app, t));
            ui.add_space(12.0);
            theme::card(ui, t).show(ui, |ui| inbox_section(ui, app, t));
            ui.add_space(12.0);
            theme::card(ui, t).show(ui, |ui| install_section(ui, app, t));
        },
    );
}

/// Which CLI the run screens drive.
fn cli_section(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    heading(ui, "AI CLI", t);
    let clis = app
        .state
        .bootstrap
        .as_ref()
        .map(|b| b.clis.clone())
        .or_else(|| app.state.config.as_ref().map(|c| c.clis.clone()))
        .unwrap_or_default();
    if clis.is_empty() {
        ui.label(RichText::new("No CLI list is available.").color(t.fg_muted));
        return;
    }

    // Installed first, and everything else folded away.
    //
    // The engine knows ten CLIs; a given machine has three or four. Listing all
    // ten as peers makes the picker a puzzle where most of the answers are
    // wrong, and the installed ones are the only answers that can start a run.
    let (installed, missing): (Vec<_>, Vec<_>) = clis.into_iter().partition(|c| c.installed);

    for cli in &installed {
        if cli_row(ui, app, cli, t) {
            app.settings.cli_id = cli.id.clone();
            let _ = app.settings.save();
        }
    }
    if installed.is_empty() {
        ui.label(
            RichText::new("No AI CLI is installed on this machine yet.")
                .color(t.warn),
        );
    }

    if !missing.is_empty() {
        egui::CollapsingHeader::new(
            RichText::new(format!("Not installed ({})", missing.len()))
                .text_style(theme::meta())
                .color(t.fg_muted),
        )
        .id_salt("clis-not-installed")
        .show(ui, |ui| {
            for cli in &missing {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&cli.name).color(t.fg_muted));
                    ui.label(
                        RichText::new(&cli.run)
                            .monospace()
                            .text_style(theme::meta())
                            .color(t.fg_muted),
                    );
                    if !cli.url.is_empty() && ui.link("install").clicked() {
                        crate::views::pipeline::open_url(ui, &cli.url);
                    }
                });
            }
        });
    }
}

/// One selectable CLI row. Returns true when it was clicked.
fn cli_row(
    ui: &mut egui::Ui,
    app: &CareerOpsApp,
    cli: &crate::model::Cli,
    t: Palette,
) -> bool {
    let selected = app.settings.cli_id == cli.id;
    let mut clicked = false;
    ui.horizontal(|ui| {
        if ui
            .selectable_label(selected, RichText::new(&cli.name).color(t.fg))
            .on_hover_text(&cli.run)
            .clicked()
        {
            clicked = true;
        }
        if selected {
            pill(ui, "selected", t.accent);
        }
        if let Some(path) = &cli.path {
            ui.label(
                RichText::new(path)
                    .monospace()
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        }
    });
    clicked
}

/// The follow-up cadence knobs, from config/profile.yml.
fn cadence_section(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    heading(ui, "Follow-up cadence", t);
    let mut cadence = match crate::backend::call_typed::<_, crate::model::Cadence>(
        app.backend.as_ref(),
        "followups",
        serde_json::json!({ "action": "cadence.get" }),
    ) {
        Ok(value) => value,
        Err(e) => {
            ui.label(
                RichText::new(format!("could not read the cadence: {e}")).color(t.fg_muted),
            );
            return;
        }
    };

    ui.checkbox(&mut cadence.enabled, "Follow-ups are on");

    let mut draft = cadence.effective.clone();
    let keys: Vec<String> = draft.keys().cloned().collect();
    for key in keys {
        let value = draft.get(&key).copied().unwrap_or(0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(humanize(&key))
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
            let mut edited = value as f32;
            ui.spacing_mut().slider_width = 190.0;
            if ui
                .add(egui::Slider::new(&mut edited, 0.0..=60.0).step_by(1.0))
                .changed()
            {
                draft.insert(key.clone(), edited as i64);
            }
        });
    }

    ui.add_space(6.0);
    if primary_button(ui, "Save the cadence", true).clicked() {
        let mut params = serde_json::Map::new();
        params.insert(
            "action".into(),
            serde_json::Value::String("cadence.set".into()),
        );
        for (key, value) in &draft {
            params.insert(key.clone(), serde_json::Value::from(*value));
        }
        params.insert("enabled".into(), serde_json::Value::Bool(cadence.enabled));
        match app
            .backend
            .call("followups", serde_json::Value::Object(params))
        {
            Ok(_) => {
                app.state.say("saved the follow-up cadence");
                app.load_config();
            }
            Err(e) => app.state.fail(format!("could not save the cadence: {e}")),
        }
    }
}

/// The candidate details held in config/profile.yml.
fn profile_section(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    heading(ui, "Candidate", t);
    let Some(config) = app.state.config.clone() else {
        ui.label(RichText::new("No profile loaded.").color(t.fg_muted));
        return;
    };
    if config.malformed {
        ui.horizontal_wrapped(|ui| {
            pill(ui, "refused", t.bad);
            ui.label(
                RichText::new(
                    "config/profile.yml exists but is not valid YAML — it will not be overwritten.",
                )
                .color(t.bad),
            );
        });
        return;
    }
    let candidate = config
        .profile
        .get("candidate")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let mut name = candidate
        .get("full_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mut email = candidate
        .get("email")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mut location = candidate
        .get("location")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    egui::Grid::new("candidate-fields")
        .num_columns(2)
        .spacing([14.0, 8.0])
        .show(ui, |ui| {
            field(ui, "Name", &mut name, t);
            field(ui, "Email", &mut email, t);
            field(ui, "Location", &mut location, t);
        });

    ui.add_space(6.0);
    if primary_button(ui, "Save the profile", true).clicked() {
        let params = serde_json::json!({
            "action": "setProfile",
            "name": name,
            "email": email,
            "location": location,
        });
        match app.backend.call("config", params) {
            Ok(_) => {
                app.state.say("saved the profile");
                app.load_config();
            }
            Err(e) => app.state.fail(format!("could not save the profile: {e}")),
        }
    }
}

fn field(ui: &mut egui::Ui, label: &str, value: &mut String, t: Palette) {
    ui.label(RichText::new(label).text_style(theme::meta()).color(t.fg_muted));
    ui.add(egui::TextEdit::singleline(value).desired_width(320.0));
    ui.end_row();
}

/// What the core's own validator says about the profile.
fn profile_check_section(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    heading(ui, "Profile check", t);
    ui.horizontal_wrapped(|ui| {
        if primary_button(ui, "Check the profile", true).clicked() {
            app.validate_profile();
        }
        if ghost_button(ui, "Health check").clicked() {
            app.run_maintenance("verify", false);
        }
    });
    let Some(validation) = app.state.profile_validation.clone() else {
        return;
    };
    ui.add_space(6.0);
    if validation.ok {
        ui.horizontal_wrapped(|ui| {
            pill(ui, "fine", t.good);
            ui.label(RichText::new("the profile is fine").color(t.good));
        });
        return;
    }
    for finding in validation.errors.iter().chain(validation.warnings.iter()) {
        ui.horizontal_wrapped(|ui| {
            pill(
                ui,
                &finding.level,
                if finding.level == "error" { t.bad } else { t.warn },
            );
            ui.label(RichText::new(&finding.message).color(t.fg));
            if let Some(suggestion) = &finding.suggestion {
                ui.label(
                    RichText::new(suggestion)
                        .text_style(theme::meta())
                        .color(t.fg_muted),
                );
            }
        });
    }
}

/// The output language and the market mode set.
fn language_section(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    heading(ui, "Language and market", t);
    ui.horizontal_wrapped(|ui| {
        if ghost_button(ui, "Read it").clicked() {
            match app
                .backend
                .call("profile", serde_json::json!({ "action": "state" }))
            {
                Ok(value) => {
                    let language = value.get("language").cloned().unwrap_or_default();
                    let output = language
                        .get("output")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("en");
                    let modes = language
                        .get("modesDir")
                        .map(|v| {
                            if let Some(list) = v.as_array() {
                                list.iter()
                                    .filter_map(|x| x.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            } else {
                                v.as_str().unwrap_or("modes").to_string()
                            }
                        })
                        .unwrap_or_else(|| "modes".to_string());
                    app.state
                        .say(format!("output {output} · market {modes}"));
                }
                Err(e) => app.state.fail(format!("could not read the language: {e}")),
            }
        }
        if ghost_button(ui, "Turn the personalization on").clicked() {
            app.run_op(
                "profile",
                serde_json::json!({ "action": "state" }),
            );
        }
    });
    if !app.state.notice.is_empty() {
        ui.label(
            RichText::new(&app.state.notice)
                .monospace()
                .text_style(theme::meta())
                .color(t.fg_muted),
        );
    }
}

/// The checklist of requests for a later session.
fn inbox_section(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    heading(ui, "For a later session", t);
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.inbox_request)
                .desired_width(260.0)
                .hint_text("what to do later"),
        );
        if primary_button(ui, "Queue it", !app.inbox_request.trim().is_empty()).clicked() {
            let request = app.inbox_request.clone();
            app.run_op(
                "agent-inbox",
                serde_json::json!({ "action": "add", "request": request }),
            );
            app.inbox_request.clear();
        }
        if ghost_button(ui, "Read the checklist").clicked() {
            match app
                .backend
                .call("agent-inbox", serde_json::json!({ "action": "list" }))
            {
                Ok(value) => {
                    let items = value
                        .get("items")
                        .and_then(serde_json::Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    app.state.say(format!("{} item(s) queued", items.len()));
                }
                Err(e) => app.state.fail(format!("could not read the checklist: {e}")),
            }
        }
    });
    if !app.state.notice.is_empty() {
        ui.label(
            RichText::new(&app.state.notice)
                .monospace()
                .text_style(theme::meta())
                .color(t.fg_muted),
        );
    }
}

/// The core's own repair tools.
///
/// A proposal first and an action second: a repair is a change to the user's
/// tracker, and the user sees what the script would do before it does it.
fn maintenance_section(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    heading(ui, "Maintenance", t);
    for (tool, description) in crate::views::maintenance::TOOLS {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(description).color(t.fg));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ghost_button(ui, "Apply").clicked() {
                    app.run_maintenance(tool, true);
                }
                if ghost_button(ui, "Preview").clicked() {
                    app.run_maintenance(tool, false);
                }
            });
        });
        ui.add_space(3.0);
    }
    if let Some(result) = app.state.maintenance.clone() {
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            pill(
                ui,
                if result.apply { "applied" } else { "proposal" },
                if result.ok { t.good } else { t.warn },
            );
            ui.label(
                RichText::new(result.output.lines().next().unwrap_or("").to_string())
                    .text_style(theme::meta())
                    .color(t.fg_muted),
            );
        });
    }
}

/// The plugins the core can load.
fn plugins_section(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    heading(ui, "Plugins", t);
    ui.horizontal_wrapped(|ui| {
        if ghost_button(ui, "Reload").clicked() {
            app.load_plugins();
        }
        if ghost_button(ui, "Audit a plugin").clicked() {
            let dir = std::env::var("CAREER_OPS_PLUGIN_DIR").unwrap_or_default();
            let params = serde_json::json!({ "action": "audit", "dir": dir });
            match app.backend.call("plugins", params) {
                Ok(value) => {
                    let output = value
                        .get("output")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    app.state.say(output.lines().next().unwrap_or("audited").to_string());
                }
                Err(e) => app.state.fail(format!("could not audit the plugin: {e}")),
            }
        }
    });
    let Some(plugins) = app.state.plugins.clone() else {
        return;
    };
    if plugins.is_empty() {
        ui.label(
            RichText::new("The core reports no plugins.")
                .text_style(theme::meta())
                .color(t.fg_muted),
        );
        return;
    }
    for plugin in &plugins {
        ui.horizontal(|ui| {
            ui.label(RichText::new(&plugin.id).strong().color(t.fg));
            if !plugin.name.is_empty() {
                ui.label(RichText::new(&plugin.name).color(t.fg_muted));
            }
            pill(
                ui,
                if plugin.enabled { "enabled" } else { "off" },
                if plugin.enabled { t.good } else { t.fg_muted },
            );
            let id = plugin.id.clone();
            if ghost_button(ui, "Disable").clicked() {
                match app
                    .backend
                    .call("plugins", serde_json::json!({ "action": "disable", "id": id }))
                {
                    Ok(_) => {
                        app.load_plugins();
                        app.state.say("disabled the plugin");
                    }
                    Err(e) => app.state.fail(format!("could not disable the plugin: {e}")),
                }
            }
        });
        ui.add_space(3.0);
    }
}

/// Where the app is reading from and writing to.
fn install_section(ui: &mut egui::Ui, app: &mut CareerOpsApp, t: Palette) {
    heading(ui, "This install", t);
    let Some(state) = &app.state.bootstrap else {
        ui.label(RichText::new("Not loaded.").color(t.fg_muted));
        return;
    };
    let version = app
        .state
        .version
        .as_ref()
        .map(|v| {
            if v.protocol == crate::bridge::PROTOCOL_VERSION {
                format!("bridge protocol {} · engine {}", v.protocol, v.engine)
            } else {
                format!(
                    "MISMATCH — the app speaks protocol {} and the bridge speaks {}",
                    crate::bridge::PROTOCOL_VERSION,
                    v.protocol
                )
            }
        })
        .unwrap_or_else(|| "the bridge version is unknown".to_string());
    let rows = [
        ("Data root", state.data_root.clone()),
        ("Engine checkout", state.code_root.clone()),
        ("Client", format!("version {}", env!("CARGO_PKG_VERSION"))),
        ("Versions", version),
        (
            "Inventory",
            format!(
                "{} applications · {} pending · {} reports · {} PDFs",
                state.application_count, state.inbox_count, state.reports, state.pdfs
            ),
        ),
        (
            "Zoom",
            format!("{:.0}% — Ctrl +/- to change, Ctrl+0 to reset", app.settings.zoom * 100.0),
        ),
    ];
    egui::Grid::new("install-rows")
        .num_columns(2)
        .spacing([14.0, 6.0])
        .show(ui, |ui| {
            for (label, value) in rows {
                ui.label(
                    RichText::new(label)
                        .text_style(theme::meta())
                        .color(t.fg_muted),
                );
                ui.label(RichText::new(value).monospace().color(t.fg));
                ui.end_row();
            }
        });

    if !state.doctor.missing.is_empty() {
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            pill(ui, "missing", t.warn);
            ui.label(
                RichText::new(state.doctor.missing.join(", ")).color(t.warn),
            );
        });
    }
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        if ghost_button(ui, "Check the version").clicked() {
            app.load_version();
        }
    });
    if !app.version_matches() {
        ui.horizontal_wrapped(|ui| {
            pill(ui, "mismatch", t.bad);
            ui.label(
                RichText::new(
                    "the app and the bridge do not agree — no run will start until they do.",
                )
                .color(t.bad),
            );
        });
    }
}

/// A card heading with a hairline under it.
fn heading(ui: &mut egui::Ui, title: &str, t: Palette) {
    ui.label(
        RichText::new(title)
            .text_style(theme::h2())
            .strong()
            .color(t.fg),
    );
    ui.add_space(4.0);
    crate::views::hairline(ui, t);
    ui.add_space(8.0);
    let _ = Margin::same(0);
}

/// "applied_first_days" -> "First follow-up (days after applying)".
fn humanize(key: &str) -> String {
    match key {
        "applied_first_days" => "First follow-up (days after applying)".into(),
        "applied_second_days" => "Second follow-up (days after applying)".into(),
        "applied_third_days" => "Third follow-up (days after applying)".into(),
        "applied_max_followups" => "Maximum follow-ups".into(),
        "responded_first_days" => "First follow-up after a reply (days)".into(),
        "responded_second_days" => "Second follow-up after a reply (days)".into(),
        "rejected_followups" => "Follow-ups after a rejection".into(),
        other => other.replace('_', " "),
    }
}
