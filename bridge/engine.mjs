// The desktop bridge's ONE import surface onto the web app's engine.
//
// Every module the bridge uses is imported HERE and nowhere else, so:
//
//   1. There is exactly one implementation of the engine rules. The bridge does
//      not reimplement spawning, CLI fencing, envelope parsing or report
//      persistence — it calls the very modules the web app's API routes call.
//   2. The parity guard has a single place to look. tests/desktop-engine-parity
//      .test.mjs asserts that every engine module named here is the same file
//      the web route imports, so the two clients cannot drift apart silently.
//
// Rule 1 of web/AGENTS.md — "Orchestrate the core; never reimplement it" —
// applies to this bridge exactly as it applies to the web app.
//
// The "@/…" specifiers below are resolved by alias-hook.mjs, which the bridge's
// server installs before this module is imported.
import {
  careerOpsRoot,
  rootScript,
  readApplications,
  readInbox,
  readScanDates,
  readMemory,
  readLanguageConfig,
  findReportFile,
  readReport,
  findApplication,
  pdfReadyForReport,
  doctorState,
  pipelineSummary,
  pdfPathStatusForReport,
} from "@/lib/career-ops";
import { canonicalizeStatus, canonicalLabels } from "@/lib/core/states";
import { parseApplications, WEB_FIELD_NAMES } from "@/lib/tracker-table.mjs";
import { parseInbox } from "@/lib/pipeline-table.mjs";
import { parseCliJson, trackerRowArg, clientErrorMessage } from "@/lib/status-cli.mjs";
import { atomicWrite, atomicWriteWithBackup } from "@/lib/core/safe-write";
import { acquireTrackerWrite, releaseTrackerWrite } from "@/lib/core/run-registry";
import {
  resolveCliOrFallback,
  cliUnavailableError,
  cliSubstitutionNotice,
  detectClis,
} from "@/lib/clis";
import { pickDefaultInstalled, keepIfInstalled } from "@/lib/cli-pick.mjs";
import { capabilitiesFor } from "@/lib/worker-capabilities.mjs";
import { isCliAllowedForCapabilities, fencingReport } from "@/lib/cli-fencing.mjs";
import { spawnHeadlessCli } from "@/lib/spawn-cli.mjs";
import { claudeCliArgs } from "@/lib/claude-invocation.mjs";
import {
  codexStreamArgs,
  parseClaudeEvent,
  parseCodexEvent,
  parseGrokEvent,
  accumulateTokens,
  createAgentTextJoiner,
  killMsForKind,
  timeoutMessage,
  isFatalClaudeStderr,
  isFatalCodexStderr,
  isFatalGenericStderr,
} from "@/lib/run-cli-support.mjs";
import { createReportEnvelopeFilter } from "@/lib/report-envelope.mjs";
import { createCvEnvelopeFilter } from "@/lib/cv-envelope.mjs";
import { persistEvaluation, evaluateRunOutcome, parseReportMeta } from "@/lib/report-persist.mjs";
import { pdfRunOutcome, renderAndMarkPdf, writeCvHtml } from "@/lib/pdf-render.mjs";
import { buildPrompt, isShellSafeCompanyName } from "@/lib/run-prompts.mjs";
import { resolvePdfPaths } from "@/lib/pdf-paths.mjs";
import { parseCoverRequest } from "@/lib/cover-brief.mjs";
import { resolveCvTemplate } from "@/lib/core/cv-template.mjs";
import { createRunFinalizer } from "@/lib/run-finalizer.mjs";
import {
  selectDueFollowups,
  selectUpcomingFollowups,
  pickNextUpcoming,
} from "@/lib/core/followup-view.mjs";
import { CHANNELS, isRealISODate, localISODate, PROFILE_CADENCE_KEYS } from "@/lib/followups";
import { parseFollowupId } from "@/lib/followup-id.mjs";
import { followupsLogPath, withFollowupsWrite, readFollowupsEnabled } from "@/lib/followups-server";
import { followupsEnabledFromProfile } from "@/lib/followups-setting.mjs";
import { setInboxSkip } from "@/lib/inbox-skip.mjs";
import { resolveTailoredCv } from "@/lib/apply/cv";
import { resolveTailoredCover } from "@/lib/apply/cover";
import { profilePatchError } from "@/lib/profile-patch.mjs";
import { isMapping } from "@/lib/portals-config.mjs";
import { parseReport, CANONICAL_STATES, scoreNum, scoreTone, legitimacyTone } from "@/lib/format";
import {
  estimateRunCost,
  formatTokenCount,
  formatRunCostEstimate,
  requiresBatchSpendConfirmation,
  formatBatchSpendConfirmation,
} from "@/lib/run-cost-estimate.mjs";
import { claudeUsageTokens } from "@/lib/claude-usage.mjs";
import { readMachineSummary, scoreLevers } from "@/lib/machine-summary.mjs";
import { splitTiers, coverageModel, coverageBand } from "@/lib/keyword-coverage.mjs";
import { skillGapModel, barPct, blockerText } from "@/lib/skill-gaps.mjs";
import { companyCardModel, verdictFor, isConclusion, factLine } from "@/lib/company-history.mjs";
import { quietModel, checkRan } from "@/lib/quiet-companies.mjs";
import { COVER_TONES } from "@/lib/cover-tones.mjs";
import { profileTargetKeywords } from "@/lib/profile-keywords.mjs";
import {
  readPersonalizationState,
  savePersonalization,
  mergeSections,
  templatePath,
  UnsureProfileParseError,
} from "@/lib/personalization-file.mjs";
import { compareIds, summaryFields, COMPARE_MAX } from "@/lib/pipeline-compare.mjs";
import { discoverDocuments, encodeId, decodeId } from "@/lib/documents";
import { collectWhatsNew, resolveOfferLimit } from "@/lib/whats-new.mjs";
import { evaluatedKeys, isEvaluated } from "@/lib/whats-new-suppression.mjs";
import { getNormalizeTextKey } from "@/lib/core/text-key";
import { loadPortalsDocument, mergePortalFilters, watchedCompanies } from "@/lib/portals-config.mjs";
import { htmlText } from "@/lib/cv-studio/html-blocks.mjs";
import {
  splitSections,
  sectionPreview,
  isLeadSection,
  firstProseParagraph,
  cleanHeading,
} from "@/lib/report-sections.mjs";

