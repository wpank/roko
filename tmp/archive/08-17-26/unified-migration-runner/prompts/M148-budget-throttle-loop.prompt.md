# M148 — Wire Budget-Aware Tier Throttling

## Objective
Wire budget-aware tier throttling into the orchestrate.rs dispatch loop. Track daily usage percentage from efficiency events and apply progressive throttling rules: 80% → theta slowdown, 90% → restrict to T0+T1, 95% → T0 only. Emit a `BudgetWarning` Pulse at the 80% threshold. This ensures roko gracefully degrades rather than hitting hard budget limits mid-execution.

## Scope
- Crates: `roko-cli`, `roko-runtime`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (throttle logic)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat.rs` (theta slowdown)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/event_bus.rs` (BudgetWarning event)
- Depth doc: `tmp/unified-depth/05-heartbeat/` (budget-aware scheduling)

## Steps
1. Read how efficiency events and budget are currently tracked:
   ```bash
   grep -rn 'efficiency\|daily_budget\|budget\|cost.*track\|usage_pct' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs | head -15
   grep -rn 'budget\|Budget\|daily_limit' /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/efficiency.rs | head -15
   ```

2. Read the existing event bus enum:
   ```bash
   grep -n 'pub enum RokoEvent\|Budget' /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/event_bus.rs | head -10
   ```

3. Add `BudgetState` tracker struct in orchestrate.rs:
   ```rust
   /// Tracks daily budget usage and applies progressive throttling.
   struct BudgetState {
       daily_limit_usd: f64,
       daily_spent_usd: f64,
       warning_emitted: bool,
   }

   impl BudgetState {
       fn usage_pct(&self) -> f64 {
           if self.daily_limit_usd <= 0.0 { return 0.0; }
           self.daily_spent_usd / self.daily_limit_usd * 100.0
       }

       fn record_spend(&mut self, cost_usd: f64) {
           self.daily_spent_usd += cost_usd;
       }

       /// Returns the tier restriction based on current usage.
       fn tier_restriction(&self) -> TierRestriction {
           let pct = self.usage_pct();
           if pct >= 95.0 {
               TierRestriction::T0Only
           } else if pct >= 90.0 {
               TierRestriction::T0T1Only
           } else {
               TierRestriction::None
           }
       }

       /// Returns the theta multiplier based on current usage.
       fn theta_multiplier(&self) -> f64 {
           let pct = self.usage_pct();
           if pct >= 90.0 { 4.0 }
           else if pct >= 80.0 { 2.0 }
           else { 1.0 }
       }
   }
   ```

4. Add `BudgetWarning` variant to `RokoEvent` (or use existing Pulse mechanism):
   ```bash
   grep -n 'enum RokoEvent' /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/event_bus.rs
   ```

5. Wire into dispatch path in orchestrate.rs:
   - After each agent dispatch returns, `budget_state.record_spend(cost)`
   - Before next dispatch, check `tier_restriction()` and pass to CascadeRouter
   - At 80% threshold (first crossing), emit BudgetWarning Pulse and log warning
   - Apply `theta_multiplier()` to HeartbeatPolicy theta interval

6. Read daily_limit from config:
   ```bash
   grep -rn 'daily.*budget\|budget.*daily\|spending_limit\|cost.*limit' /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/ --include='*.rs' | head -10
   ```
   If config field exists, use it. If not, add `daily_budget_usd: Option<f64>` to the learning config section.

7. Write tests:
   - `BudgetState` at 79% → no restriction
   - `BudgetState` at 80% → theta_multiplier = 2.0
   - `BudgetState` at 90% → T0T1Only + theta_multiplier = 4.0
   - `BudgetState` at 95% → T0Only
   - Warning emitted exactly once at 80% crossing

## Verification
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
cargo test -p roko-cli -- budget
cargo check -p roko-runtime
```

## What NOT to do
- Do NOT hard-stop execution at 100% — graceful degradation only (operator decides to kill)
- Do NOT add external API calls to check budget — track locally from efficiency events
- Do NOT reset daily_spent automatically — that is a daemon cron job concern (out of scope)
- Do NOT modify the CascadeRouter internals — pass tier restriction as a filter hint
- Do NOT add new crate dependencies for budget tracking
