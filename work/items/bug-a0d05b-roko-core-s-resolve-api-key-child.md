+++
id = "bug-a0d05b"
kind = "bug"
title = "roko-core's resolve_api_key child tests run no test: --exact with a bare test name matches nothing, and the parent checks only the exit status"
status = "open"
triage = "unverified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-core/config"]
created = 2026-10-16
updated = 2026-10-16
source = "tmp/hermes-roko-v5/findings/roko-credfix.md (roko-credfix, 2026-10-16)"
discovered_from = "bug-3c4600: roko-core lib test output showed two 'running 0 tests' child runs"
anchors = ["crates/roko-core/src/config/schema.rs::run_resolve_api_key_child"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

`crates/roko-core/src/config/schema.rs::run_resolve_api_key_child` re-runs the test binary as
`<exe> <test_name> --exact --nocapture`, with `test_name` a bare name such as `resolve_api_key_child_missing`.
`--exact` matches only a full test path (`config::schema::tests::resolve_api_key_child_missing`), so the child runs
**no test**. It prints `running 0 tests … 0 passed; … 2032 filtered out` and exits 0, and the parent asserts only the
exit status. So the two API-key resolution tests that use it (around `schema.rs:3471` and `3479`) check nothing.

Seen in `cargo test -p roko-core --lib` output on 2026-10-16: two `running 0 tests` blocks among the
`config::serve::tests` lines.

## Why it matters

A test that cannot fail hides a regression in `resolve_api_key`, which decides which provider key roko uses.

## Plan

Pass the full test path, or drop `--exact`. Also make the parent assert that the child ran one test, as
`crate::child_env::tests::a_check_without_its_environment_does_not_run` does: capture stdout and require
`1 passed`.

## Done when

Both children report `1 passed`, and breaking `resolve_api_key` makes the parent tests fail.

## Notes

Found by roko-credfix while fixing bug-3c4600 (hermes-roko-v5 round, 2026-10-16).
