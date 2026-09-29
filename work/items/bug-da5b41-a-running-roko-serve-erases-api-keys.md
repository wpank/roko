+++
id = "bug-da5b41"
kind = "bug"
title = "A running roko serve erases API keys created by the CLI"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "release"
subsystem = ["roko-serve/auth"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "98a77c510"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/auth.rs::AuthRegistry", "crates/roko-serve/src/routes/auth.rs::persist_registry_file", "crates/roko-serve/src/routes/auth.rs::load_registry_file", "crates/roko-serve/src/routes/middleware.rs::require_api_key"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn out_of_band_api_key_survives_server_write' crates/roko-serve/ && cargo test -p roko-serve routes::auth::tests::out_of_band_api_key_survives_server_write"
+++

## Problem

`roko serve` reads `.roko/api-keys.json` once, at startup, into memory. Every later server-side change writes
the whole in-memory list back over the file. Any key added to the file by something other than this server
process after startup is therefore:

1. unusable against the running server (it authenticates from memory only), and
2. deleted from disk by the server's next write.

The next write comes quickly: the `require_api_key` middleware calls `AuthRegistry::record_api_key_use` on every request
authenticated with an API key (to update `last_used_at`), and that call persists the full list. So one
ordinary API request after the out-of-band change erases the new key. The client that received it is locked
out, even after a server restart.

Repro (unit level): load an `AuthRegistry` for a temp workdir holding key A; append key B to
`.roko/api-keys.json` directly; call `record_api_key_use("A")`; reload the file. Actual: B is gone.
Expected: B is still there.

## Why it matters

- Goal `release` (public release: security). Silent credential loss on a server that operators will run
  publicly; revocation and rotation races are the adjacent risk.
- Who writes the file out of band today: at HEAD no roko CLI command writes `.roko/api-keys.json` (only
  `crates/roko-serve/src/routes/auth.rs` does; `POST /api/api-keys` goes through the running server and is
  safe). Realistic triggers are a second `roko serve` process on the same workspace (serve takes no exclusive
  workspace lock at startup), a hand edit or restore of the file, and any future key-management CLI. The
  item title's "created by the CLI" describes that future CLI; the defect exists regardless.

## Where

- `crates/roko-serve/src/routes/auth.rs::AuthRegistry` (line ~195): in-memory `RwLock<Vec<ApiKeyEntry>>`,
  plus agent tokens and relay tokens.
  - `AuthRegistry::load` (~206): reads the file, merges `[serve.auth.api_keys]` from `roko.toml` by name.
    Called once from `crates/roko-serve/src/state.rs:1153`.
  - `insert_api_key` (~227, persist at 237), `remove_api_key` (~242, persist at 249), `rotate_api_key`
    (~254, persist at 284), `record_api_key_use` (~289, persist at 296): each clones the in-memory list,
    edits it, and writes the whole list with `persist_registry_file`.
  - `insert_agent_token` and the relay-token methods use the same whole-list pattern for
    `agent-tokens.json` and the relay registry.
- `persist_registry_file` (~345): atomic temp-file + rename replace; no cross-process lock, no re-read.
- `load_registry_file` (~336): plain read + parse; missing file is an empty list; bad JSON fails.
- `crates/roko-serve/src/routes/middleware.rs::require_api_key` (~688; the call is at ~958):
  `record_api_key_use(key_name)` on every API-key request.
- HTTP routes (`routes/auth.rs:884`): `POST/GET /api/api-keys`, `DELETE /api/api-keys/:name`,
  `POST /api/api-keys/:name/rotate`.

## Current state

- The in-process race is handled: each method holds the write lock until the file is durable, and
  `usage_update_cannot_clobber_rotation` tests that. Cross-process changes are not handled at all.
- Recent auth commits: `352f13f23` (flush tokio::fs writes), `ceb0e5fc2` (auth audit trail). Neither touches
  the load-once design.
- `fs2` (advisory file locks) is already used in the workspace (`crates/roko-cli/src/workspace_lock.rs`,
  `crates/roko-runtime/src/state_hub.rs`, `crates/roko-fs`), but is not a dependency of roko-serve.

## Plan

1. Make every API-key mutation a locked read-merge-write against the file on disk:
   - take an exclusive `fs2` lock on a sidecar lock file (for example `.roko/api-keys.json.lock`; do not lock
     the data file itself, since it is replaced by rename);
   - re-read `.roko/api-keys.json` with `load_registry_file`;
   - apply only this operation's delta (insert by name, remove by name, rotate by name, set `last_used_at`
     by name);
   - write with `persist_registry_file`, release the lock, and replace the in-memory list with the merged
     result.
   Keep the tokio `RwLock` for in-process ordering; do the blocking lock in `spawn_blocking` or with a short
   retry loop so the async runtime is not blocked.
