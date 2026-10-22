//! Where the career-ops ENGINE checkout is.
//!
//! This client is a separate repository from the engine it drives, so the
//! checkout is NAMED, not inferred. Walking up from the executable until
//! something looked like a checkout was the old rule, and it was wrong the
//! moment the client left the career-ops tree: it could find a directory that
//! merely had a `web/` folder in it and call it the engine.
//!
//! The rule, in order:
//!
//!   1. `CAREER_OPS_CODE_ROOT`, when it is set. It wins outright.
//!   2. A sibling checkout: <this repository's parent>/career-ops.
//!   3. Otherwise, one actionable sentence, and nothing else.
//!
//! `bridge/code-root.mjs` implements the same rule in Node, and
//! `tests/desktop-engine-parity.test.mjs` holds the two to the same answer —
//! the app, the bridge and the test fixtures must never disagree about which
//! checkout they are talking to.
//!
//! The engine checkout is NOT the data root. A user can point the data root at
//! a data-only directory; the scripts and the engine modules still have to come
//! from a real checkout.

use std::path::{Path, PathBuf};

/// The environment variable that names the engine checkout.
pub const CODE_ROOT_ENV: &str = "CAREER_OPS_CODE_ROOT";

/// The directory name of a career-ops checkout, for the sibling fallback.
pub const CHECKOUT_DIR_NAME: &str = "career-ops";

/// The one sentence a caller gets when no checkout can be found.
///
/// One sentence, and it names both ways out: set the variable, or put a
/// checkout where the sibling rule looks for it.
pub const ACTIONABLE: &str =
    "No career-ops checkout found: set CAREER_OPS_CODE_ROOT to the checkout, or place one at ../career-ops.";

/// The file that marks a directory as a career-ops checkout.
///
/// It is the engine module the bridge imports first, so a directory without it
/// cannot be the engine whatever else it contains.
const CHECKOUT_MARKER: [&str; 4] = ["web", "src", "lib", "career-ops.ts"];

/// Where the resolved checkout came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeRootSource {
    /// Named by `CAREER_OPS_CODE_ROOT`, or by the app's own settings.
    Explicit,
    /// A sibling checkout of the same parent directory.
    Sibling,
}

impl CodeRootSource {
    /// The word used on screen and in the settings file.
    pub fn as_str(self) -> &'static str {
        match self {
            CodeRootSource::Explicit => "explicit",
            CodeRootSource::Sibling => "sibling",
        }
    }
}

/// A resolved engine checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeRoot {
    /// The absolute path of the checkout.
    pub path: PathBuf,
    /// How it was found.
    pub source: CodeRootSource,
}

impl CodeRoot {
    /// The path as a string, for display and for the settings file.
    pub fn as_str(&self) -> String {
        self.path.display().to_string()
    }
}

/// Is this directory a career-ops checkout?
pub fn is_checkout(dir: &Path) -> bool {
    let mut marker = dir.to_path_buf();
    for part in CHECKOUT_MARKER {
        marker.push(part);
    }
    marker.is_file()
}

/// The sibling fallback: a checkout in the same parent directory as this
/// repository.
pub fn sibling_candidate(client_root: &Path) -> PathBuf {
    client_root
        .parent()
        .unwrap_or(client_root)
        .join(CHECKOUT_DIR_NAME)
}

/// Resolve the engine checkout.
///
/// `explicit` is the value of `CAREER_OPS_CODE_ROOT` (or the app's own
/// remembered value). A relative one is resolved against THIS repository's
/// root, not against the process's working directory: the bridge runs with its
/// working directory set to the engine checkout, so a relative path would
/// otherwise mean two different things depending on who started the process.
///
/// Returns the actionable sentence as the error when no checkout can be found.
pub fn resolve(explicit: Option<&str>, client_root: &Path) -> Result<CodeRoot, String> {
    let explicit = explicit.unwrap_or("").trim();
    if !explicit.is_empty() {
        let candidate = Path::new(explicit);
        let resolved = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            client_root.join(candidate)
        };
        if !is_checkout(&resolved) {
            return Err(ACTIONABLE.to_string());
        }
        return Ok(CodeRoot {
            path: resolved,
            source: CodeRootSource::Explicit,
        });
    }

    let sibling = sibling_candidate(client_root);
    if is_checkout(&sibling) {
        return Ok(CodeRoot {
            path: sibling,
            source: CodeRootSource::Sibling,
        });
    }
    Err(ACTIONABLE.to_string())
}

/// This repository's root, as the build saw it.
///
/// The client and the engine are separate checkouts, so "the parent directory"
/// is a real, meaningful place: the sibling rule looks for the engine there.
pub fn client_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
