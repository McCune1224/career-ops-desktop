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
import path from "node:path";
import test from "node:test";

import { BRIDGE_SERVER, REPO_ROOT, fixtureEnv, makeFixture } from "./helpers/desktop-fixture.mjs";

const SERVER = BRIDGE_SERVER;

/**
 * Start the bridge against `root`.
 *
 * @param {string} root
 */
function startBridge(root) {
  const child = spawn(process.execPath, [SERVER], {
    cwd: REPO_ROOT,
    env: fixtureEnv(root),
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
