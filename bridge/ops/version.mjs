// version — what is running, and do the two halves agree?
//
// The bridge and the app speak a small protocol: an op name, a set of JSON
// shapes, and a stream vocabulary. The PROTOCOL version is what both sides
// agree on. The career-ops RELEASE version is reported next to it, because it
// is what a person reads when something does not line up.
//
// A mismatch is refused, not warned about: an app talking to a bridge it does
// not understand would show a screen that is wrong in ways nobody can see.

import fs from "node:fs";
import path from "node:path";

import { careerOpsRoot, rootScript } from "../engine.mjs";
import { BridgeError } from "../util.mjs";

/**
 * The bridge protocol this build speaks.
 *
 * Bumped when an op's request or response shape changes, or when the stream
 * vocabulary changes. The app carries the same number and refuses to run
 * against a bridge that carries another.
 */
export const PROTOCOL_VERSION = 1;

/**
 * @param {{action?: string}} params
 */
export async function handle(params) {
  const action = params.action ?? "get";
  if (action !== "get") {
    throw new BridgeError("bad-request", `unknown version action: ${action}`);
  }
  const root = careerOpsRoot();
  let engine = "";
  try {
    engine = fs.readFileSync(path.join(root, "VERSION"), "utf8").trim();
  } catch {
    engine = "";
  }
  if (!engine) {
    // The checkout's own VERSION file is the fallback: a split install keeps
    // the scripts in the checkout, not in the data root.
    try {
      engine = fs.readFileSync(path.join(path.dirname(rootScript("doctor")), "VERSION"), "utf8").trim();
    } catch {
      engine = "";
    }
  }
  return { protocol: PROTOCOL_VERSION, engine, node: process.version };
}

export const op = "version";
