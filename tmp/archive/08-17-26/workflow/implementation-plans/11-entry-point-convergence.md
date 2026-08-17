# 11 — Entry Point Convergence

> Phase 5 of `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md`. Cross-references audits 09, 10, 15.

---

## Status (2026-05-01)

**PARTIAL.** `roko run` (v2) and ACP go through `WorkflowEngine`. `roko plan run` still uses `runner/event_loop.rs`. `agent_exec` uses bespoke spawn. HTTP `/api/inference/complete` doesn't assemble layered prompts.

**Coverage matrix:**

| Entry Point | WorkflowEngine? | PromptAssembly? | FeedbackService? | Notes |
|---|---|---|---|---|
| `roko` (interactive chat) | No (uses `ChatAgentSession` + `ModelCallService`) | Direct `SystemPromptBuilder` (chat builder) | NO sink attached | Highest-volume path |
| `roko "prompt"` (one-shot) | No (uses `ChatAgentSession`) | Direct `SystemPromptBuilder` | NO | |
| `roko chat` (REPL) | No (`chat.rs` BufRead loop) | None (raw println) | None | Parallel to `roko` |
| `roko run "<prompt>"` | YES (v2) | Yes (via ServiceFactory) | Yes | TODO(gateway) comment in `run.rs:~1826` indicates partial migration |
| `roko plan run plans/` | No (`runner/event_loop.rs`) | Partial (`TaskPromptComposer`) | Partial (FeedbackFacade) | Default path |
| `roko prd refine`, `roko plan generate`, research | No (`agent_exec.rs::spawn_agent_scoped`) | No | Partial | |
| `roko acp` (workflow mode) | YES | Partial (inline review prompts) | Yes | |
| `roko acp` (default) | No (`bridge_events.rs::run_*_cognitive_task`) | Inline format strings | Partial | |
| HTTP `POST /api/inference/complete` | No (passthrough) | Skipped | Yes (via state ModelCallService) | |
| HTTP `POST /api/plans/{id}/run` | Through CliRuntime → orchestrator | Same as plan run | Same | |
| HTTP `POST /api/run` | Through CliRuntime → run | Same as roko run | Same | |

---

## Goal

Every entry point converges on `WorkflowEngine` for any operation that involves more than a single bare model call. Single bare calls go through `ModelCallService` directly (chat is the prototype). After this plan:

- `roko plan run` → `WorkflowEngine::run` with `WorkflowTemplate::PlanExecution`
- `roko run "<prompt>"` → already there; verify no regressions
- ACP → fully on `WorkflowEngine`; default mode also goes through it (with `WorkflowTemplate::Express`)
- `agent_exec.rs` callers → `WorkflowEngine::run` with `WorkflowTemplate::Express` for short-lived flows; fold helpers into one
- HTTP `/api/inference/complete` → optional `assembly` body field that triggers `PromptAssemblyService`
- `roko chat` (REPL) → either delete or thin-wrap over inline chat path

---

## Why This Exists (Anti-Patterns Eliminated)

- **#3 Build Another Runtime** — main symptom; this plan kills the parallels
- **#7 Copy-Paste** — every entry point implementing its own dispatch
- **#10 God file** — `runner/event_loop.rs` and `chat_inline.rs` shrink dramatically

---

## Existing Code — Read These First

- `crates/roko-cli/src/main.rs` — top-level command dispatch
- `crates/roko-cli/src/run.rs:~1500-1830` — `run_with_workflow_engine` (the model)
- `crates/roko-cli/src/runner/event_loop.rs` — the legacy plan runner
- `crates/roko-cli/src/commands/plan.rs` — `roko plan run` entry
- `crates/roko-cli/src/agent_exec.rs` — bespoke helpers
- `crates/roko-acp/src/bridge_events.rs` — ACP "default" cognitive dispatch
- `crates/roko-acp/src/runner.rs` — ACP "workflow" path (already on `WorkflowEngine`)
- `crates/roko-serve/src/routes/run.rs`, `plans.rs`, `gateway.rs`

---

## Implementation Steps

### Step 1 — Migrate `roko plan run`

**File:** `crates/roko-cli/src/commands/plan.rs`

This is the **biggest** migration. Today `plan run` calls `runner::event_loop::run`. Plan 06 § Step 5 sketches the migration. Recap:

```rust
pub async fn run_plan(opts: PlanRunOpts) -> Result<()> {
    if opts.use_event_loop {
        return legacy_plan_run(opts).await;       // for one transition release
    }

    let services = ServiceFactory::for_plan(&opts.workdir, &opts.config).await?;
    let engine = WorkflowEngine::new(services);
    let cfg = WorkflowRunConfig {
        prompt: format!("plan_run::{}", opts.plans_dir.display()),
        workdir: opts.workdir,
        workflow: WorkflowConfig {
            template: WorkflowTemplate::PlanExecution(plan_exec_config(&opts)),
            ..Default::default()
        },
        plan_tasks: Some(load_all_tasks(&opts.plans_dir)?),
        ..base_config_from_opts(&opts)
    };
    let report = engine.run(cfg).await?;
    print_run_report(&report);
    Ok(())
}
```

Behind `--use-event-loop` keep the old path callable. Default flips to `WorkflowEngine` after one release of soak. Then `--use-event-loop` becomes a hard error (Step 5 of plan 12 deletes the old code).

### Step 2 — Migrate `agent_exec.rs` callers

**File:** `crates/roko-cli/src/agent_exec.rs`

Today functions: `run_agent`, `run_agent_capture`, `run_agent_logged`, `persist_capture_episode`.

Goal: collapse them into one helper that wraps `WorkflowEngine`:

```rust
// crates/roko-cli/src/agent_exec.rs (after migration)
pub struct AgentExecRequest {
    pub prompt: String,
    pub workdir: PathBuf,
    pub role: String,
    pub model: Option<String>,
    pub gates: Vec<String>,
    pub commit: bool,                   // false for research / PRD
    pub capture_to: Option<PathBuf>,
}

pub async fn run_agent(req: AgentExecRequest) -> Result<RunReport> {
    let services = ServiceFactory::for_workdir(&req.workdir).await?;
    let engine = WorkflowEngine::new(services);
    let template = if req.commit && !req.gates.is_empty() {
        WorkflowConfig::express()
    } else {
        WorkflowConfig {
            has_strategy: false,
            has_review: false,
            max_iterations: 1,
            max_autofix_attempts: 0,
        }
    };
    let cfg = WorkflowRunConfig {
        prompt: req.prompt,
        workdir: req.workdir,
        workflow: template,
        enabled_gates: req.gates,
        commit_prefix: if req.commit { Some("prd".into()) } else { None },
        ..Default::default()
    };
    let report = engine.run(cfg).await?;
    if let Some(path) = req.capture_to {
        std::fs::write(path, render_capture_report(&report))?;
    }
    Ok(report)
}
```

Update callers in:

- `crates/roko-cli/src/prd.rs`
- `crates/roko-cli/src/commands/plan.rs::generate_subcommand`
- `crates/roko-cli/src/commands/research*.rs`

### Step 3 — Migrate ACP default mode

**File:** `crates/roko-acp/src/bridge_events.rs:~1429-1724`

Today `run_anthropic_cognitive_task` and `run_openai_compat_cognitive_task` are bespoke loops over `ModelCallService::stream`. They handle ACP `session/update` events inline.

Refactor to:

```rust
async fn run_default_acp_prompt(
    session: &Session,
    prompt: String,
) -> Result<()> {
    let services = session.services.clone();           // ServiceFactory built earlier
    let mut engine = WorkflowEngine::new(services);
    engine.add_consumer(Arc::new(AcpEventBridge::new(session.update_sender.clone())));

    let cfg = WorkflowRunConfig {
        prompt,
        workdir: session.workdir.clone(),
        workflow: WorkflowConfig::express(),
        enabled_gates: vec![],            // default ACP doesn't run gates
        ..Default::default()
    };
    engine.run(cfg).await?;
    Ok(())
}
```

The `AcpEventBridge` is an `EventConsumer` that translates `RuntimeEvent` → ACP `session/update` messages. This already exists in spirit inside `bridge_events`; extract it as a clean bridge module under `crates/roko-acp/src/bridges.rs`.

### Step 4 — Decide: keep `roko chat` (REPL) or delete

`roko chat` is the BufRead REPL loop in `chat.rs` (~659 LOC). It is **parallel** to `roko` (the unified inline chat). Per audit doc 10 § 9B:

- `roko chat` lacks markdown, tool output, cost display, session, completions, history
- Two parallel implementations means future fixes apply to one

**Recommendation:** delete `roko chat` entirely. Users who want REPL semantics should use `roko --no-tui` (add this flag to `roko` if needed).

If keeping for back-compat: rewrite `chat.rs::run_chat_repl` to call `chat_inline::run_unified_inline` with a `RenderMode::Plaintext` flag. Then `chat.rs` is a 30-LOC wrapper, not 659 LOC.

### Step 5 — HTTP `/api/inference/complete` optional assembly

