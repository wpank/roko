# AUDIT: Batch R5_E01 — Usage extraction unit test

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R5_E01`.
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
Usage extraction unit test

## Runner Context
You are working in runner `mega-parity`, batch R5_E01.
This batch is part of Runner 5: telemetry-learning — Make cost, usage, episodes, learning, and
cascade router feedback truthful enough that dashboards show real data and self-improvement
actually works.

## Problem

There are no unit tests verifying that the Claude CLI stream-json usage parsing correctly
handles all cases: result event WITH full usage, result event WITH partial usage (only some
fields), and result event WITHOUT a usage field at all.

## Architecture Contract

- `ClaudeResultEvent.usage` is `Option<ClaudeUsage>` — `None` when the `"usage"` key is absent
- `ClaudeUsage` fields all have `#[serde(default)]` so absent sub-fields default to 0, not None
- `parse_stream_line()` on a result event WITH usage emits `TurnCompleted` THEN `TokenUsage`
- `parse_stream_line()` on a result event WITHOUT usage emits only `TurnCompleted` (no `TokenUsage`)
- A zero-valued `TokenUsage` event must NEVER be emitted for an absent usage field (regression guard)

## Key Source File to Read

Read the entire file before writing any tests:

```bash
cat /Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/claude_cli/stream.rs
```

Key points from the file:

- `ClaudeUsage` struct is at lines 110–121. Fields: `input_tokens`, `output_tokens`,
  `cache_creation_input_tokens`, `cache_read_input_tokens`. All `#[serde(default)]`.
- `ClaudeResultEvent` struct is at lines 90–108. Field `usage: Option<ClaudeUsage>`.
- `parse_stream_line` is at lines 134–221.
- For `Result` branch (lines 203–218): emits `TurnCompleted`, then IF `res.usage` is `Some`,
  emits `TokenUsage` with `cache_read_tokens = usage.cache_read_input_tokens` and
  `cache_write_tokens = usage.cache_creation_input_tokens`.
- The existing `#[cfg(test)] mod tests` block starts at line 256 and ends at line 388.
- Existing tests (DO NOT duplicate):
  - `parse_system_event` (line 260)
  - `parse_assistant_text` (line 273)
  - `parse_assistant_tool_use` (line 285)
  - `parse_tool_event` (line 298)
  - `parse_result_event` (line 311) — basic result WITHOUT usage
  - `parse_empty_line` (line 331)
  - `parse_malformed_json` (line 337)
  - `tool_output_truncation` (line 342)
  - `assistant_message_emits_text_then_usage` (line 358)
  - `generic_error_line_maps_to_error_event` (line 378)

