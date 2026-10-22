/**
 * desktop-run-e2e.test.mjs — a whole evaluation, end to end, through the
 * desktop bridge.
 *
 * This is the test that answers the Goal's hardest requirement in one run: an
 * evaluation started by the desktop client must go through the SAME spawn,
 * CLI-fencing, envelope and persist path the web app uses, and the tracker row
 * must appear without the bridge ever writing it.
 *
 * No AI is involved. A stub `claude` binary is placed on PATH: it speaks the
 * real `--output-format stream-json` protocol and emits a report inside the
 * real `<<report-md>>` envelope. Everything after the CLI's stdout is the
 * production path, unchanged:
 *
 *   bridge op `run`
 *     → resolveCliOrFallback("claude")          web/src/lib/clis.ts
 *     → capabilitiesFor("evaluate")             worker-capabilities.mjs
 *     → buildPrompt(...)                        run-prompts.mjs
 *     → claudeCliArgs(...) + fenceArgs(...)     claude-invocation.mjs, cli-fencing.mjs
 *     → spawnHeadlessCli(...)                   spawn-cli.mjs
 *     → createReportEnvelopeFilter()            report-envelope.mjs
 *     → evaluateRunOutcome()                    report-persist.mjs
 *     → persistEvaluation()                     report-persist.mjs
 *         → reserve-report-num.mjs
 *         → persist-evaluation-report.mjs
 *         → merge-tracker.mjs
 *         → reserve-report-num.mjs --release
 *
 * The assertions are on the FILES: the report exists with the persisted body,
 * the tracker has a new row with the canonical header and the parsed score, and
 * the reserved number was released.
 */
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { pathToFileURL } from "node:url";

import { BRIDGE_SERVER, CLIENT_ROOT, REPO_ROOT, fixtureEnv, makeFixture } from "./helpers/desktop-fixture.mjs";

const SERVER = BRIDGE_SERVER;

/** The report body the stub agent emits, inside the real envelope markers. */
const REPORT_BODY = `# Evaluation: Stubco — Staff Platform Engineer

**Date:** 2026-10-21
**URL:** https://stubco.example/job/77
**Score:** 4.1/5
**Legitimacy:** High Confidence

---

## Block A — Role fit

A strong match for the platform work.

## Machine Summary

\`\`\`yaml
company: Stubco
role: Staff Platform Engineer
score: 4.1
via: null
\`\`\`

## Verdict

VERDICT: 4.1/5 — apply
`;

/**
 * Write a `claude` stub that speaks the real stream-json protocol.
 *
 * The envelope markers own their own lines in the accumulated text, which is
 * what report-envelope.mjs requires: `<<report-md>>` on its own line, exactly
 * once, and the same for the closer.
 *
 * @param {string} dir
 */
function installStubClaude(dir) {
  const script = `#!/usr/bin/env node
// A stub agent: the protocol is real, the thinking is not.
const body = ${JSON.stringify(REPORT_BODY)};
const emit = (obj) => process.stdout.write(JSON.stringify(obj) + "\\n");
emit({ type: "system", subtype: "init" });
emit({ type: "stream_event", event: { type: "content_block_start", content_block: { type: "tool_use", name: "WebFetch" } } });
emit({ type: "stream_event", event: { type: "content_block_delta", delta: { text: "Reading the posting and scoring it.\\n" } } });
// One delta per line, so the filter sees the markers line-anchored.
for (const line of body.split("\\n<<report-md>>")[0] ? [] : []) { void line; }
emit({ type: "stream_event", event: { type: "content_block_delta", delta: { text: "<<report-md>>\\n" + body + "<</report-md>>\\n" } } });
emit({ type: "result", usage: { input_tokens: 1200, output_tokens: 800 }, total_cost_usd: 0.0123 });
`;
  const file = path.join(dir, "claude");
  fs.writeFileSync(file, script, { mode: 0o755 });
  return dir;
}

/**
 * Start the bridge against a fixture, with the stub CLI first on PATH.
 *
 * @param {string} root
 * @param {string} stubDir
 */
