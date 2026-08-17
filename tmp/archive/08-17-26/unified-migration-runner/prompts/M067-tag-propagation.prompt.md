# M067 — IFC Tag Propagation Rules

## Objective
Implement the four CaMeL tag propagation rules in the Extension dispatch path. Rule 1: input tags propagate to outputs. Rule 2: Extensions cannot elevate taint (Untrusted -> Trusted is forbidden). Rule 3: decision enums carry the tag of the data that influenced them. Rule 4: every tag transition is logged as a Pulse for audit. These rules ensure that capability provenance is tracked and data cannot be "laundered" through Extensions.

## Scope
- Crates: `roko-agent`
- Files: `crates/roko-agent/src/extensions/dispatch.rs` (modify or new), `crates/roko-agent/src/extensions/mod.rs`
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.1
- Spec ref: `tmp/unified/08-EXTENSION-SYSTEM.md` SS3.2 (Propagation Rules)

## Steps
1. Read the existing Extension dispatch code:
   ```bash
   grep -rn 'dispatch\|Dispatch\|run_hook\|run_extension' crates/roko-agent/src/extensions/ --include='*.rs' | head -15
   cat crates/roko-agent/src/extensions/dispatch.rs 2>/dev/null | head -50
   ```

2. Read the CamelTag types from M066:
   ```bash
   cat crates/roko-core/src/camel.rs 2>/dev/null | head -40
   ```

3. Implement Rule 1 -- Input propagation:
   ```rust
   fn propagate_input_tag(input_tag: &CamelTag, output: &mut Signal) {
       // Output inherits input's taint level (at minimum)
       // Output capabilities = intersection of input capabilities and handler capabilities
   }
   ```

4. Implement Rule 2 -- No elevation:
   ```rust
   fn enforce_no_elevation(input_tag: &CamelTag, output_tag: &CamelTag) -> Result<(), IFCViolation> {
       if output_tag.taint_level < input_tag.taint_level {
           return Err(IFCViolation::TaintElevation {
               input: input_tag.taint_level,
               output: output_tag.taint_level,
           });
       }
       Ok(())
   }
   ```

5. Implement Rule 3 -- Decision tag inheritance:
   When an Extension returns a decision (FilterDecision, ToolDecision, RouteDecision), the decision inherits the tag of the data that influenced it, plus the Extension's own provenance.

6. Implement Rule 4 -- Audit trail:
   Every tag transition emits a Pulse on `extension:{name}:ifc` topic:
   ```rust
   fn emit_ifc_audit(bus: &dyn Bus, extension_name: &str, input_tag: &CamelTag, output_tag: &CamelTag) {
       // Publish Pulse with tag transition details
   }
   ```

7. Wire propagation into the ExtensionChain's hook runner (from M065).

8. Write tests:
   - Attempt to elevate Untrusted data to Trusted -> rejected with IFCViolation
   - Input tag propagates through a pass-through Extension unchanged
   - Transform Extension adds its provenance entry
   - Audit Pulse emitted on every hook execution

## Verification
```bash
cargo check -p roko-agent
cargo clippy -p roko-agent --no-deps -- -D warnings
cargo test -p roko-agent -- extensions::ifc
cargo test -p roko-agent -- extensions::propagation
```

## What NOT to do
- Do NOT implement the CaMeL Monitor Cell -- that is M068
- Do NOT add IFC to non-Extension code paths -- tag propagation is Extension-specific
- Do NOT relax the no-elevation rule with overrides -- it is unconditional
- Do NOT persist IFC audit Pulses to durable store -- they are ephemeral audit events
