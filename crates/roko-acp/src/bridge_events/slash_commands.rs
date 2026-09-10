//! Slash command dispatch and shell command streaming for ACP sessions.

use std::{
    collections::{HashMap, VecDeque},
    path::Path,
    time::Duration,
};

use tokio::{
    io::{AsyncBufReadExt as _, AsyncRead},
    sync::mpsc,
};
use tracing::{info, warn};

use crate::session::CancelToken;
use crate::types::{ContentBlock, StopReason, ToolCallKind, ToolCallStatus};

use super::{CognitiveEvent, Result};
use super::provenance::{build_provenance, render_provenance_card};
use super::knowledge_helpers::{emit_knowledge_card, query_dispatch_knowledge};
use crate::runner::run_with_workflow_engine;

// ── Slash command dispatch ───────────────────────────────────────────

/// Runs a roko CLI slash command and streams the output as ACP updates.
pub(crate) async fn run_slash_command(
    session_id: &str,
    raw_input: &str,
    workdir: &Path,
    model_key: String,
    cancel_token: CancelToken,
    event_sender: mpsc::Sender<CognitiveEvent>,
    shared_run: crate::session::SharedWorkflowRun,
) -> Result<()> {
    let input = raw_input.trim_start_matches('/');
    let (command, args) = match input.split_once(char::is_whitespace) {
        Some((cmd, rest)) => (cmd.trim(), rest.trim()),
        None => (input.trim(), ""),
    };

    // Helper to send a usage hint and return early.
    macro_rules! require_args {
        ($cmd:expr, $hint:expr) => {
            if args.is_empty() {
                let _ = event_sender
                    .send(CognitiveEvent::TokenChunk(format!(
                        "Usage: /{} {}",
                        $cmd, $hint
                    )))
                    .await;
                let _ = event_sender
                    .send(CognitiveEvent::Complete {
                        stop_reason: StopReason::EndTurn,
                        usage: None,
                    })
                    .await;
                return Ok(());
            }
        };
    }

    // Map slash command names to roko CLI args.
    let cli_args: Vec<String> = match command {
        // ── Status & Diagnostics ──
        "status" => vec!["status".into()],
        "doctor" => vec!["doctor".into()],
        "config" => vec!["config".into(), "show".into()],
        "models" => vec!["config".into(), "models".into(), "list".into()],
        "learn" => vec!["learn".into(), "all".into()],

        // ── Research (foraging phase) ──
        "research" => {
            require_args!("research", "<topic>");
            vec![
                "research".into(),
                "topic".into(),
                "--model".into(),
                model_key.clone(),
                args.into(),
            ]
        }
        "search" => {
            require_args!("search", "<query>");
            vec!["research".into(), "search".into(), args.into()]
        }
        "enhance-prd" => {
            require_args!("enhance-prd", "<slug>");
            vec![
                "research".into(),
                "enhance-prd".into(),
                "--model".into(),
                model_key.clone(),
                args.into(),
            ]
        }

        // ── Specification (PRD lifecycle) ──
        "prd-idea" => {
            require_args!("prd-idea", "<idea text>");
            vec!["prd".into(), "idea".into(), args.into()]
        }
        "prd-draft" => {
            require_args!("prd-draft", "<slug>");
            vec![
                "prd".into(),
                "draft".into(),
                "new".into(),
                "--model".into(),
                model_key.clone(),
                args.into(),
            ]
        }
        "prd-list" => vec!["prd".into(), "list".into()],
        "prd-status" => vec!["prd".into(), "status".into()],
        "prd-plan" => {
            require_args!("prd-plan", "<slug>");
            vec![
                "prd".into(),
                "plan".into(),
                "--model".into(),
                model_key.clone(),
                args.into(),
            ]
        }
        "prd-consolidate" => vec!["prd".into(), "consolidate".into()],

        // ── Planning ──
        "plan-list" => vec!["plan".into(), "list".into()],
        "plan-generate" => {
            require_args!("plan-generate", "<description>");
            vec![
                "plan".into(),
                "generate".into(),
                "--model".into(),
                model_key.clone(),
                args.into(),
            ]
        }
        "plan-regenerate" => {
            require_args!("plan-regenerate", "<description>");
            vec![
                "plan".into(),
                "regenerate".into(),
                "--model".into(),
                model_key.clone(),
                args.into(),
            ]
        }
        "plan-validate" => {
            let dir = if args.is_empty() { "plans/" } else { args };
            vec!["plan".into(), "validate".into(), dir.into()]
        }
        "plan-run" => {
            let dir = if args.is_empty() { "plans/" } else { args };
            vec![
                "plan".into(),
                "run".into(),
                dir.into(),
                "--model".into(),
                model_key.clone(),
            ]
        }

        // ── Implementation & Execution ──
        "run" => {
            require_args!("run", "<prompt>");
            vec![
                "run".into(),
                "--model".into(),
                model_key.clone(),
                args.into(),
            ]
        }
        "do" => {
            require_args!("do", "<prompt>");
            vec![
                "do".into(),
                "--model".into(),
                model_key.clone(),
                args.into(),
            ]
        }
        "develop" => {
            require_args!("develop", "<prompt>");
            vec![
                "develop".into(),
                "--model".into(),
                model_key.clone(),
                "--yes".into(),
                args.into(),
            ]
        }
        "agents" => vec!["agent".into(), "list".into()],
        "agent-chat" => {
            require_args!("agent-chat", "<agent name>");
            vec![
                "agent".into(),
                "chat".into(),
                "--agent".into(),
                args.into(),
                "--model".into(),
                model_key.clone(),
            ]
        }

        // ── Verification & Gates ──
        "build" => {
            return run_shell_command(
                session_id,
                "cargo build --workspace",
                workdir,
                cancel_token,
                event_sender,
            )
            .await;
        }
        "test" => {
            return run_shell_command(
                session_id,
                "cargo test --workspace",
                workdir,
                cancel_token,
                event_sender,
            )
            .await;
        }
        "clippy" => {
            return run_shell_command(
                session_id,
                "cargo clippy --workspace --no-deps -- -D warnings",
                workdir,
                cancel_token,
                event_sender,
            )
            .await;
        }
        "fmt" => {
            return run_shell_command(
                session_id,
                "cargo +nightly fmt --all --check",
                workdir,
                cancel_token,
                event_sender,
            )
            .await;
        }
        "gate" => {
            // Run the full gate pipeline sequentially.
            return run_shell_command(
                session_id,
                "cargo +nightly fmt --all --check && cargo clippy --workspace --no-deps -- -D warnings && cargo test --workspace",
                workdir,
                cancel_token, event_sender,
            ).await;
        }

        // ── Knowledge & Dreams ──
        "knowledge" => {
            require_args!("knowledge", "<topic>");
            vec!["knowledge".into(), "query".into(), args.into()]
        }
        "knowledge-stats" => vec!["knowledge".into(), "stats".into()],
        "dream" => vec!["knowledge".into(), "dream".into(), "run".into()],

        // ── Code Intelligence ──
        "index" => {
            let sub = if args.is_empty() { "stats" } else { args };
            let parts: Vec<&str> = sub.splitn(2, char::is_whitespace).collect();
            let mut v = vec!["index".into(), parts[0].into()];
            if parts.len() > 1 {
                v.push(parts[1].into());
            }
            v
        }
        "explain" => {
            require_args!("explain", "<topic>");
            vec!["explain".into(), args.into()]
        }
        "replay" => {
            require_args!("replay", "<hash>");
            vec!["replay".into(), args.into()]
        }

        // ── Feedback & Learning ──
        "learn-router" => vec!["learn".into(), "router".into()],
        "learn-episodes" => vec!["learn".into(), "episodes".into()],
        "learn-tune" => {
            let target = if args.is_empty() { "gates" } else { args };
            vec!["learn".into(), "tune".into(), target.into()]
        }

        // ── New commands (plan-show, plan-resume, analyze, review, agent-start/stop, knowledge-gc/backup, audit) ──
        "plan-show" => {
            require_args!("plan-show", "<name>");
            vec!["plan".into(), "show".into(), args.into()]
        }
        "plan-resume" => {
            let path = if args.is_empty() {
                ".roko/state/executor.json"
            } else {
                args
            };
            vec![
                "plan".into(),
                "run".into(),
                "plans/".into(),
                "--resume-plan".into(),
                path.into(),
            ]
        }
        "analyze" => vec!["research".into(), "analyze".into()],
        "review" => {
            let target = if args.is_empty() { "HEAD~1" } else { args };
            return run_shell_command(
                session_id,
                &format!("git diff {target}"),
                workdir,
                cancel_token,
                event_sender,
            )
            .await;
        }
        "agent-start" => {
            require_args!("agent-start", "<name>");
            vec!["agent".into(), "start".into(), "--name".into(), args.into()]
        }
        "agent-stop" => {
            require_args!("agent-stop", "<name>");
            vec!["agent".into(), "stop".into(), "--name".into(), args.into()]
        }
        "note" => {
            require_args!("note", "<note text>");
            vec!["note".into(), args.into()]
        }
        "knowledge-gc" => vec!["knowledge".into(), "gc".into()],
        "knowledge-backup" => vec!["knowledge".into(), "backup".into()],
        "audit" => vec!["config".into(), "plugins".into(), "audit".into()],

        // ── Workflow ──
        "workflow" => {
            let sub = if args.is_empty() { "list" } else { args };
            match sub {
                "list" | "status" | "cancel" | "resume" => {
                    let msg = match sub {
                        "list" => "\
Workflow pipelines:
  none     — Single agent, no pipeline (current default)
  express  — Implement → gate → commit (fastest)
  standard — Implement → gate → review → commit
  full     — Strategy → implement → gate → multi-review → commit
  auto     — Select pipeline based on task complexity

Use the Workflow dropdown in the status bar to select, or:
  /express <prompt>      Run express pipeline
  /full <prompt>         Run full pipeline
  /review-this           Review current changes
  /pipeline <name>       Run a named pipeline"
                            .to_string(),
                        "status" => {
                            let guard = shared_run.lock().await;
                            match guard.as_ref() {
                                Some(run) => run.status_summary(),
                                None => "No active workflow run. Start one with /express, /full, or select a workflow in the config dropdown.".to_string(),
                            }
                        }
                        "cancel" => "No active workflow to cancel.".to_string(),
                        "resume" => "No halted workflow to resume.".to_string(),
                        _ => "Unknown workflow subcommand. Use: list, status, cancel, resume"
                            .to_string(),
                    };
                    let _ = event_sender.send(CognitiveEvent::TokenChunk(msg)).await;
                    let _ = event_sender
                        .send(CognitiveEvent::Complete {
                            stop_reason: StopReason::EndTurn,
                            usage: None,
                        })
                        .await;
                    return Ok(());
                }
                _ => {
                    let _ = event_sender
                        .send(CognitiveEvent::TokenChunk(format!(
                            "Unknown workflow subcommand: {sub}\n\nUse: /workflow list | status | cancel | resume"
                        )))
                        .await;
                    let _ = event_sender
                        .send(CognitiveEvent::Complete {
                            stop_reason: StopReason::EndTurn,
                            usage: None,
                        })
                        .await;
                    return Ok(());
                }
            }
        }
        "express" => {
            require_args!("express", "<prompt>");
            let knowledge = query_dispatch_knowledge(workdir, args).await;
            emit_knowledge_card(&knowledge, &event_sender).await;
            let provenance_card =
                build_provenance(&knowledge.hits, &knowledge.playbooks, args, workdir)
                    .await
                    .as_ref()
                    .map(render_provenance_card);
            let knowledge_context = knowledge.context_text();
            if std::env::var_os("ROKO_ACP_LEGACY").is_some() {
                return Ok(crate::runner::run_workflow_pipeline(
                    session_id,
                    args,
                    knowledge_context,
                    provenance_card,
                    workdir,
                    crate::runner::PipelineConfig {
                        template: crate::pipeline::WorkflowTemplate::Express,
                        max_iterations: 2,
                        clippy_enabled: true,
                        tests_enabled: true,
                        review_strictness: "standard".to_string(),
                        model_slug: model_key.clone(),
                        mcp_config: None,
                        sandbox_level: roko_core::config::schema::RunnerSandboxLevel::default(),
                    },
                    cancel_token,
                    event_sender,
                    shared_run,
                )
                .await?);
            }

            run_with_workflow_engine(
                session_id,
                args,
                workdir,
                "express",
                crate::runner::GraphEngineOptions {
                    model_key,
                    mcp_config: None,
                    provenance_card,
                    input_messages: Vec::new(),
                    route: crate::runner::AcpWorkflowRoute::LegacyDefault,
                },
                event_sender,
            )
            .await?;
            return Ok(());
        }
        "full" => {
            require_args!("full", "<prompt>");
            let knowledge = query_dispatch_knowledge(workdir, args).await;
            emit_knowledge_card(&knowledge, &event_sender).await;
            let provenance_card =
                build_provenance(&knowledge.hits, &knowledge.playbooks, args, workdir)
                    .await
                    .as_ref()
                    .map(render_provenance_card);
            let knowledge_context = knowledge.context_text();
            if std::env::var_os("ROKO_ACP_LEGACY").is_some() {
                return Ok(crate::runner::run_workflow_pipeline(
                    session_id,
                    args,
                    knowledge_context,
                    provenance_card,
                    workdir,
                    crate::runner::PipelineConfig {
                        template: crate::pipeline::WorkflowTemplate::Full,
                        max_iterations: 2,
                        clippy_enabled: true,
                        tests_enabled: true,
                        review_strictness: "standard".to_string(),
                        model_slug: model_key.clone(),
                        mcp_config: None,
                        sandbox_level: roko_core::config::schema::RunnerSandboxLevel::default(),
                    },
                    cancel_token,
                    event_sender,
                    shared_run,
                )
                .await?);
            }

            run_with_workflow_engine(
                session_id,
                args,
                workdir,
                "full",
                crate::runner::GraphEngineOptions {
                    model_key,
                    mcp_config: None,
                    provenance_card,
                    input_messages: Vec::new(),
                    route: crate::runner::AcpWorkflowRoute::LegacyDefault,
                },
                event_sender,
            )
            .await?;
            return Ok(());
        }
        "review-this" => {
            return run_shell_command(session_id, "git diff", workdir, cancel_token, event_sender)
                .await;
        }
        "pipeline" => {
            require_args!("pipeline", "<name>");
            let _ = event_sender
                .send(CognitiveEvent::TokenChunk(format!(
                    "[Pipeline: {args}] Not yet implemented. Available: express, standard, full\n\nUse /workflow list to see all pipelines."
                )))
                .await;
            let _ = event_sender
                .send(CognitiveEvent::Complete {
                    stop_reason: StopReason::EndTurn,
                    usage: None,
                })
                .await;
            return Ok(());
        }

        // ── Help ──
        "help" => {
            let help_text = "\
Available commands (organized by Will's core loop):

  Status & Diagnostics
    /status            Workspace status, signals, agents, runs
    /doctor            Diagnose workspace bootstrap state
    /config            Show roko.toml configuration
    /learn             Learning state overview

  Research (foraging)
    /research <topic>  Deep research with citations (Perplexity)
    /search <query>    Quick web search
    /enhance-prd <slug> Enrich a PRD with web research

  Specification (PRD lifecycle)
    /prd-idea <text>   Capture a work item idea
    /prd-draft <slug>  Draft a new PRD
    /prd-list          List all PRDs
    /prd-status        PRD pipeline coverage report
    /prd-plan <slug>   Generate plan from published PRD
    /prd-consolidate   Scan PRDs for gaps and duplicates

  Planning
    /plan-list         List all plans
    /plan-show <name>  Show a specific plan
    /plan-generate     Generate plan from a prompt
    /plan-validate     Lint tasks.toml without executing
    /plan-run [dir]    Execute a plan (orchestrate→gate→persist)
    /plan-resume [path] Resume an interrupted plan run

  Implementation & Execution
    /run <prompt>      Single prompt → universal loop
    /develop <prompt>  Full pipeline: scope → plan → execute → gate
    /agents            List agents and their status
    /agent-chat <name> Interactive chat with a specific agent
    /agent-start <name> Start a named agent
    /agent-stop <name>  Stop a running agent

  Verification & Gates
    /build             cargo build --workspace
    /test              cargo test --workspace
    /clippy            cargo clippy --workspace
    /fmt               cargo +nightly fmt --all --check
    /gate              Full pipeline: fmt + clippy + test
    /review [target]   git diff of target (default: HEAD~1)

  Research & Analysis
    /research <topic>  Deep research with citations (Perplexity)
    /search <query>    Quick web search
    /enhance-prd <slug> Enrich a PRD with web research
    /analyze           Analyze execution data

  Knowledge & Dreams
    /knowledge <topic> Query durable knowledge store
    /knowledge-stats   Knowledge store statistics
    /knowledge-gc      Garbage collect knowledge store
    /knowledge-backup  Backup knowledge store
    /dream             Dream consolidation (NREM→REM→integration)

  Code Intelligence
    /index [cmd]       Build/search/stats code index
    /explain <topic>   Explain a concept at 3 depth levels
    /replay <hash>     Walk signal DAG by hash

  Feedback & Learning
    /learn-router      Cascade router state and model routing
    /learn-episodes    Recent episode log
    /learn-tune [what] Tune adaptive thresholds

  Workflow Pipelines
    /workflow [sub]    list/status/cancel/resume workflows
    /express <prompt>  Express: implement → gate → commit
    /full <prompt>     Full: strategy → implement → gate → review → commit
    /review-this       Review current uncommitted changes
    /pipeline <name>   Run a named workflow pipeline

  System
    /audit             Plugin security audit

  /help               This message";
            let _ = event_sender
                .send(CognitiveEvent::TokenChunk(help_text.into()))
                .await;
            let _ = event_sender
                .send(CognitiveEvent::Complete {
                    stop_reason: StopReason::EndTurn,
                    usage: None,
                })
                .await;
            return Ok(());
        }

        _ => {
            let _ = event_sender
                .send(CognitiveEvent::TokenChunk(format!(
                    "Unknown command: /{command}\n\nType /help for available commands."
                )))
                .await;
            let _ = event_sender
                .send(CognitiveEvent::Complete {
                    stop_reason: StopReason::EndTurn,
                    usage: None,
                })
                .await;
            return Ok(());
        }
    };

    info!(session_id, command, ?cli_args, "executing slash command");

    // Find the roko binary.
    let roko_bin = std::env::current_exe().unwrap_or_else(|_| "roko".into());

    let mut child = match tokio::process::Command::new(&roko_bin)
        .args(&cli_args)
        .current_dir(workdir)
        .env("ROKO_ACP_PROGRESS", "1")
        .kill_on_drop(true)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let message = format!("Failed to run `roko {}`: {e}", cli_args.join(" "));
            let _ = event_sender
                .send(CognitiveEvent::Failure {
                    message: message.clone(),
                })
                .await;
            return Err(anyhow::anyhow!(message).into());
        }
    };

    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");
    let stream_outcome =
        forward_slash_command_streams(session_id, stdout, stderr, &cancel_token, &event_sender)
            .await;

    let SlashCommandStreamOutcome::Completed { had_output } = stream_outcome else {
        if let Err(error) =
            roko_agent::process::kill_tree(&mut child, Duration::from_millis(200)).await
        {
            warn!(session_id, %error, "failed to terminate slash command process tree");
        }
        return Ok(());
    };

    let exit_status = tokio::select! {
        _ = cancel_token.cancelled() => {
            if let Err(error) =
                roko_agent::process::kill_tree(&mut child, Duration::from_millis(200)).await
            {
                warn!(session_id, %error, "failed to terminate slash command process tree");
            }
            return Ok(());
        }
        status = child.wait() => status
    };
    let exit_status = exit_status.map_err(|error| {
        anyhow::anyhow!("failed waiting for `roko {}`: {error}", cli_args.join(" "))
    })?;
    if !exit_status.success() {
        let message = format!(
            "`roko {}` exited with status {}",
            cli_args.join(" "),
            exit_status
        );
        let _ = event_sender
            .send(CognitiveEvent::Failure {
                message: message.clone(),
            })
            .await;
        return Err(anyhow::anyhow!(message).into());
    }
    finish_slash_command_stream(command, had_output, &event_sender).await;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlashCommandStreamOutcome {
    Completed { had_output: bool },
    Cancelled,
}

