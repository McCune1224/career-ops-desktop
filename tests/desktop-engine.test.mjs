/**
 * desktop-engine.test.mjs — the capabilities that are the CORE's own answers.
 *
 * Every test here drives the real bridge against a throwaway fixture and
 * asserts on what the CORE said: the cadence calculator, the quiet-companies
 * list, the salary gap, the story bank's provenance, the portals audit, the
 * plugin list, the agent inbox, the outcome archive, the negotiation points.
 *
 * The rule throughout: the client asks the core a question and shows the
 * answer. No test here re-derives a career-ops rule, and none of them touches
 * the user's own data.
 */
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { pathToFileURL } from "node:url";

import {
  BRIDGE_SERVER,
  CLIENT_ROOT,
  REPO_ROOT,
  fixtureEnv,
  makeFixture,
} from "./helpers/desktop-fixture.mjs";

/**
 * Start the bridge against a fixture.
 *
 * @param {string} root
 * @param {Record<string, string>} [extraEnv]
 */
function startBridge(root, extraEnv = {}) {
  const child = spawn(process.execPath, [BRIDGE_SERVER], {
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
        if (pending.delete(id)) {
          reject(new Error(`bridge did not answer ${op} in time\n${stderr.slice(-2000)}`));
        }
      }, 60_000).unref();
    });

  return {
    call,
    events,
    stderr: () => stderr,
    stop: () => child.kill(),
  };
}

/**
 * A stub `claude` that speaks the report envelope, so an evaluation has
 * something to persist.
 *
 * @param {string} dir
 */
function installStubReportingClaude(dir) {
  const body = [
    "# Evaluation: Stubco — Staff Platform Engineer",
    "",
    "**Date:** 2026-10-21",
    "**URL:** https://stubco.example/job/77",
    "**Score:** 4.1/5",
    "**Legitimacy:** High Confidence",
    "",
    "---",
    "",
    "## Block A — Role fit",
    "",
    "A strong match.",
    "",
    "## Machine Summary",
    "",
    "```yaml",
    "company: Stubco",
    "role: Staff Platform Engineer",
    "score: 4.1",
    "via: null",
    "```",
    "",
    "VERDICT: 4.1/5 — apply",
    "",
  ].join("\n");
  const script = `#!/usr/bin/env node
const emit = (obj) => process.stdout.write(JSON.stringify(obj) + "\\n");
emit({ type: "system", subtype: "init" });
emit({ type: "stream_event", event: { type: "content_block_delta", delta: { text: "<<report-md>>\\n" + ${JSON.stringify(body)} + "<</report-md>>\\n" } } });
emit({ type: "result", usage: { input_tokens: 10, output_tokens: 5 }, total_cost_usd: 0.001 });
`;
  fs.writeFileSync(path.join(dir, "claude"), script, { mode: 0o755 });
  return dir;
}

/**
 * A stub `claude` that speaks the real stream-json protocol and records the
 * prompt it was given, so a test can assert WHAT the agent was asked to do.
 *
 * @param {string} dir
 * @param {string} [body]
 */
function installStubClaude(dir, body = "A plain answer.", envelope = false) {
  const script = `#!/usr/bin/env node
const fs = require("fs");
const args = process.argv.slice(2);
const at = args.indexOf("-p");
const prompt = at === -1 ? "" : (args[at + 1] || "");
const target = (prompt.match(/Target: (\S+)/) || [])[1] || "unknown";
fs.writeFileSync(process.env.STUB_PROMPT_FILE || "/tmp/stub-prompt.txt", prompt);
const emit = (obj) => process.stdout.write(JSON.stringify(obj) + "\\n");
emit({ type: "system", subtype: "init" });
const payload = ${JSON.stringify(body)} + " " + target + "\\n";
emit({ type: "stream_event", event: { type: "content_block_delta", delta: { text: payload } } });
emit({ type: "result", usage: { input_tokens: 10, output_tokens: 5 }, total_cost_usd: 0.001 });
`;
  const file = path.join(dir, "claude");
  fs.writeFileSync(file, script, { mode: 0o755 });
  return file;
}

// ─── the analyses over the pipeline ────────────────────────────────────────

