//! The native career-ops client.
//!
//! The UI is egui; the engine is the Node bridge. Nothing here reimplements a
//! career-ops rule: every read and every write goes through the bridge, which
//! calls the core's own scripts and the web app's own modules.
//!
//! The library is split out from `main.rs` so the screens can be rendered
//! headlessly in tests (`tests/screens.rs`) without a window, a display server
//! or a bridge process.

pub mod app;
pub mod backend;
pub mod bridge;
pub mod code_root;
pub mod markdown;
pub mod model;
pub mod settings;
pub mod shell;
pub mod theme;
pub mod views;

pub use app::{CareerOpsApp, RunKind, Screen};
pub use backend::Backend;
pub use code_root::CodeRoot;
pub use settings::{Settings, Theme};
