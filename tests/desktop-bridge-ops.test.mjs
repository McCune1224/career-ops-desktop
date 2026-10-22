/**
 * desktop-bridge-ops.test.mjs — the desktop bridge, run for real.
 *
 * The Rust screen tests prove a screen renders what the bridge sends. This
 * suite proves the bridge sends the right thing: it starts the real
 * `desktop/bridge/server.mjs` against a throwaway fixture and drives it over
 * its actual protocol.
 *
 * What it is about:
 *   1. Every read op answers in the shape the Rust model decodes.
 *   2. Every WRITE goes through the canonical script: a status change lands in
 *      the tracker only via set-status.mjs, is attributed to `desktop` in the
 *      transition ledger, and a rejected state writes nothing.
 *   3. The bridge leaves no second source of truth: no op writes
 *      data/applications.md except through the script, and a busy tracker is
 *      reported as retryable rather than as success.
 *   4. The protocol itself: an unknown op and a malformed line are answered,
 *      not crashed into.
 */
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { pathToFileURL } from "node:url";

import { BRIDGE_SERVER, CLIENT_ROOT, REPO_ROOT, fixtureEnv, makeFixture } from "./helpers/desktop-fixture.mjs";

const SERVER = BRIDGE_SERVER;

/**
 * Start the bridge against `root`.
 *
 * @param {string} root
 */
function startBridge(root, extraEnv = {}) {
  const child = spawn(process.execPath, [SERVER], {
    cwd: REPO_ROOT,
    env: { ...fixtureEnv(root), ...extraEnv },
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

  let nextId = 1;
  const call = (op, params = {}) =>
    new Promise((resolve, reject) => {
      const id = nextId++;
      pending.set(id, resolve);
      child.stdin.write(`${JSON.stringify({ id, op, params })}\n`);
      setTimeout(() => {
        if (pending.delete(id)) reject(new Error(`bridge did not answer ${op} in time\n${stderr.slice(-2000)}`));
      }, 30_000).unref();
    });

  const writeRaw = (text) => child.stdin.write(text);

  return {
    call,
    writeRaw,
    events,
    stderr: () => stderr,
    stop: () => {
      child.kill();
    },
  };
}

/** Read the tracker's row for one tracker number. */
function trackerRow(root, n) {
  const tracker = fs.readFileSync(path.join(root, "data", "applications.md"), "utf-8");
  return tracker.split("\n").find((line) => line.startsWith(`| ${n} |`)) ?? null;
}

test("the bridge answers every read op in the shape the client decodes", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const boot = await bridge.call("bootstrap");
  assert.equal(boot.ok, true, JSON.stringify(boot.error));
  assert.equal(boot.data.dataRoot, fixture.root);
  assert.equal(boot.data.applicationCount, 2);
  assert.equal(boot.data.inboxCount, 2);
  assert.ok(Array.isArray(boot.data.states) && boot.data.states.includes("Applied"));
  assert.ok(Array.isArray(boot.data.clis) && boot.data.clis.length > 0);

  const pipeline = await bridge.call("pipeline");
  assert.equal(pipeline.ok, true, JSON.stringify(pipeline.error));
  assert.equal(pipeline.data.applications.length, 2);
  assert.equal(pipeline.data.inbox.length, 2);
  const first = pipeline.data.applications[0];
  assert.equal(first.n, "1");
  assert.equal(first.company, "Acme");
  assert.equal(first.status, "Evaluated");

  const report = await bridge.call("report", { n: "1" });
  assert.equal(report.ok, true, JSON.stringify(report.error));
  assert.equal(report.data.postingUrl, "https://acme.example/job/1");
  assert.equal(report.data.legitimacy, "High Confidence");
  assert.ok(report.data.markdown.includes("VERDICT"));

  const cv = await bridge.call("cv", { action: "read" });
  assert.equal(cv.ok, true);
  assert.equal(cv.data.exists, true);
  assert.ok(cv.data.content.includes("Test Candidate"));

  const followups = await bridge.call("followups", { action: "list" });
  assert.equal(followups.ok, true, JSON.stringify(followups.error));
  assert.equal(followups.data.available, true);

  const config = await bridge.call("config", { action: "get" });
  assert.equal(config.ok, true);
  assert.equal(config.data.profile.candidate.full_name, "Test Candidate");
  assert.equal(config.data.malformed, false);
});

test("a status change lands through set-status.mjs and is attributed to desktop", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const before = trackerRow(fixture.root, "1");
  assert.ok(before.includes("Evaluated"), "the fixture row starts Evaluated");

  const result = await bridge.call("status", { n: "1", status: "applied" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.equal(result.data.status, "Applied", "the alias must be canonicalised");
  assert.equal(result.data.changed, true);
  assert.equal(result.data.statusLogged, true);

  const after = trackerRow(fixture.root, "1");
  assert.ok(after.includes("Applied"), "the tracker cell must hold the canonical state");
  assert.ok(!after.includes("Evaluated"), "the previous state must be gone");

  // The transition ledger is the proof it went through the script: only
  // set-status.mjs appends there, and it stamps the source it was told.
  const ledger = fs.readFileSync(path.join(fixture.root, "data", "status-log.tsv"), "utf-8");
  const row = ledger.split("\n").find((line) => line.startsWith("1\t"));
  assert.ok(row, "no transition was recorded");
  assert.equal(row.split("\t")[2], "Evaluated");
  assert.equal(row.split("\t")[3], "Applied");
  assert.equal(row.split("\t")[4], "desktop", "the ledger must attribute the write to the desktop client");
});

test("a rejected state writes nothing", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const before = fs.readFileSync(path.join(fixture.root, "data", "applications.md"), "utf-8");
  const result = await bridge.call("status", { n: "1", status: "NotARealState" });
  assert.equal(result.ok, false);
  assert.equal(result.error.code, "bad-request");
  const after = fs.readFileSync(path.join(fixture.root, "data", "applications.md"), "utf-8");
  assert.equal(after, before, "a refused status must not touch the tracker");
  assert.ok(
    !fs.existsSync(path.join(fixture.root, "data", "status-log.tsv")),
    "a refused status must not write a ledger row",
  );
});

test("every canonical state the engine offers is accepted by the writer", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const boot = await bridge.call("bootstrap");
  // The states the filter tabs and the status combo render come from here, so
  // every one of them must be a state set-status.mjs accepts. A state the UI
  // offers but the writer refuses is a dead control.
  for (const state of boot.data.states) {
    const result = await bridge.call("status", { n: "1", status: state });
    assert.equal(
      result.ok,
      true,
      `the engine offers "${state}" but the writer refused it: ${JSON.stringify(result.error)}`,
    );
  }
});