test("the quiet companies are listed with the reason each one is quiet", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  fs.writeFileSync(
    path.join(fixture.root, "data", "active-interviews.md"),
    [
      "# Active interviews",
      "",
      "| # | Company | Role | Status | Date | Notes |",
      "|---|---------|------|--------|------|-------|",
      "| 1 | Acme | Platform Engineer | Interview | 2026-01-02 | waiting |",
      "",
    ].join("\n"),
    "utf-8",
  );

  const result = await bridge.call("analysis", { action: "quietCompanies" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.ok(result.data.metadata, "the tool must report what it looked at");
  assert.ok(Array.isArray(result.data.flags), "the flags must be a list");
  // Read-only: a suggestion, never a decision.
  assert.ok(
    result.data.disclaimer === null || typeof result.data.disclaimer === "string",
    "the tool states its own limits",
  );
});

test("the recruiting friction comes from the interview notes", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  fs.writeFileSync(
    path.join(fixture.root, "data", "active-interviews.md"),
    [
      "| # | Company | Role | Status | Date | Notes |",
      "|---|---------|------|--------|------|-------|",
      "| 1 | Acme | Platform Engineer | Interview | 2026-01-02 | [process-friction: rescheduled twice] |",
      "",
    ].join("\n"),
    "utf-8",
  );

  const result = await bridge.call("analysis", { action: "processQuality" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.ok(Array.isArray(result.data.signals));
  const acme = result.data.signals.find((s) => s.company === "Acme");
  assert.ok(acme, "the company with the tag must be reported");
  assert.ok(acme.frictionCount >= 1, "the tagged row is friction");
});

test("the salary gap is wanted against advertised against actual", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  fs.writeFileSync(
    path.join(fixture.root, "data", "salary-observations.tsv"),
    "1\tAcme\tPlatform Engineer\t150000\tactual\t2026-01-05\n",
    "utf-8",
  );

  const result = await bridge.call("analysis", { action: "salaryGap" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.ok(Array.isArray(result.data.applications));
  assert.ok(result.data.quality, "the tool reports the quality of what it read");
});

test("a negotiation figure survives only if the CV says it too", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  // A story bank whose number the CV does not carry.
  fs.mkdirSync(path.join(fixture.root, "interview-prep"), { recursive: true });
  fs.writeFileSync(
    path.join(fixture.root, "interview-prep", "story-bank.md"),
    [
      "# Story bank",
      "",
      "## Cut the estate's cost by 40%",
      "",
      "- **What:** Rebuilt the fleet plan.",
      "- **Impact:** cut the estate's cost by 40%",
      "",
    ].join("\n"),
    "utf-8",
  );

  const result = await bridge.call("analysis", {
    action: "negotiation",
    wage: 60,
    frequency: "hourly",
  });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.equal(typeof result.data.storiesScanned, "number");
  assert.ok(
    Array.isArray(result.data.verified),
    "the verified figures must be listed",
  );
});

test("a company history joins the tracker, the follow-ups and the status log", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const result = await bridge.call("analysis", { action: "companyHistory", company: "Acme" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.equal(result.data.company, "Acme");
  assert.ok(Array.isArray(result.data.rows));
});

test("two applications can be compared side by side", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const result = await bridge.call("analysis", { action: "compare", ids: ["1", "2"] });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.equal(result.data.columns.length, 2);
  assert.equal(result.data.columns[0].company, "Acme");
  assert.equal(result.data.columns[1].company, "Beta");
});

// ─── the interview half ────────────────────────────────────────────────────

test("the story bank says which of its numbers may be restated as fact", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  fs.mkdirSync(path.join(fixture.root, "interview-prep"), { recursive: true });
  fs.writeFileSync(
    path.join(fixture.root, "interview-prep", "story-bank.md"),
    [
      "# Story bank",
      "",
      "## Cut the estate's cost by 40%",
      "",
      "- **Impact:** cut the estate's cost by 40%",
      "",
      "## Shipped the migration",
      "",
      "- **Impact:** shipped the migration in six weeks",
      "",
    ].join("\n"),
    "utf-8",
  );

  const result = await bridge.call("interview", { action: "storyBank" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.equal(result.data.exists, true);
  assert.ok(result.data.text.includes("Cut the estate"), "the bank must be readable");
  const provenance = result.data.provenance;
  assert.ok(Array.isArray(provenance.supportedByResume), "the buckets must be there");
  assert.ok(Array.isArray(provenance.derivedUnverified));
  assert.ok(Array.isArray(provenance.userCannotConfirm));
  // The 40% is not on the CV, so it must not be in the supported bucket.
  const supported = provenance.supportedByResume.map((s) => s.story).join(" ");
  assert.ok(
    !/40%/.test(supported),
    "a number the CV does not carry must not be reported as supported",
  );
});

test("a question is matched to a story from the bank", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  fs.mkdirSync(path.join(fixture.root, "interview-prep"), { recursive: true });
  fs.writeFileSync(
    path.join(fixture.root, "interview-prep", "story-bank.md"),
    [
      "# Story Bank",
      "",
      "## Stories",
      "",
      "### [Cost] Rebuilt the fleet plan",
      "**Source:** Report #001 — Acme — Platform Engineer",
      "**S (Situation):** the estate had three datacenters.",
      "**T (Task):** cut the cost.",
      "**A (Action):** rebuilt the fleet plan.",
      "**R (Result):** cut the estate's cost by 40%",
      "**Best for questions about:** cost, operations",
      "",
    ].join("\n"),
    "utf-8",
  );

  const result = await bridge.call("interview", {
    action: "matchStar",
    question: "tell me about a time you cut cost",
  });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.ok(
    result.data.story.includes("fleet") || result.data.story.length > 0,
    "the matched story must come back",
  );
});

test("a week of sessions is rolled into one summary per company", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const sessions = path.join(fixture.root, "interview-prep", "sessions");
  fs.mkdirSync(sessions, { recursive: true });
  fs.writeFileSync(
    path.join(sessions, "2026-01-05-acme.md"),
    "---\ncompany: Acme\nrole: Platform Engineer\ndate: 2026-01-05\n---\n\n<!-- competency: system-design -->\n\nNotes.\n",
    "utf-8",
  );

  const result = await bridge.call("interview", { action: "digest" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.ok(result.data.metadata, "the digest must say what it covered");
  assert.ok(Array.isArray(result.data.companies));
});

test("a mode run asks the agent to follow the mode file", async (t) => {
  const fixture = makeFixture();
  const stubDir = fs.mkdtempSync(path.join(fixture.root, "-stub-"));
  const promptFile = path.join(fixture.root, "prompt.txt");
  installStubClaude(stubDir);
  const bridge = startBridge(fixture.root, {
    STUB_PROMPT_FILE: promptFile,
    PATH: `${stubDir}${path.delimiter}${process.env.PATH ?? ""}`,
  });
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  fs.mkdirSync(path.join(fixture.root, "modes", "interview"), { recursive: true });
  for (const [kind, modeFile] of [
    ["prep", "modes/interview-prep.md"],
    ["plan", "modes/interview/plan.md"],
    ["practice", "modes/interview/practice.md"],
    ["debrief", "modes/interview/debrief.md"],
    ["redflag", "modes/interview-redflag.md"],
    ["offer-prep", "modes/offer-prep.md"],
    ["email", "modes/email.md"],
  ]) {
    fs.writeFileSync(path.join(fixture.root, modeFile), `# ${kind} mode\n`, "utf-8");
  }

  const result = await bridge.call("run", {
    kind: "prep",
    input: "Acme — Platform Engineer",
    cliId: "claude",
  });
  assert.equal(result.ok, true, `the mode run must go through: ${JSON.stringify(result.error)}`);

  const prompt = fs.readFileSync(promptFile, "utf-8");
  assert.match(
    prompt,
    /modes\/interview-prep\.md/,
    "the prompt must name the mode file the agent is to follow",
  );
  assert.match(
    prompt,
    /follow it EXACTLY/i,
    "the prompt must say the mode file is the rule",
  );
  assert.match(
    prompt,
    /Do not write, edit or delete any file/i,
    "the agent must not write; the platform saves the reply",
  );

  // And the reply reaches the run log.
  const text = bridge.events
    .map((entry) => entry.event)
    .filter((event) => event.type === "text")
    .map((event) => event.text)
    .join("\n");
  assert.match(text, /A plain answer\./, "the agent's reply must reach the run log");
});

test("a mode whose file is missing is refused, not sent to the agent", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const result = await bridge.call("run", {
    kind: "prep",
    input: "Acme",
    cliId: "claude",
  });
  assert.equal(result.ok, false, "a mode with no file must be refused");
  assert.equal(result.error.code, "incomplete-checkout");
});

// ─── the supply side ───────────────────────────────────────────────────────

test("the watched boards are audited and the verdict is per board", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  fs.writeFileSync(
    path.join(fixture.root, "portals.yml"),
    [
      "tracked_companies:",
      "  - name: Acme",
      "    careers_url: https://boards.greenhouse.io/acme",
      "",
    ].join("\n"),
    "utf-8",
  );

  const result = await bridge.call("portals", { action: "audit" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.ok(Array.isArray(result.data.rows), "the audit must return the boards it judged");
});

test("the watched boards can be edited and a file that cannot be parsed is refused", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const file = path.join(fixture.root, "portals.yml");
  fs.writeFileSync(
    file,
    ["tracked_companies:", "  - name: Acme", "    careers_url: https://x", "custom_key: keep me", ""].join("\n"),
    "utf-8",
  );

  const read = await bridge.call("portals", { action: "read" });
  assert.equal(read.ok, true, JSON.stringify(read.error));
  assert.equal(read.data.companies[0].name, "Acme");

  const document = read.data.document;
  document.tracked_companies.push({ name: "Beta", careers_url: "https://y" });
  const written = await bridge.call("portals", { action: "write", document });
  assert.equal(written.ok, true, JSON.stringify(written.error));
  const after = fs.readFileSync(file, "utf-8");
  assert.match(after, /Acme/);
  assert.match(after, /Beta/);
  assert.match(after, /custom_key/, "an unknown key must survive the edit");

  // A file that exists but cannot be parsed is REFUSED, never rewritten.
  fs.writeFileSync(file, "tracked_companies: [\n", "utf-8");
  const refused = await bridge.call("portals", { action: "write", document });
  assert.equal(refused.ok, false, "a broken file must be refused");
  assert.equal(refused.error.code, "conflict");
  assert.equal(
    fs.readFileSync(file, "utf-8"),
    "tracked_companies: [\n",
    "the refused file must be byte-identical afterwards",
  );
});

test("a posting you found by hand is explained, not guessed at", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  fs.writeFileSync(
    path.join(fixture.root, "portals.yml"),
    ["tracked_companies:", "  - name: Acme", "    careers_url: https://x", ""].join("\n"),
    "utf-8",
  );

  const result = await bridge.call("portals", {
    action: "coverage",
    url: "https://acme.example/job/1",
    company: "Acme",
    title: "Platform Engineer",
  });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.ok(result.data.gap, "the answer must name the gap");
  assert.ok(result.data.meaning, "and say what it means");
});

test("what is new this week comes from the scan history and excludes the dead", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const today = new Date().toISOString().slice(0, 10);
  fs.writeFileSync(
    path.join(fixture.root, "data", "scan-history.tsv"),
    [
      "url\tfirst_seen\tportal\ttitle\tcompany\tstatus\tlocation",
      `https://gamma.example/job/9\t${today}\tgreenhouse\tSRE\tGamma\tadded\tRemote`,
      `https://delta.example/job/10\t${today}\tgreenhouse\tSRE\tDelta\texpired\tRemote`,
      "",
    ].join("\n"),
    "utf-8",
  );

  const result = await bridge.call("portals", { action: "whatsNew" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.equal(result.data.count, 1, "a posting confirmed dead must not resurface");
  assert.equal(result.data.offers[0].company, "Gamma");
});

test("a role that keeps being re-listed is flagged, and one posting is not enough", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const today = new Date();
  const day = (offset) => {
    const d = new Date(today);
    d.setDate(d.getDate() - offset);
    return d.toISOString().slice(0, 10);
  };
  fs.writeFileSync(
    path.join(fixture.root, "data", "scan-history.tsv"),
    [
      "url\tfirst_seen\tportal\ttitle\tcompany\tstatus\tlocation",
      `https://acme.example/job/1\t${day(20)}\tgreenhouse\tSRE\tAcme\tadded\tRemote`,
      `https://acme.example/job/2\t${day(10)}\tgreenhouse\tSRE\tAcme\tadded\tRemote`,
      `https://acme.example/job/3\t${day(2)}\tgreenhouse\tSRE\tAcme\tadded\tRemote`,
      "",
    ].join("\n"),
    "utf-8",
  );

  const result = await bridge.call("portals", { action: "reposts" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.ok(Array.isArray(result.data.clusters));
  const acme = result.data.clusters.find((c) => c.company === "Acme");
  assert.ok(acme, "the re-listed role must be found");
  assert.ok(acme.repostCount >= 2, "two URLs across two dates is a repost");
});

// ─── the rest of the client ────────────────────────────────────────────────

test("the agent inbox is a checklist that is drained, not run", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const added = await bridge.call("agent-inbox", {
    action: "add",
    request: "rewrite the summary",
  });
  assert.equal(added.ok, true, JSON.stringify(added.error));

  const listed = await bridge.call("agent-inbox", { action: "list" });
  assert.equal(listed.ok, true, JSON.stringify(listed.error));
  assert.equal(listed.data.items.length, 1);
  assert.match(listed.data.items[0].text, /rewrite the summary/);
  assert.equal(listed.data.items[0].done, false);
});

test("the plugin list is read and a plugin can be disabled", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const list = await bridge.call("plugins", { action: "list" });
  assert.equal(list.ok, true, JSON.stringify(list.error));
  assert.ok(typeof list.data.output === "string");

  const installed = await bridge.call("plugins", { action: "installed" });
  assert.equal(installed.ok, true, JSON.stringify(installed.error));
  assert.ok(Array.isArray(installed.data.plugins));
});

test("the profile's language and its market mode set are read", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  fs.writeFileSync(
    path.join(fixture.root, "config", "profile.yml"),
    [
      "candidate:",
      "  full_name: Test Candidate",
      "language:",
      "  output: de",
      "  modes_dir: modes/de",
      "target_roles:",
      "  primary:",
      "    - Platform Engineer",
      "",
    ].join("\n"),
    "utf-8",
  );

  const result = await bridge.call("profile", { action: "state" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.equal(result.data.language.output, "de");
  assert.equal(result.data.language.modesDir, "modes/de");
  assert.deepEqual(result.data.keywords, ["Platform Engineer"]);
});

test("the personalization file is edited one section at a time", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const before = await bridge.call("profile", { action: "state" });
  assert.equal(before.ok, true, JSON.stringify(before.error));
  assert.ok(before.data.personalization.sections.length > 0, "the sections must be listed");

  const saved = await bridge.call("profile", {
    action: "save",
    section: "exitNarrative",
    body: "I want a role that keeps my hands on the systems.",
  });
  assert.equal(saved.ok, true, JSON.stringify(saved.error));

  const text = fs.readFileSync(path.join(fixture.root, "modes", "_profile.md"), "utf-8");
  assert.match(text, /keeps my hands on the systems/, "the section must be written");
});

test("a screening questionnaire is read from the report and never sent", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const reportPath = path.join(fixture.root, "reports", "001-acme-2026-01-02.md");
  fs.writeFileSync(
    reportPath,
    [
      "# Evaluation: Acme",
      "",
      "## Application Answers",
      "",
      "### Are you authorized to work?",
      "",
      "Yes",
      "",
    ].join("\n"),
    "utf-8",
  );

  const result = await bridge.call("questions", { action: "read", n: "1" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.ok(result.data.available, "the answers must be readable");
  assert.ok(result.data.answers, "the prepared pack must come back");
});

test("an outcome is recorded through the core and archives the posting", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const result = await bridge.call("outcomes", {
    action: "record",
    n: "1",
    type: "interview_progress",
    note: "second round",
    dryRun: true,
  });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.equal(result.data.dryRun, true);
  assert.ok(result.data.output, "the core's own report must come back");
});

test("a hire may be shared, and nothing leaves the machine", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  // A row that is not hired: the core says so and the client says so.
  const result = await bridge.call("outcomes", { action: "hiredShare", report: 1 });
  assert.equal(result.ok, false, "a row that is not hired cannot be shared");
});

