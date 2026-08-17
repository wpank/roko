# S-http-2: AtomicWriteSet for transactional multi-file writes

## Task
Add `AtomicWriteSet` for state that spans multiple files. Stage all writes to a temp dir, then atomic-rename each file into place. Either all succeed or none reach disk.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-http-1. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/32-http-persistence-followups.md` § HP-2.

## Exact changes

### `crates/roko-fs/src/atomic.rs` (extend)

```rust
use std::path::PathBuf;

pub struct AtomicWriteSet {
    writes: Vec<(PathBuf, Vec<u8>)>,
}

impl AtomicWriteSet {
    pub fn new() -> Self {
        Self { writes: Vec::new() }
    }

    pub fn add(&mut self, path: PathBuf, contents: Vec<u8>) {
        self.writes.push((path, contents));
    }

    /// Stage all writes; if every staging succeeds, atomically rename
    /// each into place. On any error, no file is touched.
    pub async fn commit(self) -> std::io::Result<()> {
        // Stage to a single temp dir
        let tmp = tempfile::TempDir::new()?;
        let mut staged: Vec<(PathBuf, PathBuf)> = Vec::with_capacity(self.writes.len());
        for (target, contents) in &self.writes {
            let stage_path = tmp.path().join(
                target.file_name().ok_or_else(|| std::io::Error::other("no filename"))?
            );
            tokio::fs::write(&stage_path, contents).await?;
            staged.push((stage_path, target.clone()));
        }
        // Atomic-rename each. On rename failure, rolling back already-renamed files
        // is best-effort.
        let mut renamed: Vec<PathBuf> = Vec::with_capacity(staged.len());
        for (stage_path, target) in &staged {
            if let Some(parent) = target.parent() {
                tokio::fs::create_dir_all(parent).await.ok();
            }
            if let Err(e) = tokio::fs::rename(stage_path, target).await {
                // Rollback: remove already-placed files
                for p in &renamed {
                    let _ = tokio::fs::remove_file(p).await;
                }
                return Err(e);
            }
            renamed.push(target.clone());
        }
        Ok(())
    }
}
```

### Tests

```rust
#[tokio::test]
async fn atomic_write_set_commits_all_or_none() {
    let dir = tempdir().unwrap();
    let mut set = AtomicWriteSet::new();
    set.add(dir.path().join("a.json"), b"a".to_vec());
    set.add(dir.path().join("b.json"), b"b".to_vec());
    set.commit().await.unwrap();
    assert_eq!(std::fs::read(dir.path().join("a.json")).unwrap(), b"a");
    assert_eq!(std::fs::read(dir.path().join("b.json")).unwrap(), b"b");
}

#[tokio::test]
async fn atomic_write_set_keeps_old_state_on_partial_fail() {
    // Create a target where staging would fail (e.g. parent doesn't exist
    // and is read-only). Confirm pre-existing files stay intact.
}
```

### Use sites

For state spanning multiple files (executor.json + gates.json, etc.), migrate to `AtomicWriteSet`:

```rust
let mut set = AtomicWriteSet::new();
set.add(workdir.join(".roko/state/executor.json"), exec_json);
set.add(workdir.join(".roko/state/gates.json"), gates_json);
set.commit().await?;
```

After T5-40d (resume via ledger), this matters less for executor state but still applies to other multi-file state writes.

## Write Scope
- `crates/roko-fs/src/atomic.rs`

## Verify

```bash
rg 'pub struct AtomicWriteSet' crates/roko-fs/src/atomic.rs
# Expect: 1 hit

rg 'AtomicWriteSet::new|\.commit\(\)' crates/ -g '*.rs'
# Use sites: at least 1
```

## Do NOT

- Do NOT bundle with S-http-1.
- Do NOT make `commit` infallible.
- Do NOT skip rollback on partial failure (best-effort, but try).
- Do NOT use bare `std::fs::rename` inside async code; use `tokio::fs::rename`.
