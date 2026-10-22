// config — the settings the in-scope screens depend on.
//
// The profile (config/profile.yml) is a USER-LAYER file. It is never clobbered:
// the write deep-merges only the proposed keys and refuses to touch a file that
// exists but cannot be read as a mapping. The rules here are the web app's
// /api/profile rules, unchanged — a broken profile must surface as an error
// the user can act on, not as an empty default.
import fs from "node:fs";
import path from "node:path";
import * as yaml from "js-yaml";
import {
  careerOpsRoot,
  rootScript,
  atomicWriteWithBackup,
  detectClis,
  canonicalLabels,
  isMapping,
  profilePatchError,
} from "../engine.mjs";
import { runNode, BridgeError, parseJsonOutput } from "../util.mjs";

function isObj(v) {
  return !!v && typeof v === "object" && !Array.isArray(v);
}

/** Deep-merge src onto dst (objects recurse; arrays/scalars replace). Non-mutating. */
function deepMerge(dst, src) {
  const out = isObj(dst) ? { ...dst } : {};
  for (const [k, v] of Object.entries(src)) {
    out[k] = isObj(v) ? deepMerge(out[k], v) : v;
  }
  return out;
}

/** The subset of profile.yml the config screen owns. */
function patchToProfile(p) {
  const out = {};
  const candidate = {};
  if (p.name) candidate.full_name = p.name;
  if (p.email) candidate.email = p.email;
  if (p.location) candidate.location = p.location;
  if (Object.keys(candidate).length) out.candidate = candidate;
  if (p.roles?.length) out.target_roles = { primary: p.roles.slice(0, 6) };
  const comp = {};
  if (p.compMin && p.compMax) comp.target_range = `${p.compMin}-${p.compMax}`;
  if (p.currency) comp.currency = p.currency;
  if (p.remote) comp.location_flexibility = p.remote;
  if (Object.keys(comp).length) out.compensation = comp;
  return out;
}

function profilePath() {
  return path.join(careerOpsRoot(), "config", "profile.yml");
}

/**
 * @param {Record<string, unknown>} params
 */
export async function handle(params) {
  const action = params.action ?? "get";

  if (action === "get") {
    const file = profilePath();
    let profile = {};
    let exists = false;
    let malformed = false;
    if (fs.existsSync(file)) {
      exists = true;
      try {
        const parsed = yaml.load(fs.readFileSync(file, "utf8"));
        profile = isMapping(parsed) ? parsed : {};
        malformed = !isMapping(parsed);
      } catch {
        malformed = true;
      }
    }
    return {
      profile,
      exists,
      malformed,
      clis: detectClis(),
      states: canonicalLabels(),
      codeRoot: path.dirname(rootScript("doctor")),
      dataRoot: careerOpsRoot(),
    };
  }

  if (action === "setProfile") {
    const error = profilePatchError(params);
    if (error) throw new BridgeError("bad-request", error);
    const proposed = patchToProfile(params);
    if (Object.keys(proposed).length === 0) throw new BridgeError("bad-request", "nothing to write");

    const root = careerOpsRoot();
    const file = profilePath();
    let base = {};
    let seeded = false;
    if (!fs.existsSync(file)) {
      try {
        base = yaml.load(fs.readFileSync(path.join(root, "config", "profile.example.yml"), "utf8")) || {};
        seeded = Object.keys(base).length > 0;
      } catch {
        base = {};
      }
    } else {
      let parsed;
      try {
        parsed = yaml.load(fs.readFileSync(file, "utf8"));
      } catch {
        throw new BridgeError(
          "conflict",
          "config/profile.yml exists but is not valid YAML — refusing to overwrite it.",
        );
      }
      if (!isMapping(parsed)) {
        throw new BridgeError(
          "conflict",
          "config/profile.yml must contain named settings, not a list or single value. Refusing to overwrite it.",
        );
      }
      base = parsed;
    }

    const merged = deepMerge(base, proposed);
    atomicWriteWithBackup(file, yaml.dump(merged, { lineWidth: 100, noRefs: true }));
    return { ok: true, seeded };
  }

  if (action === "validate") {
    // The core's own validator, not a second set of rules here.
    // validate-profile.mjs --json answers with the findings it found; an
    // unreadable profile is a finding, not an empty default.
    const script = rootScript("validate-profile");
    if (!fs.existsSync(script)) {
      throw new BridgeError(
        "core-script-missing",
        "validating the profile needs the career-ops scripts; this root has data only",
      );
    }
    const res = await runNode(process.execPath, [script, "--json"], {
      cwd: careerOpsRoot(),
      timeout: 30_000,
    });
    const parsed = parseJsonOutput(res.stdout);
    const findings = Array.isArray(parsed?.findings) ? parsed.findings : [];
    const errors = findings.filter((f) => f?.level === "error");
    const warnings = findings.filter((f) => f?.level === "warn");
    return {
      ok: res.code === 0 && errors.length === 0,
      exitCode: res.code,
      profile: parsed?.profile ?? null,
      findings,
      errors,
      warnings,
      message: errors.length
        ? errors[0].message ?? "the profile has problems"
        : warnings.length
          ? warnings[0].message ?? "the profile has warnings"
          : "the profile is fine",
    };
  }

  throw new BridgeError("bad-request", `unknown config action: ${action}`);
}

export const op = "config";
