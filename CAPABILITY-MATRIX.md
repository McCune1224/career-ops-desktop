# Capability matrix — native desktop client vs. the web UI

Derived from the career-ops checkout's actual routes and components, and kept
current as the client grew. The rule applied throughout: a capability is **in
scope** when a person needs it to run their search, and **out** when it needs a
browser, an OS scheduler or a credential store this client refuses to embed.

Every row names the web surface it replaces, the desktop surface that
implements it, and the **executable** check that fails if the capability
breaks. `cargo test` and `node --test` are the only instruments.

## 1. Pipeline / tracker

| # | Capability | Web source | Desktop surface | Proof |
|---|---|---|---|---|
| 1.1 | List every tracker row with company, role, score, status, date | `/pipeline` → `pipeline-view.tsx` | `views/pipeline.rs` | `pipeline_screen_renders_the_tracker_rows` |
| 1.2 | Sort by #, company, role, score, status or date | `pipeline-view.tsx` sort keys | `views/mod.rs::sort_rows` | `pipeline_filters_by_status_and_score`, `clicking_a_column_header_reorders_the_rows` |
| 1.3 | Filter by canonical status, score floor and free text | 12 status tabs, `?min=`, search box | `views/pipeline.rs::filter_bar` | `pipeline_screen_exposes_every_canonical_state_filter`, `typing_in_the_search_box_filters_as_you_type` |
| 1.4 | Change a row's status inline, through the one writer | `POST /api/status` → `set-status.mjs` | `bridge/ops/status.mjs` | `a status change lands through set-status.mjs and is attributed to desktop`, `a_status_change_goes_through_the_status_op` |
| 1.5 | Refuse a state the canonical set does not contain | `status/route.ts` 400 path | `bridge/ops/status.mjs` | `a rejected state writes nothing` |
| 1.6 | Change several rows at once, one ledger row each | `pipeline-view.tsx` bulk apply | `views/pipeline.rs::bulk_bar` → `status` op `rows` | `a bulk status change writes one ledger row per row` |
| 1.7 | Skip or un-skip a pending posting | `POST /api/inbox/skip` → `inbox-skip.mjs` | `bridge/ops/inbox.mjs`, `views/pipeline.rs::inbox_card` | `skipping a pending posting flips the checkbox and nothing else`, `the_pipeline_screen_offers_a_skip_control_for_each_pending_posting` |
| 1.8 | Shortlist a posting, hide it, and score the shortlist after a spend confirmation | shortlist tray + "Score shortlist" | `views/pipeline.rs`, `app.shortlist` (saved in settings) | `a batch scores one posting at a time, and each one is its own run`, `the cost estimate comes from this client's own runs` |
| 1.9 | Remove a tracker row, naming the orphaned report | `POST /api/tracker/delete` → `tracker.mjs` | `bridge/ops/tracker.mjs` | `removing a tracker row goes through tracker.mjs and names the orphaned report` |
| 1.10 | Add an application by hand, with the `N/A` sentinel | `modes/add.md`, TSV + merge | `bridge/ops/tracker.mjs` | `a hand-added application lands in the tracker with an N/A score` |
| 1.11 | Find a row or a report by free text | `find.mjs` | `views/pipeline.rs::find_card` | `a search finds a tracker row by name` |
| 1.12 | Compare two or three applications side by side | `/pipeline/compare` | `views/pipeline.rs::compare_card` → `analysis` op `compare` | `two applications can be compared side by side` |
| 1.13 | Open a posting URL | `web/src/lib/posting-url.mjs` | `views/pipeline.rs::open_url` | `pipeline_screen_offers_report_and_posting_actions` |

## 2. Report reading and its checks

