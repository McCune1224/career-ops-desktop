// profile — the user-layer files the client reads and, in one case, writes:
// config/profile.yml, modes/_profile.md and the master career profile.
//
// The personalization file is written through the web app's OWN
// personalization-file.mjs: it merges ONE section into the file and refuses a
// file it cannot parse rather than rewriting it. The language and the market
// mode set are read through the core's own profile-language.mjs.
import fs from "node:fs";
import path from "node:path";

import {
  careerOpsRoot,
  rootScript,
  atomicWriteWithBackup,
  readPersonalizationState,
  savePersonalization,
  templatePath,
  UnsureProfileParseError,
  profileTargetKeywords,
} from "../engine.mjs";
import { runCoreScript } from "../core-script.mjs";
import { SECTIONS, scanHeadings } from "@/lib/personalization.mjs";
import { BridgeError, parseJsonOutput } from "../util.mjs";

/**
 * @param {{action?: string, section?: string, body?: string, dryRun?: boolean}} params
 */
export async function handle(params) {
  const action = params.action ?? "state";
  const root = careerOpsRoot();

  if (action === "state") {
    const codeRoot = path.dirname(rootScript("doctor"));
    const state = readPersonalizationState(root, codeRoot);
    // The bodies, so a section can be EDITED and not only reported. The
    // headings are the core's own scan; the client does not decide where a
    // section starts.
    const bodies = readSectionBodies(root);
    let profile = {};
    try {
      const yaml = await import("js-yaml");
      profile = yaml.load(fs.readFileSync(path.join(root, "config", "profile.yml"), "utf8")) ?? {};
    } catch {
      profile = {};
    }
    const language = profile.language ?? {};
    return {
      personalization: state,
      bodies,
      keywords: profileTargetKeywords(profile),
      language: {
        output: typeof language.output === "string" ? language.output : "en",
        modesDir: language.modes_dir ?? "modes",
      },
    };
  }

  if (action === "save") {
    const section = String(params.section ?? "").trim();
    const body = typeof params.body === "string" ? params.body : "";
    if (!section) throw new BridgeError("bad-request", "section required");
    const codeRoot = path.dirname(rootScript("doctor"));
    try {
      const result = savePersonalization({
        dataRoot: root,
        codeRoot,
        patch: { [section]: body },
        dryRun: params.dryRun === true,
        write: (target, content) => atomicWriteWithBackup(target, content),
      });
      return { ok: true, ids: result.ids, dryRun: params.dryRun === true };
    } catch (err) {
      if (err instanceof UnsureProfileParseError) {
        throw new BridgeError(
          "conflict",
          "modes/_profile.md could not be read safely — refusing to guess where the section goes.",
        );
      }
      throw err;
    }
  }

  if (action === "masterProfile") {
    const file = path.join(root, "data", "career-profile.yml");
    if (!fs.existsSync(file)) {
      return { available: false, path: file, message: "there is no master career profile yet" };
    }
    const res = await runCoreScript("career-profile", ["validate", file], { timeout: 60_000 });
    return {
      available: res.code === 0,
      path: file,
      valid: res.code === 0,
      output: res.output,
    };
  }

  if (action === "expand") {
    // skill-extract.mjs is a library, not a CLI: the shared skill vocabulary.
    const script = rootScript("skill-extract");
    if (!fs.existsSync(script)) {
      throw new BridgeError(
        "core-script-missing",
        "extracting competencies needs the career-ops scripts; this root has data only",
      );
    }
    const { extractSkills } = await import(pathToFileUrl(script));
    const text = typeof params.body === "string" ? params.body : "";
    const skills = [...extractSkills(text)].sort();
    return { skills };
  }

  if (action === "intake") {
    const res = await runCoreScript("intake", ["--summary"], { json: true, timeout: 120_000 });
    const data = res.data ?? {};
    return {
      available: res.code === 0,
      documentsDir: data.documentsDir ?? null,
      sources: data.sources ?? [],
      output: res.output,
    };
  }

  throw new BridgeError("bad-request", `unknown profile action: ${action}`);
}

/**
 * Each section's current body, keyed by its id.
 *
 * The split is the core's own `scanHeadings` + `renderSection` pair, so an
 * edit lands in the section the core thinks it belongs to.
 *
 * @param {string} root
 * @returns {Record<string, string>}
 */
function readSectionBodies(root) {
  const file = path.join(root, "modes", "_profile.md");
  let text = "";
  try {
    text = fs.readFileSync(file, "utf8");
  } catch {
    return {};
  }
  const { scanHeadings, SECTIONS } = personalizationSections();
  const { heads } = scanHeadings(text);
  const bodies = {};
  for (const section of SECTIONS) {
    const head = heads.find((h) => h.heading === section.heading);
    if (!head) continue;
    bodies[section.id] = text.slice(head.bodyStart, head.end).trim();
  }
  return bodies;
}

/**
 * The section table and the heading scanner, from the web app's own module.
 *
 * @returns {{SECTIONS: Array<{id: string, heading: string}>, scanHeadings: Function}}
 */
function personalizationSections() {
  return { SECTIONS, scanHeadings };
}

/**
 * A file URL for a script path, so a plain .mjs can be imported by path.
 *
 * @param {string} p
 * @returns {string}
 */
function pathToFileUrl(p) {
  return new URL(`file://${path.resolve(p)}`).href;
}

export const op = "profile";
