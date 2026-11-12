+++
id = "gap-10ffd4"
kind = "gap"
title = "roko learn all's router block shows two totals without saying why they differ: 25 observations in the header, 32 outcomes by model below (roko: 275 and 292), because outcomes recorded without a routing context reach the rows but not the router's observations"
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
anchors = ["crates/roko-cli/src/commands/learn.rs::router_detail_lines", "crates/roko-learn/src/cascade_router.rs::record_confidence_outcome"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-ea1576"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn learn_all_says_why_router_counts_differ' crates/roko-cli/src && cargo test -p roko-cli learn_all_says_why_router_counts_differ"
+++

## Problem

After gap-ea1576, `roko learn all` labels the cascade router's two counts apart, but it still doesn't say why they
differ. On a copy of round v7's Undertow workspace (roko b81f8a915, 2026-11-12):

```
Cascade router: 25 observations, 5 models at …/cascade-router.json
  …
  Outcomes by model (32 in all):
    gpt-5.4: 10 tries, 9 passed
    gpt-5.4-mini: 22 tries, 21 passed
```

On roko's own workspace the same lines read 275 and 292.

- **The header** counts LinUCB's observations: outcomes recorded with their task's routing context
  (`observe_internal`).
- **The rows** count every outcome in `confidence_stats`. That includes outcomes that runtime feedback and serve's
  dispatch record through `record_confidence_outcome`, which by design has no routing context and leaves LinUCB alone.
  `apply_calibration_correction` would add synthetic trials to the rows too, but nothing outside its tests calls it.

A reader sees two totals for one router and no reason for the gap.

## Why it matters

- The round v8 guides show `roko learn all` as the view of what roko learned, and the router block is its first block.
- Goal: learning.

## Where

- `crates/roko-cli/src/commands/learn.rs`: `router_detail_lines` and `print_learn_router`.

## Plan

1. **Explain the gap.** When the outcomes outnumber the observations, add one line under "Outcomes by model", for
   example: "(7 of the 32 were recorded without a routing context, so the router's 25 observations leave them out)".
2. **Otherwise the text stays as it is.**
3. **Test:** `learn_all_says_why_router_counts_differ`.

## Done when

- The `[[verify]]` passes.
- On a copy of Undertow's workspace, `roko learn all` says why 32 is not 25.

## Notes

- Found by roko-learnguard's round v8 after-survey (`$V8/roko-learnguard/survey-after/`).
- Follows gap-ea1576.
