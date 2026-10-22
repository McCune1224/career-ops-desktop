// interview — the preparation half of the loop: the story bank, the match of a
// story to a question, and the week's digest.
//
// The story bank is read, never written, by this client: a story enters it
// through the CLI's own modes, and story-provenance-check.mjs decides which of
// its numbers may be restated as fact. The app shows that verdict rather than
// deciding it.
import fs from "node:fs";
import path from "node:path";

import { careerOpsRoot } from "../engine.mjs";
import { runCoreScript } from "../core-script.mjs";
import { BridgeError } from "../util.mjs";

/**
 * @param {{action?: string, question?: string, top?: number, from?: string, to?: string}} params
 */
export async function handle(params) {
  const action = params.action ?? "storyBank";
  const root = careerOpsRoot();

  if (action === "storyBank") {
    const bank = path.join(root, "interview-prep", "story-bank.md");
    let text = "";
    let exists = false;
    try {
      text = fs.readFileSync(bank, "utf8");
      exists = true;
    } catch (err) {
      if (err?.code !== "ENOENT") throw err;
    }
    const res = await runCoreScript("story-provenance-check", [], { json: true, timeout: 60_000 });
    const data = res.data ?? {};
    return {
      available: res.code === 0,
      exists,
      text,
      provenance: {
        existing: data.existing ?? [],
        supportedByResume: data.supportedByResume ?? [],
        derivedUnverified: data.derivedUnverified ?? [],
        userCannotConfirm: data.userCannotConfirm ?? [],
        malformed: data.malformed ?? [],
        lowConfidence: data.lowConfidence ?? null,
      },
      output: res.output,
    };
  }

  if (action === "matchStar") {
    const question = String(params.question ?? "").trim();
    if (!question) throw new BridgeError("bad-request", "question required");
    const args = [question];
    if (Number.isFinite(params.top) && Number(params.top) > 0) {
      args.push("--top", String(Math.round(Number(params.top))));
    }
    const res = await runCoreScript("match-star", args, { timeout: 60_000 });
    if (res.code !== 0) {
      throw new BridgeError("not-found", res.output || "no story matched that question");
    }
    return { ok: true, question, story: res.stdout.trim() };
  }

  if (action === "digest") {
    const args = [];
    if (params.from) args.push("--from", String(params.from));
    if (params.to) args.push("--to", String(params.to));
    const res = await runCoreScript("weekly-digest", args, { json: true, timeout: 60_000 });
    const data = res.data ?? {};
    return {
      available: res.code === 0,
      metadata: data.metadata ?? null,
      companies: data.companies ?? [],
      competencyTagCounts: data.competencyTagCounts ?? [],
      recurringCompetencies: data.recurringCompetencies ?? [],
      recurringGaps: data.recurringGaps ?? [],
      output: res.output,
    };
  }

  if (action === "sessions") {
    const dir = path.join(root, "interview-prep", "sessions");
    let names = [];
    try {
      names = fs.readdirSync(dir).filter((n) => n.endsWith(".md")).sort();
    } catch {
      return { sessions: [] };
    }
    return {
      sessions: names.map((name) => {
        let company = "";
        try {
          const text = fs.readFileSync(path.join(dir, name), "utf8");
          company = (text.match(/^company:\s*(.+)$/m)?.[1] ?? "").trim();
        } catch {
          /* unreadable session is listed without a company */
        }
        return { name, company };
      }),
    };
  }

  throw new BridgeError("bad-request", `unknown interview action: ${action}`);
}

export const op = "interview";
