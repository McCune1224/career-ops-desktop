// bootstrap — what the desktop app needs before it can show anything.
//
// Mirrors what the web app assembles from GET /api/doctor, GET /api/clis and
// GET /api/report/shape, so both clients answer "is this install ready" the
// same way.
import fs from "node:fs";
import path from "node:path";
import { runNode } from "../util.mjs";
import {
  careerOpsRoot,
  rootScript,
  canonicalLabels,
  detectClis,
  pickDefaultInstalled,
  doctorState,
  pipelineSummary,
  readFollowupsEnabled,
} from "../engine.mjs";

const DOCTOR_TIMEOUT_MS = 10_000;

/**
 * doctor.mjs --json, the same call the web's /api/doctor makes.
 *
 * A checkout without the script is not an error: the bridge reports
 * `available: false` and the screen says so, rather than failing to open.
 */
async function readDoctor() {
  const script = rootScript("doctor");
  if (!fs.existsSync(script)) {
    return { available: false, onboardingNeeded: false, missing: [], warnings: [] };
  }
  const res = await runNode(process.execPath, [script, "--json"], {
    cwd: careerOpsRoot(),
    timeout: DOCTOR_TIMEOUT_MS,
  });
  if (res.code !== 0) {
    return { available: false, onboardingNeeded: false, missing: [], warnings: [] };
  }
  try {
    return { available: true, ...JSON.parse(res.stdout) };
  } catch {
    return { available: false, onboardingNeeded: false, missing: [], warnings: [] };
  }
}

/**
 * @param {Record<string, unknown>} params
 * @param {{emit: (event: unknown) => void}} ctx
 */
export async function handle(params, ctx) {
  const root = careerOpsRoot();
  const summary = pipelineSummary();
  const state = doctorState();
  const reportsDir = path.join(root, "reports");
  const outputDir = path.join(root, "output");
  const count = (dir, ext) => {
    try {
      return fs.readdirSync(dir).filter((f) => f.endsWith(ext)).length;
    } catch {
      return 0;
    }
  };

  const clis = detectClis();
  return {
    dataRoot: root,
    codeRoot: path.dirname(rootScript("doctor")),
    doctor: await readDoctor(),
    doctorState: state,
    clis,
    // Which installed CLI to drive when the app has no usable preference.
    // Resolved by the web app's own picker, so "the one to offer" is one rule
    // shared by both clients rather than a second opinion held here.
    defaultCliId: pickDefaultInstalled(clis),
    states: canonicalLabels(),
    trackerExists: fs.existsSync(path.join(root, "data", "applications.md")),
    inboxCount: Array.isArray(summary?.inbox) ? summary.inbox.length : 0,
    applicationCount: Array.isArray(summary?.applications) ? summary.applications.length : 0,
    reports: count(reportsDir, ".md"),
    pdfs: count(outputDir, ".pdf"),
    followupsEnabled: readFollowupsEnabled(),
    cvExists: fs.existsSync(path.join(root, "cv.md")),
  };
}

export const op = "bootstrap";