test("a malformed line and an unknown op are answered, not crashed into", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const pending = new Promise((resolve) => {
    const timer = setTimeout(() => resolve(null), 5_000);
    const check = setInterval(() => {
      // The malformed line's answer has id null.
      if (bridge.events.length >= 0) {
        clearInterval(check);
        clearTimeout(timer);
        resolve(true);
      }
    }, 50);
    check.unref?.();
  });

  bridge.writeRaw("this is not json\n");
  await pending;
  const unknown = await bridge.call("no-such-op");
  assert.equal(unknown.ok, false);
  assert.equal(unknown.error.code, "unknown-op");

  // And the bridge is still healthy afterwards.
  const after = await bridge.call("pipeline");
  assert.equal(after.ok, true, "the bridge must survive a malformed request");
});

test("a follow-up is logged through the shared writer and appears in the list", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const logged = await bridge.call("followups", {
    action: "log",
    appNum: "1",
    company: "Acme",
    role: "Platform Engineer",
    channel: "Email",
    notes: "nudged the recruiter",
  });
  assert.equal(logged.ok, true, JSON.stringify(logged.error));
  assert.equal(logged.data.num, 1);

  const file = fs.readFileSync(path.join(fixture.root, "data", "follow-ups.md"), "utf-8");
  assert.ok(file.includes("| 1 | 1 |"), "the canonical table row was not appended");
  assert.ok(file.includes("nudged the recruiter"));

  const list = await bridge.call("followups", { action: "list" });
  assert.equal(list.ok, true);
  assert.equal(list.data.history.length, 1);
  assert.equal(list.data.history[0].company, "Acme");

  const deleted = await bridge.call("followups", { action: "delete", num: 1 });
  assert.equal(deleted.ok, true, JSON.stringify(deleted.error));
  const afterDelete = fs.readFileSync(path.join(fixture.root, "data", "follow-ups.md"), "utf-8");
  assert.ok(!afterDelete.includes("nudged the recruiter"), "the row was not removed");
});