function startBridge(root, stubDir) {
  const child = spawn(process.execPath, [SERVER], {
    cwd: REPO_ROOT,
    env: {
      ...fixtureEnv(root),
      PATH: `${stubDir}${path.delimiter}${process.env.PATH ?? ""}`,
    },
    stdio: ["pipe", "pipe", "pipe"],
  });

  let buffer = "";
  const pending = new Map();
  const events = [];
  let stderr = "";

  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    buffer += chunk;
    let newline;
    while ((newline = buffer.indexOf("\n")) !== -1) {
      const line = buffer.slice(0, newline);
      buffer = buffer.slice(newline + 1);
      if (!line.trim()) continue;
      const message = JSON.parse(line);
      if (message.event) {
        events.push({ id: message.id, event: message.event });
        continue;
      }
      const waiter = pending.get(message.id);
      if (waiter) {
        pending.delete(message.id);
        waiter(message);
      }
    }
  });
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => {
    stderr += chunk;
  });

  // A bridge that dies must fail the pending call, not leave the test hanging:
  // node:test reports a hang as a cancel, which hides the real reason.
  let nextId = 1;
  const call = (op, params = {}) =>
    new Promise((resolve, reject) => {
      const id = nextId++;
      pending.set(id, (message) => {
        clearTimeout(timer);
        resolve(message);
      });
      // Deliberately NOT unref'd: an unref'd timer lets the event loop drain
      // while this promise is still pending, and node:test reads that as a
      // cancel rather than as a timeout with a reason.
      const timer = setTimeout(() => {
        if (pending.delete(id)) reject(new Error(`bridge did not answer ${op} in time\n${stderr.slice(-3000)}`));
      }, 60_000);
      child.stdin.write(`${JSON.stringify({ id, op, params })}\n`);
    });

  child.on("exit", (code, signal) => {
    for (const [, waiter] of pending) {
      waiter({ ok: false, error: { code: "bridge-exited", message: `the bridge exited (code ${code}, signal ${signal})\n${stderr.slice(-3000)}` } });
    }
    pending.clear();
  });

  return { call, events, stderr: () => stderr, stop: () => child.kill() };
}

