// run — the structured run screen: evaluate a pasted JD or URL, tailor a CV,
// draft a cover letter, render a PDF.
//
// THIS IS THE SAME ENGINE THE WEB APP RUNS. Every rule below comes from a
// module the web app's POST /api/run imports (see engine.mjs), not from a
// second implementation:
//
//   * the CLI is resolved with the web app's resolveCliOrFallback()
//   * its capabilities come from worker-capabilities.mjs
//   * its argv is built by claude-invocation.mjs / run-cli-support.mjs and
//     fenced by cli-fencing.mjs through spawn-cli.mjs
//   * the report and CV arrive inside the same <<report-md>> / <<cv-html>>
//     envelopes (report-envelope.mjs, cv-envelope.mjs)
//   * the report is persisted by report-persist.mjs, which runs
//     reserve-report-num.mjs, persist-evaluation-report.mjs and
//     merge-tracker.mjs — the canonical writers
//   * the PDF is rendered by pdf-render.mjs, which runs generate-pdf.mjs and
//     mark-pdf-ready.mjs
//
// The only thing this file owns is the TRANSPORT: the web streams NDJSON over
// HTTP, the desktop streams the same NDJSON events over the bridge's stdio.
// The event names, the order and the honesty gates are the web app's.
//
// Nothing here submits an application, and no agent in these kinds holds a
// write tool: the backend writes every file.
import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";
import {
  careerOpsRoot,
  rootScript,
  readMemory,
  findReportFile,
  readInbox,
  readScanDates,
  readLanguageConfig,
  resolveCliOrFallback,
  cliUnavailableError,
  cliSubstitutionNotice,
  capabilitiesFor,
  isCliAllowedForCapabilities,
  fencingReport,
  spawnHeadlessCli,
  claudeCliArgs,
  codexStreamArgs,
  createAgentTextJoiner,
  accumulateTokens,
  killMsForKind,
  timeoutMessage,
  isFatalGenericStderr,
  createReportEnvelopeFilter,
  createCvEnvelopeFilter,
  persistEvaluation,
  evaluateRunOutcome,
  pdfRunOutcome,
  renderAndMarkPdf,
  writeCvHtml,
  buildPrompt,
  isShellSafeCompanyName,
  resolvePdfPaths,
  parseCoverRequest,
  resolveCvTemplate,
  acquireTrackerWrite,
  releaseTrackerWrite,
  createRunFinalizer,
  localISODate,
} from "../engine.mjs";
import { BridgeError } from "../util.mjs";
import { isModeKind, buildModePrompt, modeFile, MODE_KINDS } from "../mode-prompt.mjs";

/**
 * Run one worker, or a batch of them.
 *
 * A batch is the SAME worker, once per input, in order, each with its own
 * time limit and its own report. It is not a wider budget: `killMsForKind()`
 * applies to every run in the batch exactly as it does to a single one, and
 * the batch spends tokens one posting at a time, each of which the client
 * confirmed before it started.
 *
 * @param {{kind?: string, input?: string, inputs?: string[], cliId?: string, brief?: unknown}} params
 * @param {{emit: (event: unknown) => void}} ctx
 */
export async function handle(params, ctx) {
  const inputs = Array.isArray(params.inputs)
    ? params.inputs.map((value) => String(value ?? "").trim()).filter(Boolean)
    : [];
  if (inputs.length === 0) {
    return await runOne(params, ctx);
  }
  const results = [];
  for (const [index, input] of inputs.entries()) {
    ctx.emit({
      type: "status",
      label: `Batch ${index + 1} of ${inputs.length}: ${input}`,
    });
    const result = await runOne({ ...params, input }, ctx);
    results.push(result);
  }
  return {
    ok: results.every((result) => result && result.ok === true),
    kind: params.kind ?? "evaluate",
    batch: results.length,
    results,
  };
}

/**
 * The cover brief, as the engine's parseCoverRequest wants it.
 *
 * The four answers are an OBJECT on the engine's side. The client sends them
 * as JSON text, so the text is parsed here rather than in the engine, which
 * has no reason to know how a client chooses to carry them.
 *
 * @param {unknown} raw
 * @returns {unknown}
 */
