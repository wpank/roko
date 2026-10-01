# work/ — roko's task tracker

Every piece of open work on roko (bugs, gaps, regressions, findings, specs, decisions) is one markdown
file in `work/items/`. Each file says what is wrong, why it matters, where the code is, and how to prove it
is fixed. Any agent or person can start here with no other context, pick a task, do it, and close it.
Several agents can do this at the same time.

This folder replaces `tmp/backlog/` and `.roko/GAPS.md`, which are frozen. Their history is in `work/history/`.
The tool is `python3 tools/work.py` (run it from the repo root; `--help` lists every command).

## For agents: picking up and doing a task

1. **Get the big picture.** `work/goals.toml` lists the active goals, most important first. `work/NOW.md`
   shows the top open items of each goal. `work/DRIFT.md` lists items whose recorded state may be out of date.
2. **Pick.** Run `python3 tools/work.py next`. It returns the highest-priority open item that nobody has
   claimed, that is not on hold, whose dependencies are closed, and whose files do not overlap work already
   in progress. `next --n 4` returns four items that can run side by side; `--goal core` limits it to one goal;
   `--max-size M` skips large items.
3. **Claim it** before touching code:
   `python3 tools/work.py claim <id> --by "<who you are>" --branch work/<id>`.
   A claim is a small file in the main checkout's `.roko/work-claims/`. It is shared by every worktree and never
   committed. Claims older than 24 hours count as stale. `tools/work.py claims` lists them.
