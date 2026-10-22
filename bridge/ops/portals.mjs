// portals — the watched job boards: do they answer, what do they contain, why
// was a posting missed, and what is new this week.
//
// Every answer comes from the core's own scripts: verify-portals.mjs,
// audit-portals.mjs, coverage-check.mjs, detect-reposts.mjs and
// discover-ats.mjs, plus the web app's own whats-new module.
//
// The EDIT action is the one that writes, and it writes through the web app's
// own portals-config.mjs loader and merge: a file it cannot parse is refused
// rather than rewritten, and unknown keys survive.
import fs from "node:fs";
import path from "node:path";
// The engine's own YAML library, resolved from the career-ops checkout's
// dependencies by the alias hook — the same js-yaml the core's own scripts and
// the web app's own routes use, so there is one YAML writer in play.
import * as yaml from "js-yaml";

import { careerOpsRoot, atomicWriteWithBackup, isMapping } from "../engine.mjs";
import { runCoreScript } from "../core-script.mjs";
import { BridgeError } from "../util.mjs";

/** The watched boards, as the core reads them. */
function portalsPath(root) {
  return process.env.CAREER_OPS_PORTALS || path.join(root, "portals.yml");
}

/**
 * @param {{action?: string, url?: string, company?: string, title?: string, note?: string, names?: string[], write?: boolean}} params
 */
export async function handle(params) {
  const action = params.action ?? "verify";
  const root = careerOpsRoot();

  if (action === "verify") {
    const res = await runCoreScript("verify-portals", [], { timeout: 180_000 });
    return { ok: res.code === 0, output: res.output };
  }

  if (action === "audit") {
    const res = await runCoreScript("audit-portals", ["--json"], { json: true, timeout: 180_000 });
    const data = res.data;
    const rows = Array.isArray(data) ? data : (data?.rows ?? []);
    return {
      ok: res.code === 0,
      rows,
      drops: Array.isArray(data) ? [] : (data?.drops ?? []),
      output: res.output,
    };
  }

  if (action === "coverage") {
    const url = String(params.url ?? "").trim();
    const company = String(params.company ?? "").trim();
    const title = String(params.title ?? "").trim();
    if (!url || !company || !title) {
      throw new BridgeError("bad-request", "url, company and title are required");
    }
    const args = [url, "--company", company, "--title", title, "--json"];
    if (params.note) args.push("--note", String(params.note));
    const res = await runCoreScript("coverage-check", args, { json: true, timeout: 60_000 });
    const data = res.data ?? {};
    return {
      ok: res.code === 0,
      gap: data.gap ?? null,
      meaning: data.meaning ?? null,
      evidence: data.evidence ?? [],
      fix: data.fix ?? null,
      output: res.output,
    };
  }

  if (action === "discover") {
    const names = Array.isArray(params.names) ? params.names.map((n) => String(n).trim()).filter(Boolean) : [];
    if (names.length === 0) throw new BridgeError("bad-request", "names required");
    const file = path.join(root, "data", "portals-discover.yml");
    const yaml = names.map((name) => `  - name: ${JSON.stringify(name)}`).join("\n");
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, `companies:\n${yaml}\n`, "utf8");
    const args = ["--in", file, "--summary"];
    if (params.write === true) args.push("--write");
    const res = await runCoreScript("discover-ats", args, { json: true, timeout: 180_000 });
    return {
      ok: res.code === 0,
      metadata: res.data?.metadata ?? null,
      resolved: res.data?.resolved ?? [],
      unresolved: res.data?.unresolved ?? [],
      output: res.output,
    };
  }

  if (action === "reposts") {
    const res = await runCoreScript("detect-reposts", [], { json: true, timeout: 60_000 });
    return {
      ok: res.code === 0,
      metadata: res.data?.metadata ?? null,
      clusters: res.data?.clusters ?? [],
      output: res.output,
    };
  }

  if (action === "whatsNew") {
    const days = Math.min(30, Math.max(1, Number(params.days) || 7));
    const limit = Math.min(200, Math.max(1, Number(params.limit) || 24));
    const cutoff = Date.now() - days * 86_400_000;
    let rows;
    try {
      rows = fs
        .readFileSync(path.join(root, "data", "scan-history.tsv"), "utf8")
        .split("\n");
    } catch (err) {
      if (err?.code === "ENOENT") {
        return { available: true, offers: [], count: 0, days };
      }
      throw new BridgeError("failed", `scan history could not be read: ${err.message}`);
    }
    const { collectWhatsNew, resolveOfferLimit } = await import("../engine.mjs");
    const { evaluatedKeys, isEvaluated } = await import("../engine.mjs");
    const { readApplications } = await import("../engine.mjs");
    const { getNormalizeTextKey } = await import("../engine.mjs");
    const normalizeTextKey = await getNormalizeTextKey();
    const evaluated = evaluatedKeys(readApplications(), normalizeTextKey);
    const toOffer = (c) => {
      const [url, firstSeen, portal, title, company, status, location] = c;
      if (!url || !/^https?:\/\//i.test(url)) return null;
      if (status && /skipped|expired/i.test(status)) return null;
      if (isEvaluated(evaluated, normalizeTextKey, company, title)) return null;
      return {
        url,
        company: (company || "").trim(),
        title: (title || "").trim(),
        location: (location || "").trim(),
        postedAt: /^\d{4}-\d{2}-\d{2}$/.test(firstSeen || "") ? firstSeen : "",
        ats: (portal || "").replace(/-full$/, "").trim() || "other",
        source: "whats-new",
      };
    };
    const { offers, count } = collectWhatsNew(rows, {
      cutoff,
      toOffer,
      offerLimit: resolveOfferLimit(String(limit)),
    });
    return { available: true, offers, count, days };
  }

  if (action === "read") {
    const file = portalsPath(root);
    if (!fs.existsSync(file)) {
      return { exists: false, path: file, document: null, companies: [] };
    }
    const { loadPortalsDocument } = await import("../engine.mjs");
    const template = path.join(root, "portals.template.yml");
    try {
      const { doc } = loadPortalsDocument(file, fs.existsSync(template) ? template : file);
      return {
        exists: true,
        path: file,
        document: doc,
        companies: Array.isArray(doc.tracked_companies) ? doc.tracked_companies : [],
      };
    } catch (err) {
      throw new BridgeError("conflict", err?.message ?? "portals.yml could not be read");
    }
  }

  if (action === "write") {
    const file = portalsPath(root);
    const document = params.document;
    if (!isMapping(document)) {
      throw new BridgeError("bad-request", "document must be a mapping");
    }
    const { loadPortalsDocument } = await import("../engine.mjs");
    const template = path.join(root, "portals.template.yml");
    if (fs.existsSync(file)) {
      // A file that exists but cannot be read is REFUSED, never rewritten.
      try {
        loadPortalsDocument(file, fs.existsSync(template) ? template : file);
      } catch (err) {
        throw new BridgeError("conflict", err?.message ?? "portals.yml could not be read");
      }
    }
    atomicWriteWithBackup(file, yaml.dump(document, { lineWidth: 100, noRefs: true }));
    return { ok: true, path: file };
  }

  throw new BridgeError("bad-request", `unknown portals action: ${action}`);
}

export const op = "portals";
