# AUDIT: Batch R2_G01 — Wire pipeline template from roko.toml config

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R2_G01`.
Your job is to verify correctness and fix any issues — do NOT rewrite from scratch.

## Audit Checklist

1. **Compiles:** `cargo check -p <crate>` for each crate touched by this batch
2. **Clippy clean:** `cargo clippy -p <crate> --no-deps -- -D warnings`
3. **Prompt compliance:** Compare the implementation against the original prompt below
4. **No regressions:** Changed files don't break existing functionality
5. **Anti-patterns:** No stubs that silently pass, no inline prompts, no raw CLI spawns
6. **Correct types:** Field names, method signatures, and imports match the actual codebase
7. **Tests pass:** If the prompt required tests, verify they pass

## If You Find Issues

Fix them directly in the files. Then run the verification commands from the prompt.
If you cannot fix an issue, leave a comment in the file explaining why.

## Scope

Only touch files in the batch's write scope. Do NOT refactor unrelated code.

---

## Original Implementation Prompt

## Task

Wire pipeline template from roko.toml config

## Runner Context

You are working in runner `mega-parity`, batch R2_G01.
This batch is part of the performance optimization group: Pipeline Config — make the V2 workflow engine respect roko.toml `[pipeline.*]` configuration instead of hardcoding `"standard"`.

## Problem

`run_with_workflow_engine_with_hub` at
`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs:631`
passes `workflow_template` through `workflow_config_for_template` which always returns
`WorkflowConfig::standard()` for the default case (lines 556–562):

```rust
fn workflow_config_for_template(workflow_template: &str) -> WorkflowConfig {
    match workflow_template {
        "express" => WorkflowConfig::express(),
        "full" => WorkflowConfig::full(),
        _ => WorkflowConfig::standard(),
    }
}
```

And at line 631 it is called as:
```rust
    let config = WorkflowRunConfig {
        prompt: prompt.to_string(),
        workdir: workdir.to_path_buf(),
        workflow: workflow_config_for_template(workflow_template),
        enabled_gates,
        commit_prefix: Some("feat".to_string()),
    };
```

The roko.toml `[pipeline.*]` sections are parsed into `PipelineConfig` /
`PipelineBandConfig` in:
- `crates/roko-core/src/config/gates.rs:75` — `PipelineBandConfig` struct
- `crates/roko-core/src/config/gates.rs:228` — `PipelineConfig` struct
- `crates/roko-core/src/config/schema.rs:68` — `RokoConfig.pipeline: PipelineConfig`

`PipelineConfig` is already loaded at `crates/roko-cli/src/run.rs:497` via
`let mut model_config = roko_core::config::load_config(workdir).unwrap_or_default();`
and `model_config` is of type `RokoConfig`, so `model_config.pipeline` is available
immediately. It is NOT being passed to `WorkflowRunConfig`.

## Architecture Contract

- One config source for pipeline behavior: `PipelineConfig` from roko.toml
- CLI `--workflow` flag (if present) overrides config
- Default for `roko run` (no explicit template) uses `mechanical` band (express), not `standard`
- `roko plan run` continues using per-task complexity or default config

## `PipelineBandConfig` fields (exact, from gates.rs:75)

```rust
pub struct PipelineBandConfig {
    pub strategist: bool,       // maps to WorkflowConfig.has_strategy
    pub reviewers: bool,        // maps to WorkflowConfig.has_review
    pub reviewer_mode: PipelineReviewerMode,  // (ignored by WorkflowConfig)
    pub max_iterations: u32,    // maps to WorkflowConfig.max_iterations
}
```

## `WorkflowConfig` fields (exact, from pipeline_state.rs:37)

```rust
pub struct WorkflowConfig {
    pub has_strategy: bool,
    pub has_review: bool,
    pub max_iterations: u32,
    pub max_autofix_attempts: u32,
}
```

## Band defaults (from gates.rs:94)

| Band | strategist | reviewers | max_iterations |
|---|---|---|---|
| `mechanical` | `false` | `false` | `1` |
| `focused` | `false` | `false` | `2` |
| `integrative` | `true` | `true` | `2` |
| `architectural` | `true` | `true` | `3` |

## Changes Required

**File to modify**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`

### Step 1: Add `workflow_config_from_band` conversion function

Add this function immediately AFTER the existing `workflow_config_for_template`
function (after line 562):

```rust
/// Convert a `PipelineBandConfig` from roko.toml into a `WorkflowConfig` for the V2 engine.
fn workflow_config_from_band(band: &roko_core::config::PipelineBandConfig) -> WorkflowConfig {
    WorkflowConfig {
        has_strategy: band.strategist,
        has_review: band.reviewers,
        max_iterations: band.max_iterations,
        // When reviewers are disabled, one autofix attempt is enough.
        // When reviewers are enabled, allow two rounds.
        max_autofix_attempts: if band.reviewers { 2 } else { 1 },
    }
}
```

`roko_core::config::PipelineBandConfig` is already re-exported from `roko_core::config`
(see `crates/roko-core/src/config/mod.rs:41`). The `WorkflowConfig` type is already
imported at line 48: `use roko_runtime::pipeline_state::WorkflowConfig;`.

