+++
id = "bug-ea8c66"
kind = "bug"
title = "roko learn knowledge-stats (alias knowledge) always fails with \"unknown learning area 'knowledge'\""
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/learn"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "tmp/hermes-roko-v6/findings/learning-research.md §4 item 12 (learning-research, 2026-10-16)"
discovered_from = "hermes-roko-v6 round: make learning visible on the hub"
anchors = ["crates/roko-cli/src/commands/learn.rs::cmd_learn", "crates/roko-cli/src/commands/learn.rs::cmd_learn_json"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn learn_knowledge_stats_counts_the_store' crates/roko-cli && cargo test -p roko-cli --bin roko learn_knowledge_stats_counts_the_store"
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
