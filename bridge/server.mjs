#!/usr/bin/env node
// The desktop app's Node bridge.
//
// The native (egui) client is the UI; this is the engine. It speaks
// newline-delimited JSON on stdio: one request per line in, one response (or
// stream of events, then one response) per request out.
//
// Why stdio and not a loopback socket: the bridge has no reason to be
// reachable by anything but the app that started it. A socket would need a
// port, an auth story and a bind rule; stdio has none of those, and the
// protocol is the same either way. The app spawns this process and owns it.
//
// The bridge NEVER listens on a socket. Nothing here binds an address.
//
// Protocol
//   request  {"id": 1, "op": "pipeline", "params": {}}
//   event    {"id": 1, "event": {"type": "text", "text": "..."}}
//   response {"id": 1, "ok": true, "data": {...}}
//            {"id": 1, "ok": false, "error": {"code": "...", "message": "..."}}
//
// The event shape is the web app's NDJSON run stream verbatim (text / tool /
// status / keepalive / error / done), so the desktop run screen renders the
// same steps the web run screen does.
import { ENGINE_CHECKOUT } from "./alias-hook.mjs";
import { writeLine, readLines } from "./util.mjs";
import { pathToFileURL } from "node:url";
import path from "node:path";
import { fileURLToPath } from "node:url";

// The engine checkout is where the root scripts live (doctor.mjs,
// set-status.mjs, merge-tracker.mjs, …). It is NOT the data root: a user can
// point CAREER_OPS_ROOT at a data-only directory, and the scripts still have
// to come from a real checkout.
//
// It is resolved by the same rule the alias hook uses — CAREER_OPS_CODE_ROOT,
// else a sibling checkout, else one actionable sentence — so the two can never
// disagree about which checkout the bridge is loading the engine from.
// career-ops.ts's own resolveCodeRoot falls back to `path.resolve(cwd, "..")`,
// which is only right when the process was started from `<checkout>/web`; the
// explicit variable removes the guess entirely.
const BRIDGE_DIR = path.dirname(fileURLToPath(import.meta.url));
if (!process.env.CAREER_OPS_CODE_ROOT) {
  process.env.CAREER_OPS_CODE_ROOT = ENGINE_CHECKOUT;
}

const { OPS } = await import("./dispatch.mjs");

/**
 * One request at a time? No — the app can have a run streaming while it also
 * refreshes the tracker, so requests are handled concurrently and every
 * response carries the request's id.
 */
function write(event) {
  writeLine(process.stdout, event);
}

async function handleRequest(request) {
  const id = request.id ?? null;
  const op = request.op;
  const params = request.params ?? {};
  const handler = OPS[op];
  if (!handler) {
    write({ id, ok: false, error: { code: "unknown-op", message: `unknown op: ${op}` } });
    return;
  }
  const ctx = {
    emit: (event) => write({ id, event }),
  };
  try {
    const data = await handler(params, ctx);
    write({ id, ok: true, data: data ?? null });
  } catch (err) {
    const code = err && typeof err.code === "string" ? err.code : "failed";
    const message = err instanceof Error ? err.message : String(err);
    // The child's stderr is a Node stack trace carrying absolute paths; it is
    // logged, never sent to the screen.
    if (code === "failed") console.error(`bridge op ${op} failed: ${err?.stack ?? message}`);
    write({ id, ok: false, error: { code, message } });
  }
}

readLines(process.stdin, (value) => {
  if (value && value.__parseError) {
    write({ id: null, ok: false, error: { code: "bad-request", message: "malformed request line" } });
    return;
  }
  void handleRequest(value);
});

process.on("SIGTERM", () => process.exit(0));
process.on("SIGINT", () => process.exit(0));