function parseBrief(raw) {
  if (raw === null || raw === undefined) return null;
  if (typeof raw === "object") return raw;
  if (typeof raw !== "string") return null;
  const trimmed = raw.trim();
  if (!trimmed.startsWith("{")) return null;
  try {
    return JSON.parse(trimmed);
  } catch {
    return null;
  }
}

/**
 * The tailored CV's job order, against cv.md's.
 *
 * The engine's own guard, run on the HTML the agent produced and BEFORE the
 * PDF is rendered: a CV whose roles are out of order is not a PDF the user
 * should be told is ready.
 *
 * `validateCvExperienceOrder()` returns undefined when the order is fine and
 * THROWS when it is not, so a throw is the violation and a quiet return is the
 * pass. A guard that cannot read the CV is not a pass either.
 *
 * @param {string} html
 * @returns {Promise<{ok: boolean, message: string}>}
 */
async function checkCvOrder(html) {
  const script = rootScript("cv-experience-order");
  if (!fs.existsSync(script)) {
    // An older checkout without the guard is not a reason to refuse a render.
    return { ok: true, message: "" };
  }
  const { validateCvExperienceOrder } = await import(pathToFileURL(script).href);
  try {
    validateCvExperienceOrder(html);
    return { ok: true, message: "" };
  } catch (err) {
    return {
      ok: false,
      message:
        err instanceof Error && err.message
          ? err.message
          : "The tailored CV's roles are not in cv.md's order.",
    };
  }
}

/**
 * One worker run.
 *
 * @param {{kind?: string, input?: string, cliId?: string, brief?: unknown}} params
 * @param {{emit: (event: unknown) => void}} ctx
 */