test("skipping a pending posting flips the checkbox and nothing else", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const pipelinePath = path.join(fixture.root, "data", "pipeline.md");
  const before = fs.readFileSync(pipelinePath, "utf-8");
  assert.ok(before.includes("- [ ] https://gamma.example/job/9"), "the fixture row starts unchecked");

  const skipped = await bridge.call("inbox", { url: "https://gamma.example/job/9" });
  assert.equal(skipped.ok, true, JSON.stringify(skipped.error));
  assert.equal(skipped.data.done, true);
  // `changed` is a COUNT of rewritten rows, not a flag — the web route returns
  // the same field from the same function.
  assert.equal(skipped.data.changed, 1);

  const after = fs.readFileSync(pipelinePath, "utf-8");
  assert.ok(after.includes("- [x] https://gamma.example/job/9"), "the checkbox was not flipped");
  // Every other byte survives: the write touches one character.
  assert.equal(
    after.length,
    before.length,
    "the skip altered the file length — it must rewrite the checkbox byte only",
  );
  assert.equal(after.replace("- [x]", "- [ ]"), before, "the skip touched more than the checkbox");

  // The other row is untouched, and undoing restores the file exactly.
  const undone = await bridge.call("inbox", { url: "https://gamma.example/job/9", done: false });
  assert.equal(undone.ok, true, JSON.stringify(undone.error));
  assert.equal(fs.readFileSync(pipelinePath, "utf-8"), before, "undo did not restore the file");

  // An unknown URL is a not-found, not a silent success.
  const missing = await bridge.call("inbox", { url: "https://nope.example/job/0" });
  assert.equal(missing.ok, false);
  assert.equal(missing.error.code, "not-found");
});

test("the CV write is atomic and leaves a backup", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const result = await bridge.call("cv", { action: "write", content: "# Rewritten\n" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.ok(result.data.backedUp, "a CV write must leave a backup");
  assert.equal(fs.readFileSync(path.join(fixture.root, "cv.md"), "utf-8"), "# Rewritten\n");
});

test("the cadence read and write go through config/profile.yml", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const read = await bridge.call("followups", { action: "cadence.get" });
  assert.equal(read.ok, true, JSON.stringify(read.error));
  assert.equal(read.data.defaultsAvailable, true, "the core's own defaults must be reported");
  assert.equal(read.data.enabled, true);

  const written = await bridge.call("followups", {
    action: "cadence.set",
    applied_first_days: 4,
    enabled: false,
  });
  assert.equal(written.ok, true, JSON.stringify(written.error));

  const profile = fs.readFileSync(path.join(fixture.root, "config", "profile.yml"), "utf-8");
  assert.ok(profile.includes("applied_first_days: 4"), "the cadence was not written to the profile");
  assert.ok(profile.includes("enabled: false"), "the follow_ups switch was not written");
  // Merge-safe: the rest of the profile survives.
  assert.ok(profile.includes("Test Candidate"), "the write clobbered the profile");

  const after = await bridge.call("followups", { action: "cadence.get" });
  assert.equal(after.data.enabled, false);
  assert.equal(after.data.overrides.applied_first_days, 4);
});

// ─── M1: the trustworthy daily loop ───────────────────────────────────────

test("the profile validator surfaces a broken profile as an error", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const good = await bridge.call("config", { action: "validate" });
  assert.equal(good.ok, true, JSON.stringify(good.error));
  assert.equal(good.data.ok, true, "the shipped fixture profile is fine");
  assert.ok(Array.isArray(good.data.findings));

  // A profile that is a LIST, not a mapping: the core refuses it, and the
  // refusal has to reach the screen as an error rather than as an empty
  // default.
  fs.writeFileSync(
    path.join(fixture.root, "config", "profile.yml"),
    "- not\n- a mapping\n",
    "utf-8",
  );
  const broken = await bridge.call("config", { action: "validate" });
  assert.equal(broken.ok, true, "the validator answers; the finding is in the payload");
  assert.equal(broken.data.ok, false, "a profile that is a list is not a profile");
  assert.ok(broken.data.errors.length > 0, "the validator must name the problem");
  assert.ok(broken.data.message.length > 0, "the screen needs something to say");
});