### Step 2: Update `run_with_workflow_engine_with_hub` to use config-driven bands

**Current** `WorkflowRunConfig` construction in `run_with_workflow_engine_with_hub`
(lines 628–634):

```rust
    let config = WorkflowRunConfig {
        prompt: prompt.to_string(),
        workdir: workdir.to_path_buf(),
        workflow: workflow_config_for_template(workflow_template),
        enabled_gates,
        commit_prefix: Some("feat".to_string()),
    };
```

**After** — replace the `workflow` field value. The `model_config` variable (type
`RokoConfig`, loaded at line 497) is NOT in scope inside `run_with_workflow_engine_with_hub`
because it lives inside `build_workflow_effect_services`. You need to load the
pipeline config separately inside `run_with_workflow_engine_with_hub`.

Insert this block BEFORE the `WorkflowRunConfig { ... }` construction:

```rust
    // Load the pipeline config from roko.toml (or defaults if no config file).
    let pipeline_config = {
        let roko_cfg = roko_core::config::load_config(workdir).unwrap_or_default();
        roko_cfg.pipeline
    };

    // Map the workflow_template string to the corresponding band config.
    // When no explicit template was passed (the caller passes "standard" as default),
    // use the mechanical band (fast, no reviewer) as the default for `roko run`.
    let workflow = match workflow_template {
        "express" | "mechanical" => workflow_config_from_band(&pipeline_config.mechanical),
        "focused" => workflow_config_from_band(&pipeline_config.focused),
        "integrative" => workflow_config_from_band(&pipeline_config.integrative),
        "full" | "architectural" => workflow_config_from_band(&pipeline_config.architectural),
        "standard" => {
            // "standard" is the old default; treat it as mechanical unless the user
            // explicitly passed it via --workflow.
            workflow_config_from_band(&pipeline_config.mechanical)
        }
        _ => workflow_config_for_template(workflow_template), // unknown template: old fallback
    };
```

Then update the `WorkflowRunConfig` to use the new `workflow` variable:

```rust
    let config = WorkflowRunConfig {
        prompt: prompt.to_string(),
        workdir: workdir.to_path_buf(),
        workflow,  // <- was: workflow_config_for_template(workflow_template)
        enabled_gates,
        commit_prefix: Some("feat".to_string()),
    };
```

### Complete Before→After diff

**Before** (lines 628–634 in `run_with_workflow_engine_with_hub`):
```rust
    let config = WorkflowRunConfig {
        prompt: prompt.to_string(),
        workdir: workdir.to_path_buf(),
        workflow: workflow_config_for_template(workflow_template),
        enabled_gates,
        commit_prefix: Some("feat".to_string()),
    };
```

**After**:
```rust
    // Load the pipeline config from roko.toml (or defaults if no config file).
    let pipeline_config = {
        let roko_cfg = roko_core::config::load_config(workdir).unwrap_or_default();
        roko_cfg.pipeline
    };

    // Map the workflow_template string to the corresponding band config.
    let workflow = match workflow_template {
        "express" | "mechanical" => workflow_config_from_band(&pipeline_config.mechanical),
        "focused" => workflow_config_from_band(&pipeline_config.focused),
        "integrative" => workflow_config_from_band(&pipeline_config.integrative),
        "full" | "architectural" => workflow_config_from_band(&pipeline_config.architectural),
        "standard" => workflow_config_from_band(&pipeline_config.mechanical),
        _ => workflow_config_for_template(workflow_template),
    };

    let config = WorkflowRunConfig {
        prompt: prompt.to_string(),
        workdir: workdir.to_path_buf(),
        workflow,
        enabled_gates,
        commit_prefix: Some("feat".to_string()),
    };
```

## Write Scope (files you may modify)

- `crates/roko-cli/src/run.rs`
- `crates/roko-runtime/src/pipeline_state.rs` (no changes needed — `WorkflowConfig` already has all the right fields)

## Read-Only Context (do not modify these)

- `crates/roko-core/src/config/gates.rs` — `PipelineConfig`, `PipelineBandConfig` definitions
- `crates/roko-cli/src/config.rs` — CLI config loading (note: CLI `Config` struct does NOT have a `pipeline` field; use `roko_core::config::load_config` which returns `RokoConfig`)
- `crates/roko-cli/src/commands/util.rs` — utility functions

## Acceptance Criteria

- [ ] `roko run` with default config (no `--workflow`) uses `mechanical` band (no reviewer spawned)
- [ ] `roko run --workflow full` uses `architectural` band settings from roko.toml
- [ ] `[pipeline.mechanical]` config changes in roko.toml take effect without code changes
- [ ] `PipelineBandConfig.strategist = false` means `WorkflowConfig.has_strategy = false`
- [ ] `PipelineBandConfig.reviewers = false` means `WorkflowConfig.has_review = false`
- [ ] Existing `workflow_config_for_template()` still works as fallback
- [ ] `cargo check -p roko-cli` passes

## Verification

```bash
cargo check -p roko-cli -p roko-runtime
```

