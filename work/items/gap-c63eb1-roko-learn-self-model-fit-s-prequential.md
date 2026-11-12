+++
id = "gap-c63eb1"
kind = "gap"
title = "roko learn self-model fit's prequential line says 'over all 129 units' while the fit trains on 139: it leaves out the failover substitutes without saying so, and the live-forecast line reads '1 scored against their verdicts'"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
source = "round-v8 roko-learnguard after-survey ($V8/roko-learnguard/survey-after, roko b81f8a915, a copy of Undertow's .roko)"
discovered_from = "agent:roko-learnguard (round v8)"
anchors = ["crates/roko-cli/src/commands/learn_self_model.rs::render_fit"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-45f6bb", "bug-a35286"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn the_prequential_line_counts_the_units_it_scores' crates/roko-cli/src && cargo test -p roko-cli the_prequential_line_counts_the_units_it_scores"
+++

## Problem

gap-45f6bb named the window each `roko learn self-model fit` score covers. Its prequential line overstates that
window, and its live-forecast line reads badly when only one forecast was made. On a copy of round v7's Undertow
workspace (roko b81f8a915, 2026-11-12):

```
units: 191 read, 52 duplicates dropped, 139 kept, 139 to train on
prequential, over all 129 units: Brier 0.126, base rate 0.752, Brier skill 0.323, …
```

- **"All 129" is 129 of the 139 units the fit trains on.** The prequential scores leave the 10 failover substitutes
  out: they teach the model, but nobody chose them, so they are not scored (bug-a35286). A reader sees 139 on one
  line and "all 129" on the next.
- **On roko's own workspace,** one forecast reads "1 scored against their verdicts".

## Why it matters

- The round v8 guides quote these two lines as what the self-model knows and what it said at the time.
- Goal: learning.

## Where

- `crates/roko-cli/src/commands/learn_self_model.rs`: `render_fit`.

## Plan

1. **Name the units scored.** When the fit trained on failover substitutes, the line reads "prequential, over 129 of
   the 139 units (the 10 failover substitutes train but aren't scored)". Otherwise it reads "over all N units".
2. **Make the live-forecast count read right for any n.** For example, "live forecasts, as the runs made them: 93 with
   a verdict, Brier …".
3. **Tests:**
   - `the_prequential_line_counts_the_units_it_scores`, new;
   - `the_fit_names_its_windows_and_scores_the_live_forecasts`, updated to the new text.

## Done when

- The `[[verify]]` passes.
- On a copy of Undertow's workspace, the prequential line says 129 of 139.

## Notes

- Found by roko-learnguard's round v8 after-survey (`$V8/roko-learnguard/survey-after/`).
- Follows gap-45f6bb.
