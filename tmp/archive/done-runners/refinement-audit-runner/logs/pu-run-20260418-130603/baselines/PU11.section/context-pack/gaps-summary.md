# Gap Inventory — 11 Safety

## Focus Now

### 1. Orchestrator Safety Is Under-Documented — HIGH

- `crates/roko-orchestrator/src/safety/` is the biggest invisible shipping surface,
- several PRDs describe its contents as future or target design.

### 2. Doc 01 Capability Framing Is Materially Wrong — HIGH

- `Capability<K>` with PhantomData and typed marker kinds already ships,
- the docs still treat it as aspirational.

### 3. Doc 16 Title/Body Drift Is The Main Status Hotspot — HIGH

- the body already describes partial closure,
- the title and banners still imply a generic unresolved integration failure.

### 4. Taint And Audit Need Honest Narrowing, Not Inflation — MEDIUM

- `AuditChain` and `TaintTracker` are real,
- the docs need to separate shipping subset behavior from richer frontier designs.

### 5. Frontier Halo Needs Harder Banners — MEDIUM

- compliance-framework chapters,
- chain-safety chapters,
- cognitive-kernel chapters,
- forensic pre-compliance packaging,
- all need clearer "informational" or "Phase 2+" framing.

## Working Rule

If a batch starts requiring new Rust or Solidity to make the docs true,
that is usually the wrong batch. Mark the seam and defer it.
