//! Agent execution helper for direct CLI flows such as PRD/research/plan generation.
//!
//! Used by `roko prd`, `roko research`, and `roko plan generate` to invoke
//! an agent that can read/write files while preserving provider-aware routing,
//! safety scoping, resume threading, and learning-episode persistence.

use std::path::Path;
use std::time::Instant;

use crate::agent_config::{command_from_config, model_from_config};
use crate::agent_episode::build_capture_episode;
use crate::agent_spawn::{SpawnAgentSpec, spawn_agent_scoped};
use crate::learning_helpers::{
    capture_runtime_model_slugs, distillation_model_caller, install_capture_distillation,
    provider_id_for_model, record_persisted_provider_health, resolve_capture_model_slug,
};
use anyhow::{Context as _, Result};
use roko_core::agent::ProviderKind;
use roko_core::agent::resolve_model;
use roko_core::{Body, Context, Kind, Signal, Usage};
use roko_learn::runtime_feedback::{CompletedRunInput, LearningRuntime};

/// Options for agent execution.
pub struct AgentExecOpts<'a> {
    /// The prompt to send to the agent.
    pub prompt: &'a str,
    /// Working directory for the agent.
    pub workdir: &'a Path,
    /// Model to use (e.g. "claude-sonnet-4-6"). If None, uses CLI default.
    pub model: Option<&'a str>,
    /// Reasoning effort label to pass to Claude.
    pub effort: Option<&'a str>,
    /// Additional system prompt to append.
    pub system_prompt: Option<&'a str>,
    /// Claude session id to resume, if any.
    pub resume_session: Option<&'a str>,
    /// Extra env vars for the child process (gateway config, etc).
    pub env_vars: &'a [(String, String)],
    /// Logical role used to scope safety policies and model routing.
    ///
    /// When set, the safety layer applies role-specific policies and the
    /// CascadeRouter can make role-aware model selection decisions.
    pub role: Option<&'a str>,
    /// Tool restriction. `Some("none")` disables all tools. `None` uses provider defaults.
    pub allowed_tools: Option<&'a str>,
}

/// Episode metadata for agent execution paths that should persist learning data.
pub struct AgentExecEpisode<'a> {
    /// Logical task kind used for episode routing and summaries.
    pub task_kind: &'a str,
    /// Stable task identifier for the episode record.
    pub task_id: &'a str,
}

/// What one direct agent run returned, with the usage the provider reported.
#[derive(Debug, Clone)]
pub struct AgentCapture {
    /// `0` when the agent succeeded, `1` otherwise.
    pub exit_code: i32,
    /// The agent's rendered output text.
    pub output: String,
    /// Tokens and cost of the run. The cost is back-filled from model pricing
    /// when the provider reported tokens but no dollar amount.
    pub usage: Usage,
    /// API slug of the model that ran.
    pub model: String,
    /// Configured provider id of the model, or its provider kind's label when
    /// no configured provider names it.
    pub provider: String,
    /// Wall-clock milliseconds the run took.
    pub duration_ms: u64,
}

/// Run the configured direct agent path and return just the exit code.
///
/// Convenience wrapper around [`run_agent_capture`] for callers that
/// don't need the agent's text output.
pub async fn run_agent(opts: AgentExecOpts<'_>) -> Result<i32> {
    run_agent_capture(opts).await.map(|(code, _)| code)
}

/// Run the configured direct agent path, echo the output, and persist an episode.
pub async fn run_agent_logged(
    opts: AgentExecOpts<'_>,
    episode: AgentExecEpisode<'_>,
) -> Result<i32> {
    run_agent_capture_logged(opts, episode)
        .await
        .map(|(code, _)| code)
}

/// Run the configured direct agent path and return `(exit_code, output_text)`.
pub async fn run_agent_capture(opts: AgentExecOpts<'_>) -> Result<(i32, String)> {
    run_agent_capture_impl(opts, true, None)
        .await
        .map(|capture| (capture.exit_code, capture.output))
}

/// Run the configured direct agent path, echo the output, and persist an episode.
pub async fn run_agent_capture_logged(
    opts: AgentExecOpts<'_>,
    episode: AgentExecEpisode<'_>,
) -> Result<(i32, String)> {
    run_agent_capture_impl(opts, true, Some(episode))
        .await
        .map(|capture| (capture.exit_code, capture.output))
}

