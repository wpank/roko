+++
id = "bug-ea8c66"
kind = "bug"
title = "roko learn knowledge-stats (alias knowledge) always fails with \"unknown learning area 'knowledge'\""
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/learn"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "f870523c0"
source = "tmp/hermes-roko-v6/findings/learning-research.md §4 item 12 (learning-research, 2026-10-16)"
discovered_from = "hermes-roko-v6 round: make learning visible on the hub"
anchors = ["crates/roko-cli/src/commands/learn.rs::cmd_learn", "crates/roko-cli/src/commands/learn.rs::cmd_learn_json"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn learn_knowledge_stats_counts_the_store' crates/roko-cli && cargo test -p roko-cli --bin roko learn_knowledge_stats_counts_the_store"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T21:14:58Z"
commit = "f870523c0"
by = "commit trailer"
executor = "unknown"
forced = false
evidence = "f870523c0: cmd_learn and cmd_learn_json accept knowledge (one LEARN_AREAS list, which the error names), and the view counts KnowledgeStore::stats by tier and source with the average confidence (--json: total_entries, by_tier, by_source, average_confidence). Verify: learn_knowledge_stats_counts_the_store passes (roko bin test). Checked as a stack on f870523c0 (main 8357603f0 + gap-655ea5 + gap-de0e96 + bug-ea8c66): roko-cli lib 3738 passed (6 ignored, gist skipped), roko-serve lib 1047, roko-core lib 2035, the roko bin's learn test, clippy -D warnings clean on roko-core, roko-cli and roko-serve, fmt, plan_branch_integration (4) and connected_efficiency_render (1), with a scratch HOME. On the showcase workspace `roko learn knowledge-stats` prints 1 durable entry, by tier consolidated 1, by source runtime:gate_verdict 1, average confidence 0.934, exit 0; main 47b3cdd20 failed it with \"unknown learning area 'knowledge'\" (exit 2)."
+++

## Problem

`roko learn knowledge-stats` (alias `roko learn knowledge`), listed in `--help` and CLAUDE.md as "Show durable
knowledge entry counts", always fails:

```
$ roko learn knowledge-stats
Error: unknown learning area 'knowledge'. Available: router, experiments, efficiency, episodes, reflexes, gates, all
```

`LearnCmd::KnowledgeStats` calls `cmd_learn(&wd, "knowledge")` (and `cmd_learn_json` with `--json`), which accept
only the six areas above and `all`; the knowledge view runs only under `all`, where it counted the store's JSON
lines rather than its entries (learning-research §4 item 12).

## Why it matters

Goal `hermes`: the learning showcase's "after" view (`snap.sh`) and anyone checking what roko kept use it; it is the
one CLI view of the lessons a run stored.

## Where

- `crates/roko-cli/src/commands/learn.rs::cmd_learn`, `::cmd_learn_json`: the area dispatch.
- `::print_learn_knowledge`, `::collect_knowledge_json`: the knowledge view.

## Plan

1. Accept `knowledge` in both paths (one list of areas, which the error names).
2. Count the store's entries with `KnowledgeStore::stats`, by tier and source, with the average confidence.

## Done when

The `[[verify]]` test passes: the text and JSON views of `knowledge` succeed and count the store, and an unknown area
still fails.
