// report — reading one application's evaluation report, and the links that
// hang off it (the source posting, the tailored CV PDF, the cover letter PDF).
//
// The report file is resolved through the web app's own findReportFile(), which
// follows the tracker row's report link first (report file numbers can differ
// from application numbers) and containment-checks the result. The desktop
// client does not get a second, looser resolver.
import {
  findReportFile,
  readReport,
  findApplication,
  parseReport,
  pdfPathStatusForReport,
  resolveTailoredCv,
  resolveTailoredCover,
  splitSections,
  sectionPreview,
  isLeadSection,
  firstProseParagraph,
} from "../engine.mjs";
import { BridgeError } from "../util.mjs";

/**
 * @param {{n?: string}} params
 */
export async function handle(params) {
  const n = String(params.n ?? "").trim();
  if (!n) throw new BridgeError("bad-request", "n required");

  const app = findApplication(n);
  const report = readReport(n);
  if (!app && !report) {
    throw new BridgeError("not-found", `no application or report numbered ${n}`);
  }

  const meta = report ? parseReport(report.content) : null;
  const urlField = meta?.fields.find((f) => f.label === "URL")?.value ?? null;

  // The tracker's URL cell is the other half of the truth: a report written
  // before the URL header existed still has the posting URL in the tracker.
  const trackerUrl = app?.url && app.url !== "—" ? app.url : null;

  // The report's sections, split by the web app's own splitter, so the screen
  // can show the lead and fold the rest without re-deriving what a block is.
  const body = report?.content ?? "";
  const split = splitSections(body);
  const sections = split.sections.map((section) => ({
    heading: section.heading,
    letter: section.letter,
    content: section.content,
    preview: sectionPreview(section.content),
    lead: isLeadSection(section),
  }));

  // The VERDICT line is the report's headline conclusion. The blocks fold, so
  // it is lifted out: a folded body must never hide the answer.
  const verdictLine = body
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.startsWith("VERDICT:"))
    .pop();

  const pdf = await pdfPathStatusForReport(n);
  const cvPdf = pdf.status === "found" ? pdf.path : null;
  const coverPdf = app ? await resolveTailoredCover(app.company, n) : null;
  const cvHtml = app ? await resolveTailoredCv(app.company, n) : null;

  return {
    n,
    application: app,
    file: report?.file ?? null,
    markdown: report?.content ?? null,
    title: meta?.title ?? null,
    legitimacy: meta?.legitimacy ?? null,
    fields: meta?.fields ?? [],
    intro: split.intro,
    lead: firstProseParagraph(body),
    verdict: verdictLine ?? null,
    sections,
    postingUrl: urlField ?? trackerUrl,
    trackerUrl,
    cvPdf,
    cvHtml,
    coverPdf,
  };
}

export const op = "report";
