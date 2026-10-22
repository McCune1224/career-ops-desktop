// pipeline — the tracker rows and the inbox, the same read the web app's
// GET /api/pipeline performs.
//
// Sorting and filtering happen in the client (the Rust screen), exactly as they
// do in the web app's client component: the server hands over the parsed rows
// and the view decides the order. That keeps ONE implementation of the parse
// (tracker-table.mjs) and lets the desktop screen offer the same sort keys.
import { careerOpsRoot, readInbox, readApplications, WEB_FIELD_NAMES } from "../engine.mjs";
import fs from "node:fs";
import path from "node:path";

/**
 * @param {Record<string, unknown>} params
 */
export async function handle(params) {
  const root = careerOpsRoot();
  const inbox = readInbox();

  // The tracker file may sit at <root>/data/applications.md or <root>/applications.md.
  // readApplications() already resolves it through the shared tracker-files.mjs
  // reader, so the rows come from the canonical path with no second guess.
  const applications = readApplications();

  const trackerPath = fs.existsSync(path.join(root, "data", "applications.md"))
    ? path.join(root, "data", "applications.md")
    : path.join(root, "applications.md");

  return {
    root,
    rootExists: fs.existsSync(root),
    trackerPath,
    inbox,
    applications,
    fieldNames: WEB_FIELD_NAMES,
  };
}

export const op = "pipeline";
