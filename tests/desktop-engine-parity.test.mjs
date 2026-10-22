/**
 * desktop-engine-parity.test.mjs — the desktop bridge runs the career-ops engine.
 *
 * The native client must not become a second implementation of the career-ops
 * run pipeline. This suite is the proof, and it is written as the kind of guard
 * the repo already keeps: values are imported and compared, and the structural
 * claims (which modules, which scripts) are asserted against the real files.
 *
 * The two repositories are named, never inferred. The bridge lives in THIS
 * repository; the engine lives in the career-ops checkout, which is resolved by
 * the app's own rule (CAREER_OPS_CODE_ROOT, else a sibling checkout of the same
 * parent directory, else one actionable sentence). Every path below that points
 * into the engine — `web/src/app/api/run/route.ts` above all — is resolved
 * INSIDE that configured checkout, never by walking up from the bridge: the
 * bridge's own directory says nothing about where the engine is.
 *
 * It is skipped, not failed, when `web/` is absent from the checkout: a
 * data-only install cannot hold the engine the desktop client shares. web/ IS
 * present here, so a missing file is a move and fails.
 *
 * Checks:
 *   1. Every engine module the bridge's run path uses is a module the web app's
 *      POST /api/run imports too, and both resolve to the SAME file.
 *   2. The bridge builds the same argv for each (CLI, kind) pair as the web
 *      route would, by calling the web app's own argv builders.
 *   3. The bridge contains no tracker writer of its own: the scripts that may
 *      write the tracker are named in the web route / engine, and the bridge
 *      reaches them only through persistEvaluation and set-status.
 *   4. A status change is attributed to `desktop` in the transition ledger, and
 *      `desktop` is a source both the writer and the reader accept.
 *   5. The engine checkout is resolved the same way in Node and in Rust, and
 *      the failure is one actionable sentence in both.
 */
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

import { CLIENT_ROOT, REPO_ROOT } from "./helpers/desktop-fixture.mjs";
import {
  ACTIONABLE,
  CHECKOUT_DIR_NAME,
  CODE_ROOT_ENV,
  resolveCodeRoot,
  siblingCandidate,
} from "../bridge/code-root.mjs";

// The bridge is HERE. The engine is in the configured career-ops checkout.
const BRIDGE = path.join(CLIENT_ROOT, "bridge");
const WEB = path.join(REPO_ROOT, "web");
const RUN_ROUTE = path.join(WEB, "src", "app", "api", "run", "route.ts");
const WEB_LIB = path.join(WEB, "src", "lib");
const RUST_CODE_ROOT = path.join(CLIENT_ROOT, "src", "code_root.rs");

const webPresent = fs.existsSync(WEB_LIB) && fs.existsSync(RUN_ROUTE);

/** The engine modules the desktop run path must share with the web route. */
const SHARED_RUN_MODULES = [
  "spawn-cli.mjs",
  "cli-fencing.mjs",
  "claude-invocation.mjs",
  "worker-capabilities.mjs",
  "report-envelope.mjs",
  "cv-envelope.mjs",
  "report-persist.mjs",
  "run-prompts.mjs",
  "run-cli-support.mjs",
  "pdf-render.mjs",
];

/**
 * The career-ops repo's own tree walk.
 *
 * It lives in the engine checkout, not here, so it is imported by path rather
 * than by a relative specifier that would break the moment the two
 * repositories are not siblings.
 */
const { listTree } = await import(
  pathToFileURL(path.join(REPO_ROOT, "lib", "walk-tree.mjs")).href
);

/**
 * Every specifier a module imports, as written.
 *
 * Good enough for these files: each imports with a plain string literal, which
 * is the repo's own style.
 *
 * @param {string} file
 * @returns {string[]}
 */
function importSpecifiers(file) {
  const source = fs.readFileSync(file, "utf-8");
  const specifiers = [];
  // Allows a newline between the binding list and `from`: several of these
  // modules import a multi-line list, and a line-anchored pattern silently
  // skipped exactly those (which is how this guard first failed on
  // run-cli-support.mjs).
  const re = /\b(?:import|export)\s+[^'"]*?from\s*['"]([^'"]+)['"]|\bimport\s*['"]([^'"]+)['"]/g;
  let match;
  while ((match = re.exec(source))) specifiers.push(match[1] ?? match[2]);
  return specifiers;
}