export {
  careerOpsRoot,
  rootScript,
  readApplications,
  readInbox,
  readScanDates,
  readMemory,
  readLanguageConfig,
  findReportFile,
  readReport,
  findApplication,
  pdfReadyForReport,
  doctorState,
  pipelineSummary,
  pdfPathStatusForReport,
  canonicalizeStatus,
  canonicalLabels,
  parseApplications,
  WEB_FIELD_NAMES,
  parseInbox,
  parseCliJson,
  trackerRowArg,
  clientErrorMessage,
  atomicWrite,
  atomicWriteWithBackup,
  acquireTrackerWrite,
  releaseTrackerWrite,
  resolveCliOrFallback,
  cliUnavailableError,
  cliSubstitutionNotice,
  detectClis,
  pickDefaultInstalled,
  keepIfInstalled,
  capabilitiesFor,
  isCliAllowedForCapabilities,
  fencingReport,
  spawnHeadlessCli,
  claudeCliArgs,
  codexStreamArgs,
  parseClaudeEvent,
  parseCodexEvent,
  parseGrokEvent,
  accumulateTokens,
  createAgentTextJoiner,
  killMsForKind,
  timeoutMessage,
  isFatalClaudeStderr,
  isFatalCodexStderr,
  isFatalGenericStderr,
  createReportEnvelopeFilter,
  createCvEnvelopeFilter,
  persistEvaluation,
  evaluateRunOutcome,
  parseReportMeta,
  pdfRunOutcome,
  renderAndMarkPdf,
  writeCvHtml,
  buildPrompt,
  isShellSafeCompanyName,
  resolvePdfPaths,
  parseCoverRequest,
  resolveCvTemplate,
  createRunFinalizer,
  selectDueFollowups,
  selectUpcomingFollowups,
  pickNextUpcoming,
  CHANNELS,
  isRealISODate,
  localISODate,
  PROFILE_CADENCE_KEYS,
  parseFollowupId,
  followupsLogPath,
  withFollowupsWrite,
  readFollowupsEnabled,
  followupsEnabledFromProfile,
  setInboxSkip,
  resolveTailoredCv,
  resolveTailoredCover,
  profilePatchError,
  isMapping,
  parseReport,
  CANONICAL_STATES,
  scoreNum,
  scoreTone,
  legitimacyTone,
  estimateRunCost,
  formatTokenCount,
  formatRunCostEstimate,
  requiresBatchSpendConfirmation,
  formatBatchSpendConfirmation,
  claudeUsageTokens,
  readMachineSummary,
  scoreLevers,
  splitTiers,
  coverageModel,
  coverageBand,
  skillGapModel,
  barPct,
  blockerText,
  companyCardModel,
  verdictFor,
  isConclusion,
  factLine,
  quietModel,
  checkRan,
  COVER_TONES,
  profileTargetKeywords,
  readPersonalizationState,
  savePersonalization,
  mergeSections,
  templatePath,
  UnsureProfileParseError,
  compareIds,
  summaryFields,
  COMPARE_MAX,
  discoverDocuments,
  encodeId,
  decodeId,
  collectWhatsNew,
  resolveOfferLimit,
  evaluatedKeys,
  isEvaluated,
  getNormalizeTextKey,
  loadPortalsDocument,
  mergePortalFilters,
  watchedCompanies,
  splitSections,
  sectionPreview,
  isLeadSection,
  firstProseParagraph,
  cleanHeading,
  htmlText,
};

