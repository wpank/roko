+++
id = "gap-c0d709"
kind = "gap"
title = "Should plan run's earlier spec_gate_before_run also read the self-model's refine requests?"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli"]
created = 2026-10-05
updated = 2026-10-05
last_verified = 2026-10-05
source = "wave-20 follow-up reports 2026-10-05 (gap-2b0575, work/gap-2b0575)"
discovered_from = "gap-2b0575 (open; own Progress note records this as a deliberate, open choice)"
anchors = ["crates/roko-cli/src/commands/plan.rs::spec_gate_before_run", "crates/roko-cli/src/spec_gate.rs::gate_plans"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn spec_gate_before_run_reads_refine_requests' crates/roko-cli/ && cargo test -p roko-cli spec_gate_before_run_reads_refine_requests"
+++

## Problem

Only the plan-load gate (`gate_plans`) reads the self-model's refine requests; `plan run`'s
earlier check, `spec_gate_before_run`, doesn't. `gap-2b0575`'s fix (on `work/gap-2b0575`, not
yet merged) wired `apply_refine_requests`/`read_refine_requests` into `gate_plans`
(`crates/roko-cli/src/spec_gate.rs:176-186`, its own doc comment: "the plan-load gate (3231):
`check_plans` over `files`... and the self-model's refine requests in the workspace's runs
(`apply_refine_requests`)"). `spec_gate_before_run`
(`crates/roko-cli/src/commands/plan.rs:2655-2678`), the earlier pre-check `roko plan run`
performs, calls only `check_plans` — confirmed by reading its full body: zero references to
`apply_refine_requests`, `read_refine_requests`, or "refine" anywhere in the function. The
item's own Progress note records this explicitly as a deliberate, open choice: "Left as is:
`plan run`'s early pre-check (`spec_gate_before_run`) does not read requests; the plan-load
gate, which every run passes before its first dispatch, does."

## Why it matters

Goal: cybernetic, S07 spec quality / M3 self-model interface, same goal as `gap-2b0575`. If
`spec_gate_before_run` reports a clean pass (no blocking findings) while an open refine request
exists for one of the plan's tasks, and a human or script reads that early pass as the final
word (rather than waiting for the plan-load gate, which runs later, closer to first dispatch),
they could be told "the plan looks fine" when a self-model-flagged weak spec would in fact
block the task once the run actually starts.

## Where

- `crates/roko-cli/src/commands/plan.rs::spec_gate_before_run` (the earlier check; doesn't
  read refine requests).
- `crates/roko-cli/src/spec_gate.rs::gate_plans`, `apply_refine_requests` (the plan-load gate;
  does).

## Plan (decision needed)

- **Option A — make `spec_gate_before_run` read refine requests too**, mirroring
  `gate_plans`'s `apply_refine_requests` call, so an operator sees the same verdict at both
  checkpoints and isn't told "clean" early only to have the run block later at first dispatch.
- **Option B — leave it as is**, on the reasoning that `spec_gate_before_run` is explicitly an
  early, cheaper sanity check (it already skips the red-on-base check for this reason, per its
  own comment: "the red-on-base check... runs once, in the plan-load gate... so it is not
  repeated") and the plan-load gate is the one true enforcement point every run passes before
  dispatch regardless — document this distinction explicitly so it's not mistaken for an
  oversight.

## Done when

Will picks an option; if A, `spec_gate_before_run` reads refine requests with a regression
test; if B, the distinction is documented next to both functions' doc comments.

## Notes

- 2026-10-05 (wave-20 follow-up, gap-2b0575, work/gap-2b0575 not yet merged): confirmed
  directly by reading both functions in full. Filed as `kind = "gap"` with the decision
  embedded in the body, per the instruction.

## Progress

- 2026-10-05 (wave 21, option A as the lead assigned): implemented at 89893d6e4 on `work/gap-c0d709`; cargo
  verification deferred to the batch gate. `spec_gate_before_run` applies the refine requests through
  `spec_gate::workspace_refine_requests` and `apply_refine_requests`, the path `gate_plans` uses, before the
  holdout, so under `enforce` an open request from a routing self-model refuses the run before it starts and
  under `advise` it advises.
- Why the naive mirror was not enough: the early check scores without the red-on-base check, while a request
  carries the plan-load gate's score, which counts SQ06 (weight 15) where that check ran. A spec refined to a
  higher static score could still score below that, and the early check would refuse it while the plan-load
  gate let it run. Requests now compare with the spec in static mode on both checks: `refine_requests`
  rescores the run's `spec.quality` record the self-model read without SQ06 (`RefineRequest::static_score`),
  and `refine_verdict` compares it with the current record's static score, falling back to the recorded
  scores when a record has no rule scores. Tests: `spec_gate_before_run_reads_refine_requests` (roko-cli bin)
  and `refine_verdict_compares_specs_in_static_mode` (roko-gate).