test("an evaluation started by the desktop client is persisted through the canonical scripts", async (t) => {
  const fixture = makeFixture();
  const stubDir = fs.mkdtempSync(path.join(fixture.root, "-stub-"));
  installStubClaude(stubDir);
  const bridge = startBridge(fixture.root, stubDir);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  // The CLI must be visible to the bridge, not just to this process.
  const boot = await bridge.call("bootstrap");
  const claude = boot.data.clis.find((c) => c.id === "claude");
  assert.ok(claude, "the stub claude is not on PATH — the test cannot run");
  assert.equal(claude.installed, true);

  const before = fs.readFileSync(path.join(fixture.root, "data", "applications.md"), "utf-8");
  const reportsBefore = fs.readdirSync(path.join(fixture.root, "reports"));

  const result = await bridge.call("run", {
    kind: "evaluate",
    input: "https://stubco.example/job/77",
    cliId: "claude",
  });

  assert.equal(result.ok, true, `run failed: ${JSON.stringify(result.error)}\n${bridge.stderr().slice(-3000)}`);

  const events = bridge.events.map((entry) => entry.event);
  assert.ok(events.length > 0, "the run produced no streamed events");

  // The same event vocabulary the web app streams, in the same order:
  // status (fencing) → text/tool → status (saving) → done.
  const types = events.map((event) => event.type);
  assert.ok(types.includes("tool"), "the tool event from the CLI's stream never reached the client");
  assert.ok(types.includes("text"), "the agent's text never reached the client");
  assert.ok(
    types.includes("status") && events.some((e) => e.type === "status" && /Saving report/i.test(e.label)),
    "the persist phase was not announced",
  );
  const done = events.at(-1);
  assert.equal(done.type, "done", "the run did not end with a done event");
  assert.ok(done.reportN, "the done event carries no report number");
  // The CLI's own `system/init` frame reached the client as a status: the
  // stream was parsed by run-cli-support.mjs, not by this test's stub path.
  // (A fully fenced CLI reports no fencing NOTICE — the notice exists for the
  // runtimes with no mechanism, and claude is not one of them; that split is
  // pinned by tests/desktop-engine-parity.test.mjs.)
  assert.ok(
    events.some((e) => e.type === "status" && typeof e.label === "string" && e.label.length > 0),
    "no status event from the CLI's stream reached the client",
  );

  // ── The report file ──────────────────────────────────────────────────────
  const reportN = String(done.reportN);
  const reportsDir = path.join(fixture.root, "reports");
  const reportsAfter = fs.readdirSync(reportsDir);
  const newReports = reportsAfter.filter((name) => !reportsBefore.includes(name));
  assert.equal(newReports.length, 1, `expected exactly one new report, got ${newReports.join(", ")}`);
  const reportFile = newReports[0];
  // The file name is zero-padded (`003-…`) while the event carries the number
  // the client opens it by (`3`), which is the same pair the web route sends.
  assert.equal(
    Number.parseInt(reportFile, 10),
    Number(reportN),
    `the report is not numbered ${reportN}: ${reportFile}`,
  );
  const reportText = fs.readFileSync(path.join(reportsDir, reportFile), "utf-8");
  assert.ok(reportText.startsWith("# Evaluation: Stubco"), "the persisted report lost its title");
  assert.ok(reportText.includes("VERDICT: 4.1/5"), "the persisted report lost its verdict");
  assert.ok(
    !reportText.includes("<<report-md>>"),
    "the envelope markers leaked into the persisted report",
  );

  // ── The tracker row, written by merge-tracker.mjs ────────────────────────
  const after = fs.readFileSync(path.join(fixture.root, "data", "applications.md"), "utf-8");
  const added = after.split("\n").filter((line) => line.includes("Stubco"));
  assert.equal(added.length, 1, `expected one new tracker row for Stubco, found ${added.length}:\n${added.join("\n")}`);
  const row = added[0];
  assert.ok(row.includes("Staff Platform Engineer"), "the tracker row lost the role");
  assert.ok(row.includes("4.1/5"), "the tracker row lost the parsed score");
  assert.ok(row.includes("Evaluated"), "a new evaluation row must start in Evaluated");
  assert.ok(row.includes(reportFile), "the tracker row does not link the report it came from");
  // The pre-existing rows are untouched.
  assert.ok(after.includes("Acme"), "the merge clobbered the existing rows");

  // ── The report number was released ───────────────────────────────────────
  const sentinels = fs
    .readdirSync(reportsDir)
    .filter((name) => /-RESERVED\.md$/.test(name));
  assert.deepEqual(sentinels, [], `the reserved report number was never released: ${sentinels.join(", ")}`);

  assert.notEqual(after, before, "the tracker was not written at all");
});

// ─── the other run kinds ───────────────────────────────────────────────────

test("the cover run produces a draft letter in the run log", async (t) => {
  const fixture = makeFixture();
  const stubDir = fs.mkdtempSync(path.join(fixture.root, "-stub-"));
  installStubClaude(stubDir);
  const bridge = startBridge(fixture.root, stubDir);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const result = await bridge.call("run", {
    kind: "cover",
    input: "1",
    cliId: "claude",
    brief: JSON.stringify({
      why: "the role matches my background",
      problem: "the team needs someone on the estate",
      approach: "I would start with the estate inventory",
      tone: "formal",
    }),
  });
  assert.equal(result.ok, true, `the cover run must go through: ${JSON.stringify(result.error)}`);
  const text = bridge.events
    .map((entry) => entry.event)
    .filter((event) => event.type === "text")
    .map((event) => event.text)
    .join("\n");
  assert.match(text, /Evaluation: Stubco/, "the agent's reply must reach the run log");
  // Nothing is persisted by a cover run: the letter is a draft.
  const reports = fs.readdirSync(path.join(fixture.root, "reports"));
  assert.equal(reports.length, 1, "a cover run must not write a report");
});

