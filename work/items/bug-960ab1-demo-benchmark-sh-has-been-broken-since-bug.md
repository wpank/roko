+++
id = "bug-960ab1"
kind = "bug"
title = "demo-benchmark.sh has been broken since bug-28becc: its oracle reads the gold patch from the agent payload, and it expects control rows and learning writes"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["demo/demo-resources"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report)"
anchors = ["demo/demo-resources/benchmark-flow/demo-benchmark.sh", "demo/demo-resources/benchmark-flow/README.md"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-28becc", "bug-a445eb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qF 'sys.stdin)[' demo/demo-resources/benchmark-flow/demo-benchmark.sh"
+++

## Problem

`demo/demo-resources/benchmark-flow/demo-benchmark.sh` runs positive and negative controls through `roko bench swe` (:29-33). Its "oracle" command agent prints the agent payload's `patch` field (:36: `json.load(sys.stdin)["patch"]`). bug-28becc stopped leaking the gold patch into the payload, so the oracle has nothing to print. The script also expects control rows in `.roko/bench/scores.jsonl` (:42), and learning writes for the empty control, which bug-28becc changed (reported by wk-honestbench).

## Why it matters

Release: the demo of the benchmark flow fails, and a demo that passes only by reading the answer was never a fair demo.

## Where

The script and its README.

## Plan

1. Use `--agent-mode gold` (an explicit flag since bug-32d57f) for the positive control, instead of reading the patch from the payload.
2. Update the expected outputs to bug-28becc's records: where controls are written, and what an empty control learns.
3. Run the script end to end.

## Done when

- [ ] The script passes against the current bench.
- [ ] The `[[verify]]` command passes (static; also run the script).