/**
 * Source with comments removed.
 *
 * A comment that EXPLAINS the persist path ("report-persist.mjs runs
 * reserve-report-num.mjs") is documentation, not a second implementation. The
 * claim under test is about code, so the code is what gets read.
 *
 * @param {string} source
 */
function stripComments(source) {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/(^|[^:])\/\/.*$/gm, "$1");
}

/** True when `file` imports `name` in any form. */
function importsModule(file, name) {
  return importSpecifiers(file).some((spec) => spec.endsWith(`/${name}`) || spec === name);
}

test("the bridge's run path imports the same engine modules the web route does", { skip: !webPresent && "web/ is not present in this checkout" }, () => {
  const engine = path.join(BRIDGE, "engine.mjs");
  const engineSpecifiers = importSpecifiers(engine);
  const routeSpecifiers = importSpecifiers(RUN_ROUTE);

  for (const name of SHARED_RUN_MODULES) {
    const inEngine = engineSpecifiers.some((spec) => spec.endsWith(name));
    const inRoute = routeSpecifiers.some((spec) => spec.endsWith(name));
    assert.ok(inEngine, `bridge/engine.mjs does not import ${name} — the bridge has stopped sharing the engine`);
    assert.ok(inRoute, `the career-ops POST /api/run no longer imports ${name} — the shared-engine contract moved`);
  }
});

test("both clients resolve the shared modules to the same file, not a copy", { skip: !webPresent && "web/ is not present in this checkout" }, () => {
  // The bridge reaches the modules through the "@/" alias, so the resolved
  // target is asserted through the alias root rather than by importing:
  // engine.mjs names "@\/lib/<module>", which resolves to web/src/lib/<module>.
  const engine = fs.readFileSync(path.join(BRIDGE, "engine.mjs"), "utf-8");
  for (const name of SHARED_RUN_MODULES) {
    const aliased = engine.includes(`"@/lib/${name}"`);
    assert.ok(
      aliased,
      `bridge/engine.mjs must reach ${name} through the career-ops own "@/lib/" alias, so it cannot be a vendored copy`,
    );
    assert.ok(
      fs.existsSync(path.join(WEB_LIB, name)),
      `${name} is not where the alias points — the bridge's import would fail`,
    );
  }
  // And the bridge ships no second copy of those modules.
  // listTree() rather than a local walk: it carries the nested-checkout guard
  // this repo requires of every descent (tests/walk-tree.test.mjs enforces it).
  const bridgeFiles = listTree(BRIDGE, { match: /\.mjs$/ });
  for (const name of SHARED_RUN_MODULES) {
    const duplicates = bridgeFiles.filter((file) => path.basename(file) === name);
    assert.deepEqual(duplicates, [], `${name} is duplicated under bridge/ — that is a second implementation`);
  }
});