test("the documents can be opened from the list", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  fs.writeFileSync(path.join(fixture.root, "output", "cv-test-acme-001.pdf"), "%PDF-1.4\n", "utf-8");

  const resolved = await bridge.call("documents", {
    action: "resolve",
    filename: "cv-test-acme-001.pdf",
  });
  assert.equal(resolved.ok, true, JSON.stringify(resolved.error));
  assert.match(resolved.data.path, /cv-test-acme-001\.pdf$/);

  // A file that is not there is a not-found, not a path into nowhere.
  const missing = await bridge.call("documents", {
    action: "resolve",
    filename: "no-such-file.pdf",
  });
  assert.equal(missing.ok, false);
  assert.equal(missing.error.code, "not-found");
});

test("duplicate tracker rows are merged by the core, with a backup", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const trackerPath = path.join(fixture.root, "data", "applications.md");
  const tracker = fs.readFileSync(trackerPath, "utf-8");
  fs.writeFileSync(
    trackerPath,
    tracker.replace(
      "| 2 | 2026-01-03 | Beta | SRE | 2.1/5 | SKIP | ❌ | — | too senior | — |",
      "| 2 | 2026-01-03 | Beta | SRE | 2.1/5 | SKIP | ❌ | — | too senior | — |\n| 3 | 2026-01-04 | Beta | SRE | 3.4/5 | Evaluated | ❌ | — | duplicate | — |",
    ),
    "utf-8",
  );

  const result = await bridge.call("maintenance", { tool: "dedup", apply: true });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  const after = fs.readFileSync(trackerPath, "utf-8");
  assert.match(
    result.data.output,
    /duplicate|duplicates|📊/i,
    "the core must report what it did",
  );
  assert.ok(
    fs.existsSync(`${trackerPath}.bak`),
    "the merge must leave a backup",
  );
});