## Step 1: Add 5 tests to the existing `mod tests` block

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/claude_cli/stream.rs`

Insert BEFORE the closing `}` of the `mod tests` block (currently at line 388). The closing
`}` of `mod tests` is the second-to-last `}` in the file (line 388), followed by the final
`}` of the outer module scope. Insert before line 388.

Add these 5 tests verbatim:

```rust
    #[test]
    fn parse_result_event_with_full_usage() {
        let line = r#"{"type":"result","session_id":"sess-abc","total_cost_usd":0.0123,"num_turns":2,"is_error":false,"usage":{"input_tokens":850,"output_tokens":320,"cache_creation_input_tokens":100,"cache_read_input_tokens":200}}"#;
        let events = parse_stream_line(line);
        assert_eq!(events.len(), 2, "expected TurnCompleted + TokenUsage, got {events:?}");

        match &events[0] {
            AgentRuntimeEvent::TurnCompleted {
                session_id,
                total_cost_usd,
                num_turns,
                is_error,
            } => {
                assert_eq!(session_id.as_deref(), Some("sess-abc"));
                assert!(
                    (total_cost_usd.unwrap() - 0.0123).abs() < 1e-9,
                    "total_cost_usd mismatch: {total_cost_usd:?}"
                );
                assert_eq!(*num_turns, Some(2));
                assert!(!is_error);
            }
            other => panic!("events[0]: expected TurnCompleted, got {other:?}"),
        }

        match &events[1] {
            AgentRuntimeEvent::TokenUsage {
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_write_tokens,
            } => {
                assert_eq!(*input_tokens, 850);
                assert_eq!(*output_tokens, 320);
                // cache_read_input_tokens maps to cache_read_tokens
                assert_eq!(*cache_read_tokens, 200);
                // cache_creation_input_tokens maps to cache_write_tokens
                assert_eq!(*cache_write_tokens, 100);
            }
            other => panic!("events[1]: expected TokenUsage, got {other:?}"),
        }
    }

    #[test]
    fn parse_result_event_with_partial_usage_absent_cache_fields_default_to_zero() {
        // Only input_tokens and output_tokens present; cache fields absent
        let line = r#"{"type":"result","session_id":"sess-partial","total_cost_usd":0.005,"num_turns":1,"is_error":false,"usage":{"input_tokens":400,"output_tokens":150}}"#;
        let events = parse_stream_line(line);
        assert_eq!(events.len(), 2, "expected TurnCompleted + TokenUsage, got {events:?}");

        match &events[1] {
            AgentRuntimeEvent::TokenUsage {
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_write_tokens,
            } => {
                assert_eq!(*input_tokens, 400, "input_tokens must be populated");
                assert_eq!(*output_tokens, 150, "output_tokens must be populated");
                // #[serde(default)] on ClaudeUsage fields means absent cache fields = 0
                assert_eq!(*cache_read_tokens, 0, "absent cache field defaults to 0");
                assert_eq!(*cache_write_tokens, 0, "absent cache field defaults to 0");
            }
            other => panic!("events[1]: expected TokenUsage, got {other:?}"),
        }
    }

    #[test]
    fn parse_result_event_without_usage_emits_only_turn_completed() {
        // No "usage" key at all in the result event (already tested in parse_result_event
        // but that test does not assert len or absence of TokenUsage explicitly)
        let line = r#"{"type":"result","session_id":"sess-no-usage","total_cost_usd":0.002,"num_turns":1,"is_error":false}"#;
        let events = parse_stream_line(line);
        assert_eq!(
            events.len(), 1,
            "no usage field => only TurnCompleted, got {events:?}"
        );
        assert!(
            matches!(events[0], AgentRuntimeEvent::TurnCompleted { .. }),
            "expected TurnCompleted, got {:?}", events[0]
        );
    }

    #[test]
    fn absent_usage_in_result_must_not_emit_zero_token_usage() {
        // Regression guard: an absent usage field must NOT produce a TokenUsage event
        // with all-zero values. Zero tokens would be indistinguishable from a real
        // zero-token call and would corrupt learning data.
        let line = r#"{"type":"result","session_id":"sess-absent","is_error":false}"#;
        let events = parse_stream_line(line);

        let has_token_usage = events
            .iter()
            .any(|e| matches!(e, AgentRuntimeEvent::TokenUsage { .. }));
        assert!(
            !has_token_usage,
            "absent usage field must NOT produce a TokenUsage event; got {events:?}"
        );
    }

    #[test]
    fn stream_without_result_event_produces_no_turn_completed_and_no_token_usage() {
        // A stream that only has assistant messages but no result event.
        let lines = [
            r#"{"type":"assistant","subtype":"message","message":{"content":[{"type":"text","text":"Working..."}],"usage":null}}"#,
            r#"{"type":"assistant","subtype":"message","message":{"content":[{"type":"text","text":"Done."}],"usage":null}}"#,
        ];

        let all_events: Vec<AgentRuntimeEvent> = lines
            .iter()
            .flat_map(|line| parse_stream_line(line))
            .collect();

        let has_turn_completed = all_events
            .iter()
            .any(|e| matches!(e, AgentRuntimeEvent::TurnCompleted { .. }));
        assert!(
            !has_turn_completed,
            "no result event => no TurnCompleted; got {all_events:?}"
        );

        let has_token_usage = all_events
            .iter()
            .any(|e| matches!(e, AgentRuntimeEvent::TokenUsage { .. }));
        assert!(
            !has_token_usage,
            "no usage in assistant events (null) => no TokenUsage; got {all_events:?}"
        );
    }