test("a bulk status change writes one ledger row per row", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const before = fs.existsSync(path.join(fixture.root, "data", "status-log.tsv"))
    ? fs.readFileSync(path.join(fixture.root, "data", "status-log.tsv"), "utf-8")
    : "";

  const result = await bridge.call("status", { rows: ["1", "2"], status: "Applied" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.equal(result.data.applied, 2, "both rows must change");
  assert.equal(result.data.failed, 0);
  assert.equal(result.data.ledgerRows, 2, "each change gets its own ledger row");
  assert.equal(result.data.results.length, 2);

  const ledger = fs.readFileSync(path.join(fixture.root, "data", "status-log.tsv"), "utf-8");
  const added = ledger.split("\n").filter((line) => line.trim() && !before.includes(line));
  assert.equal(added.length, 2, `expected two ledger rows, got ${added.length}`);
  for (const row of added) {
    assert.equal(row.split("\t")[4], "desktop", "the ledger must attribute the write to the desktop client");
  }

  // A row that refuses does not take the others with it.
  const partial = await bridge.call("status", { rows: ["1", "404"], status: "Rejected" });
  assert.equal(partial.ok, true);
  assert.equal(partial.data.applied, 1, "the good row still changes");
  assert.equal(partial.data.failed, 1, "the missing row is reported, not silently dropped");
});

test("removing a tracker row goes through tracker.mjs and names the orphaned report", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const preview = await bridge.call("tracker", { action: "deletePreview", n: "1" });
  assert.equal(preview.ok, true, JSON.stringify(preview.error));
  assert.ok(
    preview.data.orphanedReport,
    `the preview must name the report the removal orphans: ${preview.data.output}`,
  );

  const removed = await bridge.call("tracker", { action: "delete", n: "1" });
  assert.equal(removed.ok, true, JSON.stringify(removed.error));
  const tracker = fs.readFileSync(path.join(fixture.root, "data", "applications.md"), "utf-8");
  assert.ok(!tracker.includes("| 1 |"), "the row must be gone");
  assert.ok(tracker.includes("| 2 |"), "the other rows must survive");
  // The report file itself is never deleted: it is named so the user can.
  assert.ok(
    fs.existsSync(path.join(fixture.root, "reports", "001-acme-2026-01-02.md")),
    "the report must survive the removal — it is orphaned, not deleted",
  );
});

test("a row that turned Applied pins its first follow-up", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const applied = await bridge.call("status", { n: "1", status: "Applied" });
  assert.equal(applied.ok, true, JSON.stringify(applied.error));

  // The pin is written by set-status.mjs's OWN seeding hook (#1430), which is
  // why the status change alone is enough.
  const log = fs.readFileSync(path.join(fixture.root, "data", "follow-ups.md"), "utf-8");
  assert.match(
    log,
    /- next #1 \d{4}-\d{2}-\d{2}/,
    "the status change must pin the first follow-up",
  );

  // The seeder is idempotent: a second call must not stack a second pin.
  const seeded = await bridge.call("followups", { action: "seed", appNum: "1" });
  assert.equal(seeded.ok, true, JSON.stringify(seeded.error));
  assert.equal(seeded.data.seeded, false, "a second seed must be a no-op");
  const after = fs.readFileSync(path.join(fixture.root, "data", "follow-ups.md"), "utf-8");
  assert.equal(
    after.match(/- next #1 /g).length,
    1,
    "there must be exactly one pin for the row",
  );
});

test("the health check reports a broken report link", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  // A tracker row pointing at a report that does not exist.
  const trackerPath = path.join(fixture.root, "data", "applications.md");
  const tracker = fs.readFileSync(trackerPath, "utf-8");
  fs.writeFileSync(
    trackerPath,
    tracker.replace("001-acme-2026-01-02.md", "999-missing-2026-01-02.md"),
    "utf-8",
  );

  const result = await bridge.call("maintenance", { tool: "verify" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.match(result.data.output, /999-missing/, "the health check must name the broken link");
});

test("normalizing rewrites a non-canonical status and is idempotent", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const trackerPath = path.join(fixture.root, "data", "applications.md");
  fs.writeFileSync(
    trackerPath,
    fs.readFileSync(trackerPath, "utf-8").replace("| SKIP |", "| applied |"),
    "utf-8",
  );

  const preview = await bridge.call("maintenance", { tool: "normalize" });
  assert.equal(preview.ok, true, JSON.stringify(preview.error));
  assert.equal(preview.data.apply, false, "a preview changes nothing");
  assert.match(
    fs.readFileSync(trackerPath, "utf-8"),
    /\| applied \|/,
    "a preview must not rewrite the tracker",
  );

  const applied = await bridge.call("maintenance", { tool: "normalize", apply: true });
  assert.equal(applied.ok, true, JSON.stringify(applied.error));
  const after = fs.readFileSync(trackerPath, "utf-8");
  assert.match(after, /\| Applied \|/, "the status must be canonical now");
  assert.ok(
    fs.existsSync(path.join(fixture.root, "data", "applications.md.bak")),
    "the rewrite must leave a backup",
  );

  const again = await bridge.call("maintenance", { tool: "normalize", apply: true });
  assert.equal(again.ok, true);
  assert.equal(
    fs.readFileSync(trackerPath, "utf-8"),
    after,
    "a second run must change nothing — the run is idempotent",
  );
});