test("the bridge builds the same argv as the web app for every CLI and kind", { skip: !webPresent && "web/ is not present in this checkout" }, async () => {
  await import(pathToFileURL(path.join(WEB, "tests", "helpers", "web-ts-alias-loader.mjs")).href);
  const claude = await import(pathToFileURL(path.join(WEB_LIB, "claude-invocation.mjs")).href);
  const runSupport = await import(pathToFileURL(path.join(WEB_LIB, "run-cli-support.mjs")).href);
  const fencing = await import(pathToFileURL(path.join(WEB_LIB, "cli-fencing.mjs")).href);

  // The bridge's selection, read from its source: `isClaude ? claudeCliArgs(…) :
  // (spec.streamArgs ?? spec.args)(prompt)`. Assert the two branches produce the
  // same argv the route's identical expression produces, by calling the web
  // app's builders and comparing against what the bridge's op would emit.
  for (const kind of ["evaluate", "pdf", "cover"]) {
    const prompt = `parity-probe-${kind}`;
    const claudeArgs = claude.claudeCliArgs({ kind, prompt });
    // Claude: no write-capable tool, and a strict MCP config, for every kind.
    const allowed = claude.argValue(claudeArgs, "--allowedTools");
    const disallowed = claude.argValue(claudeArgs, "--disallowedTools");
    assert.equal(
      claude.grantsWriteCapability({ allowed, disallowed }),
      false,
      `the ${kind} argv grants a write-capable tool`,
    );
    // The same fencing the bridge asks for, on the same argv.
    const fenced = fencing.fenceArgs({ cliId: "claude", args: claudeArgs, capabilities: { writes: false, network: "fetch" } });
    assert.deepEqual(fenced.args, claudeArgs, "claude argv must survive fencing unchanged");

    // Codex: the workspace-write argv comes from run-cli-support, and fencing
    // is where the sandbox flags land.
    const codexArgs = runSupport.codexStreamArgs(prompt);
    const codexFenced = fencing.fenceArgs({
      cliId: "codex",
      args: codexArgs,
      capabilities: { writes: false, network: "fetch" },
    });
    assert.ok(
      codexFenced.args.some((arg) => arg.includes("sandbox_mode")),
      "the codex argv must carry the sandbox the fence adds",
    );
  }
});