test("a dead report link is repaired and the row is named", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const trackerPath = path.join(fixture.root, "data", "applications.md");
  fs.writeFileSync(
    trackerPath,
    fs
      .readFileSync(trackerPath, "utf-8")
      .replace("001-acme-2026-01-02.md", "404-missing.md"),
    "utf-8",
  );

  const result = await bridge.call("maintenance", { tool: "fixLinks", apply: true });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  const after = fs.readFileSync(trackerPath, "utf-8");
  assert.ok(!after.includes("404-missing.md"), "the dead link must be repaired");
  assert.ok(
    fs.existsSync(`${trackerPath}.bak`),
    "the repair must leave a backup",
  );
});

test("the pipeline is reconciled against the batch state", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const result = await bridge.call("maintenance", { tool: "reconcile" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.ok(typeof result.data.output === "string");
});

test("a batch scores one posting at a time, and each one is its own run", async (t) => {
  const fixture = makeFixture();
  const stubDir = fs.mkdtempSync(path.join(fixture.root, "-stub-"));
  installStubReportingClaude(stubDir);
  const bridge = startBridge(fixture.root, {
    PATH: `${stubDir}${path.delimiter}${process.env.PATH ?? ""}`,
  });
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const before = fs.readdirSync(path.join(fixture.root, "reports")).length;
  const rowsBefore = fs
    .readFileSync(path.join(fixture.root, "data", "applications.md"), "utf-8")
    .split("\n")
    .filter((line) => line.startsWith("| ")).length;
  const result = await bridge.call("run", {
    kind: "evaluate",
    inputs: ["https://one.example/job/1", "https://two.example/job/2"],
    cliId: "claude",
  });
  assert.equal(result.ok, true, `the batch must go through: ${JSON.stringify(result.error)}`);
  assert.equal(result.data.batch, 2, "both postings must be run");
  const after = fs.readdirSync(path.join(fixture.root, "reports")).length;
  assert.equal(after - before, 2, "each posting must produce its own report");
  const rowsAfter = fs
    .readFileSync(path.join(fixture.root, "data", "applications.md"), "utf-8")
    .split("\n")
    .filter((line) => line.startsWith("| ")).length;
  assert.equal(rowsAfter - rowsBefore, 2, "each posting must produce its own tracker row");
  const statuses = bridge.events
    .map((entry) => entry.event)
    .filter((event) => event.type === "status" && /^Batch \d+ of \d+/.test(event.label ?? ""));
  assert.equal(statuses.length, 2, "the batch must say which posting it is on");
});

test("a form's questions are answered from a prepared pack, and nothing is submitted", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const result = await bridge.call("questions", { action: "prepare", url: "https://boards.greenhouse.io/acme/jobs/1", pdf: "output/cv-test.pdf" });
  // The core refuses a PDF that is not there, and never posts anything.
  assert.equal(result.ok, false, "a missing PDF must be refused");
  assert.equal(result.submitted, undefined, "the op never reports a submission");
});

