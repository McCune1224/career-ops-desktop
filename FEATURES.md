# career-ops: feature inventory

Every capability this repository has, one row per issue candidate.

This is the **source document** for a GitHub backlog and a kanban board. It says
what exists, where it lives, and whether the native desktop client has it yet.
It does not propose new product features. Anything not here does not exist in the
repository today.

- **Scope:** the whole system. Core scripts, modes, the web UI, the native
  desktop client, providers, plugins.
- **Audience:** whoever is cutting issues. Every row is meant to become one
  issue, or be merged into a sibling when it is too small to stand alone.
- **Companion:** [`ROADMAP.md`](ROADMAP.md) for sequencing, kanban columns,
  definition of done, and issue templates.

Counts at the time of writing, all from the tree rather than from memory: 145
root scripts, 32 `lib/` modules, 47 mode files at the top of `modes/` plus 4
under `modes/interview/`, 122 providers, 49 web API routes, 15 web pages, 9
desktop bridge operations, 547 test suites.

The inventory below is **153 rows** across 18 epics.

---

## How to read a row

| Column | Meaning |
|---|---|
| **ID** | Stable reference. `E04-S03` is epic 4, story 3. Never renumber; retire by marking `DROPPED`. |
| **Capability** | What a person can do. Phrased as the outcome, not the script name. |
| **Evidence** | Where it lives today. A file path is the proof it exists; if the path moves, the row is wrong. |
| **Desktop** | Status in this client. `done`, `partial`, `missing`, `gap` (deliberately out of scope, named in `README.md`), `n/a` (no native equivalent is wanted). |
| **Acceptance** | How you know it is finished. Must be checkable by a test or by doing the thing once. |
| **Size** | `S` half a day, `M` one to three days, `L` more than three. |
| **Deps** | Other IDs that must land first. |

**Desktop status is the only column that changes as the app is built.** Update it
in the same commit that changes the code, or the board lies.

**On the Evidence column.** A path is relative to the **career-ops checkout**
unless it names a file this repository owns (`src/`, `bridge/`, `dev/`,
`tests/screens.rs`, `tests/desktop-*.test.mjs`), in which case it is relative to
this repository's root. A path in `web/`, `modes/`, `lib/`, `scripts/` or
`templates/` exists in the career-ops tree today; if it does not, the row is
wrong and should be fixed before the issue is worked. Five paths are user-layer
files that are created on first use and may not exist yet in a given checkout:
`config/plugins.yml`, `data/contacts.tsv`, `data/Connections.csv`,
`data/salary-observations.tsv`, `data/career-profile.yml`. Their absence is the
documented state, not a broken citation.

---

## Cross-cutting invariants

Every issue, in every epic, is bound by these. An issue that violates one is
wrong no matter how well it works.

| ID | Invariant | Where it is enforced |
|---|---|---|
| X-1 | Every tracker write goes through `set-status.mjs`, `merge-tracker.mjs`, `reserve-report-num.mjs`, `persist-evaluation-report.mjs`, or `tracker.mjs delete`. Never a parallel writer. | `DECISIONS.md` D-4; `tests/desktop-bridge-ops.test.mjs` |
| X-2 | One implementation per rule. A new client calls the shared engine rather than re-deriving it. | `tests/desktop-engine-parity.test.mjs` |
| X-3 | A missing file is not a malformed file. `ENOENT` and a parse error are different outcomes. | `web/AGENTS.md`; `web/src/lib/portals-config.mjs` |
| X-4 | Untrusted content is data, never instructions. Postings, forms and emails can inform a score, never change a rule. | `AGENTS.md`; `validate-untrusted-content-coverage.mjs` |
| X-5 | Nothing submits an application. No code path fills and sends a form. | `web/AGENTS.md` rule 3 |
| X-6 | The native client embeds no webview. | `tests/no_webview.rs` |
| X-7 | A generated claim traces to a primary source file, or it is not made. | `AGENTS.md` source-of-truth boundary; `story-provenance-check.mjs` |
| X-8 | The updater's coverage guard stays green. Every tracked file is in `SYSTEM_PATHS`, `USER_PATHS`, or a declared exclusion. | `validate-system-paths-coverage.mjs` |

---

## E00. Foundations, install, and the profile

