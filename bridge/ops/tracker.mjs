// tracker — the tracker's own maintenance: finding a row, adding one by hand,
// and removing one.
//
// Every write here goes through the core's own writer:
//
//   * a hand-added row is a TSV in batch/tracker-additions/, merged by
//     merge-tracker.mjs — the same file, the same lock, the same numbering
//     rules as a batch worker's row
//   * a removal is tracker.mjs delete, which reindexes the tracker and names
//     the report it orphans
//
// The client never edits data/applications.md itself. A second writer would
// race the core's lock silently.
import fs from "node:fs";
import path from "node:path";

import { careerOpsRoot, rootScript, readApplications } from "../engine.mjs";
import { runNode, BridgeError, parseJsonOutput } from "../util.mjs";

const SCRIPT_TIMEOUT_MS = 60_000;

/**
 * The orphaned report a tracker.mjs delete names.
 *
 * The CLI says it in prose: "(report file would be orphaned: [002](…))" for a
 * dry run and "Note: report file may now be orphaned — <link>" for a live one.
 * The link is what the screen shows, so it is lifted out of the sentence.
 *
 * @param {string} output
 * @returns {string | null}
 */
export function orphanedReportFrom(output) {
  const text = String(output ?? "");
  const match = text.match(/orphaned(?:\s+file)?[:\s—-]*\s*(\[[^\]]+\]\([^)]+\)|[^\s(]+\.md)/i);
  if (match) return match[1].trim();
  const bare = text.match(/\b(\d{3}-[^\s)]+\.md)\b/);
  return bare ? bare[1] : null;
}

/**
 * Run tracker.mjs delete, in dry-run or live mode.
 *
 * @param {string} n
 * @param {boolean} dryRun
 */
async function runDelete(n, dryRun) {
  const script = rootScript("tracker");
  if (!fs.existsSync(script)) {
    throw new BridgeError(
      "core-script-missing",
      "removing a tracker row needs the career-ops scripts; this root has data only",
    );
  }
  const args = ["delete", "--num", n];
  if (dryRun) args.push("--dry-run");
  const res = await runNode(process.execPath, [script, ...args], {
    cwd: careerOpsRoot(),
    timeout: SCRIPT_TIMEOUT_MS,
  });
  if (res.spawnFailed) {
    throw new BridgeError("spawn-failed", "the tracker removal did not run");
  }
  // tracker.mjs prints its report on STDERR, so both streams are read.
  const output = `${res.stdout}\n${res.stderr}`;
  if (res.code !== 0) {
    const message = /not found|no tracker row/i.test(output)
      ? `no tracker row matches #${n}`
      : "the tracker row could not be removed";
    throw new BridgeError(res.code === 1 ? "not-found" : "failed", message);
  }
  return {
    orphanedReport: orphanedReportFrom(output),
    output: output.trim(),
  };
}

/** The next free tracker number, from the rows the client can already see. */
function nextTrackerNumber() {
  const rows = readApplications();
  let max = 0;
  for (const row of rows) {
    const n = Number.parseInt(String(row.n ?? "").trim(), 10);
    if (Number.isFinite(n) && n > max) max = n;
  }
  return max + 1;
}

/** A filesystem-safe slug for a file name. */
function slug(text) {
  return (
    String(text ?? "")
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 48) || "application"
  );
}

/**
 * @param {{action?: string, n?: string, query?: string, row?: Record<string, unknown>}} params
 */
export async function handle(params) {
  const action = params.action ?? "find";

  if (action === "find") {
    const query = typeof params.query === "string" ? params.query.trim() : "";
    if (!query) throw new BridgeError("bad-request", "query required");
    const script = rootScript("find");
    if (!fs.existsSync(script)) {
      throw new BridgeError(
        "core-script-missing",
        "searching the tracker needs the career-ops scripts; this root has data only",
      );
    }
    const res = await runNode(process.execPath, [script, "--json", query], {
      cwd: careerOpsRoot(),
      timeout: SCRIPT_TIMEOUT_MS,
    });
    const parsed = parseJsonOutput(res.stdout);
    return {
      query,
      matches: Array.isArray(parsed) ? parsed : [],
      found: Array.isArray(parsed) ? parsed.length : 0,
    };
  }

  if (action === "deletePreview" || action === "delete") {
    const n = String(params.n ?? "").trim();
    if (!/^\d+$/.test(n)) throw new BridgeError("bad-request", "n must be a tracker row number");
    const result = await runDelete(n, action === "deletePreview");
    return { ok: true, n, ...result, dryRun: action === "deletePreview" };
  }

  if (action === "add") {
    const row = params.row ?? {};
    const company = String(row.company ?? "").trim();
    const role = String(row.role ?? "").trim();
    if (!company) throw new BridgeError("bad-request", "company required");
    const status = String(row.status ?? "Evaluated").trim() || "Evaluated";
    const date = String(row.date ?? "").trim();
    if (!date) throw new BridgeError("bad-request", "date required (YYYY-MM-DD)");
    if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) {
      throw new BridgeError("bad-request", "date must be YYYY-MM-DD");
    }
    const score = String(row.score ?? "N/A").trim() || "N/A";
    if (!["N/A", "—", "-"].includes(score)) {
      throw new BridgeError(
        "bad-request",
        "a hand-added row has no evaluation, so its score must be the N/A sentinel",
      );
    }

    const root = careerOpsRoot();
    const num = nextTrackerNumber();
    // The TSV carries a HEADER, so merge-tracker.mjs resolves every field by
    // name and the column order carries no meaning.
    const header = ["num", "date", "company", "role", "score", "status", "pdf", "report", "notes", "url"];
    const values = [
      String(num),
      date,
      company,
      role,
      score,
      status,
      "❌",
      "—",
      String(row.notes ?? "").trim(),
      String(row.url ?? "").trim(),
    ];
    const tsv = `${header.join("\t")}\n${values.join("\t")}\n`;
    const dir = path.join(root, "batch", "tracker-additions");
    fs.mkdirSync(dir, { recursive: true });
    const file = path.join(dir, `${num}-${slug(company)}.tsv`);
    fs.writeFileSync(file, tsv, "utf8");

    const script = rootScript("merge-tracker");
    if (!fs.existsSync(script)) {
      throw new BridgeError(
        "core-script-missing",
        "adding a tracker row needs the career-ops scripts; this root has data only",
      );
    }
    const res = await runNode(process.execPath, [script], {
      cwd: careerOpsRoot(),
      timeout: SCRIPT_TIMEOUT_MS,
    });
    if (res.code !== 0) {
      throw new BridgeError("failed", "the tracker row could not be merged");
    }
    // The merge is the writer; the tracker is read back to say which number it
    // actually gave the row.
    const rows = readApplications();
    const match = rows.find(
      (r) => String(r.company ?? "").trim() === company && String(r.role ?? "").trim() === role,
    );
    return {
      ok: true,
      num: match?.n ? Number.parseInt(match.n, 10) : num,
      company,
      role,
      status,
      score,
    };
  }

  throw new BridgeError("bad-request", `unknown tracker action: ${action}`);
}

export const op = "tracker";
