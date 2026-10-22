// outcomes — the end of an application: what happened, what it orphaned, and
// the one thing the user may share about it.
//
// outcome.mjs is the core's own: it archives the posting and the CV, syncs
// the tracker through set-status.mjs, and writes the outcome journal. The
// hired-share draft is the core's own too, and it never submits anything —
// the draft is shown in full and the user decides.
import fs from "node:fs";

import { rootScript } from "../engine.mjs";
import { runCoreScript } from "../core-script.mjs";
import { BridgeError } from "../util.mjs";

/** The outcome kinds outcome.mjs accepts. */
const OUTCOME_TYPES = [
  "interview_progress",
  "offer_received",
  "hired",
  "offer_declined",
  "rejected",
  "no_response",
  "interview_only",
];

/**
 * @param {{action?: string, n?: string, company?: string, type?: string, note?: string, stage?: string, url?: string, report?: number, anonymity?: string, dryRun?: boolean}} params
 */
export async function handle(params) {
  const action = params.action ?? "record";

  if (action === "record") {
    const type = String(params.type ?? "").trim();
    if (!OUTCOME_TYPES.includes(type)) {
      throw new BridgeError("bad-request", `type must be one of: ${OUTCOME_TYPES.join(", ")}`);
    }
    const selector = String(params.n ?? params.company ?? "").trim();
    if (!selector) throw new BridgeError("bad-request", "a report number or a company is required");
    const args = [selector, type, "--json"];
    if (params.stage) args.push("--stage", String(params.stage));
    if (params.note) args.push("--note", String(params.note));
    if (params.url) args.push("--url", String(params.url));
    if (params.dryRun === true) args.push("--dry-run");
    const res = await runCoreScript("outcome", args, { json: true, timeout: 180_000 });
    const data = res.data ?? {};
    if (res.code !== 0 || data?.error) {
      throw new BridgeError(
        res.code === 2 ? "not-found" : "failed",
        data?.error ?? "the outcome could not be recorded",
      );
    }
    return {
      ok: true,
      num: data.num ?? null,
      company: data.company ?? null,
      role: data.role ?? null,
      outcomeType: data.outcomeType ?? type,
      canonicalState: data.canonicalState ?? null,
      outcomeDir: data.outcomeDir ?? null,
      postingArchived: data.postingArchived ?? false,
      dryRun: params.dryRun === true,
      output: res.output,
    };
  }

  if (action === "hiredStatus") {
    const res = await runCoreScript("hired-share", ["--status"], { json: true, timeout: 60_000 });
    const data = res.data ?? {};
    return {
      available: res.code === 0,
      hires: data.hires ?? [],
      askable: data.askable ?? [],
      output: res.output,
    };
  }

  if (action === "hiredShare") {
    const report = Number(params.report);
    if (!Number.isFinite(report)) throw new BridgeError("bad-request", "report number required");
    const args = ["--report", String(report), "--dry-run"];
    if (params.anonymity) args.push("--anonymity", String(params.anonymity));
    if (params.note) args.push("--story", String(params.note));
    const res = await runCoreScript("hired-share", args, { timeout: 60_000 });
    return {
      ok: res.code === 0,
      draft: res.output,
      // Nothing leaves this machine: the draft is text the user copies.
      submitted: false,
      output: res.output,
    };
  }

  throw new BridgeError("bad-request", `unknown outcomes action: ${action}`);
}

export const op = "outcomes";