**File:** `crates/roko-serve/src/routes/gateway.rs`

Add optional `assembly` body field:

```rust
#[derive(Deserialize)]
pub struct CompletionRequest {
    pub model: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub assembly: Option<AssemblyHint>,         // NEW
}

#[derive(Deserialize)]
pub struct AssemblyHint {
    pub role: String,
    pub task: Option<String>,
    pub plan_id: Option<String>,
    pub gate_feedback: Option<Vec<GateFeedback>>,
}

async fn complete(State(state): State<AppState>, Json(req): Json<CompletionRequest>) -> impl IntoResponse {
    let model_req = if let Some(hint) = req.assembly {
        let assembled = state.services.prompt_assembler.assemble(PromptSpec {
            role: Some(hint.role),
            task: hint.task,
            gate_feedback: hint.gate_feedback.unwrap_or_default(),
            ..Default::default()
        }).await?;
        ModelCallRequest {
            model: req.model.unwrap_or(state.default_model().clone()),
            system: Some(assembled.system),
            messages: req.messages,
            prompt_section_ids: assembled.diagnostics.included_sections.iter().map(|s| s.id.clone()).collect(),
            knowledge_ids: assembled.diagnostics.knowledge_ids.clone(),
            caller: Some("serve".into()),
            ..Default::default()
        }
    } else {
        ModelCallRequest {
            model: req.model.unwrap_or(state.default_model().clone()),
            messages: req.messages,
            caller: Some("serve".into()),
            ..Default::default()
        }
    };
    let response = state.services.model_caller.call(model_req).await?;
    Json(response_to_completion_payload(response))
}
```

Backward compatible — existing clients without `assembly` keep working.

### Step 6 — `POST /api/plans/{id}/run` and `POST /api/run` go through engine

Today `roko-serve` delegates to `state.runtime.run_once(workdir, prompt)` via `CliRuntime`. After Step 1+2 those CLI entries already use `WorkflowEngine`, so HTTP gets it for free.

Verify:

- `POST /api/plans/{id}/run` triggers `WorkflowTemplate::PlanExecution`
- `POST /api/run` triggers `WorkflowTemplate::Express` or auto-selected
- `POST /api/runs/{id}/cancel` flips a `CancelToken` shared with the engine (verify via plan 07 cancellation work)

Add explicit tests for HTTP triggering each template.

### Step 7 — Add `WorkflowTemplate::auto_select` for ACP

The ACP `session/prompt` body has a `workflow` config; today auto-select is in `crates/roko-acp/src/workflow.rs::WorkflowTemplate::auto_select(prompt)`.

Move `auto_select` into `roko-runtime::pipeline_state::WorkflowConfig::auto_select`:

```rust
impl WorkflowConfig {
    pub fn auto_select(prompt: &str) -> WorkflowConfig {
        let words = prompt.split_whitespace().count();
        let lower = prompt.to_lowercase();
        let express_kw = ["fix typo", "rename", "update", "add comment"];
        let full_kw = ["refactor", "redesign", "architecture", "rewrite"];

        if words < 15 && express_kw.iter().any(|k| lower.contains(k)) {
            WorkflowConfig::express()
        } else if words > 50 || full_kw.iter().any(|k| lower.contains(k)) {
            WorkflowConfig::full()
        } else {
            WorkflowConfig::standard()
        }
    }
}
```

ACP, HTTP, and CLI all use the same heuristic.

### Step 8 — End-to-end smoke tests for every entry point

```rust
// crates/roko-cli/tests/entry_points.rs
#[tokio::test]
async fn roko_run_uses_workflow_engine() {
    let temp = test_workdir();
    let output = Command::new(roko_bin()).arg("run").arg("add a comment").current_dir(&temp).output()?;
    assert!(output.status.success());
    let episode = read_latest_episode(&temp)?;
    assert_eq!(episode.caller, "cli");
    assert_eq!(episode.workflow_template, "express");
}

#[tokio::test]
async fn roko_plan_run_uses_workflow_engine() {
    let temp = test_workdir_with_plan("examples/diamond-plan");
    let output = Command::new(roko_bin()).arg("plan").arg("run").arg("plans/").current_dir(&temp).output()?;
    assert!(output.status.success());
    let runs = read_all_runs(&temp)?;
    assert!(runs.iter().all(|r| r.engine == "workflow_v2"));
}

#[tokio::test]
async fn http_inference_complete_with_assembly() {
    let server = test_server().await?;
    let resp = Client::new().post(format!("{}/api/inference/complete", server.url()))
        .json(&json!({
            "messages": [{"role": "user", "content": "implement a function"}],
            "assembly": {"role": "implementer", "task": "implement a function"}
        }))
        .send().await?;
    let body: Value = resp.json().await?;
    assert!(body["episode"]["prompt_section_ids"].as_array().unwrap().len() > 0);
}
```

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #3 Build another runtime | Adding new commands that bypass WorkflowEngine | Every command goes through one of: `WorkflowEngine`, `ModelCallService` (single bare call), or a tool dispatch |
| #7 Copy-paste | Different commands re-implementing flag parsing → `WorkflowRunConfig` | One `args_to_workflow_run_config(args)` helper |
| #10 God file | `chat.rs` and `chat_inline.rs` keep growing | Plan 16 trims them |

