# Decisions

A running record of the choices this client is made of, and the evidence behind
each one. Ordered by when they had to be made.

---

## D-1. Placement: inside the career-ops checkout (superseded by D-14)

**Decision.** The native client lives in the career-ops checkout, under
`desktop/`, as a Cargo package plus a Node bridge.

**Evidence for in-repo, over anywhere else:**

| Fact | Where |
|---|---|
| Its engine is the repo's own code. The bridge imports `web/src/lib/**` and spawns the core's root scripts; a separate repository would have to vendor or re-fetch both on every release. | `bridge/engine.mjs`, `bridge/alias-hook.mjs` |
| The core already carries a native-adjacent tree at a stable path for exactly this reason (`dashboard/`, a Go app, is in `SYSTEM_PATHS`). | `update-system.mjs` `SYSTEM_PATHS` |
| The repo's convention for "one script = one job, every script registered" is enforced by a coverage guard that only looks at *tracked files in this repo*. A sibling repo would be outside every guard. | `validate-system-paths-coverage.mjs` |
| The user's own working tree is the data root this app is developed against. | `dev/make-fixture.mjs` |

**Registered in `SYSTEM_PATHS`**, as the Goal required: `'desktop/'` plus one
carve-out, `'web/src/lib/'`, because the bridge is built on the web app's engine
modules rather than on a copy of them (see D-2). `node
validate-system-paths-coverage.mjs` passes, and its self-test asserted both
halves of that carve-out: the engine tree ships, the web app itself does not.

**Not in the npm package.** `.npmignore` excluded `desktop/`: it is a Rust
source tree and an npm install cannot build it. The build tree (`target/`) is
gitignored; the updater prunes only tracked files (`git ls-files`), so it can
never delete a user's cargo cache.

**Superseded.** Every one of those reasons was about *installing* the client
alongside the engine through the engine's own updater. That is not what this
client is for: it is one person's client, developed beside a checkout, and the
owner of the upstream repository asked for their tree to be left untouched. D-14
records what replaced this.

---

## D-2. Reuse, not reimplementation: the bridge imports the web app's engine

**Decision.** The bridge holds no engine logic. `bridge/engine.mjs` is
the single import surface, and every module on it is the web app's own file,
reached through the web app's own `@/` alias:

```
bridge/engine.mjs  ──"@/lib/…"──▶  web/src/lib/…
        ▲                                        ▲
        │                                        │
bridge/ops/run.mjs            web/src/app/api/run/route.ts
```

The `@/` alias is a webpack/Turbopack convention; Node has no idea what it
means. `bridge/alias-hook.mjs` installs the same mapping as an ESM
resolve hook, and `server.mjs` imports the ops **dynamically** after the hook is
installed — ESM resolves a module graph before any body runs, so a static import
of an aliased module would be resolved before the hook existed. Node 22 strips
`.ts` types on import, so the web app's TypeScript server modules load as they
are.

**Why not move the engine out of `web/` instead?** `web/server.mjs` states the
invariant in its own header: *"Imports nothing outside web/"*, because the web
tree is excluded from the core's packaging and coverage contracts. Moving the
engine to a top-level `engine/` would have every web module import across that
boundary — inverting the invariant to satisfy a client that is younger than it.
Taking the engine up through the alias kept `web/` untouched: **not one line of
`web/` changed for this client.**

**Evidence it is the same engine:**

- `tests/desktop-engine-parity.test.mjs` — the bridge's run path imports the
  same ten engine modules the web route imports, through `@/lib/`, and the
  bridge tree contains no file with any of those names (no vendored copy).
- The same suite builds the Claude and Codex argv through the web app's own
  functions (`claudeCliArgs`, `codexStreamArgs`, `fenceArgs`) for all three run
  kinds and asserts the fencing still holds and no write-capable tool is
  granted.
- The same suite asserts the run op cannot reach the generic script runner and
  never calls `spawn()` itself: it *hands* `spawn` to `persistEvaluation` and
  `renderAndMarkPdf`, exactly as the web route does. Those are the functions
  that run `reserve-report-num.mjs`, `persist-evaluation-report.mjs`,
  `merge-tracker.mjs`, `generate-pdf.mjs` and `mark-pdf-ready.mjs`.

