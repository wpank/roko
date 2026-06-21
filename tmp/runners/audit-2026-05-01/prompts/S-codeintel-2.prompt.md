# S-codeintel-2: Incremental index updates by file fingerprint

## Task
Refresh only changed files (by mtime + content fingerprint) instead of full rescan on every dispatch.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-codeintel-1. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/33-code-intelligence-followups.md` § CI-2.

## Exact changes

### `crates/roko-codeintel/src/index.rs`

```rust
pub async fn refresh_index(
    index: &mut PersistedIndex,
    workdir: &Path,
) -> Result<RefreshSummary, IndexError> {
    let changed_files = list_changed_files_since(workdir, index.indexed_at)?;
    let mut updated = 0;
    for file in &changed_files {
        let new_fp = compute_file_fingerprint(file)?;
        if Some(&new_fp) != index.fingerprints.get(file) {
            let symbols = parse_file_symbols(file)?;
            index.symbols.update_file(file, symbols);
            index.fingerprints.insert(file.clone(), new_fp);
            updated += 1;
        }
    }
    index.indexed_at = chrono::Utc::now();
    Ok(RefreshSummary {
        scanned: changed_files.len(),
        updated,
    })
}

#[derive(Debug)]
pub struct RefreshSummary {
    pub scanned: usize,
    pub updated: usize,
}

fn list_changed_files_since(workdir: &Path, since: chrono::DateTime<chrono::Utc>) -> Result<Vec<PathBuf>, IndexError> {
    // Walk the source tree; return files with mtime > since.
    // For Git workspaces, prefer `git diff --name-only HEAD@{since}` if cheap.
}

fn compute_file_fingerprint(path: &Path) -> Result<FileFingerprint, IndexError> {
    let bytes = std::fs::read(path)?;
    Ok(FileFingerprint::Blake3(blake3::hash(&bytes).to_hex().to_string()))
}
```

### Wire from orchestrator startup

`crates/roko-cli/src/orchestrate.rs`:

```rust
let mut index = PersistedIndex::load(workdir).unwrap_or_else(|_| PersistedIndex::fresh(workdir).unwrap());
let summary = refresh_index(&mut index, workdir).await?;
tracing::info!(scanned = summary.scanned, updated = summary.updated, "codeintel index refreshed");
index.save(workdir).await?;
```

### Tests

```rust
#[tokio::test]
async fn refresh_skips_unchanged_files() {
    let dir = tempdir().unwrap();
    write_test_file(dir.path(), "a.rs", "pub fn foo() {}");
    let mut idx = PersistedIndex::fresh(dir.path()).unwrap();

    // Touch nothing; refresh should report 0 updated.
    let summary = refresh_index(&mut idx, dir.path()).await.unwrap();
    assert_eq!(summary.updated, 0);

    // Modify file; refresh should report 1 updated.
    write_test_file(dir.path(), "a.rs", "pub fn foo() { println!(); }");
    let summary = refresh_index(&mut idx, dir.path()).await.unwrap();
    assert_eq!(summary.updated, 1);
}
```

## Write Scope
- `crates/roko-codeintel/src/index.rs`
- `crates/roko-cli/src/orchestrate.rs` (or post-T5-35 location)

## Verify

```bash
rg 'fn refresh_index|FileFingerprint::Blake3' crates/roko-codeintel/src/index.rs
# Expect: at least 2 hits
```

## Do NOT

- Do NOT bundle with S-codeintel-1.
- Do NOT walk the entire tree on refresh; use mtime + fingerprint.
- Do NOT block dispatch on a slow refresh; consider running in a `tokio::task::spawn` if it gets hot.
- Do NOT use `git diff` if the workspace isn't a git repo (fallback to mtime walk).