## Do NOT

- Remove the existing `workflow_config_for_template()` function (keep as fallback)
- Change `WorkflowConfig` struct fields
- Modify `PipelineConfig` or `PipelineBandConfig` in roko-core
- Add new CLI flags
- Change the plan runner code path (only affects `roko run`)

## Evidence

- `crates/roko-cli/src/run.rs:556` — `workflow_config_for_template` hardcodes express/standard/full
- `crates/roko-cli/src/run.rs:631` — `workflow_config_for_template(workflow_template)` always called
- `crates/roko-core/src/config/gates.rs:228` — `PipelineConfig` with mechanical/focused/integrative/architectural bands
- `crates/roko-core/src/config/gates.rs:75` — `PipelineBandConfig { strategist, reviewers, reviewer_mode, max_iterations }`
- `crates/roko-runtime/src/pipeline_state.rs:37` — `WorkflowConfig { has_strategy, has_review, max_iterations, max_autofix_attempts }`

---

## Current Implementation (as written by implementation agent)

### `crates/roko-cli/src/run.rs` (3553 lines — truncated)

```rust
//! The universal loop: prompt → compose → agent → gate → persist → policy.
//!
//! This is the body of `roko run <prompt>`. It reads [`Config`], opens a
//! [`FileSubstrate`] under `.roko/`, seeds prompt sections, composes them
//! into a single Prompt signal, invokes the configured agent backend, runs
//! each configured gate on the working directory, and emits an Episode.

use crate::agent_config::{
    synthesize_claude_cli_config, synthesize_known_protocol_config, synthesize_subprocess_config,
};
use crate::agent_spawn::{SpawnAgentSpec, spawn_agent_scoped};
use crate::clean;
use crate::config::{Config, GateConfig, PromptFile};
use crate::knowledge_helpers::{build_strategy_fragment_context, query_anti_knowledge_patterns};
use crate::learning_helpers::{
    load_or_create_playbook_store, load_or_create_skill_library, playbook_query_context,
    render_prior_experience,
};
use crate::episode::EpisodePolicy;
use crate::model_selection::{EffectiveModelSelection, resolve_effective_model};
use crate::output_format;
#[cfg(feature = "legacy-orchestrate")]
use crate::prompting::{PromptBuildOptions, build_role_system_prompt_validated};
use crate::task_helpers::extract_task_symbols;
use crate::state_hub::{StateHub, StateHubSender};
use anyhow::{Context as _, Result, anyhow};
use chrono::Utc;
use roko_agent::provider::is_known_protocol_command;
use roko_agent::translate::{ClaudeTranslator, OllamaTranslator, RenderedTools, Translator};
use roko_agent::{AgentResult, OllamaLlmBackend};
#[cfg(feature = "legacy-orchestrate")]
use roko_compose::{Placement, PromptComposer, PromptSection, SectionPriority, TaskContext};
use roko_core::agent::resolve_model;
use roko_core::config::schema::RokoConfig;
use roko_core::dashboard_snapshot::DashboardEvent;
use roko_core::foundation::{
    EventConsumer as WorkflowEventConsumer, ShellGateCommand as CoreShellGateCommand,
};
use roko_core::metric::{ConfigHash, TaskMetric};
use roko_core::tool::ExternalAction;
use roko_core::tool::ToolRegistry;
use roko_core::{
    AgentRole, Body, Budget, Compose, Context, Engram, Kind, Provenance, Store, TaskCategory,
    Verdict, Verify,
};
use roko_fs::FileSubstrate;
use roko_gate::{BuildSystem, ClippyGate, CompileGate, GatePayload, ShellGate, TestGate};
use roko_learn::episode_logger::{Episode, EpisodeLogger, GateVerdict, Usage as EpisodeUsage};
use roko_learn::playbook::Playbook;
use roko_learn::runtime_feedback::{CompletedRunInput, LearningRuntime};
use roko_learn::skill_library::{Skill, SkillQuery};
use roko_orchestrator::{ServiceConfig, ServiceFactory};
use roko_runtime::effect_driver::EffectServices;
use roko_runtime::pipeline_state::WorkflowConfig;
use roko_runtime::workflow_engine::{WorkflowEngine, WorkflowRunConfig, WorkflowRunReport};
use roko_serve::bench::BenchStrategy;
use roko_std::NoOpScorer;
use roko_std::StaticToolRegistry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Summary of a single `run` invocation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RunReport {
    /// Content hash of the episode signal emitted at the end.
    pub episode_id: String,
    /// Content hash of the assembled prompt signal.
    pub prompt_id: String,
    /// Content hash of the agent's output signal.
    pub agent_output_id: String,
    /// Whether the agent invocation succeeded (exit code 0, no timeout).
    pub agent_success: bool,
    /// Per-gate verdicts in declaration order: (gate name, passed).
    pub gate_verdicts: Vec<(String, bool)>,
    /// How many signals are now in the substrate.
    pub total_signals: usize,
    /// Final agent output text, if it was a text payload.
    pub output_text: Option<String>,
    /// Token usage reported by the agent dispatch, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<RunUsage>,
}

/// Token usage captured from a single run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunUsage {
    /// Input (prompt) tokens consumed.
    pub input_tokens: u64,
    /// Output (completion) tokens produced.
    pub output_tokens: u64,
}

impl RunReport {
    /// True if the agent succeeded and every configured gate passed.
    #[must_use]
    pub fn overall_success(&self) -> bool {
        self.agent_success && self.gate_verdicts.iter().all(|(_, ok)| *ok)
    }

    /// Return the first gate that failed, if any.
    #[must_use]
    pub(crate) fn first_failed_gate(&self) -> Option<&str> {
        self.gate_verdicts
            .iter()
            .find_map(|(gate, passed)| (!*passed).then_some(gate.as_str()))
    }
}

struct StrategyPromptAugmentation {
    system_prompt: String,
    injected_playbook_ids: Vec<String>,
}

struct ContextEnrichmentOverlay {
    text: String,
    injected_playbook_ids: Vec<String>,
}

struct PlaybookSection {
    text: String,
    injected_playbook_ids: Vec<String>,
}

struct DispatchOutcome {
    agent_result: AgentResult,
    external_actions: Vec<ExternalAction>,
    injected_playbook_ids: Vec<String>,
}

/// Write a RunReport to `.roko/shared/{token}.json` and return the token.
#[cfg(feature = "legacy-orchestrate")]
pub fn write_shared_run(workdir: &std::path::Path, report: &RunReport) -> anyhow::Result<String> {
    let token = roko_core::generate_share_token();
    let transcript = roko_serve::routes::shared_runs::RunTranscript {
        id: token.clone(),
        agent: "unknown".to_string(),
        role: "unknown".to_string(),
        prompt: report.prompt_id.clone(),
        success: report.overall_success(),
        gates: report.gate_verdicts.clone(),
        output: report.output_text.clone(),
        cost_usd: None,
        input_tokens: report.usage.map(|usage| usage.input_tokens),
        output_tokens: report.usage.map(|usage| usage.output_tokens),
        model: None,
        duration_s: None,
        episode_id: Some(report.episode_id.clone()),
        transcript: Vec::new(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    write_shared_transcript(workdir, &transcript)
}

pub fn write_shared_workflow_run(
    workdir: &std::path::Path,
    prompt: &str,
    agent: &str,
    role: &str,
    report: &WorkflowRunReport,
) -> anyhow::Result<String> {
    let token = roko_core::generate_share_token();
    let (report_agent, report_role) = workflow_report_agent_role(report);
    let transcript = roko_serve::routes::shared_runs::RunTranscript {
        id: token.clone(),
        agent: non_empty(agent)
            .map(ToOwned::to_owned)
            .or(report_agent)
            .unwrap_or_else(|| "workflow".to_string()),
        role: non_empty(role)
            .map(ToOwned::to_owned)
            .or(report_role)
            .unwrap_or_else(|| "workflow".to_string()),
        prompt: prompt.to_string(),
        success: report.success,
        gates: report
            .gates
            .iter()
            .map(|gate| (gate.name.clone(), gate.passed))
            .collect(),
        output: non_empty(&report.output).map(ToOwned::to_owned),
        cost_usd: report.cost,
        // GAP: WorkflowRunReport exposes only a combined `token_usage: u64` total; the
        // workflow engine does not track input vs. output token counts separately. To
        // populate these fields the engine would need to accumulate per-turn TokenUsage
        // breakdowns and surface them on WorkflowRunReport.
        input_tokens: None,
        output_tokens: None,
        model: non_empty(&report.model).map(ToOwned::to_owned),
        duration_s: Some(report.duration_secs),
        episode_id: Some(report.run_id.clone()),
        transcript: report.events.clone(),
        timestamp: report
            .events
            .first()
            .map(|event| event.ts.to_rfc3339())
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    };
    write_shared_transcript(workdir, &transcript)
}
// ... (3153 lines omitted) ...
        };
        let workflow = workflow_config_from_band(&architectural);
        assert!(workflow.has_strategy);
        assert!(workflow.has_review);
        assert_eq!(workflow.max_iterations, 4);
        assert_eq!(workflow.max_autofix_attempts, 2);
    }

    #[test]
    fn run_template_band_mapping_prefers_pipeline_defaults_and_keeps_fallback() {
        let pipeline = roko_core::config::PipelineConfig {
            mechanical: roko_core::config::PipelineBandConfig {
                strategist: false,
                reviewers: false,
                reviewer_mode: roko_core::config::PipelineReviewerMode::Quick,
                max_iterations: 1,
            },
            focused: roko_core::config::PipelineBandConfig {
                strategist: false,
                reviewers: false,
                reviewer_mode: roko_core::config::PipelineReviewerMode::Quick,
                max_iterations: 2,
            },
            integrative: roko_core::config::PipelineBandConfig {
                strategist: true,
                reviewers: true,
                reviewer_mode: roko_core::config::PipelineReviewerMode::Quick,
                max_iterations: 2,
            },
            architectural: roko_core::config::PipelineBandConfig {
                strategist: true,
                reviewers: true,
                reviewer_mode: roko_core::config::PipelineReviewerMode::Full,
                max_iterations: 3,
            },
        };

        let standard = workflow_config_for_run_template("standard", &pipeline);
        assert!(!standard.has_strategy);
        assert!(!standard.has_review);
        assert_eq!(standard.max_iterations, 1);
        assert_eq!(standard.max_autofix_attempts, 1);

        let full = workflow_config_for_run_template("full", &pipeline);
        assert!(full.has_strategy);
        assert!(full.has_review);
        assert_eq!(full.max_iterations, 3);
        assert_eq!(full.max_autofix_attempts, 2);

        let fallback = workflow_config_for_run_template("unknown", &pipeline);
        assert_eq!(fallback.has_strategy, WorkflowConfig::standard().has_strategy);
        assert_eq!(fallback.has_review, WorkflowConfig::standard().has_review);
        assert_eq!(fallback.max_iterations, WorkflowConfig::standard().max_iterations);
        assert_eq!(
            fallback.max_autofix_attempts,
            WorkflowConfig::standard().max_autofix_attempts
        );
    }

    #[cfg(feature = "legacy-orchestrate")]
    #[tokio::test]
    async fn minimal_strategy_leaves_system_prompt_unmodified() {
        let tempdir = TempDir::new().expect("tempdir");
        let mut config = Config::default();
        config.prompt.role = "implementer".to_string();

        let base_prompt = build_system_prompt(&config, "Implement the feature.", "Read,Edit");
        let augmented = augment_system_prompt_for_strategy(
            base_prompt.clone(),
            tempdir.path(),
            &config.prompt.role,
            "Implement the feature.",
            "mock-model",
            Some(BenchStrategy::Minimal),
        )
        .await;

        assert_eq!(augmented.system_prompt, base_prompt);
        assert!(augmented.injected_playbook_ids.is_empty());
        assert!(skip_bench_enrichment(Some(BenchStrategy::Minimal)));
        assert!(!skip_bench_enrichment(None));
        assert!(!skip_bench_enrichment(Some(BenchStrategy::ContextEnriched)));
    }

    #[cfg(feature = "legacy-orchestrate")]
    #[tokio::test]
    async fn record_injected_playbook_outcomes_updates_bench_playbooks_only() {
        let tempdir = TempDir::new().expect("tempdir");
        let playbook_root = tempdir.path().join(".roko").join("learn").join("playbooks");
        let store = load_or_create_playbook_store(&playbook_root)
            .await
            .expect("playbook store");
        let playbook = Playbook::new("pb-1", "Audit dependencies");
        store.save(&playbook).await.expect("save playbook");
        let injected_ids = vec!["pb-1".to_string()];

        record_injected_playbook_outcomes(
            tempdir.path(),
            Some(BenchStrategy::ContextEnriched),
            &injected_ids,
            true,
        )
        .await;
        let loaded = store.load("pb-1").await.expect("load").expect("playbook");
        assert_eq!(loaded.success_count, 1);
        assert_eq!(loaded.failure_count, 0);

        record_injected_playbook_outcomes(
            tempdir.path(),
            Some(BenchStrategy::NeuroAugmented),
            &injected_ids,
            false,
        )
        .await;
        let loaded = store.load("pb-1").await.expect("load").expect("playbook");
        assert_eq!(loaded.success_count, 1);
        assert_eq!(loaded.failure_count, 1);

        record_injected_playbook_outcomes(
            tempdir.path(),
            Some(BenchStrategy::Minimal),
            &injected_ids,
            false,
        )
        .await;
        let loaded = store.load("pb-1").await.expect("load").expect("playbook");
        assert_eq!(loaded.success_count, 1);
        assert_eq!(loaded.failure_count, 1);

        record_injected_playbook_outcomes(tempdir.path(), None, &injected_ids, false).await;
        let loaded = store.load("pb-1").await.expect("load").expect("playbook");
        assert_eq!(loaded.success_count, 1);
        assert_eq!(loaded.failure_count, 1);

        let empty_ids: Vec<String> = Vec::new();
        record_injected_playbook_outcomes(
            tempdir.path(),
            Some(BenchStrategy::FullCascade),
            &empty_ids,
            false,
        )
        .await;
        let loaded = store.load("pb-1").await.expect("load").expect("playbook");
        assert_eq!(loaded.success_count, 1);
        assert_eq!(loaded.failure_count, 1);
    }

    #[tokio::test]
    async fn dispatch_agent_uses_exec_agent_for_plain_commands_without_routing() {
        if std::env::var("ANTHROPIC_API_KEY").is_err() {
            eprintln!("skipping: ANTHROPIC_API_KEY not set");
            return;
        }
        let tempdir = TempDir::new().expect("tempdir");
        let config = Config::default();
        let prompt = Engram::builder(Kind::Prompt)
            .body(Body::text("plain-exec-ok"))
            .build();

        let result = dispatch_agent(
            tempdir.path(),
            &config,
            &prompt,
            "plain-exec-ok",
            &Context::now(),
            None,
        )
        .await
        .expect("dispatch succeeds");

        assert!(result.agent_result.success);
        assert_eq!(
            result.agent_result.output.body.as_text().unwrap_or(""),
            "plain-exec-ok"
        );
        assert!(result.external_actions.is_empty());
        assert!(result.injected_playbook_ids.is_empty());
    }

    fn init_git_workdir(workdir: &std::path::Path) {
        run_git(workdir, &["init"]);
        run_git(workdir, &["config", "user.email", "test@example.com"]);
        run_git(workdir, &["config", "user.name", "Roko Test"]);
    }

    fn run_git(workdir: &std::path::Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(workdir)
            .output()
            .expect("run git command");

        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
```