**One extraction was needed, and it is a two-word one.** A status change from
this client should be attributable in `data/status-log.tsv` without pretending
to be the web app. `set-status.mjs` has an allow-list (`WRITER_SOURCES`) and
`funnel-velocity.mjs` has its own (`VALID_SOURCES`, `DAY_MATH_SOURCES`); a label
in one and not the other is persisted and then silently dropped. `desktop` was
added to both, with the reasoning written beside each. Behavior for every
existing source is unchanged, and
`tests/desktop-engine-parity.test.mjs::\`desktop\` is a source both the writer and
the reader accept` pins the pair.

---

## D-3. The bridge is stdio, and binds nothing

**Decision.** The Rust app starts `node bridge/server.mjs` and talks
newline-delimited JSON over its stdin/stdout. No socket, no port, no token.

The Goal allows a local bridge that "binds loopback only". A stdio bridge
satisfies that by construction rather than by discipline: there is no address to
get wrong, no port to collide with a second instance, and no authentication
story to get wrong either. `tests/desktop-engine-parity.test.mjs::the bridge
never binds a socket` fails if any bridge module ever grows `listen`,
`createServer` or an HTTP server.

**Protocol.** One request per line in, one response per line out, plus
`{"id":N,"event":…}` lines for a streaming run. The event vocabulary is the web
app's NDJSON stream verbatim — `text`, `tool`, `status`, `keepalive`, `error`,
`done` — so the desktop run screen shows the same steps the web run screen does,
and a keepalive stays transport noise rather than becoming a log line
(`app.rs::absorb_events`, pinned by `tests/screens.rs`).

---

## D-4. Every write goes through the canonical script. No second source of truth.

| Write | Path | What the client never does |
|---|---|---|
| Status change | `set-status.mjs --row … <state> --source desktop --json` | never rewrites the tracker cell itself |
| Evaluation report | `persistEvaluation` → `reserve-report-num.mjs`, `persist-evaluation-report.mjs`, `merge-tracker.mjs` | never picks a report number, never writes a TSV, never merges |
| Tailored CV PDF | `pdf-render.mjs` → `generate-pdf.mjs`, `mark-pdf-ready.mjs` | never writes `output/` or the PDF column |
| Follow-up log | `data/follow-ups.md` under the same in-process queue + core file lock as `/api/followups/log` | never appends outside the lock |
| CV | `atomicWriteWithBackup` (temp + rename, timestamped `.bak`) | never writes `cv.md` in place |
| Profile / cadence | `atomicWriteWithBackup` + the same malformed-YAML and non-mapping refusals as `/api/profile` | never seeds over a profile that exists |
| Inbox skip | `inbox-skip.mjs` (checkbox byte only, under the pipeline lock) | never rewrites `data/pipeline.md` |
| Pending-postings skip in the tracker | — | (out of scope; see the matrix) |

`tests/desktop-bridge-ops.test.mjs` drives the real bridge against a throwaway
fixture and asserts each of these on the files: the transition appears in
`status-log.tsv` attributed to `desktop`, a refused state leaves the tracker
byte-identical and writes no ledger row, the follow-up row has the canonical
shape, the CV write leaves a backup.

---

## D-5. The engine checkout and the data root are two different things

`web/src/lib/career-ops.ts` resolves the data root from `CAREER_OPS_ROOT` and
the scripts from `CAREER_OPS_CODE_ROOT`, and its own comment explains why: a
user can point the data root at a data-only directory. The bridge sets
`CAREER_OPS_CODE_ROOT` to the checkout it lives in before importing the engine,
because the library's fallback is `path.resolve(cwd, "..")` — right only when
the process was started from `<checkout>/web`.

`dev/make-fixture.mjs` builds that arrangement for development: user
files are **copied** (a write cannot escape to the original), core scripts are
**symlinked** (a script's `import.meta.url` resolves to the checkout, so it
carries its own configuration, while `CAREER_OPS_ROOT` decides what it reads
and writes). This is the "develop against a fixture copy of the data root via
`CAREER_OPS_ROOT`" requirement, made reproducible.

`tests/helpers/desktop-fixture.mjs` does the same thing in miniature for the
test suites, with its own tracker, report and profile — no test ever reads the
user's real data.

---

## D-6. PDFs are handed to the user's viewer, not rendered (superseded by D-22)

**Decision.** The report screen offers "Open the tailored CV (PDF)" and "Open
the cover letter (PDF)", which call `xdg-open` on the resolved file. It does not
render PDF pages inside the window.

**Why.** The web app shows a PDF in an `<iframe>` — a browser capability. There
is no `<iframe>` without a webview, which this client refuses to embed. The
alternatives were a PDF rasterising crate (a large new dependency, and a second
renderer whose output would not be the file the user is about to send) or
asking the desktop to show the file it already has. The second is both smaller
and more honest: the user sees the exact PDF that would be submitted.

