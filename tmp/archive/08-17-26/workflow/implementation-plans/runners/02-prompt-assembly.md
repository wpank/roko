# Runner 02 — Prompt Assembly Completion

> **Give this entire file to a fresh agent.** It contains everything needed to execute Plan 02.

---

## Context

You are working on the `roko` codebase at `/Users/will/dev/nunchi/roko/roko`. The goal is to make **every prompt** go through `PromptAssemblyService` using the 9-layer `SystemPromptBuilder`, eliminating inline `format!()` role prompts scattered across 6+ files.

**Read these files first (in order):**

1. `tmp/workflow/ANTI-PATTERNS.md`
2. `tmp/workflow/implementation-plans/02-prompt-assembly-completion.md` — full plan
3. `crates/roko-core/src/foundation.rs` — `PromptAssembler` trait, `PromptSpec`
4. `crates/roko-compose/src/prompt_assembly_service.rs` — existing `PromptAssemblyService`
5. `crates/roko-compose/src/system_prompt_builder.rs` — 9-layer builder
6. `crates/roko-orchestrator/src/service_factory.rs` — `ServiceFactory::build()`
7. `crates/roko-runtime/src/effect_driver.rs` — search for `prompt_assembler.assemble`
8. `crates/roko-acp/src/runner.rs` — search for `build_review_prompt`, `format!`
9. `crates/roko-acp/src/session.rs` — search for `build_system_prompt`, hardcoded roles
10. `crates/roko-cli/src/chat_session.rs` — search for `build_chat_system_prompt`
11. `crates/roko-compose/src/auction.rs` — VCG code to delete

---

## Work Items (Execute In Order)

### Step 1: Extend `PromptSpec` (02-A)

**File:** `crates/roko-core/src/foundation.rs`

Add these fields to `PromptSpec` (keep existing ones, add new):

```rust
pub struct PromptSpec {
    // existing
    pub role: Option<String>,
    pub task: Option<String>,
    pub workdir: Option<PathBuf>,
    pub gate_feedback: Vec<GateFeedback>,   // CHANGED: was Vec<String>
    pub anti_patterns: Vec<String>,

    // NEW
    pub plan_context: Option<String>,
    pub prior_task_outputs: Vec<String>,
    pub strategy_brief: Option<String>,
    pub review_findings: Vec<String>,
    pub attempt: u32,
    pub token_budget: Option<u32>,
    pub tool_allowlist: Option<Vec<String>>,
    pub warnings: Vec<String>,
}
```

Add the `GateFeedback` struct:

```rust
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GateFeedback {
    pub gate_name: String,
    pub rung: u8,
    pub passed: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}
```

Add `AssembledPrompt` and `PromptDiagnostics`:

```rust
pub struct AssembledPrompt {
    pub system: String,
    pub diagnostics: PromptDiagnostics,
}

#[derive(Debug, Clone, Default)]
pub struct PromptDiagnostics {
    pub included_sections: Vec<String>,
    pub dropped_sections: Vec<String>,
    pub estimated_tokens: u32,
    pub knowledge_ids: Vec<String>,
    pub playbook_ids: Vec<String>,
}
```

Update the `PromptAssembler` trait:

```rust
#[async_trait]
pub trait PromptAssembler: Send + Sync {
    async fn assemble(&self, spec: PromptSpec) -> Result<AssembledPrompt>;
    fn last_prompt_section_ids(&self) -> Vec<String> { Vec::new() }
    fn last_knowledge_ids(&self) -> Vec<String> { Vec::new() }
}
```

After: `cargo check --workspace` — fix all compilation errors from the changed signatures.

### Step 2: Fix naming collision (02-B)

1. In `crates/roko-cli/src/dispatch/prompt_builder.rs`: rename `struct PromptAssembler` → `TaskPromptComposer`. Update all imports.
2. In `crates/roko-compose/src/templates/assembly.rs`: rename `struct PromptAssembler` → `TemplateAssembler`. Update all imports.
3. Verify: `rg 'pub (struct|trait) PromptAssembler' crates/ --type rust` returns exactly 1 result (the trait in `foundation.rs`).

### Step 3: Migrate `roko run` (02-C-1)

**File:** `crates/roko-cli/src/run.rs`

Replace `dispatch_helpers::build_system_prompt_with_context_validated(...)` with:

```rust
let assembler = state.services.prompt_assembler();
let assembled = assembler.assemble(PromptSpec {
    role: Some(role.to_string()),
    task: Some(task_description.clone()),
    workdir: Some(workdir.clone()),
    gate_feedback: prior_gate_failures.clone(),
    attempt,
    ..Default::default()
}).await?;

let req = ModelCallRequest {
    system: Some(assembled.system),
    prompt_section_ids: assembled.diagnostics.included_sections.clone(),
    knowledge_ids: assembled.diagnostics.knowledge_ids.clone(),
    ..base_req
};
```

### Step 4: Migrate ACP runner review prompts (02-C-3)

**File:** `crates/roko-acp/src/runner.rs`

Find `build_review_prompt` and all inline `format!("You are...")` strings. Replace with:

```rust
let role = match config.review_strictness.as_str() {
    "quick" => "quick_reviewer",
    "thorough" => "auditor",
    _ => "reviewer",
};
let assembled = assembler.assemble(PromptSpec {
    role: Some(role.to_string()),
    task: Some(self.original_prompt.clone()),
    review_findings: prior_findings,
    ..Default::default()
}).await?;
```

### Step 5: Migrate ACP session (02-C-4)

**File:** `crates/roko-acp/src/session.rs`

Find `build_system_prompt` and hardcoded role strings. Replace with:

```rust
let role = match self.mode {
    SessionMode::Plan => "strategist",
    SessionMode::Research => "researcher",
    _ => "implementer",
};
let assembled = self.services.prompt_assembler.assemble(PromptSpec {
    role: Some(role.to_string()),
    task: Some(prompt_text),
    workdir: Some(self.workdir.clone()),
    ..Default::default()
}).await?;
```

### Step 6: Migrate chat (02-C-5)

**File:** `crates/roko-cli/src/chat_session.rs`

Find `build_chat_system_prompt`. Replace with `assembler.assemble(PromptSpec { role: Some("interactive_chat"), ... })`.

Add `interactive_chat` role to `crates/roko-core/src/builtin_roles/core_roles.toml`.

### Step 7: Delete VCG (02-D)

1. Delete `crates/roko-compose/src/auction.rs`
2. Remove `VcgWelfare` from strategy enum in `crates/roko-compose/src/strategy.rs`
3. Remove `vcg_allocate` calls from `crates/roko-compose/src/prompt.rs`
4. Remove re-exports from `crates/roko-compose/src/lib.rs`
5. Verify: `rg 'vcg_allocate|VcgWelfare' crates/ --type rust | grep -v test` returns 0

### Step 8: Update `PromptAssemblyService` impl

**File:** `crates/roko-compose/src/prompt_assembly_service.rs`

Update the `impl PromptAssembler for PromptAssemblyService` to:
- Return `AssembledPrompt` with populated `diagnostics`
- Pass new `PromptSpec` fields to the appropriate builder layers
- `gate_feedback` → Layer 4b
- `review_findings` → appended to Layer 4
- `strategy_brief` → Layer 4 header
- `warnings` → Layer 8 (replacing pheromones)
- `token_budget` → budget enforcement
- `tool_allowlist` → Layer 5 filtering

---

## Verification Checklist

```bash
# No inline role strings outside templates
rg 'You are (the|a|an) \*\*' crates/ --type rust | grep -v 'roko-compose/src/templates' | grep -v test
# MUST return 0

# VCG gone
rg 'vcg_allocate|VcgWelfare|VcgAuction' crates/ --type rust | grep -v test | grep -v '#\[cfg('
# MUST return 0

# Naming resolved
rg 'pub (struct|trait) PromptAssembler' crates/ --type rust
# MUST return exactly 1

# Compiles
cargo check --workspace
cargo test --workspace
```

---

## Critical Rules

1. **NEVER write `let prompt = format!("You are a ...")`.** All role identity strings live in templates under `roko-compose/src/templates/`.
2. **NEVER if/else on role for prompt content.** The role name selects the template; the runner only picks which role.
3. **NEVER skip `prompt_section_ids` in `ModelCallRequest`.** They feed section effectiveness learning.
4. **Keep the 9-layer order stable** — it's cache-aligned for Anthropic API prompt caching.
5. **Use `..Default::default()` liberally** on `PromptSpec` — only set fields you have data for.