### `crates/roko-runtime/src/pipeline_state.rs` (1109 lines — truncated)

```rust
//! PipelineStateV2: config-driven workflow state machine.
//!
//! This is a PURE state machine with no side effects. It takes events and
//! returns actions. The effect driver executes the actions.
//!
//! Config determines which phases are active:
//! - Express: implement -> gate -> commit
//! - Standard: implement -> gate -> review -> commit
//! - Full: strategy -> implement -> gate -> review -> commit

use serde::{Deserialize, Serialize};

// TODO(arch): Use roko_core::runtime_event::WorkflowOutcome once the crate
// dependency graph matches the architecture reference. In this checkout,
// roko-core currently depends on roko-runtime, so importing roko-core here
// would create a circular dependency and Cargo.toml edits are out of scope.
/// Outcome of a completed workflow run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowOutcome {
    /// Workflow completed successfully, optionally with a commit hash.
    Success {
        /// Commit hash created by the workflow, when commit creation was requested.
        commit_hash: Option<String>,
    },
    /// Workflow halted due to an error or resource limit.
    Halted {
        /// Human-readable halt reason.
        reason: String,
    },
    /// Workflow was cancelled by the user.
    Cancelled,
}

/// Configuration for the pipeline. Determines which phases are active.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkflowConfig {
    /// Include a strategist phase before implementation.
    pub has_strategy: bool,
    /// Include a review phase after gates pass.
    pub has_review: bool,
    /// Maximum implement -> gate -> review iterations.
    pub max_iterations: u32,
    /// Maximum autofix attempts per gate failure.
    pub max_autofix_attempts: u32,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self::standard()
    }
}

#[derive(Debug, serde::Deserialize)]
struct WorkflowConfigToml {
    template: Option<String>,
    has_strategy: Option<bool>,
    has_review: Option<bool>,
    max_iterations: Option<u32>,
    max_autofix_attempts: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkflowTomlScope {
    Root,
    Workflow,
    WorkflowStep,
    Other,
}

impl WorkflowConfig {
    /// Express: implement -> gate -> commit.
    pub fn express() -> Self {
        Self {
            has_strategy: false,
            has_review: false,
            max_iterations: 1,
            max_autofix_attempts: 1,
        }
    }

    /// Standard: implement -> gate -> review -> commit.
    pub fn standard() -> Self {
        Self {
            has_strategy: false,
            has_review: true,
            max_iterations: 2,
            max_autofix_attempts: 2,
        }
    }

    /// Full: strategy -> implement -> gate -> review -> commit.
    pub fn full() -> Self {
        Self {
            has_strategy: true,
            has_review: true,
            max_iterations: 3,
            max_autofix_attempts: 2,
        }
    }

    /// Parse a `WorkflowConfig` from a TOML string.
    ///
    /// The string may contain a `[workflow]` table or just the bare keys. If a
    /// `template` key is present (`"express"`, `"standard"`, or `"full"`), that
    /// preset is used as the base; any additional keys override the preset values.
    ///
    /// Returns an error if the TOML is malformed or `template` is an unknown value.
    pub fn from_toml_str(s: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let raw = parse_workflow_config_toml(s)?;

        let mut config = match raw.template.as_deref() {
            Some("express") => Self::express(),
            Some("standard") | None => Self::standard(),
            Some("full") => Self::full(),
            Some(template) => {
                return Err(config_parse_error(format!(
                    "unknown workflow template: {template}"
                )));
            }
        };

        if let Some(has_strategy) = raw.has_strategy {
            config.has_strategy = has_strategy;
        }
        if let Some(has_review) = raw.has_review {
            config.has_review = has_review;
        }
        if let Some(max_iterations) = raw.max_iterations {
            config.max_iterations = max_iterations;
        }
        if let Some(max_autofix_attempts) = raw.max_autofix_attempts {
            config.max_autofix_attempts = max_autofix_attempts;
        }

        Ok(config)
    }

    /// Load a `WorkflowConfig` from a TOML file on disk.
    ///
    /// The file is read synchronously (this is configuration loading, not hot path).
    /// Returns an error if the file cannot be read or the TOML is invalid.
    pub fn from_toml(
        path: &std::path::Path,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let contents = std::fs::read_to_string(path)?;
        Self::from_toml_str(&contents)
    }
}

fn parse_workflow_config_toml(
    s: &str,
) -> Result<WorkflowConfigToml, Box<dyn std::error::Error + Send + Sync>> {
    let mut workflow = WorkflowConfigToml {
        template: None,
        has_strategy: None,
        has_review: None,
        max_iterations: None,
        max_autofix_attempts: None,
    };
    let mut scope = WorkflowTomlScope::Root;
    let mut saw_workflow_table = false;
    let mut saw_workflow_steps = false;
    let mut steps_have_strategy = false;
    let mut steps_have_review = false;

    for (idx, raw_line) in s.lines().enumerate() {
        let line_number = idx + 1;
        let line = strip_toml_comment(raw_line)
            .map_err(|err| config_parse_error(format!("line {line_number}: {err}")))?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("[[") || line.starts_with('[') {
            scope = parse_workflow_toml_scope(line)
                .map_err(|err| config_parse_error(format!("line {line_number}: {err}")))?;
            if scope == WorkflowTomlScope::Workflow {
                saw_workflow_table = true;
            } else if scope == WorkflowTomlScope::WorkflowStep {
                saw_workflow_steps = true;
            }
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(config_parse_error(format!(
                "line {line_number}: expected key = value"
            )));
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() || value.is_empty() {
            return Err(config_parse_error(format!(
                "line {line_number}: expected key = value"
            )));
        }

        let should_read = if saw_workflow_table {
// ... (709 lines omitted) ...

    #[test]
    fn toml_full_template_with_override() {
        let cfg = WorkflowConfig::from_toml_str("template = \"full\"\nmax_iterations = 5").unwrap();
        assert!(cfg.has_strategy);
        assert_eq!(cfg.max_iterations, 5);
    }

    #[test]
    fn toml_table_form() {
        let src = "[workflow]\ntemplate = \"standard\"\nmax_autofix_attempts = 3";
        let cfg = WorkflowConfig::from_toml_str(src).unwrap();
        assert_eq!(cfg.max_autofix_attempts, 3);
        assert!(cfg.has_review);
    }

    #[test]
    fn toml_bare_keys_no_template() {
        let cfg = WorkflowConfig::from_toml_str(
            "has_strategy = true\nhas_review = false\nmax_iterations = 2\nmax_autofix_attempts = 1",
        )
        .unwrap();
        assert!(cfg.has_strategy);
        assert!(!cfg.has_review);
    }

    #[test]
    fn toml_unknown_template_is_error() {
        assert!(WorkflowConfig::from_toml_str("template = \"bogus\"").is_err());
    }

    #[test]
    fn toml_steps_infer_strategy_and_review_flags() {
        let src = r#"
[workflow]
max_iterations = 4

[[workflow.steps]]
name = "strategy"
role = "strategist"

[[workflow.steps]]
name = "implement"
role = "implementer"
"#;
        let cfg = WorkflowConfig::from_toml_str(src).unwrap();
        assert!(cfg.has_strategy);
        assert!(!cfg.has_review);
        assert_eq!(cfg.max_iterations, 4);
    }

    #[test]
    fn checkpoint_round_trip_implementing() {
        let mut sm = PipelineStateV2::new(WorkflowConfig::standard(), "add feature".into());
        sm.step(PipelineInput::Start);

        let json = sm.checkpoint().unwrap();
        let restored = PipelineStateV2::from_checkpoint(&json).unwrap();

        assert_eq!(restored.phase, Phase::Implementing);
        assert_eq!(restored.iteration, 1);
        assert_eq!(restored.original_prompt, "add feature");
    }

    #[test]
    fn checkpoint_preserves_review_findings() {
        let mut sm = PipelineStateV2::new(WorkflowConfig::standard(), "feat".into());
        sm.review_findings = vec!["needs error handling".into(), "add docs".into()];
        sm.iteration = 2;
        sm.phase = Phase::Implementing;

        let json = sm.checkpoint().unwrap();
        let restored = PipelineStateV2::from_checkpoint(&json).unwrap();

        assert_eq!(
            restored.review_findings,
            vec!["needs error handling", "add docs"]
        );
        assert_eq!(restored.iteration, 2);
    }

    #[test]
    fn checkpoint_halted_phase() {
        let mut sm = PipelineStateV2::new(WorkflowConfig::express(), "task".into());
        sm.phase = Phase::Halted {
            reason: "compile failed".into(),
        };

        let json = sm.checkpoint().unwrap();
        let restored = PipelineStateV2::from_checkpoint(&json).unwrap();

        assert!(restored.phase.is_terminal());
        assert!(
            matches!(restored.phase, Phase::Halted { ref reason } if reason == "compile failed")
        );
    }

    #[test]
    fn checkpoint_full_config() {
        let config = WorkflowConfig::full();
        let sm = PipelineStateV2::new(config, "complex".into());

        let json = sm.checkpoint().unwrap();
        let restored = PipelineStateV2::from_checkpoint(&json).unwrap();

        assert!(restored.config.has_strategy);
        assert!(restored.config.has_review);
        assert_eq!(restored.config.max_iterations, 3);
    }

    #[test]
    fn from_checkpoint_rejects_invalid_json() {
        assert!(PipelineStateV2::from_checkpoint("not json at all").is_err());
        assert!(PipelineStateV2::from_checkpoint("{}").is_err());
    }

    #[test]
    fn checkpoint_preserves_phase_and_iteration() {
        let mut sm = PipelineStateV2::new(WorkflowConfig::express(), "fix bug".into());

        sm.step(PipelineInput::Start);
        sm.step(PipelineInput::AgentCompleted {
            output: "done".into(),
            files_changed: 2,
        });

        let checkpoint = sm.clone();

        assert_eq!(checkpoint.phase, Phase::Gating);
        assert_eq!(checkpoint.iteration, 1);
        assert_eq!(checkpoint.original_prompt, "fix bug");
    }

    #[test]
    fn resume_from_gating_checkpoint_skips_implement() {
        let mut sm = PipelineStateV2::new(WorkflowConfig::express(), "fix bug".into());

        sm.step(PipelineInput::Start);
        sm.step(PipelineInput::AgentCompleted {
            output: "done".into(),
            files_changed: 2,
        });

        let mut resumed = sm.clone();

        let out = resumed.step(PipelineInput::GatesPassed);
        assert!(matches!(out, PipelineOutput::Commit));

        let out = resumed.step(PipelineInput::CommitDone {
            hash: "abc123".into(),
        });
        assert!(matches!(
            out,
            PipelineOutput::Done {
                outcome: WorkflowOutcome::Success {
                    commit_hash: Some(hash)
                }
            } if hash == "abc123"
        ));
        assert_eq!(resumed.phase, Phase::Complete);
    }

    #[test]
    fn resume_does_not_affect_original() {
        let mut original = PipelineStateV2::new(WorkflowConfig::express(), "fix bug".into());

        original.step(PipelineInput::Start);
        original.step(PipelineInput::AgentCompleted {
            output: "done".into(),
            files_changed: 2,
        });

        let mut checkpoint = original.clone();

        original.step(PipelineInput::GatesPassed);
        original.step(PipelineInput::CommitDone {
            hash: "original".into(),
        });

        let out = checkpoint.step(PipelineInput::GateFailed {
            gate: "compile".into(),
            output: "error[E0308]".into(),
        });
        assert!(matches!(out, PipelineOutput::SpawnAutoFixer { .. }));

        checkpoint.step(PipelineInput::AgentCompleted {
            output: "fixed".into(),
            files_changed: 1,
        });
        checkpoint.step(PipelineInput::GatesPassed);
        checkpoint.step(PipelineInput::CommitDone {
            hash: "checkpoint".into(),
        });

        assert_eq!(original.phase, Phase::Complete);
        assert_eq!(checkpoint.phase, Phase::Complete);
        assert_eq!(original.autofix_attempts, 0);
        assert!(checkpoint.autofix_attempts > 0);
    }
}
```