This is recorded as a deliberate difference from the web UI, not an oversight.

---

## D-7. The app's own settings do not go in `config/profile.yml`

Which CLI to drive and which theme to use are preferences of *this client*.
They live in `$XDG_CONFIG_HOME/career-ops-desktop/settings.json`
(`settings.rs`). Putting them in `config/profile.yml` would add keys the core
does not know about to a user-layer file that `validate-profile.mjs` and the
CLI both read, and would make the two clients fight over one document.

The data root can also be set there, which is how the app is pointed at the
fixture during development without an environment variable.

---

## D-8. What was rejected, and on what evidence

| Rejected | Why |
|---|---|
| Building the desktop UI on Tauri (or any webview shell) | Explicitly excluded by the Goal, and the dependency audit would fail: `wry`/`tauri` in `Cargo.lock`, WebKit in `ldd`. |
| Running the Next.js server in the background and talking HTTP to it | It would reuse the routes verbatim, but it needs `web/node_modules`, a Next build, and a listening socket; it also makes `cargo test` depend on a Node server. The engine modules are already plain ESM, so the bridge gets the same reuse with one process and no port. |
| Reimplementing the run pipeline in Rust | Forbidden by the Goal, and the parity suite exists to catch it drifting back. |
| Moving `web/src/lib/**` into a new top-level `engine/` | Violates `web/server.mjs`'s stated "imports nothing outside web/" invariant and puts the web build at risk for no gain; see D-2. |
| A separate git repository for the client | Rejected at the time: no access to the engine, outside every repo guard. Later reversed — see D-14. |
| Rendering reports by shelling out to a markdown converter | `markdown.rs` is ~350 lines, has no dependency, and renders in the same frame. A subprocess would add a binary dependency for less control. |

---

## D-9. Getting `node test-all.mjs` green: two environment failures, one real defect

The Goal requires the repo's gate green if the repo was touched, so a red run
had to be explained rather than waved at. It was three failures, of three
different kinds.

### D-9a. Two environment failures, not ours

`Did not expect a Playwright MCP warning` (×2). `doctor.mjs` takes the active
CLI from the `OPENCODE=1` marker OpenCode stamps on every process it starts. The
assertion writes a `.claude/settings.json` MCP registration and expects that to
satisfy the check — true for `claude`, not for `opencode`. Running the same two
doctor invocations by hand, from a temp cwd, with and without `OPENCODE`,
reproduces it exactly: unset, no warning. Running the gate from inside an agent
session is the trigger.

### D-9b. One real defect, in the gate's own excerpt formatter

`a failing test-runner suite is one failure with an excerpt` failed
deterministically in the full run, and **also on a pristine
`git worktree add --detach 373aba4`** with no `desktop/` present — so it
predates this work.

The first diagnosis was wrong and worth recording. The obvious suspect was
`formatRunFailure()`'s 2000-character head/tail clip. It is not that: the child
output is only 809 characters, so nothing was clipped there. The excerpt comes
from `failureExcerpt()` in `lib/failure-excerpt.mjs`, which keeps the last
twelve non-empty lines and recovers "error header" lines from above them so the
message survives the window. Its header pattern is
`SomeError: message` — and `node:test`'s **TAP** reporter does not use that
shape. TAP spells a failure as structured fields:

```
not ok 1 - d fails
  ---
  failureType: 'testCodeFailure'
  error: |-
    Expected values to be strictly equal:

    1 !== 2

  code: 'ERR_ASSERTION'
```

None of those lines matches, so the recovery pass found nothing, the window kept
the stack frames and `# fail 1`, and the report said a suite failed without ever
saying why. The gate's own self-test looks for `ERR_ASSERTION` in exactly that
output, which is how the defect surfaced.

**Fixed** in `lib/failure-excerpt.mjs`: the recovery pass now also recognises a
TAP record's own anchor lines (`not ok`, `failureType:`, `error:`, `code:`,
`expected:`, `actual:`, `operator:`, `name:`) and the body of the `error: |-`
block that carries the interpolated values. Still bounded by `maxHeaders`, so
truncation is still reported rather than silent. The excerpt now reads:

```
not ok 1 - d fails
  failureType: 'testCodeFailure'
  error: |-
    Expected values to be strictly equal:
  code: 'ERR_ASSERTION'
    Test.start (node:internal/test_runner/test:944:17)
```

`tests/failure-excerpt.test.mjs` gained two cases pinning the TAP shape,
including the negative control that the old twelve-line window dropped
`ERR_ASSERTION` from the same fixture — so the test fails against the code it
replaces rather than passing either way.

