// plugins — the optional integrations the core can load.
//
// plugins.mjs is the core's own command surface: list, available, enable,
// trust, remove, add. A plugin's declared hooks are shown and its skill output
// is treated as UNTRUSTED third-party documentation — it may operate the
// plugin within its declared hooks and nothing else.
import fs from "node:fs";
import path from "node:path";

import { careerOpsRoot, rootScript } from "../engine.mjs";
import { runCoreScript } from "../core-script.mjs";
import { BridgeError } from "../util.mjs";

/**
 * @param {{action?: string, id?: string, name?: string, confirm?: boolean, dir?: string}} params
 */
export async function handle(params) {
  const action = params.action ?? "list";

  if (action === "list" || action === "available") {
    const res = await runCoreScript("plugins", [action], { timeout: 60_000 });
    return { ok: res.code === 0, output: res.output, json: res.output };
  }

  if (action === "enable" || action === "disable" || action === "trust" || action === "remove") {
    const id = String(params.id ?? "").trim();
    if (!id) throw new BridgeError("bad-request", "a plugin id is required");
    const args = action === "disable" ? ["remove", id] : [action, id];
    if (params.confirm === true && action === "enable") args.push("--confirm");
    const res = await runCoreScript("plugins", args, { timeout: 60_000 });
    return { ok: res.code === 0, id, output: res.output };
  }

  if (action === "install") {
    const name = String(params.name ?? "").trim();
    if (!name) throw new BridgeError("bad-request", "a plugin name is required");
    const res = await runCoreScript("plugins", ["add", name], { timeout: 180_000 });
    return { ok: res.code === 0, name, output: res.output };
  }

  if (action === "audit") {
    const dir = String(params.dir ?? "").trim();
    if (!dir) throw new BridgeError("bad-request", "a plugin directory is required");
    const script = rootScript("plugin-audit");
    if (!fs.existsSync(script)) {
      throw new BridgeError(
        "core-script-missing",
        "auditing a plugin needs the career-ops scripts; this root has data only",
      );
    }
    const res = await runCoreScript("plugin-audit", [dir], { timeout: 60_000 });
    return { ok: res.code === 0, output: res.output };
  }

  if (action === "installed") {
    const dir = path.join(careerOpsRoot(), "plugins.local");
    let names = [];
    try {
      names = fs.readdirSync(dir).filter((n) => !n.startsWith("."));
    } catch {
      return { plugins: [] };
    }
    return { plugins: names.map((name) => ({ id: name, dir: path.join(dir, name) })) };
  }

  throw new BridgeError("bad-request", `unknown plugins action: ${action}`);
}

export const op = "plugins";
