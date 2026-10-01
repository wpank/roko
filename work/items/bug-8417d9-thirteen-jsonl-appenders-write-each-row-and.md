+++
id = "bug-8417d9"
kind = "bug"
title = "Thirteen JSONL appenders write each row and its newline in two writes, so concurrent appends interleave and lose rows"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-learn", "roko-neuro", "roko-agent", "roko-core", "roko-acp", "roko-dreams"]
created = 2026-09-30
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "1bf49188d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-specq's report, checked on work/bug-779ae7 at 101afcda1)"
anchors = ["crates/roko-learn/src/tool_metrics_store.rs", "crates/roko-learn/src/run_metrics.rs", "crates/roko-core/src/forensic.rs", "crates/roko-neuro/src/admission.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["bug-779ae7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rqF --include='*.rs' 'writeln!(file, \"{line}\")' crates/ && grep -rqw 'fn concurrent_jsonl_appends_never_interleave' crates/roko-core/src/ && cargo test -p roko-core --lib concurrent_jsonl_appends_never_interleave"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T09:00:14Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20c gate on fcdaf32ae/ca5645373 (MAIN 1bf49188d has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/gate/learn/neuro/serve; lib tests roko-cli 3273, roko-agent 2278, roko-core 1962, roko-learn 1209, roko-serve 989, roko-gate 692, roko-neuro 239, roko-acp 199, roko-dreams 100 all pass; extras: C1 1/1, C7 2/2, learn_paths 7, cost_comparison 1, bin 429, verify loop 10/10, speclint 91, including concurrent_jsonl_appends_never_interleave and the one-write recording test. Merged e3df791b8 (work/bug-8417d9 348cfcf22)."
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

## Notes

- Implemented on `work/bug-8417d9` at `2587848e4`; cargo verification deferred to the batch check. Targeted
  tests ran in wk-specq's own target clone.
- 2026-10-01 (wk-specq): `roko_core::io` gains `write_jsonl_line`, `append_jsonl_line` and `append_jsonl`.
  Each writes the row and its newline with one `write_all`, and every appender listed above uses one of them.
  - Two appenders the list missed: the TUI's inject and confirm signals (`tui/app/actions.rs`), which go to
    `.roko/signals.jsonl`. They use the helper too.
  - The review log (`commands/plan.rs`) and its test helper (`attempt_workspace.rs`) write the row with one
    `write_all`, so each hunk is one line beside wk-tiers' edits there.
  - Left as they are: roko-runtime's `state_hub` and `metrics`, and roko-fs's `classified_writer`, write through a
    `BufWriter`, which flushes whole rows unless one exceeds its 8 KiB buffer. roko-fs's `append_jsonl_line_sync`
    writes a missing newline separately, but under its own advisory lock.
  - Tests: `a_jsonl_row_and_its_newline_are_one_write` (a recording writer sees one write per row) and
    `concurrent_jsonl_appends_never_interleave` (16 threads append 50 rows each; every line parses). The tests
    of every touched module pass. On bug-779ae7's branch, a 64-writer check lost rows in 20 of 20 rounds with
    split writes and in 0 of 20 with one write per row.
