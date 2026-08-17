# W13-D: Atomic State Writes + Cross-Plan State Leakage Prevention

**Wave**: 13 -- Speed & Reliability
**IMPROVEMENTS ref**: 3.1 + 3.3
**Priority**: P1 -- prevents data corruption on crash + prevents stale state bugs
**Effort**: 1-2 hours
**Files to modify**: 4 files
**Dependencies**: None

## Problem

**3.1 Atomic state writes**: The snapshot writer already uses `atomic_write` (write to
`.tmp`, then rename). However, there is no **checkpoint file** that records expected
state files + their hashes. If the process crashes between writing `executor.json.tmp`
and `run-state.json.tmp`, resume can load inconsistent state (executor thinks task T3
is done, run-state says T2 is in progress).

**3.3 Cross-plan state leakage**: The `--fresh` handler at
`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs` lines 246-271
only archives `executor.json`. But state is spread across 3 files:
`executor.json`, `orchestrator.json`, and `run-state.json`. Starting a new plan with
stale orchestrator/run-state causes confusing behavior.

## Root Cause

1. No checkpoint coordination between the 3 state files that `save_snapshot` writes.
2. `--fresh` only cleans `executor.json`, leaving `orchestrator.json` and `run-state.json`.

## Exact Code to Change

### File 1: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`

#### Change 1: Expand `--fresh` to archive ALL state files

**Find this code** (lines 246-271):
```rust
            if fresh {
                let state_path = wd.join(".roko").join("state").join("executor.json");
                if state_path.exists() {
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis();
                    let backup_path = state_path.with_extension(format!("json.bak.{ts}"));
                    match std::fs::rename(&state_path, &backup_path) {
                        Ok(()) => {
                            if !cli.quiet {
                                eprintln!(
                                    "▸ --fresh: archived old state to {}",
                                    backup_path.display()
                                );
                            }
                        }
                        Err(err) => {
                            eprintln!(
                                "warning: --fresh: could not archive {}: {err}",
                                state_path.display()
                            );
                        }
                    }
                }
            }
```

**Replace with:**
```rust
            if fresh {
                let state_dir = wd.join(".roko").join("state");
                let state_files = [
                    "executor.json",
                    "orchestrator.json",
                    "run-state.json",
                ];
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();
                let mut archived = 0u32;
                for file in &state_files {
                    let path = state_dir.join(file);
                    if path.exists() {
                        let backup_path =
                            path.with_extension(format!("json.bak.{ts}"));
                        match std::fs::rename(&path, &backup_path) {
                            Ok(()) => {
                                archived += 1;
                                if !cli.quiet {
                                    eprintln!(
                                        "▸ --fresh: archived {} to {}",
                                        file,
                                        backup_path.display()
                                    );
                                }
                            }
                            Err(err) => {
                                eprintln!(
                                    "warning: --fresh: could not archive {}: {err}",
                                    path.display()
                                );
                            }
                        }
                    }
                }
                if archived > 0 && !cli.quiet {
                    eprintln!(
                        "▸ --fresh: archived {archived} state file(s) \
                         (timestamp {ts})"
                    );
                }
            }
```

### File 2: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/persist.rs`

#### Change 2: Add checkpoint write/verify functions after `atomic_write`

The `atomic_write` function ends at line 250. Add the checkpoint functions after it,
before the existing `append_jsonl` function (line 252).

**Find this code** (lines 247-252):
```rust
    fs::rename(&tmp, path)
        .with_context(|| format!("renaming {} → {}", tmp.display(), path.display()))?;
    Ok(())
}

/// Append a JSON line to a JSONL file.
```