4. **Work in your own worktree and branch**, never in the main checkout:
   `git worktree add ../roko-work-<id> -b work/<id>` (from the current working branch). If you were started in
   a worktree already, `git switch -c work/<id>` there. Rust: `export CARGO_TARGET_DIR=<main checkout>/target`
   to reuse the warm build cache (builds queue on cargo's lock, so run at most 4-5 Rust workers at once).
   Portal (`apps/portal`): symlink `node_modules` from the main checkout instead of installing.
5. **Read the item fully.** If its body lacks the sections of the template below, or is too thin to act on,
   rewrite it first from the code and the documents named in `source` / `discovered_from`, keeping the old
   text under a final `## Original notes` heading. A clear item is part of the work.
6. **Do the work.** Keep to the item. If you find another problem, file it as a new item
   (`tools/work.py new …`) instead of fixing it on the side.
7. **Done means the item's `[[verify]]` command passes.** If the item has none, write one first: it must fail
   before your change and pass after it (see "Verify commands").
8. **Commit on your branch** with `Closes: <id>` as the last line of the message. Then record the closure and
   commit the item file:
   `python3 tools/work.py close <id> --commit HEAD --evidence "what changed, where, and which check proves it"`.
   Do not run `render` or commit the generated views (`NOW.md`, `STATUS.md`, …) on your branch: every branch
   would conflict on them. They are regenerated in the main checkout after merging. Your claim stays until the
   closed item is merged.
9. **Merge.** In batch mode the orchestrator merges your branch (below). Working alone, merge it yourself the
   same way.

## Parallel work

- **Batch:** one session runs `/work-batch N` (a Claude Code skill in `.claude/skills/`). It picks `next --n N`,
  claims the items, gives each to an agent in its own worktree, and merges the branches that pass.
- **Independent sessions:** any number of sessions run `/work-next`. Each claims one item and does steps 3-9.
- **No collisions:** `next` never hands out an item that is claimed, and never two items whose anchors point at
  the same file. Items with no file anchors have an unknown footprint, so `next` flags them.
- **Git rules for workers and the orchestrator** (decided by Will, 2026-09-29):
  - Workers commit only on their own branch `work/<id>`, in their own worktree.
  - The orchestrator merges passing branches into the current working branch of the main checkout with
    `git merge --no-ff work/<id>`, without asking each time. It never merges into `main` and never pushes.
  - Several sessions merge and commit in the main checkout, so every merge or commit there runs under the shared
    lock, after checking that no merge is in progress, and stages explicit paths only (never `git add -A` or
    `git add work/`):
    `lockf -k -t 900 "$(git rev-parse --git-common-dir)/roko-merge.lock" bash -c 'test ! -e "$(git rev-parse --git-path MERGE_HEAD)" && git merge --no-ff work/<id> -m "…"'`
    (macOS `lockf`; on Linux use `flock -w 900` with the same lock file). Workers committing in their own
    worktree do not need the lock.
  - After each merge, re-run the item's `[[verify]]` on the merged tree. If it fails, stop merging and report.
  - The main checkout may have other sessions' uncommitted changes. Never stash, reset, restore, force or check
    out anything there. If git refuses a merge because of local changes or a conflict, run `git merge --abort`
    (only for a conflict you caused) and report the branch as not merged.
  - Do not delete worktrees or branches after merging. Will removes them.
- **Checks:** a worker's branch counts as done when the item's `[[verify]]` passes (Will's choice, 2026-09-29).
  The full `cargo fmt` / `clippy` / `cargo test --workspace` suite from CLAUDE.md is still required before
  anything is pushed.

## Big picture and priority

- `work/goals.toml` (hand-edited): the active goals, highest priority first. An item joins a goal with
  `goal = "<key>"`; items without a goal are "later".
- Within a goal, items are ordered by `rank` (optional, lower first), then severity (p0 first).
- `hold = "reason"` takes an item out of `next` and NOW.md until someone removes the line. Use it for
  "not now" decisions.
- `size = "S" | "M" | "L"` (optional): S is under an hour, M about a day, L several days.
  Batches prefer items that fit.
- Decisions and questions (`kind = "decision" | "question"`) need Will. `next` skips them; they are listed in
  `DECISIONS.md`.

## Layout

```
work/
  README.md                 this file
  goals.toml                hand-edited: active goals, highest priority first
  items/<id>-<slug>.md      one file per item (tracked, public)
  parked/<id>-<slug>.md     parked items: not planned, kept for search (see "Parking")
  NOW.md                    GENERATED — what to work on next: the top items of each goal
  STATUS.md                 GENERATED — open items by subsystem, recently closed
  DRIFT.md                  GENERATED — open items whose recorded state may be out of date
  CLAUDE-OPEN.md            GENERATED — p0/p1 open items
  DECISIONS.md              GENERATED — open decisions and questions
  TRIAGE.md                 GENERATED — imported items not yet verified against current code
  history/                  frozen records (old GAPS.md sections, migration summary)
.roko/work-local/items/     same format, untracked: private/local items (e.g. application-specific work)
.roko/work-claims/          untracked: who is working on what right now
```

Regenerate the views with `python3 tools/work.py render` after changing items, and validate with
`python3 tools/work.py check` (`check --lint` also lists weak verify commands).

## Item format

Markdown with TOML front matter between `+++` lines. `python3 tools/work.py new --kind bug --title "…" --source "…"`
creates one with the body template filled in.

```toml
+++
id = "gap-7f3a2c"                 # <prefix>-<6 hex>; see "IDs"
kind = "gap"                      # gap | bug | regression | finding | decision | spec | question
title = "Job cancellation returns 500"
status = "open"                   # open | in_progress | blocked | done | wontfix | superseded | parked
triage = "verified"               # verified (checked against code on last_verified) | unverified (imported, not yet checked)
severity = "p2"                   # p0 (broken core loop / security) … p3 (polish)
goal = "core"                     # optional: a key from goals.toml; open items without a goal are "later"
rank = 1                          # optional: pin the order within a goal (lower first)
size = "M"                        # optional: S | M | L
hold = "Waiting for the auth redesign"   # optional: keeps the item out of `next` and NOW.md
subsystem = ["roko-serve/jobs"]   # crate or crate/area; used to group STATUS.md
created = 2026-09-26
updated = 2026-09-28
last_verified = 2026-09-28        # omit if never verified
last_verified_rev = "d9e79e9d8"   # optional: the commit it was checked against (drift is measured from here)
source = "gaps-md#gap-2026-09-26" # where this item came from (provenance)
discovered_from = "plan:portal-programme/01-backend-plan-service#T09"   # optional: what surfaced it
anchors = ["crates/roko-serve/src/routes/jobs.rs::cancel_job_endpoint"] # path::symbol preferred; path:line allowed
doc = ""                          # kind = spec/decision: path to a long-form document
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]                         # optional: fails while the problem exists
command = "cargo test -p roko-serve --test job_lifecycle -- --include-ignored test_cancel_from_assigned_state"

[[verify]]                        # required to close as done: fails now, passes once fixed
command = "grep -rqw 'fn cancel_from_assigned_state' crates/roko-serve/ && cargo test -p roko-serve --test job_lifecycle cancel_from_assigned_state"

[closed]                          # filled when status leaves open: evidence of the transition
# at = 2026-10-02, commit = "abc1234", run_id = "graph-…", by = "plan:…#T03", evidence = "…"
+++
```

The anchors matter beyond navigation: `next` uses them to keep parallel workers apart, and `DRIFT.md` uses them
to notice when the code an item describes has changed.

### Body template

A worker with no other context must be able to act on the body alone. Use these sections, briefly for small
items:

- `## Problem`: what is wrong or missing, as observed: the symptom, how to see it, actual vs expected.
- `## Why it matters`: the goal it serves and what it unblocks or puts at risk; related item IDs.
- `## Where`: the code involved (the anchors, explained) and the entry point that reaches it.
- `## Current state`: what exists today, what is already partly fixed (with commits), what is known about the cause.
- `## Plan`: the suggested approach, step by step; if there is a real design choice, the options and trade-off.
- `## Done when`: observable acceptance criteria, including the `[[verify]]` command.
- `## Notes`: constraints (files not to touch, decisions already made, risky areas) and anything else to know first.

### Verify commands

A `[[verify]]` command must **fail while the problem exists and pass once it is fixed**. Common mistakes that
make it pass while the bug is still there:

- a pipeline ending in `| head`, which exits 0 whatever grep found;
- `cargo test -p <crate>` with no filter, which passes while the bug is present;
- a cargo test filter naming a test that does not exist yet: cargo exits 0 when nothing matches. Guard it:
  `grep -rqw 'fn <test_name>' crates/<crate>/ && cargo test -p <crate> <test_name>`;
- grepping for the problem pattern instead of for the fix.

Prefer a named test that the fix adds, guarded as above, plus a static check where one is distinctive.
`python3 tools/work.py check --lint` flags the first three.

### IDs

`<prefix>-<first 6 hex of sha256("<kind>|<title>|<created>|<source>")>`; on collision use 8 hex.
Prefixes: `gap`, `bug`, `reg` (regression), `find` (finding), `dec` (decision), `spec`, `q` (question).
Hash IDs never collide across parallel agents or worktrees, unlike counters. File name: `<id>-<short-slug>.md`.

### Status and closure rules

- `open` → `done` requires evidence: the `[[verify]]` commands pass at a recorded commit, **or** a plan task
  declaring `closes = ["<id>"]` succeeded (record `run_id`). `tools/work.py close` fills `[closed]` and refuses
  if the static part of the verify command fails.
- `wontfix` / `superseded` require a reason in `[closed].evidence` (and `links.duplicate_of` or a pointer).
- A `done` item whose verify command later fails is reopened as `kind = "regression"` linking the old item.
- `triage = "unverified"` items (bulk imports) are listed in `TRIAGE.md` until someone checks them against the
  code and sets `triage = "verified"` and `last_verified`.
- Moving or renaming files is never closure.

## Keeping the graph current

Items go stale when work lands without anyone closing them: a big commit fixes three bugs and nobody updates
their files. Three layers prevent that.

1. **Close at the source.** When work finishes, the item is closed in the same flow:
   - workers close their item as step 8 above;
   - any commit whose message says `Closes: <id>` (or `Fixes:` / `Resolves:`) is picked up by
     `python3 tools/work.py sync`, which closes the item with that commit as evidence;
   - a plan task with `closes = ["<id>"]` that passes its gates is picked up by `sync` from the Graph checkpoint
     (`.roko/state/graph/<plan>/checkpoint.json`), with the `run_id` as evidence.
   `sync` will not close an item whose verify command fails; it reports a conflict instead. The orchestrator
   runs `sync` after every batch.
2. **Drift is visible.** `render` writes `DRIFT.md` and a one-line summary at the top of `NOW.md`:
   - items a commit or passed plan task claims to close;
   - items whose anchored code no longer exists;
   - items whose anchored files changed since `last_verified_rev` / `last_verified`;
   - items mentioned in commits since then;
   - items not checked for 21 days.
   After merging someone else's branch, `python3 tools/work.py touched --rev <base>..HEAD` lists the open items
   it touched.
3. **Sweep what is left.** After each batch of merges or programme, and at least weekly, run `/work-sweep`.
   It re-checks the drifted items against the code with agents, one verdict per item, and applies the verdicts
   (`tools/work.py apply-verdicts`). Closed items keep their evidence; partly fixed items get a dated note that
   says exactly what remains.

## Parking

`parked` means "not planned": nobody has decided to act on the item, so it is not work. It is the
default for bulk imports and for audit findings nobody has picked up. Parked items live in
`work/parked/` with a `[parked]` table (`at`, `from_status`, `reason`). The views only count them.
They stay searchable with grep.

- Park: `tools/work.py park <id>… --reason "…"`. Revive: `tools/work.py unpark <id>…` restores the previous
  status and moves the file back to `items/`.
- Parking is not closure: a parked defect may still be real. Revive it, and verify it first, when it
  matters again.

## Rules for agents and humans

1. New gap / bug / finding → new item file (not prose in another document). Include at least one anchor,
   a body that follows the template, and a `[[verify]]` command whenever one can be written.
2. Audits and research stay as dated documents; each finding they contain becomes an item with
   `discovered_from` pointing at the document.
3. Plans reference items: `closes = ["gap-…"]` on the task that fixes them.
4. Code markers carry IDs: `TODO(gap-…)`, `#[ignore = "gap-…: reason"]`.
5. Commits that finish an item say `Closes: <id>`.
6. Never hand-edit the generated views: `NOW.md`, `STATUS.md`, `DRIFT.md`, `CLAUDE-OPEN.md`, `DECISIONS.md`,
   `TRIAGE.md`.
7. Status claims about roko do not go in CLAUDE.md or README — they come from here.
