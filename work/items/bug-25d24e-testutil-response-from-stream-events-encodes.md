+++
id = "bug-25d24e"
kind = "bug"
title = "testutil::response_from_stream_events encodes cache reads under a key the parser never reads, so three streaming parity tests are ignored"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-agent/testutil"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "5c44d75d6"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-runstate's report, checked on work/bug-4c4eea at 3cb4a818f)"
anchors = ["crates/roko-agent/src/testutil.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["bug-afcf63"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn streamed_and_batch_usage_agree_on_cache_reads' crates/roko-agent/src/ && cargo test -p roko-agent --lib streamed_and_batch_usage_agree_on_cache_reads"
+++

## Problem

`testutil::response_from_stream_events` (`crates/roko-agent/src/testutil.rs`) builds the batch response a stream should agree with. It writes cache reads under a key (`"cached_tokens"`, :884 on the runstate branch) that the usage parser never reads. The streamed and batch usage then disagree on cache reads, and three streaming parity tests are `#[ignore]`d because of it (wk-runstate).

## Why it matters

One settled record per attempt (epic spec-b7303f): the parity tests are the guard that streamed and batch dispatch record the same usage. Ignored, they guard nothing. p3.

## Where

`response_from_stream_events` and the ignored parity tests in roko-agent.

## Plan

1. Encode cache reads where the parser reads them (for example `prompt_tokens_details.cached_tokens`), and un-ignore the three tests.
2. Add `streamed_and_batch_usage_agree_on_cache_reads`, or rename one of the three to it.

## Done when

- [ ] The parity tests run, and pass, with cache reads.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/bug-739dcc` at `5c44d75d6`; cargo verification deferred to the batch check. `streamed_and_batch_usage_agree_on_cache_reads` (targeted `cargo test` passed). `response_from_stream_events` encodes the usage with `translate::openai::usage_to_wire`. Not done: the three streaming parity tests stay ignored. With the usage fixed they fail on the next check, "streamed session metadata mismatch", because stream events carry no response, session or thread id (follow-up: carry them as bug-bfd241 carries the model). Their ignore reason now says so.