2. Let the running server see out-of-band keys: on an authentication miss, reload the file (rate limited, or
   only when its mtime changed) before rejecting. Alternative: reload on every mutation only (step 1) and
   accept that new out-of-band keys work after the next server write. Recommend the auth-miss reload so a new
   key works immediately.
3. Consider reducing write amplification: `record_api_key_use` rewrites the file on every request. Either
   keep the per-request write (now merge-safe) or coalesce `last_used_at` updates (for example at most once a
   minute per key). Optional; not required for correctness.
4. Apply the same read-merge-write to `agent-tokens.json` and the relay registry, or file a follow-up item if
   that grows the change too much.
5. Tests in `routes::auth::tests`:
   - `out_of_band_api_key_survives_server_write`: write key B to the file behind a loaded registry, then call
     `record_api_key_use("A")`, `insert_api_key(C)`, `rotate_api_key("A", ..)` and `remove_api_key("C")`;
     assert B survives every write.
   - one test that an out-of-band key authenticates without a restart (if step 2 is done).

## Done when

- Keys added to `.roko/api-keys.json` by another process survive every server-side write.
- A revoked key stays revoked when another process writes the file concurrently (the merge must not
  resurrect it from a stale copy).
- Existing auth tests still pass (`usage_update_cannot_clobber_rotation`,
  `registry_survives_restart_and_tracks_usage`, `malformed_registry_fails_closed_on_startup`).
- Verify: `grep -rqw 'fn out_of_band_api_key_survives_server_write' crates/roko-serve/ && cargo test -p roko-serve routes::auth::tests::out_of_band_api_key_survives_server_write`

## Notes

- Security-sensitive (authentication and persistence). Fail closed: if the file cannot be read or parsed
  during a merge, return an error and do not write; never fall back to writing the in-memory copy.
- Merge semantics need care for removal: a delete must remove by name from the freshly read list, and must not
  be undone by another process writing an older snapshot. With the lock held across read-modify-write in every
  writer, that holds as long as all writers use the lock; a hand edit cannot be protected.
- Adding `fs2` to `crates/roko-serve/Cargo.toml` is a new direct dependency (already in the lockfile).
- Touches only `routes/auth.rs` (and possibly `middleware.rs` for the auth-miss reload). Safe to do in
  parallel with work outside roko-serve auth.
- 2026-09-29: Implemented on `work/bug-da5b41` at `1098aead2`; cargo verification deferred to the batch check.
  Plan steps 1-3 and 5 are done. Writes use the existing `roko_fs::with_locked_json_transaction`, so no new
  dependency. Every lookup re-reads the file when its stamp changes (not only on an auth miss), so
  out-of-band revocations also apply at once. Step 4 (agent/relay token files) is left for a follow-up item.

## Original notes

The serve key store loads `.roko/api-keys.json` once (`routes/auth.rs:207`), and every later mutation persists its whole in-memory list (`persist_registry_file(&api_keys_path(..), &updated)` at `:237`, `:249`, `:284`, `:296`).
A key created with the CLI while serve runs is overwritten by the next server-side write, locking that client out.
Fix: make every server-side write a locked read-merge-write of a single delta against the file on disk, and test that an out-of-band key survives a server write.
