# M030 — Wire demurrage into Store operations

## Objective
Integrate demurrage into the knowledge store's read and write paths. On `get()`, update `last_touched_at` and apply retrieval reinforcement. On `query()`, filter out Signals below the cold threshold (balance < 0.01). On `prune()`, archive cold Signals to ColdStore. This makes the decay system live in production.

## Scope
- Crates: `roko-neuro`, `roko-fs`, `roko-core`
- Files:
  - `crates/roko-neuro/src/store.rs` (knowledge store — primary target)
  - `crates/roko-neuro/src/demurrage.rs` (reinforcement from M029)
  - `crates/roko-fs/src/` (FileSubstrate — underlying storage)
  - `crates/roko-core/src/demurrage.rs` (Demurrage trait)
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.6
- Spec ref: `tmp/unified/11-MEMORY-AND-KNOWLEDGE.md` §4.4
- Depends on: M028 (demurrage fields), M029 (reinforcement)

## Steps
1. Read the knowledge store's current get/query/put interface:
   ```bash
   grep -n 'pub fn get\|pub fn query\|pub fn put\|pub fn prune\|pub async fn' crates/roko-neuro/src/store.rs | head -20
   grep -n 'pub trait.*Substrate\|pub trait.*Store' crates/roko-core/src/traits.rs | head -5
   ```

2. Find the FileSubstrate implementation:
   ```bash
   grep -rn 'impl.*Substrate\|impl.*Store.*for' crates/roko-fs/src/ --include='*.rs' | head -10
   ```

3. On `get()` — after retrieving a Signal, apply time decay and retrieval reinforcement:
   ```rust
   pub fn get(&mut self, id: &str) -> Option<&Engram> {
       if let Some(engram) = self.inner.get_mut(id) {
           let elapsed = hours_since(engram.last_touched_at);
           engram.tick(elapsed);
           apply_reinforcement(engram, config.retrieved_bonus, tracker.count(id));
           Some(engram)
       } else {
           None
       }
   }
   ```

4. On `query()` — filter out cold Signals:
   ```rust
   pub fn query(&self, topic: &str) -> Vec<&Engram> {
       self.inner.query(topic)
           .filter(|e| e.balance >= COLD_THRESHOLD) // 0.01
           .collect()
   }
   ```
   Define `const COLD_THRESHOLD: f64 = 0.01;`

5. Add `prune()` or extend existing GC to archive cold Signals:
   ```rust
   pub fn prune_cold(&mut self) -> Vec<Engram> {
       let cold: Vec<_> = self.signals()
           .filter(|e| e.balance < COLD_THRESHOLD)
           .cloned()
           .collect();
       for e in &cold {
           self.remove(&e.id);
           // Archive to cold store if available
       }
       cold
   }
   ```

6. Add tier multipliers for demurrage rate (from §1.6):
   ```rust
   fn tier_multiplier(tier: &str) -> f64 {
       match tier {
           "transient" => 10.0,  // decays 10x faster
           "working" => 2.0,     // decays 2x faster
           "consolidated" => 1.0, // baseline
           "persistent" => 0.2,  // decays 5x slower
           _ => 1.0,
       }
   }
   ```

7. Add tests:
   ```rust
   #[test]
   fn cold_signals_excluded_from_query() {
       // Store a signal, decay it below threshold, verify query excludes it
   }

   #[test]
   fn prune_archives_cold_signals() {
       // Store signals, decay some below threshold, prune, verify archived
   }

   #[test]
   fn get_applies_reinforcement() {
       // Store a signal, get it, verify balance increased
   }
   ```

## Verification
```bash
cargo check -p roko-neuro
cargo check -p roko-fs
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo test -p roko-neuro -- store
cargo test -p roko-neuro -- demurrage
```

## What NOT to do
- Do NOT apply demurrage on write (put) — only on read (get/query) and explicit tick
- Do NOT modify the Substrate/Store trait in roko-core — add demurrage logic in roko-neuro's wrapper layer
- Do NOT remove cold Signals permanently on first prune — archive them to cold storage first
- Do NOT change the JSONL storage format in roko-fs — demurrage fields serialize naturally via serde
