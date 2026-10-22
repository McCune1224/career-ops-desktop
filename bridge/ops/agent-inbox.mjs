// agent-inbox — a checklist of requests for later.
//
// agent-inbox.mjs owns the file and its lock. The client appends an item, lists
// the checklist and resolves one; nothing is ever submitted from here, and
// nothing runs on its own.

import fs from "node:fs";
import path from "node:path";

import { careerOpsRoot } from "../engine.mjs";
import { runCoreScript } from "../core-script.mjs";
import { BridgeError } from "../util.mjs";

/**
 * @param {{action?: string, request?: string, num?: number, result?: string}} params
 */
export async function handle(params) {
  const action = params.action ?? "list";

  if (action === "add") {
    const request = String(params.request ?? "").trim();
    if (!request) throw new BridgeError("bad-request", "request required");
    const res = await runCoreScript("agent-inbox", ["add", request], { timeout: 30_000 });
    if (res.code !== 0) {
      throw new BridgeError("failed", "the request could not be queued");
    }
    return { ok: true, output: res.output };
  }

  if (action === "list") {
    const res = await runCoreScript("agent-inbox", ["list"], { timeout: 30_000 });
    const file = inboxFile();
    let items = [];
    try {
      items = parseChecklist(fs.readFileSync(file, "utf8"));
    } catch {
      items = [];
    }
    return { ok: res.code === 0, items, output: res.output };
  }

  if (action === "resolve") {
    const num = Number(params.num);
    if (!Number.isFinite(num)) throw new BridgeError("bad-request", "num required");
    const args = ["resolve", String(num)];
    if (params.result) args.push("--result", String(params.result));
    const res = await runCoreScript("agent-inbox", args, { timeout: 30_000 });
    if (res.code !== 0) {
      throw new BridgeError("failed", res.output || "the item could not be resolved");
    }
    return { ok: true, num, output: res.output };
  }

  throw new BridgeError("bad-request", `unknown agent-inbox action: ${action}`);
}

/**
 * The inbox file, as the core resolves it.
 *
 * agent-inbox.mjs writes relative to the WORKING DIRECTORY unless
 * CAREER_OPS_INBOX says otherwise, so the client resolves it the same way and
 * passes the path through the environment.
 */
function inboxFile() {
  return (
    process.env.CAREER_OPS_INBOX || path.join(careerOpsRoot(), "data", "agent-inbox.md")
  );
}

/**
 * The checklist, read from the file's own grammar.
 *
 * The core's file is a markdown checklist; this reads the boxes and the text
 * so the screen can show it. The core remains the writer.
 *
 * @param {string} text
 * @returns {Array<{num: number, done: boolean, text: string}>}
 */
export function parseChecklist(text) {
  const items = [];
  let num = 0;
  for (const line of String(text ?? "").split("\n")) {
    const match = line.match(/^\s*[-*]\s*\[( |x|X)\]\s*(.*)$/);
    if (!match) continue;
    num += 1;
    items.push({
      num,
      done: match[1].toLowerCase() === "x",
      text: match[2].trim(),
    });
  }
  return items;
}

export const op = "agent-inbox";
