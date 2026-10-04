+++
id = "gap-86286e"
kind = "gap"
title = "B3 (verify-depth floor) has no S5 ceiling field, unlike its B7/B8 siblings"
status = "done"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-core/config", "roko-learn/homeostasis"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "1a778a57c"
source = "wave-10 follow-up reports 2026-10-04 (PK70 gap-fa4d4b)"
discovered_from = "gap-fa4d4b; decision 8101 point c"
anchors = ["crates/roko-core/src/config/harness_params.rs", "crates/roko-learn/src/homeostasis/controller.rs::SafetyBox"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn b3_is_bounded_by_the_s5_verify_depth_ceiling' crates/roko-learn/ && cargo test -p roko-learn b3_is_bounded_by_the_s5_verify_depth_ceiling"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T10:30:45Z"
commit = "1a778a57c"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-04T08:39:32Z"
forced = false
evidence = "Gate 16b (merged 1a778a57c): verify b3_is_bounded_by_the_s5_verify_depth_ceiling passes. viability.toml verify.max_floor (default V4, so existing policies are unchanged) caps B3, and the SafetyBox raises FloorAboveMax for any change above it; the property test runs under a V3 ceiling. S06's gates.max_rung wording is queued as a spec fix."
+++

## Problem

M1's B3 move (`crates/roko-core/src/config/harness_params.rs:45,71,532`, "verify-depth floor request... from
none (V0) to V4") has no ceiling field anywhere in the S5 viability-policy file (`ViabilityPolicy`). Its
siblings B7 and B8 do: B7's doc comment is explicit — "multiple of **S5's audit rate**" — and B8's: "share of
**the S5 per-task budget ceiling**" (`harness_params.rs:66-68,547-549`). B3 has no equivalent "S5's verify-depth
ceiling" to bound how high the controller may push a task's verify-depth floor; it's constrained only by the
generic one-notch-per-move mechanism (`homeostasis/catalog.rs`) and the `SafetyBox`'s broader safety checks
(`homeostasis/controller.rs::SafetyBox`), neither of which is specific to verify depth.

## Why it matters

Goal: cybernetic, M1 controller (S06). B7 and B8 both have a policy-defined ceiling the controller's moves are
checked against; B3 doesn't, so there's no S5-defined limit on how deep the controller may push verify depth
before some other, more generic mechanism happens to stop it. If the intent (per decision 8101, point c) was for
every M1 knob to have a policy-defined bound, B3 is the one missing it.

## Where

- `crates/roko-core/src/config/harness_params.rs` (B3's definition, and B7/B8's ceiling fields for comparison).
- `crates/roko-learn/src/homeostasis/controller.rs::SafetyBox` (the generic check B3 currently relies on instead).

## Current state

Confirmed: no verify-depth ceiling field exists in the S5 viability-policy structure.

## Plan

1. Add a verify-depth ceiling field to the S5 viability-policy file, mirroring B7's audit-rate ceiling and B8's
   budget ceiling, and have B3's moves check against it the same way.

## Done when

- B3 is bounded by an explicit S5-defined verify-depth ceiling, not just the generic notch-ladder/SafetyBox
  mechanism.
- The `[[verify]]` command passes.

## Notes

- Discovered during PK70's work (gap-fa4d4b, done).
