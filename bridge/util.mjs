// Small helpers the bridge ops share: JSON-line IO on stdio, and a promise
// wrapper around execFile that keeps the CHILD's exit code and stderr instead
// of collapsing them into an exception.
//
// Nothing here knows about career-ops. The engine knowledge lives in engine.mjs.
import { createInterface } from "node:readline";
import { execFile } from "node:child_process";

/**
 * Write one JSON document as a single line.
 *
 * NDJSON, not a JSON array: the run screen needs progress while the run is
 * still going, and a partial line must never be mistaken for a complete
 * document. A newline inside a string would break the framing, so it is
 * escaped by JSON.stringify itself.
 *
 * @param {NodeJS.WritableStream} stream
 * @param {unknown} value
 */
export function writeLine(stream, value) {
  stream.write(`${JSON.stringify(value)}\n`);
}

/**
 * Read newline-delimited JSON requests from a stream.
 *
 * A malformed line is reported and skipped rather than fatal: one bad request
 * must not take the whole bridge down and lose the screen's connection.
 *
 * @param {NodeJS.ReadableStream} stream
 * @param {(value: any) => void} onValue
 */
export function readLines(stream, onValue) {
  const rl = createInterface({ input: stream, crlfDelay: Infinity });
  rl.on("line", (line) => {
    const trimmed = line.trim();
    if (!trimmed) return;
    let parsed;
    try {
      parsed = JSON.parse(trimmed);
    } catch {
      onValue({ __parseError: true, raw: trimmed });
      return;
    }
    onValue(parsed);
  });
  return rl;
}

/**
 * Run a Node script and resolve with its outcome instead of throwing.
 *
 * The web app's routes use execFile and map exit codes onto HTTP statuses; the
 * bridge has no HTTP, so it keeps the exit code and the child's stderr and lets
 * each op decide what the failure means. The child's stderr is never returned
 * to the screen: it is a Node stack trace carrying absolute paths.
 *
 * @param {string} execPath - node binary.
 * @param {string[]} args
 * @param {{cwd?: string, timeout?: number, env?: NodeJS.ProcessEnv, maxBuffer?: number}} [options]
 * @returns {Promise<{code: number, stdout: string, stderr: string, timedOut: boolean, spawnFailed: boolean}>}
 */
export function runNode(execPath, args, options = {}) {
  return new Promise((resolve) => {
    execFile(
      execPath,
      args,
      {
        cwd: options.cwd,
        timeout: options.timeout,
        env: options.env ? { ...process.env, ...options.env } : process.env,
        maxBuffer: options.maxBuffer ?? 8 * 1024 * 1024,
        windowsHide: true,
      },
      (err, stdout, stderr) => {
        const e = err;
        const timedOut = e?.killed === true;
        const code = typeof e?.code === "number" ? e.code : e ? -1 : 0;
        resolve({
          code,
          stdout: stdout || "",
          stderr: stderr || "",
          timedOut,
          spawnFailed: !timedOut && code === -1,
        });
      },
    );
  });
}

/**
 * An error the bridge reports to the screen with a stable code.
 *
 * The screen renders `message`; `code` lets it tell "the tracker is busy,
 * retry" from "this is broken".
 */
export class BridgeError extends Error {
  /**
   * @param {string} code
   * @param {string} message
   * @param {Record<string, unknown>} [details]
   */
  constructor(code, message, details = {}) {
    super(message);
    this.code = code;
    this.details = details;
  }
}
