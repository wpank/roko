+++
id = "bug-234204"
kind = "bug"
title = "Something runs roko with the temp directory itself as its workspace: a stray /private/tmp/.roko (roko.log: roko plan generate with cwd /private/tmp) failed 9 tui tests for any TMPDIR under /private/tmp; the writer, likely a test, is not found yet"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/tests"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
source = "round v8 lead and roko-recovery, 2026-11-19: the stray directory, moved by the lead"
discovered_from = "agent:roko-recovery (round v8, 2026-11-19)"
anchors = ["crates/roko-cli/tests/safety_funnel.rs", "crates/roko-core/src/child_env.rs::check_gh_config_dir"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-0e081d"], supersedes = [], duplicate_of = "" }
+++

## Problem

A `.roko` workspace appeared directly in `/private/tmp`, the root of the default temp directory on macOS. Its
`roko.log` showed `roko plan generate` running with `/private/tmp` as its working directory, at 12:00:49Z on
2026-11-19. Its `.roko/immune/tool-controls.json.lock` was dated 2026-11-18 13:36, so something wrote there on two
days. The likely writer is a test that uses the temp directory itself as its workspace, instead of a temp directory of
its own.

While it was there, 9 of roko-cli's tui tests failed for any TMPDIR under `/private/tmp`. A test workspace under such a
TMPDIR found the stray `.roko` above it. The round v8 lead moved the directory on 2026-11-19. CI is not affected.

## Why it matters

Goal tooling. A test that writes outside its own temp directory leaves state that later tests, and later runs, pick
up. Here it made a correct test suite fail on one machine.

## Ruled out (2026-11-19, main c4f45d4eb)

Each suite below ran with an empty TMPDIR of its own, and only what it left behind was listed. None left a `.roko`
in TMPDIR itself:

- roko-cli's `tests/safety_funnel.rs`, whose RunConfig test uses `std::env::temp_dir()` as its workdir. It left
  nothing.
- roko-std's lib tests, whose bash-tool tests use `ToolContext::testing(std::env::temp_dir())`. They left nothing.
- roko-gate's lib tests. They left only the check environment's empty, read-only `roko-check-gh-<pid>-…` directory.
  One is made per process (`roko_core::child_env::check_gh_config_dir`) and never removed. That is by design, but it
  piles up in TMPDIR.
- roko-acp's lib tests. They left only the temp directory that bug-0e081d describes. That is a `.tmpXXXX/.roko`, not
  a `.roko` in TMPDIR itself.

## Plan

1. Run the remaining suites the same way, with an empty TMPDIR each and a check for `$TMPDIR/.roko` afterwards:
   - roko-cli's lib, bin and integration tests;
   - the other crates;
   - the Hermes plugin's pytest, which starts roko.
2. Give the test that writes it a temp directory of its own.

## Done when

No suite leaves a `.roko` in its TMPDIR. A check that fails when one does is wired into the test harness.
