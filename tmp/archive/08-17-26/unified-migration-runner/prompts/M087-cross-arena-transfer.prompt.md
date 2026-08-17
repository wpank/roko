# M087 — Cross-Arena Transfer Detection

**[BLOCKED:depth]** -- This item depends on `tmp/unified-depth/19-arenas/` depth docs. The depth docs specify HDC fingerprint similarity thresholds, transfer credit algorithms, and cross-domain correlation methods.

## Objective
Implement cross-arena skill transfer detection: when skills learned in one arena transfer to another (detected via HDC fingerprint correlation between arena episodes), credit the originating arena. This measures genuine cross-domain transfer rather than assuming it, using the HDC fingerprint infrastructure for similarity computation.

## Scope
- Crates: `roko-primitives`
- Files: `crates/roko-primitives/src/transfer.rs` (new), `crates/roko-primitives/src/lib.rs`
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.5
- Spec ref: `tmp/unified/19-ARENAS-EVALS-BOUNTIES.md` SS8
- Depth docs: `tmp/unified-depth/19-arenas/` (pending)

## Steps
1. Read existing HDC fingerprint code:
   ```bash
   grep -rn 'hdc\|HdcFingerprint\|fingerprint\|similarity' crates/roko-primitives/src/ --include='*.rs' | head -15
   ```

2. Implement transfer detection in `crates/roko-primitives/src/transfer.rs`:
   ```rust
   pub struct TransferDetector {
       similarity_threshold: f64,
   }

   pub struct TransferEvent {
       pub source_arena: String,
       pub target_arena: String,
       pub skill_fingerprint: HdcFingerprint,
       pub similarity_score: f64,
       pub improvement_in_target: f64,
   }

   impl TransferDetector {
       pub fn detect(
           &self,
           source_episodes: &[Episode],
           target_episodes: &[Episode],
       ) -> Vec<TransferEvent>;
   }
   ```

3. Detection algorithm:
   - Compute HDC fingerprints for successful strategies in each arena
   - Cross-correlate fingerprints between arenas
   - When similarity exceeds threshold AND target arena score improves, record transfer event
   - Credit the originating arena

4. Write tests: skill learned in Coding arena improves score in Security Audit arena -> transfer detected and credited.

## Verification
```bash
cargo check -p roko-primitives
cargo clippy -p roko-primitives --no-deps -- -D warnings
cargo test -p roko-primitives -- transfer
```

## What NOT to do
- Do NOT assume transfer -- measure it via HDC correlation
- Do NOT proceed without depth docs for similarity thresholds
- Do NOT implement causal inference -- correlation-based detection is sufficient
- Do NOT couple to specific arena implementations -- use generic Episode + fingerprint
