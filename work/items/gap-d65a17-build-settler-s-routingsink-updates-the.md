+++
id = "gap-d65a17"
kind = "gap"
title = "build_settler's RoutingSink updates the router without journaling, and build_settler still has no production caller"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/graph_execution/feedback"]
created = 2026-09-30
updated = 2026-10-01
last_verified = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-settle's report, checked on work/bug-f81e9b at 7db865c81)"
anchors = ["crates/roko-cli/src/graph_execution/feedback.rs::build_settler"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["gap-2ce86f", "bug-84de98"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'struct RoutingSink' crates/roko-cli/src/graph_execution/feedback.rs || (grep -rqw 'fn routing_sink_journals_its_observations' crates/roko-cli/src/ && cargo test -p roko-cli --lib routing_sink_journals_its_observations)"
+++

## Problem

`build_settler` (`crates/roko-cli/src/graph_execution/feedback.rs:33` on the settle branch) still has no production caller; only its tests call it (:1016, :1049). gap-2ce86f got the error patterns written another way. Its `RoutingSink` (:60, :366) updates the cascade router directly, without the journal the other router writers use. If anyone wires `build_settler`, those observations bypass crash recovery and double-count protection (bug-84de98).

## Why it matters

Cybernetic core (epic spec-6ac537): dead code that would reintroduce a fixed bug if wired. p3.

## Where

`build_settler` and `RoutingSink` in `graph_execution/feedback.rs`.

## Plan

Pick one:

- **(a)** Delete `build_settler` and its sinks, if gap-2ce86f's path covers what they did.
- **(b)** Make `RoutingSink` write through the journal, and add `routing_sink_journals_its_observations`.

## Done when

- [ ] `RoutingSink` is gone, or it journals.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/bug-8a78e1` at `6b2118da1` (option a); cargo verification deferred to the batch check.
  `graph_execution/feedback.rs` is deleted: `build_settler`, its 12 sinks and `CompletionSinkResult`. Only their own
  tests called them. The feedback facade and dispatch's records cover what the rows did on the Graph path.