```

## Step 2: Verify

```bash
cd /Users/will/dev/nunchi/roko/roko

# Run only the new tests (filter by test name prefix)
cargo test -p roko-agent -- parse_result_event_with 2>&1 | tail -20
cargo test -p roko-agent -- absent_usage_in_result 2>&1 | tail -20
cargo test -p roko-agent -- stream_without_result 2>&1 | tail -20

# Run all tests in the stream module to confirm no regressions
cargo test -p roko-agent -- provider::claude_cli::stream 2>&1 | tail -20

# Full roko-agent test suite
cargo test -p roko-agent 2>&1 | tail -5
```

All must pass. Zero failures.

## Write Scope (files you may modify)

- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/claude_cli/stream.rs`
  — add 5 tests inside the existing `#[cfg(test)] mod tests` block before its closing `}` at line 388

## Read-Only Context (do not modify)

- All non-test code in the same file
- `crates/roko-agent/src/runtime_events.rs` — `AgentRuntimeEvent` variants

## Acceptance Criteria

- [ ] Test `parse_result_event_with_full_usage` exists and passes
- [ ] Test `parse_result_event_with_partial_usage_absent_cache_fields_default_to_zero` exists and passes
- [ ] Test `parse_result_event_without_usage_emits_only_turn_completed` exists and passes
- [ ] Test `absent_usage_in_result_must_not_emit_zero_token_usage` exists and passes (regression guard)
- [ ] Test `stream_without_result_event_produces_no_turn_completed_and_no_token_usage` exists and passes
- [ ] `cargo test -p roko-agent -- provider::claude_cli::stream` shows 0 failures
- [ ] No changes to non-test code
- [ ] No real Claude CLI processes spawned

## Do NOT

- Change any non-test code
- Duplicate any of the 10 existing tests (lines 260–387)
- Add `[dev-dependencies]` — all types are already in scope via `use super::*`
- Write flaky tests (no sleep, no network, no filesystem I/O)
- Add `#[ignore]` to any test

---

## Current Implementation (as written by implementation agent)

### `crates/roko-agent/src/claude_cli_agent.rs` (1232 lines — truncated)