test("the pdf run leaves the tracker PDF column alone when the render does not happen", async (t) => {
  const fixture = makeFixture();
  const stubDir = fs.mkdtempSync(path.join(fixture.root, "-stub-"));
  installStubClaude(stubDir);
  const bridge = startBridge(fixture.root, stubDir);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const before = fs.readFileSync(path.join(fixture.root, "data", "applications.md"), "utf-8");

  // The stub emits a REPORT envelope, not a CV one, so the pdf run never gets
  // to a render. The tracker must be untouched: the PDF column says "there is
  // a PDF", and there is not one yet.
  const result = await bridge.call("run", {
    kind: "pdf",
    input: "1",
    cliId: "claude",
  });
  const after = fs.readFileSync(path.join(fixture.root, "data", "applications.md"), "utf-8");
  assert.equal(after, before, "the tracker must be byte-identical after a pdf run that never rendered");
  assert.ok(
    !after.includes("✅"),
    "the PDF column must not be marked without a rendered PDF",
  );
});

test("a report for a scanned URL carries the recorded posting date", async (t) => {
  const fixture = makeFixture();
  // The scan history is what carries the date a posting was first seen.
  fs.writeFileSync(
    path.join(fixture.root, "data", "scan-history.tsv"),
    "url\tfirst_seen\tportal\ttitle\tcompany\tstatus\tlocation\nhttps://stubco.example/job/77\t2026-09-30\tgreenhouse\tStaff Platform Engineer\tStubco\tadded\tRemote\n",
    "utf-8",
  );
  const stubDir = fs.mkdtempSync(path.join(fixture.root, "-stub-"));
  installStubClaude(stubDir);
  const bridge = startBridge(fixture.root, stubDir);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const result = await bridge.call("run", {
    kind: "evaluate",
    input: "https://stubco.example/job/77",
    cliId: "claude",
  });
  assert.equal(result.ok, true, `the run must go through: ${JSON.stringify(result.error)}`);
  // The date the scan recorded travels with the TRACKER ROW, as the core's
  // own notes column carries it.
  const tracker = fs.readFileSync(path.join(fixture.root, "data", "applications.md"), "utf-8");
  const row = tracker.split("\n").find((line) => line.includes("Stubco"));
  assert.ok(row, "the row must be in the tracker");
  assert.match(
    row,
    /posted: 2026-09-30/,
    "the row must carry the date the scan recorded",
  );
});

test("a CLI that cannot be fenced says so, and is refused a write-capable worker", async (t) => {
  // The engine's own fencing rules, imported from the career-ops checkout.
  const { fencingReport, isCliAllowedForCapabilities } = await import(
    pathToFileURL(path.join(REPO_ROOT, "web", "src", "lib", "cli-fencing.mjs")).href
  );

  const fencing = fencingReport({
    cliId: "hermes",
    cliName: "Hermes Agent",
    capabilities: { writes: false, network: "fetch" },
  });
  assert.ok(
    fencing.notice && /cannot be permission-restricted/i.test(fencing.notice),
    `the engine must announce that hermes cannot be permission-restricted: ${JSON.stringify(fencing)}`,
  );

  assert.equal(
    isCliAllowedForCapabilities("hermes", { writes: true, network: "fetch" }),
    false,
    "a write-capable worker must be refused for a CLI that cannot be fenced",
  );
  assert.equal(
    isCliAllowedForCapabilities("claude", { writes: true, network: "fetch" }),
    true,
    "a CLI that can be fenced must be allowed",
  );

  // And the bridge asks BEFORE it spawns, and says the notice in the log.
  const source = fs.readFileSync(
    path.join(CLIENT_ROOT, "bridge", "ops", "run.mjs"),
    "utf-8",
  );
  assert.match(
    source,
    /if \(!isCliAllowedForCapabilities\(cliId, capabilities\)\)/,
    "the run op must refuse a worker the CLI cannot be granted",
  );
  assert.match(
    source,
    /if \(fencing\.notice\) send\(\{ type: "status", label: fencing\.notice \}\)/,
    "the run op must announce the fencing notice",
  );
});
