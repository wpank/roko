# M057 — Rack: Graph + Macros + Slots

## Objective
Define the Rack struct: a Graph parameterized with Macros (user-adjustable knobs) and Slots (late-bound Cell references / jacks). Inspired by modular synthesis: a Rack is a preconfigured board where the performer adjusts parameters (Macros) and patches cables (Slots) without redesigning the circuit. Macros are expanded at load time via variable substitution in Graph TOML. Slots are bound at runtime.

## Scope
- Crates: `roko-orchestrator`
- Files: `crates/roko-orchestrator/src/graph/rack.rs` (new), `crates/roko-orchestrator/src/graph/mod.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.9
- Spec ref: `tmp/unified/04-SPECIALIZATIONS.md` SS3 (Rack)

## Steps
1. Check for existing Rack, Macro, or Slot types:
   ```bash
   grep -rn 'Rack\|Macro\|pub struct Slot' crates/roko-orchestrator/src/ --include='*.rs' | head -15
   grep -rn 'Rack\|Macro' crates/roko-core/src/ --include='*.rs' | head -10
   ```

2. Define types in `crates/roko-orchestrator/src/graph/rack.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Rack {
       pub graph: Graph,
       pub macros: Vec<Macro>,
       pub slots: Vec<Slot>,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Macro {
       pub name: String,
       pub description: String,
       pub type_schema: TypeSchema,
       pub default_value: Value,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Slot {
       pub name: String,
       pub description: String,
       pub cell_schema: CellSchema,
       pub bound_to: Option<CellId>,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct CellSchema {
       pub protocols: Vec<String>,
       pub input_schema: Option<TypeSchema>,
       pub output_schema: Option<TypeSchema>,
   }
   ```

3. Implement Macro expansion:
   ```rust
   impl Rack {
       /// Instantiate the Rack with macro values, producing a concrete Graph.
       pub fn instantiate(&self, macro_values: &HashMap<String, Value>) -> Result<Graph> {
           // 1. For each macro, use provided value or default
           // 2. Substitute ${macro_name} patterns in the Graph TOML
           // 3. Validate the resulting Graph
           // 4. Return the concrete Graph
       }
   }
   ```

4. Implement Slot binding:
   ```rust
   impl Rack {
       /// Bind a Cell to a Slot by name.
       pub fn bind_slot(&mut self, slot_name: &str, cell_id: CellId) -> Result<()>;
       /// Check if all slots are bound.
       pub fn all_slots_bound(&self) -> bool;
       /// Get unbound slot names.
       pub fn unbound_slots(&self) -> Vec<&str>;
   }
   ```

5. Support Rack TOML format:
   ```toml
   [rack]
   name = "code-review"
   version = "0.1.0"

   [[macros]]
   name = "review_depth"
   description = "How thorough the review should be"
   type = "string"
   default = "standard"

   [[slots]]
   name = "linter"
   description = "The linting Cell to use"
   protocols = ["Verify"]
   ```

6. Write tests:
   - Rack with 2 macros can be instantiated with different values
   - Macro substitution replaces `${macro_name}` in node labels and metadata
   - Slot binding validates Cell schema compatibility
   - Unbound slots are detected before execution

## Verification
```bash
cargo check -p roko-orchestrator
cargo clippy -p roko-orchestrator --no-deps -- -D warnings
cargo test -p roko-orchestrator -- rack
```

## What NOT to do
- Do NOT implement Rack execution -- use `instantiate()` to get a Graph, then execute the Graph normally
- Do NOT add a Rack registry or marketplace integration -- that is M062/M063
- Do NOT make Macros Turing-complete -- they are simple value substitution, not a template language
- Do NOT allow Slot binding to bypass capability checks
