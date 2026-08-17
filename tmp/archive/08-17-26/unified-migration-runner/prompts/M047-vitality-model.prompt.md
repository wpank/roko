# M047 — Vitality Behavioral Phases

## Objective
Implement the vitality model: `vitality = remaining_budget / initial_budget` (0.0..1.0) with five behavioral phases that modulate Agent decision-making. Vitality creates economic pressure -- an Agent that has never faced resource constraints has never learned to prioritize. Phase transitions affect model tier selection, exploration vs exploitation balance, task acceptance criteria, and knowledge transfer behavior.

## Scope
- Crates: `roko-agent`
- Files: `crates/roko-agent/src/vitality.rs` (new), `crates/roko-agent/src/lib.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.5
- Spec ref: `tmp/unified/07-AGENT-RUNTIME.md` SS3 (Vitality)

## Steps
1. Check for existing vitality or budget-related code:
   ```bash
   grep -rn 'vitality\|Vitality\|budget\|Budget\|remaining_budget' crates/roko-agent/src/ --include='*.rs' | head -15
   grep -rn 'vitality\|Vitality' crates/roko-core/src/ --include='*.rs' | head -10
   ```

2. Define the vitality types in `crates/roko-agent/src/vitality.rs`:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   pub enum VitalityPhase {
       Thriving,       // 1.0 - 0.7
       Stable,         // 0.7 - 0.4
       Conservation,   // 0.4 - 0.2
       Declining,      // 0.2 - 0.05
       Terminal,       // < 0.05
   }

   pub struct VitalityTracker {
       initial_budget: f64,
       remaining_budget: f64,
       phase: VitalityPhase,
       phase_entered_at: DateTime<Utc>,
   }
   ```

3. Implement VitalityTracker methods:
   - `new(initial_budget: f64) -> Self`
   - `spend(amount: f64) -> VitalityPhase` -- deduct and return current phase
   - `vitality(&self) -> f64` -- current ratio
   - `phase(&self) -> VitalityPhase` -- current phase
   - `remaining(&self) -> f64`
   - `is_terminal(&self) -> bool`

4. Implement phase-dependent constraints:
   ```rust
   impl VitalityPhase {
       /// Maximum model tier allowed in this phase.
       pub fn max_model_tier(&self) -> u8;
       /// Exploration probability (higher = more exploration).
       pub fn exploration_rate(&self) -> f64;
       /// Whether to accept new tasks.
       pub fn accepts_tasks(&self) -> bool;
       /// Whether to prioritize knowledge transfer over task execution.
       pub fn prioritize_transfer(&self) -> bool;
   }
   ```

   | Phase | Max tier | Exploration | Accept tasks | Prioritize transfer |
   |---|---|---|---|---|
   | Thriving | T3 | 0.3 | yes | no |
   | Stable | T2 | 0.2 | yes | no |
   | Conservation | T1 | 0.1 | selective | no |
   | Declining | T0 | 0.0 | no | yes |
   | Terminal | T0 | 0.0 | no | yes |

5. Emit a Pulse on phase transitions (prepare the data; actual emission requires Bus access).

6. Write tests:
   - Agent with 30% budget remaining is in Conservation phase
   - Agent with 90% remaining is in Thriving phase
   - Conservation phase restricts to T0/T1 models
   - Spending below 0.05 transitions to Terminal
   - Phase transitions are monotonic (Thriving -> Stable -> ... -> Terminal, never backwards)

## Verification
```bash
cargo check -p roko-agent
cargo clippy -p roko-agent --no-deps -- -D warnings
cargo test -p roko-agent -- vitality
```

## What NOT to do
- Do NOT wire vitality into the dispatcher yet -- that requires integrating with the type-state Agent from M046
- Do NOT implement budget refill/top-up -- vitality declines monotonically
- Do NOT add real cost tracking from LLM providers -- use abstract spend amounts
- Do NOT couple to the CascadeRouter -- vitality constraints inform routing but are separate
