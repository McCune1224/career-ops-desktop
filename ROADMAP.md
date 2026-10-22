# Roadmap: from inventory to a working app

Companion to [`FEATURES.md`](FEATURES.md). That file says what exists. This one
says what to do first, how to cut it into issues, and how a column gets to Done.

Rules of engagement, in order of authority:

1. **`FEATURES.md` is the list.** New work gets a row there before it gets an
   issue. Work with no row is either a bug or a new feature, and both are
   decisions rather than tasks.
2. **Function before polish.** `E17` (GUI rework) is deferred on purpose. It is
   recorded so it is not lost, not so it gets started.
3. **Never duplicate the engine.** A row that would reimplement a core rule is
   wrong. Either call the shared script, or the row becomes an extraction with
   its own test. See `DECISIONS.md` D-2.
4. **One writer per file.** `DECISIONS.md` D-4 and invariant X-1.

---

## Labels

| Label | Meaning |
|---|---|
| `epic:E01` … `epic:E17` | Which epic. One per issue, exactly. |
| `area:desktop` | The Rust client. |
| `area:bridge` | The Node bridge. |
| `area:engine` | A shared module or a root script. Changes both clients. |
| `area:web` | The Next.js app. |
| `area:docs` | Documentation only. |
| `kind:feature` | New capability. |
| `kind:parity` | Bringing the desktop client up to an existing capability. |
| `kind:bug` | It exists and it is wrong. |
| `kind:chore` | Maintenance, refactor, test. |
| `kind:spike` | Time-boxed question, one day, outcome is a decision. |
| `needs:fixture` | Must be developed against `CAREER_OPS_ROOT`, never real data. |
| `risk:writes` | The issue can write user data. Needs the canonical-writer check. |
| `risk:spend` | The issue can spend model tokens. Needs a cost estimate first. |
| `blocked` | Cannot start. The body must name the blocker. |

Sizes are on the issue body, not as labels. `S`, `M`, `L` as defined in
`FEATURES.md`.

---

## Kanban columns

Six columns, with WIP limits. The limits matter more than the columns: they are
what stops six half-finished epics from existing at once.

| Column | WIP | What may sit here |
|---|---|---|
| **Backlog** | none | Every row from `FEATURES.md` that is not yet ready. Unsorted is fine. |
| **Ready** | 8 | Has an ID, an acceptance check, a size, and no open dependency. This is the only column you pull from. |
| **Doing** | 2 | One person, one issue, one branch. A second card only if it is genuinely unblocked by the first. |
| **Verify** | 3 | Code written, acceptance check written, and the check is run. Waiting for review or for the user's walkthrough. |
| **Done** | none | Acceptance check green in `cargo test` or `node --test`, status column in `FEATURES.md` updated, `DECISIONS.md` entry if a choice was made. |
| **Blocked** | none | Has a `blocked` label and a one-sentence reason naming what unblocks it. |

Two rules that keep the board honest:

- **A card returns to Ready, not to Doing,** when a review finds a problem. Doing
  is a state about attention, not about correctness.
- **A card leaves Verify only with its check green.** "It looked right when I ran
  it once" is a note, not a Done.

---

## Definition of Done

Every issue, regardless of kind:

- [ ] Acceptance check from `FEATURES.md` exists and passes.
- [ ] `node dev/verify.mjs` green if this repository was touched (it runs
      `cargo test`, every Node suite and the dependency audit); `node test-all.mjs`
      green if the career-ops checkout was touched. Both, if both.
- [ ] No new dependency in `Cargo.lock` that names a webview crate
      (`tests/no_webview.rs`).
- [ ] `FEATURES.md` status column updated in the same commit.
- [ ] Written choice recorded in `DECISIONS.md` if the issue settled one.
- [ ] Developed against a fixture (`CAREER_OPS_ROOT`), never against real data.

Additionally, for `risk:writes`:

- [ ] The write goes through a canonical script or the shared atomic-write helper.
- [ ] A test asserts the file's bytes, not just the call.

Additionally, for `risk:spend`:

