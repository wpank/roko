# M063 — `roko marketplace publish/install/fork` CLI

## Objective
Implement the marketplace CLI commands: `publish` (package Cell + manifest + tests into an artifact), `install` (download and register an artifact), and `fork` (copy a Cell with new author, linking provenance to the original). These commands operate on the local Cell registry and prepare for future remote marketplace integration.

## Scope
- Crates: `roko-cli`
- Files: `crates/roko-cli/src/marketplace.rs` (new), CLI argument registration
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.11
- Spec ref: `tmp/unified/15-MARKETPLACE-AND-SHARING.md` SS5-7

## Steps
1. Check for existing marketplace CLI code:
   ```bash
   grep -rn 'marketplace\|Marketplace\|publish\|install\|fork' crates/roko-cli/src/ --include='*.rs' | head -15
   ```

2. Read the existing job/marketplace infrastructure if any:
   ```bash
   grep -rn 'marketplace\|Marketplace' crates/roko-serve/src/ --include='*.rs' | head -10
   ```

3. Implement `roko marketplace publish` in `crates/roko-cli/src/marketplace.rs`:
   ```rust
   pub async fn publish(path: &Path, output: Option<&Path>) -> Result<ArtifactInfo> {
       // 1. Read manifest.toml from path
       // 2. Validate manifest completeness
       // 3. Package: manifest + source files + tests into a .tar.gz artifact
       // 4. Generate content hash for artifact identity
       // 5. Write artifact to output path (default: .roko/artifacts/)
       // 6. Print artifact info (name, version, hash, size)
   }
   ```

4. Implement `roko marketplace install`:
   ```rust
   pub async fn install(artifact_path: &Path) -> Result<()> {
       // 1. Read and validate artifact
       // 2. Extract to .roko/cells/<name>/
       // 3. Register in local CellRegistry
       // 4. Print installation summary
   }
   ```

5. Implement `roko marketplace fork`:
   ```rust
   pub async fn fork(source: &str, new_name: &str, new_author: &str) -> Result<()> {
       // 1. Resolve source Cell from registry
       // 2. Copy Cell files to new directory
       // 3. Update manifest: new name, new author, add forked_from provenance
       // 4. Register forked Cell in registry
       // 5. Print fork info with provenance link
   }
   ```

6. Register CLI subcommands:
   ```
   roko marketplace list                          # List all discoverable Cells
   roko marketplace publish <path> [--output dir] # Package and publish
   roko marketplace install <artifact>            # Install from artifact
   roko marketplace fork <source> --name <new> --author <author>  # Fork with provenance
   ```

7. Write tests:
   - Publish creates a valid artifact archive
   - Install extracts and registers the Cell
   - Fork copies files and updates manifest with provenance
   - List shows installed Cells

## Verification
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
cargo test -p roko-cli -- marketplace
# Manual smoke test:
# cargo run -p roko-cli -- marketplace list
```

## What NOT to do
- Do NOT implement remote marketplace (HTTP upload/download) -- that is M064
- Do NOT implement payment or pricing -- this is local-only for now
- Do NOT implement version conflict resolution -- fail on name collision
- Do NOT add cryptographic signing -- that is a follow-up security concern
