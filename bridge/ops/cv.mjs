// cv — reading and writing cv.md.
//
// The write goes through atomicWriteWithBackup, the same helper the web app's
// POST /api/cv uses: a temp file renamed over the target, with a timestamped
// .bak snapshot of the previous content. A half-written CV is the one failure
// a user cannot recover from, so the write is never a plain overwrite.
import fs from "node:fs";
import path from "node:path";
import { careerOpsRoot, atomicWriteWithBackup } from "../engine.mjs";
import { BridgeError } from "../util.mjs";

// The web route's cap. cv.md is a markdown document a person edits; 200 KB is
// far past any real one and far below anything that would wedge the editor.
const MAX_CV_BYTES = 200 * 1024;

function cvPath() {
  return path.join(careerOpsRoot(), "cv.md");
}

/**
 * @param {{action?: string, content?: string}} params
 */
export async function handle(params) {
  const action = params.action ?? "read";
  const file = cvPath();

  if (action === "read") {
    let content = "";
    let exists = false;
    try {
      content = fs.readFileSync(file, "utf8");
      exists = true;
    } catch (err) {
      if (err?.code !== "ENOENT") throw err;
    }
    return { content, exists, path: file };
  }

  if (action === "write") {
    const content = typeof params.content === "string" ? params.content : null;
    if (content === null) throw new BridgeError("bad-request", "content required");
    if (Buffer.byteLength(content, "utf8") > MAX_CV_BYTES) {
      throw new BridgeError("too-large", `cv.md is over the ${MAX_CV_BYTES} byte limit`);
    }
    const backedUp = atomicWriteWithBackup(file, content);
    return { ok: true, backedUp, path: file };
  }

  throw new BridgeError("bad-request", `unknown cv action: ${action}`);
}

export const op = "cv";
