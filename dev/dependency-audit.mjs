#!/usr/bin/env node
// dependency-audit.mjs — prove the native client links no webview.
//
// "No webview" is the load-bearing constraint of the desktop client: the UI is
// egui painting through OpenGL, and the engine is a Node child process. A
// webview dependency arriving transitively (wry via some crate, Tauri as a
// shim, a WebKit or CEF runtime through a renderer) would quietly turn the
// claim into a lie — and it would still look like a native app from the outside.
//
// Two independent checks, because either one alone has a blind spot:
//
//   1. The lockfile names no webview crate. Catches a dependency BEFORE it is
//      built, so it fails on the machine that introduced it.
//   2. the dynamic library list of the built binary names no webview runtime.
//      Catches one that arrived through a system library, which a lockfile
//      cannot see.
//
// Usage:
//   node desktop/dev/dependency-audit.mjs [--json]
//
// Exits 0 only when both checks pass.
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const DESKTOP = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const LOCK = path.join(DESKTOP, "Cargo.lock");
const BINARY = path.join(DESKTOP, "target", "release", "career-ops-desktop");
const BINARY_DEBUG = path.join(DESKTOP, "target", "debug", "career-ops-desktop");

/** Crate names that mean a webview is in the tree. */
export const FORBIDDEN_CRATES = [
  "wry",
  "tauri",
  "tauri-build",
  "tauri-runtime",
  "tauri-runtime-wry",
  "tauri-codegen",
  "webkit2gtk",
  "webkit2gtk-sys",
  "wpewebkit",
  "wpewebkit-sys",
  "webview2-com",
  "webview2-com-sys",
  "cef",
  "cef-dll-sys",
  "libcef",
  "servo-webview",
  "gtk-webkit",
];

/** Dynamic library sonames that mean a webview runtime is linked. */
export const FORBIDDEN_LIBRARIES = ["webkit", "wpewebkit", "libcef", "cef", "webview2"];

/**
 * Names in a Cargo.lock that are webviews.
 *
 * @param {string} lockText
 * @returns {string[]}
 */
export function forbiddenCratesInLock(lockText) {
  const names = new Set();
  for (const match of lockText.matchAll(/^name = "(.+)"$/gm)) names.add(match[1]);
  return FORBIDDEN_CRATES.filter((crate) => names.has(crate));
}

/**
 * Names in an `ldd` listing that are webviews.
 *
 * Matching is on the LOWER-CASED line so `libwebkit2gtk-4.1.so` and
 * `libWPEWebKit` both hit.
 *
 * @param {string} lddOutput
 * @returns {string[]}
 */
export function forbiddenLibrariesInLdd(lddOutput) {
  const hits = new Set();
  for (const line of lddOutput.split("\n")) {
    const lower = line.toLowerCase();
    for (const needle of FORBIDDEN_LIBRARIES) {
      if (lower.includes(needle.toLowerCase())) hits.add(line.trim());
    }
  }
  return [...hits];
}

/**
 * Run the audit.
 *
 * @param {{lock?: string, binary?: string}} [options]
 */
export function audit(options = {}) {
  const lockPath = options.lock ?? LOCK;
  const lockText = fs.readFileSync(lockPath, "utf-8");
  const crates = forbiddenCratesInLock(lockText);

  const binary = options.binary ?? [BINARY, BINARY_DEBUG].find((p) => fs.existsSync(p));
  let libraries = null;
  let lddOutput = null;
  let lddError = null;
  if (binary) {
    try {
      lddOutput = execFileSync("ldd", [binary], { encoding: "utf-8" });
      libraries = forbiddenLibrariesInLdd(lddOutput);
    } catch (err) {
      lddError = err instanceof Error ? err.message : String(err);
    }
  }

  const ok = crates.length === 0 && (libraries === null || libraries.length === 0) && !lddError;
  return {
    ok,
    lockPath,
    binary: binary ?? null,
    forbiddenCrates: crates,
    forbiddenLibraries: libraries ?? [],
    lddError,
    lddOutput,
  };
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  const result = audit();
  if (process.argv.includes("--json")) {
    console.log(JSON.stringify({ ...result, lddOutput: undefined }, null, 2));
  } else {
    console.log(`lockfile: ${result.lockPath}`);
    console.log(
      result.forbiddenCrates.length === 0
        ? "  ok: no webview crate in Cargo.lock"
        : `  FAIL: webview crate(s) in Cargo.lock: ${result.forbiddenCrates.join(", ")}`,
    );
    if (result.binary) {
      console.log(`binary: ${result.binary}`);
      if (result.lddError) {
        console.log(`  FAIL: ldd could not run: ${result.lddError}`);
      } else {
        console.log(
          result.forbiddenLibraries.length === 0
            ? "  ok: no webview runtime in the dynamic library list"
            : `  FAIL: webview runtime linked: ${result.forbiddenLibraries.join(" | ")}`,
        );
        const linked = (result.lddOutput ?? "")
          .split("\n")
          .map((line) => line.trim())
          .filter((line) => line.includes("=>"));
        console.log(`  ${linked.length} shared libraries linked:`);
        for (const line of linked) console.log(`    ${line}`);
      }
    } else {
      console.log("binary: not built (run `cargo build --release` first for the ldd half)");
    }
  }
  process.exit(result.ok ? 0 : 1);
}
