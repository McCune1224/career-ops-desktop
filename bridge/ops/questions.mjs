// questions — the screening questionnaire for one application.
//
// application-answers.mjs owns the `## Application Answers` section inside the
// report: it writes it, reads it back, and keeps the draft. The client reads
// the prepared pack and shows it; it never fills in a form and never sends
// anything.

import fs from "node:fs";
import path from "node:path";

import { careerOpsRoot, findReportFile } from "../engine.mjs";
import { runCoreScript } from "../core-script.mjs";
import { BridgeError } from "../util.mjs";

/**
 * @param {{action?: string, n?: string, file?: string}} params
 */
export async function handle(params) {
  const action = params.action ?? "read";

  if (action === "read") {
    const file = String(params.file ?? "").trim() || reportFileFor(params.n);
    const res = await runCoreScript("application-answers", ["--report", file, "--read"], {
      json: true,
      timeout: 60_000,
    });
    const data = res.data;
    return {
      available: res.code === 0,
      report: path.basename(file),
      answers: data ?? null,
      output: res.output,
    };
  }

  if (action === "readDraft") {
    const file = String(params.file ?? "").trim() || reportFileFor(params.n);
    const res = await runCoreScript("application-answers", ["--report", file, "--read-draft"], {
      json: true,
      timeout: 60_000,
    });
    return {
      available: res.code === 0,
      report: path.basename(file),
      draft: res.data ?? null,
      output: res.output,
    };
  }

  if (action === "prepare") {
    // prepare-application.mjs reads the form's fields from a URL and the
    // profile, and NEVER posts anything. It needs a PDF inside output/.
    const url = String(params.url ?? "").trim();
    const pdf = String(params.pdf ?? "").trim();
    if (!url || !pdf) {
      throw new BridgeError("bad-request", "url and pdf are required");
    }
    const root = careerOpsRoot();
    const args = ["--url", url, "--pdf", pdf];
    if (params.cover) args.push("--cover", String(params.cover));
    const res = await runCoreScript("prepare-application", args, { timeout: 60_000 });
    if (res.code !== 0) {
      throw new BridgeError(
        "bad-request",
        res.output || "the form could not be prepared",
      );
    }
    return {
      available: true,
      // Never a submission: the core script has no submit path at all.
      submitted: false,
      output: res.output,
    };
  }

  throw new BridgeError("bad-request", `unknown questions action: ${action}`);
}

/**
 * The report file for one application, or a refusal.
 */
function reportFileFor(n) {
  const found = findReportFile(String(n ?? "").trim());
  if (!found) {
    throw new BridgeError("not-found", `no report for application ${n}`);
  }
  return found;
}

export const op = "questions";
