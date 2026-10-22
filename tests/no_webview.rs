//! No webview, proven rather than promised.
//!
//! "egui, with no webview involved" is the constraint this client exists to
//! satisfy, and it is the kind of claim that quietly stops being true: a crate
//! pulled in for an unrelated reason, a renderer that ships a WebKit runtime,
//! a helper that links CEF. So it is asserted twice, on two different
//! artifacts, from two different directions.
//!
//!   1. `Cargo.lock` — names the crates. Catches a webview dependency before it
//!      is even built, which is the only moment the offending crate is obvious.
//!   2. `ldd` on the built binary — names the runtimes. Catches one that
//!      arrived through a system library, which no lockfile can see.
//!
//! The second check is skipped (with a message) when the binary has not been
//! built: `cargo test` builds the test harness, not necessarily the release
//! binary. `cargo build` first, or run `node desktop/dev/dependency-audit.mjs`,
//! which does both and prints the full library list.

use std::path::{Path, PathBuf};

/// Crate names that mean a webview is in the dependency tree.
const FORBIDDEN_CRATES: &[&str] = &[
    "wry",
    "tauri",
    "tauri-build",
    "tauri-runtime",
    "tauri-runtime-wry",
    "tauri-codegen",
    "webkit2gtk",
    "webkit2gtk-sys",
    "wpewebkit",
    "wpewebkit-sys",
    "webview2-com",
    "webview2-com-sys",
    "cef",
    "cef-dll-sys",
    "libcef",
    "servo-webview",
];

/// Substrings that mean a webview runtime is linked, matched lower-cased.
const FORBIDDEN_LIBRARY_SUBSTRINGS: &[&str] = &["webkit", "wpewebkit", "libcef", "cef", "webview2"];

fn desktop_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The `name = "…"` entries in a Cargo.lock.
fn lock_names(lock: &str) -> Vec<String> {
    lock.lines()
        .filter_map(|line| line.trim().strip_prefix("name = \""))
        .filter_map(|rest| rest.split('"').next())
        .map(str::to_string)
        .collect()
}

#[test]
fn the_lockfile_names_no_webview_crate() {
    let lock_path = desktop_dir().join("Cargo.lock");
    let lock = std::fs::read_to_string(&lock_path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", lock_path.display()));
    let names = lock_names(&lock);
    assert!(!names.is_empty(), "Cargo.lock has no crate names — this check would be a no-op");

    let offenders: Vec<&str> = FORBIDDEN_CRATES
        .iter()
        .copied()
        .filter(|crate_name| names.iter().any(|n| n == crate_name))
        .collect();
    assert!(
        offenders.is_empty(),
        "Cargo.lock names a webview crate: {}. This app renders with egui and must never embed a browser.",
        offenders.join(", ")
    );
}

#[test]
fn the_lockfile_lists_the_renderer_it_does_use() {
    // The mirror of the check above: it is not enough that the forbidden names
    // are absent — the expected renderer must be there, or the binary would not
    // be drawing anything at all.
    let lock = std::fs::read_to_string(desktop_dir().join("Cargo.lock")).expect("Cargo.lock");
    let names = lock_names(&lock);
    for expected in ["eframe", "egui", "egui_glow", "glow"] {
        assert!(
            names.iter().any(|n| n == expected),
            "Cargo.lock does not name {expected} — the audit is looking at the wrong tree"
        );
    }
}

#[test]
fn the_built_binary_links_no_webview_runtime() {
    // Try release first, then debug: either is a real link of the real app.
    let candidates = [
        desktop_dir().join("target/release/career-ops-desktop"),
        desktop_dir().join("target/debug/career-ops-desktop"),
    ];
    let Some(binary) = candidates.iter().find(|p| p.is_file()) else {
        eprintln!(
            "skipped: no built binary yet — run `cargo build`, or `node desktop/dev/dependency-audit.mjs`, which builds and checks in one step"
        );
        return;
    };

    let output = std::process::Command::new("ldd")
        .arg(binary)
        .output()
        .expect("ldd must be available on a Linux target");
    assert!(
        output.status.success(),
        "ldd failed on {}: {}",
        binary.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    let listing = String::from_utf8_lossy(&output.stdout);
    assert!(
        listing.contains("=>"),
        "ldd printed no library mappings — the check would be a no-op:\n{listing}"
    );

    let offenders: Vec<&str> = listing
        .lines()
        .filter(|line| {
            let lower = line.to_lowercase();
            FORBIDDEN_LIBRARY_SUBSTRINGS
                .iter()
                .any(|needle| lower.contains(needle))
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "the binary links a webview runtime:\n{}\nFull listing:\n{listing}",
        offenders.join("\n")
    );
}

#[test]
fn the_renderer_is_glow_not_wgpu() {
    // A thin but real check on the manifest: the OpenGL backend is what keeps
    // the linked-library surface small enough that the ldd assertion above is
    // meaningful (glow is loaded through libloading, so the binary links almost
    // nothing).
    let manifest = std::fs::read_to_string(desktop_dir().join("Cargo.toml")).expect("Cargo.toml");
    assert!(
        manifest.contains("\"glow\""),
        "eframe must be built with the glow (OpenGL) backend"
    );
    assert!(
        !manifest.contains("\"wgpu\""),
        "eframe must not be built with the wgpu backend — the audit's linked-library claim is about glow"
    );
    let _ = Path::new("Cargo.toml");
}