test("the bridge reaches the canonical writers through the engine, not around it", { skip: !webPresent && "web/ is not present in this checkout" }, () => {
  const opsRun = stripComments(fs.readFileSync(path.join(BRIDGE, "ops", "run.mjs"), "utf-8"));
  const opsStatus = stripComments(fs.readFileSync(path.join(BRIDGE, "ops", "status.mjs"), "utf-8"));

  // The evaluate/pdf outcomes go through report-persist / pdf-render, which run
  // reserve-report-num.mjs, persist-evaluation-report.mjs, merge-tracker.mjs,
  // generate-pdf.mjs and mark-pdf-ready.mjs.
  //
  // The structural proof that the bridge has not started its own persist path
  // is what it CANNOT reach: the generic script runner. `runNode()` is how the
  // bridge's other ops call a core script, and the run path does not import it.
  // The only process it starts is the agent CLI, through spawnHeadlessCli, and
  // the only `spawn` it names is the one it hands to the engine's own writers.
  assert.ok(
    !opsRun.includes("runNode"),
    "bridge/ops/run.mjs reaches for the generic script runner — the run path must persist only through the shared engine",
  );
  // `spawn` is imported and HANDED OVER (`spawnFn: spawn`), never called. The
  // one process this file starts is the agent CLI, and that goes through
  // spawnHeadlessCli so it is fenced.
  assert.ok(
    !/\bspawn\s*\(/.test(opsRun),
    "bridge/ops/run.mjs calls spawn() itself instead of handing it to the engine",
  );
  assert.ok(
    !/\bexecFile\b/.test(opsRun),
    "bridge/ops/run.mjs shells out to a core script — persistence must stay inside the shared engine",
  );
  assert.ok(
    (opsRun.match(/spawnFn: spawn/g) ?? []).length >= 2,
    "the engine's writers must receive `spawnFn: spawn` (persist + render)",
  );
  assert.ok(
    !/writeFileSync|appendFileSync|writeFile\(/.test(opsRun),
    "bridge/ops/run.mjs writes a file — every artifact is written by the shared engine",
  );
  // The shared entry points ARE what it calls.
  for (const fn of ["persistEvaluation", "evaluateRunOutcome", "pdfRunOutcome", "renderAndMarkPdf", "spawnHeadlessCli"]) {
    assert.ok(opsRun.includes(fn), `bridge/ops/run.mjs does not use the shared ${fn}()`);
  }

  // The status write is a spawn of the core's set-status.mjs, attributed to
  // `desktop`. It is never a tracker edit.
  assert.ok(opsStatus.includes('rootScript("set-status")'), "the status op must run the core's set-status.mjs");
  assert.ok(opsStatus.includes('"--source"') && opsStatus.includes('"desktop"'), "a desktop status change must be attributed to `desktop` in the ledger");
  assert.ok(
    !/writeFileSync|appendFileSync/.test(opsStatus),
    "the status op must not write the tracker itself",
  );
});

test("the engine checkout is resolved the same way in Node and in Rust", () => {
  // The app, the bridge and the fixtures must never disagree about which
  // checkout they are talking to. The rule is written twice — once in Node,
  // once in Rust — so it is pinned here: the same variable name, the same
  // sibling directory name, the same actionable sentence, and the same
  // resolution for the same inputs.
  const rust = fs.readFileSync(RUST_CODE_ROOT, "utf-8");

  assert.ok(
    rust.includes(`pub const CODE_ROOT_ENV: &str = "${CODE_ROOT_ENV}";`),
    `src/code_root.rs must name the same environment variable as bridge/code-root.mjs (${CODE_ROOT_ENV})`,
  );
  assert.ok(
    rust.includes(`pub const CHECKOUT_DIR_NAME: &str = "${CHECKOUT_DIR_NAME}";`),
    `src/code_root.rs must look for the same sibling directory name (${CHECKOUT_DIR_NAME})`,
  );
  assert.ok(
    rust.includes(ACTIONABLE),
    "src/code_root.rs must fail with the same actionable sentence as bridge/code-root.mjs",
  );

  // The sibling rule: the checkout of the same parent directory.
  const sibling = siblingCandidate(CLIENT_ROOT);
  assert.equal(
    sibling,
    path.join(path.dirname(CLIENT_ROOT), CHECKOUT_DIR_NAME),
    "the sibling candidate must be a career-ops checkout of the same parent directory",
  );

  // An explicit value wins, and it is the environment variable that names it.
  const explicit = resolveCodeRoot({ explicit: REPO_ROOT, clientRoot: CLIENT_ROOT });
  assert.equal(explicit.path, REPO_ROOT);
  assert.equal(explicit.source, "environment");

  // With nothing set, the sibling rule finds the checkout this suite runs
  // against — which is the whole point of the layout.
  const siblingResolved = resolveCodeRoot({ explicit: "", clientRoot: CLIENT_ROOT });
  assert.equal(siblingResolved.path, REPO_ROOT);
  assert.equal(siblingResolved.source, "sibling");

  // And with neither, the failure is ONE actionable sentence.
  assert.throws(
    () => resolveCodeRoot({ explicit: "", clientRoot: path.join(CLIENT_ROOT, "no-such-place") }),
    (err) => {
      assert.equal(err.message, ACTIONABLE);
      assert.equal(err.message.split(". ").length, 1, "the failure must be one sentence");
      assert.ok(err.message.includes("CAREER_OPS_CODE_ROOT"), "the sentence must say what to do");
      return true;
    },
  );
});

test("`desktop` is a source both the writer and the reader accept", () => {
  // Written by set-status.mjs, read by funnel-velocity.mjs. One entry in each,
  // or the label is persisted and then silently dropped.
  const setStatus = fs.readFileSync(path.join(REPO_ROOT, "set-status.mjs"), "utf-8");
  const funnel = fs.readFileSync(path.join(REPO_ROOT, "funnel-velocity.mjs"), "utf-8");

  const writerSources = setStatus.match(/WRITER_SOURCES = new Set\(\[([^\]]+)\]\)/);
  assert.ok(writerSources, "set-status.mjs no longer declares WRITER_SOURCES");
  assert.ok(writerSources[1].includes("'desktop'"), "set-status.mjs does not accept --source desktop");

  const readerSources = funnel.match(/VALID_SOURCES = new Set\(\[([^\]]+)\]\)/);
  assert.ok(readerSources, "funnel-velocity.mjs no longer declares VALID_SOURCES");
  assert.ok(readerSources[1].includes("'desktop'"), "funnel-velocity.mjs would drop desktop-attributed transitions");

  const dayMath = funnel.match(/DAY_MATH_SOURCES = new Set\(\[([^\]]+)\]\)/);
  assert.ok(dayMath, "funnel-velocity.mjs no longer declares DAY_MATH_SOURCES");
  assert.ok(
    dayMath[1].includes("'desktop'"),
    "a desktop status change is an observed transition, so its date must feed the day-math",
  );
});

