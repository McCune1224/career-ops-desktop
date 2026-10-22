// followups — the DEMAND loop: what is due, what was logged, and the cadence
// knobs behind it.
//
// The cadence verdict comes from the core's own followup-cadence.mjs --json,
// the SAME calculator the CLI uses. The desktop client never reimplements the
// cadence rule; it reads the core's answer (mirrors the web's /api/followups).
//
// The log (data/follow-ups.md) is written under the same guard the web route
// uses: the in-process queue OUTSIDE, the core's cross-process file lock
// INSIDE. A user with the CLI running at the same time must not lose a row.
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";
import * as yaml from "js-yaml";
import {
  careerOpsRoot,
  rootScript,
  atomicWrite,
  atomicWriteWithBackup,
  followupsLogPath,
  withFollowupsWrite,
  readFollowupsEnabled,
  followupsEnabledFromProfile,
  selectDueFollowups,
  selectUpcomingFollowups,
  pickNextUpcoming,
  parseFollowupId,
  isRealISODate,
  localISODate,
  CHANNELS,
  PROFILE_CADENCE_KEYS,
  isMapping,
} from "../engine.mjs";
import { runNode, BridgeError } from "../util.mjs";

const CADENCE_TIMEOUT_MS = 12_000;

/** Table cells must stay single-line and pipe-free. */
function cell(v, max) {
  const s = String(v ?? "")
    .replace(/[\r\n]+/g, " ")
    .replace(/\|/g, "/")
    .trim();
  return max ? s.slice(0, max) : s;
}

const TABLE_HEADER = "| num | appNum | date | company | role | channel | contact | notes |\n|---|---|---|---|---|---|---|---|\n";

/**
 * The core script's own parsers for data/follow-ups.md.
 *
 * Loaded from the checkout the rest of the engine comes from, so the log's
 * grammar has exactly one definition. A missing script is not fatal: the log
 * history is then empty and the screen says so.
 */
async function loadCoreParsers() {
  const script = rootScript("followup-cadence");
  if (!fs.existsSync(script)) return null;
  try {
    return await import(pathToFileURL(script).href);
  } catch {
    return null;
  }
}

/** followup-cadence.mjs --json — the core's verdict, verbatim. */
async function readCadenceJson() {
  const script = rootScript("followup-cadence");
  if (!fs.existsSync(script)) return null;
  const res = await runNode(process.execPath, [script, "--json"], {
    cwd: careerOpsRoot(),
    timeout: CADENCE_TIMEOUT_MS,
  });
  if (res.code !== 0 && !res.stdout.trim()) return null;
  try {
    const start = res.stdout.indexOf("{");
    if (start === -1) return null;
    return JSON.parse(res.stdout.slice(start));
  } catch {
    return null;
  }
}

/** The pure defaults the core emits, mapped onto the profile's key names. */
function readCoreDefaults(json) {
  if (!json || !isMapping(json.cadenceDefaults)) return null;
  const core = json.cadenceDefaults;
  const out = {};
  for (const key of PROFILE_CADENCE_KEYS) {
    const coreKey = key === "applied_max_followups" ? key : key.replace(/_days$/, "");
    const raw = core[coreKey];
    if (typeof raw !== "number" || !Number.isInteger(raw) || raw < 0) return null;
    out[key] = raw;
  }
  return out;
}

function profilePath() {
  return path.join(careerOpsRoot(), "config", "profile.yml");
}

function readProfile() {
  const file = profilePath();
  if (!fs.existsSync(file)) return { exists: false, profile: {} };
  try {
    const parsed = yaml.load(fs.readFileSync(file, "utf8"));
    return { exists: true, profile: isMapping(parsed) ? parsed : {} };
  } catch {
    return { exists: true, profile: {}, malformed: true };
  }
}

/**
 * @param {Record<string, unknown>} params
 */