/// Like [`run_agent_logged`], but also record what the call cost through
/// `spend`. The episode it persists carries no usage, so this is where the
/// call's spend is recorded.
pub async fn run_agent_logged_with_spend(
    opts: AgentExecOpts<'_>,
    episode: AgentExecEpisode<'_>,
    spend: &crate::plan_authoring::AuthoringSpend,
) -> Result<i32> {
    let call = run_agent_capture_impl(opts, true, Some(episode)).await?;
    spend.record(&call).await;
    Ok(call.exit_code)
}

/// Run the configured direct agent path and return `(exit_code, output_text)`
/// without echoing the agent's rendered output to stdout.
pub async fn run_agent_capture_silent(opts: AgentExecOpts<'_>) -> Result<(i32, String)> {
    run_agent_capture_silent_with_usage(opts)
        .await
        .map(|capture| (capture.exit_code, capture.output))
}

/// Like [`run_agent_capture_silent`], but also return the usage the provider
/// reported, so the caller can account for what the run cost.
pub async fn run_agent_capture_silent_with_usage(opts: AgentExecOpts<'_>) -> Result<AgentCapture> {
    run_agent_capture_impl(opts, false, None).await
}

/// Like [`run_agent_capture_silent`], but also record what the call cost
/// through `spend`. The capture episode its caller persists carries no usage,
/// so this is where the call's spend is recorded (bug-86ff56).
pub async fn run_agent_capture_silent_recorded(
    opts: AgentExecOpts<'_>,
    spend: &crate::plan_authoring::AuthoringSpend,
) -> Result<(i32, String)> {
    let call = run_agent_capture_silent_with_usage(opts).await?;
    spend.record(&call).await;
    Ok((call.exit_code, call.output))
}