| # | Capability | Web source | Desktop surface | Proof |
|---|---|---|---|---|
| 2.1 | Open one application's report | `/pipeline/[id]` → `report-view.tsx` | `bridge/ops/report.mjs`, `views/report.rs` | `report_screen_shows_the_report_and_the_source_posting`, `clicking_report_opens_that_application` |
| 2.2 | Fold the blocks: the lead and the verdict stay open | `report-view.tsx` progressive disclosure | `views/report.rs::body_card` | `the_report_screen_opens_the_lead_and_folds_the_rest` |
| 2.3 | Show the verdict: score, status, legitimacy, the VERDICT line | `report-view.tsx`, `format.ts` | `views/report.rs::verdict_card` | `report_screen_shows_the_report_and_the_source_posting` |
| 2.4 | Keyword coverage against the CV, never writing to it | `keyword-match.mjs`, `GET /api/keyword-match` | `analysis` op `keywordMatch` | `the keyword coverage is reported without touching the CV` |
| 2.5 | Score levers from the report's own machine summary | `score-levers.tsx`, `machine-summary.mjs` | `analysis` op `scoreLevers` | `the score levers are read from the report's own machine summary` |
| 2.6 | A company's evidence card | `company-history.mjs`, `GET /api/company-history` | `analysis` op `companyHistory` | `a company history joins the tracker, the follow-ups and the status log` |
| 2.7 | Keep the JD archived with the report | `check-jd-archive.mjs`, `jd-capture.mjs` | `maintenance` op `jdArchive`; the report op never strips it | `a report keeps the JD it archived`, `the JDs every report kept are checked by the core's own tool` |
| 2.8 | The screening questionnaire, read and never submitted | `application-answers.mjs` | `questions` op, `views/report.rs` | `a screening questionnaire is read from the report and never sent` |
| 2.9 | Is the posting still live? | `check-liveness.mjs` | `liveness` op, `views/report.rs` | `a liveness verdict is never 'active' for a posting that is not there` |

## 3. Runs and CLI integration

| # | Capability | Web source | Desktop surface | Proof |
|---|---|---|---|---|
| 3.1 | Spawn a chosen AI CLI headlessly, argv fenced to the worker's capabilities | `POST /api/run`, `spawn-cli.mjs`, `cli-fencing.mjs` | `bridge/ops/run.mjs` | `the bridge's run path imports the same engine modules the web route does`, `the bridge builds the same argv as the web app for every CLI and kind` |
| 3.2 | Stream progress as text, tool, status, error, done | `job-stream.mjs` | `app.rs::absorb_events` → `views/run.rs` | `run_screen_renders_streamed_progress` |
| 3.3 | Say when a CLI has gone quiet | `views/run.rs`, `app.rs::run_clock` | same | `a_keepalive_does_not_count_as_the_agent_speaking`, `a_silent_run_says_so_instead_of_only_saying_running` |
| 3.4 | Say when the chosen CLI was substituted | `cliSubstitutionNotice()` | `app.rs::resolve_cli`, `bridge/ops/run.mjs` | `a run on an absent CLI says it substituted one` |
| 3.5 | Refuse a worker the CLI cannot be granted | `isCliAllowedForCapabilities` | `bridge/ops/run.mjs` | `a CLI that cannot be fenced says so, and is refused a write-capable worker` |
| 3.6 | Estimate a run's cost before spending it | `run-cost-estimate.mjs` | `cost` op `estimate` | `the cost estimate comes from this client's own runs` |
| 3.7 | Record tokens and cost per run, and show the 5h and 7d windows | `claude-usage.mjs`, `GET /api/usage` | `runs` op, `cost` op `usage`, `views/run.rs::sensor_card` | `the_run_screen_shows_what_has_been_spent` |
| 3.8 | Save a run log the CLI assistant can read | `POST /api/runs/save` | `runs` op → `data/runs/` | `a finished run leaves a readable record the assistant can read` |
| 3.9 | Repair a broken portal from the run screen | `POST /api/run` kind `fix-portal` | `views/run.rs` kind, `bridge/ops/run.mjs` | `a broken portal is repaired only for a company name that is safe to pass` |
| 3.10 | A mode run: prep, plan, practice, debrief, red flag, offer walk, email, outreach | the CLI's own modes | `bridge/mode-prompt.mjs`, `views/run.rs` | `a mode run asks the agent to follow the mode file`, `a mode whose file is missing is refused, not sent to the agent` |
| 3.11 | A batch: one posting at a time, each its own report | `batch/` | `bridge/ops/run.mjs` `inputs` | `a batch scores one posting at a time, and each one is its own run` |

## 4. CV

