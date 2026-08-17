# M090 — Brain Import with Decay Factor

## Objective
Implement brain import with a configurable decay factor. Imported Signals start with `balance * decay_factor`, preventing imported knowledge from dominating local knowledge. Conflicts are resolved by content hash (identical = skip, different = keep both with lineage link). The CLI command `roko knowledge import <file> --decay=0.5` integrates external brain exports into the local store.

## Scope
- Crates: `roko-neuro`, `roko-cli`
- Files: `crates/roko-neuro/src/brain/import.rs` (new), `crates/roko-cli/src/` (add CLI subcommand)
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.6

## Steps
1. Read the brain format from M088:
   ```bash
   cat crates/roko-neuro/src/brain/format.rs 2>/dev/null | head -40
   ```

2. Read the store's put/write interface:
   ```bash
   grep -rn 'pub fn put\|pub fn store\|pub fn insert' crates/roko-neuro/src/ --include='*.rs' | head -10
   ```

3. Implement the importer in `crates/roko-neuro/src/brain/import.rs`:
   ```rust
   pub struct BrainImporter {
       store: Arc<dyn Store>,
       learning_path: PathBuf,
   }

   pub struct ImportConfig {
       pub decay_factor: f64,       // 0.0 to 1.0, default 0.5
       pub conflict_strategy: ConflictStrategy,
       pub merge_learning: bool,
   }

   pub enum ConflictStrategy {
       SkipIdentical,     // Same content hash -> skip
       KeepBoth,          // Always import with lineage link
       PreferLocal,       // On conflict, keep local version
       PreferImported,    // On conflict, keep imported version
   }

   impl BrainImporter {
       pub async fn import(&self, export: BrainExport, config: ImportConfig) -> Result<ImportSummary> {
           // 1. Verify Merkle root
           // 2. For each Signal:
           //    a. Check for conflict (same content hash in local store)
           //    b. Apply conflict strategy
           //    c. Apply decay factor to balance: balance *= config.decay_factor
           //    d. Set lineage: imported_from = export.manifest.agent_id
           //    e. Insert into store
           // 3. If merge_learning: merge learning state (section effects, calibration)
           // 4. Return summary (imported, skipped, conflicted)
       }

       pub async fn import_from_file(&self, path: &Path, config: ImportConfig) -> Result<ImportSummary>;
   }
   ```

4. Register CLI command:
   ```
   roko knowledge import <file> [--decay=0.5] [--conflict=skip-identical] [--merge-learning]
   ```

5. Write tests:
   - Import with decay 0.5 gives imported Signals half the balance of native ones
   - Identical content hash is skipped with SkipIdentical strategy
   - Different content hash creates linked entry with KeepBoth strategy
   - Invalid Merkle root rejects the import
   - Import summary shows correct counts

## Verification
```bash
cargo check -p roko-neuro
cargo check -p roko-cli
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo test -p roko-neuro -- brain::import
```

## What NOT to do
- Do NOT import without Merkle verification -- reject tampered exports
- Do NOT allow decay_factor > 1.0 -- imported knowledge must not outweigh local
- Do NOT merge learning state by default -- it must be opt-in
- Do NOT overwrite local Signals on conflict -- always prefer local or keep both
