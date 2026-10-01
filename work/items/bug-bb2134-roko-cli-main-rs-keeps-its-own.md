+++
id = "bug-bb2134"
kind = "bug"
title = "roko-cli main.rs keeps its own mentions_http_401, a copy of roko-agent's"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/main"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-e03f92"
anchors = ["crates/roko-cli/src/main.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-e03f92"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test $(grep -rn 'fn mentions_http_401' crates/ --include=*.rs | wc -l) -le 1"
+++

## Problem

bug-28c193 added `mentions_http_401` to roko-cli `main.rs`, and bug-e03f92 added the same check in roko-agent. There are now two copies.

## Plan

Keep one: export roko-agent's, and use it from main.rs.

## Done when

- The verify passes.

## Notes

- Reported on 2026-10-01 by wk-model-truth, working on bug-e03f92, during the evening close-out round.
- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  main.rs's `error_hint` calls `roko_agent::provider::error_classify::mentions_http_401` (already `pub`), and
  main.rs's copy and its `is_http_status_word` are gone. roko-agent's version also counts "code" as a status word
  ("error code 401"); bug-28c193's `error_hint` tests still hold. The verify passes (run under bash).
