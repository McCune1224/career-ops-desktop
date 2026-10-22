// documents — the generated artefacts: the tailored CVs and cover letters, and
// the archived postings.
//
// The list comes from the web app's own discoverDocuments(), which reads
// output/ and the pdf-index manifest and classifies each file. The archive is
// the core's own archive-posting.mjs, which needs a browser and a network —
// so the app asks before it spends either.
import fs from "node:fs";
import path from "node:path";

import { careerOpsRoot, discoverDocuments, encodeId, decodeId } from "../engine.mjs";
import { runCoreScript } from "../core-script.mjs";
import { BridgeError } from "../util.mjs";

/** The manifest the web app's document list is built on. */
function manifestPath(root) {
  return path.join(root, "data", "pdf-index.tsv");
}

/** True when the path is a regular file inside the workspace. */
function isRegularContainedFile(root, target) {
  try {
    const real = fs.realpathSync(target);
    const base = fs.realpathSync(root);
    if (!real.startsWith(base + path.sep)) return false;
    return fs.statSync(real).isFile();
  } catch {
    return false;
  }
}

/**
 * @param {{action?: string, id?: string, filename?: string, url?: string, company?: string, role?: string, report?: string, dryRun?: boolean}} params
 */
export async function handle(params) {
  const action = params.action ?? "list";
  const root = careerOpsRoot();

  if (action === "list") {
    const documents = discoverDocuments({
      outputDir: path.join(root, "output"),
      manifestPath: manifestPath(root),
      workspaceRoot: root,
      isRegularContainedFile: (target) => isRegularContainedFile(root, target),
    });
    return {
      documents: documents.map((doc) => ({
        filename: doc.filename,
        kind: doc.kind,
        date: doc.date,
        reportId: doc.reportId,
        format: doc.format,
        id: encodeId(doc.filename),
      })),
      count: documents.length,
    };
  }

  if (action === "resolve") {
    const filename = params.filename
      ? String(params.filename)
      : decodeId(String(params.id ?? ""));
    if (!filename) throw new BridgeError("bad-request", "id or filename required");
    const target = path.join(root, "output", filename);
    if (!isRegularContainedFile(root, target)) {
      throw new BridgeError("not-found", "that document is not in output/");
    }
    return { ok: true, filename, path: target };
  }

  if (action === "archive") {
    const url = String(params.url ?? "").trim();
    if (!/^https?:\/\//i.test(url)) {
      throw new BridgeError("bad-request", "a posting URL is required");
    }
    const args = [url];
    if (params.company) args.push("--company", String(params.company));
    if (params.role) args.push("--role", String(params.role));
    if (params.report) args.push("--report", String(params.report));
    if (params.dryRun !== false) args.push("--dry-run");
    const res = await runCoreScript("archive-posting", args, { timeout: 180_000 });
    return {
      ok: res.code === 0,
      output: res.output,
      dryRun: params.dryRun !== false,
    };
  }

  throw new BridgeError("bad-request", `unknown documents action: ${action}`);
}

export const op = "documents";