- [ ] The user is told the estimate before the tokens are spent.

---

## Milestones

Ordered by dependency and by what makes the app usable soonest. Each milestone
is a shippable state, not a phase of a project plan.

### M1. Trustworthy daily loop

The app is already close. This is the milestone that closes the gap between "it
works" and "I use it every day".

Includes: E00-S02, E01-S05, E01-S07, E01-S09, E02-S05, E03-S05, E03-S07,
E03-S10, E06-S02, E06-S03, E06-S05, E06-S06, E12-S01, E12-S02, E16-S07, E16-S08.

Why these: bulk status (you change several rows after a scan), inbox skip and row
delete (the tracker has to be maintainable from the app), the folded report body
(a report you cannot scan is a report you avoid), the substitution notice (a
silent CLI swap is a wrong result), the cost estimate (never surprise yourself),
the run log (so a run that failed can be read back), the follow-up pin and
delete, the health check, and keyboard navigation.

Exit criterion: a week of use with no CLI session needed for tracker hygiene.

### M2. CV and cover parity

Tailoring and letters are the two things that pay for the tool.

Includes: E04-S03, E04-S05, E04-S06, E04-S11, E05-S02, E02-S06, E02-S07,
E02-S10, E14-S01, E14-S03, E12-S06, E12-S07.

Why: the `pdf` run already produces a CV. What is missing is the layer that makes
the result *trustworthy*: facts verified against sources, structure preserved,
the template you chose honoured, and the keyword gap visible. A tailored CV you
cannot check is worse than no CV.

Exit criterion: tailor a CV, verify it in the app, see the keyword gap, open the
letter, and find all three in Documents.

### M3. Interview and outcome

The second half of the loop, and the part that only matters when things go well.

Includes: E10-S01, E10-S02, E10-S06, E10-S07, E06-S09, E06-S10, E06-S11,
E13-S04, E06-S07, E10-S04.

Why: prep packs and the story bank are the highest-value missing features for
anyone actually getting interviews. The provenance rule in E10-S06 is why this is
not just another text generator, and it must not be shortcut.

Exit criterion: an interview happens, and the app carries it from prep to
debrief without a CLI.

### M4. Supply side

Discovery is where the app currently stops, and it is deliberately last because
the CLI does it well and the native path is expensive to get right.

Includes: E07-S07, E07-S08, E07-S09, E07-S12, E07-S14, E01-S08, E07-S11,
E07-S13, E07-S10.

Why in this order: verification and configuration first (cheap, high value),
then liveness and "new this week", then the batch shortlist, then discovery. Each
step is useful alone.

Exit criterion: a scan's results are reviewable and skippable in the app.

### M5. Everything else, then the rework

E08, E09, E11, E13, E14, E15 stay as they are unless a specific need appears.
E17 starts only after M4, and only with a written design brief. A GUI rework
without a functional app is a nicer way to be stuck.

---

## Dependency spine

The rows most likely to block others, in order:

```
E00-S01 doctor ─────────────┐
E00-S08 CLI detection ──────┤
E03-S01 spawn + fencing ────┼──> every AI-backed epic (E02, E04, E05, E09, E10)
E01-S01 tracker read ───────┘
E03-S02 stream ──> E03-S04 silence, E03-S07 cost, E03-S10 run log
E01-S03 status write ──> E01-S05 bulk, E06-S06 seed, E12-S02 normalise
E02-S02 persist ──> E02-S03 read report ──> E02-S05..S09, E05-S03, E08-S05, E10-S01
E04-S02 pdf run ──> E04-S05 verify, E04-S06 structure, E14-S01 documents
E06-S01 cadence ──> E06-S02 log, E06-S03 pin, E13-S04 sweep
E07-S08 portals ──> E07-S09 coverage, E07-S10 discover
```

Anything not on this spine is a leaf. Leaves are good first issues: they unblock
nothing, so a wrong estimate costs nothing.

---

## Issue templates

### Epic issue