export async function handle(params) {
  const action = params.action ?? "list";

  if (action === "list") {
    const json = await readCadenceJson();
    const parsers = await loadCoreParsers();
    const logPath = followupsLogPath();
    let logContent = "";
    try {
      logContent = fs.readFileSync(logPath, "utf8");
    } catch {
      /* no log yet */
    }
    const history = parsers?.parseFollowups ? parsers.parseFollowups(logContent) : [];
    const overrides = parsers?.parseNextOverrides ? parsers.parseNextOverrides(logContent) : [];

    if (!json) {
      return {
        available: false,
        metadata: null,
        entries: [],
        due: [],
        upcoming: [],
        nextUpcoming: null,
        history,
        overrides: [...overrides.entries()].map(([appNum, value]) => ({ appNum, ...value })),
        followupsEnabled: readFollowupsEnabled(),
      };
    }
    const entries = Array.isArray(json.entries) ? json.entries : [];
    return {
      available: true,
      metadata: json.metadata ?? null,
      entries,
      due: selectDueFollowups(entries),
      upcoming: selectUpcomingFollowups(entries),
      nextUpcoming: pickNextUpcoming(entries),
      history,
      overrides: [...overrides.entries()].map(([appNum, value]) => ({ appNum, ...value })),
      followupsEnabled: json.followupsEnabled !== false,
    };
  }

  if (action === "log") {
    const appNum = parseFollowupId(params.appNum ?? params.num);
    if (appNum === null) throw new BridgeError("bad-request", "appNum (application #) required");
    const company = cell(params.company, 80);
    if (!company) throw new BridgeError("bad-request", "company required");
    const date = String(params.date ?? localISODate()).trim();
    if (!isRealISODate(date)) {
      throw new BridgeError("bad-request", "date must be a real calendar date (YYYY-MM-DD)");
    }
    const rawChannel = String(params.channel ?? "Other").trim();
    const channel = CHANNELS.find((c) => c.toLowerCase() === rawChannel.toLowerCase());
    if (!channel) {
      throw new BridgeError("bad-request", `channel must be one of: ${CHANNELS.join(", ")}`);
    }
    const role = cell(params.role, 80);
    const contact = cell(params.contact, 120);
    const notes = cell(params.notes ?? params.note ?? "");

    const file = followupsLogPath();
    const num = await withFollowupsWrite(() => {
      fs.mkdirSync(path.dirname(file), { recursive: true });
      let existing = "";
      if (fs.existsSync(file)) existing = fs.readFileSync(file, "utf8");
      else fs.writeFileSync(file, "# Follow-ups\n\n", "utf8");

      let maxNum = 0;
      let hasHeader = false;
      for (const line of existing.split("\n")) {
        if (!line.startsWith("|")) continue;
        const first = line.split("|")[1]?.trim() ?? "";
        // Both header spellings exist in the wild: "num" (this writer) and "#"
        // (files created per modes/followup.md).
        if (/^(num|#)$/i.test(first)) hasHeader = true;
        const n = Number.parseInt(first, 10);
        if (Number.isInteger(n) && n > maxNum) maxNum = n;
      }
      const next = maxNum + 1;

      let out = "";
      if (existing && !existing.endsWith("\n")) out += "\n";
      if (!hasHeader) out += TABLE_HEADER;
      out += `| ${next} | ${appNum} | ${date} | ${company} | ${role} | ${channel} | ${contact} | ${notes} |\n`;
      fs.appendFileSync(file, out, "utf8");
      return next;
    });
    return { ok: true, num, appNum, date, channel };
  }

  if (action === "delete") {
    const num = parseFollowupId(params.num);
    if (num === null) throw new BridgeError("bad-request", "num required");
    const file = followupsLogPath();
    if (!fs.existsSync(file)) throw new BridgeError("not-found", "no follow-up log");
    await withFollowupsWrite(() => {
      const lines = fs.readFileSync(file, "utf8").split("\n");
      const idx = lines.findIndex((line) => {
        if (!line.startsWith("|")) return false;
        const first = line.split("|")[1]?.trim() ?? "";
        return Number.parseInt(first, 10) === num && /^\d+$/.test(first);
      });
      if (idx === -1) throw new BridgeError("not-found", `follow-up #${num} not found`);
      lines.splice(idx, 1);
      atomicWrite(file, lines.join("\n"));
    });
    return { ok: true, num };
  }

  if (action === "override") {
    const appNum = parseFollowupId(params.appNum);
    if (appNum === null) throw new BridgeError("bad-request", "appNum required");
    const date = String(params.date ?? "").trim();
    if (!isRealISODate(date)) {
      throw new BridgeError("bad-request", "date must be a real calendar date (YYYY-MM-DD)");
    }
    const file = followupsLogPath();
    const today = localISODate();
    await withFollowupsWrite(() => {
      fs.mkdirSync(path.dirname(file), { recursive: true });
      const lines = fs.existsSync(file) ? fs.readFileSync(file, "utf8").split("\n") : ["# Follow-ups", ""];
      // A pin supersedes any earlier pin for the same application: drop the
      // previous directive rather than leaving two, so the LAST one wins
      // without the reader having to guess which.
      const kept = lines.filter((line) => !/^\s*-\s+next\s+#\d+\b/i.test(line));
      kept.push(`- next #${appNum} ${date} (set ${today})`);
      atomicWrite(file, `${kept.join("\n").replace(/\n+$/, "")}\n`);
    });
    return { ok: true, appNum, date };
  }

  if (action === "unoverride") {
    const appNum = parseFollowupId(params.appNum);
    if (appNum === null) throw new BridgeError("bad-request", "appNum required");
    const file = followupsLogPath();
    if (!fs.existsSync(file)) return { ok: true, appNum };
    await withFollowupsWrite(() => {
      const lines = fs.readFileSync(file, "utf8").split("\n");
      const kept = lines.filter((line) => !new RegExp(`^\\s*-\\s+next\\s+#${appNum}\\b`, "i").test(line));
      atomicWrite(file, kept.join("\n"));
    });
    return { ok: true, appNum };
  }

  if (action === "cadence.get") {
    const { profile } = readProfile();
    const overrides = {};
    const source = isMapping(profile.followup_cadence) ? profile.followup_cadence : {};
    for (const key of PROFILE_CADENCE_KEYS) {
      const n = Number.parseInt(String(source[key]), 10);
      if (Number.isFinite(n) && n >= 0) overrides[key] = n;
    }
    const defaults = readCoreDefaults(await readCadenceJson());
    return {
      defaults: defaults ?? {},
      defaultsAvailable: defaults !== null,
      overrides,
      effective: { ...(defaults ?? {}), ...overrides },
      enabled: followupsEnabledFromProfile(profile),
    };
  }

  if (action === "cadence.set") {
    const cadence = {};
    for (const key of PROFILE_CADENCE_KEYS) {
      if (params[key] == null) continue;
      const n = Number.parseInt(String(params[key]), 10);
      if (!Number.isInteger(n) || n < 0) {
        throw new BridgeError("bad-request", `${key} must be a non-negative integer`);
      }
      cadence[key] = n;
    }
    const enabled = params.enabled === undefined ? undefined : Boolean(params.enabled);
    if (Object.keys(cadence).length === 0 && enabled === undefined) {
      throw new BridgeError("bad-request", "nothing to write");
    }

    const root = careerOpsRoot();
    const file = profilePath();
    let base = {};
    if (!fs.existsSync(file)) {
      try {
        const seeded = yaml.load(fs.readFileSync(path.join(root, "config", "profile.example.yml"), "utf8"));
        base = isMapping(seeded) ? seeded : {};
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
          "config/profile.yml exists but could not be read as YAML — refusing to overwrite it.",
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

    const merged = { ...base };
    if (Object.keys(cadence).length > 0) {
      merged.followup_cadence = { ...(isMapping(base.followup_cadence) ? base.followup_cadence : {}), ...cadence };
    }
    if (enabled !== undefined) {
      merged.follow_ups = { ...(isMapping(base.follow_ups) ? base.follow_ups : {}), enabled };
    }
    atomicWriteWithBackup(file, yaml.dump(merged, { lineWidth: 100, noRefs: true }));
    return { ok: true, followup_cadence: merged.followup_cadence, follow_ups: merged.follow_ups };
  }

  throw new BridgeError("bad-request", `unknown followups action: ${action}`);
}

export const op = "followups";