The things that must be true before any other epic is usable.

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E00-S01 | Check whether this install is ready and say exactly what is missing | `doctor.mjs`, `GET /api/doctor` | done | `the bridge answers every read op in the shape the client decodes` | S |  |
| E00-S02 | Read and validate `config/profile.yml` against the schema | `validate-profile.mjs` | done | `the profile validator surfaces a broken profile as an error` | S |  |
| E00-S03 | Edit the candidate details in the profile, merge-safely | `POST /api/profile`, `web/src/lib/profile-patch.mjs` | done | `saving_the_profile_from_config_merges_rather_than_replaces` | S | E00-S02 |
| E00-S04 | Configure the output language and market mode set | `profile-language.mjs`, `modes/<market>/`, `web/src/lib/profile-keywords.mjs` | done | `the profile's language and its market mode set are read` | M | E03-S01 |
| E00-S05 | Edit the personalization file (`modes/_profile.md`) without destroying its structure | `POST /api/personalization`, `web/src/lib/personalization-file.mjs` | done | `the_config_screen_edits_one_personalization_section`, `the personalization sections come back with their bodies, so one can be edited` | M |  |
| E00-S06 | Seed a first-run profile from the shipped example | `config/profile.example.yml` | done | `a first save on a fresh install seeds the profile and never overwrites one` | S |  |
| E00-S07 | Update the system layer without touching user data | `update-system.mjs`, `modes/update.md` | gap | Not wanted natively; the CLI owns it | S | |
| E00-S08 | Detect the installed AI CLIs and pick a usable one | `web/src/lib/clis.ts`, `web/src/lib/cli-pick.mjs` | done | `loading_the_app_selects_a_usable_cli_without_a_visit_to_config` | S | |

## E01. Tracker and pipeline

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E01-S01 | Read the tracker, header-alias aware | `tracker-parse.mjs`, `tracker-aliases.json`, `web/src/lib/tracker-table.mjs` | done | `pipeline_screen_renders_the_tracker_rows` | S |  |
| E01-S02 | List, sort and filter the tracker | `web/src/lib/core/pipeline-sort.mjs`, `web/src/components/pipeline-view.tsx` | done | `pipeline_filters_by_status_and_score`, `clicking_a_column_header_reorders_the_rows` | M | E01-S01 |
| E01-S03 | Change a row's status to a canonical state | `set-status.mjs`, `POST /api/status` | done | `a status change lands through set-status.mjs and is attributed to desktop` | M | E01-S01 |
| E01-S04 | Reject a status the canonical set does not contain, writing nothing | `templates/states.yml`, `set-status.mjs` | done | `a rejected state writes nothing` | S | E01-S03 |
| E01-S05 | Bulk-change several rows at once | `web/src/components/pipeline-view.tsx` (selection + bulk apply) | done | `a bulk status change writes one ledger row per row` | M | E01-S03 |
| E01-S06 | Show the pending-posting inbox | `data/pipeline.md`, `web/src/lib/pipeline-table.mjs` | done | `today_screen_renders_the_daily_loop_tiles` | S |  |
| E01-S07 | Skip or un-skip an inbox posting | `web/src/lib/inbox-skip.mjs`, `POST /api/inbox/skip` | done | `the_pipeline_screen_offers_a_skip_control_for_each_pending_posting` | S | E01-S06 |
| E01-S08 | Shortlist inbox rows and score them in a batch | `web/src/components/inbox/inbox-triage.tsx`, `batch/` | done | `a batch scores one posting at a time, and each one is its own run` | L | E03-S01 |
| E01-S09 | Delete a tracker row, with the orphaned-report consequence shown | `tracker.mjs delete`, `POST /api/tracker/delete` | done | `removing a tracker row goes through tracker.mjs and names the orphaned report` | S | E01-S01 |
| E01-S10 | Open a posting URL from the tracker | `web/src/lib/posting-url.mjs` | done | `pipeline_screen_offers_report_and_posting_actions` | S |  |
| E01-S11 | Add an application by hand | `modes/add.md`, `add-entry.mjs` | done | `a hand-added application lands in the tracker with an N/A score` | M | E01-S01 |
| E01-S12 | Find a tracker row or report by free text | `find.mjs` | done | `a search finds a tracker row by name` | M | E01-S01 |