async fn run_agent_capture_impl(
    opts: AgentExecOpts<'_>,
    echo_output: bool,
    episode: Option<AgentExecEpisode<'_>>,
) -> Result<AgentCapture> {
    let started = Instant::now();
    let routing_config = roko_core::config::loader::load_config_unified(opts.workdir)
        .with_context(|| format!("load routing config from {}", opts.workdir.display()))?;
    let routing_enabled = !routing_config.providers.is_empty() || !routing_config.models.is_empty();

    // Fail fast if the agent command is still the test-only default.
    // `"cat"` just echoes the prompt back, producing garbage output.
    if !routing_enabled {
        let cmd = command_from_config(opts.workdir).unwrap_or_default();
        if cmd == "cat" || cmd.is_empty() {
            anyhow::bail!(
                "agent command is {:?} (the test-only default). \
                 Set `command = \"claude\"` (or another agent CLI) in roko.toml under [agent], \
                 or re-run `roko init` to generate a working config.",
                if cmd.is_empty() { "cat" } else { &cmd }
            );
        }
    }
    let model = opts
        .model
        .map(str::to_string)
        .or_else(|| model_from_config(opts.workdir))
        .unwrap_or_else(|| {
            if routing_enabled {
                routing_config.agent.default_model.clone()
            } else {
                "claude-opus-4-6".to_string()
            }
        });
    let resolved = resolve_model(&routing_config, &model);
    let mut extra_args = Vec::new();
    if resolved.provider_kind == ProviderKind::ClaudeCli
        && let Some(session_id) = opts.resume_session
    {
        extra_args.push("--resume".to_string());
        extra_args.push(session_id.to_string());
    }
    let agent = spawn_agent_scoped(
        &routing_config,
        SpawnAgentSpec {
            model: model.clone(),
            command: routing_config.agent.command.clone(),
            timeout_ms: Some(300_000), // 5 min for CLI flows (plan generation / research tasks)
            system_prompt: opts.system_prompt.map(str::to_string),
            cached_content: None,
            tools: opts.allowed_tools.map(str::to_string),
            mcp_config: None,
            working_dir: Some(opts.workdir.to_path_buf()),
            env: opts.env_vars.to_vec(),
            extra_args,
            effort: Some(opts.effort.unwrap_or("medium").to_string()),
            bare_mode: true,
            // Provider permission checks stay on unless the workspace opts
            // out with `runner.dangerously_skip_permissions`.
            dangerously_skip_permissions: routing_config.runner.dangerously_skip_permissions,
            name: format!("{}:{model}", resolved.provider_kind.label()),
            role: opts.role.map(str::to_string),
        },
        format!("create agent for model {model}"),
    )?;

    let prompt = Signal::builder(Kind::Prompt)
        .body(Body::text(opts.prompt))
        .build();
    tracing::info!(
        model = %model,
        role = ?opts.role,
        provider = %resolved.provider_kind.label(),
        prompt_len = opts.prompt.len(),
        "agent_exec: dispatching prompt"
    );

    // Run agent with a concurrent progress ticker so the user sees activity
    // during long-running LLM calls (especially non-Claude backends that
    // don't emit streaming stderr).
    let tick_model = model.clone();
    let tick_provider = resolved.provider_kind.label().to_string();
    let tick_start = Instant::now();
    let ticker = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        interval.tick().await; // skip immediate first tick
        loop {
            interval.tick().await;
            let elapsed = tick_start.elapsed().as_secs();
            let mins = elapsed / 60;
            let secs = elapsed % 60;
            if mins > 0 {
                eprintln!(
                    "  ⏳ Agent working... ({mins}m {secs}s) [{tick_model} via {tick_provider}]"
                );
            } else {
                eprintln!("  ⏳ Agent working... ({secs}s) [{tick_model} via {tick_provider}]");
            }
        }
    });
    let result = agent.run(&prompt, &Context::now()).await;
    ticker.abort();

    let rendered = result.output.body.as_text().unwrap_or("").to_string();
    let elapsed_ms = started.elapsed().as_millis();
    tracing::info!(
        model = %model,
        success = result.success,
        output_len = rendered.len(),
        output_empty = rendered.trim().is_empty(),
        elapsed_ms = elapsed_ms,
        "agent_exec: agent returned"
    );
    // Print completion summary to stderr
    {
        let secs = elapsed_ms / 1000;
        let mins = secs / 60;
        let s = secs % 60;
        let len = rendered.len();
        if result.success {
            if mins > 0 {
                eprintln!("  ✓ Agent completed ({mins}m {s}s, {len} bytes) [{model}]");
            } else {
                eprintln!("  ✓ Agent completed ({s}s, {len} bytes) [{model}]");
            }
        } else {
            if mins > 0 {
                eprintln!("  ✗ Agent failed ({mins}m {s}s, {len} bytes) [{model}]");
            } else {
                eprintln!("  ✗ Agent failed ({s}s, {len} bytes) [{model}]");
            }
        }
    }
    if rendered.trim().is_empty() {
        tracing::warn!(
            model = %model,
            role = ?opts.role,
            "agent_exec: agent returned empty output text"
        );
    }
    if echo_output && !rendered.is_empty() {
        print!("{rendered}");
    }

    let exit_code = i32::from(!result.success);
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    if let Some(episode) = episode {
        persist_capture_episode(
            opts.workdir,
            resolved.provider_kind.label(),
            Some(&resolved.slug),
            episode.task_kind,
            episode.task_id,
            opts.prompt,
            &rendered,
            exit_code == 0,
            duration_ms,
            opts.resume_session,
        )
        .await?;
    }

    let mut usage = result.usage;
    crate::dispatch_v2::fill_usage_cost_from_pricing(
        &mut usage,
        resolved.profile.as_ref(),
        &resolved.slug,
    );
    Ok(AgentCapture {
        exit_code,
        output: rendered,
        usage,
        provider: provider_id_for_model(&routing_config, &model)
            .unwrap_or_else(|| resolved.provider_kind.label().to_string()),
        model: resolved.slug,
        duration_ms,
    })
}

