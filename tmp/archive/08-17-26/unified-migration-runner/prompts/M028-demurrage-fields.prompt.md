# M028 — Add demurrage fields to Signal (Engram)

## Objective
Add `balance: f64`, `demurrage_paid: f64`, and `last_touched_at: DateTime<Utc>` fields to the Engram struct, implementing the Gesellian demurrage model from the unified spec. A Signal starts with balance 1.0 and decays over time unless actively used (retrieved, cited, etc.).

## Scope
- Crates: `roko-core`
- Files:
  - `crates/roko-core/src/engram.rs` (Engram struct — add fields)
  - `crates/roko-core/src/demurrage.rs` (existing Demurrage trait — implement for Engram)
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.6
- Spec ref: `tmp/unified/01-SIGNAL.md` §6 (Demurrage), `tmp/unified/11-MEMORY-AND-KNOWLEDGE.md` §4

## Steps
1. Read the current Engram struct:
   ```bash
   grep -n -A 40 'pub struct Engram' crates/roko-core/src/engram.rs
   ```

2. Read the existing Demurrage trait:
   ```bash
   cat crates/roko-core/src/demurrage.rs
   ```

3. Add demurrage fields to Engram (with serde defaults for backward compatibility):
   ```rust
   pub struct Engram {
       // ... existing fields ...

       /// Attention/value balance in [0.0, 1.0]. Starts at 1.0, decays over time.
       /// See: tmp/unified/01-SIGNAL.md §6.
       #[serde(default = "default_balance")]
       pub balance: f64,

       /// Cumulative demurrage paid (total decay applied).
       #[serde(default)]
       pub demurrage_paid: f64,

       /// Last time this Signal was actively used (retrieved, cited, etc.).
       #[serde(default = "default_last_touched")]
       pub last_touched_at: DateTime<Utc>,
   }

   fn default_balance() -> f64 { 1.0 }
   fn default_last_touched() -> DateTime<Utc> { Utc::now() }
   ```

4. Implement the `Demurrage` trait for `Engram`:
   ```rust
   impl Demurrage for Engram {
       fn balance(&self) -> f64 { self.balance }
       fn demurrage_rate(&self) -> f64 {
           // Base rate from unified spec — configurable later
           0.001 // 0.1% per hour
       }
       fn tick(&mut self, elapsed_hours: f64) {
           if elapsed_hours > 0.0 {
               let decay = (1.0 - self.demurrage_rate()).powf(elapsed_hours);
               let lost = self.balance * (1.0 - decay);
               self.balance *= decay;
               self.demurrage_paid += lost;
           }
       }
       fn replenish(&mut self, amount: f64) {
           self.balance = (self.balance + amount).min(1.0);
           self.last_touched_at = Utc::now();
       }
   }
   ```

5. Update `Engram::builder()` to initialize demurrage fields:
   ```bash
   grep -n 'fn builder\|fn build' crates/roko-core/src/engram.rs | head -10
   ```
   Ensure `balance: 1.0`, `demurrage_paid: 0.0`, `last_touched_at: Utc::now()`.

6. Add tests:
   ```rust
   #[test]
   fn engram_decays_over_time() {
       let mut e = Engram::builder(Kind::Episode).body(Body::text("test")).build();
       assert_eq!(e.balance, 1.0);
       e.tick(720.0); // 30 days
       assert!(e.balance < 1.0);
       assert!(e.balance > 0.0);
       assert!(e.demurrage_paid > 0.0);
   }

   #[test]
   fn replenish_restores_balance() {
       let mut e = Engram::builder(Kind::Episode).body(Body::text("test")).build();
       e.tick(100.0);
       let decayed = e.balance;
       e.replenish(0.5);
       assert!(e.balance > decayed);
   }
   ```

7. Ensure backward compatibility — existing serialized Engrams without these fields should deserialize with defaults (balance=1.0, demurrage_paid=0.0).

## Verification
```bash
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
cargo test -p roko-core -- engram
cargo test -p roko-core -- demurrage
# Verify deserialization backward compat:
cargo test -p roko-core -- deserialize
```

## What NOT to do
- Do NOT change the Engram hash computation to include balance — balance is mutable state, not identity
- Do NOT apply demurrage automatically on every access — tick() must be called explicitly
- Do NOT change the serde format of existing fields — only add new fields with defaults
- Do NOT wire demurrage into Store yet — that's M030