## E02. Evaluation and the report

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E02-S01 | Evaluate a posting into an A-F block report with a score and a verdict | `modes/oferta.md`, `modes/_shared.md`, `POST /api/run` kind `evaluate` | done | `an evaluation started by the desktop client is persisted through the canonical scripts` | L | E03-S01 |
| E02-S02 | Persist the report and merge the tracker row, atomically | `persist-evaluation-report.mjs`, `merge-tracker.mjs`, `reserve-report-num.mjs` | done | `an evaluation started by the desktop client is persisted through the canonical scripts` | M | E02-S01 |
| E02-S03 | Read a report in the app, rendered | `web/src/components/report-view.tsx`, `web/src/lib/report-sections.mjs` | done | `report_screen_shows_the_report_and_the_source_posting` | M | E02-S02 |
| E02-S04 | Show the verdict lead, score, and legitimacy tier | `web/src/lib/report-sections.mjs`, `web/src/lib/format.ts` | done | `report_screen_shows_the_report_and_the_source_posting` | S | E02-S03 |
| E02-S05 | Progressive disclosure: Block B open, the rest folded, machine artifacts separate | `web/src/components/report-view.tsx` | done | `the_report_screen_opens_the_lead_and_folds_the_rest` | S | E02-S03 |
| E02-S06 | Keyword coverage against the CV for one report | `keyword-match.mjs`, `GET /api/keyword-match` | done | `the keyword coverage is reported without touching the CV` | M | E02-S03 |
| E02-S07 | Score levers and score methodology | `web/src/components/score-levers.tsx`, `web/src/components/score-methodology.tsx` | done | `the score levers are read from the report's own machine summary` | M | E02-S03 |
| E02-S08 | Company evidence card for a report | `company-history.mjs`, `GET /api/company-history` | done | `a company history joins the tracker, the follow-ups and the status log` | M | E02-S03 |
| E02-S09 | Compare two or three applications side by side | `web/src/app/pipeline/compare/page.tsx` | done | `two applications can be compared side by side` | M | E02-S03 |
| E02-S10 | Keep the JD archived with the report | `check-jd-archive.mjs`, `jds/`, `jd-capture.mjs` | done | `a report keeps the JD it archived` | M | E02-S02 |
| E02-S11 | Attach a posting date rather than letting the agent guess it | `POST /api/run`, `readScanDates()` | done | `a report for a scanned URL carries the recorded posting date` | S | E02-S01 |

## E03. Worker runs and CLI integration

The engine that every AI-backed feature stands on. The desktop client already
uses this path unchanged, so most rows are about surfaces rather than plumbing.

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E03-S01 | Spawn a chosen AI CLI headlessly, with argv fenced to the worker's capabilities | `web/src/lib/spawn-cli.mjs`, `web/src/lib/cli-fencing.mjs`, `web/src/lib/claude-invocation.mjs`, `web/src/lib/worker-capabilities.mjs` | done | `the bridge's run path imports the same engine modules the web route does`, `the bridge builds the same argv as the web app for every CLI and kind` | L |  |
| E03-S02 | Stream progress as text, tool, status, error and done | `web/src/lib/job-stream.mjs`, `POST /api/run` | done | `run_screen_renders_streamed_progress` | M | E03-S01 |
| E03-S03 | Announce when a CLI cannot be permission-restricted | `web/src/lib/cli-fencing.mjs::fencingReport` | done | `a CLI that cannot be fenced says so, and is refused a write-capable worker` | S | E03-S01 |
| E03-S04 | Say when a CLI has gone quiet, rather than looking hung | `app.rs::run_clock`, `src/views/run.rs` | done | `a_keepalive_does_not_count_as_the_agent_speaking`, `a_silent_run_says_so_instead_of_only_saying_running` | S | E03-S02 |
| E03-S05 | Say when the chosen CLI was substituted for one that is not installed | `cli-pick.mjs::pickUsableCli`, `cliSubstitutionNotice()` | done | `a run on an absent CLI says it substituted one` | S | E00-S08 |
| E03-S06 | Refuse a worker whose capability the CLI cannot be granted | `web/src/lib/cli-fencing.mjs::isCliAllowedForCapabilities` | done | `a CLI that cannot be fenced says so, and is refused a write-capable worker` | S | E03-S01 |
| E03-S07 | Estimate a run's cost from recent runs before spending it | `web/src/lib/run-cost-estimate.mjs`, `web/src/components/cost/cost-badge.tsx` | done | `the cost estimate comes from this client's own runs` | M | E03-S02 |
| E03-S08 | Confirm a batch spend before a fan-out | `web/src/lib/run-cost-estimate.mjs::requiresBatchSpendConfirmation` | done | `the cost estimate comes from this client's own runs` | M | E03-S07 |
| E03-S09 | Record token and cost per run, and aggregate usage | `web/src/lib/claude-usage.mjs`, `GET /api/usage`, `utils/token-tracker.mjs` | done | `the_run_screen_shows_what_has_been_spent` | M | E03-S02 |
| E03-S10 | Save a run log the CLI assistant can read later | `POST /api/runs/save`, `web/src/components/jobs/job-store.tsx` | done | `a finished run leaves a readable record the assistant can read` | M | E03-S02 |
| E03-S11 | Repair a broken portal from the run screen | `POST /api/run` kind `fix-portal`, `modes/scan.md` | done | `a broken portal is repaired only for a company name that is safe to pass` | M | E03-S01 |

