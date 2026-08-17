# S-codeintel-1: Persist symbol index to .roko/index/

## Task
The `roko-codeintel` symbol index is rebuilt fresh every dispatch. Persist it under `.roko/index/symbols.json` and load on startup.

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/33-code-intelligence-followups.md` § CI-1.

## Read first

```bash
rg 'fn scan|pub struct SymbolGraph|fn save|fn load' crates/roko-codeintel/src/index.rs -n
```

## Exact changes

### Add `PersistedIndex`

```rust
// crates/roko-codeintel/src/index.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedIndex {
    pub symbols: SymbolGraph,
    pub fingerprints: HashMap<PathBuf, FileFingerprint>,
    pub indexed_at: chrono::DateTime<chrono::Utc>,
    pub schema_version: u32,
}

const INDEX_SCHEMA_VERSION: u32 = 1;

impl PersistedIndex {
    pub fn path_for(workdir: &Path) -> PathBuf {
        workdir.join(".roko").join("index").join("symbols.json")
    }

    pub fn load(workdir: &Path) -> Result<Self, IndexError> {
        let path = Self::path_for(workdir);
        let raw = std::fs::read_to_string(&path)?;
        let parsed: Self = serde_json::from_str(&raw)?;
        if parsed.schema_version != INDEX_SCHEMA_VERSION {
            return Err(IndexError::SchemaMismatch(parsed.schema_version, INDEX_SCHEMA_VERSION));
        }
        Ok(parsed)
    }

    pub async fn save(&self, workdir: &Path) -> Result<(), IndexError> {
        let path = Self::path_for(workdir);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let json = serde_json::to_string_pretty(self)?;
        roko_fs::atomic::write_atomic(&path, json.as_bytes()).await?;
        Ok(())
    }

    pub fn fresh(workdir: &Path) -> Result<Self, IndexError> {
        Ok(Self {
            symbols: SymbolGraph::scan(workdir)?,
            fingerprints: compute_fingerprints(workdir)?,
            indexed_at: chrono::Utc::now(),
            schema_version: INDEX_SCHEMA_VERSION,
        })
    }
}
```

### Tests

```rust
#[tokio::test]
async fn persisted_index_roundtrips() {
    let dir = tempdir().unwrap();
    let idx = PersistedIndex::fresh(dir.path()).unwrap();
    idx.save(dir.path()).await.unwrap();
    let loaded = PersistedIndex::load(dir.path()).unwrap();
    assert_eq!(loaded.schema_version, INDEX_SCHEMA_VERSION);
}
```

## Write Scope
- `crates/roko-codeintel/src/index.rs`

## Verify

```bash
rg 'PersistedIndex|pub fn save|pub fn load' crates/roko-codeintel/src/index.rs
# Expect: at least 3 hits
```

## Do NOT

- Do NOT bundle with S-codeintel-2.
- Do NOT skip schema version check on load.
- Do NOT use `std::fs::write` for the save (use atomic).
- Do NOT compress the index in this batch.