| # | Capability | Web source | Desktop surface | Proof |
|---|---|---|---|---|
| 4.1 | Read, edit and save `cv.md`, atomically, with a backup | `/api/cv` → `atomicWriteWithBackup` | `bridge/ops/cv.mjs`, `views/cv.rs` | `cv_screen_shows_the_cv_and_its_editor`, `the CV write is atomic and leaves a backup` |
| 4.2 | Tailor the CV to a posting and render a PDF | `modes/pdf.md`, `generate-pdf.mjs` | `bridge/ops/run.mjs` → `pdf-render.mjs` | `the pdf run leaves the tracker PDF column alone when the render does not happen` |
| 4.3 | Check the tailored CV's order before it is rendered | `cv-experience-order.mjs` | `bridge/ops/run.mjs::checkCvOrder` | `a tailored CV whose roles are out of order is reported before it is rendered` |
| 4.4 | Check the CV's structure against the file the user wrote | `verify-cv-structure.mjs` | `cv` op `verify`, `views/cv.rs` | `the CV's structure is checked against the file the user wrote` |
| 4.5 | Verify a generated document's claims against its sources | `verify-cv-facts.mjs` | `cv` op `facts` | `the CV's claims are checked against the source files` |
| 4.6 | Choose the template the `pdf` run fills | `cv-templates.mjs`, `core/cv-template.mjs` | `config` op `setCvTemplate`, `views/cv.rs::template_card` | `the CV template setting changes the template the run fills` |
| 4.7 | Propose additions from `documents/`, never writing them | `modes/intake.md`, `intake.mjs` | `profile` op `intake` | `the intake lists the documents it found, and proposes nothing by itself` |
| 4.8 | Discover competencies the CV is missing | `skill-extract.mjs` | `profile` op `expand` | `the competency extractor only proposes what the core's vocabulary knows` |
| 4.9 | Weighted skill gaps from the pipeline | `upskill.mjs` | `analysis` op `upskill` | `the skill gaps exclude the skills already on the CV` |
| 4.10 | A source-backed master career profile | `career-profile.mjs` | `profile` op `masterProfile` | `the master career profile is validated by the core` |

## 5. Cover letters and outreach

| # | Capability | Web source | Desktop surface | Proof |
|---|---|---|---|---|
| 5.1 | Draft a cover letter from the four answers the mode asks | `modes/cover.md`, `POST /api/run` kind `cover` | `views/run.rs` kind `cover` | `the cover run produces a draft letter in the run log` |
| 5.2 | Choose the letter's tone from the engine's own set | `cover-tones.mjs` | `views/run.rs::COVER_TONES` | `the_run_screen_offers_the_cover_tones` |
| 5.3 | Draft an application email, never sent | `modes/email.md` | `views/run.rs` kind `email` | `a mode run asks the agent to follow the mode file` |
| 5.4 | Find the right contact and draft outreach in the channel's limit | `modes/contacto.md`, `contact-lookup.mjs` | `views/run.rs` kind `contacto` | `the_run_screen_offers_the_outreach_mode` |
| 5.5 | A phonebook that round-trips through vCard | `contacts.mjs` | `contacts` op, `views/outcomes.rs` | `the phonebook is read, exported and joined against a company` |
| 5.6 | A warm introduction from a LinkedIn export | `linkedin-join.mjs` | `contacts` op `linkedin` | `the phonebook is read, exported and joined against a company` |
| 5.7 | Match a pasted invite to a tracker row | `invite-match.mjs` | `contacts` op `inviteMatch` | `a pasted invite is matched to a tracker row` |

## 6. Follow-ups and outcomes