**Replace with:**
```rust
    fs::rename(&tmp, path)
        .with_context(|| format!("renaming {} → {}", tmp.display(), path.display()))?;
    Ok(())
}

/// Write a checkpoint file listing state files and their FNV-1a hashes.
///
/// This allows `verify_checkpoint` to detect partially-written state sets
/// on resume (e.g. executor.json updated but run-state.json stale from a crash).
pub fn write_checkpoint(state_dir: &Path, files: &[(&str, &[u8])]) -> Result<()> {
    let t0 = std::time::Instant::now();
    let entries: Vec<String> = files
        .iter()
        .map(|(name, data)| {
            let payload = std::str::from_utf8(data).unwrap_or("");
            let hash = fnv1a_hex(payload);
            format!("{name}:{hash}")
        })
        .collect();
    let result = atomic_write(
        &state_dir.join("checkpoint.txt"),
        entries.join("\n").as_bytes(),
    );
    let elapsed_us = t0.elapsed().as_micros();
    tracing::debug!(elapsed_us, files = files.len(), "write_checkpoint");
    result
}

/// Verify that all state files match the checkpoint.
///
/// Returns `Ok(true)` if all hashes match, `Ok(false)` if any file is
/// corrupted or missing, `Err` if the checkpoint itself is unreadable.
pub fn verify_checkpoint(state_dir: &Path) -> Result<bool> {
    let checkpoint_path = state_dir.join("checkpoint.txt");
    if !checkpoint_path.exists() {
        // No checkpoint = no prior run, nothing to verify.
        return Ok(true);
    }
    let checkpoint = fs::read_to_string(&checkpoint_path)
        .with_context(|| format!("read {}", checkpoint_path.display()))?;
    for line in checkpoint.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (name, expected_hash) = line
            .split_once(':')
            .with_context(|| format!("bad checkpoint line: {line}"))?;
        let file_path = state_dir.join(name);
        if !file_path.exists() {
            tracing::warn!(file = %name, "checkpoint: file missing");
            return Ok(false);
        }
        let data = fs::read_to_string(&file_path)
            .with_context(|| format!("read {}", file_path.display()))?;
        let actual_hash = fnv1a_hex(&data);
        if actual_hash != expected_hash {
            tracing::warn!(
                file = %name,
                expected = %expected_hash,
                actual = %actual_hash,
                "checkpoint: hash mismatch"
            );
            return Ok(false);
        }
    }
    Ok(true)
}

/// Append a JSON line to a JSONL file.
```

Note: `fnv1a_hex` is already defined in this file (line 495). No new dependencies needed --
we reuse the existing FNV-1a hash instead of adding `sha2`/`hex`.

### File 3: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/snapshot_writer.rs`

#### Change 3: Call `write_checkpoint` from `write_all_files`

**Find this code** (lines 181-187):
```rust
fn write_all_files(payload: &SnapshotPayload) -> anyhow::Result<()> {
    use super::persist::atomic_write;
    atomic_write(&payload.orchestrator_path, &payload.orchestrator_json)?;
    atomic_write(&payload.executor_path, &payload.executor_json)?;
    atomic_write(&payload.run_state_path, &payload.run_state_json)?;
    Ok(())
}
```

**Replace with:**
```rust
fn write_all_files(payload: &SnapshotPayload) -> anyhow::Result<()> {
    use super::persist::{atomic_write, write_checkpoint};
    atomic_write(&payload.orchestrator_path, &payload.orchestrator_json)?;
    atomic_write(&payload.executor_path, &payload.executor_json)?;
    atomic_write(&payload.run_state_path, &payload.run_state_json)?;

    // Write checkpoint after all files are persisted so resume can
    // detect partially-written state sets.
    if let Some(state_dir) = payload.executor_path.parent() {
        let files: Vec<(&str, &[u8])> = vec![
            ("orchestrator.json", &payload.orchestrator_json),
            ("executor.json", &payload.executor_json),
            ("run-state.json", &payload.run_state_json),
        ];
        if let Err(e) = write_checkpoint(state_dir, &files) {
            tracing::warn!(error = %e, "failed to write state checkpoint");
        }
    }
    Ok(())
}
```

### File 4: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`

#### Change 4: Verify checkpoint on resume

**Find this code** (lines 152-162):
```rust
    let prior_snapshot = match persist::load_run_state(&paths) {
        Ok(Some(snapshot)) => Some(snapshot),
        Ok(None) => None,
        Err(err) => {
            warn!(
                error = %err,
                "failed to read prior run-state.json; continuing without seeded resume state"
            );
            None
        }
    };
```

