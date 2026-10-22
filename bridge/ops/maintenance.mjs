// maintenance — the tracker's and the pipeline's own health and repair tools.
//
// Every one of these is a CORE script, called with the core's own flags and
// reported with the core's own output. The desktop client adds no rule of its
// own: it decides which script to run, and it shows what the script said.
//
//   verify        verify-pipeline.mjs   the whole health check, one command
//   normalize     normalize-statuses.mjs  non-canonical states → canonical
//   dedup         dedup-tracker.mjs     duplicate rows removed
//   fixLinks      fix-report-links.mjs  a dead Report cell → "—"
//   reconcile     reconcile-pipeline.mjs pipeline.md vs the batch state
//   jdArchive     check-jd-archive.mjs  every report kept its JD
//   pdfFlags      sync-pdf-flags.mjs    the PDF column matches the disk
//   fixSlugs      fix-slugs.mjs         malformed portal slugs
//
// The scripts that write take the core's own lock and leave the core's own
// .bak. `--dry-run` is offered for every writer, and the app's default for
// each one is the dry run: a repair is a change to the user's tracker, and the
// user sees the proposal before it happens.
import fs from "node:fs";

import { careerOpsRoot, rootScript } from "../engine.mjs";
import { runNode, BridgeError, parseJsonOutput } from "../util.mjs";

const SCRIPT_TIMEOUT_MS = 120_000;

/**
 * The scripts this op knows how to run, and how each one is asked for its
 * output.
 *
 * `json: true` means the script prints a JSON document; `text: true` means it
 * prints prose the screen shows verbatim.
 */
const TOOLS = {
  verify: { script: "verify-pipeline", json: false, text: true, writes: false },
  normalize: { script: "normalize-statuses", json: false, text: true, writes: true },
  dedup: { script: "dedup-tracker", json: false, text: true, writes: true },
  fixLinks: { script: "fix-report-links", json: false, text: true, writes: true },
  reconcile: { script: "reconcile-pipeline", json: false, text: true, writes: true },
  jdArchive: { script: "check-jd-archive", json: true, text: false, writes: false },
  pdfFlags: { script: "sync-pdf-flags", json: true, text: true, writes: true },
  fixSlugs: { script: "fix-slugs", json: false, text: true, writes: true },
};

/**
 * @param {{tool?: string, apply?: boolean}} params
 */
export async function handle(params) {
  const tool = String(params.tool ?? "verify");
  const spec = TOOLS[tool];
  if (!spec) throw new BridgeError("bad-request", `unknown maintenance tool: ${tool}`);

  const script = rootScript(spec.script);
  if (!fs.existsSync(script)) {
    throw new BridgeError(
      "core-script-missing",
      `${tool} needs the career-ops scripts; this root has data only`,
    );
  }

  const args = [];
  if (spec.json) args.push("--json");
  if (spec.writes) {
    // Every writer is offered as a proposal first. `apply` is the user's
    // explicit yes, never a default.
    if (!params.apply) args.push("--dry-run");
  }
  if (tool === "fixSlugs" && params.apply) args.push("--fix");

  const res = await runNode(process.execPath, [script, ...args], {
    cwd: careerOpsRoot(),
    timeout: SCRIPT_TIMEOUT_MS,
  });
  if (res.spawnFailed) {
    throw new BridgeError("spawn-failed", `${tool} did not run`);
  }

  const output = `${res.stdout}\n${res.stderr}`.trim();
  const data = spec.json ? parseJsonOutput(res.stdout) : null;

  return {
    tool,
    apply: params.apply === true,
    ok: res.code === 0,
    exitCode: res.code,
    output,
    data,
  };
}

export const op = "maintenance";
