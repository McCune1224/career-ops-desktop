#!/usr/bin/env node
// verify.mjs — the client's whole gate, in one command.
//
// Four things have to be true before this repository can be called green, and
// running them by hand is how one of them quietly stops being run:
//
//   1. `cargo test` — the Rust side: screens, payloads, the PDF rasteriser, the
//      dependency audit, the opt-in live-data decode.
//   2. `node --test tests/*.test.mjs` — the Node side: engine parity, the live
//      bridge, the end-to-end run, the core's own answers, and the acceptance
//      gate on FEATURES.md.
//   3. `dev/dependency-audit.mjs` — no webview crate and no webview runtime.
//
// The reason this file exists at all: the README used to document
// `node --test tests/desktop-*.test.mjs`, and that glob does NOT match
// `tests/acceptance.test.mjs` — so the check that proves the inventory is not
// lying was the one check the documented command skipped. A gate nobody runs is
// a gate that is not there.
//
//   node dev/verify.mjs            # everything
//   node dev/verify.mjs --fast     # skip the Rust side (the slow half)
//
// Exits non-zero on the first failure, with that command's own output.
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import fs from "node:fs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const fast = process.argv.includes("--fast");

/** One command, run from the repository root, output streamed through. */
function run(label, command, args) {
  process.stdout.write(`\n── ${label}\n   ${command} ${args.join(" ")}\n`);
  const result = spawnSync(command, args, {
    cwd: ROOT,
    stdio: "inherit",
    env: process.env,
  });
  if (result.error) {
    return { ok: false, why: result.error.message };
  }
  if (result.status !== 0) {
    return { ok: false, why: `exit ${result.status}` };
  }
  return { ok: true };
}

const steps = [];

if (!fast) {
  steps.push(["cargo test", () => run("the Rust side", "cargo", ["test"])]);
}

steps.push([
  "the Node side",
  () =>
    run("the Node side", process.execPath, [
      "--test",
      ...fs
        .readdirSync(path.join(ROOT, "tests"))
        .filter((name) => name.endsWith(".test.mjs"))
        .sort()
        .map((name) => path.join("tests", name)),
    ]),
]);

steps.push([
  "the dependency audit",
  () => run("the dependency audit", process.execPath, ["dev/dependency-audit.mjs"]),
]);

const failures = [];
for (const [label, step] of steps) {
  const outcome = step();
  if (!outcome.ok) {
    failures.push(`${label}: ${outcome.why}`);
    break;
  }
}

process.stdout.write("\n");
if (failures.length) {
  process.stdout.write(`❌ ${failures.join("\n❌ ")}\n`);
  process.exit(1);
}
process.stdout.write(
  `✅ green: ${steps.map(([label]) => label).join(", ")}\n`,
);