**Replace with:**
```rust
    let prior_snapshot = match persist::load_run_state(&paths) {
        Ok(Some(snapshot)) => Some(snapshot),
        Ok(None) => None,
        Err(err) => {
            warn!(
                error = %err,
                "failed to read prior run-state.json; continuing without seeded resume state"
            );
            None
        }
    };

    // Verify state file integrity via checkpoint hashes.
    if prior_snapshot.is_some() {
        let state_dir = paths.executor_json.parent().unwrap_or(std::path::Path::new(".roko/state"));
        match persist::verify_checkpoint(state_dir) {
            Ok(true) => {
                info!("state checkpoint verified -- all files consistent");
            }
            Ok(false) => {
                warn!(
                    "state checkpoint mismatch -- state files may be from different \
                     snapshots; consider `--fresh` to start clean"
                );
            }
            Err(e) => {
                warn!(error = %e, "failed to verify state checkpoint");
            }
        }
    }
```

Note: `std::path::Path` is already imported at the top of event_loop.rs (line 6:
`use std::path::Path`). `info` and `warn` are imported from `tracing` at line 23.

## Verification

```bash
# Compile check
cargo check -p roko-cli 2>&1 | head -20

# Verify --fresh archives all 3 files
cargo run -p roko-cli -- plan run plans/ --fresh 2>&1 | grep "archived"
# Should show 3 files archived (executor.json, orchestrator.json, run-state.json)

# Verify checkpoint functions exist
grep -n "write_checkpoint\|verify_checkpoint" crates/roko-cli/src/runner/persist.rs
```

## Agent Prompt

```
You are implementing W13-D: Atomic State Writes + Cross-Plan State Leakage Prevention.
This prevents data corruption on crash and stale state bugs on --fresh.

## Changes to make (4 files)

### 1. commands/plan.rs -- expand --fresh to archive all 3 state files

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`, find the
`if fresh {` block (line 246). Currently it only archives `executor.json`.

Replace the entire block (lines 246-271) so it iterates over an array of 3 filenames:
`["executor.json", "orchestrator.json", "run-state.json"]` and archives each one.
Use a counter to report how many were archived.

### 2. persist.rs -- add checkpoint write/verify functions

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/persist.rs`, add two
new public functions after `atomic_write` (line 250) and before `append_jsonl` (line 252):

- `write_checkpoint(state_dir: &Path, files: &[(&str, &[u8])]) -> Result<()>`
  Uses the existing `fnv1a_hex` function (line 495) to hash file contents.
  Writes `name:hash` lines to `checkpoint.txt` via `atomic_write`.

- `verify_checkpoint(state_dir: &Path) -> Result<bool>`
  Reads `checkpoint.txt`, hashes each file with `fnv1a_hex`, returns false on mismatch.

No new dependencies needed -- reuses the existing `fnv1a_hex` in this file.

### 3. snapshot_writer.rs -- call write_checkpoint after all writes

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/snapshot_writer.rs`,
update `write_all_files` (line 181). After the 3 `atomic_write` calls, call
`write_checkpoint` with all 3 file payloads. Handle errors non-fatally with
`tracing::warn!`.

### 4. event_loop.rs -- verify checkpoint on resume

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`, after
the `prior_snapshot` match block (line 162), add checkpoint verification. Use
`paths.executor_json.parent()` to get the state directory (PersistPaths has no
`state_dir()` method).

Do NOT run cargo build/test/clippy/fmt -- compilation is deferred.
```

## Commit

This batch is committed with all Wave 13 batches together. Do not commit individually.

## Checklist

- [ ] `--fresh` archives executor.json, orchestrator.json, AND run-state.json
- [ ] `--fresh` shows count of archived files
- [ ] `write_checkpoint()` added to persist.rs (uses existing `fnv1a_hex`, no new deps)
- [ ] `verify_checkpoint()` added to persist.rs
- [ ] `write_all_files` in snapshot_writer.rs calls `write_checkpoint` after all writes
- [ ] Checkpoint verification added to event_loop.rs resume path
- [ ] Timing instrumentation in `write_checkpoint` via `tracing::debug!`
- [ ] Pre-commit checks pass

## Audit Status

Audited: 2026-05-05. PASS no changes needed
