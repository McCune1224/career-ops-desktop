// The op table the bridge dispatches through.
//
// One module per capability area, each owning one op. The modules import the
// web app's engine (engine.mjs), never a copy of it.
import * as bootstrap from "./ops/bootstrap.mjs";
import * as pipeline from "./ops/pipeline.mjs";
import * as status from "./ops/status.mjs";
import * as report from "./ops/report.mjs";
import * as run from "./ops/run.mjs";
import * as cv from "./ops/cv.mjs";
import * as followups from "./ops/followups.mjs";
import * as config from "./ops/config.mjs";
import * as inbox from "./ops/inbox.mjs";

export const OPS = {
  [bootstrap.op]: bootstrap.handle,
  [pipeline.op]: pipeline.handle,
  [status.op]: status.handle,
  [report.op]: report.handle,
  [run.op]: run.handle,
  [cv.op]: cv.handle,
  [followups.op]: followups.handle,
  [config.op]: config.handle,
  [inbox.op]: inbox.handle,
};