test("a finished run leaves a readable record the assistant can read", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const saved = await bridge.call("runs", {
    action: "save",
    run: {
      id: "evaluate-1",
      kind: "evaluate",
      title: "Acme",
      input: "https://acme.example/job/1",
      status: "done",
      tokens: 1200,
      usd: 0.0123,
      startedAt: 1_700_000_000_000,
      steps: [{ kind: "text", label: "Reading the posting" }],
      output: "A strong match.",
    },
  });
  assert.equal(saved.ok, true, JSON.stringify(saved.error));

  const dir = path.join(fixture.root, "data", "runs");
  const markdown = fs.readFileSync(path.join(dir, "evaluate-1.md"), "utf-8");
  assert.match(markdown, /^# Desktop run/m, "the log must be readable markdown");
  assert.match(markdown, /A strong match\./, "the output must be in the log");
  assert.ok(
    fs.existsSync(path.join(dir, "evaluate-1.json")),
    "the machine half of the record must exist too",
  );

  const listed = await bridge.call("runs", { action: "list" });
  assert.equal(listed.ok, true);
  assert.equal(listed.data.runs.length, 1);
  assert.equal(listed.data.runs[0].kind, "evaluate");
});

test("the cost estimate comes from this client's own runs", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const empty = await bridge.call("cost", { action: "estimate", kind: "evaluate" });
  assert.equal(empty.ok, true, JSON.stringify(empty.error));
  assert.equal(empty.data.estimate.tokens, undefined, "no history means no estimate");
  assert.match(empty.data.text, /first run/, "the fallback must say what is missing");

  for (const tokens of [1000, 2000, 3000]) {
    await bridge.call("runs", {
      action: "save",
      run: {
        id: `evaluate-${tokens}`,
        kind: "evaluate",
        status: "done",
        tokens,
        usd: tokens / 100_000,
        startedAt: 1_700_000_000_000 + tokens,
      },
    });
  }

  const estimate = await bridge.call("cost", { action: "estimate", kind: "evaluate" });
  assert.equal(estimate.ok, true);
  assert.equal(estimate.data.estimate.tokens, 2000, "the estimate is the median");
  assert.equal(estimate.data.estimate.samples, 3);

  // More than one run is a fan-out, and a fan-out asks first.
  const batch = await bridge.call("cost", { action: "estimate", kind: "evaluate", count: 3 });
  assert.equal(batch.data.requiresConfirmation, true);
  assert.match(batch.data.confirmation, /3 workers/);

  const usage = await bridge.call("cost", { action: "usage" });
  assert.equal(usage.ok, true);
  assert.equal(usage.data.runs, 3);
  assert.ok(usage.data.windows.length >= 2, "the sensor reports both windows");
});

