# Capability matrix — native desktop client vs. the web UI

Derived from `web/`'s actual routes and components, **before** the desktop
client was built, from these sources:

| Source read | What it establishes |
|---|---|
| `web/src/lib/nav-items.ts` | The app's primary destinations: Today, Explore, Scheduled scans, Pipeline, Follow-ups, Portals, Analytics, CV, Documents, Config. |
| `web/src/app/**/page.tsx` (16 routes) | One route = one screen. |
| `web/src/app/api/**/route.ts` (44 routes) | The exact operations each screen performs, and which of them WRITE. |
| `web/src/components/**` | The interactions: `pipeline-view.tsx` (sort keys, status tabs, min-score filter, search, inline status), `report-view.tsx` (verdict, posting link, PDF links), `job-store.tsx` + `web/src/lib/job-stream.mjs` (the NDJSON progress stream), `cv-editor.tsx`, `followups-view.tsx`, `config-form.tsx`. |
| `web/src/lib/core/` + `web/src/lib/*.mjs` | The access layer, so the desktop client reuses it instead of re-deriving it. |

The rule applied: a capability is **in scope** when it is part of the daily
loop named in the Goal — tracker, reports, runs, CV, follow-ups, config.
Everything else is named as a gap below rather than half-built.

## In scope, and what proves it

Every row names the web surfaces it replaces, the desktop surface that
implements it, and the **executable** check that would fail if the capability
broke. `cargo test`, `node --test`, and the two scripts at the bottom are the
only instruments.

### 1. Pipeline / tracker

| # | Capability | Web source | Desktop surface | Proof |
|---|---|---|---|---|
| 1.1 | List every tracker row with company, role, score, status, date | `/pipeline` → `pipeline-view.tsx` | `views/pipeline.rs` | `tests/screens.rs::pipeline_screen_renders_the_tracker_rows` |
| 1.2 | Sort by #, company, role, score, status or date, ascending and descending, tie-broken by tracker # | `pipeline-view.tsx` sort keys + `web/src/lib/core/pipeline-sort.mjs` | `views/mod.rs::sort_rows` | `tests/screens.rs::pipeline_filters_by_status_and_score` (the rule), `::clicking_a_column_header_reorders_the_rows` (the header actually sorts) |
| 1.3 | Filter by canonical status | 12 status tabs in `pipeline-view.tsx` | `views/pipeline.rs::filter_bar` | `tests/screens.rs::pipeline_screen_exposes_every_canonical_state_filter`, `::clicking_a_status_tab_filters_the_table` |
| 1.4 | Filter by a minimum score | `?min=` in `pipeline-view.tsx` | `views/pipeline.rs::filter_bar` | `tests/screens.rs::pipeline_filters_by_status_and_score` |
| 1.5 | Free-text search over company, role, notes | `pipeline-view.tsx` search box | `views/mod.rs::filter_rows` | `tests/screens.rs::pipeline_filters_by_status_and_score`, `::typing_in_the_search_box_filters_as_you_type` |
| 1.6 | Change a row's status inline, to a **canonical** state, through the one writer | `POST /api/status` → `set-status.mjs` | `bridge/ops/status.mjs` → `set-status.mjs --source desktop` | `tests/desktop-bridge-ops.test.mjs::a status change lands through set-status.mjs and is attributed to desktop`; `tests/screens.rs::a_status_change_goes_through_the_status_op` |
| 1.7 | A refused state writes nothing | `status/route.ts` 400 path | `bridge/ops/status.mjs` | `tests/desktop-bridge-ops.test.mjs::a rejected state writes nothing` |
| 1.8 | Every state the UI offers is writable | `core/states.ts` reads `templates/states.yml` | `views/pipeline.rs` renders the engine's list | `tests/desktop-bridge-ops.test.mjs::every canonical state the engine offers is accepted by the writer` |
| 1.9 | A busy tracker reads as retryable, not as success | `status/route.ts` exit-4 → 503 | `app.rs::describe_status_error` | `tests/screens.rs::a_busy_tracker_is_reported_as_retryable` |
| 1.10 | Show the pending-posting inbox with its count | `GET /api/pipeline` → `inbox` | `views/pipeline.rs` header, `views/today.rs` | `tests/screens.rs::pipeline_screen_renders_the_tracker_rows` |
| 1.11 | Skip a pending posting, and undo it | `POST /api/inbox/skip` → `inbox-skip.mjs` under the pipeline lock | `bridge/ops/inbox.mjs` | `tests/desktop-bridge-ops.test.mjs::skipping a pending posting flips the checkbox and nothing else` — asserts the byte-for-byte round trip and the `not-found` refusal |

