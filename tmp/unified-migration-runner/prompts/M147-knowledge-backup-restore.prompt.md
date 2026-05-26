# M147 — Wire Knowledge Backup/Restore Pipeline

## Objective
Wire the knowledge backup and restore pipeline in `roko-neuro`. The `roko knowledge backup` and `roko knowledge restore` CLI commands exist but the actual pipeline logic — serialize signals with HDC fingerprints, apply generational confidence decay on restore, and route through quarantine/validate/adopt stages — is not implemented. Wire the full backup→archive and restore→decay→validate→adopt flows.

## Scope
- Crates: `roko-neuro`, `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/lifecycle.rs` (backup/restore logic)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/knowledge_store.rs` (store API)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/lib.rs` (CLI command wiring)
- Depth doc: `tmp/unified-depth/10-neuro/` (knowledge lifecycle)

## Steps
1. Read existing knowledge store and lifecycle:
   ```bash
   grep -n 'pub fn\|pub async fn' /Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/lifecycle.rs | head -20
   grep -n 'pub fn\|pub async fn' /Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/knowledge_store.rs | head -20
   ```

2. Read existing CLI knowledge commands:
   ```bash
   grep -rn 'knowledge.*backup\|knowledge.*restore\|KnowledgeBackup\|KnowledgeRestore' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/ --include='*.rs' | head -10
   ```

3. Read the HDC fingerprint module:
   ```bash
   grep -n 'pub fn\|hdc_fingerprint\|HdcFingerprint' /Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/hdc.rs | head -10
   ```

4. Implement `backup()` in lifecycle.rs:
   ```rust
   /// Create a knowledge backup archive.
   ///
   /// Serializes all signals with:
   /// - Signal content and metadata
   /// - Confidence scores
   /// - Tier classifications
   /// - HDC fingerprints for similarity matching on restore
   /// - Lineage references
   pub async fn backup(store: &KnowledgeStore, output_path: &Path) -> Result<BackupManifest> {
       let signals = store.all_signals().await?;
       let archive = BackupArchive {
           version: 1,
           created_at: Utc::now(),
           signal_count: signals.len(),
           signals: signals.iter().map(|s| BackupEntry {
               signal: s.clone(),
               fingerprint: s.hdc_fingerprint().cloned(),
               confidence: s.confidence(),
               tier: s.tier(),
           }).collect(),
       };
       // Serialize to file
       // ...
   }
   ```

5. Implement `restore()` with generational decay:
   ```rust
   /// Restore knowledge from a backup archive.
   ///
   /// Applies 0.85^N confidence decay where N = generational_distance.
   /// Routes each signal through quarantine → validate → adopt pipeline.
   pub async fn restore(
       store: &KnowledgeStore,
       archive_path: &Path,
       generational_distance: u32,
   ) -> Result<RestoreReport> {
       let archive: BackupArchive = load_archive(archive_path)?;
       let decay_factor = 0.85f32.powi(generational_distance as i32);

       let mut report = RestoreReport::default();
       for entry in archive.signals {
           let decayed_confidence = entry.confidence * decay_factor;

           // Stage 1: Quarantine — check for conflicts
           if let Some(conflict) = store.find_conflict(&entry.signal).await? {
               report.conflicts.push(conflict);
               continue;
           }

           // Stage 2: Validate — verify signal integrity
           if !validate_signal_integrity(&entry) {
               report.rejected.push(entry.signal.id());
               continue;
           }

           // Stage 3: Adopt — insert with decayed confidence
           store.insert_with_confidence(entry.signal, decayed_confidence).await?;
           report.adopted += 1;
       }
       Ok(report)
   }
   ```

6. Wire into CLI commands:
   - `roko knowledge backup` → calls `lifecycle::backup(store, path)`
   - `roko knowledge restore <path>` → calls `lifecycle::restore(store, path, distance)`
   - Add `--generational-distance` flag (default 1)

7. Write tests:
   - Backup serializes correct number of signals
   - Restore applies decay correctly (0.85^1, 0.85^2, etc.)
   - Conflicts are detected and reported
   - Invalid signals are rejected

## Verification
```bash
cargo check -p roko-neuro
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo test -p roko-neuro -- lifecycle
cargo check -p roko-cli
```

## What NOT to do
- Do NOT modify the knowledge_store.rs internal data structures — use its public API
- Do NOT add compression (gzip, zstd) — plain JSON/JSONL is sufficient for now
- Do NOT wire into automatic triggers — backup/restore is operator-initiated only
- Do NOT skip the decay step — it is essential for genomic bottleneck semantics
- Do NOT add network I/O — backup is local filesystem only
