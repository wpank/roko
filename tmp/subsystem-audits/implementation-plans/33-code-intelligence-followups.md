# 33 — Code Intelligence Follow-Ups

`roko-codeintel` (~8.2K LOC) has tree-sitter symbol graphs, HDC
fingerprints, and an MCP server adapter. The audit calls it "solid but
under-utilized": HDC similarity is disabled in prompt assembly; the
index is rebuilt fresh every time instead of incrementally.

Source: subsystem-audits/code-intelligence/AUDIT.md, doc 35.

---

## Anti-Patterns

1. **Don't rebuild the index on every dispatch.** Incremental updates
   keyed on file mtime / git tree.
2. **Don't store the index in memory only.** Persist under `.roko/index/`.
3. **Don't query without a cache.** HDC similarity computation is hot
   path; cache by fingerprint.
4. **Don't add tree-sitter languages without testing.** Each grammar is
   a binary dep; test before adding.

---

## Plan

### [ ] CI-1: Persist the symbol index

**File**: `crates/roko-codeintel/src/index.rs`

```rust
pub struct PersistedIndex {
    pub symbols: SymbolGraph,
    pub fingerprints: HashMap<PathBuf, FileFingerprint>,
    pub indexed_at: chrono::DateTime<chrono::Utc>,
}

impl PersistedIndex {
    pub fn load(workdir: &Path) -> Result<Self> {
        let path = workdir.join(".roko").join("index").join("symbols.json");
        // ...
    }

    pub fn save(&self, workdir: &Path) -> Result<()> { /* ... */ }
}
```

### [ ] CI-2: Incremental updates

When a dispatch happens, update only changed files:

```rust
pub async fn refresh_index(index: &mut PersistedIndex, workdir: &Path) -> Result<()> {
    let changed_files = git_changed_since_last_index(workdir, index.indexed_at)?;
    for file in changed_files {
        let new_fp = compute_file_fingerprint(&file)?;
        if Some(&new_fp) != index.fingerprints.get(&file) {
            let symbols = parse_file_symbols(&file)?;
            index.symbols.update_file(&file, symbols);
            index.fingerprints.insert(file, new_fp);
        }
    }
    index.indexed_at = chrono::Utc::now();
    index.save(workdir)?;
    Ok(())
}
```

### [ ] CI-3: Re-enable HDC similarity in prompt assembly

See plan 30 § PA-3. Wires this crate's `find_similar` API.

### [ ] CI-4: MCP server health

The MCP adapter exposes the symbol graph as an MCP server. Verify:

- Process lifecycle: starts on-demand, shuts down when no clients.
- Health probe: `mcp.list_tools()` returns expected tools.
- Error mode: missing index → typed error, not panic.

---

## Combined Verification

```bash
cargo test -p roko-codeintel --lib

# Index persisted
ls .roko/index/symbols.json   # exists after first dispatch

# Incremental
touch crates/roko-cli/src/lib.rs
roko show index   # should show only roko-cli/src/lib.rs as changed

# HDC similarity working
roko think "how does X work"   # should reference past task summaries
```

**Estimated effort**: 8-15 hours.