/// Persist a lightweight learning episode for a direct agent-exec CLI path.
pub async fn persist_capture_episode(
    workdir: &Path,
    agent_command: &str,
    model: Option<&str>,
    task_kind: &str,
    task_id: &str,
    prompt: &str,
    output: &str,
    success: bool,
    wall_time_ms: u64,
    resume_session: Option<&str>,
) -> Result<()> {
    let config = roko_core::config::loader::load_config_unified(workdir).unwrap_or_default();
    let capture_model = resolve_capture_model_slug(&config, model);
    let provider_from_config = capture_model
        .as_deref()
        .and_then(|model_slug| provider_id_for_model(&config, model_slug))
        .or_else(|| model.and_then(|model_key| provider_id_for_model(&config, model_key)));

    let episode_model = capture_model.as_deref().or(model);
    let (mut episode, fallback_provider) = build_capture_episode(
        agent_command,
        episode_model,
        task_kind,
        task_id,
        prompt,
        output,
        success,
        wall_time_ms,
        resume_session,
    );
    let provider = provider_from_config.unwrap_or(fallback_provider);
    if !provider.trim().is_empty() {
        episode
            .extra
            .insert("provider".to_string(), serde_json::json!(provider.clone()));
    }

    let model_slugs = capture_runtime_model_slugs(&config, episode.model.as_str());
    let mut runtime = if model_slugs.is_empty() {
        LearningRuntime::open_for_project(workdir).await
    } else {
        LearningRuntime::open_for_project_with_models(workdir, model_slugs).await
    }
    .map_err(|e| anyhow::anyhow!("open learning runtime: {e}"))?;
    install_capture_distillation(&mut runtime, workdir, distillation_model_caller(workdir));

    let mut completed = CompletedRunInput::from_episode(episode);
    completed.provider = (!provider.trim().is_empty()).then_some(provider.clone());
    runtime
        .record_completed_run(completed)
        .await
        .map_err(|e| anyhow::anyhow!("record learning feedback: {e}"))?;
    record_persisted_provider_health(workdir, &provider, success)?;
    Ok(())
}

/// Classification of agent crash from stderr output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentCrashClass {
    AuthenticationError,
    RateLimited,
    ContextOverflow,
    ModelNotFound,
    NetworkError,
    Unknown,
}

impl AgentCrashClass {
    /// Whether this crash class is worth retrying automatically.
    ///
    /// `Unknown` is treated as retriable because the agent process can crash
    /// for transient reasons (OOM kill, signal, subprocess race) that a fresh
    /// invocation will not reproduce. Callers should cap Unknown retries lower
    /// than the cap used for `RateLimited`/`NetworkError`.
    pub fn is_retriable(&self) -> bool {
        matches!(self, Self::RateLimited | Self::NetworkError | Self::Unknown)
    }

    /// Human-readable hint for recovering from this crash class.
    pub fn recovery_hint(&self) -> &str {
        match self {
            Self::AuthenticationError => {
                "Check your API key: ensure the correct key is set in the environment or roko.toml"
            }
            Self::RateLimited => {
                "Rate limited by the provider; wait a moment and retry, or switch models"
            }
            Self::ContextOverflow => {
                "Prompt exceeds the model's context window; reduce input size or switch to a larger-context model"
            }
            Self::ModelNotFound => {
                "The requested model was not found; check the model name in roko.toml or provider docs"
            }
            Self::NetworkError => {
                "Network error reaching the provider; check your connection and try again"
            }
            Self::Unknown => "Agent crashed for an unknown reason; inspect the full stderr output",
        }
    }
}

