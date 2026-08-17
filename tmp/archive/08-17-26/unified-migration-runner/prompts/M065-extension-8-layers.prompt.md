# M065 — Formalize 8 Extension Layers

## Objective
Formalize the 8 Extension interception layers as typed hook points in the Agent pipeline. Each layer (L0 Foundation through L7 Recovery) defines specific hooks that receive data flow and return modified data flow. Extensions are Cells that intercept another Cell's pipeline -- they do not replace or wrap the target, they hook into well-defined points. This formalizes what is currently ad-hoc interception logic.

## Scope
- Crates: `roko-agent`
- Files: `crates/roko-agent/src/extensions/` (refactor existing), `crates/roko-agent/src/extensions/layers.rs` (new)
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.1
- Spec ref: `tmp/unified/08-EXTENSION-SYSTEM.md` SS4-5

## Steps
1. Read the existing Extension infrastructure:
   ```bash
   grep -rn 'Extension\|extension\|Hook\|hook\|Layer\|layer' crates/roko-agent/src/extensions/ --include='*.rs' | head -20
   ls crates/roko-agent/src/extensions/
   ```

2. Define the 8 layers in `crates/roko-agent/src/extensions/layers.rs`:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
   pub enum ExtensionLayer {
       L0Foundation,  // PreExecute, PostExecute, OnError
       L1Perception,  // PreObserve, PostObserve
       L2Memory,      // PreRetrieve, PostRetrieve, PreStore, PostStore
       L3Cognition,   // PreReason, PostReason, PreCompose, PostCompose
       L4Action,      // PreExecuteTool, PostExecuteTool, OnToolError
       L5Social,      // PreCommunicate, PostCommunicate
       L6Meta,        // PreReflect, PostReflect
       L7Recovery,    // OnPanic, OnBudgetExhausted
   }
   ```

3. Define hook traits for each layer:
   ```rust
   #[async_trait]
   pub trait FoundationHooks: Send + Sync {
       async fn pre_execute(&self, ctx: &mut ExecutionContext) -> Result<()> { Ok(()) }
       async fn post_execute(&self, ctx: &mut ExecutionContext, result: &Signal) -> Result<()> { Ok(()) }
       async fn on_error(&self, ctx: &mut ExecutionContext, error: &CellError) -> ErrorAction { ErrorAction::Propagate }
   }

   #[async_trait]
   pub trait PerceptionHooks: Send + Sync {
       async fn pre_observe(&self, pulse: &Pulse) -> FilterDecision { FilterDecision::Pass }
       async fn post_observe(&self, pulse: &Pulse) -> Option<Pulse> { Some(pulse.clone()) }
   }

   // ... similarly for L2-L7
   ```

4. Define the `ExtensionChain` that manages registered extensions per layer:
   ```rust
   pub struct ExtensionChain {
       layers: [Vec<Box<dyn Extension>>; 8],
   }

   impl ExtensionChain {
       pub fn register(&mut self, layer: ExtensionLayer, extension: Box<dyn Extension>);
       pub async fn run_hooks(&self, layer: ExtensionLayer, hook: HookKind, data: &mut HookData) -> Result<()>;
   }
   ```

5. Note which layers operate on Pulses (L1, L5 -- ephemeral) vs Signals (L2, L3, L4, L6 -- durable) vs lifecycle events (L0, L7).

6. Write tests:
   - Extension registered at L4 can intercept a tool call
   - Extension can log the call and pass through unchanged
   - Hooks run in registration order within a layer
   - Extensions at different layers do not interfere

## Verification
```bash
cargo check -p roko-agent
cargo clippy -p roko-agent --no-deps -- -D warnings
cargo test -p roko-agent -- extensions::layers
```

## What NOT to do
- Do NOT implement CaMeL IFC tags here -- that is M066/M067
- Do NOT implement the CaMeL Monitor -- that is M068
- Do NOT wire into the Agent pipeline yet -- this defines the framework
- Do NOT remove existing extension code -- refactor to use the new layer types