```
Title: [Epic E04] CV: viewing, tailoring, verification
Labels: epic:E04, kind:feature
Body:
  Objective: <one sentence, the outcome for the user>
  Rows: E04-S01 … E04-S13 (link FEATURES.md section)
  Currently done: <list of IDs>
  Not started: <list of IDs>
  Exit criterion: <the sentence from ROADMAP.md>
  Invariants in play: X-1, X-2, X-5, X-7
```

Epic issues stay open until every in-scope row is `done` or `gap`. They are a
view, not a container: do not put story issues inside them.

### Story issue (the default)

```
Title: [E02-S05] Fold a report's blocks behind the verdict
Labels: epic:E02, area:desktop, kind:parity, size:S
Body:
  Capability: <from FEATURES.md>
  Today: <what happens now, with a screenshot or the file:line>
  Change: <what the app does after>
  Acceptance: <the check from FEATURES.md, verbatim>
  Fixture: CAREER_OPS_ROOT=<path>  (needs:fixture if it writes)
  Invariants: <which X-n apply, or "none">
  Out of scope: <what this issue deliberately does not do>
```

The "Out of scope" line is not optional. It is what stops a `S` issue becoming
`L` in review.

### Spike issue

```
Title: [E07-S10 spike] Can a careers URL be resolved to an ATS board reliably?
Labels: epic:E07, kind:spike, size:S
Body:
  Question: <one question>
  Today: <what the script does and where it fails>
  Time box: one day
  Outcome: a decision recorded in DECISIONS.md, plus either a story issue or a
           "do not build" note. No production code.
```

---

## Cutting 152 rows into issues

Do not create 152 issues. The board is a working surface, and a hundred-card
Backlog is a wall.

1. **Create the 18 epic issues.** They are the index.
2. **Create issues only for the current milestone.** M1 is 16 rows, which is a
   reasonable first batch.
3. **Merge siblings under `S`.** `E02-S04` and `E02-S05` are one afternoon of
   work on one screen; they can be one issue that closes two rows. Record both
   IDs in the title when you merge: `[E02-S04+E02-S05]`.
4. **Split any row that grew.** If `E04-S05` turns out to need a schema, that is
   a second issue with a new ID appended as `E04-S05a` rather than a renumber.
5. **Leave `gap` rows out.** They are boundary markers, not backlog. If one ever
   becomes wanted, it gets a `kind:spike` first.

---

## Working the board

A session looks like this:

1. Pull one card from **Ready**, highest in the milestone first.
2. Read its `FEATURES.md` row. If the row no longer matches the code, fix the row
   first; the row is the claim under test.
3. Write the acceptance check before the code when the check is cheap. For a
   screen, that is an `egui_kittest` test. For a bridge op, a `node:test` against
   a fixture. For a rule, a unit test in the shared module.
4. Implement, then run the check. Run `cargo test` and, if the repo was touched,
   `node test-all.mjs`.
5. Move to **Verify** with the check's output in the card. Do not write "works"
   without it.
6. On Done: status column updated, decision recorded, card closed with the check
   that proves it.

When the milestone's rows are all Done, walk `CAPABILITY-MATRIX.md` against real
data before starting the next milestone. A milestone that passes its tests but
does not survive the walkthrough is not done, and the matrix row is what gets
corrected first.

---

## What not to build

Recorded because the temptation is real and each of these has been considered
once already:

| Temptation | Why not |
|---|---|
| A second CLI table in this client | `web/src/lib/clis.ts` is the shared table. A copy diverges silently. |
| A desktop-only progress format | The NDJSON vocabulary is the web app's; the desktop client renders it, it does not invent it. |
| Rendering PDFs in-process | That is a second renderer whose output is not the file you would send. See `DECISIONS.md` D-6. |
| A bigger run budget for slower CLIs | `killMsForKind` is shared. Raising it here changes the web app's behaviour too, silently. Test the CLI, not the budget. |
| A local cache of tracker rows | `data/applications.md` is the source of truth. A cache is a second one. |
| Webview-based "rich" panels | Invariant X-6, and the reason this client exists. |
