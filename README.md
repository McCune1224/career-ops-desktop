# career-ops desktop

A native Linux desktop client for
[career-ops](https://github.com/career-ops-hq/career-ops): the daily loop
— your tracker, your reports, your runs, your CV, your follow-ups — in an egui
window, on top of the same Node engine the web UI and the CLI use.

**No webview.** No Electron, no Tauri, no wry, no embedded browser, no WebKit,
no CEF. The window is egui painting through OpenGL; the engine is a Node child
process this app starts and owns.

```bash
cargo run --release
```

Needs `cargo` (1.85+) and `node` (22.13+, the same floor the core has). Nothing
else: no `npm install`, no Next build, no browser.

---

## What it is, in one picture

```
┌───────────────────────────┐      stdio, NDJSON       ┌──────────────────────────┐
│  career-ops-desktop       │  ◀────────────────────▶  │  bridge/                 │
│  Rust + egui + eframe     │   requests + events      │  Node, no listening port │
└───────────────────────────┘                          └───────────┬──────────────┘
                                                                   │ imports
                                                                   ▼
                                                       ┌──────────────────────────┐
                                                       │  web/src/lib/**          │
                                                       │  the web app's own engine │
                                                       └───────────┬──────────────┘
                                                                   │ spawns
                                                                   ▼
                                             set-status.mjs · merge-tracker.mjs ·
                                             reserve-report-num.mjs · generate-pdf.mjs
                                             + the AI CLI the core already drives
```

There is one implementation of every career-ops rule, and it lives in the
career-ops checkout, not here. This client is not it. See
[DECISIONS.md](DECISIONS.md) D-2 for how that is enforced rather than promised.

---

## The look

One visual system, in `src/theme.rs`: a 26/19/16.5/15/13.5 type scale, a
4/8/12/16/24 spacing rhythm, 36-point table rows, and fifteen semantic colour
slots with a light and a dark value. A screen names `palette().good`, never a
green.

**It is adjustable, because a fixed scale cannot know your display.** `Ctrl +`
and `Ctrl -` zoom, `Ctrl 0` resets to 115%, and the sidebar's `A-` / `A+` do the
same with the percentage shown. The choice is saved next to the theme, in
`~/.config/career-ops-desktop/settings.json`. `Light mode` / `Dark mode` in the
top bar switches the theme.

**Your three CLIs.** `pi`, `hermes` and `opencode2` are all driveable. Measured
on this machine: `hermes` works and buffers its output (so a long run is silent —
the run screen now names the silence and counts the elapsed time); `opencode2` is
OpenCode 2.0 preview, a separate binary that was missing from the shared CLI
table and is now in it; `pi` is blocked by an expired API key, which the app
reports in seconds with the provider's own message. A full evaluation is a
12-minute job for a slow CLI and the core's budget is 740 s — see
`DECISIONS.md` D-12 for the numbers.

If the window opens smaller than you want, that is your window manager: this one
tiles every window to a quarter of the display, and no size the app requests
changes it. Toggle the window to floating, or maximize it, and the layout follows
(see `DECISIONS.md` D-10).

## The screens

| Screen | What it does |
|---|---|
| **Today** | Tracker and inbox counts, what is due, setup state, a way into a run. |
| **Pipeline** | Every tracker row, sortable by #/company/role/score/status/date, filterable by canonical status, by a score floor and by free text, with an inline status control that writes through `set-status.mjs`. Report and Posting actions per row. |
| **Report** | The evaluation in full — score, legitimacy, verdict, the rendered markdown body — plus "Open the source posting", the tailored CV and cover letter, and an in-window **PDF preview** of either. |
| **Run** | Evaluate a pasted JD or URL, tailor the CV, or draft a cover letter, with the web app's streamed progress (text, tool calls, status lines, errors, the saved report number). |
| **CV** | Edit `cv.md` with a live preview; save atomically with a backup. |
| **Follow-ups** | What the core's cadence calculator says is due, the logged history, and a dialog that appends to `data/follow-ups.md` under the same lock the web route uses. |
| **Documents** | Every generated CV and cover letter, and the posting archive that keeps them with the report they belong to. |
| **Portals** | What the last scan found, the watched boards and their audit, why a posting you found by hand was missed, and a search for a company's board. |
| **Interview** | The story bank with its provenance verdict, the match of a story to a question, and the week's gaps. |
| **Outcomes** | What happened to an application, who is still silent, the salary gap, the phonebook and the warm-introduction lookup, and the one thing a hire may share. |
| **Config** | Choose the AI CLI, tune the follow-up cadence, edit the candidate details in `config/profile.yml`, check the profile, list the plugins, run the maintenance tools, read the language and market set, queue a request for later, and see which data root and checkout the app is reading. |

[CAPABILITY-MATRIX.md](CAPABILITY-MATRIX.md) maps every one of those to the web
route it replaces, the desktop surface that implements it, and the test that
proves it.

Two more documents turn the whole product, not just this client, into a backlog:

- **[FEATURES.md](FEATURES.md)** — every capability career-ops has, 152 rows
  across 18 epics, each with its evidence, its status in this client, and an
  acceptance check. This is the list issues are cut from.
- **[ROADMAP.md](ROADMAP.md)** — what to do first, kanban columns and WIP
  limits, the definition of done, and issue templates.

---

## Named gaps

Deliberately not built. Each needs a browser, a scheduler or an OS-level
credential store — things this client refuses to embed. The CLI remains the place
to do them, and `FEATURES.md` records every one as `gap` with the reason.

- **Apply form prefill** — `/apply`, `/api/apply/*`. It drives a real browser and
  fills a real form.
- **Agent chat** — the assistant console, `/api/assistant`, chat history.
- **Scanning and explore** — `/explore`, `/api/explore*`, the scanners. The
  supply loop is the CLI's; this client shows what a scan already found.
- **CV studio** — `/api/cv-studio/[n]`. Per-block CV editing is a deferred
  surface (E04-S10).
- **Scheduled scans** — `/scheduled-scans`, `/api/scheduled-jobs*`. It installs
  OS-level schedulers.
- **Analytics** — `/analytics`, `/api/stats`, `/api/patterns`, the Sankey.

Everything else the web app offers is here: the tracker and its repairs, the
report with its checks, the CV and its template, cover letters with a tone,
follow-ups, the documents, the watched boards and their audit, the interview
half, outcomes and outreach, and the core's own maintenance tools.

---

## What it will never do

- **Never submits an application.** There is no code path that fills or sends an
  application form.
- **Never moves data off the machine**, beyond the AI CLI calls the core already
  makes. The bridge is a child process over pipes; it listens on nothing.
- **Never writes a second source of truth.** Every tracker or data write goes
  through `set-status.mjs`, `merge-tracker.mjs`, `reserve-report-num.mjs`,
  `persist-evaluation-report.mjs`, `generate-pdf.mjs` or the same
  atomic-write-plus-backup helper the web routes use. See DECISIONS.md D-4.
- **Never reimplements the engine.** The parity suite fails if it starts to.
- **Never embeds a browser.** A PDF page is rasterised by poppler into pixels
  and painted by egui; there is no HTML engine anywhere in the tree
  (DECISIONS.md D-22, and `tests/no_webview.rs`).

---

## Working on it

```bash
node dev/verify.mjs           # THE gate: cargo test, every Node suite, the audit
node dev/verify.mjs --fast    # the Node side and the audit, skipping cargo
cargo run --release           # the app
```

`dev/verify.mjs` is the gate rather than a list of commands in this file,
because the list had a hole: it documented `node --test tests/desktop-*.test.mjs`,
which does not match `tests/acceptance.test.mjs` — the one check that proves
`FEATURES.md` is not lying about what works. The script globs the directory, so
a new suite is in the gate the moment it exists.

Each piece on its own, when you want just one:

```bash
cargo test                              # the Rust side
node --test tests/desktop-*.test.mjs    # the bridge and the engine parity
node --test tests/acceptance.test.mjs   # FEATURES.md against the code
node dev/dependency-audit.mjs           # lockfile + ldd, with the library list
```

Before walking the capability matrix against your own data, check that the
client's types match what your engine actually sends. This is read-only and
asserts your data root is byte-identical afterwards:

```bash
CAREER_OPS_REAL_ROOT_TESTS=1 cargo test --test live_data -- --nocapture
```

### Developing against a fixture, not your own data

The app reads and writes the career-ops data root, so development must not point
it at real data:

```bash
node dev/make-fixture.mjs /tmp/career-ops-fixture
CAREER_OPS_ROOT=/tmp/career-ops-fixture cargo run --release
```

`make-fixture.mjs` **copies** your user files (so a write cannot escape) and
**symlinks** the core's scripts (so a script edit is visible with no re-copy).
You can also set the data root in `~/.config/career-ops-desktop/settings.json`,
which is where the app keeps its own preferences — not in `config/profile.yml`
(DECISIONS.md D-7).

### How the screens are tested

`tests/screens.rs` renders one screen at a time inside an `egui_kittest` harness
and asserts on the widgets it produced — and, for six of them, clicks the control
and asserts what the app then did, because a screen can render a button wired to
nothing and only a click can tell the difference. No window, no display server, no bridge
process, no data root: the engine is a `FakeBackend` answering with the exact
JSON shapes the bridge sends. If a screen stops showing what the web app shows
for the same capability, one of those tests fails.

`tests/desktop-bridge-ops.test.mjs` is the other half: it starts the real bridge
against a real fixture and drives it over its real protocol. Every suite, and
what each one is for, is listed in [CAPABILITY-MATRIX.md](CAPABILITY-MATRIX.md).

### How a PDF gets into the window

`src/pdf.rs` rasterises a page with poppler's `pdftoppm` and reads the PPM it
writes; `views/pdfview.rs` paints it as a texture. No webview, no PDF crate, and
no new system dependency — poppler is what the core already needs for
`pdftotext` (DECISIONS.md D-23).

---

## Layout

```
career-ops-desktop/
├── Cargo.toml
├── src/
│   ├── main.rs          the binary: opens the window
│   ├── lib.rs           the library
│   ├── shell.rs         the frame, the sidebar, the screen router
│   ├── app.rs           app state, and every action a screen can take
│   ├── bridge.rs        the stdio client for the Node bridge
│   ├── backend.rs       the engine trait (+ the fake the tests use)
│   ├── model.rs         the bridge's JSON, typed
│   ├── markdown.rs      reports and CVs → egui
│   ├── settings.rs      the app's own preferences
│   └── views/           one module per screen
├── bridge/
│   ├── server.mjs       the stdio loop
│   ├── dispatch.mjs     the op table
│   ├── engine.mjs       THE import surface onto web/src/lib
│   ├── alias-hook.mjs   the web app's "@/…" alias, for plain Node
│   └── ops/             one module per capability area
├── tests/               headless screen tests, and the Node suites
│   ├── screens.rs       one screen at a time, in a kittest harness
│   ├── payloads.rs      can the client decode what the engine sends
│   ├── no_webview.rs    the "no webview" claim, in the lockfile and in ldd
│   ├── live_data.rs     the real data root, read-only, opt-in
│   ├── desktop-engine-parity.test.mjs
│   ├── desktop-bridge-ops.test.mjs
│   ├── desktop-run-e2e.test.mjs
│   └── helpers/         the fixture builder the Node suites share
└── dev/                 fixture builder, dependency audit
```
