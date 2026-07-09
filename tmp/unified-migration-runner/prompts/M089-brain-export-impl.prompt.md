# M089 — Brain Export with Filters

## Objective
Implement the brain export CLI command with configurable filters: minimum tier, date range, include/exclude episodes, include/exclude learning state. The command `roko knowledge export` produces a BrainExport CBOR file that can be transferred to other workspaces. Filters control the size and scope of the export.

## Scope
- Crates: `roko-neuro`, `roko-cli`
- Files: `crates/roko-neuro/src/brain/export.rs` (new), `crates/roko-cli/src/` (add CLI subcommand)
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.6

## Steps
1. Read the brain format from M088:
   ```bash
   cat crates/roko-neuro/src/brain/format.rs 2>/dev/null | head -40
   ```

2. Read the existing knowledge/neuro store for data access patterns:
   ```bash
   grep -rn 'pub fn query\|pub fn list\|pub fn get' crates/roko-neuro/src/ --include='*.rs' | head -15
   ```

3. Implement the exporter in `crates/roko-neuro/src/brain/export.rs`:
   ```rust
   pub struct BrainExporter {
       store: Arc<dyn Store>,
       learning_path: PathBuf,
   }

   pub struct ExportConfig {
       pub min_tier: String,      // "Transient", "Working", "Consolidated", "Persistent"
       pub since: Option<Duration>,
       pub until: Option<DateTime<Utc>>,
       pub include_episodes: bool,
       pub include_learning: bool,
       pub max_signals: Option<usize>,
   }

   impl BrainExporter {
       pub async fn export(&self, config: ExportConfig) -> Result<BrainExport> {
           // 1. Query store for Signals matching filters
           // 2. Load learning state if include_learning
           // 3. Load episodes if include_episodes
           // 4. Build Merkle tree over entries
           // 5. Package into BrainExport
       }

       pub async fn export_to_file(&self, config: ExportConfig, path: &Path) -> Result<ExportSummary>;
   }
   ```

4. Register CLI command:
   ```
   roko knowledge export [--min-tier=Working] [--since=30d] [--include-episodes] [--output=brain.cbor]
   ```

5. Write tests:
   - Export with `--min-tier=Consolidated` excludes Transient and Working Signals
   - Export with `--since=7d` only includes recent Signals
   - Export without `--include-episodes` produces smaller file
   - Output file is valid CBOR

## Verification
```bash
cargo check -p roko-neuro
cargo check -p roko-cli
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo test -p roko-neuro -- brain::export
```

## What NOT to do
- Do NOT export full Signal content -- only metadata and fingerprints (per M088 format)
- Do NOT include secrets or API keys in exports
- Do NOT skip Merkle root computation -- it is needed for import verification
- Do NOT add compression -- CBOR is already compact; if needed, the user can gzip externally
