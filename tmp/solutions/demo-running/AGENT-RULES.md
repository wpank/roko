# Agent Execution Rules

These rules apply to ANY agent executing work from this plan. They are non-negotiable.
Violating them means the work is not done, regardless of what code was written.

---

## Rule 1: Wire, Don't Build

**Before writing a single line of code**, search for existing implementations.
This codebase has 40% dead code from prior batch work. If what you need already exists
as a struct, trait, or function with zero callers, your job is to WIRE it, not recreate it.

```bash
grep -rn 'StructName\|function_name' crates/ --include='*.rs' | grep -v target/
```

If you build something that already exists, your work will be rejected.

**Known existing infrastructure (from 18-agent audit):**
- `WorkflowEngine` → roko-runtime (1876 lines, has `run()` method)
- `RuntimeEvent` → roko-core (THE canonical event type, 12 variants)
- `EventConsumer` trait → roko-core/src/foundation.rs:421
- `SseAdapter` → roko-serve/src/adapters.rs (RuntimeEvent → SSE)
- `DashboardEventBridge` → roko-serve/src/lib.rs:476 (RuntimeEvent → DashboardEvent)
- `InlineTerminal` + 11 primitives → roko-cli/src/inline/
- `Workspace` → roko-core/src/workspace.rs (public workspace path boundary)
- `RokoLayout` → roko-fs/src/layout.rs (lower-level layout catalog during migration)
- `PlanComplexity` → roko-gate (Trivial/Simple/Standard/Complex)
- `Verify` trait → roko-gate (gate plugin interface)
- `ComposedGatePipeline` → roko-gate (Sequential/Parallel/Voting/Fallback)
- `TimeoutConfig` → roko-core (9 Duration fields, on RokoConfig)
- `AdaptiveBudget` → roko-compose (scales to context_window)
- `EventStreamContext` → demo-app (SSE connection management)
- `useBenchSSE` → demo-app (existing SSE consumption hook)
- `RunConfig` → roko-cli/src/runner/types.rs:1231 (28 fields, holds Arc<RokoConfig>)
- `HttpEventSink` → roko-runtime (reusable by CLI + ACP; already wired on wp-arch2)

---

## Rule 2: One Engine — event_loop.rs (v2)

**The legacy orchestrate.rs is ALREADY feature-gated OFF.**
- It's `#[cfg(feature = "legacy-orchestrate")]`, NOT in default features
- `serve_runtime.rs` ALREADY calls `crate::runner::run()` (v2) at line 277
- No production binary includes orchestrate.rs

Do NOT:
- Wire anything into orchestrate.rs
- Add features to the legacy path
- Create dual implementations "for safety"
- Verify that serve uses v2 (it already does — confirmed by audit)

---

## Rule 3: End-to-End Verification Required

Every task must be verified by running the actual code path.

**Verification means ALL of these pass:**

1. **Compilation**: `cargo clippy --workspace --no-deps -- -D warnings` — zero errors
2. **Tests**: `cargo test --workspace` — zero failures
3. **Runtime observation**: Execute the actual CLI command or HTTP endpoint

For streaming work:
```bash
cargo run -p roko-cli -- serve &
curl -N http://127.0.0.1:6677/api/events/stream
# Trigger operation, confirm events appear
```

---

## Rule 4: No Band-Aids

Do NOT:
- Patch `None` fields with `Some(thing)` without understanding why they were None
- Add `if` branches around problems instead of fixing root causes
- Create "adapter" layers between broken abstractions instead of fixing the abstraction
- Duplicate logic because "the other place is too hard to modify"
- Add feature flags or `--legacy` modes to avoid making decisions
- Bridge between DashboardEvent and ServerEvent (use RuntimeEvent → EventConsumer)

DO:
- Fix the root cause
- Delete dead code that's in the way
- Restructure when the structure is wrong
- Converge duplicate implementations

---

## Rule 5: No Orphan Code

Every struct, trait, function, or module you create MUST have at least one caller
in the production code path (not just tests).

After finishing:
```bash
grep -rn 'YourNewThing' crates/ --include='*.rs' | grep -v target/ | grep -v '#\[test\]'
```

---

## Rule 6: Streaming Must Be Observable

Any event you claim to emit must be observable:

1. Start `roko serve`
2. Connect: `curl -N http://127.0.0.1:6677/api/events/stream`
3. Trigger the operation
4. See the event JSON in curl output

---

## Rule 7: Task Scope = Minimal Viable Slice

Each task produces ONE verifiable outcome. If you can't verify in under 5 minutes of
testing, it's too big.

---

## Rule 8: Delete What's Dead

If dead code is legacy-only (orchestrate.rs, deprecated paths), DELETE rather than WIRE.
Check if v2 already has an equivalent. If yes, delete the dead code.

---

## Rule 9: Dependencies Are Explicit

If your task depends on another, say so. Do not assume implicit ordering.

---

## Rule 10: State the Root Cause

Every doc and task must state the ROOT CAUSE.

Bad: "Events don't reach the frontend"
Good: "serve_runtime.rs creates a disconnected SharedStateHub (line 274) instead of
using AppState.state_hub"

---

## Rule 11: Config Fields Must Have Consumers

If you wire a config field, verify:
1. The field is read from `RokoConfig` at runtime
2. The read value actually changes behavior
3. The default matches current hardcoded behavior

**Key insight**: `RunConfig` already holds `Arc<RokoConfig>`. You do NOT need to thread
new parameters through function signatures to access config. Just use the existing field.

---

## Rule 12: When Blocked, Escalate — Don't Hack Around

If you encounter:
- A type missing a field → add the field (don't patch around it)
- A private function → make it pub (don't duplicate it)
- A dependency cycle → restructure (don't create micro-crates)

---

## Rule 13: RuntimeEvent Is Canonical

**The event flow is:**
```
RuntimeEvent (roko-core)
  → EventConsumer trait implementations:
    → SseAdapter (roko-serve) → SSE JSON to browsers
    → DashboardEventBridge (roko-serve) → DashboardEvent → StateHub → TUI
    → JsonlLogger → .roko/events.jsonl
    → HttpForwarder → POST to remote server (for subprocesses)
```

Do NOT:
- Add ServerEvent variants (RuntimeEvent is canonical)
- Bridge DashboardEvent → ServerEvent (wrong direction)
- Emit ServerEvent directly from runner code
- Create new event types when RuntimeEvent variants suffice

DO:
- Add variants to RuntimeEvent when new events are needed
- Add match arms to SseAdapter for new variants
- Use EventConsumer trait for any new event sink

---

## Rule 14: Use Existing Types, Don't Duplicate

| Need | Use This | NOT This |
|------|----------|----------|
| Event type | `RuntimeEvent` (roko-core) | ServerEvent, new enums |
| Path abstraction | `Workspace` (roko-core public boundary) + `RokoLayout` only for roko-fs internals/migration exceptions | New raw `.join(".roko/...")`, new public use of `RokoLayout` |
| Complexity level | `PlanComplexity` (roko-gate) | New Formality enum |
| Gate interface | `Verify` trait (roko-gate) | Inline match arms |
| Config access | `RunConfig.roko_config` | New parameter threading |
| SSE adapter | `SseAdapter` (roko-serve) | New SSE emission code |
| Event consumption | `EventConsumer` trait | Custom subscriber patterns |
| Workflow execution | `WorkflowEngine` (roko-runtime) | New facade |
| Inline output | `InlineTerminal` + primitives | `RunOutputSink` / `StderrSink` |
| SSE client hook | Build on `EventStreamContext` | New EventSource management |
