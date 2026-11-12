+++
id = "gap-ea1576"
kind = "gap"
title = "roko learn all overstates what roko learned: it counts retired knowledge as durable (Undertow's 62 entries include 12 retired), its router header and rows count different things (25 observations against 32 trials), its Compounding block mixes the provider's prompt cache and the first model's pass rate in with learning rates and shows an error-dedup rate that no record can feed, and its reflex line says run tasks while reflexes are off"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-cli/commands", "roko-learn/aggregate"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
source = "round-v8 roko-learnguard survey ($V8/roko-learnguard/survey, roko a0089c19c, a copy of Undertow's .roko)"
discovered_from = "agent:roko-learnguard (round v8)"
anchors = ["crates/roko-cli/src/commands/learn.rs::print_learn_router", "crates/roko-cli/src/commands/learn.rs::format_compounding_metrics", "crates/roko-learn/src/aggregate.rs::compute_compounding_metrics"]
links = { depends_on = [], blocks = [], related = ["bug-f8655e", "bug-dd5dca", "gap-8752ce"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn learn_all_says_what_it_measures' crates/roko-cli/src && cargo test -p roko-cli learn_all_says_what_it_measures"
+++

## Problem

`roko learn all` is the one command that shows what roko has learned, and it overstates it. On a copy of v7's
Undertow workspace (2026-11-12, roko a0089c19c):

- **Retired knowledge counts as durable.** It reports "Knowledge: 62 durable entries … distiller 14", and 12 of those
  14 were retired with `roko knowledge retire` (bug-f8655e). Retrieval skips retired entries, so they aren't durable
  knowledge. It doesn't show retirement at all.
- **The router's header and rows count different things.** The header says "25 observations", the LinUCB count. The
  model rows sum to 32 trials, from `confidence_stats`. It also lists every configured model, 0-trial ones
  included: 21 of 23 in the roko repo. "Range: n/a" and "Latest: snapshot stage=static total_observations=25" don't
  tell a reader that the router is still static, and switches to its confidence stage at 50 observations.
- **The "Compounding" block mixes rates that aren't learning with those that are:**
  - "cache hit rate 93.4%" is the provider's prompt cache;
  - "routing accuracy 54.8%" is the share of attempts whose first model passed (`initial_model ==
    successful_model` and a pass), not whether a route was right;
  - "error dedup rate 0.0%" can't be anything else: none of Undertow's 114 failed gate verdicts records a
    `signature`, so there is nothing to match.
- **The reflex line gives the wrong next step.** It says "(no rules yet; run tasks to build reflex history)", but
  `[learning] t0_reflexes` is off by default, and then running tasks builds nothing.
- **Every section opens with a `path:` line**, an absolute path, before its numbers.

## Why it matters

- Round v8's roko-for-Hermes guide shows how roko gets better the more it is used, from these views. Each number it
  shows has to mean what it says.
- Goal: learning.

## Where

- `crates/roko-cli/src/commands/learn.rs`: `print_learn_router`, `learn_router_model_rows`,
  `format_compounding_metrics`, the knowledge section (`Knowledge: {} durable entries`) and the reflex section. The
  `--json` structs `LearnJsonKnowledge` and `LearnJsonReflexes` are there too.
- `crates/roko-learn/src/aggregate.rs::compute_compounding_metrics`: the metric definitions, which stay as they are.
- The retirement ledger: `crates/roko-neuro/src/knowledge_store/retire.rs`.

## Plan

1. **Knowledge:** count live entries only, and show retired ones separately ("50 live, 12 retired"). The JSON gains
   `retired`, and its tier and source counts cover live entries.
2. **Router:**
   - one count, named for what it is, in the header and the rows: trials per model, with the LinUCB observations as
     their own line;
   - models with no trial on one line;
   - the stage in words, and the threshold of the next one;
   - no "Range" when no stage transition happened.
3. **Compounding:**
   - group the rates into learning (knowledge reuse, playbook hits, gate pass rate, cost per success) and
     provider-side (prompt-cache hit rate);
   - name "routing accuracy" for what it measures ("first model passed");
   - show error dedup as "n/a: no failure signatures recorded" when none is recorded.

   The JSON field names don't change.
4. **Reflexes:** with `t0_reflexes` off, say it's off and name the key.
5. **The path lines:** behind `--verbose`, or one line at the end.
6. **Test:** `learn_all_says_what_it_measures`. It covers a store with retired entries, a router with both counts, a
   window with no failure signatures and reflexes off.

## Done when

- The `[[verify]]` passes.
- `roko learn all` on a copy of Undertow's workspace shows 50 live and 12 retired knowledge entries, one router
  count, and no learning rate that isn't one.

## Notes

- Found by roko-learnguard's round v8 survey: `$V8/findings/roko-learnguard.md` and `$V8/roko-learnguard/survey/`.
- Related: bug-f8655e (the retirements), bug-dd5dca (why 27 of the live entries are at confidence 0), and
  gap-8752ce.
