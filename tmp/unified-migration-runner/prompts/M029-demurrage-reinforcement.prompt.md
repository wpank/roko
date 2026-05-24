# M029 — Implement reinforcement kinds for demurrage

## Objective
Implement the reinforcement system that restores Signal balance when a Signal is actively used. Reinforcement kinds include Retrieved, Cited, GatePassed, Surprised, and AgentQuoted. Each kind restores balance by a configurable amount with novelty weighting: `bonus * 1/(1 + ln(freq))`, so frequently-cited Signals get diminishing returns.

## Scope
- Crates: `roko-learn`, `roko-neuro`
- Files:
  - `crates/roko-learn/src/reinforce_kind.rs` (existing ReinforceKind enum)
  - `crates/roko-neuro/src/` (knowledge store — apply reinforcement on access)
  - New: `crates/roko-neuro/src/demurrage.rs` (or extend existing)
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.6
- Spec ref: `tmp/unified/11-MEMORY-AND-KNOWLEDGE.md` §4.3 (Reinforcement)

## Steps
1. Read existing reinforcement types:
   ```bash
   cat crates/roko-learn/src/reinforce_kind.rs | head -60
   ```

2. Check if roko-neuro has demurrage support:
   ```bash
   ls crates/roko-neuro/src/demurrage* 2>/dev/null
   grep -rn 'demurrage\|reinforc\|replenish' crates/roko-neuro/src/ --include='*.rs' | head -10
   ```

3. Extend `ReinforceKind` with demurrage-specific variants if not present:
   ```rust
   // These may already exist — check first
   Retrieved { query: String },
   Cited { citing_signal_id: String },
   AgentQuoted { agent_id: String, context: String },
   Surprised { surprise_score: f64 },
   ```

4. Create `crates/roko-neuro/src/demurrage.rs` with the reinforcement logic:
   ```rust
   use roko_core::demurrage::Demurrage;
   use roko_core::Engram;

   /// Configurable reinforcement amounts per kind.
   pub struct ReinforcementConfig {
       pub retrieved_bonus: f64,    // default: 0.05
       pub cited_bonus: f64,        // default: 0.10
       pub gate_passed_bonus: f64,  // default: 0.15
       pub surprised_bonus: f64,    // default: 0.20
       pub agent_quoted_bonus: f64, // default: 0.08
   }

   /// Apply reinforcement with novelty weighting.
   ///
   /// Formula: bonus * 1/(1 + ln(freq))
   /// First use (freq=1): full bonus
   /// 10th use (freq=10): bonus * 0.30
   /// 100th use (freq=100): bonus * 0.18
   pub fn apply_reinforcement(
       engram: &mut Engram,
       base_bonus: f64,
       access_frequency: u64,
   ) {
       let novelty_weight = 1.0 / (1.0 + (access_frequency.max(1) as f64).ln());
       let effective_bonus = base_bonus * novelty_weight;
       engram.replenish(effective_bonus);
   }
   ```

5. Add a frequency tracking mechanism — a simple counter map persisted to `.roko/learn/access-frequency.json`:
   ```rust
   pub struct AccessFrequencyTracker {
       counts: HashMap<String, u64>,  // signal_id -> access count
       path: PathBuf,
   }
   ```

6. Add tests:
   ```rust
   #[test]
   fn novelty_weighting_diminishes_with_frequency() {
       let mut e = Engram::builder(Kind::Episode).body(Body::text("test")).build();
       e.tick(100.0);  // decay first
       let before = e.balance;
       apply_reinforcement(&mut e, 0.10, 1);   // first access
       let after_first = e.balance;
       e.tick(100.0);  // decay again
       apply_reinforcement(&mut e, 0.10, 100);  // 100th access
       // The 100th access should restore less than the 1st
   }
   ```

7. Export from roko-neuro lib.rs.

## Verification
```bash
cargo check -p roko-neuro
cargo check -p roko-learn
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo test -p roko-neuro -- demurrage
cargo test -p roko-neuro -- reinforce
```

## What NOT to do
- Do NOT call reinforcement automatically from every Store access — the caller decides when to reinforce
- Do NOT modify the ReinforceKind enum if the needed variants already exist
- Do NOT couple this to a specific store implementation — the reinforcement function takes a mutable Engram reference
- Do NOT add complex frequency persistence — a simple JSON file is sufficient for now
