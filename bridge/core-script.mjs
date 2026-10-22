// Running a CORE script from the bridge.
//
// One helper, so every op that calls a core script does it the same way: the
// script comes from the configured career-ops checkout (never from the data
// root), it runs with the data root as its working directory, and a missing
// script is a refusal rather than a silent no-op.
//
// The ops do not reimplement any of these scripts' rules. They choose which
// script to run and what to show; the script decides what it means.
import fs from "node:fs";

import { careerOpsRoot, rootScript } from "./engine.mjs";
import { runNode, BridgeError, parseJsonOutput } from "./util.mjs";

/**
 * Run a core script and return its outcome.
 *
 * @param {string} name the script's name without the `.mjs`
 * @param {string[]} [args]
 * @param {{timeout?: number, json?: boolean}} [options]
 * @returns {Promise<{code: number, stdout: string, stderr: string, output: string, data: unknown, timedOut: boolean}>}
 */
export async function runCoreScript(name, args = [], options = {}) {
  const script = rootScript(name);
  if (!fs.existsSync(script)) {
    throw new BridgeError(
      "core-script-missing",
      `${name} needs the career-ops scripts; this root has data only`,
    );
  }
  const res = await runNode(process.execPath, [script, ...args], {
    cwd: careerOpsRoot(),
    timeout: options.timeout ?? 120_000,
  });
  if (res.spawnFailed) {
    throw new BridgeError("spawn-failed", `${name} did not run`);
  }
  return {
    ...res,
    output: `${res.stdout}\n${res.stderr}`.trim(),
    data: options.json ? parseJsonOutput(res.stdout) : null,
  };
}