---

## Things NOT To Do

1. **Don't migrate the chat path to `WorkflowEngine`.** Chat is a single-call pattern; introducing the engine for each turn would add latency and break streaming. Chat keeps `ModelCallService` direct.
2. **Don't change CLI flags' user-visible behavior** during migration. `roko plan run`'s flags must work identically — flag-equivalent migrations only.
3. **Don't gate the engine path behind feature flags after one release.** Either it works or it doesn't. Long-lived flags are rot.
4. **Don't keep `agent_exec.rs` helpers as a thin wrapper.** Either fully inline `WorkflowEngine::run(...)` at each caller or delete the helpers.
5. **Don't make `auto_select` smart.** Keyword matching is fine; doing semantic prediction is overengineering.
6. **Don't break the share-page URL contract.** `/runs/{token}` URLs are bookmarkable. Maintain backward compat even after route consolidation (plan 10 § 4).
7. **Don't add per-entry-point telemetry.** Every event already carries `caller`; HTTP routes pass `caller: "serve"`, CLI `caller: "cli"`. Don't add ad-hoc fields.
8. **Don't merge `roko run` and `roko plan run` into one command.** Different mental models, different defaults. Keep separate.

---

## Tests / Proof Criteria

```bash
# 1. event_loop is no longer the default plan runner
rg 'runner::event_loop::run' crates/roko-cli/src/commands/plan.rs
# expected: gated behind --use-event-loop only

# 2. agent_exec uses WorkflowEngine
rg 'WorkflowEngine|spawn_agent_scoped' crates/roko-cli/src/agent_exec.rs
# expected: WorkflowEngine usage; no spawn_agent_scoped

# 3. ACP default mode uses WorkflowEngine
rg 'run_anthropic_cognitive_task|run_openai_compat_cognitive_task' crates/roko-acp/ --type rust
# expected: 0 callers (deleted) or only as test fixtures

# 4. roko chat is gone OR a thin wrapper
wc -l crates/roko-cli/src/chat.rs
# expected: < 50 lines (after migration to inline)
```

Functional proofs:

- [ ] All 3 unit tests above pass
- [ ] Two-week soak: every CLI command runs through `WorkflowEngine` (where applicable); inspect `.roko/episodes.jsonl` to verify `engine` field
- [ ] HTTP smoke: every documented `POST /api/...` endpoint returns 200 with the expected episode written
- [ ] `roko acp` from Zed editor produces same `RuntimeEvent`s as `roko run` for equivalent prompts (modulo session-bridge events)
- [ ] `roko prd refine prds/sample.md` produces an episode tagged `caller: "research"` with template `Express`
- [ ] Performance: `roko plan run plans/sample` total wall-clock within 10% of legacy `event_loop` baseline

---

## Dependencies

This plan **requires** plans 01-09 to be complete (or close to). Specifically:

- 01 (ModelCallService) — for ACP cognitive dispatch migration
- 02 (PromptAssembly) — for HTTP assembly hint
- 03 (FeedbackService) — for chat path
- 05+06+07 (Pipeline+Scheduler+EffectDriver) — for plan run migration
- 09 (Safety) — to ensure migrated paths inherit safety

This plan **blocks** plan 12 (Retirement).

---

## Estimated Effort

**L–XL.** ~2 weeks. Mostly migration work — moderate per-file but lots of files.

- Step 1 (plan run migration) — L (4-5 days; biggest)
- Step 2 (agent_exec) — M (2-3 days)
- Step 3 (ACP default) — M (2-3 days)
- Step 4 (chat REPL decision + cleanup) — S (1 day)
- Step 5 (HTTP assembly) — S (1 day)
- Step 6 (HTTP plan + run wiring) — S (1 day; mostly delegated)
- Step 7 (auto_select consolidation) — S (1 day)
- Step 8 (smoke tests) — M (2 days)