**Measured after the fix:** `env -u OPENCODE node test-all.mjs` →
`15139 passed, 0 failed`, exit 0.

## D-10. The look: one token module, and the window size is the window manager's

The first build worked and looked wrong. Two measurements explain it, and one
of them is not something an app can fix.

### The type was egui's default

Measured first: the primary display is 2560x1440 at 597x336 mm — about 109 PPI —
with a desktop scale factor of 1.0. egui's default body size is 14 points, so
text landed near 10.5 physical points, and the app set exactly two colours of
its own and inherited everything else.

So the visual system is now one module, `src/theme.rs`, and a screen never names
a colour or a size:

| | Before | Now |
|---|---|---|
| Body | 14 (egui default) | 16.5 |
| Type steps | 1 | 5: 26 / 19 / 16.5 / 15 / 13.5, plus two named styles |
| Table row | 26 | 36 |
| Colours | 2 fills set inline | 15 semantic slots, each with a light and dark value |
| Surfaces | flat | cards with a hairline edge, a 1px shadow, 10pt corners |
| Nav | text labels | a selected pill with a 3px accent bar |
| Status | raw text | a tinted pill (or a tinted combo, see below) |
| Zoom | none | Ctrl +/-/0, persisted; sidebar A-/A+ |

The palette is a developer-tool slate (`#0F172A` ground, `#1B2336` card) rather
than pure black: the tracker is read for long stretches, and a true-black ground
with pure-white text is harsher, not more legible.

**One deliberate departure from the theme this was seeded from.** Its accent was
green, and green is already this app's *success* semantic — a strong score, an
applied row. An accent meaning "clickable" that competes with one meaning "went
well" weakens both, so interactive is blue and green keeps one meaning.

**Still a ComboBox in the table.** The status control has to stay a combo rather
than become a pill: a combo's selected text reaches the accessibility tree as the
control's *value*, while a pill would be a second node named "Applied" beside the
filter tab of the same name. The screen tests refuse an ambiguous query, which is
the correct behaviour, so the tint is carried on the text instead.

**No custom font.** The repository's fonts are WOFF2 and egui cannot decode them;
system fonts differ per machine. Typographic contrast comes from egui's built-in
monospace for numbers, dates and tracker ids, which is free and identical
everywhere. A specific typeface is a separate decision with its own asset.

### The window: a window-manager fact, not a number to tune

The window opened at 1280x820, which is half of a 2560x1440 panel — the larger
half of "very small". Getting it bigger took three attempts, each measured:

1. `with_inner_size([1500,980])` — honoured, still not big.
2. Resize to 86% of the monitor once `monitor_size` is known. **Never fired**:
   `winit` reports the monitor only after the window is mapped, so a one-shot
   that cleared its own flag on frame 0 did nothing. A later `InnerSize` command
   landed at 1113x1208 points on a display that should have taken 1914x1051.
3. `with_maximized(true)` plus a `Maximized(true)` command — measured arriving as
   `maximized: false`, and in a second run the window came back quarter-tiled at
   1275x700, *smaller* than the fixed sizes.

**What is actually happening:** this machine's window manager tiles every window
to a quarter of the display. `xprop -root _NET_CLIENT_LIST` is empty, and every
launch of every size comes back 1270x685. No `inner_size` the app asks for will
change that; it is the window manager's contract, and the user's to change.

So the app does the two things it can: opens at a generous 2000x1240 for window
managers that honour it, and makes its content fit and stay legible when one
tiles it down to a quarter. That second half is testable, so it is tested:

- `the_tracker_table_fits_the_narrowest_canvas_it_is_designed_for` — the seven
  column minimums plus their gaps must fit 840 points, which is what a
  quarter-tiled window leaves after the sidebar and margins. An over-wide table
  does not error, it clips the Actions column, so the budget is asserted.
- `the_body_text_is_larger_than_egui_default` — body > 16, rows >= 34, default
  zoom >= 1.1. A silent regression to the default 14 fails.

---

## D-11. The controls are tested by being clicked, not only by being rendered

A screen can render a button wired to nothing. The first version of the screen
suite asserted only what each screen *showed*, which proves the read side and
says nothing about whether pressing anything does what it looks like it does.

Eight tests now drive the real widgets through the real harness and assert what
the app then did:

