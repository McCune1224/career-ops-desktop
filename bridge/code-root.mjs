// Where the career-ops ENGINE checkout is.
//
// This client is a separate repository from the engine it drives, so the
// checkout is NAMED, not inferred. Walking up from the bridge until something
// looks like a checkout was the old rule, and it was wrong the moment the
// client left the career-ops tree: it could find a directory that merely had a
// `web/` folder in it and call it the engine.
//
// The rule, in order:
//
//   1. `CAREER_OPS_CODE_ROOT`, when it is set. It wins outright.
//   2. A sibling checkout: <this repository's parent>/career-ops.
//   3. Otherwise, one actionable sentence, and nothing else.
//
// `src/code_root.rs` implements the same rule in Rust, and
// `tests/desktop-engine-parity.test.mjs` holds the two to the same answer —
// the app, the bridge and the test fixtures must never disagree about which
// checkout they are talking to.
//
// The engine checkout is NOT the data root. A user can point the data root at
// a data-only directory; the scripts and the engine modules still have to come
// from a real checkout.
import path from "node:path";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";

/** The environment variable that names the engine checkout. */
export const CODE_ROOT_ENV = "CAREER_OPS_CODE_ROOT";

/** The directory name of a career-ops checkout, for the sibling fallback. */
export const CHECKOUT_DIR_NAME = "career-ops";

/**
 * The one sentence a caller gets when no checkout can be found.
 *
 * One sentence, and it names both ways out: set the variable, or put a
 * checkout where the sibling rule looks for it.
 */
export const ACTIONABLE =
  "No career-ops checkout found: set CAREER_OPS_CODE_ROOT to the checkout, or place one at ../career-ops.";

/**
 * The file that marks a directory as a career-ops checkout.
 *
 * It is the engine module the bridge imports first, so a directory without it
 * cannot be the engine whatever else it contains.
 */
const CHECKOUT_MARKER = ["web", "src", "lib", "career-ops.ts"];

/** This repository's root: the directory holding `bridge/`. */
export const CLIENT_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);

/**
 * Is this directory a career-ops checkout?
 *
 * @param {string} dir
 * @returns {boolean}
 */
export function isCheckout(dir) {
  return existsSync(path.join(dir, ...CHECKOUT_MARKER));
}

/**
 * The sibling fallback: a checkout in the same parent directory as this
 * repository.
 *
 * @param {string} [clientRoot]
 * @returns {string}
 */
export function siblingCandidate(clientRoot = CLIENT_ROOT) {
  return path.join(path.dirname(path.resolve(clientRoot)), CHECKOUT_DIR_NAME);
}

/**
 * Resolve the engine checkout.
 *
 * @param {{explicit?: string | null, clientRoot?: string}} [options]
 *   `explicit` is the value of CAREER_OPS_CODE_ROOT; it is read from the
 *   environment when not given.
 * @returns {{path: string, source: "environment" | "sibling"}}
 * @throws {Error} with `ACTIONABLE` when no checkout can be found.
 */
export function resolveCodeRoot(options = {}) {
  const clientRoot = options.clientRoot ?? CLIENT_ROOT;
  const raw = options.explicit ?? process.env[CODE_ROOT_ENV] ?? "";
  const explicit = String(raw).trim();

  if (explicit) {
    // A relative value is resolved against THIS repository's root, not against
    // the process's working directory: the bridge runs with its working
    // directory set to the engine checkout, so a relative path would otherwise
    // mean two different things depending on who started the process.
    const resolved = path.resolve(clientRoot, explicit);
    if (!isCheckout(resolved)) {
      throw new Error(ACTIONABLE);
    }
    return { path: resolved, source: "environment" };
  }

  const sibling = siblingCandidate(clientRoot);
  if (isCheckout(sibling)) {
    return { path: sibling, source: "sibling" };
  }
  throw new Error(ACTIONABLE);
}
