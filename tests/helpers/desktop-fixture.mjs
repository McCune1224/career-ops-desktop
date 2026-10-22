// A minimal career-ops install, for the desktop bridge's tests.
//
// The bridge's scripts are spawned from the DATA root (`report-persist.mjs`
// builds `path.join(root, "reserve-report-num.mjs")`), and the core scripts
// resolve their own configuration from the checkout. A test fixture therefore
// needs both halves: real user files it may write to, and the core's own
// scripts within reach.
//
// The scripts are symlinked, not copied. A script resolves its own location
// through `import.meta.url`, which resolves a symlink to its real path, so the
// script's `__dirname` is the checkout while the data it reads and writes comes
// from `CAREER_OPS_ROOT` — the fixture. That is the same arrangement a real
// split install has, and it keeps a core-script edit visible to the fixture
// with no re-copy step.
//
// Nothing here reads the user's own data: every user file below is written by
// this module.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { resolveCodeRoot } from "../../bridge/code-root.mjs";

/** This repository's root: the client, its bridge and its tests. */
export const CLIENT_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
);

/**
 * The career-ops checkout the tests drive.
 *
 * Resolved by the app's own rule — CAREER_OPS_CODE_ROOT, else a sibling
 * checkout of the same parent directory, else one actionable sentence — so a
 * test can never end up exercising a different checkout than the app does.
 */
export const REPO_ROOT = resolveCodeRoot({ clientRoot: CLIENT_ROOT }).path;

/** The bridge entry point, in this repository. */
export const BRIDGE_SERVER = path.join(CLIENT_ROOT, "bridge", "server.mjs");

/** Directories of the checkout the core scripts need at reach. */
// `batch/` is deliberately NOT linked: persist-evaluation-report.mjs writes
// `batch/tracker-additions/` relative to the DATA root, so a link would put a
// test's TSV inside the real checkout. The fixture gets a real directory.
const LINKED_DIRS = ["lib", "templates", "providers", "utils", "scripts", "seeds", "fonts", "examples"];

// The evaluation and cover modes the run screens require to exist. They are
// system files, so they are linked; `modes/_profile.md` is user data and is
// written below.
const LINKED_MODES = ["oferta.md", "cover.md", "pdf.md"];

/** A tracker with two rows, one of them a SKIP, so filters have something to do. */
const TRACKER = `# Applications Tracker

| # | Date | Company | Role | Score | Status | PDF | Report | Notes | URL |
|---|------|---------|------|-------|--------|-----|--------|-------|-----|
| 1 | 2026-01-02 | Acme | Platform Engineer | 4.2/5 | Evaluated | ❌ | [001](../reports/001-acme-2026-01-02.md) | good fit | https://acme.example/job/1 |
| 2 | 2026-01-03 | Beta | SRE | 2.1/5 | SKIP | ❌ | — | too senior | — |
`;

// The inbox grammar pipeline-table.mjs parses:
//   - [ ] {url} | {company} | {role} | {location} | posted: {date}
const PIPELINE = `# Pipeline — Pending URLs

## Pending

- [ ] https://gamma.example/job/9 | Gamma | Site Reliability Engineer | Remote | posted: 2026-01-04
- [x] https://delta.example/job/10 | Delta | Platform Engineer | Remote
`;

const REPORT = `# Evaluation: Acme — Platform Engineer

**Date:** 2026-01-02
**URL:** https://acme.example/job/1
**Score:** 4.2/5
**Legitimacy:** High Confidence

---

## Block A — Role fit

A solid match.

## Keywords extracted

platform engineer, python, kubernetes, terraform

## Machine Summary

\`\`\`yaml
final_decision: apply
top_strengths:
  - Python
hard_stops: []
soft_gaps:
  - Kubernetes
requirement_importance:
  - requirement: Run an estate
    importance: critical
    match: strong
confidence_gaps: []
\`\`\`

## Job Description (archived verbatim)

We are hiring a platform engineer to run our estate.

VERDICT: 4.2/5 — apply
`;

const PROFILE = `candidate:
  full_name: Test Candidate
  email: test@example.com
  location: Remote
target_roles:
  primary:
    - Platform Engineer
`;

/**
 * Build a fresh fixture in a temp directory.
 *
 * @param {{checkout?: string}} [options]
 * @returns {{root: string, cleanup: () => void}}
 */
export function makeFixture(options = {}) {
  const checkout = options.checkout ?? REPO_ROOT;
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "career-ops-desktop-fixture-"));

  // The core's own scripts, reachable from the data root.
  for (const entry of fs.readdirSync(checkout)) {
    if (!entry.endsWith(".mjs") && entry !== "tracker-aliases.json" && entry !== "VERSION") continue;
    fs.symlinkSync(path.join(checkout, entry), path.join(root, entry), "file");
  }
  for (const dir of LINKED_DIRS) {
    const from = path.join(checkout, dir);
    if (fs.existsSync(from)) fs.symlinkSync(from, path.join(root, dir), "dir");
  }

  // The user's own files — real files, because the bridge writes to them.
  fs.mkdirSync(path.join(root, "data"), { recursive: true });
  fs.mkdirSync(path.join(root, "reports"), { recursive: true });
  fs.mkdirSync(path.join(root, "output"), { recursive: true });
  fs.mkdirSync(path.join(root, "config"), { recursive: true });
  fs.mkdirSync(path.join(root, "modes"), { recursive: true });
  fs.mkdirSync(path.join(root, "batch", "tracker-additions"), { recursive: true });
  for (const mode of LINKED_MODES) {
    const from = path.join(checkout, "modes", mode);
    if (fs.existsSync(from)) fs.symlinkSync(from, path.join(root, "modes", mode), "file");
  }

  fs.writeFileSync(path.join(root, "data/applications.md"), TRACKER);
  fs.writeFileSync(path.join(root, "data/pipeline.md"), PIPELINE);
  fs.writeFileSync(path.join(root, "data/follow-ups.md"), "# Follow-ups\n\n");
  fs.writeFileSync(
    path.join(root, "data/pdf-index.tsv"),
    `1\t${path.join(root, "output", "cv-test-acme-001.pdf")}\t\tpdf\t2026-01-02\tcv\n`,
    "utf-8",
  );
  fs.writeFileSync(path.join(root, "reports/001-acme-2026-01-02.md"), REPORT);
  fs.writeFileSync(path.join(root, "cv.md"), "# Test Candidate\n\n## Experience\n\n### Acme — Remote\n\n**Platform Engineer**\n\n01/2024 – Present\n\n- Ran things.\n");
  fs.writeFileSync(path.join(root, "config/profile.yml"), PROFILE);
  fs.copyFileSync(
    path.join(checkout, "config/profile.example.yml"),
    path.join(root, "config/profile.example.yml"),
  );
  fs.copyFileSync(path.join(checkout, "modes/_profile.template.md"), path.join(root, "modes/_profile.md"));

  return {
    root,
    cleanup: () => fs.rmSync(root, { recursive: true, force: true }),
  };
}

/**
 * Environment for any process that must read and write the fixture.
 *
 * CAREER_OPS_CODE_ROOT points at the checkout so the core's scripts come from
 * there; CAREER_OPS_ROOT points at the fixture so every file they read and
 * write is the fixture's.
 *
 * @param {string} root
 * @param {string} [checkout]
 */
export function fixtureEnv(root, checkout = REPO_ROOT) {
  return {
    ...process.env,
    CAREER_OPS_ROOT: root,
    CAREER_OPS_CODE_ROOT: checkout,
  };
}
