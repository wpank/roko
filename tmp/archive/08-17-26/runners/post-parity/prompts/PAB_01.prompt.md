# PAB_01: Wire plugin extension hooks to execute declarative tool profiles and triggers

## Task
Make plugin extension hooks actually execute the actions declared in plugin manifests instead of just logging.

## Runner Context
Runner PAB, batch 1 of 4. No dependencies.

## Problem
`extension_loader.rs:45-98` has all plugin hooks as pure logging stubs:
```rust
async fn on_init(&mut self) -> Result<()> {
    info!("plugin extension initialized");
    Ok(())  // never executes declarative tool profiles or triggers
}
async fn pre_inference(&self, req: &mut InferenceRequest) -> Result<()> {
    debug!("plugin pre_inference hook");
    Ok(())  // never restricts or enriches the request
}
```

The module doc says: "When a plugin declares tool profiles or triggers, this wrapper is the right place to enforce them."

## Exact Changes

### Step 1: In `on_init`, load plugin manifest

```rust
async fn on_init(&mut self) -> Result<()> {
    info!(plugin = %self.plugin.name, "plugin extension initialized");
    // Parse manifest for tool profiles and triggers
    self.tool_allow_list = self.plugin.manifest.tools.as_ref()
        .map(|t| t.allowed.clone())
        .unwrap_or_default();
    self.prompt_templates = self.plugin.manifest.prompts.clone().unwrap_or_default();
    Ok(())
}
```

### Step 2: In `pre_inference`, apply tool filtering and prompt injection

```rust
async fn pre_inference(&self, req: &mut InferenceRequest) -> Result<()> {
    // Apply tool allow list from manifest
    if !self.tool_allow_list.is_empty() {
        req.allowed_tools.retain(|t| self.tool_allow_list.contains(&t.name));
    }
    // Inject prompt template if manifest declares one
    if let Some(template) = self.prompt_templates.get(&req.role) {
        req.system_prompt = format!("{}\n\n{}", req.system_prompt, template);
    }
    Ok(())
}
```

### Step 3: Fix hardcoded layer

```rust
// BEFORE
layer: ExtensionLayer::Cognition,
// AFTER
layer: self.plugin.manifest.layer
    .as_deref()
    .map(ExtensionLayer::from_str)
    .unwrap_or(Ok(ExtensionLayer::Cognition))?,
```

## Write Scope
- `crates/roko-cli/src/runner/extension_loader.rs`


## Verify
```bash
cargo build -p roko-cli 2>&1 | head -30
cargo test -p roko-cli 2>&1 | tail -20
```
## Acceptance Criteria
- Plugin tool profiles restrict which tools the agent can use
- Plugin prompt templates injected into the system prompt
- Plugin layer read from manifest, not hardcoded to Cognition
- Plugins with no manifest behave identically to current (no regression)

## Do NOT
- Change unrelated code in the same file
- Add features beyond what's specified
- Remove existing tests