| # | Capability | Web source | Desktop surface | Proof |
|---|---|---|---|---|
| 6.1 | Show what is due, from the core's cadence calculator | `GET /api/followups` | `bridge/ops/followups.mjs` | `followups_screen_lists_due_followups_and_the_log` |
| 6.2 | Log a follow-up, and remove a mis-logged one | `POST/DELETE /api/followups/log` | same | `a follow-up is logged through the shared writer and appears in the list`, `the_follow_ups_screen_offers_a_remove_and_a_pin` |
| 6.3 | Pin the next date, and clear the pin | `POST/DELETE /api/followups/override` | same | `the_follow_ups_screen_offers_a_remove_and_a_pin` |
| 6.4 | Seed the first date when a row turns Applied | `followup-seed.mjs` | `set-status.mjs`'s own hook, `followups` op `seed` | `a row that turned Applied pins its first follow-up` |
| 6.5 | Turn follow-ups off entirely | `followups-setting.mjs` | `views/config.rs`, `shell.rs::nav_entries` | `the_follow_ups_entry_is_hidden_when_follow_ups_are_off` |
| 6.6 | What is due on Today, without the CLI | `scripts/followup-sweep.sh` | `views/today.rs` | `today_screen_renders_the_daily_loop_tiles` |
| 6.7 | Companies still silent after an interview | `rejection-latency.mjs` | `analysis` op `quietCompanies` | `the quiet companies are listed with the reason each one is quiet` |
| 6.8 | Per-company recruiting friction | `process-quality.mjs` | `analysis` op `processQuality` | `the recruiting friction comes from the interview notes` |
| 6.9 | Record an outcome and archive the artifacts | `outcome.mjs` | `outcomes` op, `views/outcomes.rs` | `the_outcomes_screen_proposes_an_outcome_before_recording_it`, `an outcome is recorded through the core and archives the posting` |
| 6.10 | Walk an offer's clauses | `modes/offer-prep.md` | `views/run.rs` kind `offer-prep` | `a mode run asks the agent to follow the mode file` |
| 6.11 | Negotiation points anchored in a verified figure | `negotiation-roi.mjs` | `analysis` op `negotiation` | `a negotiation figure survives only if the CV says it too` |
| 6.12 | The salary gap: wanted, advertised, actual | `salary-gap.mjs` | `analysis` op `salaryGap` | `the salary gap is wanted against advertised against actual` |
| 6.13 | Share a hire, once, with nothing leaving the machine | `hired-share.mjs` | `outcomes` op `hiredShare` | `a hire may be shared, and nothing leaves the machine` |

## 7. The supply side

| # | Capability | Web source | Desktop surface | Proof |
|---|---|---|---|---|
| 7.1 | What is new this week, with the dead excluded | `GET /api/whats-new` | `portals` op `whatsNew` | `what is new this week comes from the scan history and excludes the dead` |
| 7.2 | Audit the watched boards | `audit-portals.mjs`, `POST /api/portals/verify` | `portals` op `audit` | `the watched boards are audited and the verdict is per board` |
| 7.3 | Edit the watched companies, preserving unknown keys | `portals.yml`, `portals-config.mjs` | `portals` op `read`/`write` | `the watched boards can be edited and a file that cannot be parsed is refused` |
| 7.4 | Explain why a posting you found by hand was missed | `coverage-check.mjs` | `portals` op `coverage` | `a posting you found by hand is explained, not guessed at` |
| 7.5 | Flag roles re-listed repeatedly | `detect-reposts.mjs` | `portals` op `reposts`, `views/portals.rs` | `a role that keeps being re-listed is flagged, and one posting is not enough` |
| 7.6 | Discover a company's board from its name | `discover-ats.mjs` | `portals` op `discover` | `a company name becomes a verified board, never a guessed URL` |
| 7.7 | Fetch a JD without a browser where an ATS API exists | `fetch-jd.mjs` | `liveness` op `fetchJd` | `the JD is fetched without a browser where an ATS API exists` |

## 8. Documents

| # | Capability | Web source | Desktop surface | Proof |
|---|---|---|---|---|
| 8.1 | List every generated CV and cover letter | `GET /api/documents` | `documents` op `list`, `views/documents.rs` | `the_documents_screen_lists_what_was_generated` |
| 8.2 | Open a generated document | `GET /api/documents?id=` | `documents` op `resolve` | `the documents can be opened from the list` |
| 8.3 | Archive a posting as a capture | `archive-posting.mjs`, `jd-capture.mjs` | `documents` op `archive` | `the_documents_screen_lists_what_was_generated` |
| 8.4 | Keep the PDF column honest | `sync-pdf-flags.mjs`, `data/pdf-index.tsv` | `maintenance` op `pdfFlags` | `the PDF column is synced with the files that exist` |
| 8.5 | **Preview a PDF page in the window**, and page through it | the web app's `<iframe>` | `src/pdf.rs` (poppler → PPM), `views/pdfview.rs` | `a_page_is_rasterised_into_pixels`, `the_report_screen_offers_a_pdf_preview_and_the_viewer_opens_it`, `the_documents_screen_offers_a_preview_of_what_it_lists`, `a_preview_that_cannot_be_made_says_why` |

## 9. Interview preparation

