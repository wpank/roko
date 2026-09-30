+++
id = "bug-af3cf4"
kind = "bug"
title = "write_token writes serve.token.tmp with default permissions before chmodding it to 0600"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve/endpoint"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "2ae9d2a7f"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-serve-sec's report, checked on work/bug-928add at 090b81f3e)"
anchors = ["crates/roko-serve/src/endpoint.rs::write_token"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-928add"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn write_token_never_leaves_a_readable_temp_file' crates/roko-serve/src/ && cargo test -p roko-serve --lib write_token_never_leaves_a_readable_temp_file"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 1c7442b58. The launch-token temp file is created 0600 with create_new, synced, then renamed. Batch 14 gate: first run on 4e030ab47 (check, clippy clean; tests pass: roko-cli 3171, roko-agent 2263, roko-core 1952, roko-learn 1203, roko-serve 986, roko-graph 472, roko-fs 259, roko-neuro 239), then re-gated on 8ce3bb131 (same code as MAIN 2ae9d2a7f) after the coordinator's rustfmt commits and serve-sec's bug-633b68 root fix: check, nightly fmt, clippy -p roko-cli -p roko-serve -p roko-core -p roko-agent -p roko-learn --keep-going -D warnings clean; roko-cli lib 3172 passed (one sibling-settle race flake passes alone, bug-779ae7); --test secret_canary 11 passed; --test secrets_and_git_guard_canary 1 passed, 1 ignored (bug-0d9ac4). Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

`endpoint.rs::write_token` (:113) writes the launch token to a sibling temp file with `std::fs::write(&tmp, token)` (:121). That creates the file with the process's default permissions, often 0644, and only then sets 0600 (:127) and renames it. Until then, other local users can read the token.

## Why it matters

Release blockers: the token authenticates to serve. The window is short, but it's a real exposure on shared machines. p2.

## Where

`write_token`.

## Plan

1. Create the temp file with mode 0600 from the start (`OpenOptions` with `.mode(0o600)` and `create_new(true)`), write, fsync, then rename.
2. Add `write_token_never_leaves_a_readable_temp_file`.

## Done when

- [ ] The token file is never readable by others, at any point.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-09-30 (wk-serve-sec): Implemented on `work/bug-af1020` at `513f016cc`; cargo verification deferred to the batch check. `write_token` creates `serve.token.tmp` with `OpenOptions` mode 0600 and `create_new` (`create_owner_only`), removes a leftover temp file first, syncs the file, then renames it into place. Test: `endpoint::tests::write_token_never_leaves_a_readable_temp_file`.
