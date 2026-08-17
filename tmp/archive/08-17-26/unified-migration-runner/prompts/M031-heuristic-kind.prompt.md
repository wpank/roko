# M031 — Define Kind::Heuristic with calibration payload

## Objective
Add a `Heuristic` variant to the `Kind` enum with a structured payload containing `when`, `then`, `falsifier`, and `Calibration` data. The mandatory `falsifier` field specifies what observation would disprove the heuristic, enabling automatic falsification tracking.

## Scope
- Crates: `roko-core`
- Files:
  - `crates/roko-core/src/kind.rs` (Kind enum)
  - `crates/roko-core/src/signal_kinds.rs` (if heuristic-specific types live here)
  - `crates/roko-core/src/lib.rs` (exports)
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.7
- Spec ref: `tmp/unified/01-SIGNAL.md` §4 (Heuristic Kind), `tmp/unified/11-MEMORY-AND-KNOWLEDGE.md` §6

## Steps
1. Read the current Kind enum:
   ```bash
   grep -n -A 80 'pub enum Kind' crates/roko-core/src/kind.rs
   ```

2. Check if Heuristic already exists:
   ```bash
   grep -rn 'Heuristic' crates/roko-core/src/kind.rs
   grep -rn 'HeuristicPayload\|Calibration' crates/roko-core/src/ --include='*.rs'
   ```

3. Define the Calibration struct (in kind.rs or a new `heuristic.rs`):
   ```rust
   /// Calibration state for a Heuristic signal.
   ///
   /// Tracks how well the heuristic's predictions match reality.
   /// See: tmp/unified/11-MEMORY-AND-KNOWLEDGE.md §6.
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   pub struct HeuristicCalibration {
       /// Total number of trials (times the heuristic was relevant).
       pub trials: u32,
       /// Number of times the heuristic's prediction was confirmed.
       pub confirmations: u32,
       /// Number of times the heuristic's prediction was violated.
       pub violations: u32,
       /// Brier score (mean squared error of predictions).
       pub brier_score: f64,
       /// Wilson score confidence interval (lower, upper).
       pub wilson_ci: (f64, f64),
   }

   impl Default for HeuristicCalibration {
       fn default() -> Self {
           Self {
               trials: 0,
               confirmations: 0,
               violations: 0,
               brier_score: 0.0,
               wilson_ci: (0.0, 1.0), // maximum uncertainty
           }
       }
   }
   ```

4. Define the Heuristic payload:
   ```rust
   /// Payload for Kind::Heuristic signals.
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   pub struct HeuristicPayload {
       /// Condition: "When X is true..."
       pub when: String,
       /// Prediction: "...then Y happens."
       pub then: String,
       /// Falsifier: "This would disprove the heuristic: ..."
       /// Mandatory — forces every heuristic to be disprovable.
       pub falsifier: String,
       /// Calibration state (updated as evidence accumulates).
       #[serde(default)]
       pub calibration: HeuristicCalibration,
   }
   ```

5. Add `Heuristic` to the Kind enum:
   ```rust
   pub enum Kind {
       // ... existing variants ...
       /// A learned heuristic with calibration data.
       /// See: tmp/unified/01-SIGNAL.md §4.
       Heuristic,
       // Note: the payload lives in Body, not in Kind itself,
       // since Kind is used as a lightweight discriminator.
   }
   ```
   The `HeuristicPayload` is stored in the Signal's `Body` field (as JSON), with `Kind::Heuristic` as the discriminator. This follows the existing pattern where Kind is a lightweight enum and payload details are in Body.

6. Add helper methods:
   ```rust
   impl Engram {
       /// Create a heuristic Signal with the given payload.
       pub fn heuristic(payload: HeuristicPayload) -> Self {
           Self::builder(Kind::Heuristic)
               .body(Body::json(serde_json::to_value(&payload).unwrap()))
               .build()
       }

       /// Extract the heuristic payload if this is a Heuristic signal.
       pub fn as_heuristic(&self) -> Option<HeuristicPayload> {
           if self.kind == Kind::Heuristic {
               serde_json::from_value(self.body.to_json_value()?).ok()
           } else {
               None
           }
       }
   }
   ```

7. Implement Wilson score CI calculation:
   ```rust
   impl HeuristicCalibration {
       /// Recompute Wilson score confidence interval.
       pub fn update_wilson_ci(&mut self) {
           if self.trials == 0 { return; }
           let n = self.trials as f64;
           let p = self.confirmations as f64 / n;
           let z = 1.96; // 95% CI
           let denom = 1.0 + z * z / n;
           let center = (p + z * z / (2.0 * n)) / denom;
           let margin = z * ((p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt()) / denom;
           self.wilson_ci = ((center - margin).max(0.0), (center + margin).min(1.0));
       }
   }
   ```

8. Add tests for serialization round-trip, calibration update, and Wilson CI.

## Verification
```bash
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
cargo test -p roko-core -- heuristic
cargo test -p roko-core -- kind
```

## What NOT to do
- Do NOT store the full HeuristicPayload inside the Kind enum variant — Kind should stay lightweight
- Do NOT make HeuristicCalibration mutable from outside the module without going through update methods
- Do NOT add Heuristic-specific methods to the Engram struct that break the generic pattern — keep them as convenience helpers
- Do NOT wire calibration updates here — that's M032
