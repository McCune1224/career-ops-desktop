// Resolves the "@/…" import alias that web/src's TypeScript modules use, so the
// desktop bridge can load the web app's own engine modules under plain Node.
//
// The web app gets this for free from webpack/Turbopack (web/tsconfig.json
// declares `paths: { "@/*": ["./src/*"] }`). Plain Node does not, so the bridge
// installs the same mapping as an ESM resolve hook. The mapping is byte-for-byte
// the web app's own — this hook only makes the alias resolvable, it never
// changes what a module means.
//
// Node 22 already strips types from .ts on import, so this hook is about
// SPECIFIER RESOLUTION only, not transpilation. The web test suite solves the
// same problem in web/tests/helpers/web-ts-alias-loader.mjs; the two must agree.
//
// The alias root is the ENGINE CHECKOUT's web/src — not a directory near this
// file. The checkout is named explicitly (CAREER_OPS_CODE_ROOT, else a sibling
// checkout, else one actionable sentence); see code-root.mjs.
//
// Import order matters. ESM resolves every static specifier in a module graph
// before any module body runs, so a STATIC import of an aliased module would be
// resolved before this file's body installed the hook. The bridge therefore
// imports this file for its side effect and then reaches the aliased modules
// through a DYNAMIC import (see server.mjs).
import { register } from "node:module";
import { pathToFileURL } from "node:url";
import { existsSync } from "node:fs";
import path from "node:path";

import { resolveCodeRoot, ACTIONABLE } from "./code-root.mjs";

// The engine checkout, resolved once. `resolveCodeRoot` throws the actionable
// sentence when there is none, and that is the whole error the app sees: a
// checkout that is missing is a configuration fact, not a stack trace.
const CODE_ROOT = resolveCodeRoot();

/** The checkout's web tree — the root the "@/…" alias maps into. */
export const WEB_SRC = path.join(CODE_ROOT.path, "web", "src");

/** The engine checkout the alias resolves into. */
export const ENGINE_CHECKOUT = CODE_ROOT.path;

if (!globalThis.__careerOpsDesktopAliasRegistered) {
  globalThis.__careerOpsDesktopAliasRegistered = true;
  register(import.meta.url, import.meta.url);
}

const HAS_EXT = /\.(m?[jt]sx?|json)$/i;

/**
 * A file inside the engine checkout, used as the resolution parent when a bare
 * specifier has to come from the engine's own dependencies.
 */
const ENGINE_PARENT = pathToFileURL(path.join(ENGINE_CHECKOUT, "package.json")).href;

export async function resolve(specifier, context, nextResolve) {
  if (specifier.startsWith("@/")) {
    const base = path.join(WEB_SRC, specifier.slice(2));
    // Specifiers that already name an extension resolve as-is; extensionless
    // ones try the TS convention first, then .mjs (the web tree keeps plain
    // modules in .mjs, and some imports name them without the extension).
    const candidates = HAS_EXT.test(specifier)
      ? [base]
      : [`${base}.ts`, `${base}.tsx`, `${base}.mjs`];
    // Fall back to the FIRST candidate rather than the raw specifier so Node's
    // own error names the file that is actually absent.
    const hit = candidates.find((c) => existsSync(c)) ?? candidates[0];
    return nextResolve(pathToFileURL(hit).href, context);
  }

  // A bare specifier the bridge cannot resolve on its own is resolved from the
  // engine checkout, so a dependency the engine already has is THE engine's
  // copy and not a second one. The default resolution is tried first and
  // untouched: an engine module's own imports already resolve from its own
  // directory, and re-parenting them would move `web/node_modules` out of
  // reach.
  if (!specifier.startsWith(".") && !specifier.startsWith("/") && !specifier.startsWith("node:")) {
    try {
      return await nextResolve(specifier, context);
    } catch {
      return nextResolve(specifier, { ...context, parentURL: ENGINE_PARENT });
    }
  }
  return nextResolve(specifier, context);
}

export { ACTIONABLE };
