// The op table the bridge dispatches through.
//
// One module per capability area, each owning one op. The modules import the
// web app's engine (engine.mjs), never a copy of it.
import * as bootstrap from "./ops/bootstrap.mjs";
import * as pipeline from "./ops/pipeline.mjs";
import * as status from "./ops/status.mjs";
import * as tracker from "./ops/tracker.mjs";
import * as report from "./ops/report.mjs";
import * as run from "./ops/run.mjs";
import * as runs from "./ops/runs.mjs";
import * as cost from "./ops/cost.mjs";
import * as cv from "./ops/cv.mjs";
import * as followups from "./ops/followups.mjs";
import * as config from "./ops/config.mjs";
import * as inbox from "./ops/inbox.mjs";
import * as maintenance from "./ops/maintenance.mjs";
import * as analysis from "./ops/analysis.mjs";
import * as documents from "./ops/documents.mjs";
import * as portals from "./ops/portals.mjs";
import * as liveness from "./ops/liveness.mjs";
import * as interview from "./ops/interview.mjs";
import * as contacts from "./ops/contacts.mjs";
import * as outcomes from "./ops/outcomes.mjs";
import * as plugins from "./ops/plugins.mjs";
import * as profile from "./ops/profile.mjs";
import * as questions from "./ops/questions.mjs";
import * as agentInbox from "./ops/agent-inbox.mjs";
import * as version from "./ops/version.mjs";

export const OPS = {
  [bootstrap.op]: bootstrap.handle,
  [pipeline.op]: pipeline.handle,
  [status.op]: status.handle,
  [tracker.op]: tracker.handle,
  [report.op]: report.handle,
  [run.op]: run.handle,
  [runs.op]: runs.handle,
  [cost.op]: cost.handle,
  [cv.op]: cv.handle,
  [followups.op]: followups.handle,
  [config.op]: config.handle,
  [inbox.op]: inbox.handle,
  [maintenance.op]: maintenance.handle,
  [analysis.op]: analysis.handle,
  [documents.op]: documents.handle,
  [portals.op]: portals.handle,
  [liveness.op]: liveness.handle,
  [interview.op]: interview.handle,
  [contacts.op]: contacts.handle,
  [outcomes.op]: outcomes.handle,
  [plugins.op]: plugins.handle,
  [profile.op]: profile.handle,
  [questions.op]: questions.handle,
  [agentInbox.op]: agentInbox.handle,
  [version.op]: version.handle,
};