pub(crate) async fn forward_slash_command_streams<Stdout, Stderr>(
    session_id: &str,
    stdout: Stdout,
    stderr: Stderr,
    cancel_token: &CancelToken,
    event_sender: &mpsc::Sender<CognitiveEvent>,
) -> SlashCommandStreamOutcome
where
    Stdout: AsyncRead + Unpin,
    Stderr: AsyncRead + Unpin,
{
    let mut stdout_lines = tokio::io::BufReader::new(stdout).lines();
    let mut stderr_lines = tokio::io::BufReader::new(stderr).lines();
    let mut stdout_done = false;
    let mut stderr_done = false;
    let mut had_output = false;
    let mut progress_task_counter: u64 = 0;
    let mut progress_calls: HashMap<String, VecDeque<String>> = HashMap::new();

    loop {
        if cancel_token.is_cancelled() {
            close_progress_calls(&mut progress_calls, "cancelled", event_sender).await;
            return SlashCommandStreamOutcome::Cancelled;
        }
        if stdout_done && stderr_done {
            break;
        }
        tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                close_progress_calls(
                    &mut progress_calls,
                    "cancelled",
                    event_sender,
                )
                .await;
                return SlashCommandStreamOutcome::Cancelled;
            }
            line = stdout_lines.next_line(), if !stdout_done => {
                match line {
                    Ok(Some(l)) => {
                        had_output = true;
                        if let Some(json_str) = l.strip_prefix("ROKO_PROGRESS: ") {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                                match value.get("type").and_then(|t| t.as_str()) {
                                    Some("task_started") => {
                                        progress_task_counter += 1;
                                        let title = value.get("title")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("task");
                                        let task_id = value.get("task_id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        let call_id = format!("progress-{}-{}", task_id, progress_task_counter);
                                        progress_calls
                                            .entry(task_id.to_owned())
                                            .or_default()
                                            .push_back(call_id.clone());
                                        let _ = event_sender.send(CognitiveEvent::ToolCallStart {
                                            tool_call_id: call_id,
                                            title: title.to_string(),
                                            kind: ToolCallKind::Terminal,
                                            locations: None,
                                        }).await;
                                    }
                                    Some("task_completed") => {
                                        let task_id = value.get("task_id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        let completed = value.get("completed")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(0);
                                        let total = value.get("total")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(0);
                                        let call_id = pop_progress_call(
                                            &mut progress_calls,
                                            task_id,
                                        )
                                            .unwrap_or_else(|| format!("progress-{}-unmatched", task_id));
                                        let _ = event_sender.send(CognitiveEvent::ToolCallComplete {
                                            tool_call_id: call_id,
                                            status: ToolCallStatus::Completed,
                                            content: vec![ContentBlock::Text {
                                                text: format!("{}/{} tasks done", completed, total),
                                            }],
                                        }).await;
                                    }
                                    Some("task_failed") => {
                                        let task_id = value.get("task_id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        let error = value.get("error")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("task failed");
                                        if let Some(call_id) = pop_progress_call(
                                            &mut progress_calls,
                                            task_id,
                                        ) {
                                            let _ = event_sender.send(CognitiveEvent::ToolCallComplete {
                                                tool_call_id: call_id,
                                                status: ToolCallStatus::Failed,
                                                content: vec![ContentBlock::Text {
                                                    text: error.to_owned(),
                                                }],
                                            }).await;
                                        } else {
                                            let _ = event_sender
                                                .send(CognitiveEvent::TokenChunk(format!("{l}\n")))
                                                .await;
                                        }
                                    }
                                    Some("agent_started") => {
                                        let provider = value.get("provider")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        let model = value.get("model")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        let _ = event_sender.send(CognitiveEvent::TokenChunk(
                                            format!("[agent] {} ({})\n", model, provider),
                                        )).await;
                                    }
                                    _ => {
                                        let _ = event_sender
                                            .send(CognitiveEvent::TokenChunk(format!("{l}\n")))
                                            .await;
                                    }
                                }
                            } else {
                                let _ = event_sender
                                    .send(CognitiveEvent::TokenChunk(format!("{l}\n")))
                                    .await;
                            }
                        } else {
                            let _ = event_sender
                                .send(CognitiveEvent::TokenChunk(format!("{l}\n")))
                                .await;
                        }
                    }
                    Ok(None) => stdout_done = true,
                    Err(e) => {
                        warn!(session_id, error = %e, "error reading slash command stdout");
                        stdout_done = true;
                    }
                }
            }
            line = stderr_lines.next_line(), if !stderr_done => {
                match line {
                    Ok(Some(l)) => {
                        had_output = true;
                        let _ = event_sender
                            .send(CognitiveEvent::TokenChunk(format!("\x1b[2m{l}\x1b[0m\n")))
                            .await;
                    }
                    Ok(None) => stderr_done = true,
                    Err(e) => {
                        warn!(session_id, error = %e, "error reading slash command stderr");
                        stderr_done = true;
                    }
                }
            }
        }
    }

    close_progress_calls(
        &mut progress_calls,
        "progress stream ended before task completion",
        event_sender,
    )
    .await;
    SlashCommandStreamOutcome::Completed { had_output }
}

