+++
id = "bug-8417d9"
kind = "bug"
title = "Thirteen JSONL appenders write each row and its newline in two writes, so concurrent appends interleave and lose rows"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-learn", "roko-neuro", "roko-agent", "roko-core", "roko-acp", "roko-dreams"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-specq's report, checked on work/bug-779ae7 at 101afcda1)"
anchors = ["crates/roko-learn/src/tool_metrics_store.rs", "crates/roko-learn/src/run_metrics.rs", "crates/roko-core/src/forensic.rs", "crates/roko-neuro/src/admission.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["bug-779ae7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rqF --include='*.rs' 'writeln!(file, \"{line}\")' crates/ && grep -rqw 'fn concurrent_jsonl_appends_never_interleave' crates/roko-core/src/ && cargo test -p roko-core --lib concurrent_jsonl_appends_never_interleave"
+++

## Problem

These JSONL appenders open their file in append mode and write a row with `writeln!(file, "{line}")`, for example:

- `crates/roko-learn/src/tool_metrics_store.rs:109`, `run_metrics.rs:78`;
- `crates/roko-core/src/forensic.rs:374`;
- `crates/roko-neuro/src/admission.rs:1095`.

On a bare `File`, `writeln!` issues separate `write` calls for the row and the newline. Two processes, or two tasks, appending at once can interleave between them: two rows end up on one line, a newline lands elsewhere, and the readers drop the malformed lines. So rows are lost.

wk-specq fixed the same bug in `append_jsonl_line_async` (bug-779ae7's branch) and lists the rest:

- roko-learn: `tool_metrics_store`, `run_metrics`, `generational_metrics`, `telemetry/report`;
- roko-neuro: `tier_progression`, `lifecycle`, `admission`;
- roko-agent: `safety/provenance`, `witness`;
- roko-core: `forensic`;
- roko-acp: `bridge_events/cost`;
- roko-dreams: `phase2/advanced`;
- one roko-cli integration test.

At 739ee8266, `writeln!(file, "{line}")` appears 16 times under `crates/`.

## Why it matters

One settled record per attempt (epic spec-b7303f): parallel tasks append to the same learning and telemetry files. Lost rows mean lost records, silently.

## Where

Each appender above.

## Plan

1. Add one shared helper (for example in roko-core) that serializes the row, appends `\n` in memory, and writes the whole line with one `write_all` on an append-mode file. Use it everywhere.
2. Add `concurrent_jsonl_appends_never_interleave`: several threads append at once, and every line parses.

## Done when

- [ ] Every JSONL appender writes each line with one write.
- [ ] The `[[verify]]` command passes.
