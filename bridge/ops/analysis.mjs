// analysis — the read-only analyses over the pipeline: keyword coverage, skill
// gaps, score levers, a company's history, the quiet ones, recruiting friction,
// the salary gap and negotiation points.
//
// Every one of them is the CORE's own script or the WEB app's own module:
// keyword-match.mjs, upskill.mjs, machine-summary.mjs, company-history.mjs,
// rejection-latency.mjs, process-quality.mjs, salary-gap.mjs and
// negotiation-roi.mjs. Nothing here decides what a score means, what a gap is
// or whether a company is quiet.
import path from "node:path";

import { careerOpsRoot, findReportFile, readApplications, readReport } from "../engine.mjs";
import { runCoreScript } from "../core-script.mjs";
import { BridgeError } from "../util.mjs";

/**
 * The report file for one application, or a refusal.
 *
 * The report is resolved through the engine's own resolver, which follows the
 * tracker's report link and containment-checks the result.
 */
function reportFileFor(n) {
  const found = findReportFile(String(n ?? "").trim());
  if (!found) {
    throw new BridgeError("not-found", `no report for application ${n}`);
  }
  return found;
}

/**
 * @param {{action?: string, n?: string, company?: string, query?: string, count?: number, ids?: string[], wage?: number, frequency?: string}} params
 */
export async function handle(params) {
  const action = params.action ?? "keywordMatch";

  if (action === "keywordMatch") {
    const file = reportFileFor(params.n);
    const res = await runCoreScript("keyword-match", [file, "--json"], { json: true, timeout: 60_000 });
    const data = res.data ?? {};
    return {
      available: res.code === 0 && typeof data.total === "number",
      total: data.total ?? 0,
      presentCount: data.presentCount ?? 0,
      coveragePct: data.coveragePct ?? null,
      present: data.present ?? [],
      thin: data.thin ?? [],
      missing: data.missing ?? [],
      keywordSource: data.keywordSource ?? null,
      output: res.output,
    };
  }

  if (action === "upskill") {
    const res = await runCoreScript("upskill", [], { json: true, timeout: 120_000 });
    const data = res.data ?? {};
    return {
      available: res.code === 0 && Array.isArray(data.gaps),
      metadata: data.metadata ?? null,
      gaps: data.gaps ?? [],
      excludedAsKnown: data.excludedAsKnown ?? [],
      knownSkills: data.knownSkills ?? [],
      output: res.output,
    };
  }

  if (action === "scoreLevers") {
    const file = reportFileFor(params.n);
    const report = readReport(String(params.n ?? "").trim());
    const { readMachineSummary, scoreLevers } = await import("../engine.mjs");
    const summary = report?.content ? readMachineSummary(report.content) : null;
    if (!summary) {
      return { available: false, reason: "this report has no Machine Summary", levers: null };
    }
    return { available: true, summary, levers: scoreLevers(summary) };
  }

  if (action === "companyHistory") {
    const company = String(params.company ?? "").trim();
    if (!company) throw new BridgeError("bad-request", "company required");
    const res = await runCoreScript("company-history", ["--company", company], { json: true, timeout: 60_000 });
    const data = res.data ?? {};
    return {
      available: res.code === 0 && Boolean(data.company),
      company: data.company ?? company,
      rows: data.rows ?? data.facts ?? [],
      output: res.output,
    };
  }

  if (action === "quietCompanies") {
    const res = await runCoreScript("rejection-latency", [], { json: true, timeout: 60_000 });
    const data = res.data ?? {};
    return {
      available: res.code === 0,
      metadata: data.metadata ?? null,
      flags: data.flags ?? [],
      disclaimer: data.disclaimer ?? null,
      output: res.output,
    };
  }

  if (action === "processQuality") {
    const res = await runCoreScript("process-quality", [], { json: true, timeout: 60_000 });
    const data = res.data ?? {};
    return {
      available: res.code === 0,
      metadata: data.metadata ?? null,
      signals: data.signals ?? [],
      output: res.output,
    };
  }

  if (action === "salaryGap") {
    const res = await runCoreScript("salary-gap", [], { json: true, timeout: 60_000 });
    const data = res.data ?? {};
    return {
      available: res.code === 0,
      applications: data.applications ?? [],
      aggregates: data.aggregates ?? null,
      quality: data.quality ?? null,
      output: res.output,
    };
  }

  if (action === "negotiation") {
    const args = [];
    if (Number.isFinite(params.wage) && Number(params.wage) > 0) args.push("--wage", String(Math.round(Number(params.wage))));
    if (typeof params.frequency === "string" && params.frequency.trim()) {
      args.push("--frequency", params.frequency.trim());
    }
    const res = await runCoreScript("negotiation-roi", args, { json: true, timeout: 60_000 });
    const data = res.data ?? {};
    return {
      available: res.code === 0,
      storiesScanned: data.storiesScanned ?? 0,
      verified: data.verified ?? [],
      calculable: data.calculable ?? [],
      uncalculable: data.uncalculable ?? [],
      warnings: data.warnings ?? [],
      output: res.output,
    };
  }

  if (action === "compare") {
    const ids = Array.isArray(params.ids) ? params.ids.map((v) => String(v).trim()).filter(Boolean) : [];
    if (ids.length < 2) throw new BridgeError("bad-request", "compare needs at least two applications");
    const { compareIds, summaryFields } = await import("../engine.mjs");
    const chosen = compareIds(ids);
    if (!chosen || chosen.length < 2) {
      throw new BridgeError("bad-request", "compare takes two or three tracker numbers");
    }
    const applications = readApplications();
    const columns = chosen.map((id) => {
      const row = applications.find((r) => String(r.n).trim() === String(id));
      // The summary fields are the report's own two-column table, so the
      // comparison shows what the reports say and not what the tracker repeats.
      const report = readReport(String(id));
      return {
        n: String(id),
        company: row?.company ?? "",
        role: row?.role ?? "",
        score: row?.score ?? "",
        status: row?.status ?? "",
        date: row?.date ?? "",
        fields: report ? summaryFields(report.content) : [],
      };
    });
    return { ids: chosen, columns };
  }

  throw new BridgeError("bad-request", `unknown analysis action: ${action}`);
}

export const op = "analysis";