## E04. CV

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E04-S01 | Read and edit `cv.md` with a live preview | `GET/POST /api/cv`, `web/src/components/cv-editor.tsx` | done | `cv_screen_shows_the_cv_and_its_editor`, `the CV write is atomic and leaves a backup` | M |  |
| E04-S02 | Tailor the CV to one posting and render a PDF | `modes/pdf.md`, `build-cv-html.mjs`, `generate-pdf.mjs`, `web/src/lib/pdf-render.mjs` | done | `the pdf run leaves the tracker PDF column alone when the render does not happen` | L | E03-S01 |
| E04-S03 | Choose a CV template, including a named variant | `cv-templates.mjs`, `templates/`, `web/src/lib/core/cv-template.mjs` | done | `the CV template setting changes the template the run fills` | M | E04-S02 |
| E04-S04 | Import a CV from pasted text or a document, as a proposal | `POST /api/cv/ingest`, `modes/intake.md`, `intake.mjs` | done | `the intake lists the documents it found, and proposes nothing by itself` | L | E04-S01 |
| E04-S05 | Verify a tailored CV's claims against the source files | `verify-cv-facts.mjs` | done | `the CV's claims are checked against the source files` | L | E04-S02 |
| E04-S06 | Verify the tailored CV kept its structure and order | `verify-cv-structure.mjs`, `cv-experience-order.mjs`, `cv-title-check.mjs` | done | `a tailored CV whose roles are out of order is reported before it is rendered`, `the CV's structure is checked against the file the user wrote` | M | E04-S02 |
| E04-S07 | Produce an ATS-parseable copy alongside the styled PDF | `ats-payload.mjs`, `verify-ats.mjs`, `modes/ats.md` | gap | Named as out of scope for the daily loop; the CLI path remains | M | E04-S02 |
| E04-S08 | Produce a LaTeX CV from a `.tex` template | `build-cv-latex.mjs`, `generate-latex.mjs`, `modes/latex.md` | gap | Out of scope natively; the CLI owns it | L | E04-S02 |
| E04-S09 | Tailor a hand-maintained `.tex` CV in place | `modes/latex-tex.md`, `patch-latex-content.mjs` | gap | Out of scope natively | M | E04-S08 |
| E04-S10 | Edit a tailored CV block by block before rendering | `POST /api/cv-studio/[n]`, `web/src/lib/cv-studio/` | gap | Named as a gap; studio is deferred | L | E04-S02 |
| E04-S11 | Surface weighted skill gaps from the pipeline | `upskill.mjs`, `GET /api/upskill`, `web/src/components/skill-gaps.tsx` | done | `the skill gaps exclude the skills already on the CV` | M | E01-S01 |
| E04-S12 | Discover competencies missing from the CV | `modes/expand.md`, `skill-extract.mjs` | done | `the competency extractor only proposes what the core's vocabulary knows` | M | E04-S01 |
| E04-S13 | Keep a source-backed master career profile | `career-profile.mjs`, `modes/master-profile.md`, `data/career-profile.yml` | done | `the master career profile is validated by the core` | L | E04-S01 |

## E05. Cover letters and outreach

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E05-S01 | Draft a cover letter for one application, with a brief | `modes/cover.md`, `generate-cover-letter.mjs`, `POST /api/run` kind `cover` | done | `the cover run produces a draft letter in the run log`, `report_screen_shows_the_report_and_the_source_posting` | L | E03-S01 |
| E05-S02 | Choose the letter's tone from a fixed set | `web/src/lib/cover-tones.mjs`, `web/src/lib/cover-brief.mjs` | done | `the_run_screen_offers_the_cover_tones` | S | E05-S01 |
| E05-S03 | Draft a formal application email, and never send it | `modes/email.md` | done | `a mode run asks the agent to follow the mode file` | M | E02-S03 |
| E05-S04 | Find the right contact and draft outreach in the channel's limit | `modes/contacto.md`, `contact-extract.mjs`, `contact-lookup.mjs` | done | `the_run_screen_offers_the_outreach_mode` | L | E02-S03 |
| E05-S05 | Keep a job-search contact list and export it | `contacts.mjs`, `data/contacts.tsv` | done | `the phonebook is read, exported and joined against a company` | M | E05-S04 |
| E05-S06 | Find a warm introduction through a LinkedIn export | `linkedin-join.mjs`, `data/Connections.csv` | done | `the phonebook is read, exported and joined against a company` | M | E05-S05 |
| E05-S07 | Match a pasted interview invite to a tracker row | `invite-match.mjs` | done | `a pasted invite is matched to a tracker row` | S | E01-S01 |

