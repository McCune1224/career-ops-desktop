// set-feature-status.mjs — update FEATURES.md's status and acceptance columns.
//
// The status column is the claim the board makes about the client, so it is
// updated in the same commit as the code that closes the row. This script is
// the only thing that writes it, so the format cannot drift.
//
//   node dev/set-feature-status.mjs <changes.json>
//
// The JSON is a map of row id to { status, acceptance }. `acceptance` is the
// name of the test that proves the row; the acceptance suite checks that the
// name exists in this repository, so a row cannot claim a check that is not
// there.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FEATURES = path.join(HERE, "..", "FEATURES.md");

/** The statuses a row may carry. */
const STATUSES = new Set(["done", "partial", "missing", "gap", "n/a"]);

/**
 * Update the rows named in `changes`.
 *
 * @param {string} file
 */
export function applyChanges(file) {
  const changes = JSON.parse(fs.readFileSync(file, "utf8"));
  const lines = fs.readFileSync(FEATURES, "utf8").split("\n");
  const seen = new Set();
  const out = lines.map((line) => {
    const match = line.match(/^\|\s*(E\d\d-S\d\d)\s*\|/);
    if (!match) return line;
    const id = match[1];
    const change = changes[id];
    if (!change) return line;
    seen.add(id);
    const cells = splitRow(line);
    if (cells.length < 7) {
      throw new Error(`${id}: the row does not have seven cells`);
    }
    if (change.status !== undefined) {
      if (!STATUSES.has(change.status)) {
        throw new Error(`${id}: unknown status ${change.status}`);
      }
      cells[3] = change.status;
    }
    if (change.acceptance !== undefined) {
      // Each check is named on its own, so the acceptance suite can tell
      // which one is a test that exists.
      const names = Array.isArray(change.acceptance)
        ? change.acceptance
        : [change.acceptance];
      cells[4] = names.map((name) => `\`${String(name).trim()}\``).join(", ");
    }
    return `| ${cells.join(" | ")} |`;
  });
  const missing = Object.keys(changes).filter((id) => !seen.has(id));
  if (missing.length) {
    throw new Error(`no row matched: ${missing.join(", ")}`);
  }
  fs.writeFileSync(FEATURES, `${out.join("\n")}`);
  return Object.keys(changes).length;
}

/**
 * Split a markdown table row into its cells.
 *
 * @param {string} line
 * @returns {string[]}
 */
function splitRow(line) {
  const trimmed = line.trim();
  const inner = trimmed.slice(1, trimmed.endsWith("|") ? -1 : undefined);
  return inner.split("|").map((cell) => cell.trim());
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const file = process.argv[2];
  if (!file) {
    console.error("usage: node dev/set-feature-status.mjs <changes.json>");
    process.exit(1);
  }
  const count = applyChanges(file);
  console.log(`updated ${count} row(s) in FEATURES.md`);
}
