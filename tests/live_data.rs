//! Decode the REAL data root, on demand.
//!
//! The other suites use fixtures, which is right for asserting behaviour but
//! leaves one question open: does the client's model match what THIS machine's
//! engine actually sends? A field that is `null` here and `""` in a fixture is
//! invisible to every other test and shows up as an empty window.
//!
//! So this suite is opt-in and reads the live root:
//!
//! ```bash
//! CAREER_OPS_REAL_ROOT_TESTS=1 cargo test --test live_data -- --nocapture
//! ```
//!
//! It is READ-ONLY by construction — every operation it calls is a read, and it
//! asserts the data root's digest is unchanged afterwards, so running it before
//! a walkthrough cannot move the user's data.
//!
//! Skipped (with a printed reason, never a silent pass) when the variable is
//! unset, because `cargo test` must not depend on a user's files.

use std::path::{Path, PathBuf};

use career_ops_desktop::backend::BridgeBackend;
use career_ops_desktop::bridge::{Bridge, BridgePaths};
use career_ops_desktop::code_root;
use career_ops_desktop::model::{Bootstrap, Config, Cv, Followups, Pipeline, Report};

fn enabled() -> bool {
    matches!(
        std::env::var("CAREER_OPS_REAL_ROOT_TESTS").as_deref(),
        Ok("1") | Ok("true")
    )
}

/// A digest of every file the client reads, so the suite can prove it wrote
/// nothing.
fn digest(root: &Path) -> String {
    use std::collections::BTreeMap;
    use std::hash::{Hash, Hasher};

    fn walk(dir: &Path, into: &mut BTreeMap<PathBuf, u64>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                walk(&path, into);
            } else if meta.is_file() {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                if let Ok(bytes) = std::fs::read(&path) {
                    bytes.hash(&mut hasher);
                }
                into.insert(path, hasher.finish());
            }
        }
    }

    let mut files = BTreeMap::new();
    for part in ["data", "reports", "config", "modes", "output"] {
        walk(&root.join(part), &mut files);
    }
    for file in ["cv.md", "portals.yml"] {
        let path = root.join(file);
        if let Ok(bytes) = std::fs::read(&path) {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            bytes.hash(&mut hasher);
            files.insert(path, hasher.finish());
        }
    }

    let count = files.len();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for (path, hash) in files {
        path.hash(&mut hasher);
        hash.hash(&mut hasher);
    }
    format!("{:016x} over {count} files", hasher.finish())
}

fn open_bridge(root: &Path) -> std::sync::Arc<Bridge> {
    // The engine comes from the checkout this app is configured against, even
    // when the data root is elsewhere. The rule is the app's own: the
    // environment variable, else a sibling checkout, else one sentence.
    let paths = BridgePaths::discover(None).expect(
        "the career-ops checkout must be reachable: set CAREER_OPS_CODE_ROOT, or place one at ../career-ops",
    );
    Bridge::start(paths, Some(root)).expect("the bridge must start")
}

/// The engine checkout this suite would use, resolved the same way the app
/// resolves it.
fn engine_checkout() -> PathBuf {
    code_root::resolve(
        std::env::var(code_root::CODE_ROOT_ENV).ok().as_deref(),
        &code_root::client_root(),
    )
    .expect("the career-ops checkout must be reachable")
    .path
}

#[test]
fn the_live_data_root_decodes_and_is_left_untouched() {
    if !enabled() {
        eprintln!(
            "skipped: set CAREER_OPS_REAL_ROOT_TESTS=1 to decode the live data root (read-only)"
        );
        return;
    }

    // The data root the app itself would use: CAREER_OPS_ROOT, else the
    // engine checkout.
    let root = std::env::var("CAREER_OPS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| engine_checkout());
    assert!(root.is_dir(), "the data root does not exist: {}", root.display());

    let before = digest(&root);
    let bridge = open_bridge(&root);
    let backend = BridgeBackend::new(bridge);

    let bootstrap: Bootstrap = career_ops_desktop::backend::call_typed(
        &backend,
        "bootstrap",
        serde_json::json!({}),
    )
    .unwrap_or_else(|e| panic!("bootstrap does not decode against {}: {e}", root.display()));
    eprintln!(
        "bootstrap: {} applications, {} pending, {} reports, {} PDFs, {} states, {} CLIs",
        bootstrap.application_count,
        bootstrap.inbox_count,
        bootstrap.reports,
        bootstrap.pdfs,
        bootstrap.states.len(),
        bootstrap.clis.len()
    );
    assert!(!bootstrap.states.is_empty(), "the engine offered no canonical states");

    let pipeline: Pipeline = career_ops_desktop::backend::call_typed(
        &backend,
        "pipeline",
        serde_json::json!({}),
    )
    .expect("pipeline does not decode against the live root");
    assert_eq!(
        pipeline.applications.len(),
        bootstrap.application_count,
        "the bootstrap count and the pipeline rows disagree — the two reads are not of one tracker"
    );
    eprintln!("pipeline: {} rows, {} inbox", pipeline.applications.len(), pipeline.inbox.len());

    // Every application must decode, not just the first: one row with a shape
    // the model cannot hold would empty the whole table at runtime.
    for app in &pipeline.applications {
        let report: Result<Report, _> = career_ops_desktop::backend::call_typed(
            &backend,
            "report",
            serde_json::json!({ "n": app.n }),
        );
        if let Err(e) = report {
            // A row with no report at all is legitimate; a decode failure is not.
            assert!(
                e.code == "not-found",
                "report {} of {} does not decode: {e}",
                app.n,
                pipeline.applications.len()
            );
        }
    }
    eprintln!("reports: {} rows decoded", pipeline.applications.len());

    let _cv: Cv = career_ops_desktop::backend::call_typed(
        &backend,
        "cv",
        serde_json::json!({ "action": "read" }),
    )
    .expect("cv does not decode against the live root");

    let followups: Followups = career_ops_desktop::backend::call_typed(
        &backend,
        "followups",
        serde_json::json!({ "action": "list" }),
    )
    .expect("followups do not decode against the live root");
    eprintln!(
        "followups: available={}, {} entries, {} due, {} logged",
        followups.available,
        followups.entries.len(),
        followups.due.len(),
        followups.history.len()
    );

    let config: Config = career_ops_desktop::backend::call_typed(
        &backend,
        "config",
        serde_json::json!({ "action": "get" }),
    )
    .expect("config does not decode against the live root");
    assert!(!config.malformed, "config/profile.yml is not a mapping — the config screen would refuse to edit it");
    eprintln!("config: {} CLIs known, data root {}", config.clis.len(), config.data_root);

    // Nothing above may have written. This is the guarantee that makes it safe
    // to run before a walkthrough.
    let after = digest(&root);
    assert_eq!(before, after, "a read-only decode changed the data root");
    eprintln!("digest unchanged: the live root was not modified");
}