### 2. Report reading, and the source posting

| # | Capability | Web source | Desktop surface | Proof |
|---|---|---|---|---|
| 2.1 | Open one application's evaluation report | `/pipeline/[id]` → `report-view.tsx`, `career-ops.ts::readReport` | `bridge/ops/report.mjs`, `views/report.rs` | `tests/screens.rs::report_screen_shows_the_report_and_the_source_posting`, `::clicking_report_opens_that_application` (asserts the exact row was requested AND the screen opened) |
| 2.2 | Render the report's markdown (headings, emphasis, lists, tables, code) | `report-markdown.tsx` | `markdown.rs` | `tests/screens.rs::report_screen_shows_the_report_and_the_source_posting` (body + VERDICT line) |
| 2.3 | Show the verdict: score, status, legitimacy tier | `report-view.tsx` header, `format.ts::legitimacyTone` | `views/report.rs` | same test (score, status, "High Confidence") |
| 2.4 | **Open the source posting** | posting link in `report-view.tsx` / `pipeline-view.tsx` | `views/report.rs` + `views/pipeline.rs::open_url` | `tests/screens.rs::report_screen_shows_the_report_and_the_source_posting`, `::pipeline_screen_offers_report_and_posting_actions` |
| 2.5 | Open the tailored CV / cover PDF for that application | `GET /api/cv-pdf`, `GET /api/cover-pdf` | `bridge/ops/report.mjs` (`resolveTailoredCv`/`resolveTailoredCover`), opened via `xdg-open` | `tests/screens.rs::report_screen_shows_the_report_and_the_source_posting`; resolution is `web/src/lib/apply/{cv,cover}.ts` itself |

### 3. Structured run screens, with streamed progress

| # | Capability | Web source | Desktop surface | Proof |
|---|---|---|---|---|
| 3.1 | Evaluate a pasted JD or URL | `/` quick-evaluate, `POST /api/run` kind `evaluate` | `views/run.rs`, `bridge/ops/run.mjs` | `tests/screens.rs::run_screen_offers_the_three_structured_runs`, `::clicking_start_hands_the_run_to_the_engine` (the Start button produces exactly one `run` call with the right kind, input and CLI); the whole path is pinned by `tests/desktop-engine-parity.test.mjs` |
| 3.2 | Tailor the CV (kind `pdf`) | report page "Generate tailored CV" | `views/run.rs` | same |
| 3.3 | Draft a cover letter (kind `cover`) | report page "Draft cover letter" | `views/run.rs` | same |
| 3.4 | Stream progress: text, tool, status, error, done — and say when a CLI has gone quiet | `job-stream.mjs` + `job-store.tsx` | `bridge/ops/run.mjs` → `app.rs::absorb_events_at` → `views/run.rs` | `tests/screens.rs::run_screen_renders_streamed_progress`, `::a_keepalive_does_not_count_as_the_agent_speaking`, `::a_silent_run_says_so_instead_of_only_saying_running` |
| 3.5 | Same spawn, CLI fencing, envelope and persist path as the web app | `POST /api/run` | `bridge/engine.mjs` + `bridge/ops/run.mjs` | `tests/desktop-engine-parity.test.mjs` (module identity, argv parity, no second writer) **and** `tests/desktop-run-e2e.test.mjs`, which runs a whole evaluation against a stub CLI and asserts on the files it produced |
| 3.6 | The report is saved by `reserve-report-num.mjs` → `persist-evaluation-report.mjs` → `merge-tracker.mjs`, and only then does the run report its number | `report-persist.mjs::persistEvaluation` | same module, unchanged | `tests/desktop-run-e2e.test.mjs` — the new report file, the new tracker row with its score and report link, and the released `-RESERVED` sentinel are all asserted |
| 3.7 | The report number the run produced can be opened immediately | `reportN` in the `done` event | `app.rs` → `views/run.rs` "Open it" | `tests/screens.rs::a_done_event_endpoints_the_run_and_records_the_report_number` |

### 4. CV viewing and generation

| # | Capability | Web source | Desktop surface | Proof |
|---|---|---|---|---|
| 4.1 | Read `cv.md` | `GET /api/cv` | `bridge/ops/cv.mjs` | `tests/desktop-bridge-ops.test.mjs::the bridge answers every read op…` |
| 4.2 | Edit and save `cv.md`, atomically, with a backup | `POST /api/cv` → `atomicWriteWithBackup` | `bridge/ops/cv.mjs`, `views/cv.rs` | `tests/desktop-bridge-ops.test.mjs::the CV write is atomic and leaves a backup` |
| 4.3 | Live markdown preview while editing | `cv-editor.tsx` | `views/cv.rs` | `tests/screens.rs::cv_screen_shows_the_cv_and_its_editor` |
| 4.4 | Generate a tailored CV as a PDF | `POST /api/run` kind `pdf` → `generate-pdf.mjs` | `bridge/ops/run.mjs` → `pdf-render.mjs` | `tests/desktop-engine-parity.test.mjs` (argv + persistence) |

