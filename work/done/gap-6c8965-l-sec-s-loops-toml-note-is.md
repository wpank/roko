+++
id = "gap-6c8965"
kind = "gap"
title = "L-sec's loops.toml note is stale, the census's L-know check reads a dead file, [sections] is undocumented, and 4131's fixture has no knowledge store"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-learn/loop-audit", "roko-core/config", "roko-cli/tests"]
created = 2026-10-03
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "d16bc9969"
source = "wave-7 follow-up reports 2026-10-03 (PK38 gap-894977)"
discovered_from = "gap-894977"
anchors = ["crates/roko-learn/src/loop_audit/loops.toml", "crates/roko-learn/src/loop_audit/census.rs::RETRIEVAL_OUTCOMES", "crates/roko-core/src/config/sections.rs", "crates/roko-cli/tests/learning_wiring_census.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn census_l_know_counts_exposures_not_the_retired_retrieval_log' crates/roko-learn/ && cargo test -p roko-learn census_l_know_counts_exposures_not_the_retired_retrieval_log"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T13:06:35Z"
commit = "d16bc9969"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-04T10:31:55Z"
forced = false
evidence = "Gate 17b (merged d16bc9969): verify census_l_know_counts_exposures_not_the_retired_retrieval_log passes. The census's L-know counts the runs' exposures (the retired retrieval log only when no run records exposures), the L-sec note is updated, and [sections] is documented. The fourth point (a knowledge store in the 4131 census fixture) moved to gap-5b8767."
+++

## Problem

Four small, related staleness/completeness gaps in the knowledge/section learning loops and their census,
surfaced by PK38 (gap-894977, done):

1. **loops.toml's L-sec note is stale.** `crates/roko-learn/src/loop_audit/loops.toml`'s `L-sec` entry (section
   bandit) still says `notes = ["Not built: S02.P1-10. ..."]` and its static finding claims "No section is ever
   excluded at random: prompt assembly only reads section effects through SectionEffectivenessSource" — both
   dated `verified_at = "976220c3ebb91d3879e9d7701fa4aec5d9723d1f"`. The section bandit is now fully built and
   wired: `crates/roko-cli/src/dispatch/prompt_builder.rs` imports `roko_learn::section_effect::SectionBandit`,
   carries a `section_bandit: Option<Arc<SectionBandit>>` field, and draws each droppable section's inclusion
   through it (`~lines 1259-1272`, `bandit.decide(...)`). The "Not built" note and the "only reads... through
   SectionEffectivenessSource" claim are both false at HEAD.
2. **The loop census's L-know liveness check reads a file nothing writes.** `crates/roko-learn/src/loop_audit/census.rs:315`:
   `logs.retrievals = read_jsonl(&paths.root.join(RETRIEVAL_OUTCOMES)).len()`, where `RETRIEVAL_OUTCOMES =
   "retrieval-outcomes.jsonl"` (line 37). Backlog task 4129 ("Stop writing `retrieval-outcomes.jsonl` once
   exposures record what reached the prompt," part of gap-894977, done) removed the writer: "`retrieval_ctx` and
   `retrieval_outcomes_path` removed; the TUI and serve readers keep the historical file" (4129's own Progress
   note). So `self.retrievals` is now always 0 in any fresh workspace, which makes the L-know "Unlogged" check
   (`census.rs:404-416`: `unlogged = self.retrievals > 0 && episodes_with_knowledge == 0 && efficiency_with_knowledge
   == 0`) **vacuous** — `self.retrievals > 0` can never be true, so this check can never fire even if knowledge
   genuinely stopped reaching episodes/efficiency rows. The replacement signal exists:
   `crates/roko-cli/src/graph_task_dispatch/decision_log.rs::record_exposures`/`ExposureRecord` writes
   `exposures.jsonl`, "one row per item the prompt retrieved" (its own module doc comment) — the census should
   count retrievals from there instead.
3. **`[sections] pinned` has no docs.** `crates/roko-core/src/config/sections.rs` defines the config section
   (module doc: "`[sections]`: the prompt sections the section bandit never leaves out... built on top of the
   built-in ones"), and `crates/roko-core/src/config/schema.rs:211` wires it. `docs/v3/depth/21-config/` has no
   mention of `[sections]` or `pinned` by direct grep.
4. **Task 4131's census fixture can't demonstrate a verified pass reinforcing included knowledge.** Its own
   comment (`crates/roko-cli/tests/learning_wiring_census.rs:560`): "workspace has no knowledge store, so its
   rows are prompt sections and..." — the fixture that's supposed to prove every in-scope loop is live,
   observe-only or retired structurally cannot exercise L-know's reinforcement path (a verified pass crediting
   the knowledge entries it used), since there's no knowledge store for a pass to reinforce entries in.

## Why it matters

Goal: learning (S02 loop census accuracy). (1) and (3) are stale/missing documentation — low risk on their own,
but (1) specifically means the census's own machine-readable record of "what is built" disagrees with reality.
(2) is the more serious one: a real integrity check (does knowledge actually reach episodes/efficiency rows) has
been silently disabled since 4129 landed, and nobody would notice because the check still runs and still passes
(vacuously) rather than erroring. (4) means the census's own acceptance fixture (4131, "every loop live,
observe-only or retired") can never actually prove L-know's reinforcement half works, even though the census as
a whole is meant to be the evidence S03/S09 rely on.

## Where

- `crates/roko-learn/src/loop_audit/loops.toml` (`L-sec`'s `notes` and `static_findings`).
- `crates/roko-learn/src/loop_audit/census.rs` (`RETRIEVAL_OUTCOMES`, `logs.retrievals`, the `"L-know"` match arm).
- `crates/roko-cli/src/graph_task_dispatch/decision_log.rs` (`record_exposures`/`ExposureRecord`, the replacement
  signal).
- `crates/roko-core/src/config/sections.rs` / `schema.rs:211` (the undocumented config) and
  `docs/v3/depth/21-config/01-schema-sections.md` (where to add it — note: this file already has other pending
  doc-gap Notes from earlier batches, e.g. `[pricing]`, `stream_usage`, `[experiments]` on `gap-d2c64f`; add
  `[sections]` to that same running list rather than duplicating the item).
- `crates/roko-cli/tests/learning_wiring_census.rs` (the 4131 fixture, line ~560).

## Current state

All four confirmed at HEAD as described. None is yet tracked.

## Plan

1. Update `loops.toml`'s `L-sec` entry: replace the "Not built" note and the now-false static finding with a
   current one pointing at `prompt_builder.rs`'s actual bandit wiring, re-verified at today's commit.
2. Change `census.rs`'s L-know liveness check to count retrievals from `exposures.jsonl`
   (`ExposureItemKind`/whatever marks a knowledge-kind exposure) instead of the dead `retrieval-outcomes.jsonl`.
3. Add `[sections]` (fields: at least `pinned`) to `docs/v3/depth/21-config/01-schema-sections.md`'s pending-gaps
   list (or directly, if that's simpler than queuing another note).
4. Give the 4131 fixture a minimal knowledge store (a couple of seeded entries) so a verified pass can actually
   exercise and show L-know's reinforcement crediting them.

## Done when

- `loops.toml`'s L-sec entry matches the built section bandit.
- The census's L-know check reads live data from `exposures.jsonl`, not the dead `retrieval-outcomes.jsonl`.
- `[sections]` is documented.
- The 4131 fixture's workspace has a knowledge store and its test shows a verified pass reinforcing an included
  entry.
- The `[[verify]]` command passes.

## Notes

- `--no-holdout`/`roko run --no-holdout` is explicitly out of scope here — already tracked as `gap-29fe0a`.
- These four are grouped in one item because they're all small, same-area (PK38's own knowledge/section wiring)
  findings; a worker may close them independently (e.g. one commit per facet) if that's cleaner, but they don't
  need four separate work-graph items.
- 2026-10-04 (wave-17b follow-up): facet 4 (the 4131 fixture needs a seeded knowledge store to
  show L-know reinforcement) is split out as `gap-5b8767`, since it needs the real binary to
  check and couldn't be verified in the same static-only round as facets 1-3. On
  `work/backlog-batch-17b` (tip `00a28ea0c`, not yet merged), facets 1-3 are done (`census.rs`,
  `loops.toml` and `docs/v3/depth/21-config/01-schema-sections.md` are all in that branch's
  diff); `learning_wiring_census.rs` (facet 4) is untouched there. This item can close on
  facets 1-3 alone; track facet 4 via `gap-5b8767` instead of this item's own "Done when".