// The engine module files this bridge depends on, by their path inside web/src.
// The parity test walks this list and asserts the web route imports the same
// files — that is what keeps the two clients on one implementation.
export const ENGINE_MODULES = [
  "lib/career-ops",
  "lib/core/states",
  "lib/tracker-table.mjs",
  "lib/pipeline-table.mjs",
  "lib/status-cli.mjs",
  "lib/core/safe-write",
  "lib/core/run-registry",
  "lib/clis",
  "lib/worker-capabilities.mjs",
  "lib/cli-fencing.mjs",
  "lib/spawn-cli.mjs",
  "lib/claude-invocation.mjs",
  "lib/run-cli-support.mjs",
  "lib/report-envelope.mjs",
  "lib/cv-envelope.mjs",
  "lib/report-persist.mjs",
  "lib/pdf-render.mjs",
  "lib/run-prompts.mjs",
  "lib/pdf-paths.mjs",
  "lib/cover-brief.mjs",
  "lib/core/cv-template.mjs",
  "lib/run-finalizer.mjs",
  "lib/core/followup-view.mjs",
  "lib/followups",
  "lib/followup-id.mjs",
  "lib/followups-server",
  "lib/followups-setting.mjs",
  "lib/inbox-skip.mjs",
  "lib/apply/cv",
  "lib/apply/cover",
  "lib/profile-patch.mjs",
  "lib/format",
  "lib/run-cost-estimate.mjs",
  "lib/claude-usage.mjs",
  "lib/machine-summary.mjs",
  "lib/keyword-coverage.mjs",
  "lib/skill-gaps.mjs",
  "lib/company-history.mjs",
  "lib/quiet-companies.mjs",
  "lib/cover-tones.mjs",
  "lib/profile-keywords.mjs",
  "lib/personalization-file.mjs",
  "lib/pipeline-compare.mjs",
  "lib/documents",
  "lib/whats-new.mjs",
  "lib/whats-new-suppression.mjs",
  "lib/core/text-key",
  "lib/portals-config.mjs",
  "lib/report-sections.mjs",
  "lib/cv-studio/html-blocks.mjs",
];
