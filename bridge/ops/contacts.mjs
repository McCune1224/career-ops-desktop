// contacts — the job-search phonebook, the warm-introduction lookup and the
// interview-invite matcher.
//
// contacts.mjs and linkedin-join.mjs are the core's own; invite-match.mjs is
// the core's own too. Nothing here writes the phonebook: a contact is added
// through the CLI's own contacto mode, and this client only reads it, exports
// it and matches against it.
import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

import { careerOpsRoot, rootScript } from "../engine.mjs";
import { runCoreScript } from "../core-script.mjs";
import { BridgeError, parseJsonOutput } from "../util.mjs";

/**
 * @param {{action?: string, company?: string, invite?: string, write?: boolean}} params
 */
export async function handle(params) {
  const action = params.action ?? "list";
  const root = careerOpsRoot();

  if (action === "list") {
    const res = await runCoreScript("contacts", [], { json: true, timeout: 60_000 });
    const data = res.data ?? {};
    return {
      available: res.code === 0,
      contacts: data.contacts ?? [],
      quality: data.quality ?? null,
      total: data.total ?? 0,
      output: res.output,
    };
  }

  if (action === "export") {
    // contacts.mjs --vcf refuses a relative path outside the data root and
    // writes output/contacts.vcf by default.
    const args = ["--vcf"];
    const target = path.join(root, "output", "contacts.vcf");
    if (params.write === true) args.push(target);
    const res = await runCoreScript("contacts", args, { timeout: 60_000 });
    return { ok: res.code === 0, path: params.write === true ? target : null, output: res.output };
  }

  if (action === "linkedin") {
    const company = String(params.company ?? "").trim();
    if (!company) throw new BridgeError("bad-request", "company required");
    const res = await runCoreScript("linkedin-join", ["--company", company], {
      json: true,
      timeout: 60_000,
    });
    const data = res.data ?? {};
    return {
      available: res.code === 0,
      targets: data.targets ?? [],
      totals: data.totals ?? null,
      output: res.output,
    };
  }

  if (action === "inviteMatch") {
    const invite = String(params.invite ?? "").trim();
    if (!invite) throw new BridgeError("bad-request", "invite required");
    const script = rootScript("invite-match");
    if (!fs.existsSync(script)) {
      throw new BridgeError(
        "core-script-missing",
        "matching an invite needs the career-ops scripts; this root has data only",
      );
    }
    const res = await runInvite(script, invite, root);
    return {
      available: res.code === 0,
      classification: res.data?.classification ?? null,
      signals: res.data?.signals ?? null,
      candidates: res.data?.candidates ?? [],
      output: res.output,
    };
  }

  throw new BridgeError("bad-request", `unknown contacts action: ${action}`);
}

/**
 * Run invite-match.mjs with the invite text on its standard input.
 *
 * The script reads the invite from stdin, which is why it is spawned here
 * rather than through the argv runner.
 *
 * @param {string} script
 * @param {string} invite
 * @param {string} cwd
 */
function runInvite(script, invite, cwd) {
  return new Promise((resolve) => {
    const child = spawn(process.execPath, [script, "--json"], {
      cwd,
      env: { ...process.env },
      stdio: ["pipe", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("close", (code) => {
      resolve({ code, output: `${stdout}\n${stderr}`.trim(), data: parseJsonOutput(stdout) });
    });
    child.on("error", () => {
      resolve({ code: -1, output: "the invite matcher did not run", data: null });
    });
    child.stdin.end(invite);
  });
}

export const op = "contacts";
