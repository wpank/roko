+++
id = "dec-01be49"
kind = "decision"
title = "Decide how the doctor tests stop depending on the machine's claude and API keys: injectable probes or relaxed assertions"
status = "open"
triage = "unverified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/doctor"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-ci2's report on find-8cc7ac)"
anchors = ["crates/roko-cli/src/doctor.rs::check_provider_usable", "crates/roko-cli/src/doctor.rs::run_doctor"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["find-8cc7ac"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "env -i HOME=\"$(mktemp -d)\" CARGO_HOME=\"$HOME/.cargo\" RUSTUP_HOME=\"$HOME/.rustup\" PATH=\"$HOME/.cargo/bin:/usr/bin:/bin\" cargo test -p roko-cli --lib doctor::tests::"
+++

## Problem

Two doctor tests fail on any machine without `claude` on PATH and without provider keys, which includes Linux CI (find-8cc7ac, item 3):

- `doctor::tests::run_doctor_passes_bootstrapped_workspace_without_serve_probe` (`crates/roko-cli/src/doctor.rs:3427`);
- `doctor::tests::doctor_skips_mcp_allowlist_when_no_config` (:4007).

Both fail with `assertion failed: report.healthy`. wk-ci2 traced this to the machine-dependent checks the reporter names `shared_credentials_none` and `provider_usable`.

`check_provider_usable` (:670) fails when `detect_auth_from_config` finds no working auth, and `healthy` is `summary.fail == 0` (:283). The tests assert a healthy report, so they assert something about the machine, not about the code.

## Why it matters

Hygiene (epic spec-9a3131): CI stays red for reasons unrelated to the change under test, which trains everyone to ignore CI.

## Decision

The two options:

- **(a) Injectable probes.** `DoctorOptions` gets the environment probes (auth detection, the shared-credentials lookup) as a small struct or trait, which defaults to the real environment. The tests inject "claude available" or "no auth" and assert both outcomes. The tests stay meaningful and deterministic, at a small API change.
- **(b) Relaxed assertions.** The tests assert only the checks they target (the MCP allowlist skip, the bootstrap checks), not `report.healthy`. This is quicker, but the tests no longer prove that a fully set-up workspace is healthy, and the next machine-dependent check can break them again.

**Recommended default: (a).** It fixes the cause, lets the provider checks themselves be tested, and keeps `report.healthy` meaningful.

## Where

`run_doctor`, `check_provider_usable` and the shared-credentials check in `crates/roko-cli/src/doctor.rs`, and the two tests.

## Done when

- [ ] Will (or the item's owner, if Will delegates) picks (a) or (b).
- [ ] The doctor tests pass with no `claude` on PATH and no keys: the `[[verify]]` command runs them with an empty HOME and a minimal PATH.
