// liveness — is a posting still open, and what does it say?
//
// Both answers come from the core: check-liveness.mjs for the verdict, and
// fetch-jd.mjs for the text when the posting's ATS has a public API.
//
// The verdict's rules are the core's: expired evidence outranks a generic
// "Apply" button, and an unreadable posting is UNCONFIRMED, never closed.
import { runCoreScript } from "../core-script.mjs";
import { BridgeError } from "../util.mjs";

/**
 * @param {{action?: string, url?: string, record?: boolean}} params
 */
export async function handle(params) {
  const action = params.action ?? "check";
  const url = String(params.url ?? "").trim();
  if (!/^https?:\/\//i.test(url)) {
    throw new BridgeError("bad-request", "a posting URL is required");
  }

  if (action === "check") {
    const args = [url];
    // The core records an expired posting in the scan history so it stops
    // resurfacing. The app asks first: recording is a write to a user-layer
    // file the user may want to keep clean.
    if (params.record !== true) args.push("--no-record");
    const res = await runCoreScript("check-liveness", args, { timeout: 120_000 });
    const text = res.output;
    const verdict = /active/i.test(text) ? "active" : /expired/i.test(text) ? "expired" : "uncertain";
    return { ok: res.code === 0, verdict, output: text };
  }

  if (action === "fetchJd") {
    const res = await runCoreScript("fetch-jd", [url], { timeout: 60_000 });
    // A miss is the EXPECTED case: the caller falls through to the browser
    // path. It is not an error, so it is not reported as one.
    return {
      ok: res.code === 0,
      found: res.code === 0 && res.stdout.trim().length > 0,
      text: res.stdout.trim(),
    };
  }

  throw new BridgeError("bad-request", `unknown liveness action: ${action}`);
}

export const op = "liveness";
