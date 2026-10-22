// status — the ONE way a tracker row's status changes.
//
// The desktop client never edits data/applications.md itself. It delegates to
// set-status.mjs, exactly like the web app's POST /api/status does: that script
// holds the shared tracker lock across the whole read-modify-write, validates
// the state against templates/states.yml, and appends the transition ledger.
// A second writer here would race it silently.
//
// The only difference from the web route is the ledger attribution: `--source
// desktop` instead of `--source web`, so a status change made in the native
// client is distinguishable in data/status-log.tsv.
import fs from "node:fs";
import { careerOpsRoot, rootScript, canonicalizeStatus, trackerRowArg, parseCliJson, clientErrorMessage } from "../engine.mjs";
import { runNode, BridgeError } from "../util.mjs";

const SET_STATUS_TIMEOUT_MS = 30_000;
const LOCK_WAIT_DEFAULT_MS = 10_000;
const LOCK_WAIT_CEILING_MS = SET_STATUS_TIMEOUT_MS - 5_000;

// set-status.mjs exit codes (CLI_EXIT in tracker-utils.mjs).
// 4 = another writer holds the tracker lock: transient, so the caller retries.
const EXIT_CODES = {
  2: "not-found",
  3: "conflict",
  4: "busy",
};

const CLIENT_ERROR_CODES = new Set(["usage", "invalid-state"]);

/**
 * Bounded wait for the tracker's own lock.
 *
 * The CLI waits a full minute by default, which is right for a batch run and
 * wrong for an interactive screen: the user would sit there while the app's own
 * timeout killed the child, and the "busy" signal would be lost. The child's
 * wait is capped below the app's kill timeout so its exit-4 path actually
 * fires.
 */
function boundedLockWait() {
  const raw = Number(process.env.CAREER_OPS_TRACKER_LOCK_TIMEOUT_MS);
  const requested = Number.isFinite(raw) && raw > 0 ? raw : LOCK_WAIT_DEFAULT_MS;
  return { CAREER_OPS_TRACKER_LOCK_TIMEOUT_MS: String(Math.min(requested, LOCK_WAIT_CEILING_MS)) };
}

/**
 * @param {{n?: string | number, status?: string, note?: string}} params
 */
export async function handle(params) {
  const n = params.n;
  const status = typeof params.status === "string" ? params.status : "";
  if (n === undefined || n === null || !status.trim()) {
    throw new BridgeError("bad-request", "n and status required");
  }
  // Rejected before spawning: these characters would break the markdown row.
  if (/[|\r\n*]/.test(status)) {
    throw new BridgeError("bad-request", "invalid status (table-breaking characters)");
  }
  const canon = canonicalizeStatus(status);
  if (!canon) {
    throw new BridgeError("bad-request", `not a canonical status: ${status}`);
  }
  // `n` arrives as untrusted JSON; String(n) on an object would send
  // "[object Object]" to --row and return the CLI's usage text.
  const row = trackerRowArg(n);
  if (!row) {
    throw new BridgeError("bad-request", "n must be a tracker row number");
  }

  const script = rootScript("set-status");
  if (!fs.existsSync(script)) {
    throw new BridgeError(
      "core-script-missing",
      "status updates need the career-ops scripts; this root has data only",
    );
  }

  const args = ["--row", row, canon, "--source", "desktop", "--json"];
  if (typeof params.note === "string" && params.note.trim()) {
    args.push("--note", params.note.trim());
  }

  const res = await runNode(process.execPath, [script, ...args], {
    cwd: careerOpsRoot(),
    timeout: SET_STATUS_TIMEOUT_MS,
    env: boundedLockWait(),
  });

  if (res.spawnFailed) {
    throw new BridgeError("spawn-failed", "status update failed to run");
  }
  if (res.timedOut) {
    throw new BridgeError(
      "timeout",
      "status update timed out; the change may or may not have been applied",
    );
  }

  const parsed = parseCliJson(res.stdout);
  if (res.code !== 0) {
    const cliCode = typeof parsed?.code === "string" ? parsed.code : undefined;
    const kind = res.code === 1 && cliCode && CLIENT_ERROR_CODES.has(cliCode) ? "bad-request" : (EXIT_CODES[res.code] ?? "failed");
    throw new BridgeError(kind, clientErrorMessage(parsed, res.stderr), { cliCode });
  }
  if (!parsed) {
    throw new BridgeError("failed", "status update returned no result");
  }
  return {
    status: canon,
    changed: parsed.changed === true,
    statusLogged: parsed.statusLogged === true,
  };
}

export const op = "status";
