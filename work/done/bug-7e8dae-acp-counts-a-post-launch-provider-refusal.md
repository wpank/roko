+++
id = "bug-7e8dae"
kind = "bug"
title = "ACP counts a post-launch provider refusal as a failed prompt-experiment trial instead of abandoning it"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-05
updated = 2026-10-05
last_verified = 2026-10-05
last_verified_rev = "8e04833bb"
source = "wave-20 follow-up reports 2026-10-05 (bug-897879, work/bug-897879)"
discovered_from = "bug-897879 (open; its fix covers the pre-dispatch half of this asymmetry, not post-launch)"
anchors = ["crates/roko-acp/src/bridge_events/cost.rs::acp_dispatch_succeeded", "crates/roko-cli/src/graph_task_dispatch/prompt_experiment.rs::settlement"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn acp_abandons_a_post_launch_provider_refusal' crates/roko-acp/ && cargo test -p roko-acp acp_abandons_a_post_launch_provider_refusal"

[closed]
at = 2026-10-05
at_ts = "2026-10-05T18:58:22Z"
commit = "8e04833bb"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-05T17:36:18Z"
forced = false
evidence = "Gate 21 (merged 8e04833bb): verify acp_abandons_a_post_launch_provider_refusal passes (ACP provider_failover suite). acp_learning_success gives ACP Graph's learning label, so a provider failure after launch abandons the prompt-experiment trial instead of charging the variant."
+++

## Problem

Once a prompt has launched, ACP still counts a refusal by every candidate provider as a failed
prompt-experiment trial, while Graph dispatch abandons provider errors. `acp_dispatch_succeeded`
(`crates/roko-acp/src/bridge_events/cost.rs:606-615`) is a blunt boolean: `task_error.is_none()
&& stream_error.is_none() && ...StopReason::EndTurn`. Any error at all — `stream_error:
Some(...)` from a provider failing or refusing mid-stream, exactly as much as a model genuinely
responding and failing the task — makes it `false`. That `false` is passed straight through as
`record_acp_experiment_outcome`'s `success` parameter
(`crates/roko-acp/src/bridge_events/mod.rs:1043-1058`), which settles the receipt as
`AssignmentSettlement::Observed { success: false }` — a counted failed trial, charged against
the variant, exactly as if the model had answered and gotten it wrong.

`bug-897879`'s own fix (on `work/bug-897879`, not yet merged) only moved *when* the receipt is
marked dispatched to "once the provider failover has picked a model," so a prompt that fails
*before any provider is picked at all* (the pre-dispatch cases) now correctly settles as
`Abandoned` with no trial counted. It did not touch what happens *after* a model is picked and
dispatch genuinely starts — that path still goes through `acp_dispatch_succeeded`'s
undifferentiated boolean.

Graph dispatch does distinguish this, in `crates/roko-cli/src/graph_task_dispatch/prompt_experiment.rs`'s
own outcome-to-settlement table (lines 297-307): `(AttemptOutcome::ProviderError, true,
abandoned)` and `(AttemptOutcome::ProviderExhausted, false, abandoned)` both settle as
`Abandoned`, never as an observed failure — a provider-side outage never counts against a
variant's trial statistics on that path.

## Why it matters

Goal: learning, prompt-experiment validity (same goal as `bug-a3f005`/`bug-897879`). An
experiment's whole point is measuring a real difference in outcomes between variants. A
provider outage has nothing to do with which variant was served; counting it as a failed trial
anyway biases a variant's measured failure rate upward by however often its model's provider
happens to be unavailable — pure noise that looks like signal.

## Where

- `crates/roko-acp/src/bridge_events/cost.rs::acp_dispatch_succeeded` (the undifferentiated
  boolean; needs to distinguish a provider-side error from a genuine task outcome).
- `crates/roko-acp/src/bridge_events/mod.rs:1043-1058` (the caller; needs to route a
  provider-error case to `AssignmentSettlement::Abandoned` instead of `Observed { success:
  false }`).
- `crates/roko-cli/src/graph_task_dispatch/prompt_experiment.rs:297-307` (Graph dispatch's
  outcome-to-settlement table; the pattern to match).

## Current state

Confirmed on `work/bug-897879` (not yet merged): `acp_dispatch_succeeded` has no path to
`Abandoned`; every error, provider-side or not, becomes a counted failed observation.

## Plan

1. Distinguish a provider-side error (`stream_error`/`task_error` originating from the
   provider/transport layer, not a model response) from a genuine task outcome at the call site
   that currently computes `dispatch_succeeded`.
2. When the failure is provider-side, settle the receipt as `AssignmentSettlement::Abandoned`
   instead of `Observed { success: false }`.
3. Regression test: a dispatch whose only candidate provider refuses mid-stream settles its
   receipt as `Abandoned`, not as a failed trial.

## Done when

- A provider refusal after dispatch has launched abandons the receipt, as Graph dispatch does,
  instead of counting as a failed trial.
- The `[[verify]]` command passes.

## Notes

- 2026-10-05 (wave-20 follow-up, bug-897879, work/bug-897879 not yet merged): confirmed
  directly by reading `acp_dispatch_succeeded`, its caller, and Graph dispatch's
  outcome-to-settlement table side by side. `bug-897879`'s own fix addresses only the
  pre-dispatch (before a provider is picked) half of this asymmetry; this is the remaining,
  post-launch half.

## Progress

- 2026-10-05 (w4-length): implemented on `work/bug-7e8dae`; cargo verification deferred to the batch gate.
  - `acp_learning_success` (cost.rs) gives ACP Graph's learning label. It returns `Some(true)` for a turn that
    succeeded, `Some(false)` for one that failed, and `None` when the dispatch's own error is one that
    `classify_failure_text` names: exhaustion, billing, auth, rate limit, server error or empty response. A timeout
    counts as a provider failure only before any answer arrived; after part of an answer it counts against the
    turn, as in Graph.
  - The receipt is now settled by `settle_acp_experiment` (experiments.rs): `None` abandons it with no trial
    counted. `acp_dispatch_succeeded` still drives the efficiency event and the router observation.
  - The classification reads only the dispatch's error, not `stream_error`, so a post-dispatch safety block
    (a model's own fault) still counts as a failed trial.
  - Tests: `acp_abandons_a_post_launch_provider_refusal` in `tests/provider_failover.rs` (a pinned prompt's only
    provider refuses with its session limit after launch; the receipt ends Abandoned with no trial), and
    `acp_learning_success_says_nothing_after_a_provider_failure`.
