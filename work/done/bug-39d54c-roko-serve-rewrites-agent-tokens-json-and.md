+++
id = "bug-39d54c"
kind = "bug"
title = "roko serve rewrites agent-tokens.json and relay-tokens.json whole from memory, losing other processes' tokens and revocations"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve/auth"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "b11ca807d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-serve-keys's report on bug-da5b41, branch work/bug-da5b41)"
anchors = ["crates/roko-serve/src/routes/auth.rs::insert_agent_token", "crates/roko-serve/src/routes/auth.rs::revoke_agent_token", "crates/roko-serve/src/routes/auth.rs::persist_registry_file"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = ["bug-da5b41"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_token_revoked_by_another_process_stays_revoked' crates/roko-serve/src/ && cargo test -p roko-serve --lib a_token_revoked_by_another_process_stays_revoked"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "agent-tokens.json and relay-tokens.json writes are locked read-merge-write changes through a generic apply_registry_change (shared with API keys); RegistryCache re-reads on file change so another process's revocation applies on the next lookup; unreadable files fail closed; test a_token_revoked_by_another_process_stays_revoked (e6dd57a6f, merged 3b98206e3). Batch 6 gate (work/rust-batch-5 tree plus fmt-only and unused-import fixes fdb2a9b72, 579ffd0e6, a8dd7f09f, 2aa55ab1f): cargo check --workspace --tests clean; clippy -p roko-cli -p roko-core -p roko-agent -p roko-serve -p roko-learn -p roko-gateway --no-deps -D warnings clean; lib tests roko-cli 3087, roko-agent 2249, roko-core 1922, roko-learn 1177, roko-serve 955, roko-gate 685, roko-gateway 41, 0 failed; merged MAIN tree re-checked (cargo check --workspace --tests clean)."
+++

## Problem

bug-da5b41 (on `work/bug-da5b41`) turns every API-key write into a locked read-merge-write of one change (`roko_fs::with_locked_json_transaction`), and re-reads the key file when it changes. Agent and relay tokens keep the old pattern: `insert_agent_token`, `revoke_agent_token` and the relay-token issue and revoke paths clone the in-memory list, change the clone, and `persist_registry_file` writes the whole list over `.roko/agent-tokens.json` or `.roko/relay-tokens.json`. The token lists are read once, at startup.

With two writers on one workspace (two serve processes, an overlapping restart, or a hand edit), the last writer's stale list wins:

- a token the other process issued disappears;
- a revocation the other process wrote is undone, and the revoked token works again;
- a running serve never sees another process's revocation until it restarts.

## Why it matters

Release blockers (epic spec-ae5f94): a revocation must stick. bug-da5b41's plan step 4 covers these files, and its branch notes leave step 4 to a follow-up item. This is that item.

## Where

`crates/roko-serve/src/routes/auth.rs` on `work/bug-da5b41`: `insert_agent_token` (about :436), `revoke_agent_token` (about :448), the relay-token writes (about :1083 and :1102), `persist_registry_file` (about :579) and `load_relay_registry` (about :741). The same functions exist at BASE, without the API-key change.

## Current state

Only roko-serve writes these files; unlike `api-keys.json`, no CLI command does. So the trigger is narrower than bug-da5b41's. Writes are atomic (temporary file plus rename), so a reader never sees a torn file; the loss comes from whole-list overwrites.

## Plan

1. After bug-da5b41 merges, move both token files to `with_locked_json_transaction`: each write applies one change (insert, or revoke by id) to the file as it is on disk.
2. Reload the token lists when the file's stamp changes, as the API-key path does, so another process's revocation applies at once.
3. Keep the cascade: revoking an agent token revokes its relay tokens, in the same locked transactions.
4. Add `a_token_revoked_by_another_process_stays_revoked`: two stores on one workdir; one revokes a token, then the other issues a token. The revocation survives, and the second store rejects the revoked token.

## Done when

- [ ] Token writes from two processes never lose each other's changes, and a revocation applies across processes.
- [ ] The `[[verify]]` command passes.

## Notes

- Fail closed, as bug-da5b41 does: if the file can't be read or parsed during a merge, return an error and don't write.
- 2026-09-29: premise confirmed at `407ce30d5`, where bug-da5b41 is merged. Implemented on `work/bug-39d54c` at `e6dd57a6f`;
  cargo verification deferred to the batch check. Plan steps 1-4 are done: both token files go through the shared
  `apply_registry_change` / `run_registry_change` (the API-key path now uses them too), relay issuance validates its
  parent chain against the relay file as it is on disk, and `persist_registry_file` is removed. Tests: the
  verify-named test plus `a_relay_revoked_by_another_process_stays_revoked`,
  `concurrent_token_writers_on_one_workspace_keep_every_token` and `unreadable_token_files_fail_closed_while_running`.
  GET /api/agent-tokens now returns 500 on an unreadable file instead of a stale list.
