//! The native career-ops client.
//!
//! The window, the screens and everything they do live in the library; this
//! binary only opens the window.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result<()> {
    career_ops_desktop::shell::run()
}
