+++
id = "bug-904a1c"
kind = "bug"
title = "roko-serve's JWKS proactive-refresh test races the cache under load: it waits for the server's hit, then asserts the key is stored"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-serve/jwks"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "hermes-roko round v5, roko-chathost, 2026-10-15"
anchors = ["crates/roko-serve/src/jwks.rs::proactive_refresh_task_fetches_without_validation_traffic"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

`roko-serve`'s `jwks::tests::proactive_refresh_task_fetches_without_validation_traffic` fails when the machine is
busy. Seen 2026-10-15 20:41 (1-minute load about 150, `cargo test -p roko-core -p roko-learn -p roko-serve -p
roko-cli --lib`): `assertion left == right failed, left: 0, right: 1` at `crates/roko-serve/src/jwks.rs:878`. The
same test passed in the same worktree 50 minutes earlier, at a lower load.

It is intermittent, not load-only: running the built test binary 20 times in a row (`--exact`, load about 40)
failed 3 times at `main` 5875fa441 and 5 times with gap-115bae's changes, which do not touch `jwks.rs`.

The test starts the proactive refresh task with a 10 ms interval, waits until the test JWKS server has counted a
request (`hits > 0`), then asserts that the cache holds one key. But the server counts the request when it
arrives, before the cache has read and stored the response (`#[tokio::test]` runs the server, the refresh task
and the test body on one thread, so which runs first after the request is a matter of scheduling), and the cache's
client timeout in the test is 250 ms (`cache_for`). Under load the refresh can still be in flight, or can time out ("proactive JWKS refresh failed;
retaining stale keys"), when the assertion runs.

## Why it matters

A flaky test in a crate every Rust worker tests makes passing work look broken and costs a rerun, which under the
round's cargo limits can wait 10 minutes for a slot.

## Where

`crates/roko-serve/src/jwks.rs::tests::proactive_refresh_task_fetches_without_validation_traffic` and its
`cache_for` helper.

## Current state

Open at `5875fa441`.

## Plan

Wait, within the test's timeout, until `cache.keys_count().await == 1` instead of until the server has a hit, and
give this test's cache a client timeout that a loaded machine meets (a few seconds).

## Done when

The test polls the cache, not the hit counter, and passes ten runs in a row while another crate builds.
