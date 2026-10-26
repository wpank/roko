+++
id = "bug-b7d9ad"
kind = "bug"
title = "A warm build tree keeps compiled-in source paths: relocation rewrites flags.make but keeps its mtime, so an object built with a definition naming the person's checkout (Undertow's UNDERTOW_SOURCE_DIR) is not rebuilt, and a check writes into the person's checkout (rehearsal p41: T2's PNGs landed in the base, T2 settled already_satisfied, the delivery has none)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/worktree"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "round-v7 rehearsal p41 (roko-ladder), the lead's ask"
discovered_from = "agent:roko-ladder (round v7 rehearsal p41)"
anchors = ["crates/roko-cli/src/orchestrator/worktree/build_trees.rs::relocate", "crates/roko-cli/src/orchestrator/worktree/build_trees.rs::write_keeping_times"]
links = { depends_on = [], blocks = [], related = ["gap-6754ab", "bug-40bf59"], supersedes = [], duplicate_of = "" }
+++

## Problem

A warm build tree that roko clones from the person's checkout into an attempt's worktree (gap-6754ab) keeps the
compile-time source paths of what it already built. `relocate` rewrites every absolute path in the tree's text
files to the worktree's, but gives each rewritten file back its times (`write_keeping_times`), so make rebuilds
nothing. That covers a target's `flags.make` too. An object compiled with a definition that names the source
checkout therefore keeps naming it, and a check that writes files through that path writes into the **person's
checkout**, not the attempt's worktree.

Undertow's spec target has `UNDERTOW_SOURCE_DIR="${PROJECT_SOURCE_DIR}"` (`tests/spec/CMakeLists.txt:67`), and its
review PNGs are written under it.

Round v7 rehearsal p41 (`tmp/hermes-roko-v7/roko-ladder/rehearsals/p41/`, run `20261021-120220`):
- T2, "Refresh and commit the phase 4.1 review PNGs": try 1 ran with 2 warm trees. Its check
  `tools/spec.sh 4 budget` passed in 131.9 s.
- The PNGs went to the base checkout. `ws/` shows `M artifacts/ui/editor-{idle,playing,clipping}.png`, written at
  12:11 during the run.
- The attempt's worktree changed nothing, so T2 settled `already_satisfied` and the delivery has no PNGs.
- In a live run the base is the person's real checkout. The person's tree gets dirtied, and the task commits
  nothing.

## Where

- `crates/roko-cli/src/orchestrator/worktree/build_trees.rs::relocate` and `write_keeping_times`: every rewritten
  file keeps its times.
- What runs an attempt's checks in its worktree: nothing looks at the person's checkout afterwards.

## Plan

Undertow has since moved its checks to run-time paths (Undertow ceac70e), so its spec no longer bakes the checkout
into the binary. roko's side is detection, since any project can compile a path in:
- Before an attempt's checks run, note the person's checkout's state (`git status --porcelain`, untracked files
  included).
- After the checks, compare. If the checkout gained changes while the attempt's worktree did not, that is a red
  flag: say so in the attempt's records and on the hub, naming the files. Don't settle the attempt
  `already_satisfied` on a run that wrote into the person's checkout.
- Optionally, make a compiled-in path rebuild: when relocating changes the definitions a target compiles with (a
  `*_DEFINES` line of its `flags.make`), or a generated source or header, give that file a fresh mtime so make
  rebuilds just those objects.

## Done when

An attempt whose checks change the person's checkout instead of its own worktree says so, on the hub and in its
records, and is not settled `already_satisfied`. Checked by the tests in `[[verify]]`.
