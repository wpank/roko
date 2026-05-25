# M046 — Type-State Agent Struct

## Objective
Define the type-state Agent struct with compile-time enforced lifecycle states: Provisioning, Active, Dreaming, Terminal. Each state restricts which operations are permitted. Calling a method unavailable in the current state is a type error, not a runtime error. This eliminates an entire class of "agent in wrong state" bugs that currently require runtime checks.

## Scope
- Crates: `roko-agent`
- Files: `crates/roko-agent/src/lifecycle.rs` (new), `crates/roko-agent/src/lib.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.5
- Spec ref: `tmp/unified/07-AGENT-RUNTIME.md` SS2 (Type-State Lifecycle)

## Steps
1. Read the current Agent struct and lifecycle management:
   ```bash
   grep -rn 'pub struct Agent\|AgentState\|AgentStatus' crates/roko-agent/src/ --include='*.rs' | head -15
   grep -rn 'fn start\|fn stop\|fn execute' crates/roko-agent/src/ --include='*.rs' | head -15
   ```

2. Define the type-state types in `crates/roko-agent/src/lifecycle.rs`:
   ```rust
   use std::marker::PhantomData;

   // Sealed trait pattern to prevent external implementations
   mod sealed {
       pub trait Sealed {}
   }

   pub trait AgentState: sealed::Sealed {}

   pub struct Provisioning;
   pub struct Active;
   pub struct Dreaming;
   pub struct Terminal;

   impl sealed::Sealed for Provisioning {}
   impl sealed::Sealed for Active {}
   impl sealed::Sealed for Dreaming {}
   impl sealed::Sealed for Terminal {}

   impl AgentState for Provisioning {}
   impl AgentState for Active {}
   impl AgentState for Dreaming {}
   impl AgentState for Terminal {}
   ```

3. Define `Agent<S: AgentState>`:
   ```rust
   pub struct Agent<S: AgentState> {
       pub id: AgentId,
       pub config: AgentConfig,
       // ... shared fields available in all states
       _state: PhantomData<S>,
   }
   ```

4. Implement state-specific methods:
   - `Agent<Provisioning>`: `load_extensions()`, `validate_space()`, `activate() -> Agent<Active>`
   - `Agent<Active>`: `tick()`, `execute()`, `query_memory()`, `sleep() -> Agent<Dreaming>`, `terminate() -> Agent<Terminal>`
   - `Agent<Dreaming>`: `run_dream_cycle()`, `query_memory_readonly()`, `wake() -> Agent<Active>`, `terminate() -> Agent<Terminal>`
   - `Agent<Terminal>`: `export_knowledge()`, `flush_episodes()`

5. Implement transition methods that consume self and return the new state:
   ```rust
   impl Agent<Provisioning> {
       pub fn activate(self) -> Result<Agent<Active>> {
           // Validate extensions loaded, space grants validated, memory initialized
           // Emit AgentStateTransition Pulse
           Ok(Agent { id: self.id, config: self.config, _state: PhantomData })
       }
   }
   ```

6. Add a runtime-typed wrapper for contexts that need dynamic dispatch:
   ```rust
   pub enum DynAgent {
       Provisioning(Agent<Provisioning>),
       Active(Agent<Active>),
       Dreaming(Agent<Dreaming>),
       Terminal(Agent<Terminal>),
   }
   ```

7. Write tests:
   - `Agent<Provisioning>` can call `activate()` -> `Agent<Active>`
   - `Agent<Active>` can call `sleep()` -> `Agent<Dreaming>`
   - Compile-time check: `Agent<Provisioning>` does NOT have `execute()` (add a doc comment noting this)
   - Transition emits correct Pulse

## Verification
```bash
cargo check -p roko-agent
cargo clippy -p roko-agent --no-deps -- -D warnings
cargo test -p roko-agent -- lifecycle
```

## What NOT to do
- Do NOT replace the existing Agent struct yet -- lifecycle.rs is a parallel implementation
- Do NOT wire into the dispatcher or CLI yet -- that requires broader refactoring
- Do NOT add vitality here -- that is M047
- Do NOT add slot management here -- that is M048
- Do NOT make the state transitions depend on real LLM calls