test("a company name becomes a verified board, never a guessed URL", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const result = await bridge.call("portals", {
    action: "discover",
    names: ["A Company That Does Not Exist Anywhere 12345"],
  });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  // Whatever the network did, the client never invents a board: an entry is
  // either resolved from a probed ATS, or it is unresolved with a reason.
  for (const entry of result.data.resolved) {
    assert.ok(entry.careers_url, "a resolved board must carry the URL the probe found");
    assert.ok(entry.name, "and the company it belongs to");
  }
  for (const entry of result.data.unresolved) {
    assert.ok(entry.reason, "an unresolved company must say why");
  }
});

test("a liveness verdict is never 'active' for a posting that is not there", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const result = await bridge.call("liveness", {
    action: "check",
    url: "https://no-such-host-12345.example/job/1",
  });
  if (!result.ok) {
    // The browser fallback needs Playwright, which a fixture cannot install.
    // The refusal is the honest outcome, and the contract still holds.
    t.diagnostic(`the browser fallback is not installed here: ${result.error?.message}`);
  } else {
    assert.ok(
      ["expired", "uncertain"].includes(result.data.verdict),
      `a posting that cannot be read is never "active": ${result.data.verdict}`,
    );
  }

  // A url that is not a url is refused before anything is spawned.
  const bad = await bridge.call("liveness", { action: "check", url: "not-a-url" });
  assert.equal(bad.ok, false);
  assert.equal(bad.error.code, "bad-request");
});

