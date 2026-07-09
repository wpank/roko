# Wiring Strategy — How to Avoid the "Built But Never Connected" Trap

## The Anti-Pattern

Roko has ~15K LOC that compiles, exports publicly, and does nothing at runtime. This
happened because development followed the pattern:

1. Read spec
2. Implement trait + struct
3. Write tests
4. Move to next spec item
5. **Never wire step 2 into any runtime path**

The result: sophisticated implementations (Bayesian confidence, active inference,
calibration policies, Pareto optimization) that sit in crate boundaries, never called.

## The Rule: No Build Without a Wire

**Every new piece of code must have a wiring target defined BEFORE implementation begins.**

A "wiring target" is one of:
- A CLI command that exercises it (`roko <something>`)
- A runtime path that calls it (WorkflowEngine, Runner v2, serve)
- A test that exercises the integration (not unit — integration)

If you can't name the wiring target, don't build it yet.

## The Checklist (per item)

Before marking any checklist item as done, verify:

```
[ ] Code compiles
[ ] At least one integration test exercises the code through its wiring target
[ ] Running `cargo run -p roko-cli -- <command>` triggers the new code
[ ] The code is called from a non-test, non-cfg(test) path
[ ] grep -rn 'NewThing' crates/ --include='*.rs' | grep -v test | grep -v target/
    shows at least one callsite outside the defining crate
```

## Build New, Wire Immediately, Delete Old Later

### Why "build new" beats "migrate old"

The existing Runner v2 event loop works. Rewriting it in place risks breaking a working
system. Instead:

1. **Build the new Engine as a separate crate** (`roko-engine` or within `roko-graph`)
2. **Wire it to a new CLI path first** — e.g., `roko run --engine graph` or `roko graph run`
3. **Run both paths in parallel** — old Runner v2 stays working, new Engine is opt-in
4. **Migrate when confident** — swap default engine, deprecate old path
5. **Delete old code** — only after new path handles all cases

This is how Runner v2 replaced orchestrate.rs: built alongside, became default, old path
feature-gated. Follow the same pattern.

### Why "delete old" matters

Dead code is worse than no code:
- It confuses AI assistants into using/extending the wrong path
- It adds compile time
- It creates false confidence ("we have Bayesian confidence estimation!" — no, you have
  code that implements it but is never called)

After wiring new code, actively delete or feature-gate the superseded path.

## The "Wire First" Development Flow

```
1. Define the wiring target
   → "This Cell will be called from WorkflowEngine when processing a `roko run` prompt"

2. Write the integration point FIRST (the call site)
   → Add the call in the runtime path, even if it calls a stub

3. Implement the actual logic
   → Fill in the stub with real behavior

4. Test through the wire
   → Run `roko run "test prompt"` and verify the new code executes

5. Only THEN write unit tests
   → Unit tests supplement the integration test, not replace it
```

## Specific Anti-Patterns to Avoid

### 1. "I'll wire it later"
If you can't wire it now, it means you're building the wrong thing or building in the
wrong order. Build the thing that CAN be wired now.

### 2. "The trait is done, I'll add impls later"
A trait with zero implementations is dead code. Build at least one implementation and
wire it in the same PR.

### 3. "Tests pass so it works"
Unit tests prove the implementation is correct in isolation. They don't prove it's
called from runtime. A function with 100% test coverage and zero runtime callsites
is still dead code.

### 4. "I added it to Cargo.toml dependencies"
Adding a dependency is not wiring. The dependency must be `use`d from a non-test path
that gets executed at runtime.

### 5. "I exported it from lib.rs"
Public exports are an API contract, not a wiring target. Something must import and call
the export from a runtime path.

## How This Applies to V2 Refactoring

### Phase 1 (Cell + Signal + Protocols)
- **Wire target**: Existing implementations of Store, Score, Verify, Route, Compose, React
  gain Cell as supertrait — they already have callers
- **Verification**: `cargo test --workspace` — existing tests must still pass
- **Risk**: Low — additive changes to working traits

### Phase 2 (Graph + Engine)
- **Wire target**: `roko graph run <file.toml>` — NEW CLI command
- **Verification**: Write a simple 3-node graph (Score → Compose → Verify) and run it
- **Risk**: Medium — new crate, but doesn't touch existing paths

### Phase 3 (Feeds + Graduation)
- **Wire target**: `roko feed subscribe <feed-id>` or Bus graduation in Engine
- **Verification**: Start a feed, see Pulses graduate to Signals in Store
- **Risk**: Low — new Cells on top of Engine

### Phase 4 (Migration)
- **Wire target**: `roko plan run` uses Engine instead of Runner v2
- **Verification**: Run an existing plan through both paths, compare results
- **Risk**: High — replacing the load-bearing path