| Click | Asserted |
|---|---|
| a status tab | the filter state changed **and** the non-matching row left the table |
| the Company header | the two rows swapped vertical order (by their actual rects, not by a flag) |
| a row's Report | exactly one `report` request for **that** row, and `screen == Report(n)` |
| Start | exactly one `run` call carrying the chosen kind, input and CLI, and `run.running` |
| a run kind | the kind, the input label and the extra brief field all followed |
| the search box, then typing | the text reached the filter state and narrowed the table |
| the follow-up dialog's Save | exactly one `followups`/`log` write, with the dialog's company, channel, contact and note |
| Config's Save the profile | a `setProfile` patch of the fields shown — never a replace |

Two things worth recording about how they are written. Typing needs focus, so
the click that focuses a field is part of the interaction rather than a
workaround. And the follow-up dialog's note is set through its state rather than
typed: that screen has three text inputs, so picking one by role order would
test the picker instead of the plumbing — typing-into-a-field-reaching-state is
already proven by the search-box test, and what the dialog test adds is that its
fields reach the *write*.

---

## D-12. Three CLIs, measured — and what the measurements changed

The client is for one person on one machine, and that machine runs `pi`,
`hermes` and `opencode2`. Those three were probed directly, then a full
evaluation was attempted through the bridge for each, against a throwaway
fixture.

### `pi` — blocked by the account, not the app

```
$ pi --no-session -p "Reply with exactly: PI_OK"
401 {"type":"authentication_error","code":"api_key_expired", ...}
```

Through the app, in a few seconds:

> The CLI exited with an error — is it installed and authenticated?
> (401: {… "code":"api_key_expired", "message":"This API key has expired. Create a new API key in OpenCode Console …"})

**Verdict: the transport is fine; the key is expired.** The honesty gate did its
job — the stderr snippet reaches the screen, and the generic classifier's
`api[ -]?key` term is what surfaced it. Nothing to fix here.

### `hermes` — works, and says nothing while it works

Direct probe: `hermes chat -q … --oneshot -Q --no-restore-cwd` answered
`HERMES_OK` in seconds. A full evaluation then ran **742 s and was cut off by
the app's own 740 s budget**, having streamed **zero text events** — one status
line and one timeout notice. `-Q` buffers stdout until the run finishes, so a
CLI that is thinking hard is indistinguishable from one that has hung.

**Verdict: the wiring is right and the CLI is slower than the budget, silently.**
Neither is something the app may paper over: the budget is the core's
(`killMsForKind`), shared with the web app, and inventing a bigger one here
would be exactly the second behaviour this client exists not to have.

### `opencode2` — a separate binary the shared table did not know

`opencode2` is not a version select on `opencode`: it is its own executable
(`/usr/local/bin/opencode2`, "OpenCode 2.0 preview") with its own subcommands and
flags. `web/src/lib/clis.ts` had no entry for it, so **neither client could drive
it** — the picker did not offer it and the run would have 404'd.

Now added to the shared table (`bin: "opencode2"`, `args: (p) => ["run", p]`,
plain text like `opencode` above it, no parser needed) with a guard in
`web/tests/lib/clis-coverage.test.mjs` that names it explicitly and asserts it
did not shadow the v1 entry.

A full evaluation then produced two text events — it started reading the mode
file — and **the CLI exited on its own at 300.5 s, twice, exactly**, before
emitting a report. Not the app's budget (740 s) and not a timeout event: the
process ended.

**Verdict: driveable, but it will not finish an evaluation inside its own five
minutes.**

### What the three runs changed in the app

Two things, both from watching real output rather than imagining it:

1. **A silent run now says it is silent.** The run screen shows `elapsed 4m 12s`
   and, past 15 s without an informative event, a `no output for 2m 08s` pill.
   A keepalive deliberately does not reset that clock — it proves the transport
   is alive and says nothing about the agent. Before this, `hermes`'s twelve
   minutes and `opencode2`'s five both looked exactly like a hang.
2. **The app picks a CLI that exists.** On load, a saved id that names an
   uninstalled CLI is replaced by the engine's own `defaultCliId`
   (`pickDefaultInstalled`, shared with the web app). Previously a fresh install
   opened with Start disabled and "pick one in Config" on a machine where four
   CLIs were installed. Config now lists installed CLIs first and folds the rest
   behind `Not installed (n)`.

Tests: `a_keepalive_does_not_count_as_the_agent_speaking`,
`a_silent_run_says_so_instead_of_only_saying_running`,
`a_quiet_run_under_the_threshold_does_not_nag`,
`durations_read_the_way_a_person_would_say_them`,
`a_missing_saved_cli_is_replaced_by_one_that_is_installed`,
`an_installed_saved_cli_is_left_alone`,
`loading_the_app_selects_a_usable_cli_without_a_visit_to_config`,
`config_shows_the_installed_clis_first_and_folds_the_rest_away`.