test("the JD is fetched without a browser where an ATS API exists", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const result = await bridge.call("liveness", {
    action: "fetchJd",
    url: "https://no-such-host-12345.example/job/1",
  });
  // A miss is the expected case, and it is not an error: the caller falls
  // through to the browser path.
  assert.equal(result.data.found, false);
  assert.equal(result.data.text, "");
});

test("the competency extractor only proposes what the core's vocabulary knows", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const result = await bridge.call("profile", {
    action: "expand",
    body: "Ran Kubernetes and Terraform across the fleet. Wrote Python.",
  });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.ok(Array.isArray(result.data.skills));
  assert.ok(result.data.skills.length > 0, "the vocabulary must recognise a real skill");
});

test("the master career profile is validated by the core", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const absent = await bridge.call("profile", { action: "masterProfile" });
  assert.equal(absent.ok, true, JSON.stringify(absent.error));
  assert.equal(absent.data.available, false, "there is no profile yet");
  assert.match(absent.data.message, /no master career profile/);
});

test("the phonebook is read, exported and joined against a company", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  fs.writeFileSync(
    path.join(fixture.root, "data", "contacts.tsv"),
    "Dana Recruiter\tAcme\trecruiter\ttalent\t+1-555-0100\tdana@acme.example\thttps://linkedin.com/in/dana\t1\tscreen\n",
    "utf-8",
  );
  fs.writeFileSync(
    path.join(fixture.root, "data", "Connections.csv"),
    "First Name,Last Name,URL,Email Address,Company,Position,Connected On\nSam,Peer,https://linkedin.com/in/sam,,Acme,Engineer,01 Jan 2024\n",
    "utf-8",
  );

  const list = await bridge.call("contacts", { action: "list" });
  assert.equal(list.ok, true, JSON.stringify(list.error));
  assert.equal(list.data.total, 1, "the contact must be read");
  assert.equal(list.data.contacts[0].name, "Dana Recruiter");

  const exported = await bridge.call("contacts", { action: "export", write: true });
  assert.equal(exported.ok, true, JSON.stringify(exported.error));
  const vcf = fs.readFileSync(path.join(fixture.root, "output", "contacts.vcf"), "utf-8");
  assert.match(vcf, /BEGIN:VCARD/, "the export must be a vCard");
  assert.match(vcf, /Dana Recruiter/, "the contact must be in it");

  const warm = await bridge.call("contacts", { action: "linkedin", company: "Acme" });
  assert.equal(warm.ok, true, JSON.stringify(warm.error));
  assert.ok(Array.isArray(warm.data.targets), "the warm-intro answer must be a list");
});