## E06. Follow-ups, outcomes, negotiation

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E06-S01 | Show what is due, from the core's cadence calculator | `followup-cadence.mjs`, `GET /api/followups` | done | `followups_screen_lists_due_followups_and_the_log` | M |  |
| E06-S02 | Log a follow-up, and remove a mis-logged one | `POST/DELETE /api/followups/log` | done | `the_follow_ups_screen_offers_a_remove_and_a_pin` | S | E06-S01 |
| E06-S03 | Pin a next follow-up date, and clear the pin | `POST/DELETE /api/followups/override` | done | `the_follow_ups_screen_offers_a_remove_and_a_pin` | S | E06-S01 |
| E06-S04 | Tune the cadence knobs, merge-safely | `POST /api/followups/cadence` | done | `the cadence read and write go through config/profile.yml` | S |  |
| E06-S05 | Turn follow-ups off entirely | `web/src/lib/followups-setting.mjs` | done | `the_follow_ups_entry_is_hidden_when_follow_ups_are_off` | S | E06-S04 |
| E06-S06 | Seed the first follow-up date when a row turns Applied | `followup-seed.mjs` | done | `a row that turned Applied pins its first follow-up` | S | E01-S03 |
| E06-S07 | Flag companies still silent after an interview, as a suggestion | `rejection-latency.mjs`, `GET /api/quiet-companies` | done | `the quiet companies are listed with the reason each one is quiet` | M | E01-S01 |
| E06-S08 | Measure per-company recruiting friction | `process-quality.mjs` | done | `the recruiting friction comes from the interview notes` | M | E06-S07 |
| E06-S09 | Record an outcome and archive the artifacts | `outcome.mjs`, `modes/outcome.md` | done | `the_outcomes_screen_proposes_an_outcome_before_recording_it`, `an outcome is recorded through the core and archives the posting` | M | E01-S01 |
| E06-S10 | Walk an offer's clauses with a lawyer question list | `modes/offer-prep.md` | done | `a mode run asks the agent to follow the mode file` | M | E02-S03 |
| E06-S11 | Draft negotiation points anchored in a verified achievement | `negotiation-roi.mjs` | done | `a negotiation figure survives only if the CV says it too` | M | E04-S05 |
| E06-S12 | Analyse the salary gap between wanted, advertised and actual | `salary-gap.mjs`, `data/salary-observations.tsv` | done | `the salary gap is wanted against advertised against actual` | M | E01-S01 |

## E07. Discovery: scanning, exploration, portals

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E07-S01 | Scan configured portals with zero model tokens | `scan.mjs`, `providers/` (122), `portals.yml` | gap | Named as out of scope; the CLI owns it | L | |
| E07-S02 | Reverse-scan public ATS datasets with filters | `scan-ats-full.mjs` | gap | Out of scope natively | L | E07-S01 |
| E07-S03 | Scan German public-sector postings | `scan-interamt.mjs` | gap | Out of scope natively | M | E07-S01 |
| E07-S04 | Scan Dayforce career sites | `scan-dayforce.mjs` | gap | Out of scope natively | M | E07-S01 |
| E07-S05 | Run an AI web search for postings, with cost shown | `modes/discover.md`, `web/src/lib/explore-ai.ts`, `POST /api/explore/ai` | gap | Out of scope natively | L | E03-S01 |
| E07-S06 | Add discovered postings to the pipeline | `POST /api/explore/add`, `web/src/lib/core/pipeline.ts` | gap | Out of scope natively | M | E07-S05 |
| E07-S07 | Verify a watched board answers, and audit what it contains | `verify-portals.mjs`, `audit-portals.mjs`, `POST /api/portals/verify` | done | `the watched boards are audited and the verdict is per board` | M |  |
| E07-S08 | Edit the watched companies and query filters | `portals.yml`, `POST /api/portals`, `web/src/lib/portals-config.mjs` | done | `the watched boards can be edited and a file that cannot be parsed is refused` | M | E07-S07 |
| E07-S09 | Explain why a posting you found by hand was missed | `coverage-check.mjs` | done | `a posting you found by hand is explained, not guessed at` | M | E07-S08 |
| E07-S10 | Discover a company's ATS board from its careers URL | `discover-ats.mjs`, `modes/discover.md`, `POST /api/explore/ai/known` | done | `a company name becomes a verified board, never a guessed URL` | L | E07-S08 |
| E07-S11 | Flag roles re-listed repeatedly | `detect-reposts.mjs` | done | `a role that keeps being re-listed is flagged, and one posting is not enough` | M | E07-S01 |
| E07-S12 | Check whether a posting is still live | `check-liveness.mjs`, `liveness-core.mjs`, `web/src/lib/core/scan.ts` | done | `a liveness verdict is never 'active' for a posting that is not there` | M | E01-S01 |
| E07-S13 | Fetch a JD without a browser where an ATS API exists | `fetch-jd.mjs`, `liveness-api.mjs` | done | `the JD is fetched without a browser where an ATS API exists` | M | E07-S12 |
| E07-S14 | Show what is new this week | `GET /api/whats-new` | done | `what is new this week comes from the scan history and excludes the dead` | S | E07-S12 |