### Note on distribution

This client is for one person and is not being distributed. The manifest
registration and `.npmignore` entry that the Goal required stay in place because
the updater's coverage guard checks them, but nothing else will be spent on
packaging, and the `web/src/lib/` carve-out exists only so an install could load
the engine — on this machine the checkout is already there.

---

## D-13. Open item: the acceptance walkthrough

The Goal's last completion item is the user's own walkthrough of the capability
matrix against their real data. That is theirs to give, so it is recorded as
outstanding rather than assumed.

**What was done to make it start from a known state.** `tests/live_data.rs`
decodes the real data root through both clients' boundary:

```
$ CAREER_OPS_REAL_ROOT_TESTS=1 cargo test --test live_data -- --nocapture
bootstrap: 15 applications, 59 pending, 15 reports, 6 PDFs, 10 states, 10 CLIs
pipeline: 15 rows, 59 inbox
reports: 15 rows decoded
followups: available=true, 2 entries, 0 due, 0 logged
config: 10 CLIs known, data root /home/…/career-ops
digest unchanged: the live root was not modified
```

Every application's report decodes, the bootstrap count and the pipeline row
count agree (the two reads are of one tracker), and the data root's digest is
identical before and after — so running the app, or that suite, moves nothing.
Opening the release build against the real root was checked the same way: same
digest, and a clean stderr.

**What remains.** Launching it and clicking through the matrix:

```bash
cargo run --release
```

Any row that does not behave as the matrix says is the next unit of work — and
the matrix row, not the code, is what gets corrected first, since the matrix is
the claim under test.

---

## D-14. The client lives in its own repository, and the engine checkout is named

**Decision.** The native client is its own repository
(`McCune1224/career-ops-desktop`), beside the career-ops checkout it drives.
The engine checkout is NAMED, never inferred:

1. `CAREER_OPS_CODE_ROOT`, when it is set.
2. A sibling checkout of the same parent directory: `../career-ops`.
3. Otherwise, one actionable sentence, and nothing else.

The resolved value is written back to the app's own settings file, so the
choice survives a restart.

**Why this replaced the old rule.** D-1 put the client inside the career-ops
checkout and inferred the checkout by walking up from the bridge until a
directory with a `web/` folder appeared. That rule was only ever a guess: it
could find a directory that merely looked like a checkout, and it could not
survive the client leaving the tree. The upstream owner asked for their tree
to be left untouched, so the client moved — and the guess had to go with it.

**Why the sibling rule.** One person's client, developed beside one checkout,
is the shape this actually has: `career-ops-desktop/` and `career-ops/` in the
same parent directory. The rule is a default, not a requirement: the
environment variable overrides it, and the app's own settings remember what was
resolved.

**One implementation of the rule.** `bridge/code-root.mjs` and
`src/code_root.rs` implement it, in two languages, and
`tests/desktop-engine-parity.test.mjs::the engine checkout is resolved the
same way in Node and in Rust` holds the two to the same variable name, the
same sibling directory name, the same resolved path and the same failure
sentence. The app resolves it and passes it to the bridge in the environment;
the bridge resolves it itself only when it is started on its own.

**What it costs.** The career-ops checkout is no longer part of the npm
package or of the updater's manifest, and `validate-system-paths-coverage.mjs`
goes back to covering the upstream tree exactly. The client is no longer
installed by the core's updater — it is one person's tool, and it is a
repository of its own.

---

## D-15. A MODE run's prompt is the client's, and only the client's

**Decision.** The client can run a career-ops MODE — an interview prep pack, a
practice round, a debrief, an offer walk-through, an application email — by
spawning the agent CLI with a prompt that names the mode file in the checkout
and tells the agent to follow it exactly. The prompt itself lives in
`bridge/mode-prompt.mjs`.

**Why the prompt is not the engine's.** `web/src/lib/run-prompts.mjs`'s
`buildPrompt()` knows five kinds: evaluate, pdf, cover, research and
fix-portal. The modes above are not among them, and the fallback branch for an
unknown kind is the EVALUATION prompt. So there was no engine prompt to call.

The alternative was to add a branch to `buildPrompt()`, and that was refused:
the career-ops checkout is upstream's, and the whole point of D-14 is that it
is left byte-for-byte alone. Writing to it to serve this client would put the
client's needs back into the engine.

