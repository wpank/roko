# M149 — Create Tool-to-Cell Trait Bridge

## Objective
Create a bridge between the existing `ToolDef` (the current tool metadata struct used throughout the codebase) and the new `Cell` trait (the unified protocol abstraction). Add an `impl Cell for ToolDefCell` wrapper that exposes ToolDef metadata as Cell metadata (id, name, capabilities, protocols, cost_estimate). This enables existing tools to participate in the unified Cell protocol without rewriting them. Do NOT replace ToolDef — wrap it.

## Scope
- Crates: `roko-std`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/tool/mod.rs` (add bridge)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/tool/registry.rs` (verify compatibility)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/` (Cell trait location)
- Depth doc: `tmp/unified-depth/13-builtin-catalog/` (tool-cell bridge)

## Steps
1. Read the existing ToolDef struct:
   ```bash
   grep -n 'pub struct ToolDef\|pub trait ToolRegistry' /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/tool/ -r --include='*.rs' | head -10
   grep -n 'pub struct ToolDef' /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/ -r --include='*.rs' | head -5
   ```

2. Read the Cell trait (if it exists from M012):
   ```bash
   grep -rn 'pub trait Cell\|trait Cell' /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/ --include='*.rs' | head -5
   grep -rn 'pub trait Cell\|trait Cell' /Users/will/dev/nunchi/roko/roko/crates/roko-std/src/ --include='*.rs' | head -5
   ```

3. Read ToolDef fields to understand what to map:
   ```bash
   grep -A 30 'pub struct ToolDef' /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/tool/mod.rs 2>/dev/null || grep -rn -A 30 'pub struct ToolDef' /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/ --include='*.rs' | head -40
   ```

4. Create `ToolDefCell` wrapper in `crates/roko-std/src/tool/mod.rs`:
   ```rust
   /// Bridge wrapper that adapts a `ToolDef` to the Cell protocol.
   ///
   /// This allows existing tools to participate in Cell-based composition
   /// and routing without modifying their implementation.
   pub struct ToolDefCell {
       inner: ToolDef,
   }

   impl ToolDefCell {
       pub fn new(tool: ToolDef) -> Self {
           Self { inner: tool }
       }

       pub fn inner(&self) -> &ToolDef {
           &self.inner
       }

       pub fn into_inner(self) -> ToolDef {
           self.inner
       }
   }
   ```

5. Implement Cell trait (or a local CellMetadata trait if Cell is not yet defined):
   ```rust
   impl ToolDefCell {
       /// Cell-compatible unique identifier.
       pub fn cell_id(&self) -> &str {
           &self.inner.name
       }

       /// Cell-compatible display name.
       pub fn cell_name(&self) -> &str {
           &self.inner.name
       }

       /// Capabilities exposed as Cell protocol tags.
       pub fn capabilities(&self) -> Vec<String> {
           // Map ToolDef capabilities/tags to Cell capability strings
       }

       /// Protocols this cell supports (always includes "Connect" for tools).
       pub fn protocols(&self) -> Vec<&'static str> {
           vec!["Connect"]
       }

       /// Estimated cost per invocation (from tick_budget if available).
       pub fn cost_estimate(&self) -> f64 {
           // Map from ToolDef's tick_budget or similar field
           0.01 // default minimal cost
       }
   }
   ```

6. Add conversion: `impl From<ToolDef> for ToolDefCell` and `impl From<ToolDefCell> for ToolDef`.

7. Write tests:
   - Round-trip: ToolDef → ToolDefCell → ToolDef preserves all fields
   - `cell_id()` matches `tool.name`
   - `protocols()` always includes "Connect"
   - `capabilities()` maps correctly from ToolDef tags

## Verification
```bash
cargo check -p roko-std
cargo clippy -p roko-std --no-deps -- -D warnings
cargo test -p roko-std -- tool
```

## What NOT to do
- Do NOT replace ToolDef anywhere — this is a *wrapper*, not a replacement
- Do NOT add roko-std as a dependency of roko-core — the bridge lives in roko-std
- Do NOT implement execution logic in ToolDefCell — it only exposes metadata
- Do NOT modify existing tool registry code — add alongside it
- Do NOT make ToolDefCell the primary type — ToolDef remains canonical