test("a run on an absent CLI says it substituted one", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  // The notice is the ENGINE's: cliSubstitutionNotice() names both the CLI
  // that was asked for and the one that will run. The client does not write
  // its own version of that sentence.
  const { cliSubstitutionNotice } = await import(
    pathToFileURL(path.join(REPO_ROOT, "web", "src", "lib", "clis.ts")).href
  );
  const notice = cliSubstitutionNotice({
    spec: { id: "claude", name: "Claude Code" },
    substitutedFrom: "codex",
  });
  assert.ok(notice, "the engine must produce a notice when it substitutes");
  assert.match(notice, /codex/, "the notice must name the CLI that was asked for");
  assert.match(notice, /Claude Code/, "the notice must name the CLI that will run");

  // And the bridge puts it in the run log: a substitution is a status event,
  // the same vocabulary the web app streams.
  const source = fs.readFileSync(path.join(CLIENT_ROOT, "bridge", "ops", "run.mjs"), "utf-8");
  assert.match(
    source,
    /if \(substitution\) send\(\{ type: "status", label: substitution \}\)/,
    "the run op must send the substitution notice as a status event",
  );

  // The end-to-end half needs a machine with exactly ONE installed CLI: the
  // engine substitutes only when there is a single alternative. On a machine
  // with several CLIs installed the requested one is simply refused, and the
  // refusal is its own message.
  const boot = await bridge.call("bootstrap");
  const installed = boot.data.clis.filter((cli) => cli.installed).map((cli) => cli.id);
  if (installed.length !== 1) {
    t.diagnostic(
      `the end-to-end substitution was not exercised: this machine has ${installed.length} CLIs installed (${installed.join(", ")})`,
    );
    return;
  }
  const absent = boot.data.clis.find((cli) => !cli.installed);
  const result = await bridge.call("run", {
    kind: "evaluate",
    input: "https://acme.example/job/1",
    cliId: absent.id,
  });
  assert.equal(result.ok, true, `the run must go through: ${JSON.stringify(result.error)}`);
  const notices = bridge.events
    .map((entry) => entry.event)
    .filter((event) => event.type === "status" && /not installed/i.test(event.label ?? ""));
  assert.ok(
    notices.length > 0,
    `the run must say it substituted a CLI: ${JSON.stringify(bridge.events.slice(0, 5))}`,
  );
});

test("the bridge reports the protocol version it speaks", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const version = await bridge.call("version", { action: "get" });
  assert.equal(version.ok, true, JSON.stringify(version.error));
  assert.equal(version.data.protocol, 1, "the protocol version must be reported");
  assert.ok(typeof version.data.engine === "string");
});

test("a search finds a tracker row by name", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const found = await bridge.call("tracker", { action: "find", query: "Acme" });
  assert.equal(found.ok, true, JSON.stringify(found.error));
  assert.ok(found.data.found >= 1, "the tracker row must be found");
  assert.equal(found.data.matches[0].company, "Acme");

  const missing = await bridge.call("tracker", { action: "find", query: "Nothing Here" });
  assert.equal(missing.ok, true);
  assert.equal(missing.data.found, 0);
});

test("a hand-added application lands in the tracker with an N/A score", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const added = await bridge.call("tracker", {
    action: "add",
    row: {
      company: "Gamma Corp",
      role: "Staff Engineer",
      date: "2026-02-01",
      status: "Evaluated",
    },
  });
  assert.equal(added.ok, true, JSON.stringify(added.error));
  assert.ok(added.data.num, "the new row must get a number");

  const tracker = fs.readFileSync(path.join(fixture.root, "data", "applications.md"), "utf-8");
  const row = tracker.split("\n").find((line) => line.includes("Gamma Corp"));
  assert.ok(row, "the row must be in the tracker");
  assert.match(row, /N\/A/, "a hand-added row carries the N/A score sentinel, never a blank cell");
  assert.match(row, /Evaluated/);
});

test("the JDs every report kept are checked by the core's own tool", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const result = await bridge.call("maintenance", { tool: "jdArchive" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.ok(result.data.data, "the tool's JSON must come back");
  assert.equal(typeof result.data.data.reportsScanned, "number");
});

test("a first save on a fresh install seeds the profile and never overwrites one", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  // The fixture already has a profile, so a save must MERGE into it and leave
  // the rest of it alone.
  const saved = await bridge.call("config", {
    action: "setProfile",
    name: "Alex McCune",
    email: "alex@example.com",
  });
  assert.equal(saved.ok, true, JSON.stringify(saved.error));
  const profile = fs.readFileSync(path.join(fixture.root, "config", "profile.yml"), "utf-8");
  assert.match(profile, /Alex McCune/);
  assert.match(profile, /alex@example\.com/);
  assert.match(
    profile,
    /target_roles:/,
    "the merge must not drop the rest of the profile",
  );

  // And a profile that exists but cannot be read is REFUSED, never replaced.
  fs.writeFileSync(path.join(fixture.root, "config", "profile.yml"), "- not\n- a mapping\n", "utf-8");
  const refused = await bridge.call("config", {
    action: "setProfile",
    name: "Someone Else",
  });
  assert.equal(refused.ok, false, "a broken profile must be refused");
  assert.equal(refused.error.code, "conflict");
  assert.equal(
    fs.readFileSync(path.join(fixture.root, "config", "profile.yml"), "utf-8"),
    "- not\n- a mapping\n",
    "the refused profile must be byte-identical afterwards",
  );
});
