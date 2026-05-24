# M021 — Align Pulse struct with unified spec

## Objective
A `Pulse` struct already exists in `crates/roko-core/src/pulse.rs` with fields `seq`, `topic`, `kind`, `body`, `created_at_ms`, `tags`. The unified spec (`tmp/unified/01-SIGNAL.md` §3) requires additional fields: `source: CellId`, `lineage_hint: Option<SignalRef>`, `trace_id: TraceId`. Add these fields and implement the `graduate() -> Signal` pathway (Pulse promotion to durable Signal).

## Scope
- Crates: `roko-core`
- Files:
  - `crates/roko-core/src/pulse.rs` (Pulse struct, lines ~75-88)
  - `crates/roko-core/src/engram.rs` (Signal/Engram struct — graduation target)
  - `crates/roko-core/src/lib.rs` (exports)
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.2
- Spec ref: `tmp/unified/01-SIGNAL.md` §3 (Pulse struct), §5 (Graduation)

## Steps
1. Read current Pulse struct:
   ```bash
   grep -n -A 20 'pub struct Pulse' crates/roko-core/src/pulse.rs
   ```

2. Check what identifier types exist:
   ```bash
   grep -rn 'CellId\|SignalRef\|TraceId\|EngineId' crates/roko-core/src/ --include='*.rs' | grep 'pub.*type\|pub.*struct' | head -10
   ```

3. Add missing fields to `Pulse`:
   ```rust
   pub struct Pulse {
       pub seq: u64,
       pub topic: Topic,
       pub kind: Kind,
       pub body: Body,
       pub created_at_ms: i64,
       pub tags: BTreeMap<String, String>,
       // New fields from unified spec §3:
       /// The Cell that produced this Pulse.
       #[serde(default, skip_serializing_if = "Option::is_none")]
       pub source: Option<String>,
       /// Optional link to a parent Signal for lineage tracking.
       #[serde(default, skip_serializing_if = "Option::is_none")]
       pub lineage_hint: Option<String>,
       /// Trace ID for distributed tracing correlation.
       #[serde(default, skip_serializing_if = "Option::is_none")]
       pub trace_id: Option<String>,
   }
   ```
   Use `Option<String>` if CellId/TraceId types don't exist yet. Use proper types if they do.

4. Update `Pulse::new()` and `PulseBuilder` to handle the new fields (defaulting to `None`).

5. Implement `Pulse::graduate()` that converts a Pulse into an Engram (Signal):
   ```rust
   impl Pulse {
       /// Promote this Pulse to a durable Signal (Engram).
       ///
       /// This is the only sanctioned path from transport traffic into the
       /// audit DAG. See: tmp/unified/01-SIGNAL.md §5.
       pub fn graduate(&self) -> Engram {
           Engram::builder(self.kind.clone())
               .body(self.body.clone())
               // Copy metadata from pulse tags
               .build()
       }
   }
   ```

6. Implement `Engram::to_pulse()` (lossy projection — drops DAG links, hash, etc.):
   ```rust
   impl Engram {
       /// Lossy projection to a Pulse (drops DAG links, hash).
       pub fn to_pulse(&self, seq: u64, topic: Topic) -> Pulse { ... }
   }
   ```

7. Add round-trip test: `Signal -> Pulse -> graduate -> Signal` preserves content.

8. Update PulseBuilder to support the new fields via chaining:
   ```rust
   .source("my-cell-id")
   .lineage_hint("parent-signal-ref")
   .trace_id("trace-abc-123")
   ```

## Verification
```bash
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
cargo test -p roko-core -- pulse
# Verify graduation works:
cargo test -p roko-core -- graduate
```

## What NOT to do
- Do NOT remove existing fields — the new fields are additions
- Do NOT make new fields required — use `Option` with `#[serde(default)]` for backward compat
- Do NOT change the serialization format of existing fields
- Do NOT introduce CellId/TraceId as new types unless they already exist — use String for now
