#!/usr/bin/env node
// make-fixture.mjs — materialize a throwaway copy of the career-ops data root.
//
// The desktop app is developed against a FIXTURE, never against the user's
// live data: every write the app performs (a status change, a logged follow-up,
// a saved CV, a generated report) must land somewhere disposable while the
// screens are being built and tested.
//
// The fixture is laid out the way a real install is: the USER files are copied
// (so a write cannot escape to the original), the SYSTEM files are symlinked
// (so a change to a core script is visible in the fixture immediately, with no
// re-copy step).
//
// Usage:
//   node desktop/dev/make-fixture.mjs [targetDir]
//
// The target defaults to <tmp>/career-ops-fixture. The result is a directory
// that can be used as CAREER_OPS_ROOT.
//
// Why symlinks for the system files: the scripts resolve their own location
// through import.meta.url, which resolves a symlink to its REAL path. That is
// exactly what we want — the script's own __dirname is the checkout it lives
// in, while the DATA it reads and writes comes from CAREER_OPS_ROOT, which
// points at the fixture. A script run through the fixture therefore reads and
// writes the fixture's tracker and never the user's.
import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { fileURLToPath } from "node:url";

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

// Copied, not linked: these hold the user's own content, and a write through
// the fixture must never reach the original.
const COPIED_DIRS = [
  "data",
  "reports",
  "output",
  "jds",
  "interview-prep",
  "documents",
  "writing-samples",
  "config",
  "modes",
  "templates",
  "fonts",
  "evals",
];

// Never copied: machine-local state and build trees.
const SKIPPED = new Set([".git", "node_modules", "target", ".tmp-script-test-", ".career-ops-locks", ".career-ops-web"]);

/**
 * @param {string} source
 * @param {string} target
 */
function copyTree(source, target) {
  fs.cpSync(source, target, { recursive: true, dereference: true });
}

/**
 * @param {string} target
 * @param {string} link
 */
function link(target, link) {
  fs.symlinkSync(path.resolve(target), link, "dir");
}

/**
 * Build the fixture.
 *
 * @param {string} target - directory to create.
 * @param {{source?: string, quiet?: boolean}} [options]
 * @returns {{target: string, source: string, copied: string[], linked: string[]}}
 */
export function makeFixture(target, options = {}) {
  const source = options.source ?? REPO;
  fs.mkdirSync(target, { recursive: true });

  const copied = [];
  const linked = [];

  for (const entry of fs.readdirSync(source)) {
    if (SKIPPED.has(entry)) continue;
    const from = path.join(source, entry);
    const to = path.join(target, entry);
    if (!fs.existsSync(to)) {
      if (COPIED_DIRS.includes(entry) && fs.statSync(from).isDirectory()) {
        copyTree(from, to);
        copied.push(entry);
        continue;
      }
      // Files and everything else: a symlink back to the checkout.
      fs.symlinkSync(from, to, fs.statSync(from).isDirectory() ? "dir" : "file");
      linked.push(entry);
    }
  }

  // The copied config/ carries the user's profile.yml; the shipped example is
  // a system file and stays a link so it never drifts from the checkout.
  const configDir = path.join(target, "config");
  for (const name of ["profile.example.yml", "local-paths.example.txt", "cv-facts.example.json"]) {
    const from = path.join(source, "config", name);
    const to = path.join(configDir, name);
    if (fs.existsSync(from) && !fs.existsSync(to)) {
      fs.symlinkSync(from, to, "file");
      linked.push(`config/${name}`);
    }
  }

  // The core scripts resolve their own location through import.meta.url, which
  // resolves a symlink to its REAL path. The fixture's root scripts are
  // therefore links to the checkout's, and their __dirname is the checkout —
  // which is why CAREER_OPS_ROOT (not the script's own directory) decides
  // which tracker they read and write.
  return { target, source, copied, linked };
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  const target = process.argv[2] ?? path.join(os.tmpdir(), "career-ops-fixture");
  const result = makeFixture(target);
  if (!process.env.CAREER_OPS_FIXTURE_QUIET) {
    console.log(`fixture: ${result.target}`);
    console.log(`  copied user files: ${result.copied.join(", ")}`);
    console.log(`  linked system files: ${result.linked.length}`);
  }
}
