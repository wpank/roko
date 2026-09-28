+++
id = "bug-121c35"
kind = "bug"
title = "Hindsight Regression Detection Uses Temporal Ordering, Not Data-Flow"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/92-hindsight-causal-inference.md#92 — Hindsight Regression Detection Uses Temporal Ordering, Not Data-Flow"
discovered_from = "audit:tmp/backlog/archive/92-hindsight-causal-inference.md#92 — Hindsight Regression Detection Uses Temporal Ordering, Not Data-Flow"
anchors = ["crates/roko-learn/", "src/hindsight.rs", "src/episode_logger.rs", "crates/roko-learn/src/hindsight.rs", "crates/roko-learn/src/episode_logger.rs", "mod.rs", "HindsightRelabeler::scan", "Episode::new"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
False-positive regressions in parallel plans corrupt the learning signal; episodes are penalized for failures they did not cause. Roko's learning system records episodes for every agent task and uses a "hindsight relabeler" to retroactively adjust episode outcomes. The relabeler's job is to detect…

Imported without verification from:
- `tmp/backlog/archive/92-hindsight-causal-inference.md#92 — Hindsight Regression Detection Uses Temporal Ordering, Not Data-Flow`

How to verify: Check: Two concurrent successful and failing episodes that share only a noise file (e.g. `Cargo.toml`) do not produce a regression adjustment.; Two episodes connected by a signal-hash chain (`earlier.output_signal_hash == later.input_signal_hash`)… [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): M | 5 |]
