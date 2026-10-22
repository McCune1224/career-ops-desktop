// inbox — the pending-posting queue in data/pipeline.md.
//
// "Skip" flips one Pending row's checkbox. The write goes through the web
// app's own inbox-skip.mjs under the core's pipeline lock, which rewrites the
// checkbox byte only — every other byte of the file is preserved. The desktop
// client does not have its own pipeline writer.
import fs from "node:fs";
import path from "node:path";
import { careerOpsRoot, setInboxSkip } from "../engine.mjs";
import { BridgeError } from "../util.mjs";

/**
 * The error codes inbox-skip.mjs answers, said in the user's terms.
 *
 * Same mapping the web route applies. `busy` is transient — the CLI or another
 * client holds the pipeline lock — so it keeps its own code and the screen can
 * offer a retry rather than implying the write was rejected.
 */
const ERROR_MESSAGES = {
  "invalid-url": "that is not an http(s) posting URL",
  unmatched: "no pipeline row for that URL",
  "not-found": "data/pipeline.md was not found",
  busy: "the pipeline is being written right now — try again",
};

const ERROR_CODES = {
  "invalid-url": "bad-request",
  unmatched: "not-found",
  "not-found": "not-found",
  busy: "busy",
};

/**
 * @param {{url?: string, done?: boolean}} params
 */
export async function handle(params) {
  const url = typeof params.url === "string" ? params.url.trim() : "";
  if (!url) throw new BridgeError("bad-request", "url required");
  if (params.done !== undefined && typeof params.done !== "boolean") {
    throw new BridgeError("bad-request", "done must be a boolean");
  }
  const done = params.done !== false;

  const root = careerOpsRoot();
  const pipelinePath = path.join(root, "data", "pipeline.md");
  // The lock module lives beside the scripts, in the DATA root. Without it the
  // write would be unguarded against the CLI's own pipeline writer, so its
  // absence is a hard refusal rather than a silent unguarded write — the same
  // decision the web route makes.
  const lockModule = path.join(root, "pipeline-lock.mjs");
  if (!fs.existsSync(lockModule)) {
    throw new BridgeError(
      "core-script-missing",
      "skipping a pending posting needs the career-ops scripts; this root has data only",
    );
  }

  const result = await setInboxSkip(pipelinePath, url, done, { lockModule });
  if (!result.ok) {
    const code = result.error;
    throw new BridgeError(ERROR_CODES[code] ?? "failed", ERROR_MESSAGES[code] ?? "the skip failed");
  }
  return { ok: true, url, done, matched: result.matched === true, changed: result.changed };
}

export const op = "inbox";