---

## Read-Only Context (do not modify)

### `crates/roko-cli/src/config.rs` (3806 lines — signatures only)

```rust
22:pub struct Config {
24:    pub agent: AgentConfig,
27:    pub auto_plan: bool,
30:    pub dreams: DreamsConfig,
33:    pub daimon: DaimonConfig,
36:    pub tools: ToolsConfig,
39:    pub prompt: PromptConfig,
42:    pub repos: Vec<RepoConfig>,
45:    pub gates: Vec<GateConfig>,
48:    pub executor: ExecutorConfig,
51:    pub runtime: RuntimeControlConfig,
54:    pub budget: BudgetConfig,
57:    pub providers: HashMap<String, ProviderConfig>,
60:    pub models: HashMap<String, ModelProfile>,
63:    pub serve: ServeConfig,
66:    pub log_format: Option<String>,
69:    pub bind: Option<String>,
72:    pub data_dir: Option<PathBuf>,
75:impl Default for Config {
99:impl Config {
101:    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
109:    pub fn parse_toml(text: &str) -> Result<Self> {
114:    pub fn to_toml(&self) -> Result<String> {
119:    pub fn default_toml_template(cloud: bool) -> Result<String> {
126:pub struct AgentConfig {
128:    pub command: String,
131:    pub args: Vec<String>,
134:    pub model: Option<String>,
137:    pub effort: String,
140:    pub bare_mode: bool,
143:    pub fallback_model: Option<String>,
146:    pub timeout_ms: u64,
150:    pub env: Vec<(String, String)>,
155:    pub clean_output: bool,
160:    pub mcp_config: Option<PathBuf>,
163:    pub tier_models: std::collections::HashMap<String, String>,
166:    pub escalation: EscalationConfig,
171:pub struct ToolsConfig {
174:    pub prefer_mcp: bool,
177:    pub global_denied: Vec<String>,
```

### `crates/roko-cli/src/commands/util.rs` (1532 lines — signatures only)

```rust
```

---

## Verification Commands

Run these and fix any failures:
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
cargo check -p roko-runtime
cargo clippy -p roko-runtime --no-deps -- -D warnings
```

## Do NOT

- Rewrite the entire implementation from scratch
- Add features not in the original prompt
- Modify files outside the write scope
- Skip running verification commands
