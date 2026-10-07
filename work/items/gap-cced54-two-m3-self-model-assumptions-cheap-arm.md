+++
id = "gap-cced54"
kind = "gap"
title = "Two M3 self-model assumptions (cheap-arm threshold, matrix-arm key fallback) need S04/S09 confirmation"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/self-model"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-7 follow-up reports 2026-10-03 (PK48 gap-d90ef6)"
discovered_from = "gap-d90ef6; two of its four assumptions are already tracked in backlog task 6117's Notes"
anchors = ["crates/roko-learn/src/self_model/policy.rs::cheap_arms", "crates/roko-learn/src/self_model/replay.rs::arm_key", "crates/roko-learn/src/self_model/mod.rs::ArmKey"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn matrix_replay_uses_the_registry_arm_key_not_the_replay_fallback' crates/roko-learn/ && cargo test -p roko-learn matrix_replay_uses_the_registry_arm_key_not_the_replay_fallback"
+++

## Problem

PK48's work on the self-model replay (gap-d90ef6, done) surfaced four implementation assumptions that need
confirming against S04/S09. I checked all four against the current code and the tracked specs:

1. **"Cheap arms" for `refine_spec` = arms at or below the median expected cost.** Confirmed as the literal
   implementation (`crates/roko-learn/src/self_model/policy.rs:112-114`, `cheap_arms`: "The cheap arms of
   `eligible`: those whose expected cost is at most the median expected [cost]"). But S04's own spec
   (`tmp/cybernetic-harness/specs/S04-*.md:41`, SC2/H4) only says "best static **cheap arm**" — it never defines
   the threshold numerically. The median-cost rule is a code-level choice the spec doesn't itself state or
   confirm. **Open: does Will/S04 agree "cheap" means "at or below the median," or was something else intended
   (a fixed cost ceiling, a tier cutoff)?**
2. **Matrix arms are S08 arm ids seen by the self-model as `roko/replay/<arm>`.** Confirmed as a real fallback in
   code: `crates/roko-learn/src/self_model/replay.rs::arm_key` wraps any arm string that doesn't already parse as
   an `ArmKey` into `ArmKey{harness: "roko", provider: "replay", model: arm, ...}`, which `Display`s as
   `roko/replay/<arm>@Default#V0` (`self_model/mod.rs:148-158`). But S09's authoritative arm registry
   (`tmp/cybernetic-harness/specs/S09-experiments.md:190-195`, the "Arm id | Harness | S04 arm key" table) gives
   every real S08 arm id (`cheap_direct`, `roko_full`, `fd_claude`, etc.) its own **real** S04 arm key —
   e.g. `cheap_direct` → `mini-loop/cerebras/gpt-oss-120b@default`, not `roko/replay/cheap_direct`. The
   `roko/replay/<arm>` stand-in is a generic fallback for a matrix cell whose arm string isn't already a well-formed
   key — it is not itself one of the registry's real mappings. **Open: when the self-model replays the pilot
   matrix (6120, 6125), does it look up each S08 arm id's real S04 key from the registry table first, falling
   back to the synthetic `roko/replay/<arm>` only for truly unregistered/ad-hoc arm strings — or does it fall
   through to the synthetic key for real, registered S08 arm ids too, which would track their calibration under
   the wrong identity?** This needs tracing through 6120/6125's actual call path before it can be called
   confirmed either way.
3. **The two ladder constants are copied from roko-cli's private `ladder.rs`.** Already tracked: backlog task
   6117's own `## Notes` says, verbatim: "The two ladder constants are private in roko-cli's `ladder.rs`. Mirror
   them here with a comment that points at that file. Moving them into `LadderConfig` would touch the hot
   `ladder.rs`, so do that only in a hot-file window." Confirmed in code: `LADDER_FAILURES_PER_RUNG = 2` and
   `LADDER_MAX_ESCALATIONS = 2` (`self_model/baselines.rs:27,31`). Nothing new to track — not re-filed.
4. **H4-BL is a new baseline id that S09's pre-registration lock (S09.E4) must name.** Also already tracked:
   6117's Notes, verbatim: "`H4-BL` is a new baseline id. S09's pre-registration lock (S09.E4) must name it, or
   rename it." Confirmed in code (`ProductionLadder::name() -> "H4-BL"`, `baselines.rs:217`). Nothing new to
   track — not re-filed.

## Why it matters

Goal: cybernetic, M3 self-model (S04/S09). Assumptions 1 and 2 are places the implementation made a specific
choice the spec text doesn't itself pin down; if either choice is wrong, the self-model's matrix-replay results
(6125, feeding S04's SC2/H4 success criterion) would be measuring the wrong thing — a miscounted "cheap arm" set
changes which arms `refine_spec` compares against, and a mis-keyed matrix arm would attribute replay performance
to the wrong (synthetic) identity instead of its real S04 arm key.

## Where

- `crates/roko-learn/src/self_model/policy.rs::cheap_arms` (assumption 1).
- `crates/roko-learn/src/self_model/replay.rs::arm_key`, `crates/roko-learn/src/self_model/mod.rs::ArmKey` and its
  `Display`/`FromStr` impls (assumption 2).
- The matrix-replay call path that would show whether assumption 2's fallback is reached for real, registered S08
  arm ids: backlog tasks 6120 ("replay routing policies prequentially on the benchmark's run...") and 6125 ("run
  the M3 replay on the pilot matrix...").
- `tmp/cybernetic-harness/specs/S04-*.md` (SC2/H4, "best static cheap arm") and
  `tmp/cybernetic-harness/specs/S09-experiments.md:190-195` (the arm registry table) — the spec text to confirm
  against.

## Current state

Assumptions 3 and 4 are already self-documented as open follow-ups in backlog task 6117's Notes (quoted above) —
no action needed from this item. Assumptions 1 and 2 are implemented as specific code choices with no matching
spec confirmation found; assumption 2 additionally needs tracing through 6120/6125 to know whether it's even
exercised the way described.

## Plan

1. Ask Will/S04 to confirm or correct assumption 1's median-cost definition of "cheap arm."
2. Trace 6120/6125's actual matrix-replay call path: for each S08 arm id in the registry table, does it reach
   `arm_key` with a string that already parses as the table's real S04 key, or with the bare S08 arm id (which
   would then fall back to the synthetic `roko/replay/<arm>`)? Confirm or fix.
3. If assumption 2's fallback is reached for real, registered arm ids, fix the replay path to look up the
   registry's real key first.

## Done when

- Will/S04 has confirmed (or corrected) the "cheap arm" median-cost definition, recorded as a decision.
- 6120/6125's matrix-replay path is confirmed to use each S08 arm id's real S04 arm key from the registry table,
  not the synthetic `roko/replay/<arm>` fallback, for arm ids that registry already names.
- The `[[verify]]` command passes.

## Notes

- Assumptions 3 and 4 (ladder constants, H4-BL naming) are not part of this item's scope — they're already
  tracked in backlog task 6117's own Notes and need no duplicate tracking here.
- This is a confirm-the-assumption item, not a confirmed bug: assumption 2 in particular needs the 6120/6125
  trace (step 2) before anyone can say whether it's actually wrong in practice.