/// Classify an agent crash from its stderr output.
///
/// Uses simple `contains()` checks — no regex needed.
pub fn classify_agent_crash(stderr: &str) -> AgentCrashClass {
    // Check most-specific patterns first.
    if stderr.contains("401")
        || stderr.contains("invalid_api_key")
        || stderr.contains("Missing API key")
    {
        return AgentCrashClass::AuthenticationError;
    }
    if stderr.contains("429") || stderr.contains("rate_limit") {
        return AgentCrashClass::RateLimited;
    }
    if stderr.contains("context_length_exceeded") || stderr.contains("too many tokens") {
        return AgentCrashClass::ContextOverflow;
    }
    if stderr.contains("model_not_found") || stderr.contains("does not exist") {
        return AgentCrashClass::ModelNotFound;
    }
    if stderr.contains("connection")
        || stderr.contains("timeout")
        || stderr.contains("ECONNREFUSED")
    {
        return AgentCrashClass::NetworkError;
    }
    AgentCrashClass::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan_authoring::AuthoringSpend;
    use roko_learn::episode_logger::EpisodeLogger;
    use tempfile::TempDir;

    #[tokio::test]
    async fn persist_capture_episode_records_learning_episode() {
        let tmp = TempDir::new().expect("tempdir");

        persist_capture_episode(
            tmp.path(),
            "claude",
            Some("claude-sonnet-4-6"),
            "plan-generate",
            "plan:generate:demo",
            "prompt body",
            "output body",
            true,
            42,
            Some("sess-1"),
        )
        .await
        .expect("persist capture episode");

        let episodes_path = tmp.path().join(".roko").join("episodes.jsonl");
        let episodes = EpisodeLogger::read_all_lossy(&episodes_path).await.unwrap();
        assert_eq!(episodes.len(), 1);
        let episode = &episodes[0];
        assert_eq!(episode.agent_id, "claude");
        assert_eq!(episode.task_id, "plan:generate:demo");
        assert_eq!(episode.kind, "agent_turn");
        assert_eq!(episode.model, "claude-sonnet-4-6");
        assert!(episode.success);
        assert_eq!(
            episode.extra.get("task_kind"),
            Some(&serde_json::json!("plan-generate"))
        );
        assert_eq!(
            episode.extra.get("task_category"),
            Some(&serde_json::json!("scaffolding"))
        );
        assert_eq!(
            episode.extra.get("plan_id"),
            Some(&serde_json::json!("demo"))
        );
        assert!(
            !tmp.path()
                .join(".roko")
                .join("learn")
                .join("episodes.jsonl")
                .exists()
        );
        assert!(
            !tmp.path()
                .join(".roko")
                .join("memory")
                .join("episodes.jsonl")
                .exists()
        );
    }

    #[tokio::test]
    async fn persist_capture_episode_resolves_model_key_to_slug_and_provider() {
        let tmp = TempDir::new().expect("tempdir");
        std::fs::write(
            tmp.path().join("roko.toml"),
            r#"
[agent]
default_model = "glm-mini"
command = "claude"

[providers.zai]
kind = "openai_compat"
base_url = "https://api.z.ai/api/paas/v4"
api_key_env = ""

[models.glm-mini]
provider = "zai"
slug = "glm-5.1"
context_window = 131072
tool_format = "openai_json"
"#,
        )
        .expect("write roko.toml");

        persist_capture_episode(
            tmp.path(),
            "claude",
            Some("glm-mini"),
            "plan-generate",
            "plan:generate:glm",
            "prompt body",
            "output body",
            true,
            42,
            None,
        )
        .await
        .expect("persist capture episode");

        let episodes_path = tmp.path().join(".roko").join("episodes.jsonl");
        let episodes = EpisodeLogger::read_all_lossy(&episodes_path).await.unwrap();
        assert_eq!(episodes.len(), 1);
        assert!(!tmp.path().join(".roko/learn/episodes.jsonl").exists());
        assert!(!tmp.path().join(".roko/memory/episodes.jsonl").exists());
        assert_eq!(episodes[0].model, "glm-5.1");
        assert_eq!(
            episodes[0].extra.get("provider"),
            Some(&serde_json::json!("zai"))
        );

        let cascade_path = tmp
            .path()
            .join(".roko")
            .join("learn")
            .join("cascade-router.json");
        let cascade: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(cascade_path).unwrap()).unwrap();
        assert_eq!(
            cascade
                .pointer("/confidence_stats/glm-5.1/trials")
                .and_then(serde_json::Value::as_u64),
            Some(1)
        );

        let health_path = tmp
            .path()
            .join(".roko")
            .join("learn")
            .join("provider-health.json");
        let health: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(health_path).unwrap()).unwrap();
        assert_eq!(
            health
                .pointer("/providers/zai/total_requests")
                .and_then(serde_json::Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn dispatch_surfaces_provide_episodes() {
        let pipeline = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/plan_generate/pipeline.rs"
        ))
        .unwrap();
        assert!(
            pipeline.contains("\"plan-generate\"") && pipeline.contains("persist_capture_episode"),
            "generate_plan must persist a plan-generate episode"
        );
    }

    #[test]
    fn classify_agent_crash_auth() {
        assert_eq!(
            classify_agent_crash("error: 401 Unauthorized"),
            AgentCrashClass::AuthenticationError
        );
        assert_eq!(
            classify_agent_crash("invalid_api_key: check your key"),
            AgentCrashClass::AuthenticationError
        );
        assert_eq!(
            classify_agent_crash("Missing API key"),
            AgentCrashClass::AuthenticationError
        );
    }

    #[test]
    fn classify_agent_crash_rate_limit() {
        assert_eq!(
            classify_agent_crash("HTTP 429 Too Many Requests"),
            AgentCrashClass::RateLimited
        );
        assert_eq!(
            classify_agent_crash("rate_limit exceeded"),
            AgentCrashClass::RateLimited
        );
    }

    #[test]
    fn classify_agent_crash_context() {
        assert_eq!(
            classify_agent_crash("context_length_exceeded"),
            AgentCrashClass::ContextOverflow
        );
        assert_eq!(
            classify_agent_crash("too many tokens for this model"),
            AgentCrashClass::ContextOverflow
        );
    }

    #[test]
    fn classify_agent_crash_model() {
        assert_eq!(
            classify_agent_crash("model_not_found: gpt-99"),
            AgentCrashClass::ModelNotFound
        );
        assert_eq!(
            classify_agent_crash("The model does not exist"),
            AgentCrashClass::ModelNotFound
        );
    }

    #[test]
    fn classify_agent_crash_network() {
        assert_eq!(
            classify_agent_crash("connection refused"),
            AgentCrashClass::NetworkError
        );
        assert_eq!(
            classify_agent_crash("request timeout after 30s"),
            AgentCrashClass::NetworkError
        );
        assert_eq!(
            classify_agent_crash("ECONNREFUSED 127.0.0.1:443"),
            AgentCrashClass::NetworkError
        );
    }

    #[test]
    fn classify_agent_crash_unknown() {
        assert_eq!(
            classify_agent_crash("segfault at 0xdeadbeef"),
            AgentCrashClass::Unknown
        );
    }

    #[test]
    fn retriable_variants() {
        assert!(!AgentCrashClass::AuthenticationError.is_retriable());
        assert!(AgentCrashClass::RateLimited.is_retriable());
        assert!(!AgentCrashClass::ContextOverflow.is_retriable());
        assert!(!AgentCrashClass::ModelNotFound.is_retriable());
        assert!(AgentCrashClass::NetworkError.is_retriable());
        // Unknown is retriable: agent can crash for transient reasons (OOM,
        // signal) that a fresh invocation will not reproduce.
        assert!(AgentCrashClass::Unknown.is_retriable());
    }

    #[test]
    fn recovery_hints_non_empty() {
        let variants = [
            AgentCrashClass::AuthenticationError,
            AgentCrashClass::RateLimited,
            AgentCrashClass::ContextOverflow,
            AgentCrashClass::ModelNotFound,
            AgentCrashClass::NetworkError,
            AgentCrashClass::Unknown,
        ];
        for v in variants {
            assert!(
                !v.recovery_hint().is_empty(),
                "{v:?} hint must not be empty"
            );
        }
    }

    /// What the fake planner charges per call. A power of two, so it survives
    /// the provider usage's `f32` exactly.
    const CALL_COST_USD: f64 = 0.0625;

    const DEMO_PLAN: &str = r#"[meta]