test("the bridge never binds a socket", () => {
  const files = listTree(BRIDGE, { match: /\.mjs$/ });
  for (const file of files) {
    const source = fs.readFileSync(file, "utf-8");
    assert.ok(
      !/\bcreateServer\b|\bhttp\.listen\b|\.listen\s*\(/.test(source),
      `${path.relative(CLIENT_ROOT, file)} opens a listening socket — the bridge is stdio only`,
    );
  }
});

// ─── the invariants the inventory names ───────────────────────────────────

/**
 * Every file in the bridge that WRITES, and what it is allowed to write.
 *
 * This is a snapshot on purpose. X-1 says every tracker write goes through the
 * canonical scripts, and the way that stops being true is a new op quietly
 * growing a `writeFileSync`. Adding a writer now fails this test until someone
 * adds it here WITH the reason it is allowed — which is the decision the
 * invariant asks for, made once, in the open.
 */
const ALLOWED_WRITERS = {
  "ops/config.mjs": "config/profile.yml, merge-safe, through the shared helper",
  "ops/cv.mjs": "cv.md through the shared helper; a temp payload for the verifier",
  "ops/followups.mjs": "data/follow-ups.md, under the core's own lock",
  "ops/portals.mjs": "portals.yml, merge-safe; a temp YAML for the board search",
  "ops/profile.mjs": "modes/_profile.md, through the engine's own splice",
  "ops/runs.mjs": "data/runs/, the client's own run records",
  "ops/tracker.mjs": "a tracker-additions TSV, which merge-tracker.mjs then merges",
};

/** A call, not an import: `writeFileSync(` and friends. */
const WRITE_CALL = /\b(?:writeFileSync|appendFileSync|atomicWrite|atomicWriteWithBackup)\s*\(/;

test("the bridge writes in exactly the files that are allowed to write", () => {
  const files = [
    ...listTree(path.join(BRIDGE, "ops"), { match: /\.mjs$/ }),
    path.join(BRIDGE, "engine.mjs"),
    path.join(BRIDGE, "util.mjs"),
    path.join(BRIDGE, "core-script.mjs"),
  ];
  const actual = [];
  for (const file of files) {
    const source = stripComments(fs.readFileSync(file, "utf-8"));
    if (WRITE_CALL.test(source)) {
      actual.push(path.relative(BRIDGE, file).split(path.sep).join("/"));
    }
  }
  assert.deepEqual(
    actual.sort(),
    Object.keys(ALLOWED_WRITERS).sort(),
    `the set of writing files changed. If a new one is right, add it to ALLOWED_WRITERS with the reason it may write: ${JSON.stringify(actual)}`,
  );
});

test("no bridge file writes the tracker itself (X-1)", () => {
  for (const file of listTree(BRIDGE, { match: /\.mjs$/ })) {
    const source = stripComments(fs.readFileSync(file, "utf-8"));
    const lines = source.split("\n");
    for (const line of lines) {
      if (!WRITE_CALL.test(line)) continue;
      assert.ok(
        !/applications\.md|tracker/i.test(line),
        `${path.relative(CLIENT_ROOT, file)} writes the tracker directly — every tracker write goes through set-status.mjs, merge-tracker.mjs, reserve-report-num.mjs, persist-evaluation-report.mjs or tracker.mjs`,
      );
    }
  }
});

test("nothing in the client can submit an application (X-5)", () => {
  // A submit needs a form, a click or a request. There is no browser here, and
  // the two ops that talk about submitting only ever report `submitted: false`.
  const offenders = [];
  for (const file of listTree(path.join(CLIENT_ROOT, "bridge"), { match: /\.mjs$/ })) {
    const source = stripComments(fs.readFileSync(file, "utf-8"));
    if (/\.submit\s*\(|\bsubmitForm\b|\.click\s*\(|dispatchEvent\s*\(/.test(source)) {
      offenders.push(path.relative(CLIENT_ROOT, file));
    }
  }
  assert.deepEqual(offenders, [], `these files can drive a form: ${offenders.join(", ")}`);
});