### 5. Follow-ups

| # | Capability | Web source | Desktop surface | Proof |
|---|---|---|---|---|
| 5.1 | Show what is due, from the core's own cadence calculator | `GET /api/followups` → `followup-cadence.mjs --json` | `bridge/ops/followups.mjs` | `tests/desktop-bridge-ops.test.mjs::the bridge answers every read op…`; `tests/screens.rs::followups_screen_lists_due_followups_and_the_log` |
| 5.2 | Show urgency and the countdown | `followups-view.tsx`, `followups.ts` | `views/followups.rs` | `tests/screens.rs::followups_screen_lists_due_followups_and_the_log` ("3 days overdue") |
| 5.3 | Log a follow-up into `data/follow-ups.md` | `POST /api/followups/log` | `bridge/ops/followups.mjs` (same table row, same lock) | `tests/desktop-bridge-ops.test.mjs::a follow-up is logged through the shared writer and appears in the list` (the write), `tests/screens.rs::logging_a_followup_from_the_dialog_writes_through_the_bridge` (the dialog's Save produces exactly one write with its fields) |
| 5.4 | Remove a mis-logged follow-up | `DELETE /api/followups/log` | same | same test |
| 5.5 | Show the logged history | `log-dialog.tsx` history panel | `views/followups.rs` | `tests/screens.rs::followups_screen_lists_due_followups_and_the_log` |
| 5.6 | Read and write the cadence knobs in `config/profile.yml`, merge-safely | `GET/POST /api/followups/cadence` | `bridge/ops/followups.mjs` (cadence.get / cadence.set) | `tests/desktop-bridge-ops.test.mjs::the cadence read and write go through config/profile.yml` |
| 5.7 | Follow-ups can be turned off in the profile | `followups-setting.mjs` | `views/followups.rs`, `app.rs::due_followups` | `tests/desktop-bridge-ops.test.mjs::the cadence read and write…` (`enabled: false`) |

### 6. Config and settings the above depends on

| # | Capability | Web source | Desktop surface | Proof |
|---|---|---|---|---|
| 6.1 | Detect installed AI CLIs and choose one | `GET /api/clis`, `config-form.tsx` | `views/config.rs`, `cli.rs` table via `clis.ts` | `tests/screens.rs::config_screen_shows_the_cli_picker_and_the_cadence` |
| 6.2 | A run cannot start without a usable CLI | run buttons disabled in `job-store.tsx` | `views/run.rs` | `tests/screens.rs::run_screen_keeps_start_disabled_when_no_cli_is_selected` |
| 6.3 | Show setup state / what is missing | `GET /api/doctor`, `onboarding-banner.tsx` | `bridge/ops/bootstrap.mjs` → `views/today.rs`, `views/config.rs` | `tests/screens.rs::today_screen_renders_the_daily_loop_tiles` |
| 6.4 | Read and merge-safe write the candidate profile | `POST /api/profile` | `bridge/ops/config.mjs` | `tests/screens.rs::config_screen_shows_the_cli_picker_and_the_cadence`, `::saving_the_profile_from_config_merges_rather_than_replaces` (the Save is a patch of the fields it shows, never a replace); the merge itself is `web/src/lib/profile-patch.mjs` + the same guards, asserted in `tests/desktop-bridge-ops.test.mjs` |
| 6.5 | Show which data root and checkout the app is reading | `web/src/lib/core/data-root.mjs` | `views/config.rs::install_section` | `tests/screens.rs::config_screen_reports_where_the_data_comes_from` |
| 6.6 | Land on a daily overview: counts, what is due, a way into a run | `/` → `today-dashboard.tsx` | `views/today.rs` | `tests/screens.rs::today_screen_renders_the_daily_loop_tiles` |

## Not in scope — named gaps

These are **deliberately absent**, not unfinished. Each is a real web surface
with real routes behind it, and building a partial version would be worse than
not building it: a half-working apply flow or scan could act on the user's
files in ways the web app's guards exist to prevent.

| Gap | Web surface | Why it is out of scope for this Goal |
|---|---|---|
| **Apply form prefill** | `/apply`, `POST /api/apply/{session,prefill,fill,drive,close}` | The Goal names it out of scope. It drives a real browser and fills a real form; reproducing it natively would mean embedding or driving a browser, which is exactly what this client refuses to do. |
| **Agent chat** | `assistant-console.tsx`, `POST /api/assistant`, `POST /api/assistant/chats` | Named out of scope. It is a conversational surface over the same engine, not part of the tracker/report/run/CV/follow-up loop. |
| **Explore and scan** | `/explore`, `POST /api/explore`, `/api/explore/ai`, `/api/explore/add`, `/api/whats-new` | Named out of scope. Discovery is a supply-side loop of its own, with its own streaming protocol and cost model. |
| **CV studio** | `POST /api/cv-studio/[n]` | Named out of scope. The desktop client generates the tailored CV through the `pdf` run; the studio's per-block editing surface is not reproduced. |
| **Scheduled scans** | `/scheduled-scans`, `/api/scheduled-jobs*`, `/api/scheduler` | Named out of scope. It installs OS-level schedulers; that is a system-administration surface, not a daily-loop screen. |
| **Analytics** | `/analytics`, `/api/stats`, `/api/patterns`, `pipeline-sankey.tsx` | Named out of scope. Read-only reporting over the whole history, with its own visualisation work. |
| **Documents browser** | `/documents`, `GET /api/documents` | Not in the Goal's list. The report screen still opens the tailored CV and cover PDFs for the application in front of the user, which is the daily-loop need. |
| **Portals health check** | `/portals`, `POST /api/portals/verify` | Not in the Goal's list. It verifies watched job boards; that is supply-side maintenance. |
| **Inbox triage scoring** | shortlist tray, "Score shortlist", facet chips in `inbox-triage.tsx` | The inbox is shown and can be skipped through the canonical writer; the shortlist/score-many flow is a batch driver, and browser-local state (`localStorage`) has no native equivalent worth inventing. |
| **Compare two or three offers** | `/pipeline/compare` | Not in the Goal's list. Read-only, and the report screen already shows each report in full. |
| **Tracker row deletion** | `POST /api/tracker/delete` | Not in the Goal's list. It is destructive and irreversible; the CLI remains the place to do it. |
| **Memory / personalization editing** | `POST /api/memory`, `POST /api/personalization` | Not in the Goal's list. Both edit user-layer files the daily loop only reads. |
| **PDF rendering inside the app** | (the web app shows PDFs in an `<iframe>`) | There is no `<iframe>` without a webview. The desktop client hands the PDF to the user's own viewer via `xdg-open` instead — see `DECISIONS.md` D-6. |

## Scope note

This file covers what this client **has to reach** for the daily loop. The whole
product is wider than that, and the wider list lives in
[FEATURES.md](FEATURES.md): 152 rows across 18 epics, with every capability the
repository has, whether or not a native client should have it. Where the two
disagree, FEATURES.md is the inventory and this file is the parity claim.

## The evidence, in one place

```bash
cd desktop && cargo test                       # 49: 36 headless screen + interaction + layout, 8 payload, 4 audit, 1 live-data (opt-in)
cd .. && node --test tests/desktop-*.test.mjs   # 16: 6 engine parity, 9 live bridge, 1 end-to-end run
node dev/dependency-audit.mjs          # lockfile + ldd, with the full library list
node test-all.mjs                              # the repo's own suite
```

The three `tests/desktop-*.test.mjs` suites are discovered by `node test-all.mjs`
like every other suite under `tests/`, so the repo's own gate covers them.

### What each suite is for

| Suite | Question it answers |
|---|---|
| `tests/screens.rs` | Does each screen show what the web UI shows for the same capability, does the layout survive the narrowest canvas it is designed for, and do the CONTROLS do what they look like they do when clicked? |
| `tests/payloads.rs` | Can the client decode what the engine actually sends — nulls, missing keys, unknown events? |
| `tests/no_webview.rs` | Is the "no webview" claim still true, in the lockfile and in the linked libraries? |
| `tests/desktop-engine-parity.test.mjs` | Is it still the same engine, or has a second implementation appeared? |
| `tests/desktop-bridge-ops.test.mjs` | Do the reads and writes behave — canonical writers, refusals, recovery? |
| `tests/desktop-run-e2e.test.mjs` | Does a whole evaluation, start to finish, land in `reports/` and the tracker through the canonical scripts? |
| `tests/live_data.rs` | Do the client's types decode THIS machine's real data root, and does the read leave it byte-identical? Opt-in: `CAREER_OPS_REAL_ROOT_TESTS=1 cargo test --test live_data -- --nocapture`. Run it before a walkthrough. |
