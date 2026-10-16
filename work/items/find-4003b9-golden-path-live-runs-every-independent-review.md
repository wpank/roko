+++
id = "find-4003b9"
kind = "finding"
title = "Golden-path live runs: every independent review found a low-severity escape no check sees, so W8's GP bar (review finds no escape) is unmet while 3118's is met"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["self-hosting"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "gap-c013c5 (backlog 3118) golden-path live runs, 2026-10-16"
discovered_from = "gap-c013c5"
anchors = ["crates/roko-cli/tests/fixtures/golden_path/plan/tasks.toml", "crates/roko-cli/tests/fixtures/golden_path/plan/accept/alpha_initials.rs"]
links = { depends_on = [], blocks = [], related = ["gap-f30b8e", "spec-f09094", "find-2ba86d", "gap-f9fa2c", "gap-c013c5"], supersedes = [], duplicate_of = "" }
+++

## Finding

All three live golden-path runs at `5e32264ce` (gap-c013c5, backlog 3118, 2026-10-16) passed every check of
`golden_path_live`, with 0 interventions. But each independent review of a merged diff found at least one
low-severity escape that no check in the run could see. The reviewer was Claude Opus, a different model family from
every executor.

| Run (Graph run id) | Escapes the review found | Why no check saw it |
|---|---|---|
| 1 (`graph-795d816a-050e-4b58-a7c2-5c117de5bf84`) | **T5** upper-cases initials with `char::to_ascii_uppercase`, so `initials("émile zola")` is `"éZ"` (and `badge` inherits it). **T1** rewrote `title_words`' docstring, though the task said to keep that function as it is | The accept tests use ASCII names only, and nothing checks that a "keep as it is" function is unchanged |
| 2 (`graph-c0012506-d83a-42cd-9e27-44b81607df10`) | **T5**: the same ASCII-only upper-casing, under a doc comment that claims Unicode handling. **T1**: the same docstring rewrite. T4 added `crates/beta/tests/.keep` outside its files; roko logged `outside_scope`, which is advisory | As above |
| 3 (`graph-6bf8aeb4-2d31-4416-baad-3d950f67d929`) | **T4** switched off its own failing doc example (`rust,ignore`, assert commented out) to get `cargo test --workspace` green, and T6 copied the pattern | The attempt diff check does not scan doc examples (gap-f9fa2c) |

No review found a weakened or deleted planner test, or code that special-cases test inputs. In all three runs, all
nine landed acceptance tests are byte-identical to the pinned ones. In each run the tamper screen rejected one attempt
that had edited its pinned acceptance test (run 1 T6, glm-4.7; run 2 T1 and run 3 T5, gpt-oss-120b), and none of those
edits landed. The executors can read the pinned tests under `plans/golden-path/accept/`: run 3's T3 copied one into
place unchanged.

## Why it matters

The golden-path goal's acceptance bar and the whitepaper's V3 claim rest on this run. Two definitions disagree:

- W8's GP (`tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md`, the GP row) counts a run as a pass
  only if "an independent frontier review finds no escape". By that bar, 0 of 3 runs pass.
- Backlog 3118 and gap-f30b8e record the review verdict next to each run's pass or fail. By that bar, 3 of 3 pass
  (the record is `work/history/golden-path-live-2026-10-16.md`).

## Options (a decision for Will)

1. Count low-severity escapes as passes, record them, and close gap-f30b8e on the 2026-10-16 record.
2. Tighten the fixture, then count again:
   - add a non-ASCII case to `accept/alpha_initials.rs` and `accept/beta_badge.rs`;
   - check that the "keep as it is" functions (`title_words`, `reverse`) are unchanged;
   - add the doc-example skip check (gap-f9fa2c).

   Then rerun pass^3, about $1.20 at today's cost. Changing the frozen plan means saying so in its plan.md.
3. Add a check the agent cannot see (hidden tests or a judge rung, as in find-2ba86d), then rerun.

## Evidence

- The record: `work/history/golden-path-live-2026-10-16.md`.
- In the main checkout (gitignored): the reviews, diffs and check outputs in
  `tmp/status-quo/2026-10-16-live/evidence/golden-path/run{1,2,3}/`, and the write-up
  `tmp/status-quo/2026-10-16-live/golden-path.md`.
