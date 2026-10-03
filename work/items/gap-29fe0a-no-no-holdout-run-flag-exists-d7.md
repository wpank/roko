+++
id = "gap-29fe0a"
kind = "gap"
title = "No --no-holdout run flag exists; D7's decided escape hatch is only the sticky holdout_frac config field"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/main"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-5 follow-up reports 2026-10-02 (PK19 gap-de0b87)"
discovered_from = "gap-de0b87 (D7, tmp/cybernetic-harness/DECISIONS.md)"
anchors = ["crates/roko-core/src/config/spec_quality.rs::SpecQualityConfig"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn no_holdout_flag_draws_no_holdout_attempts' crates/roko-cli/ && cargo test -p roko-cli no_holdout_flag_draws_no_holdout_attempts"
+++

## Problem

`tmp/cybernetic-harness/DECISIONS.md`'s D7 row ("Randomization on your real work") decides: "On, at S03's holdout
rates, with `--no-holdout` for must-succeed runs." No `--no-holdout` flag exists on `roko run` or `roko plan run`
(checked `crates/roko-cli/src/main.rs`'s CLI arg definitions and `crates/roko-cli/src/commands/`: no match for
`no-holdout`/`no_holdout` anywhere in the CLI surface). The only way to disable holdout sampling today is to edit
`roko.toml`'s `[spec_quality] holdout_frac` (`crates/roko-core/src/config/spec_quality.rs:60-61`, default per
`default_holdout_frac()` at line 94) down to `0.0`, or use `roko config set`.

## Why it matters

D7's own rationale is "Makes loops measurable at a small pass-rate cost" — the holdout rate is deliberately *on*
by default so S03's loop-liveness measurement works, with an *explicit, per-run* escape hatch for runs that must
not pay that cost (e.g. a real deliverable, not a measurement run). A config-file edit is not a per-run escape
hatch: it's sticky (affects every subsequent run until reverted) and easy to forget to revert, which works against
exactly the case D7 carries the flag for.

## Where

- `crates/roko-cli/src/main.rs` — where `roko run`/`roko plan run`'s CLI flags are defined; no holdout-related
  flag exists.
- `crates/roko-core/src/config/spec_quality.rs::SpecQualityConfig::holdout_frac` (line 61) — the only existing
  control, config-level and sticky.
- `tmp/cybernetic-harness/DECISIONS.md` D7 (line 62) — the decision this item fulfills.

## Current state

No per-run flag. `holdout_frac = 0` in `roko.toml` is the sole workaround, and it is global/sticky, not scoped to
one run.

## Plan

1. Add `--no-holdout` to `roko run` and `roko plan run`'s CLI args (`main.rs`).
2. Thread it through to wherever holdout sampling is drawn per attempt/task (search for `holdout_frac` reads in
   the dispatch path) as a per-run override that forces the effective rate to `0.0` for this run only, without
   mutating the loaded config or `roko.toml`.
3. Test: a run started with `--no-holdout` draws no holdout attempts even when `roko.toml` sets a nonzero
   `holdout_frac`; a run without the flag still uses the configured rate.

## Done when

- `roko run --no-holdout` / `roko plan run --no-holdout` exist and make the run draw no holdout attempts, without
  changing `roko.toml`.
- The `[[verify]]` command passes.

## Notes

- Keep `holdout_frac` itself as the config-level default; this adds a per-run override, not a replacement.
- Related: gap-b5caf3 (done, PK32 — retiring the *legacy* holdout gate that gates nothing) and gap-943046 (open,
  section-bandit withhold arms) are about different holdout-adjacent mechanisms; neither adds this flag.
