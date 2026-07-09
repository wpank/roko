# M080 — Wire Knowledge Publication from Memory

**[BLOCKED:depth]** -- This item depends on M078 (Rust chain clients) and M076/M077 (deployed contracts).

## Objective
Wire knowledge publication into the Memory store: when a Signal reaches Persistent tier, optionally publish its HDC fingerprint and metadata to the InsightStore on-chain. Publication is configurable per workspace (opt-in). This enables cross-agent knowledge discovery through the on-chain registry.

## Scope
- Crates: `roko-neuro`
- Files: `crates/roko-neuro/src/publish.rs` (new), `crates/roko-neuro/src/lib.rs`
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.4

## Steps
1. Read the Memory/neuro store code:
   ```bash
   grep -rn 'Persistent\|tier.*promote\|store.*signal' crates/roko-neuro/src/ --include='*.rs' | head -15
   ```

2. Read the InsightStore client from M078:
   ```bash
   grep -rn 'InsightStore\|publish.*insight\|insight.*client' crates/roko-chain/src/ --include='*.rs' | head -10
   ```

3. Implement the publisher in `crates/roko-neuro/src/publish.rs`:
   ```rust
   pub struct KnowledgePublisher {
       client: Arc<InsightStoreClient>,
       config: PublishConfig,
   }

   pub struct PublishConfig {
       pub enabled: bool,
       pub min_tier: String,  // "Persistent" by default
       pub min_confidence: f64,
       pub publish_fingerprint_only: bool,  // true = HDC fingerprint only, false = full metadata
   }

   impl KnowledgePublisher {
       pub async fn maybe_publish(&self, signal: &Signal) -> Result<Option<PublishResult>> {
           if !self.config.enabled { return Ok(None); }
           if signal.tier < self.config.min_tier { return Ok(None); }
           if signal.confidence < self.config.min_confidence { return Ok(None); }

           let fingerprint = signal.hdc_fingerprint();
           let metadata = signal.summary_metadata();
           let tx = self.client.publish(fingerprint, metadata).await?;
           Ok(Some(PublishResult { tx_hash: tx, signal_id: signal.id.clone() }))
       }
   }
   ```

4. Wire into the tier promotion path: when a Signal is promoted to Persistent, call `maybe_publish`.

5. Make configurable in roko.toml:
   ```toml
   [knowledge.publication]
   enabled = false
   min_confidence = 0.8
   ```

6. Write tests:
   - Enabled + Persistent Signal -> published
   - Disabled -> not published
   - Signal below min_confidence -> not published
   - Publication includes HDC fingerprint

## Verification
```bash
cargo check -p roko-neuro
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo test -p roko-neuro -- publish
```

## What NOT to do
- Do NOT enable publication by default -- it must be opt-in
- Do NOT publish full Signal content on-chain -- only fingerprint + summary metadata
- Do NOT block tier promotion on publication failure -- publish asynchronously
- Do NOT proceed without M078 chain clients
