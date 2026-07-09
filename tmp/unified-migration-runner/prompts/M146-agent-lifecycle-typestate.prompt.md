# M146 — Add Type-State Lifecycle to Agent Creation

## Objective
The `lifecycle.rs` module in `roko-agent` already defines `AgentCoreManifest` and `DeploymentMode` but lacks compile-time state enforcement for agent lifecycle transitions. Add type-state lifecycle states (Initializing, Bootstrapping, Ready, Running, Draining, Terminated) using phantom types so that calling a method unavailable in the current state is a type error. Wire into `roko agent create` and `roko agent stop` CLI commands.

## Scope
- Crates: `roko-agent`, `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/lifecycle.rs` (extend with type-states)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/lib.rs` (re-export)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/lib.rs` (wire into CLI commands)
- Depth doc: `tmp/unified-depth/07-agent-runtime/` (lifecycle protocol)

## Steps
1. Read the existing lifecycle module:
   ```bash
   grep -n 'pub struct\|pub enum\|pub fn\|pub trait' /Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/lifecycle.rs | head -20
   ```

2. Read how agent create/stop currently work in CLI:
   ```bash
   grep -n 'agent.*create\|agent.*stop\|AgentCreate\|AgentStop' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/lib.rs | head -10
   grep -rn 'fn.*create_agent\|fn.*stop_agent\|fn.*agent_create' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/ --include='*.rs' | head -10
   ```

3. Define sealed type-state types in `lifecycle.rs`:
   ```rust
   mod sealed { pub trait Sealed {} }

   /// Marker trait for agent lifecycle states.
   pub trait LifecycleState: sealed::Sealed + Send + Sync + 'static {}

   /// Agent is being initialized (config validated, resources allocated).
   pub struct Initializing;
   /// Agent is bootstrapping (loading extensions, connecting MCP).
   pub struct Bootstrapping;
   /// Agent is ready to receive work but not yet executing.
   pub struct Ready;
   /// Agent is actively executing tasks.
   pub struct Running;
   /// Agent is draining (finishing current work, no new tasks).
   pub struct Draining;
   /// Agent has terminated (all resources released).
   pub struct Terminated;

   impl sealed::Sealed for Initializing {}
   impl sealed::Sealed for Bootstrapping {}
   impl sealed::Sealed for Ready {}
   impl sealed::Sealed for Running {}
   impl sealed::Sealed for Draining {}
   impl sealed::Sealed for Terminated {}

   impl LifecycleState for Initializing {}
   impl LifecycleState for Bootstrapping {}
   impl LifecycleState for Ready {}
   impl LifecycleState for Running {}
   impl LifecycleState for Draining {}
   impl LifecycleState for Terminated {}
   ```

4. Define `ManagedAgent<S: LifecycleState>`:
   ```rust
   pub struct ManagedAgent<S: LifecycleState> {
       pub id: String,
       pub manifest: AgentCoreManifest,
       pub created_at: chrono::DateTime<chrono::Utc>,
       _state: std::marker::PhantomData<S>,
   }
   ```

5. Implement state-specific transitions (consuming self):
   - `ManagedAgent<Initializing>`: `bootstrap() -> Result<ManagedAgent<Bootstrapping>>`
   - `ManagedAgent<Bootstrapping>`: `ready() -> Result<ManagedAgent<Ready>>`
   - `ManagedAgent<Ready>`: `start() -> Result<ManagedAgent<Running>>`, `terminate() -> ManagedAgent<Terminated>`
   - `ManagedAgent<Running>`: `drain() -> ManagedAgent<Draining>`, `terminate() -> ManagedAgent<Terminated>`
   - `ManagedAgent<Draining>`: `finish() -> ManagedAgent<Terminated>`

6. Add `DynManagedAgent` enum for runtime dispatch:
   ```rust
   pub enum DynManagedAgent {
       Initializing(ManagedAgent<Initializing>),
       Bootstrapping(ManagedAgent<Bootstrapping>),
       Ready(ManagedAgent<Ready>),
       Running(ManagedAgent<Running>),
       Draining(ManagedAgent<Draining>),
       Terminated(ManagedAgent<Terminated>),
   }
   ```

7. Wire into CLI:
   - `roko agent create` → creates `ManagedAgent<Initializing>`, transitions through Bootstrapping → Ready
   - `roko agent stop` → transitions Running → Draining → Terminated

8. Write tests:
   - Happy path: Initializing → Bootstrapping → Ready → Running → Draining → Terminated
   - Compile-time: `ManagedAgent<Ready>` does not have `drain()` (doc comment)
   - Transition emits log event

## Verification
```bash
cargo check -p roko-agent
cargo clippy -p roko-agent --no-deps -- -D warnings
cargo test -p roko-agent -- lifecycle
cargo check -p roko-cli
```

## What NOT to do
- Do NOT remove the existing `AgentCoreManifest` or `DeploymentMode` — extend alongside them
- Do NOT wire into the dispatcher hot path yet — this is the creation/stop flow only
- Do NOT add real LLM calls to transitions — transitions are pure state changes
- Do NOT rename existing agent.rs types — ManagedAgent is a parallel struct
- Do NOT add network I/O to transition methods — keep them synchronous
