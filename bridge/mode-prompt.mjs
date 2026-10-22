// Running a career-ops MODE from the client.
//
// The engine's buildPrompt() knows five kinds: evaluate, pdf, cover, research
// and fix-portal. The desktop client offers more — an interview prep pack, a
// practice round, a debrief, an offer walk-through, an application email — and
// the engine has no prompt for them.
//
// So the prompt is built HERE, and the mode file in the career-ops checkout
// stays the rule: the prompt names the file and tells the agent to follow it
// exactly. Nothing about the mode's content is restated, so nothing can drift
// from the mode.
//
// Everything else about the run is the engine's, unchanged: the CLI is
// resolved by resolveCliOrFallback(), the capabilities come from
// capabilitiesFor(), the argv is built by claude-invocation.mjs /
// run-cli-support.mjs, the spawn is spawnHeadlessCli() with the fence from
// cli-fencing.mjs, the stream is parsed by run-cli-support.mjs, and the time
// limit is killMsForKind()'s shared default. This file adds a prompt, not a
// pipeline.
//
// DECISIONS.md D-15 records why the prompt lives here rather than in the
// engine.
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

import { careerOpsRoot, rootScript, readMemory } from "./engine.mjs";

/**
 * The modes the client can run, and the file that defines each.
 *
 * The file path is relative to the DATA root, which is where a mode file
 * lives: `modes/` is a user-layer directory and a market mode set may be
 * declared elsewhere, so the path is resolved against the data root and
 * containment-checked before it is named in a prompt.
 */
export const MODE_KINDS = {
  email: { file: "modes/email.md", label: "application email" },
  "offer-prep": { file: "modes/offer-prep.md", label: "offer clause walk" },
  prep: { file: "modes/interview-prep.md", label: "interview prep pack" },
  plan: { file: "modes/interview/plan.md", label: "interview preparation plan" },
  practice: { file: "modes/interview/practice.md", label: "interview practice" },
  debrief: { file: "modes/interview/debrief.md", label: "interview debrief" },
  redflag: { file: "modes/interview-redflag.md", label: "company red-flag review" },
};

/** Is this kind one of the modes this client runs? */
export function isModeKind(kind) {
  return Object.hasOwn(MODE_KINDS, kind);
}

/**
 * The mode file for a kind, resolved inside the data root.
 *
 * A path that resolves outside it is refused: the file is named in a prompt,
 * and a prompt is an instruction.
 *
 * @param {string} kind
 * @returns {string} absolute path
 */
export function modeFile(kind) {
  const spec = MODE_KINDS[kind];
  if (!spec) throw new Error(`unknown mode kind: ${kind}`);
  const root = careerOpsRoot();
  const resolved = path.resolve(root, spec.file);
  if (resolved !== root && !resolved.startsWith(root + path.sep)) {
    throw new Error("the mode file resolves outside the data root");
  }
  return resolved;
}

/**
 * The core's own output-language rule, read from its library.
 *
 * `profile-language.mjs` is the canonical statement of the rule the CLI and
 * the web app both inject, so the client injects the same one rather than a
 * second wording of it.
 *
 * @returns {Promise<string>}
 */
async function languageInstruction() {
  const script = rootScript("profile-language");
  if (!fs.existsSync(script)) {
    return "Write all human-facing output in the language configured in config/profile.yml.";
  }
  const { outputLanguageInstruction, parseOutputLanguage } = await import(
    pathToFileURL(script).href
  );
  let profile = "";
  try {
    profile = fs.readFileSync(path.join(careerOpsRoot(), "config", "profile.yml"), "utf8");
  } catch {
    profile = "";
  }
  return outputLanguageInstruction(parseOutputLanguage(profile));
}

/**
 * The prompt for a mode run.
 *
 * @param {string} kind
 * @param {string} input
 * @param {{today?: string}} [options]
 * @returns {Promise<string>}
 */
export async function buildModePrompt(kind, input, options = {}) {
  const spec = MODE_KINDS[kind];
  const file = modeFile(kind);
  const today = options.today ?? "";
  const memory = readMemory();
  const mem = memory.trim() ? `\n\nDurable notes about the user (from their profile):\n${memory.trim()}\n` : "";
  const language = await languageInstruction();
  return `You are running the REAL career-ops ${spec.label} mode, HEADLESS, on the user's own machine${today ? `. Today is ${today}` : ""}.

1. Read ${spec.file} and follow it EXACTLY. Do not improvise your own rules: that file is the only source of truth for what this mode does.
2. This is an unattended run, so nobody can answer a question mid-run. Where the mode stops to ask the candidate something, do not stop: state the question and the answer you assumed, under the heading "For you to confirm".
3. Do not submit, send, or click Apply anywhere. Do not write, edit or delete any file — the platform saves your reply for you.${mem}
${language}

Target: ${input}`;
}
