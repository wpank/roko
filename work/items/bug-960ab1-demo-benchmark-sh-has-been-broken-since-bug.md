+++
id = "bug-960ab1"
kind = "bug"
title = "demo-benchmark.sh has been broken since bug-28becc: its oracle reads the gold patch from the agent payload, and it expects control rows and learning writes"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["demo/demo-resources"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "119cb831b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report)"
anchors = ["demo/demo-resources/benchmark-flow/demo-benchmark.sh", "demo/demo-resources/benchmark-flow/README.md"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-28becc", "bug-a445eb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qF 'sys.stdin)[' demo/demo-resources/benchmark-flow/demo-benchmark.sh"

[closed]
at = 2026-09-30
commit = "119cb831b"
by = "commit trailer"
evidence = "119cb831b: gold and empty are labeled controls, read from controls.jsonl and checked to write no learning state; the command oracle (oracle-agent.py) replays the gold control's --export-predictions file by instance_id and fails if the payload carries hidden fields. Verify passes; the script ran end to end (exit 0, fresh and reused workspace) against the debug build at f0b8b037d, whose bench matches BASE 4cf2e329b."
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

- [x] The script passes against the current bench.
- [x] The `[[verify]]` command passes (static; also run the script).

## Notes

- **wk-honestbench (2026-09-30):** Ran `demo-benchmark.sh` with `ROKO` set to an APFS clone of the coordinator's debug build at `f0b8b037d` (from `roko-work-gap-a8d786`, built 2026-09-30 13:26; its bench code matches BASE `4cf2e329b`): exit 0 on a fresh workspace and again on the same one. MAIN's `target/debug/roko` (2026-09-29 13:07) predates bug-28becc; the script now fails against it with "a control wrote learning state" instead of passing.