## E08. Apply: form prefill

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E08-S01 | Open a real application form and extract its fields | `POST /api/apply/session`, `web/src/lib/apply/extract.ts` | gap | Named as out of scope. Requires a browser, which this client will not embed | L | |
| E08-S02 | Draft an answer per field from the profile and CV | `POST /api/apply/prefill`, `web/src/lib/apply/fill-stream.mjs` | gap | Out of scope natively | L | E08-S01 |
| E08-S03 | Fill the form and hand the window back, never submitting | `POST /api/apply/fill`, `web/src/lib/apply/submit-guard.mjs` | gap | Out of scope, and the submit guard is the reason it must stay out | L | E08-S02 |
| E08-S04 | Drive a form agentically toward "reach" or "full" | `POST /api/apply/drive`, `web/src/lib/apply/planner.ts` | gap | Out of scope natively | L | E08-S03 |
| E08-S05 | Answer a screening questionnaire from a prepared pack | `application-answers.mjs`, `prepare-application.mjs` | done | `a screening questionnaire is read from the report and never sent` | M | E02-S03 |

## E09. Assistant and agent chat

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E09-S01 | Chat with the assistant, streamed | `POST /api/assistant`, `web/src/components/assistant-console.tsx` | gap | Named as out of scope | L | E03-S01 |
| E09-S02 | Persist, rename, delete and export chats | `POST /api/assistant/chats`, `web/src/lib/assistant-chat-store.mjs` | gap | Out of scope natively | M | E09-S01 |
| E09-S03 | Confirm a write the assistant proposes before it runs | `web/src/app/actions/registry.ts`, `web/src/lib/act-envelope.mjs` | gap | Out of scope natively, and the confirmation gate must not be reimplemented loosely | L | E09-S01 |
| E09-S04 | Let the assistant remember a durable fact in the profile | `POST /api/memory`, `web/src/lib/profile-memory.mjs` | gap | Out of scope natively | M | E09-S03 |

## E10. Interview preparation

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E10-S01 | Build a prep pack for one application | `modes/interview-prep.md`, `web/src/components/prep-pack.tsx` | done | `a mode run asks the agent to follow the mode file` | M | E02-S03 |
| E10-S02 | Plan time-blocked preparation | `modes/interview/plan.md` | done | `a mode run asks the agent to follow the mode file` | M | E10-S01 |
| E10-S03 | Practise questions with feedback | `modes/interview/practice.md` | done | `a mode run asks the agent to follow the mode file` | L | E10-S01 |
| E10-S04 | Debrief a real interview and close the gaps | `modes/interview/debrief.md`, `interview-prep/sessions/` | done | `a mode run asks the agent to follow the mode file` | M | E10-S03 |
| E10-S05 | Judge whether a company is safe to join | `modes/interview-redflag.md` | done | `a mode run asks the agent to follow the mode file` | M | E02-S03 |
| E10-S06 | Keep a STAR story bank with provenance | `interview-prep/story-bank.md`, `lib/story-bank.mjs`, `story-provenance-check.mjs`, `templates/story-bank.template.md` | done | `the story bank says which of its numbers may be restated as fact` | M | E10-S01 |
| E10-S07 | Match a story to a question | `match-star.mjs` | done | `a question is matched to a story from the bank` | M | E10-S06 |
| E10-S08 | Roll a week of sessions into a digest | `weekly-digest.mjs` | done | `a week of sessions is rolled into one summary per company` | S | E10-S04 |

## E11. Analytics and calibration

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E11-S01 | Lifetime pipeline statistics and funnel rates | `stats.mjs`, `GET /api/stats` | gap | Named as out of scope | M | |
| E11-S02 | Stage velocity and funnel calibration against benchmarks | `funnel-velocity.mjs`, `funnel-stages.mjs`, `data/status-log.tsv` | gap | Out of scope natively | M | E11-S01 |
| E11-S03 | Pattern analysis, including advance rate per ATS vendor | `analyze-patterns.mjs`, `GET /api/patterns` | gap | Out of scope natively | L | E11-S01 |
| E11-S04 | Whether the scores predicted real outcomes | `calibrate.mjs`, `modes/calibrate.md` | gap | Advisory only; it never changes scoring | M | E11-S01 |
| E11-S05 | Discard analytics | `discard-analytics.mjs` | gap | Out of scope natively | S | E11-S03 |
| E11-S06 | A visual funnel from the status ledger | `web/src/lib/pipeline-sankey.mjs`, `web/src/components/analytics/pipeline-sankey.tsx` | gap | Out of scope natively | M | E11-S02 |