pub(crate) fn pop_progress_call(
    progress_calls: &mut HashMap<String, VecDeque<String>>,
    task_id: &str,
) -> Option<String> {
    let call_id = progress_calls.get_mut(task_id)?.pop_front();
    if progress_calls.get(task_id).is_some_and(VecDeque::is_empty) {
        progress_calls.remove(task_id);
    }
    call_id
}

pub(crate) async fn close_progress_calls(
    progress_calls: &mut HashMap<String, VecDeque<String>>,
    reason: &str,
    event_sender: &mpsc::Sender<CognitiveEvent>,
) {
    let call_ids = progress_calls
        .drain()
        .flat_map(|(_, calls)| calls)
        .collect::<Vec<_>>();
    for tool_call_id in call_ids {
        let _ = event_sender
            .send(CognitiveEvent::ToolCallComplete {
                tool_call_id,
                status: ToolCallStatus::Failed,
                content: vec![ContentBlock::Text {
                    text: reason.to_owned(),
                }],
            })
            .await;
    }
}

pub(crate) async fn finish_slash_command_stream(
    command: &str,
    had_output: bool,
    event_sender: &mpsc::Sender<CognitiveEvent>,
) {
    if !had_output {
        let _ = event_sender
            .send(CognitiveEvent::TokenChunk(format!(
                "/{command} completed (no output)"
            )))
            .await;
    }
    let _ = event_sender
        .send(CognitiveEvent::Complete {
            stop_reason: StopReason::EndTurn,
            usage: None,
        })
        .await;
}

