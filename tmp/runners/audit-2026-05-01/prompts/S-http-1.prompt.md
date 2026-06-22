# S-http-1: Audit + consolidate atomic write helpers

## Task
Inventory every atomic-write helper across `roko-runtime`, `roko-fs`, `roko-serve`. Pick a single canonical helper. Migrate other call sites to use it.

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/32-http-persistence-followups.md` § HP-1.

## Read first

```bash
rg 'fn write_json|fn save_json|fn atomic_write|fn write_atomically|fn write_atomic|TempFile.*rename' crates/ -g '*.rs' -n
```

Identify candidates. Likely:
- `crates/roko-fs/src/atomic.rs::write_atomic`
- `crates/roko-runtime/src/persistence.rs::save_json` (if exists)
- Various route-local `tokio::fs::write` calls

## Exact changes

### 1. Pick canonical home

`crates/roko-fs/src/atomic.rs` is the natural home. Confirm its API:

```rust
pub async fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| std::io::Error::other("no parent dir"))?;
    let staged = tempfile::Builder::new()
        .prefix(".roko-tmp-")
        .tempfile_in(parent)?;
    let staged_path = staged.path().to_path_buf();
    tokio::fs::write(&staged_path, contents).await?;
    tokio::fs::rename(&staged_path, path).await?;
    Ok(())
}
```

### 2. Migrate alternate helpers

For each duplicate (`save_json`, `write_atomically`, etc.):

- If it's a thin wrapper around the canonical helper, replace with the canonical helper at every call site.
- If it adds value (e.g. `save_json` does `serde_json::to_string + write_atomic` in one call), refactor to call `write_atomic` internally.

### 3. Migrate route-local `tokio::fs::write`

For state writes that need atomicity (config, jobs, plans, agents, etc.), replace bare `tokio::fs::write(&path, &contents).await` with `roko_fs::atomic::write_atomic(&path, &contents).await`.

For purely transient writes (logs, temp files), bare `tokio::fs::write` is fine.

### 4. Tests

```rust
#[tokio::test]
async fn write_atomic_does_not_leave_partial_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("state.json");
    write_atomic(&path, b"{\"a\": 1}").await.unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), r#"{"a": 1}"#);
}

#[tokio::test]
async fn write_atomic_is_replace() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("state.json");
    std::fs::write(&path, "old").unwrap();
    write_atomic(&path, b"new").await.unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
}
```

## Write Scope
- `crates/roko-fs/src/atomic.rs`
- (Migration sites for duplicates)

## Verify

```bash
# Single canonical implementation
rg 'fn write_atomic\b' crates/ -g '*.rs'
# Expect: 1 hit (the canonical) + maybe a re-export

# No bare tokio::fs::write for state files
rg 'tokio::fs::write\(' crates/roko-serve/src/routes/ crates/roko-runtime/src/ -g '*.rs'
# Manually inspect each remaining hit; if it writes durable state, migrate.
```

## Do NOT

- Do NOT bundle with S-http-2.
- Do NOT migrate transient writes (logs, temp files).
- Do NOT change the canonical helper's signature in a backward-incompatible way.
- Do NOT introduce `std::fs` blocking writes in async code paths.