## E12. Data integrity and maintenance

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E12-S01 | Validate the pipeline's health end to end | `verify-pipeline.mjs` | done | `the health check reports a broken report link` | M | E01-S01 |
| E12-S02 | Normalise statuses to the canonical set | `normalize-statuses.mjs` | done | `normalizing rewrites a non-canonical status and is idempotent` | S | E01-S03 |
| E12-S03 | Remove duplicate tracker rows | `dedup-tracker.mjs` | done | `duplicate tracker rows are merged by the core, with a backup` | M | E12-S01 |
| E12-S04 | Repair tracker rows whose report link is dead | `fix-report-links.mjs` | done | `a dead report link is repaired and the row is named` | S | E12-S01 |
| E12-S05 | Reconcile the tracker against the reports on disk | `reconcile-pipeline.mjs` | done | `the pipeline is reconciled against the batch state` | M | E12-S01 |
| E12-S06 | Keep the PDF column honest | `sync-pdf-flags.mjs`, `mark-pdf-ready.mjs`, `data/pdf-index.tsv` | done | `the PDF column is synced with the files that exist` | M | E04-S02 |
| E12-S07 | Validate that every report archived its JD | `check-jd-archive.mjs` | done | `the JDs every report kept are checked by the core's own tool` | S | E02-S10 |
| E12-S08 | Validate the jurisdiction tables are not stale | `check-table-freshness.mjs`, `templates/*.yml` | n/a | CLI-only; nothing to show natively | S | |
| E12-S09 | Repair malformed portal slugs | `fix-slugs.mjs` | done | `every core maintenance tool is reachable and reports what it did` | S | E07-S08 |
| E12-S10 | Detect i18n drift across the market mode sets | `i18n-drift.mjs` | n/a | CLI-only | S | |

## E13. Scheduling and automation

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E13-S01 | Schedule recurring scans at the OS level | `/api/scheduler`, `/api/scheduled-jobs*`, `web/scripts/` | gap | Named as out of scope; it installs system schedulers | L | E07-S01 |
| E13-S02 | Create, edit, pause and run a scheduled job now | `web/src/components/scheduled-scans/` | gap | Out of scope natively | L | E13-S01 |
| E13-S03 | Review a scheduled job's run history | `web/src/components/scheduled-scans/run-history-drawer.tsx` | gap | Out of scope natively | M | E13-S02 |
| E13-S04 | Sweep due follow-ups on a schedule | `scripts/followup-sweep.sh`, `followup-cadence.mjs` | done | `today_screen_renders_the_daily_loop_tiles` | S | E06-S01 |
| E13-S05 | Queue a request for a later session | `agent-inbox.mjs`, `modes/agent-inbox.md` | done | `the agent inbox is a checklist that is drained, not run` | S |  |
| E13-S06 | Track session activity | `session-activity.mjs` | n/a | CLI-only | S | |

## E14. Documents and artifacts

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E14-S01 | List every generated CV and cover letter | `GET /api/documents`, `/documents` | done | `the_documents_screen_lists_what_was_generated` | M | E04-S02 |
| E14-S02 | Open a generated document | `GET /api/documents?id=` | done | `the documents can be opened from the list` | S | E14-S01 |
| E14-S03 | Archive a posting as a capture | `archive-posting.mjs`, `jds/`, `jd-capture.mjs` | done | `the_documents_screen_lists_what_was_generated` | M | E02-S10 |
| E14-S04 | Convert images to PDF | `img-to-pdf.mjs` | n/a | CLI-only | S | |
| E14-S05 | Share a hire, once, with explicit consent | `hired-share.mjs`, `hired-wall-build.mjs`, `HIRED.md` | done | `a hire may be shared, and nothing leaves the machine` | M | E06-S09 |

## E15. Plugins and integrations

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E15-S01 | List, enable, disable and audit plugins | `plugins.mjs`, `plugin-audit.mjs`, `config/plugins.yml` | done | `the plugin list is read and a plugin can be disabled` | M |  |
| E15-S02 | Install the shipped plugin set | `plugins-registry/` (11), `plugin-install.mjs` | done | `the_config_screen_confirms_a_plugin_install`, `a plugin install is confirmed, and a preview is not an install` | M | E15-S01 |
| E15-S03 | Route interview invites to a calendar | `plugins-registry/outlook-interviews.json`, `plugins-registry/google-calendar.json` | gap | Out of scope natively | M | E15-S02 |
| E15-S04 | Capture alerts from a watchlist | `plugins-registry/linkedin-alerts.json` | gap | Out of scope natively | M | E15-S02 |