| # | Capability | Web source | Desktop surface | Proof |
|---|---|---|---|---|
| 9.1 | A prep pack, generated once and readable offline | `modes/interview-prep.md` | `views/run.rs` kind `prep`, the saved run log | `a mode run asks the agent to follow the mode file` |
| 9.2 | A time-blocked plan | `modes/interview/plan.md` | kind `plan` | same |
| 9.3 | Practice with specific feedback | `modes/interview/practice.md` | kind `practice` | same |
| 9.4 | Debrief a real interview | `modes/interview/debrief.md` | kind `debrief`, `interview` op `sessions` | same |
| 9.5 | Whether a company is safe to join | `modes/interview-redflag.md` | kind `redflag` | same |
| 9.6 | A STAR story bank whose figures carry provenance | `interview-prep/story-bank.md`, `story-provenance-check.mjs` | `interview` op `storyBank`, `views/interview.rs` | `the story bank says which of its numbers may be restated as fact` |
| 9.7 | Match a story to a question | `match-star.mjs` | `interview` op `matchStar` | `a question is matched to a story from the bank` |
| 9.8 | Roll a week of sessions into a digest | `weekly-digest.mjs` | `interview` op `digest` | `a week of sessions is rolled into one summary per company` |

## 10. Config and the app's own upkeep

| # | Capability | Web source | Desktop surface | Proof |
|---|---|---|---|---|
| 10.1 | Detect the installed AI CLIs and choose one | `GET /api/clis`, `config-form.tsx` | `views/config.rs::cli_section` | `config_screen_shows_the_cli_picker_and_the_cadence`, `config_shows_the_installed_clis_first_and_folds_the_rest_away` |
| 10.2 | Read and validate `config/profile.yml` | `validate-profile.mjs` | `config` op `validate` | `the profile validator surfaces a broken profile as an error`, `the_config_screen_shows_what_the_profile_validator_found` |
| 10.3 | Merge-safe write of the candidate details | `POST /api/profile` | `config` op `setProfile` | `saving_the_profile_from_config_merges_rather_than_replaces`, `a first save on a fresh install seeds the profile and never overwrites one` |
| 10.4 | The output language and the market mode set | `profile-language.mjs`, `profile-keywords.mjs` | `profile` op `state`, `views/config.rs::language_section` | `the profile's language and its market mode set are read` |
| 10.5 | Edit `modes/_profile.md` ONE section at a time | `POST /api/personalization` | `profile` op `save`, `views/config.rs::personalization_editor` | `the_config_screen_edits_one_personalization_section`, `the personalization sections come back with their bodies, so one can be edited` |
| 10.6 | List, enable and audit plugins | `plugins.mjs`, `plugin-audit.mjs` | `plugins` op, `views/config.rs::plugins_section` | `the plugin list is read and a plugin can be disabled` |
| 10.7 | Add a plugin, confirmed and pinned | `plugin-install.mjs` | `plugins` op `install` | `the_config_screen_confirms_a_plugin_install`, `a plugin install is confirmed, and a preview is not an install` |
| 10.8 | The core's own repair tools, a proposal first | `verify-pipeline.mjs`, `normalize-statuses.mjs`, `dedup-tracker.mjs`, `fix-report-links.mjs`, `reconcile-pipeline.mjs`, `check-jd-archive.mjs`, `sync-pdf-flags.mjs`, `fix-slugs.mjs` | `maintenance` op, `views/config.rs::maintenance_section` | `the health check reports a broken report link`, `normalizing rewrites a non-canonical status and is idempotent`, `duplicate tracker rows are merged by the core, with a backup`, `a dead report link is repaired and the row is named`, `the pipeline is reconciled against the batch state`, `every core maintenance tool is reachable and reports what it did` |
| 10.9 | Queue a request for a later session | `agent-inbox.mjs` | `agent-inbox` op, `views/config.rs::inbox_section` | `the agent inbox is a checklist that is drained, not run` |
| 10.10 | Where the data and the engine checkout come from | `web/src/lib/core/data-root.mjs` | `views/config.rs::install_section` | `config_screen_reports_where_the_data_comes_from` |
| 10.11 | Say which side is behind when the pair does not match | — | `version` op, `bridge.rs::PROTOCOL_VERSION` | `a_run_is_refused_when_the_app_and_the_bridge_disagree`, `the bridge reports the protocol version it speaks` |
| 10.12 | One actionable sentence when the checkout is missing | — | `bridge/code-root.mjs`, `src/code_root.rs` | `a_missing_engine_checkout_is_one_actionable_sentence`, `the engine checkout is resolved the same way in Node and in Rust` |

## 11. The shell itself

