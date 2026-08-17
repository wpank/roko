# M014 — Define TypeSchema enum in roko-core

## Objective
Add the TypeSchema enum that enables compile-time edge validation in Graphs.
TypeSchema describes what kind of Signal a Cell accepts as input and produces as output.
This is foundational for Graph validation (Phase 2).

## Scope
- Crates: `roko-core`
- Files: `crates/roko-core/src/cell.rs` (or new `type_schema.rs`)
- Phase ref: 02-PHASE-1-KERNEL.md §1.13

## Steps
1. Check if TypeSchema already exists:
   `grep -rn 'TypeSchema' crates/ --include='*.rs' | grep -v target/`

2. If not, create it in `crates/roko-core/src/cell.rs` (alongside Cell trait) or in
   a dedicated `crates/roko-core/src/type_schema.rs`:

   ```rust
   use crate::kind::Kind;
   use serde::{Deserialize, Serialize};
   use std::collections::BTreeMap;

   /// Schema describing what types a Cell port accepts or produces.
   ///
   /// Used for compile-time edge validation in Graphs.
   /// See: tmp/unified/02-CELL.md §3.1
   #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
   pub enum TypeSchema {
       /// Accepts any Signal.
       Any,
       /// Accepts Signals of a specific Kind.
       OfKind(Kind),
       /// Accepts Signals matching a JSON Schema.
       JsonSchema(serde_json::Value),
       /// Accepts Signals matching any of these schemas.
       OneOf(Vec<TypeSchema>),
       /// Accepts Signals matching all of these schemas.
       AllOf(Vec<TypeSchema>),
       /// Accepts arrays of Signals matching the inner schema.
       ArrayOf(Box<TypeSchema>),
       /// Accepts records with named fields.
       Record(BTreeMap<String, TypeSchema>),
   }

   impl TypeSchema {
       /// Check if a Signal's kind is compatible with this schema.
       pub fn is_compatible(&self, kind: &Kind) -> bool {
           match self {
               TypeSchema::Any => true,
               TypeSchema::OfKind(k) => k == kind,
               TypeSchema::OneOf(schemas) => schemas.iter().any(|s| s.is_compatible(kind)),
               TypeSchema::AllOf(schemas) => schemas.iter().all(|s| s.is_compatible(kind)),
               _ => true, // JsonSchema, ArrayOf, Record need deeper validation
           }
       }
   }

   impl Default for TypeSchema {
       fn default() -> Self {
           TypeSchema::Any
       }
   }
   ```

3. Add optional schema fields to the Cell trait:
   ```rust
   fn input_schema(&self) -> Option<&TypeSchema> { None }
   fn output_schema(&self) -> Option<&TypeSchema> { None }
   ```

4. Export from lib.rs:
   ```rust
   pub use cell::TypeSchema;
   // or
   pub use type_schema::TypeSchema;
   ```

## Verification
```bash
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
cargo test -p roko-core --lib --no-run
```

## What NOT to do
- Do NOT implement full JSON Schema validation — just the Kind-level check for now
- Do NOT add TypeSchema to existing trait methods — they stay as-is
- Do NOT wire into Graph validation yet — that's Phase 2
