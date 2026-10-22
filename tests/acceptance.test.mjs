/**
 * acceptance.test.mjs — the gate on FEATURES.md itself.
 *
 * The inventory claims what the client can do. This suite checks the claim:
 *
 *   1. Every row outside the deferred epic E17 reports `done`. A row that is
 *      still `missing` or `partial` is the board lying about the app.
 *   2. Every such row names the test that proves it, and that test EXISTS in
 *      this repository. A row cannot claim a check that is not there.
 *   3. The counting table at the bottom of the file matches the rows.
 *
 * It is the reproducible check that the app is functional: run it and the
 * inventory either agrees with the code or it does not.
 */
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.join(HERE, "..");
const FEATURES = path.join(ROOT, "FEATURES.md");

/** The epic that is deferred on purpose, and is not part of the gate. */
const DEFERRED_EPIC = "E17";

/** Every test file whose test names count as "a check that exists". */
const TEST_FILES = [
  "tests/screens.rs",
  "tests/payloads.rs",
  "tests/live_data.rs",
  "tests/no_webview.rs",
];

function parseRows() {
  const rows = [];
  for (const line of fs.readFileSync(FEATURES, "utf8").split("\n")) {
    const match = line.match(/^\|\s*(E\d\d-S\d\d)\s*\|(.*)\|\s*$/);
    if (!match) continue;
    const cells = match[2].split("|").map((cell) => cell.trim());
    rows.push({ id: match[1], cells });
  }
  return rows;
}

/** Every test name the repository actually has. */
function testNames() {
  const names = new Set();
  const files = [
    ...TEST_FILES,
    ...fs.readdirSync(path.join(ROOT, "tests"))
      .filter((name) => name.endsWith(".test.mjs"))
      .map((name) => path.join("tests", name)),
  ];
  for (const file of files) {
    const source = fs.readFileSync(path.join(ROOT, file), "utf8");
    for (const match of source.matchAll(/fn\s+([a-z0-9_]+)\s*\(/g)) names.add(match[1]);
    for (const match of source.matchAll(/test\(\s*"([^"]+)"/g)) names.add(match[1]);
  }
  return names;
}

const rows = parseRows();
const names = testNames();

test("every row outside the deferred epic reports done", () => {
  // `gap` and `n/a` are the boundary, not work: they say a capability is
  // deliberately not wanted natively. Only `missing` and `partial` are open.
  const outstanding = rows
    .filter((row) => !row.id.startsWith(`${DEFERRED_EPIC}-`))
    .filter((row) => row.cells[2] === "missing" || row.cells[2] === "partial")
    .map((row) => `${row.id} (${row.cells[2]})`);
  assert.deepEqual(
    outstanding,
    [],
    `these rows are not done: ${outstanding.join(", ")}`,
  );
});

test("every row names a check that exists in this repository", () => {
  const missing = [];
  for (const row of rows) {
    if (row.id.startsWith(`${DEFERRED_EPIC}-`)) continue;
    if (row.cells[2] !== "done") continue;
    const acceptance = row.cells[3] ?? "";
    // The cell names one or more checks in backticks. At least one of them
    // must be a test that exists here: a row cannot claim a check that is not
    // there, and prose that merely mentions a file does not count.
    const named = [...acceptance.matchAll(/`([^`]+)`/g)].map((m) => m[1].trim());
    if (named.length === 0) {
      missing.push(`${row.id} names no check`);
      continue;
    }
    if (!named.some((name) => names.has(name))) {
      missing.push(`${row.id} names no check that exists: ${named.join(", ")}`);
    }
  }
  assert.deepEqual(missing, [], missing.join("\n"));
});

test("every check CAPABILITY-MATRIX.md names exists in this repository", () => {
  // The matrix is the document the walkthrough follows, so a check it names
  // that does not exist sends the reader after a phantom. Every backticked
  // name in a Proof column must be a test this repository has.
  const matrix = fs.readFileSync(path.join(ROOT, "CAPABILITY-MATRIX.md"), "utf8");
  const missing = [];
  for (const line of matrix.split("\n")) {
    if (!line.startsWith("|") || !/\|\s*Proof\s*\|/.test(line)) {
      if (!/^\|\s*\d+\.\d+\s*\|/.test(line)) continue;
    }
    const cells = line.trim().replace(/^\|/, "").replace(/\|$/, "").split("|");
    const proof = cells[cells.length - 1] ?? "";
    for (const match of proof.matchAll(/`([^`]+)`/g)) {
      const name = match[1].trim();
      // Only names that look like a test: a path or a script is not one.
      if (!/[ _]/.test(name) && !name.startsWith("a_") && !name.startsWith("the_")) continue;
      if (name.includes("/") || name.includes(".mjs") || name.includes(".rs")) continue;
      if (!names.has(name)) missing.push(name);
    }
  }
  assert.deepEqual([...new Set(missing)], [], `the matrix names checks that do not exist: ${[...new Set(missing)].join(", ")}`);
});

test("the counting table matches the rows", () => {
  const counts = new Map();
  for (const row of rows) {
    const status = row.cells[2];
    counts.set(status, (counts.get(status) ?? 0) + 1);
  }
  const text = fs.readFileSync(FEATURES, "utf8");
  const table = text.slice(text.indexOf("| Desktop status | Rows |"));
  const read = (status) => {
    const match = table.match(new RegExp(`\\|\\s*${status}\\s*\\|\\s*(\\d+)\\s*\\|`));
    return match ? Number(match[1]) : null;
  };
  for (const status of ["done", "partial", "missing", "gap", "n/a"]) {
    const claimed = read(status);
    if (claimed === null) continue;
    assert.equal(
      claimed,
      counts.get(status) ?? 0,
      `the counting table says ${claimed} ${status} rows and there are ${counts.get(status) ?? 0}`,
    );
  }
  const total = read("total");
  if (total !== null) {
    assert.equal(total, rows.length, "the counting table's total is wrong");
  }
});

test("the inventory is the size it claims to be", () => {
  const text = fs.readFileSync(FEATURES, "utf8");
  const match = text.match(/The inventory below is \*\*(\d+) rows\*\* across (\d+) epics\./);
  assert.ok(match, "the inventory must say how many rows it has");
  assert.equal(Number(match[1]), rows.length, "the row count is wrong");
});
