+++
id = "bug-897879"
kind = "bug"
title = "Several early returns in bridge_events/mod.rs leave ACP prompt-experiment receipts open forever"
status = "done"
triage = "verified"
severity = "p3"
goal = "learning"
size = "M"
subsystem = ["roko-learn/prompt-experiment"]
created = 2026-10-05
updated = 2026-10-05
last_verified = 2026-10-05
last_verified_rev = "55267cfd0"
source = "wave-19 follow-up reports 2026-10-05 (bug-e3bbee, work/gap-d10a97)"
discovered_from = "bug-e3bbee (done on work/gap-d10a97; own Progress note names this follow-up)"
anchors = ["crates/roko-acp/src/bridge_events/mod.rs", "crates/roko-acp/src/bridge_events/experiments.rs::mark_acp_experiment_dispatched"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn dispatch_failure_after_mark_settles_the_receipt_as_abandoned' crates/roko-acp/ && cargo test -p roko-acp dispatch_failure_after_mark_settles_the_receipt_as_abandoned"

[closed]
at = 2026-10-05
at_ts = "2026-10-05T17:35:09Z"
commit = "55267cfd0"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-05T15:32:43Z"
forced = false
evidence = "Gate 20a (merged 55267cfd0): verify dispatch_failure_after_mark_settles_the_receipt_as_abandoned passes, plus invalid_image_prompt_settles_the_receipt_as_abandoned and open_experiment_receipt_abandons_unless_settled. ACP marks Dispatched at the actual launch, and an OpenExperimentReceipt guard abandons a receipt on any other exit; post-launch provider refusals are bug-7e8dae."
+++

## Problem

`bug-e3bbee` (done on `work/gap-d10a97`, not yet merged) made both of `applicable_acp_experiment`'s
drops settle their receipt as `Abandoned`, but several other early returns in
`crates/roko-acp/src/bridge_events/mod.rs`, between where a receipt is prepared and where its
dispatch actually completes, still leave it open forever:

- **The pre-dispatch safety violation** (`return
  Err(anyhow::anyhow!("ACP pre-dispatch safety violation: {}", message).into());`, ~line 706)
  and **"no usable provider"** (`return Err(anyhow::anyhow!("no usable provider for the prompt:
  {why}").into());`, ~line 789) both occur numerically after both call sites of
  `mark_acp_experiment_dispatched` (lines 554 and 618), so by the time either fires the
  receipt has already transitioned `Prepared -> Dispatched` — and then gets stuck there, never
  reaching `Observed` or `Abandoned`.
- **The image-validation errors** (`validate_model_input_messages(...)?` at ~line 465,
  `model_input_messages_from_wire(...)?` at ~line 574) both occur before either
  `mark_acp_experiment_dispatched` call site, so they leave the receipt stuck at `Prepared`
  instead.

`assign_acp_experiment` (line 367) prepares the receipt; `applicable_acp_experiment` (line 377,
`bug-e3bbee`'s fix) now settles its own two drop paths; everything between there and a
successful dispatch/settlement is a separate, unguarded stretch of early-return sites that
this fix didn't touch.

## Why it matters

Goal: learning, same goal as `bug-e3bbee`/`bug-a3f005`. Every one of these orphaned receipts is
the same kind of accumulating leak `bug-e3bbee` fixed for its two cases: dead rows in the
experiment store, never settled, never counted as an outcome, permanently skewing the store's
own prepared-vs-settled bookkeeping for any dispatch that hits a safety violation, provider
outage, or a malformed image input.

## Where

- `crates/roko-acp/src/bridge_events/mod.rs` (all four sites: ~465, ~554/~574 boundary, ~706,
  ~789).
- `crates/roko-learn/src/prompt_experiment.rs::ExperimentStore::settle_attempt`,
  `AssignmentSettlement::Abandoned` (the mechanism `bug-e3bbee` already wired in as
  `abandon_acp_experiment`; the pattern to extend).

## Current state

Confirmed on `work/gap-d10a97` (not yet merged): all four early-return sites remain unguarded;
only `applicable_acp_experiment`'s two drop paths settle anything.

## Plan

1. Wrap (or guard with a cleanup/`defer`-style helper) the dispatch flow so any early return
   between `assign_acp_experiment`/`mark_acp_experiment_dispatched` and the eventual settlement
   call also settles the receipt as `Abandoned`, rather than fixing each of the four sites by
   hand (new early-return sites could otherwise reintroduce the same leak).
2. Regression tests: one dispatch failing at each of the safety-violation, no-usable-provider,
   and image-validation points settles its receipt as `Abandoned`, not left `Prepared` or
   `Dispatched`.

## Done when

- No early return in the ACP dispatch path between receipt preparation and settlement leaves a
  receipt permanently `Prepared` or `Dispatched`.
- The `[[verify]]` command passes.

## Notes

- 2026-10-05 (wave-19 follow-up, bug-e3bbee, work/gap-d10a97 not yet merged): confirmed
  directly on the branch, line by line. `bug-e3bbee`'s own Progress note on that branch names
  this exact follow-up explicitly: "Not in this item's scope, for the filer: other early
  returns between preparation and settlement in `bridge_events/mod.rs` leave receipts open
  too."