plan = "demo"
total = 1
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T01"
title = "Write the hello world program"
description = "Create hello/main.rs, a Rust program that prints hello world."
status = "ready"
role = "implementer"
tier = "focused"
files = ["hello/main.rs"]
depends_on = []

[[task.verify]]
phase = "structural"
command = "test -f hello/main.rs"
fail_msg = "hello/main.rs was not written"
"#;

    /// A workspace whose only model runs a fake Claude CLI: it answers every
    /// prompt with [`DEMO_PLAN`] in a fenced toml block and reports
    /// [`CALL_COST_USD`].
    fn fake_planner_workspace() -> TempDir {
        use std::os::unix::fs::PermissionsExt;

        let workspace = TempDir::new().expect("workspace");
        let text = format!("```toml\n{DEMO_PLAN}```\n");
        let assistant = serde_json::json!({
            "type": "assistant",
            "message": {"content": [{"type": "text", "text": text}]},
        });
        let result = serde_json::json!({
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "result": text,
            "model": "claude-sonnet-4-6",
            "total_cost_usd": CALL_COST_USD,
            "usage": {"input_tokens": 1200, "output_tokens": 340},
        });
        let script = workspace.path().join("fake-claude");
        std::fs::write(
            &script,
            format!("#!/bin/sh\ncat >/dev/null\ncat <<'JSON'\n{assistant}\n{result}\nJSON\n"),
        )
        .expect("write fake provider");
        let mut permissions = std::fs::metadata(&script)
            .expect("fake provider metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).expect("make fake provider executable");
        std::fs::write(
            workspace.path().join("roko.toml"),
            format!(
                r#"[agent]
default_model = "fake-model"

[providers.fake-cli]
kind = "claude_cli"
command = {script:?}

[models.fake-model]
provider = "fake-cli"
slug = "claude-sonnet-4-6"
context_window = 200000

# One planner call per generation: these tests count cost rows, and the
# spec-quality gate would ask again for this minimal plan (3218).
[spec_quality]
mode = "off"
"#,
                script = script.display().to_string()
            ),
        )
        .expect("write roko.toml");
        workspace
    }

    /// The rows of `.roko/learn/costs.jsonl`, leaving out the background
    /// distillation call's, which is recorded under its own role.
    fn agent_cost_rows(workdir: &Path) -> Vec<serde_json::Value> {
        std::fs::read_to_string(workdir.join(".roko").join("learn").join("costs.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("JSONL row"))
            .filter(|row| row["role"] != crate::learning_helpers::DISTILLATION_ROLE)
            .collect()
    }

    /// bug-ac5432: a plan generation leaves one cost record for its one agent
    /// call, carrying the reported cost. The generation episode adds no $0
    /// row beside it.
    #[tokio::test]
    async fn plan_generation_writes_one_cost_record() {
        let workspace = fake_planner_workspace();

        let request = crate::plan_generate::PlanRequest::new(
            crate::plan_generate::PlanSource::Text {
                text: "# Demo\n\nPrint hello world.\n",
                kind: "prompt",
            },
            "demo",
            workspace.path(),
        );
        crate::plan_generate::generate_plan(request)
            .await
            .expect("generate plan");

        let rows = agent_cost_rows(workspace.path());
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0]["cost_usd"], CALL_COST_USD);
        assert_eq!(rows[0]["plan_id"], "demo");
        assert_eq!(
            rows[0]["task_id"],
            crate::plan_authoring::GENERATION_SPEND_TASK_ID
        );
    }

    /// bug-ac5432: `roko plan generate --from-backlog` runs its agent through
    /// `run_agent_logged_with_spend`, which records the reported cost against
    /// the plan, where it used to leave only a $0 row.
    #[tokio::test]
    async fn plan_generate_records_the_agent_spend() {
        let workspace = fake_planner_workspace();
        let spend =
            crate::plan_authoring::AuthoringSpend::generation(workspace.path(), "demo", None);

        let exit_code = run_agent_logged_with_spend(
            AgentExecOpts {
                prompt: "Plan the demo.",
                workdir: workspace.path(),
                model: Some("fake-model"),
                effort: Some("high"),
                system_prompt: None,
                resume_session: None,
                env_vars: &[],
                role: Some("strategist"),
                allowed_tools: None,
            },
            AgentExecEpisode {
                task_kind: "plan-generate",
                task_id: "plan:generate:backlog:7",
            },
            &spend,
        )
        .await
        .expect("run agent");

        assert_eq!(exit_code, 0);
        let rows = agent_cost_rows(workspace.path());
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0]["cost_usd"], CALL_COST_USD);
        assert_eq!(rows[0]["input_tokens"], 1200);
        assert_eq!(rows[0]["plan_id"], "demo");
    }

    /// bug-86ff56: a one-off call, as `roko research`, `roko do` and the PRD
    /// drafting commands make them, records the reported cost under the
    /// operation's task and role, with no plan id.
    #[tokio::test]
    async fn research_calls_record_spend() {
        let workspace = fake_planner_workspace();
        let task_id = "research:topic:graph-engines";
        let spend = AuthoringSpend::operation(workspace.path(), task_id, "researcher");

        let (exit_code, _) = run_agent_capture_silent_recorded(
            AgentExecOpts {
                prompt: "Research graph engines.",
                workdir: workspace.path(),
                model: Some("fake-model"),
                effort: None,
                system_prompt: None,
                resume_session: None,
                env_vars: &[],
                role: Some("researcher"),
                allowed_tools: Some("Read,Write,Edit"),
            },
            &spend,
        )
        .await
        .expect("run agent");

        assert_eq!(exit_code, 0);
        let rows = agent_cost_rows(workspace.path());
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0]["cost_usd"], CALL_COST_USD);
        assert_eq!(rows[0]["role"], "researcher");
        assert_eq!(rows[0]["plan_id"], "");
        assert_eq!(rows[0]["task_id"], task_id);
        let efficiency_log = workspace.path().join(".roko/learn/efficiency.jsonl");
        let efficiency = std::fs::read_to_string(efficiency_log).expect("efficiency log");
        assert!(
            efficiency.contains(&format!("\"attempt_id\":\"{task_id}/a1\"")),
            "{efficiency}"
        );
    }
}