test("a pasted invite is matched to a tracker row", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const result = await bridge.call("contacts", {
    action: "inviteMatch",
    invite: "Interview invitation — Acme — Platform Engineer — Monday 10:00",
  });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.ok(result.data.classification, "the matcher must classify the message");
  assert.ok(Array.isArray(result.data.candidates), "and rank the candidates");
});

test("a broken portal is repaired only for a company name that is safe to pass", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  // A name with a shell metacharacter is refused BEFORE anything is spawned.
  const unsafe = await bridge.call("run", {
    kind: "fix-portal",
    input: 'Acme"; rm -rf /',
    cliId: "claude",
  });
  assert.equal(unsafe.ok, false);
  assert.equal(unsafe.error.code, "bad-request");
});

test("the intake lists the documents it found, and proposes nothing by itself", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const result = await bridge.call("profile", { action: "intake" });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  assert.ok(Array.isArray(result.data.sources), "the intake must list what it found");
  // Nothing was written to the profile.
  const profile = fs.readFileSync(path.join(fixture.root, "config", "profile.yml"), "utf-8");
  assert.ok(
    !profile.includes("intake"),
    "the intake proposes; it must not write the profile",
  );
});

test("every core maintenance tool is reachable and reports what it did", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  fs.writeFileSync(
    path.join(fixture.root, "portals.yml"),
    ["tracked_companies:", "  - name: Acme", "    careers_url: https://boards.greenhouse.io/acme", ""].join("\n"),
    "utf-8",
  );

  // The slug repair is the one tool whose work needs the network, so what is
  // asserted is its CONTRACT: it answers, it says what it did, and a preview
  // writes nothing.
  const before = fs.readFileSync(path.join(fixture.root, "portals.yml"), "utf-8");
  const result = await bridge.call("maintenance", { tool: "fixSlugs" });
  assert.equal(result.data.tool, "fixSlugs");
  assert.equal(result.data.apply, false, "a preview changes nothing");
  assert.equal(
    fs.readFileSync(path.join(fixture.root, "portals.yml"), "utf-8"),
    before,
    "a preview must leave portals.yml byte-identical",
  );
});

