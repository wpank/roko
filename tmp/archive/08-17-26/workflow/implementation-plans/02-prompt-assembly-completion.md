# 02 — PromptAssemblyService: Finish the Unification

> Foundation Phase 0.2 of `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md`. Cross-references audit `tmp/workflow/13-prompt-assembly-audit.md`.

---

## Status (2026-05-01)

**PARTIAL.** The trait + service exist; some entry points use them; ACP runner / session, chat, plan dispatch use ad-hoc prompt building.

**What's done:**

- `roko_core::foundation::PromptAssembler` trait + `PromptSpec` struct — `crates/roko-core/src/foundation.rs:247-273`
- `roko_compose::PromptAssemblyService` (concrete impl) — `crates/roko-compose/src/prompt_assembly_service.rs`. Uses `SystemPromptBuilder` (9 layers).
- `SystemPromptBuilder` itself is canonical — `crates/roko-compose/src/system_prompt_builder.rs`
- `EffectDriver::spawn_agent` calls `prompt_assembler.assemble(PromptSpec { ... })` — `crates/roko-runtime/src/effect_driver.rs:122-140`
- `ServiceFactory::build` constructs `PromptAssemblyService` with knowledge store, episodes, playbooks, optional section-effectiveness, token budget, tools — `crates/roko-orchestrator/src/service_factory.rs`
- 13 templates implementing `RolePromptTemplate` exist in `crates/roko-compose/src/templates/`
- `SectionEffectivenessRegistry` exists — `crates/roko-learn/src/section_effect.rs`
- `PromptDiagnostics { included_sections, dropped_sections, estimated_tokens, ... }` exists — `crates/roko-cli/src/dispatch/prompt_builder.rs:172-185` (CLI-side only)

**What's not:**

- **Naming collision:** three different `PromptAssembler` symbols
  1. `roko_core::foundation::PromptAssembler` (the trait — canonical)
  2. `roko_cli::dispatch::prompt_builder::PromptAssembler` (CLI struct, not the trait)
  3. `roko_compose::templates::assembly::PromptAssembler` (template helper, not the trait)
- **Inline prompt strings still present:**
  - `crates/roko-acp/src/runner.rs` — `build_review_prompt` ~line 1512–1534, `format!("{context}\n\n{prompt}")` ~line 995
  - `crates/roko-acp/src/session.rs` — hardcoded role prompts for `plan`/`research`/default ~428-437
  - `crates/roko-cli/src/chat_session.rs` — `build_chat_system_prompt` and long `role_identity` string ~1411-1441 (uses `SystemPromptBuilder` directly, no provenance, no playbooks, no effectiveness)
  - `crates/roko-cli/src/dispatch/prompt_builder.rs` — `# Role\nYou are the **{}**...` ~line 293+
  - `crates/roko-cli/src/orchestrate.rs` — many ad-hoc `format!` prompts (feature-gated; do not migrate)
- **VCG auction not deleted.** `crates/roko-compose/src/auction.rs` still exports `vcg_allocate`; `prompt.rs` still calls it; `lib.rs` still re-exports auction symbols.
- **`PromptDiagnostics` is not on the foundation trait.** Live `PromptAssemblyService` exposes only `last_prompt_section_ids` / `last_knowledge_ids`. The richer diagnostics struct lives in CLI dispatch only.
- **Section effectiveness only updates from `FeedbackService`** but the per-section outcome wiring is incomplete — the `apply_section_outcome` path is reached only when the request carries provenance. Bypass paths (chat, ACP) never populate it.
- HTTP `/api/inference/complete` does not assemble layered system prompts — caller messages pass through verbatim.

---

## Goal

**Every** prompt sent to a model is built by `PromptAssemblyService` (or a thin facade implementing `roko_core::PromptAssembler`) using `SystemPromptBuilder`. Inline prompt strings outside template files are deleted. Section provenance flows through `ModelCallRequest.prompt_section_ids` so `FeedbackService` can update section effectiveness.

---

## Why This Exists (Anti-Patterns Eliminated)

