// The core's own CV markdown parser, reached by path.
//
// lib/cv-markdown.mjs is a CORE module: the web app cannot import it
// statically, and a second copy of the Pandoc rules would drift from the
// checks that use them (web/src/lib/cv-studio/base-cv.mjs says the same).
//
// So the client imports the core's file rather than writing its own: one
// definition of what a CV heading means, in the place that owns it.
import path from "node:path";
import { pathToFileURL } from "node:url";

import { rootScript } from "./engine.mjs";

let cached = null;

/**
 * The core's markdown helpers, loaded once.
 *
 * @returns {Promise<{findExperienceSections: Function, parseCompanyHeading: Function}>}
 */
export async function coreCvMarkdown() {
  if (cached) return cached;
  const script = path.join(path.dirname(rootScript("doctor")), "lib", "cv-markdown.mjs");
  const module = await import(pathToFileURL(script).href);
  cached = {
    findExperienceSections: module.findExperienceSections,
    parseCompanyHeading: module.parseCompanyHeading,
  };
  return cached;
}

/**
 * The experience sections of a CV, as the core reads them.
 *
 * @param {string} cvText
 * @returns {Promise<string[][]>}
 */
export async function findExperienceSections(cvText) {
  const { findExperienceSections: parse } = await coreCvMarkdown();
  return parse(cvText);
}

/**
 * One `### Company — Location` heading, as the core reads it.
 *
 * @param {string} line
 * @returns {{company: string, location: string} | null}
 */
export async function parseCompanyHeading(line) {
  const { parseCompanyHeading: parse } = await coreCvMarkdown();
  return parse(line);
}
