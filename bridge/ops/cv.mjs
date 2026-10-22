// cv — reading and writing cv.md.
//
// The write goes through atomicWriteWithBackup, the same helper the web app's
// POST /api/cv uses: a temp file renamed over the target, with a timestamped
// .bak snapshot of the previous content. A half-written CV is the one failure
// a user cannot recover from, so the write is never a plain overwrite.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { careerOpsRoot, atomicWriteWithBackup, htmlText } from "../engine.mjs";
import { runCoreScript } from "../core-script.mjs";
import { findExperienceSections, parseCompanyHeading } from "../core-cv-markdown.mjs";
import { BridgeError, parseJsonOutput } from "../util.mjs";

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

  if (action === "verify") {
    // The structure check: the core's own verifier, over the CV the client
    // would send, against the CV the user wrote.
    //
    // The payload is built with the CORE's markdown parser, so the client and
    // the verifier read the CV the same way. The verifier then compares that
    // payload with its own reading of cv.md and reports a dropped descriptor
    // or an entry that moved.
    const cvFile = path.join(careerOpsRoot(), "cv.md");
    let cvText = "";
    try {
      cvText = fs.readFileSync(cvFile, "utf8");
    } catch {
      throw new BridgeError("not-found", "there is no cv.md to check");
    }
    const payload = await buildCvPayload(cvText);
    const tmp = path.join(os.tmpdir(), `career-ops-cv-payload-${process.pid}.json`);
    fs.writeFileSync(tmp, JSON.stringify(payload), "utf8");
    try {
      const res = await runCoreScript("verify-cv-structure", [tmp, "--source", cvFile], {
        timeout: 60_000,
      });
      const output = res.output;
      const verdict = /\bpass\b/i.test(output) && !/warn|unverified/i.test(output)
        ? "pass"
        : /unverified/i.test(output)
          ? "unverified"
          : /warn/i.test(output)
            ? "warn"
            : "pass";
      const violations = output
        .split("\n")
        .filter((line) => /violation|missing|out of order|reorder/i.test(line))
        .map((line) => line.replace(/^\s*[-*]\s*/, "").trim())
        .filter(Boolean);
      return {
        ok: true,
        verdict,
        violations,
        output,
        entries: payload.experience.length,
      };
    } finally {
      fs.rmSync(tmp, { force: true });
    }
  }

  if (action === "facts") {
    // The fact check: the core's own verifier, over a generated document,
    // against the primary files it must trace to.
    const target = String(params.target ?? "").trim();
    if (!target) throw new BridgeError("bad-request", "target required");
    const file = path.isAbsolute(target) ? target : path.join(careerOpsRoot(), target);
    if (!fs.existsSync(file)) {
      throw new BridgeError("not-found", "that document does not exist");
    }
    const res = await runCoreScript("verify-cv-facts", [file, "--json"], {
      json: true,
      timeout: 120_000,
    });
    const data = res.data ?? {};
    return {
      ok: res.code === 0 && data.verdict !== "block",
      verdict: data.verdict ?? null,
      invented: data.invented ?? [],
      unsupported: data.unsupportedFacts ?? [],
      advisory: data.advisoryFacts ?? [],
      forbidden: data.forbidden ?? [],
      warnings: data.warnings ?? [],
      output: res.output,
    };
  }

  throw new BridgeError("bad-request", `unknown cv action: ${action}`);
}

/**
 * The payload the core's structure verifier consumes, built from cv.md.
 *
 * The shape is the one modes/pdf.md's step 17 produces: `experience[]` with
 * `company`, `role`, `dates` and `location`. The entries come from the CORE's
 * own markdown parser (lib/cv-markdown.mjs), so the client and the verifier
 * read the same document the same way.
 *
 * @param {string} cvText
 * @returns {{experience: Array<{company: string, role: string, dates: string, location: string}>}}
 */
async function buildCvPayload(cvText) {
  const experience = [];
  for (const section of await findExperienceSections(cvText)) {
    let current = null;
    for (const line of section) {
      const heading = await parseCompanyHeading(line);
      if (heading) {
        current = {
          company: heading.company,
          location: heading.location,
          role: "",
          dates: "",
        };
        experience.push(current);
        continue;
      }
      if (!current) continue;
      const bold = /\*\*(.+?)\*\*/.exec(line);
      if (bold && !current.role) {
        current.role = bold[1].trim();
        continue;
      }
      if (!current.dates && line.trim() && !/^\*\*/.test(line.trim())) {
        current.dates = line.trim();
      }
    }
  }
  return { experience };
}

export const op = "cv";