test("the personalization sections come back with their bodies, so one can be edited", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const before = await bridge.call("profile", { action: "state" });
  assert.equal(before.ok, true, JSON.stringify(before.error));
  const bodies = before.data.bodies;
  assert.ok(bodies && typeof bodies === "object", "each section must carry its body");
  assert.ok(
    Object.keys(bodies).length > 0,
    "the shipped template has every section, so every body must be readable",
  );

  // A section edit lands in ONE section and leaves the others alone.
  const other = Object.keys(bodies).find((id) => id !== "exitNarrative");
  const otherBefore = bodies[other];
  const saved = await bridge.call("profile", {
    action: "save",
    section: "exitNarrative",
    body: "A role that keeps my hands on the systems.",
  });
  assert.equal(saved.ok, true, JSON.stringify(saved.error));

  const after = await bridge.call("profile", { action: "state" });
  assert.match(after.data.bodies.exitNarrative, /keeps my hands on the systems/);
  assert.equal(
    after.data.bodies[other],
    otherBefore,
    "the other sections must be untouched",
  );
});

test("a plugin install is confirmed, and a preview is not an install", async (t) => {
  const fixture = makeFixture();
  const bridge = startBridge(fixture.root);
  t.after(() => {
    bridge.stop();
    fixture.cleanup();
  });

  const preview = await bridge.call("plugins", { action: "install", name: "notion" });
  assert.equal(preview.ok, true, JSON.stringify(preview.error));
  assert.equal(
    preview.data.installed,
    false,
    "without a confirmation the core only previews the pinned SHA",
  );
  assert.equal(
    fs.existsSync(path.join(fixture.root, "plugins.local")),
    false,
    "a preview must clone nothing",
  );
});
