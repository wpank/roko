# M066 — Define CamelTag Types

## Objective
Define the CamelTag types for CaMeL information flow control (IFC). CamelTags attach to Signals flowing through Extensions, tracking capability provenance and taint level. Four taint levels (Trusted, Local, External, Untrusted) establish a lattice where data can never be elevated -- only maintained or demoted. This is the foundation for the security guarantees in the Extension system.

## Scope
- Crates: `roko-core`
- Files: `crates/roko-core/src/camel.rs` (new), `crates/roko-core/src/lib.rs`
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.1
- Spec ref: `tmp/unified/08-EXTENSION-SYSTEM.md` SS3 (CaMeL IFC), `tmp/unified/17-SECURITY-MODEL.md` SS3

## Steps
1. Check for existing CaMeL or taint-related types:
   ```bash
   grep -rn 'CamelTag\|CaMeL\|camel\|Taint\|taint\|Provenance\|provenance' crates/roko-core/src/ --include='*.rs' | head -10
   grep -rn 'CamelTag\|taint' crates/roko-agent/src/ --include='*.rs' | head -10
   ```

2. Define CamelTag types in `crates/roko-core/src/camel.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct CamelTag {
       pub capabilities: CapabilitySet,
       pub provenance: Vec<ProvenanceEntry>,
       pub taint_level: TaintLevel,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ProvenanceEntry {
       pub handler: String,
       pub timestamp: DateTime<Utc>,
       pub operation: TagOperation,
   }

   #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
   pub enum TaintLevel {
       Trusted,     // system-generated, highest trust
       Local,       // locally verified
       External,    // from external source
       Untrusted,   // unverified external, lowest trust
   }

   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   pub enum TagOperation {
       Passthrough,
       Transform,
       Merge,
   }
   ```

3. Implement tag operations:
   ```rust
   impl CamelTag {
       pub fn new(taint: TaintLevel, capabilities: CapabilitySet) -> Self;
       /// Merge two tags: taint = max(a.taint, b.taint), capabilities = intersection
       pub fn merge(&self, other: &CamelTag) -> CamelTag;
       /// Add provenance entry
       pub fn add_provenance(&mut self, handler: &str, op: TagOperation);
       /// Check if tag allows a capability
       pub fn allows(&self, capability: &str) -> bool;
   }
   ```

4. Implement the no-elevation invariant:
   ```rust
   impl TaintLevel {
       /// Returns the lower-trust level of two taint levels.
       /// Used to enforce: output taint >= input taint (can only demote, never elevate).
       pub fn demote(self, other: TaintLevel) -> TaintLevel {
           std::cmp::max(self, other)  // Higher ordinal = lower trust
       }
   }
   ```

5. Define `CapabilitySet` as a thin wrapper around `HashSet<String>`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct CapabilitySet(HashSet<String>);

   impl CapabilitySet {
       pub fn intersection(&self, other: &CapabilitySet) -> CapabilitySet;
       pub fn contains(&self, cap: &str) -> bool;
   }
   ```

6. Export all types from roko-core lib.rs.

7. Write tests:
   - Signal tagged Untrusted retains that tag through operations
   - Merging Trusted + Untrusted produces Untrusted
   - Capability intersection correctly narrows permissions
   - Provenance chain tracks all handlers

## Verification
```bash
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
cargo test -p roko-core -- camel
```

## What NOT to do
- Do NOT implement tag propagation rules -- that is M067
- Do NOT implement the CaMeL Monitor -- that is M068
- Do NOT add tag fields to the Signal struct yet -- that is an integration step
- Do NOT add ZK attestation for tags -- that is a Phase 3.4 concern
