# M068 — CaMeL Monitor Verify Cell

## Objective
Implement the CaMeL Monitor as a Verify-protocol Cell that checks CamelTag invariants on every Extension dispatch. The monitor flags IFC violations, runs outside the modifiable surface (an agent cannot modify its own CaMeL monitor), and emits alert Pulses when violations are detected. This is the enforcement mechanism for the information flow control system.

## Scope
- Crates: `roko-gate`
- Files: `crates/roko-gate/src/camel_monitor.rs` (new), `crates/roko-gate/src/lib.rs`
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.1
- Spec ref: `tmp/unified/17-SECURITY-MODEL.md` SS3.3

## Steps
1. Read the existing Verify/gate infrastructure:
   ```bash
   grep -rn 'pub trait.*Gate\|pub trait.*Verify\|impl.*Gate' crates/roko-gate/src/ --include='*.rs' | head -15
   ls crates/roko-gate/src/
   ```

2. Implement the CaMeL Monitor in `crates/roko-gate/src/camel_monitor.rs`:
   ```rust
   pub struct CamelMonitor {
       bus: Arc<dyn Bus>,
       violation_count: AtomicU64,
   }

   impl CamelMonitor {
       pub fn new(bus: Arc<dyn Bus>) -> Self;

       /// Check a single IFC transition for violations.
       pub fn check_transition(
           &self,
           input_tag: &CamelTag,
           output_tag: &CamelTag,
           handler: &str,
       ) -> CamelVerdict;

       /// Subscribe to all IFC audit topics and monitor continuously.
       pub async fn monitor_loop(&self, cancel: CancellationToken);
   }

   pub enum CamelVerdict {
       Clean,
       Violation(IFCViolation),
   }

   pub struct IFCViolation {
       pub kind: ViolationKind,
       pub handler: String,
       pub input_taint: TaintLevel,
       pub output_taint: TaintLevel,
       pub timestamp: DateTime<Utc>,
   }

   pub enum ViolationKind {
       TaintElevation,
       CapabilityLaundering,
       MissingProvenance,
       ProvenanceGap,
   }
   ```

3. The monitor checks:
   - Taint never decreases (Untrusted -> Local is a violation)
   - Capabilities never widen (gaining a capability the input did not have)
   - Provenance chain has no gaps (every transition has an entry)
   - No handler appears to launder capabilities through intermediate steps

4. On violation detection:
   - Emit alert Pulse on `security:ifc:violation` topic
   - Increment violation counter
   - Log the violation with full context

5. The monitor runs OUTSIDE the Agent's modifiable surface. It is registered by the runtime, not by the Agent. The Agent cannot unregister or modify its monitor.

6. Write tests:
   - Inject a taint elevation -> monitor catches it and emits alert Pulse
   - Clean transitions pass without alerts
   - Missing provenance is flagged
   - Violation counter increments correctly

## Verification
```bash
cargo check -p roko-gate
cargo clippy -p roko-gate --no-deps -- -D warnings
cargo test -p roko-gate -- camel_monitor
```

## What NOT to do
- Do NOT give the Agent API access to modify or disable the monitor
- Do NOT block execution on monitor checks -- check asynchronously and flag violations
- Do NOT add network calls -- the monitor is a local in-process component
- Do NOT implement remediation (auto-fix violations) -- the monitor only detects and reports