async function runOne(params, ctx) {
  const kind = params.kind ?? "evaluate";
  const input = typeof params.input === "string" ? params.input.trim() : "";
  const requestedCliId = typeof params.cliId === "string" ? params.cliId : "";
  if (!input || !requestedCliId) {
    throw new BridgeError("bad-request", "input and cliId required");
  }

  // A MODE run: the agent follows the mode file in the checkout, and the
  // prompt that says so is the client's (see bridge/mode-prompt.mjs and
  // DECISIONS.md D-15). The mode file must be there, and it is checked FIRST:
  // a missing mode is a configuration fact the user can act on, and it is
  // more useful than a complaint about the CLI.
  const modeRun = isModeKind(kind);
  if (modeRun && !fs.existsSync(modeFile(kind))) {
    throw new BridgeError(
      "incomplete-checkout",
      `This needs the ${MODE_KINDS[kind].file} mode file; this data root has no modes.`,
    );
  }

  if (kind === "fix-portal" && !isShellSafeCompanyName(input)) {
    throw new BridgeError(
      "bad-request",
      "That company name has characters I can't safely pass to the portal checker — rename it in portals.yml first.",
    );
  }

  const resolved = resolveCliOrFallback(requestedCliId);
  if (!resolved) {
    const err = cliUnavailableError(requestedCliId);
    throw new BridgeError("cli-unavailable", err.error, { installed: err.installed });
  }
  const { spec, binPath } = resolved;
  const cliId = spec.id;
  const substitution = cliSubstitutionNotice(resolved);
  const capabilities = capabilitiesFor(kind);
  if (!isCliAllowedForCapabilities(cliId, capabilities)) {
    throw new BridgeError(
      "bad-request",
      `CLI '${cliId}' cannot run write-capable worker '${kind}' without a verified permission adapter.`,
    );
  }

  // These run the REAL core (modes/scripts), not just data.
  const lang = readLanguageConfig();
  const needsScript = {
    evaluate: lang.evalModeFile,
    "fix-portal": "verify-portals.mjs",
    pdf: "generate-pdf.mjs",
    cover: "modes/cover.md",
  };
  const required = needsScript[kind];
  const requiredPath = required ? path.join(careerOpsRoot(), required) : "";
  if (required && !fs.existsSync(requiredPath)) {
    throw new BridgeError(
      "incomplete-checkout",
      `This needs a complete career-ops checkout (${required}). CAREER_OPS_ROOT has data only — point it at a full checkout.`,
    );
  }

  if ((kind === "evaluate" || kind === "pdf" || kind === "cover") && !fs.existsSync(path.join(careerOpsRoot(), "cv.md"))) {
    throw new BridgeError(
      "bad-request",
      "Add your CV first so I can score this against you — drop it on the CV screen.",
    );
  }

  const today = localISODate();

  let pdfPaths;
  if (kind === "pdf") {
    const pathsResult = resolvePdfPaths(input, today, careerOpsRoot(), findReportFile);
    if (!pathsResult.ok) throw new BridgeError("bad-request", pathsResult.error);
    pdfPaths = pathsResult.paths;
  }

  const coverRequest = kind === "cover"
    ? parseCoverRequest({ input, brief: parseBrief(params.brief), root: careerOpsRoot(), findReportFile })
    : undefined;
  if (coverRequest && !coverRequest.ok) {
    throw new BridgeError("bad-request", coverRequest.error);
  }

  const postedAt =
    kind === "evaluate"
      ? readInbox().find((j) => j.url === input)?.postedAt ?? readScanDates().get(input)
      : undefined;
  const cvTemplate = kind === "pdf" ? await resolveCvTemplate(careerOpsRoot()) : undefined;
  const prompt = modeRun
    ? await buildModePrompt(kind, input, { today })
    : buildPrompt({
        kind,
        input,
        memory: readMemory(),
        today,
        postedAt,
        lang,
        cvTemplate,
        reportFile: pdfPaths?.reportFile ?? (coverRequest?.ok ? coverRequest.reportFile : undefined),
        coverBrief: coverRequest?.ok ? coverRequest.brief : undefined,
      });

  const isClaude = cliId === "claude";
  const args = isClaude ? claudeCliArgs({ kind, prompt }) : (spec.streamArgs ?? spec.args)(prompt);

  const writeToken = kind === "evaluate" || kind === "pdf" ? acquireTrackerWrite() : null;

  let child;
  try {
    child = spawnHeadlessCli(
      binPath,
      args,
      { cwd: careerOpsRoot(), env: process.env },
      { cliId, capabilities },
    );
  } catch (e) {
    if (writeToken !== null) releaseTrackerWrite(writeToken);
    throw new BridgeError("spawn-failed", e instanceof Error ? e.message : "failed to start the CLI");
  }

  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");

  let closed = false;
  let killer;
  let heartbeat;
  let pdfRenderPromise = null;
  let evaluatePersistPromise = null;
  const finishRun = createRunFinalizer(child, () => {
    if (writeToken !== null) releaseTrackerWrite(writeToken);
  });

  const send = (obj) => {
    if (closed) return;
    try {
      ctx.emit(obj);
    } catch {
      closed = true;
      if (heartbeat) clearInterval(heartbeat);
    }
  };

  return await new Promise((resolve) => {
    let buf = "";
    let emittedText = false;
    let sawError = false;
    let stderrBuf = "";
    const isFatalStderr = spec.stderrIsFatal ?? isFatalGenericStderr;
    let stderrErrorSnippet = null;
    const flagStderrLine = (line) => {
      if (stderrErrorSnippet || !line.trim() || !isFatalStderr(line)) return;
      stderrErrorSnippet = line.trim().slice(0, 200);
    };
    let lastTokens = 0;
    let lastCostUsd = null;
    const killMs = killMsForKind(kind);
    let killedByTimeout = false;
    killer = setTimeout(() => {
      killedByTimeout = true;
      try { child.kill("SIGTERM"); } catch { /* ignore */ }
    }, killMs);

    const fencing = fencingReport({ cliId, cliName: spec.name, capabilities: capabilitiesFor(kind) });
    if (fencing.notice) send({ type: "status", label: fencing.notice });
    if (substitution) send({ type: "status", label: substitution });
    heartbeat = setInterval(() => send({ type: "keepalive" }), 10_000);

    const close = (result) => {
      finishRun();
      if (!closed) {
        closed = true;
        if (heartbeat) clearInterval(heartbeat);
        if (killer) clearTimeout(killer);
      }
      resolve(result);
    };

    const cvFilter = kind === "pdf" ? createCvEnvelopeFilter() : null;
    const reportFilter = kind === "evaluate" ? createReportEnvelopeFilter() : null;

    const sendAgentText = (text) => {
      const filter = cvFilter ?? reportFilter;
      const visible = filter ? filter.push(text) : text;
      if (visible) send({ type: "text", text: visible });
    };
    const sendWarnings = (warnings) => {
      for (const w of warnings) send({ type: "text", text: `⚠️ ${w}\n` });
    };
    const saveCv = (paths, envelope) => {
      const written = writeCvHtml({ pdfPaths: paths, html: envelope.html });
      if (!written.ok) send({ type: "error", msg: written.error.slice(0, 200) });
      return written.ok;
    };

    const joinAgentText = createAgentTextJoiner();
    const processParsedLine = (line) => {
      if (!spec.parseEvent) return;
      const ev = spec.parseEvent(line);
      const text = joinAgentText(ev);
      if (text) {
        emittedText = true;
        sendAgentText(text);
      }
      if (ev?.tool) send({ type: "tool", name: ev.tool });
      if (ev?.status) send({ type: "status", label: ev.status });
      lastTokens = accumulateTokens(lastTokens, ev);
      if (typeof ev?.costUsd === "number") lastCostUsd = ev.costUsd;
      if (ev?.error) {
        sawError = true;
        send({ type: "error", msg: ev.error.slice(0, 200) });
      }
    };

    child.stdout.on("data", (chunk) => {
      if (closed) return;
      if (!spec.parseEvent) {
        emittedText = true;
        sendAgentText(chunk);
        return;
      }
      buf += chunk;
      let nl;
      while ((nl = buf.indexOf("\n")) !== -1) {
        const line = buf.slice(0, nl).trim();
        buf = buf.slice(nl + 1);
        if (line) processParsedLine(line);
      }
    });

    child.stderr.on("data", (chunk) => {
      stderrBuf += chunk;
      let nl;
      while ((nl = stderrBuf.indexOf("\n")) !== -1) {
        const line = stderrBuf.slice(0, nl);
        stderrBuf = stderrBuf.slice(nl + 1);
        flagStderrLine(line);
      }
    });

    const persistEval = async (markdown) => {
      send({ type: "status", label: "Saving report…" });
      // Declared outside the try so the finally below can report the number
      // the persist actually claimed. A `const` inside the try would be out of
      // scope there, and the run's answer would lose the number the user most
      // needs — which report was just written.
      let reportNum = null;
      try {
        const result = await persistEvaluation({
          spawnFn: spawn,
          execPath: process.execPath,
          root: careerOpsRoot(),
          markdown,
          url: input,
          today,
          postedAt,
        });
        if (!result.ok) {
          send({ type: "error", msg: result.error.slice(0, 200) });
          return;
        }
        sendWarnings(result.warnings);
        const reportN = Number(result.num);
        if (Number.isFinite(reportN)) {
          reportNum = result.num;
          send({ type: "done", tokens: lastTokens, costUsd: lastCostUsd, reportN });
        } else {
          send({ type: "done", tokens: lastTokens, costUsd: lastCostUsd });
        }
      } catch (e) {
        send({ type: "error", msg: `Evaluation persist crashed unexpectedly: ${e instanceof Error ? e.message : String(e)}`.slice(0, 200) });
      } finally {
        close({ ok: reportNum !== null, kind, reportN: reportNum });
      }
    };

    const renderPdf = async (paths, format) => {
      send({ type: "status", label: "Rendering PDF…" });
      try {
        const result = await renderAndMarkPdf({
          spawnFn: spawn,
          execPath: process.execPath,
          root: careerOpsRoot(),
          pdfPaths: paths,
          format,
          reportNum: paths.reportNum,
        });
        if (result.kind === "render-failed") {
          send({ type: "error", msg: result.error.slice(0, 200) });
          return;
        }
        sendWarnings(result.warnings);
        send({ type: "done", tokens: lastTokens, costUsd: lastCostUsd });
      } catch (e) {
        send({ type: "error", msg: `PDF rendering crashed unexpectedly: ${e instanceof Error ? e.message : String(e)}`.slice(0, 200) });
      } finally {
        close({ ok: true, kind, pdf: paths.finalPdf ?? null });
      }
    };

    child.on("error", (e) => {
      send({ type: "error", msg: e.message });
      close({ ok: false, kind, error: e.message });
    });

    // The body is async because the pdf branch checks the tailored CV's order
    // against cv.md before it renders. A rejection inside an async listener
    // would leave the run's promise un-settled and the screen waiting
    // forever, so the listener is registered with a catch that closes it.
    const handleClose = async (code) => {
      if (stderrBuf) { flagStderrLine(stderrBuf); stderrBuf = ""; }
      if (closed) return close({ ok: false, kind, error: "client gone" });
      if (killedByTimeout) {
        send({ type: "error", msg: timeoutMessage(killMs, kind) });
        return close({ ok: false, kind, error: "timeout" });
      }
      const trailing = buf.trim();
      if (trailing) {
        buf = "";
        processParsedLine(trailing);
      }
      const cleanExit = code === 0;
      const noOutputError = () => {
        if (!emittedText && !sawError && !cleanExit) {
          const detail = stderrErrorSnippet ? ` (${stderrErrorSnippet})` : "";
          return `The CLI exited with an error — is it installed and authenticated?${detail}`;
        }
        if (!emittedText && !sawError) return "The CLI produced no output — is it installed and authenticated? (career-ops is best on Claude Code.)";
        return null;
      };

      if (kind === "pdf") {
        const tail = cvFilter?.flush();
        if (tail) send({ type: "text", text: tail });
        const envelope = cvFilter?.result();
        const outcome = pdfRunOutcome({
          envelope,
          noOutputMessage: noOutputError(),
          sawError,
          cleanExit,
          hasPaths: pdfPaths !== undefined,
        });
        if (!outcome.ok) {
          send({ type: "error", msg: outcome.message });
        } else if (!pdfPaths || envelope?.ok !== true) {
          send({ type: "error", msg: "Internal error: the pdf run passed its gate with no CV to save — please report this." });
        } else {
          sendWarnings(envelope.warnings);
          if (saveCv(pdfPaths, envelope)) {
            // ORDER FIRST, render second: the engine's guard reads the HTML
            // the agent produced, and a reordered CV is reported to the user
            // before a PDF exists for them to send.
            const order = await checkCvOrder(envelope.html);
            if (!order.ok) {
              send({ type: "error", msg: order.message.slice(0, 200) });
              return close({ ok: false, kind, error: "out-of-order CV" });
            }
            pdfRenderPromise = renderPdf(pdfPaths, envelope.format);
            return;
          }
        }
        return close({ ok: false, kind, error: "pdf run did not produce a CV" });
      }

      if (kind === "evaluate") {
        const tail = reportFilter?.flush();
        if (tail) send({ type: "text", text: tail });
        const envelope = reportFilter?.result();
        const outcome = evaluateRunOutcome({
          envelope,
          noOutputMessage: noOutputError(),
          sawError,
          cleanExit,
        });
        if (!outcome.ok) {
          send({ type: "error", msg: outcome.message });
        } else if (envelope?.ok !== true) {
          send({ type: "error", msg: "Internal error: the evaluate run passed its gate with no report to save — please report this." });
        } else {
          evaluatePersistPromise = persistEval(envelope.markdown);
          return;
        }
        return close({ ok: false, kind, error: "evaluate run did not produce a report" });
      }

      const baseErr = noOutputError();
      if (baseErr) {
        send({ type: "error", msg: baseErr });
      } else if (!cleanExit || sawError) {
        const detail = !sawError && stderrErrorSnippet ? ` (${stderrErrorSnippet})` : "";
        send({ type: "error", msg: `This run hit an error before finishing, so it isn't recorded as a confident result — re-run it to verify.${detail}`.slice(0, 200) });
      } else {
        send({ type: "done", tokens: lastTokens, costUsd: lastCostUsd });
      }
      close({ ok: true, kind, tokens: lastTokens, costUsd: lastCostUsd });
    };

    child.on("close", (code) => {
      handleClose(code).catch((err) => {
        if (closed) return;
        send({ type: "error", msg: String(err?.message ?? err).slice(0, 200) });
        close({ ok: false, kind, error: "the run crashed while finishing" });
      });
    });
  });
}

export const op = "run";