/// Runs a raw shell command (for /build, /test, /clippy) and streams each
/// stdout line as a TokenChunk immediately via tokio::select with cancel_token.
pub(crate) async fn run_shell_command(
    session_id: &str,
    shell_cmd: &str,
    workdir: &Path,
    cancel_token: CancelToken,
    event_sender: mpsc::Sender<CognitiveEvent>, // streams each line as TokenChunk
) -> Result<()> {
    info!(session_id, shell_cmd, "executing shell command");

    let mut child = match tokio::process::Command::new("sh")
        .args(["-c", shell_cmd])
        .current_dir(workdir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = event_sender
                .send(CognitiveEvent::TokenChunk(format!(
                    "Failed to run `{shell_cmd}`: {e}"
                )))
                .await;
            let _ = event_sender
                .send(CognitiveEvent::Complete {
                    stop_reason: StopReason::EndTurn,
                    usage: None,
                })
                .await;
            return Ok(());
        }
    };

    // Interleave stdout and stderr reading.
    let mut stdout_lines =
        tokio::io::BufReader::new(child.stdout.take().expect("stdout was piped")).lines();
    let mut stderr_lines =
        tokio::io::BufReader::new(child.stderr.take().expect("stderr was piped")).lines();
    let mut stdout_done = false;
    let mut stderr_done = false;
    let mut had_output = false;

    loop {
        if stdout_done && stderr_done {
            break;
        }
        tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                let _ = child.kill().await;
                return Ok(());
            }
            line = stdout_lines.next_line(), if !stdout_done => {
                match line {
                    Ok(Some(l)) => {
                        had_output = true;
                        let _ = event_sender
                            .send(CognitiveEvent::TokenChunk(format!("{l}\n")))
                            .await;
                    }
                    Ok(None) => stdout_done = true,
                    Err(e) => {
                        warn!(session_id, error = %e, "error reading shell command stdout");
                        stdout_done = true;
                    }
                }
            }
            line = stderr_lines.next_line(), if !stderr_done => {
                match line {
                    Ok(Some(l)) => {
                        had_output = true;
                        let _ = event_sender
                            .send(CognitiveEvent::TokenChunk(format!("\x1b[2m{l}\x1b[0m\n")))
                            .await;
                    }
                    Ok(None) => stderr_done = true,
                    Err(e) => {
                        warn!(session_id, error = %e, "error reading shell command stderr");
                        stderr_done = true;
                    }
                }
            }
        }
    }

    let exit_status = child.wait().await;
    let code = exit_status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
    if code != 0 {
        let _ = event_sender
            .send(CognitiveEvent::TokenChunk(format!(
                "\n\nProcess exited with code {code}"
            )))
            .await;
    }

    if !had_output {
        let _ = event_sender
            .send(CognitiveEvent::TokenChunk(format!(
                "`{shell_cmd}` completed (no output)"
            )))
            .await;
    }
    let _ = event_sender
        .send(CognitiveEvent::Complete {
            stop_reason: StopReason::EndTurn,
            usage: None,
        })
        .await;

    Ok(())
}

/// Maps a Claude tool name to an ACP tool call kind.
#[cfg(test)]
pub(crate) fn tool_name_to_kind(name: &str) -> ToolCallKind {
    match name {
        "Edit" | "MultiEdit" => ToolCallKind::Edit,
        "Write" => ToolCallKind::Create,
        "Bash" | "Terminal" => ToolCallKind::Terminal,
        _ => ToolCallKind::Other,
    }
}
