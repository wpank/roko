+++
id = "bug-17e9ac"
kind = "bug"
title = "roko-cli's share_run_scrubs_output_text_and_prompt publishes a public gist with the operator's gh login on every local test run, so it has been skipped since 2026-10-08 and the scrubbing it checks goes untested"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/share"]
created = 2026-10-27
updated = 2026-10-28
last_verified = 2026-10-28
last_verified_rev = "ed035ca9a"
source = "Will's call, 2026-10-27 (found 2026-10-08)"
discovered_from = "agent:roko-exact (round v7, 2026-10-08 test run)"
anchors = ["crates/roko-cli/src/share.rs::upload_gist", "crates/roko-cli/src/share.rs::share_run"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn share_run_scrubs_output_text_and_prompt' crates/roko-cli/src && cargo test -p roko-cli --lib -- share::tests::share_run_scrubs_output_text_and_prompt"

[closed]
at = 2026-10-28
at_ts = "2026-10-28T09:59:00Z"
commit = "ed035ca9a"
by = "roko-exact (round v7)"
executor = "claude-agent"
via = "manual"
model = "claude-opus-5-5"
forced = false
evidence = "ed035ca9a: share_run calls share_run_via(Path::new(\"gh\"), ...), and share_run_via and upload_gist take the gh program they run; share_run_scrubs_output_text_and_prompt passes a stub gh at an absolute path in its own temp dir (records each call's arguments, copies the file it would upload, prints a fake gist URL) and asserts the stub's URL, its arguments (--version, then gist create --public ...) and the scrubbing on what would be uploaded; a_missing_gh_never_reaches_another_gh pins that a missing gh gives a local share, never a PATH lookup. Verify: share::tests:: 14/14 behind the gh fence (chain42; fmt and clippy -D warnings clean on roko-cli), and 14/14 again with the operator's logged-in gh on PATH and the real HOME (gh auth status 0): gh gist list counted 643 before and 643 after, so no gist was created (tmp/hermes-roko-v7/roko-exact/logs/proof-gist.log)."
+++

## Problem

roko-cli's test `share::tests::share_run_scrubs_output_text_and_prompt` calls `share::share_run`. That uploads the
transcript through `gh gist create --public` with whatever `gh` login the machine has. So every local roko-cli test
run on a machine where `gh` is logged in publishes a public gist under the operator's account.

The test has been excluded from every run (`--skip share_run_scrubs_output_text_and_prompt`) since it was found
on 2026-10-08. As a result, the scrubbing it checks has not run in any suite since then.

## Why it matters

- A test must never publish anything, least of all a public gist with the operator's identity.
- The skip hides the test, so a regression in what `share_run` uploads goes unseen.

## Where

- `crates/roko-cli/src/share.rs`:
  - `share_run` saves the transcript, then calls `upload_gist`.
  - `upload_gist` runs `Command::new("gh")`, first `gh --version`, then `gh gist create --public`.
- The test is `share_run_scrubs_output_text_and_prompt` in the same file. The other share tests render or scrub
  text and spawn nothing.

## Plan

- Give `share_run` a seam: the `gh` program it runs. `share_run` keeps running `gh`. The test passes the absolute
  path of a stub script in its own temp dir. The stub records its arguments and the uploaded file, and prints a fake
  gist URL.
- The test asserts on what would be uploaded: the secrets are gone and `[REDACTED]` is there. It also asserts the
  stub's URL came back and its arguments were `gist create --public …`.
- The program path is absolute and has no fallback. A missing stub fails the spawn, `share_run` returns its local
  result, and the test fails on its URL and backend assertions. It can't reach the real `gh`.

## Design

roko-exact, 2026-10-28.

- **The seam.** `share_run` calls `share_run_via(Path::new("gh"), …)`. `share_run_via` and `upload_gist` take the
  `gh` program to run, for both `gh --version` and `gh gist create --public`. Production is unchanged.
- **`share_run_scrubs_output_text_and_prompt`.**
  - It writes the stub `STUB_GH` into its own temp dir and passes that absolute path.
  - The stub appends each call's arguments to `gh-args`, copies the file `gist create` would upload to
    `gh-uploaded.md`, and prints `https://gist.github.com/stub/0123456789`.
  - The test asserts:
    - the backend is `gist` and the URL is the stub's;
    - the arguments were `--version`, then `gist create --public …`;
    - the uploaded content equals the saved transcript and holds none of the three secrets, but does hold
      `[REDACTED]`.
- **No fallback.** An absolute program path is never looked up on `PATH`. A missing stub fails the spawn, and the
  share comes back `local`, which the test's backend and URL assertions refuse. The new test
  `a_missing_gh_never_reaches_another_gh` pins that: a missing `gh` gives a `local` share.
- **The other share tests** render or scrub text and spawn nothing. No other test in the workspace runs `gh`:
  `share.rs` holds the only two `Command::new("gh")` calls.

## Done when

- The `[[verify]]` passes on a machine whose `gh` is logged in.
- `gh gist list` counts the same before and after.
- The test runs with the rest of the suite again, with no `--skip`.

## Notes

- Found 2026-10-08. Will's call on 2026-10-27: fix the test with a fake `gh`.
- The memory note that says to exclude the test is updated once this lands.
