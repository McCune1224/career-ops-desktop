// cost — what a run will cost, and what has already been spent.
//
// The estimate is the web app's own: estimateRunCost() takes the user's recent
// COMPLETED runs of the same kind and reports the median token count and
// median dollar cost. A median, so one unusually large agent run does not
// inflate every later warning, and an empty history is an empty estimate with
// an honest fallback rather than a made-up number.
//
// The samples come from this client's own saved run logs (see runs.mjs), so the
// estimate is about the runs this client actually made.

import fs from "node:fs";
import path from "node:path";

import { careerOpsRoot } from "../engine.mjs";
import { estimateRunCost, formatRunCostEstimate, requiresBatchSpendConfirmation, formatBatchSpendConfirmation } from "../engine.mjs";
import { BridgeError } from "../util.mjs";

/** The window a spend sensor reports, in hours. */
const WINDOWS = [
  ["window5h", 5],
  ["window7d", 24 * 7],
];

/** Where this client keeps its run records. */
export function runsDir() {
  return path.join(careerOpsRoot(), "data", "runs");
}

/**
 * Every saved run, newest first.
 *
 * @returns {Array<Record<string, unknown>>}
 */
export function readRunRecords() {
  const dir = runsDir();
  let names = [];
  try {
    names = fs.readdirSync(dir).filter((n) => n.endsWith(".json"));
  } catch {
    return [];
  }
  const records = [];
  for (const name of names) {
    try {
      const parsed = JSON.parse(fs.readFileSync(path.join(dir, name), "utf8"));
      if (parsed && typeof parsed === "object") records.push(parsed);
    } catch {
      /* a half-written record is skipped, not fatal */
    }
  }
  records.sort((a, b) => Number(b.startedAt ?? 0) - Number(a.startedAt ?? 0));
  return records;
}

/**
 * @param {{action?: string, kind?: string, count?: number}} params
 */
export async function handle(params) {
  const action = params.action ?? "estimate";

  if (action === "estimate") {
    const kind = String(params.kind ?? "evaluate");
    const count = Number.isFinite(params.count) && Number(params.count) > 0 ? Number(params.count) : 1;
    const jobs = readRunRecords().map((record) => ({
      kind: record.kind,
      status: record.status,
      startedAt: record.startedAt,
      cost: { tokens: record.tokens, usd: record.usd },
    }));
    const estimate = estimateRunCost(jobs, kind, count);
    return {
      kind,
      count,
      estimate,
      text: formatRunCostEstimate(estimate, "no estimate yet — this is the first run of its kind"),
      requiresConfirmation: requiresBatchSpendConfirmation(count),
      confirmation: requiresBatchSpendConfirmation(count)
        ? formatBatchSpendConfirmation(`Run ${count} workers`, count, estimate)
        : null,
    };
  }

  if (action === "usage") {
    const records = readRunRecords();
    const now = Date.now();
    const windows = WINDOWS.map(([name, hours]) => {
      const cutoff = now - hours * 3600 * 1000;
      const inWindow = records.filter((r) => Number(r.startedAt ?? 0) >= cutoff);
      return {
        name,
        hours,
        runs: inWindow.length,
        tokens: inWindow.reduce((total, r) => total + (Number(r.tokens) || 0), 0),
        usd: Number(inWindow.reduce((total, r) => total + (Number(r.usd) || 0), 0).toFixed(4)),
      };
    });
    return { windows, computedAt: now, runs: records.length };
  }

  throw new BridgeError("bad-request", `unknown cost action: ${action}`);
}

export const op = "cost";
