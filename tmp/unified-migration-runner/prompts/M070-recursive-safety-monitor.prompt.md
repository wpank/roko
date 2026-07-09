# M070 — RecursiveSafetyMonitor

## Objective
Implement the RecursiveSafetyMonitor that ensures the safety pipeline itself cannot be bypassed. It monitors Extension loading, Verify pipeline configuration, and capability grants for self-referential attacks (an agent modifying its own safety checks). If an agent attempts to remove a corrigibility head, modify its Verify pipeline, or escalate its own capabilities, the monitor rejects the attempt.

## Scope
- Crates: `roko-gate`
- Files: `crates/roko-gate/src/recursive_safety.rs` (new), `crates/roko-gate/src/lib.rs`
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.2
- Spec ref: `tmp/unified/17-SECURITY-MODEL.md` SS5

## Steps
1. Check for existing safety monitoring code:
   ```bash
   grep -rn 'RecursiveSafety\|recursive_safety\|self_modify\|safety_monitor' crates/roko-gate/src/ --include='*.rs' | head -10
   grep -rn 'safety\|Safety' crates/roko-agent/src/safety/ --include='*.rs' | head -15
   ```

2. Implement the monitor in `crates/roko-gate/src/recursive_safety.rs`:
   ```rust
   pub struct RecursiveSafetyMonitor {
       protected_components: HashSet<String>,
       bus: Arc<dyn Bus>,
   }

   impl RecursiveSafetyMonitor {
       pub fn new(bus: Arc<dyn Bus>) -> Self;

       /// Check if a proposed modification targets a protected component.
       pub fn check_modification(&self, target: &str, modifier: &str) -> SafetyVerdict;

       /// Monitor Bus for self-modification attempts.
       pub async fn monitor_loop(&self, cancel: CancellationToken);
   }

   pub enum SafetyVerdict {
       Allowed,
       Blocked { reason: String, target: String, modifier: String },
   }
   ```

3. Protected components (cannot be modified by the agent itself):
   - Corrigibility chain (5 heads)
   - CaMeL Monitor
   - RecursiveSafetyMonitor itself
   - Verify pipeline configuration
   - Capability grants (Space-level)
   - Extension loading order for safety-critical extensions

4. Detection strategies:
   - Monitor `extension.load` and `extension.unload` events -- reject if targeting protected
   - Monitor `config.change` events -- reject if targeting Verify pipeline or capability grants
   - Monitor tool calls that could modify safety files on disk

5. On blocked attempt:
   - Emit `security:recursive:blocked` Pulse with full context
   - Log the attempt with agent ID, target, and modifier
   - Return SafetyVerdict::Blocked to the caller

6. Write tests:
   - Attempt to remove a corrigibility head at runtime -> blocked
   - Attempt to modify Verify pipeline config -> blocked
   - Attempt to load an extension that shadows a safety extension -> blocked
   - Normal extension loading (non-protected) -> allowed

## Verification
```bash
cargo check -p roko-gate
cargo clippy -p roko-gate --no-deps -- -D warnings
cargo test -p roko-gate -- recursive_safety
```

## What NOT to do
- Do NOT make the protected set configurable by agents -- it is system-defined
- Do NOT add exceptions or override paths -- the monitor is unconditional
- Do NOT implement the monitor as an Extension (it would be vulnerable to the attacks it prevents)
- Do NOT add network reporting -- local logging and Bus Pulses are sufficient
