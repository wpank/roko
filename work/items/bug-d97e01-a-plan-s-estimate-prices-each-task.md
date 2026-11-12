+++
id = "bug-d97e01"
kind = "bug"
title = "A plan's estimate prices each task from past attempts of its tier and never from its size: phase 6's said $2.03 with high confidence, and its run stopped at the $3.00 cap with three tasks unfinished, then took $5.77 to finish (T3 to T5 are 250 to 420-line widgets under integrative's 150)"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "M"
subsystem = ["roko-learn/task_estimate"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "c39fc94bc"
source = "round v8 hermes-pilot-v8 and run-analyst-v8: phase 6 run 23837ede (Undertow 8945b3c, roko e722f0e1f); filed by roko-planner-v8 at the lead's word"
discovered_from = "agent:hermes-pilot-v8 (round v8)"
anchors = ["crates/roko-learn/src/task_estimate.rs::similar_attempts", "crates/roko-learn/src/task_estimate.rs::estimate_task", "crates/roko-cli/src/plan_estimate.rs::estimate_with"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-6605f7", "gap-29c1e8", "bug-d889fa"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn phase_6s_estimate_covers_what_its_run_spent' crates/ && cargo test -p roko-cli --lib phase_6s_estimate_covers_what_its_run_spent"
+++

## Problem

A plan's cost estimate prices each task from recent attempts of the same role and tier. It never looks at the
task's size, so it prices a task sized far beyond its tier's past attempts like those attempts, and still says "high
confidence".

- **The estimate.** Round v8's phase 6 (Undertow 8945b3c, plan `p6-the-window`, run 23837ede): the approval card said
  "estimated $2.03, high confidence", worst case $7.33. The cap was $3.00 (the estimate plus 30%).
- **What the run spent.** The first invocation stopped at its cap after 11 min 49 s, at $3.0048. T3, T4 and T5 were
  unfinished and T6 had not started. The three parallel tasks burned $1.62 in about 4 minutes. The run took three
  invocations and $5.772 to finish.
- **Per task.** Spend over the three invocations, from the run's `attempts.jsonl` (`billed_usd`). The first tries of
  T3, T4 and T5 were cut short by the cap.

| Task | Plan's tier and `max_loc` | Spec's tier and size | Tries | Spend |
|---|---|---|---|---|
| T1 spectrum | architectural, 200 | integrative, about 170 lines | 1 | $0.501 |
| T2 frame | architectural, 300 | architectural, about 450 lines | 1 | $0.882 |
| T3 controls | integrative, 150 | integrative, about 420 lines | 2 | $0.887 |
| T4 step cells | integrative, 150 | integrative, about 250 lines | 3 (one a plan-check repair) | $0.944 |
| T5 console | architectural, 300 | architectural, about 420 lines | 3 | $2.191 |
| T6 window | integrative, 150 | integrative, no size | 1 | $0.366 |
| **All** | | | 11 | **$5.772** |

- **Why.** `similar_attempts` takes the median tokens of recent attempts of the same role and tier. Every
  integrative task of this plan was priced as an integrative attempt of the past, at 150 lines or fewer, whatever its
  spec said. `confidence` is high at 10 or more matching attempts, however unlike the task they are.

## Why it matters

- The estimate sets the cap a person approves (the pilot's rule is the estimate plus 30%). An estimate this low stops
  a run part way: phase 6 needed three approvals and stopped at its cap twice.
- "High confidence" on a 2.8× miss teaches people to ignore the card.
- Goal: core.

## Where

- `crates/roko-learn/src/task_estimate.rs`: `similar_attempts`, `estimate_task`, `confidence`.
- `crates/roko-cli/src/plan_estimate.rs`: `estimate_with`, which routes each task and reads the attempt history.

## Plan

1. Price a task by its size as well as its tier: scale the matched attempts' profile by the task's `max_loc` against
   theirs (record `max_loc` on an attempt's open line if it isn't there), or match on a size band too.
2. When the plan's sources state a size over the task's tier, price that size (the reading behind PLAN_064,
   gap-29c1e8). A task the spec sizes at 420 lines is priced as 420, not as its tier's 150.
3. Lower the confidence when the matched attempts are much smaller than the task.
4. Add the test `phase_6s_estimate_covers_what_its_run_spent`, using fixtures of phase 6's plan as written and the
   attempt history its run saw. Its likely total covers the $3.00 the first invocation spent with three tasks
   unfinished, its confidence is not high, and the table above is the test's data.

## Done when

- The `[[verify]]` command passes.

## Notes

- Evidence:
  - hermes-pilot-v8's findings ("Phase 6, the window");
  - run-analyst-v8's `data/phase6.json`;
  - the run's `attempts.jsonl` in Undertow's `.roko/runs/23837ede-1d23-4d53-b8a9-fbfa362760d0/` (read-only);
  - the plan as written, `tmp/hermes-roko-v8/pilot/evidence/p6-tasks.toml.as-written`.
- The resume's estimate, which prices the whole plan again, is bug-d889fa.
- Related: gap-6605f7 (the estimator), gap-29c1e8 (PLAN_064: this spec's tier words are under its sizes).