- **#2 Inline Prompt Strings** — hardcoded format!() role prompts across 6+ files
- **#5 Hardcoded Role Behavior** — if/elsing review strictness inside the runner instead of templating it
- **#6 Feedback as Afterthought** — section effectiveness is the most informative learning signal, but most paths don't populate provenance
- **#7 Copy-Paste Between Runtimes** — three different "PromptAssembler" types

---

## Existing Code — Read These First

```247:273:crates/roko-core/src/foundation.rs
#[derive(Debug, Clone, Default)]
pub struct PromptSpec {
    pub role: Option<String>,
    pub task: Option<String>,
    pub workdir: Option<PathBuf>,
    pub gate_feedback: Vec<String>,
    pub anti_patterns: Vec<String>,
}

#[async_trait]
pub trait PromptAssembler: Send + Sync {
    async fn assemble(&self, spec: PromptSpec) -> Result<String>;
    fn last_prompt_section_ids(&self) -> Vec<String>;
    fn last_knowledge_ids(&self) -> Vec<String>;
}
```

The 9 layers (per `SystemPromptBuilder`):

1. **Role identity** (from `core_roles.toml` template)
2. **Conventions** (project rules from `roko.toml`)
3. **Domain context** (knowledge store query)
4. **Task description** (the user prompt + acceptance + file context + prior outputs)
4b. **Gate feedback** (structured, per-rung — not raw text)
5. **Tool instructions** (filtered by role's `tools.capabilities`)
6. **Skills / playbooks** (matched from `PlaybookStore`)
7. **Anti-patterns** (from `roko-neuro` knowledge store + safety bounds)
8. **Warnings** (replaces dead pheromones — gate failures, system issues)

Drop layer 9 (affect) per plan 15 (cognitive cleanup).

---

## Implementation Steps

### Step 1 — Extend `PromptSpec` to carry full request context

The current `PromptSpec` is too thin. Extend it (in `crates/roko-core/src/foundation.rs`) to support all 9 layers:

```rust
#[derive(Debug, Clone, Default)]
pub struct PromptSpec {
    pub role: Option<String>,
    pub task: Option<String>,
    pub workdir: Option<PathBuf>,

    // Existing
    pub gate_feedback: Vec<GateFeedback>,        // was Vec<String> — now structured
    pub anti_patterns: Vec<String>,

    // New for full assembly
    pub plan_context: Option<PlanContext>,        // task list, acceptance, file context
    pub prior_task_outputs: Vec<TaskOutput>,
    pub strategy_brief: Option<String>,           // from strategist phase
    pub review_findings: Vec<String>,             // from prior reviewer
    pub attempt: u32,                             // retry count
    pub token_budget: Option<u32>,                // hard cap (defaults to model limit)
    pub tool_allowlist: Option<Vec<String>>,      // override role default
    pub warnings: Vec<String>,                    // layer 8 (replaces pheromones)
}

#[derive(Debug, Clone)]
pub struct GateFeedback {
    pub gate_name: String,
    pub rung: u8,
    pub passed: bool,
    pub errors: Vec<String>,        // classified
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}
```

This is a **breaking change** in `roko-core::foundation`. Bump the trait but keep prior fields (additive). Use `..Default::default()` everywhere.

### Step 2 — Add a richer `PromptDiagnostics` to the trait

```rust
// roko_core::foundation
#[derive(Debug, Clone, Default)]
pub struct PromptDiagnostics {
    pub assembled_id: String,           // hash of layers
    pub included_sections: Vec<SectionMeta>,
    pub dropped_sections: Vec<DroppedSection>,
    pub estimated_tokens: u32,
    pub knowledge_ids: Vec<String>,
    pub playbook_ids: Vec<String>,
    pub strategy: String,               // "DensityGreedy" | "WeightedSum" | ...
    pub effectiveness_baseline: HashMap<String, f64>,
}

#[async_trait]
pub trait PromptAssembler: Send + Sync {
    async fn assemble(&self, spec: PromptSpec) -> Result<AssembledPrompt>;
    // legacy fns kept as defaults for compat
    fn last_prompt_section_ids(&self) -> Vec<String> { Vec::new() }
    fn last_knowledge_ids(&self) -> Vec<String> { Vec::new() }
}

pub struct AssembledPrompt {
    pub system: String,
    pub user: Option<String>,
    pub tool_allowlist: Vec<String>,
    pub diagnostics: PromptDiagnostics,
}
```

Update `roko_compose::PromptAssemblyService` to populate this struct fully. Drop the `last_prompt_section_ids` mutable-state pattern in favor of returning `AssembledPrompt`.

### Step 3 — Resolve the naming collision

Three different `PromptAssembler`s confuse readers and grep results:

- **Keep** `roko_core::foundation::PromptAssembler` (trait, canonical).
- **Rename** `roko_cli::dispatch::prompt_builder::PromptAssembler` → `TaskPromptComposer`. Its job is "compose a per-task prompt for the dispatch path"; the new name reflects that. Once Step 4 lands, this composer mostly disappears.
- **Rename** `roko_compose::templates::assembly::PromptAssembler` → `TemplateAssembler`.

Update all call sites. Add `#[deprecated]` aliases for one release if external crates depend on the names.

### Step 4 — Migrate `roko run` and `roko plan run` task dispatch to use `PromptAssemblyService`

**Files:**

- `crates/roko-cli/src/run.rs` — uses `dispatch_helpers::build_system_prompt_with_context_validated`
- `crates/roko-cli/src/dispatch/mod.rs` + `dispatch/prompt_builder.rs` — uses `TaskPromptComposer` (renamed in Step 3)

Replace the bespoke build with:

```rust
let assembler = state.services.prompt_assembler();
let assembled = assembler
    .assemble(PromptSpec {
        role: Some(role.to_string()),
        task: Some(task.title.clone()),
        workdir: Some(workdir.clone()),
        gate_feedback: prior_failures.clone(),
        plan_context: Some(plan_ctx),
        prior_task_outputs: load_prior_task_outputs(&task_id),
        strategy_brief,
        review_findings,
        attempt,
        token_budget: model_token_limit_for(&task.model_hint),
        tool_allowlist: task.allowed_tools.clone().into(),
        warnings: state.warning_store.snapshot(),
        ..Default::default()
    })
    .await?;

// pass assembled.system + assembled.diagnostics into ModelCallRequest
let req = ModelCallRequest {
    system: Some(assembled.system),
    prompt_section_ids: assembled.diagnostics.included_sections.iter().map(|s| s.id.clone()).collect(),
    knowledge_ids: assembled.diagnostics.knowledge_ids.clone(),
    ..base_req
};
```

After this, `TaskPromptComposer` has nothing left to do. Delete it.

### Step 5 — Migrate ACP runner inline prompts to templates

**File:** `crates/roko-acp/src/runner.rs`

Replace `build_review_prompt(strictness)` (~ line 1512) with:

```rust
let template_name = match config.review_strictness.as_str() {
    "quick" => "quick_reviewer",
    "thorough" => "auditor",
    _ => "reviewer",
};
let assembled = assembler.assemble(PromptSpec {
    role: Some(template_name.to_string()),
    task: Some(self.original_prompt.clone()),
    review_findings: prior_findings,
    plan_context: Some(plan_ctx_with_diff()),
    ..Default::default()
}).await?;
```

The `quick`/`standard`/`thorough` `if`/`else` block is **anti-pattern #5**. Moving it into a role+template lookup is the fix.

### Step 6 — Migrate ACP session inline prompts

**File:** `crates/roko-acp/src/session.rs:~420-469`

`Session::build_system_prompt` constructs `SystemPromptBuilder` directly, choosing role identity inline based on `mode` (`plan` / `research` / default).

Replace with `assembler.assemble(...)` using mode-to-role mapping:

```rust
let role = match self.mode {
    SessionMode::Plan => "strategist",
    SessionMode::Research => "researcher",
    SessionMode::Default => "implementer",
};
let assembled = self.services.prompt_assembler.assemble(PromptSpec {
    role: Some(role.to_string()),
    task: Some(prompt_text),
    workdir: Some(self.workdir.clone()),
    ..Default::default()
}).await?;
```

### Step 7 — Migrate chat to `PromptAssemblyService`

**File:** `crates/roko-cli/src/chat_session.rs:~1411-1441`

`build_chat_system_prompt` constructs a long `role_identity` string and calls `SystemPromptBuilder` directly. Replace with `assembler.assemble(...)`.

The chat use case is special:

- No task description (the entire conversation IS the task)
- Multi-turn — history must not be re-included as a knowledge layer
- User configures allowed tools via `/tools` slash command

Add a chat-specific role: `"interactive_chat"` — define in `core_roles.toml`. Its template emits a minimal layer 1 with conversation guidelines; layer 4 is the user's most recent message; layer 5 is the configured tool list.

The `/tools` override updates `PromptSpec.tool_allowlist` directly — do not parallel-track it inside chat.

### Step 8 — Migrate HTTP `/api/inference/complete` to assemble system prompts

**File:** `crates/roko-serve/src/routes/gateway.rs`

Today the route forwards client `messages` straight to `ModelCallService`. For consistency:

- If the request body includes `role` or `template_name`, run it through `PromptAssemblyService` first.
- Otherwise, behave as today (raw passthrough).

Add a body field `assembly: { role: string, task?: string, plan_id?: string }`. When present, the route assembles before dispatch. This lets HTTP callers benefit from the unified system without breaking back-compat.

### Step 9 — Delete VCG auction code

**Files:**

- `crates/roko-compose/src/auction.rs` — delete (or feature-gate behind `experimental-vcg`)
- `crates/roko-compose/src/strategy.rs` — drop `VcgWelfare` strategy variant; default to `DensityGreedy`
- `crates/roko-compose/src/prompt.rs` — drop `vcg_allocate` calls
- `crates/roko-compose/src/lib.rs` — drop `pub use auction::*`

Reasoning: Per `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md` § "What gets deleted":

> "VCG auction payments — Greedy knapsack is sufficient"

VCG warmup requires 50+ observations per bidder; in practice it's never reached. Diagnostics computed from VCG don't influence selection. Delete or feature-gate.

### Step 10 — Section effectiveness wiring

After Step 4, every `ModelCallRequest` carries `prompt_section_ids` and `knowledge_ids`. `FeedbackService` (plan 03) already updates `SectionEffectivenessRegistry` when those provenance fields are present.

Verify the loop closes: Run a task twice. After the first run, `.roko/learn/section-effects.json` should exist with non-zero entries. On the second run, `PromptAssemblyService` (loaded via `ServiceFactory::with_section_effectiveness`) should adjust section priorities. Add an integration test:

```rust
#[tokio::test]
async fn section_effectiveness_persists_and_influences() {
    let svc = ServiceFactory::for_test()?.assembler();
    let first = svc.assemble(spec()).await?;
    let baseline = first.diagnostics.effectiveness_baseline.clone();

    // record outcome marking section X as failed
    feedback.record(FeedbackEvent::ModelCall {
        prompt_section_ids: vec!["section_X".into()],
        success: false,
        ..base_event
    }).await?;
    feedback.flush().await?;

    let svc2 = ServiceFactory::for_test()?.assembler();   // reload from disk
    let second = svc2.assemble(spec()).await?;
    assert!(second.diagnostics.effectiveness_baseline["section_X"] < baseline["section_X"]);
}
```

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #2 Inline prompts | New caller does `let prompt = format!("You are a ...");` | All role identity strings live in templates under `roko-compose/src/templates/` |
| #4 Wrong layer | Adding "thorough vs quick" branching inside the runner | Templates are looked up by role name; the runner picks the role |
| #5 Hardcoded role behavior | If/elsing review_strictness | Use templates: `quick_reviewer`, `reviewer`, `auditor` |
| #6 Feedback afterthought | New caller skips passing `prompt_section_ids` to ModelCallRequest | The `assembled.diagnostics` field is the only sanctioned way to get the prompt — it returns the section IDs |
| #7 Copy-paste between runtimes | New crate adds its own `build_system_prompt` | All prompt building goes through `roko_core::PromptAssembler` |
| #10 God file | Putting the new `interactive_chat` role logic into chat_session.rs | The role lives in `core_roles.toml` + a template file |

---

## Things NOT To Do

1. **Don't make `PromptAssembler` synchronous.** Knowledge store + playbook lookups require `.await`. The trait stays `async`.
2. **Don't expose mutable last_*_ids state.** Returning `AssembledPrompt` instead is cleaner; the legacy methods stay only for one transition release.
3. **Don't pre-render templates at startup.** `SystemPromptBuilder` is fast enough to run per call. Caching here is premature optimization.
4. **Don't reorder layers.** The 9-layer order is cache-aligned for prompt caching benefits (Anthropic API). Keep stable layers (role, conventions, tools) first.
5. **Don't move templates outside `roko-compose`.** Spreading them across crates kills the "one place to look" property.
6. **Don't pass arbitrary `role: String`.** Validate against `RoleRegistry` (see `roko_core::agent::AgentRole`). Unknown roles should hard-fail (or fall back to a documented default with a warning).
7. **Don't migrate VCG to a "simpler" auction.** Just delete it. Density-greedy is empirically sufficient.
8. **Don't add a 10th layer "for affect".** Per plan 15, daimon is being deleted.
9. **Don't keep `dispatch_helpers::build_system_prompt_with_context_validated` "for safety".** Delete after Step 4 lands.

---

## Tests / Proof Criteria

```bash
# 1. No inline role identity strings outside templates
rg 'You are (the|a|an) \*\*' crates/ --type rust | grep -v 'roko-compose/src/templates' | grep -v test
# expected: 0 results (some may remain in commented code; verify case-by-case)

# 2. No bare format! prompt construction in CLI/ACP
rg 'let .*prompt.* = format!\(' crates/roko-acp/src/ crates/roko-cli/src/dispatch/ --type rust
# expected: only inside test or templated wrappers

# 3. VCG references removed
rg 'vcg_allocate|VcgWelfare|VcgAuction' crates/ --type rust | grep -v test | grep -v '#\[cfg('
# expected: 0 results

# 4. PromptAssembler trait collision resolved
rg 'pub (struct|trait) PromptAssembler' crates/ --type rust
# expected: exactly 1 result (the trait in roko-core/foundation.rs)
```

Functional proofs:

- [ ] `roko run "fix the typo"` records `prompt_section_ids` in the episode
- [ ] ACP `session/prompt` → workflow=standard records reviewer prompt provenance via the `reviewer` template (not inline string)
- [ ] Chat session uses `interactive_chat` role; switching tools via `/tools` updates layer 5 only (verify via diagnostic snapshot)
- [ ] Section effectiveness test from Step 10 passes
- [ ] Snapshot tests for implementer / reviewer / strategist / retry-with-gate-feedback prompts (4 snapshot tests in `crates/roko-compose/tests/snapshots/`)
- [ ] Token budget enforcement: prompts > 100K tokens drop low-priority sections (warning logged, sections listed in `diagnostics.dropped_sections`)

---

## Dependencies

- **Plan 01 (ModelCallService)** — `prompt_section_ids` + `knowledge_ids` plumbing into `ModelCallRequest` is already there from Phase 0.1.
- **Plan 03 (FeedbackService)** — section effectiveness writes happen in the FeedbackService; this plan provides the inputs.

Can start in parallel with Plan 01 once `PromptSpec` extension is agreed.

---

## Estimated Effort

**L.** ~1-2 weeks.

- Step 1+2 (extend types) — S (1 day)
- Step 3 (rename collision) — S (1 day, mostly mechanical)
- Step 4 (run + plan run dispatch) — M (2-3 days)
- Step 5+6 (ACP runner + session) — M (2 days)
- Step 7 (chat) — M (2 days; chat path is tested visually so requires care)
- Step 8 (HTTP) — S (1 day)
- Step 9 (delete VCG) — S (1 day; mostly tests to update)
- Step 10 (section effectiveness proof) — S (1 day)