```rust
//! `ClaudeCliAgent` — choose this for the Claude CLI path with Roko's system
//! prompt, tool allowlist, safety settings, and session-aware behavior.
//!
//! This is the runtime-facing adapter for the `claude` executable. It keeps
//! the wire-specific flag construction in one place instead of scattering
//! command-building logic across the CLI entrypoints. Prefer
//! [`ExecAgent`](crate::ExecAgent) only for generic stdin/stdout CLIs where
//! Claude-specific resume and tool-loop wiring are not needed.

use crate::agent::{Agent, AgentResult};
use crate::mcp::find_mcp_config;
use crate::process::{
    GRACE_STDIN_CLOSE_MS, benign_stderr_warn_once, classify_benign_stderr, kill_tree,
    register_spawned_pid, set_process_group, unregister_pid,
};
use crate::usage::Usage;
use async_trait::async_trait;
use roko_core::{Body, Context, Engram, Kind, OperatingFrequency, Provenance};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::{Duration, timeout};

/// Build the Claude CLI `--settings` JSON payload with safety hooks.
///
/// The hooks block the destructive commands that should never be launched by
/// a model in this workspace: branch checkout/switch/rename, branch pushes,
/// and common filesystem-destruction shells.
#[must_use]
pub fn build_settings_json() -> String {
    serde_json::json!({
        "hooks": {
            "PreToolUse": [{
                "matcher": "Bash",
                "hooks": [
                    {
                        "type": "command",
                        "if": "Bash(git checkout *)",
                        "command": "echo 'BLOCKED: git checkout forbidden in plan worktrees' >&2 && exit 2"
                    },
                    {
                        "type": "command",
                        "if": "Bash(git switch *)",
                        "command": "echo 'BLOCKED: git switch forbidden in plan worktrees' >&2 && exit 2"
                    },
                    {
                        "type": "command",
                        "if": "Bash(git branch -m *)",
                        "command": "echo 'BLOCKED: branch rename forbidden in plan worktrees' >&2 && exit 2"
                    },
                    {
                        "type": "command",
                        "if": "Bash(git push *)",
                        "command": "echo 'BLOCKED: agents must not push — roko handles merges' >&2 && exit 2"
                    },
                    {
                        "type": "command",
                        "if": "Bash(rm -rf *)",
                        "command": "echo 'BLOCKED: destructive file deletion forbidden' >&2 && exit 2"
                    },
                    {
                        "type": "command",
                        "if": "Bash(rm -fr *)",
                        "command": "echo 'BLOCKED: destructive file deletion forbidden' >&2 && exit 2"
                    },
                    {
                        "type": "command",
                        "if": "Bash(rm -r *)",
                        "command": "echo 'BLOCKED: destructive file deletion forbidden' >&2 && exit 2"
                    }
                ]
            }]
        }
    })
    .to_string()
}

/// Agent wrapper around the `claude` CLI.
#[derive(Debug, Clone)]
pub struct ClaudeCliAgent {
    program: PathBuf,
    current_dir: PathBuf,
    model: String,
    effort: String,
    fallback_model: Option<String>,
    bare_mode: bool,
    system_prompt: Option<String>,
    allowed_tools: Option<String>,
    max_turns: Option<u32>,
    settings_json: String,
    extra_args: Vec<String>,
    env: Vec<(String, String)>,
    mcp_config: Option<PathBuf>,
    resume: Option<String>,
    dangerously_skip_permissions: bool,
    timeout_ms: u64,
    name: String,
}

impl ClaudeCliAgent {
    /// Construct a new Claude CLI agent rooted at `current_dir`.
    #[must_use]
    pub fn new(
        program: impl Into<PathBuf>,
        current_dir: impl Into<PathBuf>,
        model: impl Into<String>,
    ) -> Self {
        let model = model.into();
        Self {
            program: program.into(),
            current_dir: current_dir.into(),
            model: model.clone(),
            effort: "medium".to_string(),
            fallback_model: Some("claude-haiku-4-5".to_string()),
            bare_mode: true,
            system_prompt: None,
            allowed_tools: None,
            max_turns: Some(OperatingFrequency::Theta.turn_limit()),
            settings_json: build_settings_json(),
            extra_args: Vec::new(),
            env: Vec::new(),
            mcp_config: None,
            resume: None,
            dangerously_skip_permissions: true,
            timeout_ms: 120_000,
            name: format!("claude-cli:{model}"),
        }
    }

    /// Override the display name used in traces.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Override the per-request timeout in milliseconds.
    #[must_use]
    pub const fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Override the reasoning-effort label passed to Claude.
    #[must_use]
    pub fn with_effort(mut self, effort: impl Into<String>) -> Self {
        self.effort = effort.into();
        self
    }

    /// Override the fallback model passed to Claude.
    #[must_use]
    pub fn with_fallback_model(mut self, fallback_model: impl Into<String>) -> Self {
        self.fallback_model = Some(fallback_model.into());
        self
    }

    /// Disable `--bare` if the caller wants the full Claude Code shell.
    #[must_use]
    pub const fn with_bare_mode(mut self, bare_mode: bool) -> Self {
        self.bare_mode = bare_mode;
        self
    }

    /// Attach a system prompt generated by `SystemPromptBuilder`.
    #[must_use]
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Attach a Claude tool allowlist, formatted as `Read,Edit,Bash`.
    #[must_use]
    pub fn with_tools(mut self, tools: impl Into<String>) -> Self {
        self.allowed_tools = Some(tools.into());
        self
    }

    /// Attach a Claude `--allowedTools` allowlist.
    #[must_use]
    pub fn with_allowed_tools(mut self, tools: impl Into<String>) -> Self {
        self.allowed_tools = Some(tools.into());
        self
    }

    /// Set the maximum number of turns Claude may take.
    #[must_use]
    pub const fn with_max_turns(mut self, max_turns: u32) -> Self {
        self.max_turns = Some(max_turns);
        self
    }

    /// Override the settings JSON passed via `--settings`.
    #[must_use]
    pub fn with_settings_json(mut self, json: impl Into<String>) -> Self {
        self.settings_json = json.into();
// ... (832 lines omitted) ...
            .with_resume("session-123")
            .with_bare_mode(true);

        let result = agent.run(&prompt("hi there"), &Context::now()).await;
        assert!(result.success);
        assert_eq!(result.output.body.as_text().unwrap().trim(), "hello");

        let args_text = fs::read_to_string(&capture_args).unwrap();
        assert!(args_text.contains("--print"));
        assert!(args_text.contains("--verbose"));
        assert!(args_text.contains("--output-format"));
        assert!(args_text.contains("stream-json"));
        assert!(args_text.contains("--model"));
        assert!(args_text.contains("claude-test-model"));
        assert!(args_text.contains("--effort"));
        assert!(args_text.contains("medium"));
        assert!(args_text.contains("--max-turns"));
        assert!(args_text.contains("20"));
        assert!(args_text.contains("--append-system-prompt"));
        assert!(args_text.contains("system guidance"));
        assert!(args_text.contains("--settings"));
        assert!(args_text.contains("--dangerously-skip-permissions"));
        assert!(args_text.contains("--tools"));
        assert!(args_text.contains("Read,Edit"));
        assert!(args_text.contains("--resume"));
        assert!(args_text.contains("session-123"));

        let prompt_text = fs::read_to_string(&capture_prompt).unwrap();
        assert_eq!(prompt_text, "hi there");
    }

    #[tokio::test]
    async fn can_disable_dangerous_skip_permissions_flag() {
        let tmp = tempdir().unwrap();
        let capture_args = tmp.path().join("args.txt");
        let script = tmp.path().join("claude-fake.sh");
        let script_body = format!(
            r#"#!/bin/sh
set -eu
args_file="{args_file}"
printf '%s\n' "$@" > "$args_file"
cat >/dev/null
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"ok"}}}}'
"#,
            args_file = capture_args.display(),
        );
        fs::write(&script, script_body).unwrap();
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }

        let agent = ClaudeCliAgent::new(&script, tmp.path(), "claude-test-model")
            .with_dangerously_skip_permissions(false);
        let result = agent.run(&prompt("x"), &Context::now()).await;
        assert!(
            result.success,
            "{}",
            result.output.body.as_text().unwrap_or("unknown")
        );

        let args_text = fs::read_to_string(&capture_args).unwrap();
        assert!(!args_text.contains("--dangerously-skip-permissions"));
    }

    #[tokio::test]
    async fn optional_resume_none_omits_resume_flag() {
        let tmp = tempdir().unwrap();
        let capture_args = tmp.path().join("args.txt");
        let script = tmp.path().join("claude-fake.sh");
        let script_body = format!(
            r#"#!/bin/sh
set -eu
args_file="{args_file}"
printf '%s\n' "$@" > "$args_file"
cat >/dev/null
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"ok"}}}}'
"#,
            args_file = capture_args.display(),
        );
        fs::write(&script, script_body).unwrap();
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }

        let agent = ClaudeCliAgent::new(&script, tmp.path(), "claude-test-model")
            .with_optional_resume(None);
        let result = agent.run(&prompt("x"), &Context::now()).await;
        assert!(
            result.success,
            "{}",
            result.output.body.as_text().unwrap_or("unknown")
        );

        let args_text = fs::read_to_string(&capture_args).unwrap();
        assert!(!args_text.contains("--resume"));
    }

    #[tokio::test]
    async fn result_event_usage_is_threaded_into_agent_result() {
        let tmp = tempdir().unwrap();
        let script = tmp.path().join("claude-fake.sh");
        let script_body = r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"hello"}}'
printf '%s\n' '{"type":"result","session_id":"sess-1","model":"claude-sonnet-4-6","total_cost_usd":0.25,"usage":{"input_tokens":11,"output_tokens":22,"cache_creation_input_tokens":33,"cache_read_input_tokens":44}}'
"#;
        fs::write(&script, script_body).unwrap();
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }

        let agent = ClaudeCliAgent::new(&script, tmp.path(), "claude-test-model");
        let result = agent.run(&prompt("x"), &Context::now()).await;
        assert!(result.success);
        assert_eq!(result.output.body.as_text().unwrap().trim(), "hello");
        assert_eq!(result.output.tag("model"), Some("claude-sonnet-4-6"));
        assert_eq!(result.usage.input_tokens, 11);
        assert_eq!(result.usage.output_tokens, 22);
        assert_eq!(result.usage.cache_read_tokens, 44);
        assert_eq!(result.usage.cache_create_tokens, 33);
        assert!((result.usage.cost_usd - 0.25).abs() < 0.0001);
    }

    #[tokio::test]
    async fn nonzero_exit_still_carries_result_event_usage() {
        let tmp = tempdir().unwrap();
        let script = tmp.path().join("claude-fake.sh");
        let script_body = r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"result","session_id":"sess-2","model":"claude-sonnet-4-6","total_cost_usd":0.5,"usage":{"input_tokens":9,"output_tokens":8,"cache_creation_input_tokens":7,"cache_read_input_tokens":6}}'
exit 1
"#;
        fs::write(&script, script_body).unwrap();
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }

        let agent = ClaudeCliAgent::new(&script, tmp.path(), "claude-test-model");
        let result = agent.run(&prompt("x"), &Context::now()).await;
        assert!(!result.success);
        assert_eq!(result.output.tag("model"), Some("claude-sonnet-4-6"));
        assert_eq!(result.usage.input_tokens, 9);
        assert_eq!(result.usage.output_tokens, 8);
        assert_eq!(result.usage.cache_read_tokens, 6);
        assert_eq!(result.usage.cache_create_tokens, 7);
        assert!((result.usage.cost_usd - 0.5).abs() < 0.0001);
    }

    #[tokio::test]
    async fn benign_stderr_is_filtered_from_trace() {
        let tmp = tempdir().unwrap();
        let script = tmp.path().join("claude-fake.sh");
        let script_body = r#"#!/bin/sh
set -eu
cat >/dev/null
echo 'Claude CLI is starting up...' 1>&2
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"ok"}}'
"#;
        fs::write(&script, script_body).unwrap();
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }

        let agent = ClaudeCliAgent::new(&script, tmp.path(), "claude-test-model");
        let result = agent.run(&prompt("x"), &Context::now()).await;
        assert!(
            result.success,
            "{}",
            result.output.body.as_text().unwrap_or("unknown")
        );
        assert!(result.trace.is_empty());
    }

    #[test]
    fn stderr_trace_skips_stream_json_lines() {
        let agent = ClaudeCliAgent::new("claude", ".", "claude-test-model");
        let trace = agent.stderr_trace(
            "unexpected stderr line\n{\"type\":\"content_block_delta\",\"delta\":{\"text\":\"ok\"}}\n",
        );
        assert_eq!(trace.len(), 1);
        assert_eq!(trace[0].body.as_text().unwrap(), "unexpected stderr line");
    }
}
```

---

## Verification Commands

Run these and fix any failures:
```bash
cargo check -p roko-agent
cargo clippy -p roko-agent --no-deps -- -D warnings
```

## Do NOT

- Rewrite the entire implementation from scratch
- Add features not in the original prompt
- Modify files outside the write scope
- Skip running verification commands
