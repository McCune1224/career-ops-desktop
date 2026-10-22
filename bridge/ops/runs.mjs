// runs — this client's record of the runs it started.
//
// A finished run leaves two files under data/runs/:
//
//   <id>.md    the log, in markdown, so the CLI assistant can read past runs
//              ("what did we find on that Anthropic role?")
//   <id>.json  the same run's machine facts — kind, tokens, cost, when — so
//              the cost estimate has real samples to take a median of
//
// The markdown is the readable half and the JSON is the machine half, and the
// JSON is the ONLY thing the estimate reads. A log the agent can read and a
// record the client can count are the same run, not two records of it.
//
// The write is atomic: a half-written run log is a run nobody can read.

import fs from "node:fs";
import path from "node:path";

import { careerOpsRoot, atomicWrite } from "../engine.mjs";
import { BridgeError } from "../util.mjs";

/** Where the run records live. */
export function runsDir() {
  return path.join(careerOpsRoot(), "data", "runs");
}

/** A file-name-safe id for one run. */
function safeId(raw) {
  const id = String(raw ?? "").replace(/[^a-z0-9_-]/gi, "");
  if (!id) throw new BridgeError("bad-request", "id required");
  return id;
}

/**
 * The markdown the CLI assistant reads.
 *
 * The same shape the web app writes, so an agent that knows one knows the
 * other: a heading, the run's inputs, the steps, and the output.
 */
function renderMarkdown(run) {
  const steps = (Array.isArray(run.steps) ? run.steps : [])
    .map((s) => `- ${s.kind === "tool" ? `🔧 ${s.label}` : s.label}`)
    .join("\n");
  const verdict = Number.isFinite(run.score) ? `${run.score}/5` : "—";
  return `# Desktop run · ${run.title || run.id}

- id: ${run.id}
- kind: ${run.kind || "evaluate"}
- input: ${run.input || "-"}
- verdict: ${verdict}

## Steps
${steps}

## Output
${run.output || ""}
`;
}

/**
 * @param {{action?: string, id?: string, run?: Record<string, unknown>}} params
 */
export async function handle(params) {
  const action = params.action ?? "save";

  if (action === "save") {
    const run = params.run ?? {};
    const id = safeId(run.id);
    const dir = runsDir();
    fs.mkdirSync(dir, { recursive: true });
    const record = {
      id,
      kind: String(run.kind ?? "evaluate"),
      title: String(run.title ?? ""),
      input: String(run.input ?? ""),
      status: String(run.status ?? "done"),
      tokens: Number.isFinite(run.tokens) ? Number(run.tokens) : 0,
      usd: Number.isFinite(run.usd) ? Number(run.usd) : 0,
      score: Number.isFinite(run.score) ? Number(run.score) : null,
      startedAt: Number.isFinite(run.startedAt) ? Number(run.startedAt) : Date.now(),
      steps: Array.isArray(run.steps) ? run.steps : [],
      output: String(run.output ?? ""),
    };
    atomicWrite(path.join(dir, `${id}.md`), renderMarkdown(record));
    atomicWrite(path.join(dir, `${id}.json`), `${JSON.stringify(record, null, 2)}\n`);
    return { ok: true, id, path: path.join(dir, `${id}.md`) };
  }

  if (action === "list") {
    const dir = runsDir();
    let names = [];
    try {
      names = fs.readdirSync(dir).filter((n) => n.endsWith(".json"));
    } catch {
      return { runs: [] };
    }
    const runs = [];
    for (const name of names) {
      try {
        const parsed = JSON.parse(fs.readFileSync(path.join(dir, name), "utf8"));
        runs.push({
          id: parsed.id,
          kind: parsed.kind,
          title: parsed.title,
          status: parsed.status,
          tokens: parsed.tokens,
          usd: parsed.usd,
          startedAt: parsed.startedAt,
        });
      } catch {
        /* skipped */
      }
    }
    runs.sort((a, b) => Number(b.startedAt ?? 0) - Number(a.startedAt ?? 0));
    return { runs };
  }

  throw new BridgeError("bad-request", `unknown runs action: ${action}`);
}

export const op = "runs";