| # | Capability | Desktop surface | Proof |
|---|---|---|---|
| 11.1 | One process tree, no listening socket | `bridge/server.mjs` | `the bridge never binds a socket` |
| 11.2 | Survive a bridge crash without wedging | `src/bridge.rs` | `a_bridge_that_dies_fails_the_pending_calls_with_bridge_gone` |
| 11.3 | Read the real data root read-only, and prove it | `tests/live_data.rs` | `the_live_data_root_decodes_and_is_left_untouched` |
| 11.4 | Zoom and theme, persisted | `theme.rs`, `settings.rs` | `the_zoom_and_the_theme_are_saved` |
| 11.5 | Keyboard navigation to every screen | `app.rs::handle_screen_shortcuts` | `ctrl_number_keys_reach_every_screen` |
| 11.6 | Reopen where you left off | `settings.last_screen` | `the_app_reopens_on_the_screen_it_was_left_on` |
| 11.7 | Open a PDF or a URL in the user's own viewer | `views/report.rs::open_file`, `views/pipeline.rs::open_url` | `pipeline_screen_offers_report_and_posting_actions` |
| 11.8 | No webview, in the lockfile and in `ldd` | `tests/no_webview.rs` | `the_lockfile_names_no_webview_crate`, `the_built_binary_links_no_webview_runtime` |
| 11.9 | A PDF page in the window, with no browser and no PDF crate | `src/pdf.rs`, `views/pdfview.rs` | `a_page_is_rasterised_into_pixels`, `a_higher_resolution_makes_more_pixels` |

## Not in scope — named gaps

Deliberately absent. Each needs a browser, an OS scheduler or an OS credential
store, and `FEATURES.md` records every one as `gap` with the reason.

| Gap | Web surface | Why it is out of scope |
|---|---|---|
| **Apply form prefill** | `/apply`, `POST /api/apply/*` | It drives a real browser and fills a real form. |
| **Agent chat** | `assistant-console.tsx`, `POST /api/assistant` | A conversational surface over the same engine. |
| **Scanning and explore** | `/explore`, `/api/explore*`, the scanners | The supply loop is the CLI's; this client shows what a scan already found. |
| **CV studio** | `POST /api/cv-studio/[n]` | Per-block CV editing is a deferred surface. |
| **Scheduled scans** | `/api/scheduler`, `/api/scheduled-jobs*` | It installs OS-level schedulers. |
| **Analytics** | `/analytics`, `/api/stats`, `/api/patterns` | Read-only reporting over the whole history. |

## Scope note

This file covers what a person needs to run their search. The whole product is
wider than that, and the wider list lives in [FEATURES.md](FEATURES.md): 152
rows across 18 epics, with every capability the career-ops checkout has,
whether or not a native client should have it. Where the two disagree,
FEATURES.md is the inventory and this file is the parity claim.

## The evidence, in one place

```bash
cargo test                                     # the Rust side
node --test tests/desktop-*.test.mjs           # the Node side
node dev/dependency-audit.mjs                  # lockfile + ldd
node --test tests/acceptance.test.mjs          # FEATURES.md against the code
```

### What each suite is for

| Suite | Question it answers |
|---|---|
| `tests/screens.rs` | Does each screen show what the web UI shows, does the layout survive the narrowest canvas, and do the CONTROLS do what they look like they do? |
| `tests/payloads.rs` | Can the client decode what the engine actually sends — nulls, missing keys, unknown events? |
| `tests/no_webview.rs` | Is the "no webview" claim still true, in the lockfile and in `ldd`? |
| `tests/live_data.rs` | Do the client's types decode THIS machine's data root, and does the read leave it byte-identical? Opt-in: `CAREER_OPS_REAL_ROOT_TESTS=1 cargo test --test live_data -- --nocapture`. |
| `tests/desktop-engine-parity.test.mjs` | Is it still the same engine, and is the checkout resolved the same way on both sides? |
| `tests/desktop-bridge-ops.test.mjs` | Do the reads and writes behave — canonical writers, refusals, recovery? |
| `tests/desktop-run-e2e.test.mjs` | Does a whole run, start to finish, land through the canonical scripts, and is a bad CV stopped before it is rendered? |
| `tests/desktop-engine.test.mjs` | Do the core's own answers reach the client — the reader, the analyses, the interview half, the supply side, the upkeep? |
| `tests/acceptance.test.mjs` | Does FEATURES.md still tell the truth, and does every row name a check that exists? |