## E16. Native client shell

Capabilities that exist only because the app is native. These are not parity
rows; the web UI has no equivalent.

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E16-S01 | Run as one process tree with no listening socket | `bridge/server.mjs` | done | `the bridge never binds a socket` | S |  |
| E16-S02 | Survive a bridge crash without wedging | `src/bridge.rs` | done | `a_bridge_that_dies_fails_the_pending_calls_with_bridge_gone` | S |  |
| E16-S03 | Read the real data root, read-only, and prove it changed nothing | `tests/live_data.rs` | done | `the_live_data_root_decodes_and_is_left_untouched` | S |  |
| E16-S04 | Zoom and theme, persisted | `src/theme.rs`, `src/settings.rs` | done | `the_zoom_and_the_theme_are_saved` | S |  |
| E16-S05 | Open a PDF or a URL in the user's own viewer | `views/report.rs::open_file`, `views/pipeline.rs::open_url` | done | `pipeline_screen_offers_report_and_posting_actions` | S |  |
| E16-S06 | Say which data root and checkout are in use | `views/config.rs::install_section` | done | `config_screen_reports_where_the_data_comes_from` | S |  |
| E16-S07 | Keyboard navigation across screens | none | done | `ctrl_number_keys_reach_every_screen` | S |  |
| E16-S08 | Remember the last screen across launches | none | done | `the_app_reopens_on_the_screen_it_was_left_on` | S | E16-S04 |
| E16-S09 | Recover from a bridge that is missing or too old | `bridge.rs::discover` | done | `a_missing_engine_checkout_is_one_actionable_sentence` | S |  |
| E16-S10 | Report a version mismatch between app and engine | none | done | `a_run_is_refused_when_the_app_and_the_bridge_disagree` | S | E16-S09 |
| E16-S11 | Preview a PDF page in the window, and page through it | none (the web app uses an `<iframe>`) | done | `a_page_is_rasterised_into_pixels`, `the_report_screen_offers_a_pdf_preview_and_the_viewer_opens_it`, `the_documents_screen_offers_a_preview_of_what_it_lists`, `a_preview_that_cannot_be_made_says_why` | M | E16-S01 |

## E17. Deferred: GUI rework

Deferred deliberately. The app works and looks coherent; a redesign should follow
function, not precede it. Recorded here so it is not lost, and not started.

| ID | Capability | Evidence | Desktop | Acceptance | Size | Deps |
|---|---|---|---|---|---|---|
| E17-S01 | A documented design system, separate from the token module | `src/theme.rs` | partial | Tokens exist; a written rationale per token would let a redesign move fast | S | |
| E17-S02 | Resizable and collapsible sidebar, remembered | none | missing | The shell survives a 900 px window without clipping | M | E16-S07 |
| E17-S03 | A command palette over screens, rows and reports | none | missing | `Ctrl K` reaches any screen and any report by number or company | M | E16-S07 |
| E17-S04 | Table column resize and persist | none | missing | Column widths survive a restart | S | E17-S02 |
| E17-S05 | Empty, loading and error states drawn as first-class surfaces | partial | partial | Every screen has all three, and none of them is a bare spinner | M | |
| E17-S06 | Density switch for the tracker | none | missing | Comfortable and compact row heights | S | E17-S02 |

---

## What "functional app" means

A release gate, not a wish list. The app is functional for daily use when every
row marked `done` still passes, plus these four:

1. All of E00, E01, E02, E03, E04 (view and tailor), E05-S01, E06, E16 — met.
2. No row is `partial` in an epic that is otherwise complete — met; the two
   `partial` rows left are E17's.
3. Every in-scope row has an acceptance check that runs in `cargo test` or
   `node --test` — met, and `tests/acceptance.test.mjs` holds it.
4. The user has walked `CAPABILITY-MATRIX.md` against real data — **outstanding,
   and theirs to give.**

Everything else is post-functional.

## Counting

| Desktop status | Rows |
|---|---|
| done | 113 |
| partial | 2 |
| missing | 4 |
| gap | 30 |
| n/a | 4 |
| **total** | **153** |

Per epic: E00 8, E01 12, E02 11, E03 11, E04 13, E05 7, E06 12, E07 14, E08 5,
E09 4, E10 8, E11 6, E12 10, E13 6, E14 5, E15 4, E16 10, E17 6.

Every row outside E17 is `done`. The two `partial` and four `missing` rows left
are the deferred epic's, which is deliberately not started.

The `gap` and `n/a` rows are not work. They are the boundary, recorded so that a
later "why is this missing" question has an answer. `tests/acceptance.test.mjs`
is the gate: it fails if any row outside E17 is `missing` or `partial`, or if a
row names a check this repository does not have.