**What the prompt does and does not contain.** It says which mode file to
read, that the run is unattended, that the agent must not write files, and the
core's own output-language rule (`profile-language.mjs`'s
`outputLanguageInstruction()`, the same one the CLI and the web app inject).
It contains no statement of what the mode does — the mode file is the rule,
and the prompt only names it. Nothing about a mode can drift, because nothing
about a mode is restated.

**Everything else is the engine's, unchanged.** The CLI is resolved by
`resolveCliOrFallback()`, the capabilities come from `capabilitiesFor()` (an
unknown kind resolves to the least-capable record), the argv is built by
`claude-invocation.mjs` and `run-cli-support.mjs`, the fence is
`cli-fencing.mjs`'s, the spawn is `spawnHeadlessCli()`, the stream is parsed by
`run-cli-support.mjs`, and the time limit is `killMsForKind()`'s shared
default. The run budget was not raised, and no kind was given a longer one.

---

## D-16. Ten screens, ten shortcuts, and the maintenance tools in Config

**Decision.** The sidebar has ten entries — Today, Pipeline, Run, CV,
Follow-ups, Documents, Portals, Interview, Outcomes, Config — and Ctrl 1..9
plus Ctrl 0 reach every one of them.

**Why the number is ten.** A screen the keyboard cannot reach is a screen the
keyboard test cannot prove. The nav grew as the capabilities grew, and the
shortcut scheme was made to match the nav rather than the other way round:
ten entries, ten shortcuts, `tests/screens.rs::ctrl_number_keys_reach_every_screen`
walks the nav and presses the key for each entry.

**Why maintenance lives in Config.** The core's eight repair tools are upkeep,
and a screen of their own would be one more place to look for a button that
only matters when something has already gone wrong. They sit in Config next to
the plugins and the profile check, and each one proposes before it acts: the
first click shows what the script would do, the second applies it.

---

## D-17. Every remaining capability is a bridge op over the core, not a screen of its own

**Decision.** The capabilities M3 to M5 added — the quiet companies, the
recruiting friction, the salary gap, the negotiation points, the story bank,
the digest, the portals audit and its coverage check, what is new, the
re-listed roles, the agent inbox, the plugin list, the phonebook, the invite
matcher, the outcome archive, the document list — are each a bridge op that
runs the CORE's own script or calls the WEB app's own module, plus a small
surface on an existing screen.

**Why not a screen each.** A capability that is a question with an answer does
not need a screen; it needs a button and a sentence. `views/maintenance.rs`
exists as a module of tools and no longer as a screen: the eight repair tools
sit in Config, next to the plugins and the profile check, because a screen of
its own would be one more place to look for a button that only matters when
something has already gone wrong. The nav stayed at ten entries (D-16) for the
same reason: a nav that grows with every capability stops being a map.

**What each op does.** It names ONE core script or ONE engine module, runs it
with the flags the script documents, and returns the script's own words. The
client adds no rule. Where the core has a JSON mode the op returns it; where it
prints prose the op returns the prose and the screen shows it verbatim.

**The one place a prompt is the client's** is D-15, and it is still one place.

## D-18. What a row's acceptance check is

**Decision.** A `done` row's Acceptance cell names the test that proves it, in
backticks. `tests/acceptance.test.mjs` reads `FEATURES.md` and fails when a row
outside the deferred epic is `missing` or `partial`, or when a row names a check
this repository does not contain.

**Why a test name rather than prose.** Prose acceptance cannot be run. A row
that says "the card shows this company's history" is an opinion until something
executes; a row that names `a company history joins the tracker, the follow-ups
and the status log` is a claim `node --test` settles. The gate is what makes
"the inventory is not lying" checkable rather than asserted.

**What the gate does not prove.** That the check is a GOOD check. A test named
in a row runs, and it asserts what its name says; whether the name is the right
question is a review question, and the walkthrough (D-13) is where it gets
answered against real data.

## D-19. The three CLI measurements stay recorded, not fixed

The three CLI limits are unchanged and unfixed, as the Goal requires:

- `pi` is blocked by an expired API key. Nothing to fix in the app.
- `hermes` buffers its output and a full evaluation exceeded the shared 740 s
  budget, silently.
- `opencode2` exits on its own at 300 s, before emitting a report.

`killMsForKind()` was not raised and the shared run budget was not extended.
The bar for a CLI is *driveable and honest*, not *guaranteed to finish*, and
every acceptance check added here is provable without a completed model
response: the stub-CLI pattern in `tests/desktop-run-e2e.test.mjs` speaks the
real protocol, and the rest of the suite asserts against the files the core
wrote. A limit that belongs to the CLI or to the account is recorded in
`README.md` and left where it is.

---

## D-20. The tailored CV's ORDER is checked before the render; its DESCRIPTORS are checked on the payload the client would send

**Decision.** A `pdf` run checks the HTML the agent produced with the engine's
own `cv-experience-order.mjs` **before** the PDF is rendered. A CV whose roles
run oldest-first is reported in the run log and the render is refused; the
tracker's PDF column is untouched. Separately, the CV screen runs the engine's
`verify-cv-structure.mjs` over the payload the client builds from `cv.md`, so a
dropped descriptor or a moved entry in what the client would send is reported.

**Why the descriptor check cannot run on the tailored CV.** `verify-cv-structure.mjs`
takes the JSON payload `modes/pdf.md` step 17 builds. The web's `pdf` kind never
builds one: its prompt has the agent fill the template and emit HTML, and the
agent holds no write tool, so no payload exists at any point the client can
read. Closing that half needs one of two changes, and both are outside this
client's remit:

1. the engine's `pdf` kind emits the step-17 payload inside the CV envelope, or
2. `web/src/lib/run-prompts.mjs` gains a branch that asks for it.

Both are writes to the career-ops checkout, which D-14 forbids. So the order
half is done at the right moment, the descriptor half is done on the payload
the client owns, and the limitation is written here rather than papered over.

**What would unblock the descriptor half.** Either change above, as a
`kind:feature` decision in the career-ops repository. Until then the row's
acceptance names both checks and says which one covers which half.

---

## D-21. A capability that is a question with an answer needs a button, not a screen

**Decision.** The audit that closed M1 to M5 found three capabilities whose
surface was a read where the row asked for a write: the outcome form previewed
and never recorded, the personalization "editor" only listed the sections, and
the plugin install never confirmed. Each was replaced with the two-step shape
the maintenance tools already use: a proposal, then an action.

**Why the two steps.** Every one of them changes a user-layer file: the outcome
archives a posting and moves a tracker row, a personalization save splices the
user's own profile, and a plugin install clones a third-party repository. The
user sees what the core would do before it does it, and the second click is
theirs.

**What the audit also settled.** `plugins install` passes `--confirm` only when
the caller confirms, and reports `installed: false` for a preview, so a preview
cannot be mistaken for an install. `cv-experience-order.mjs` is imported as a
FUNCTION rather than spawned: the run path must not shell out to a core script
(the persistence guard in `tests/desktop-engine-parity.test.mjs`), and the
guard is a pure function over the HTML the agent produced.

---

## D-22. A PDF page is rasterised by poppler and painted as a texture

**Decision.** The client previews a PDF page in the window. The page is
rasterised by poppler's `pdftoppm` — the same poppler the career-ops core
already depends on for `pdftotext` — and painted into egui as a texture. D-6's
"hand the file to your viewer" is still offered beside it, on the same row, and
is still the answer for anything past "does this page look right".

**Why this does not break X-6.** X-6 is about a *webview*: no embedded browser,
no WebKit, no CEF, no HTML rendering engine. A subprocess that turns a PDF into
pixels, and a texture egui paints, is neither. `tests/no_webview.rs` still
passes on the lockfile and on `ldd`, and no dependency was added at all.

**The format is PPM, and that is the point.** `pdftoppm` writes `P6
<w> <h>
255
`
and then three bytes per pixel, so decoding it is thirty lines in `src/pdf.rs`
and needs no crate. A PNG or a JPEG preview would have pulled in a decoder for
a picture nobody zooms into.

**What the client owns, and what the tool owns.** The rasteriser owns the
pixels: its scale differs between poppler builds (`-r 72` gave 2x on the build
this was written against), so the tests assert the page's shape and its CONTENT,
not a pixel count. The client owns the decode, the failure sentences and the
layout.

**Three failures, three sentences.** `pdftoppm` missing says so and points at
the user's own viewer; a PDF poppler refuses says what poppler said; a PPM whose
header lies says it is unreadable. "No preview" with no reason is the failure
mode a user cannot act on, so it is not one this module can produce.

**Two bugs the tests found.** The scratch directory was keyed on process and
page, so two concurrent previews (and two tests) deleted each other's files —
it is keyed on a per-call counter now. And the card named a token text style
without installing it, which is fine inside `views::frame` and a panic outside
it; `theme::adopt` is now the one place that does the four lines, and both the
frame and a screen drawn on its own use it.
