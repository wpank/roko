# AUDIT: Batch R7_F01 — ACP conversation history accumulation + context injection

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R7_F01`.
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
ACP conversation history accumulation + context injection

## Runner Context
You are working in runner `mega-parity`, batch R7_F01.
This batch is part of Runner 7: mori-polish — Polish interactive experience to match Mori behavior.

**No dependencies** — can run immediately in parallel.

## Prerequisite Check
`crates/roko-acp/` exists. Proceed.

## What Already Exists (confirmed from source)

### `crates/roko-acp/src/session.rs` — History already fully implemented

Lines 100-103:
```rust
const MAX_HISTORY_TURNS: usize = 40;
const MAX_HISTORY_CHARS: usize = 64_000;
```

Lines 137-155: `ConversationTurn` and `TurnRole` types exist.

Lines 237-261: `AcpSession` already has `conversation_history: Vec<ConversationTurn>`.

Lines 389-425: `push_user_turn()`, `push_assistant_turn()`, and `trim_history()` already implemented:
```rust
pub fn push_user_turn(&mut self, content: String) {
    self.conversation_history.push(ConversationTurn { role: TurnRole::User, content });
    self.trim_history();
}
pub fn push_assistant_turn(&mut self, content: String) {
    self.conversation_history.push(ConversationTurn { role: TurnRole::Assistant, content });
    self.trim_history();
}
fn trim_history(&mut self) {
    while self.conversation_history.len() > MAX_HISTORY_TURNS {
        self.conversation_history.remove(0);
    }
    loop {
        let total_chars: usize = self.conversation_history.iter().map(|t| t.content.len()).sum();
        if total_chars <= MAX_HISTORY_CHARS || self.conversation_history.is_empty() { break; }
        self.conversation_history.remove(0);
    }
}
```

Lines 456-472: `build_history_context_for_cli()` already implemented, returns `<conversation_history>...</conversation_history>` XML block.

Lines 427-453: `build_messages_array()` already implemented.

### `crates/roko-acp/src/bridge_events.rs` — Already PARTIALLY wired

Lines 393-415 (ALREADY DONE — history is already wired):
```rust
// Get system prompt and history context (skip for slash commands).
let system_prompt = session.system_prompt_for_mode().to_owned();
let history_context = if is_slash_command {
    String::new()
} else {
    session.build_history_context_for_cli()
};
let messages = if is_slash_command {
    Vec::new()
} else {
    let full_system = if file_context.is_empty() {
        system_prompt.clone()
    } else {
        format!("{system_prompt}\n\n{file_context}")
    };
    session.build_messages_array(&full_system, &prompt_text)
};

// Push user turn before dispatch (skip slash commands).
if !is_slash_command {
    session.push_user_turn(prompt_text.clone());
}
```

Lines 560-566 (ALREADY DONE — push_assistant_turn is already wired):
```rust
// Push assistant turn after streaming completes (skip slash commands).
match &stream_result {
    Ok(sr) if !is_slash_command && !sr.assistant_text.is_empty() => {
        session.push_assistant_turn(sr.assistant_text.clone());
    }
    _ => {}
}
```

Lines 494-505 (ClaudeCLI path ALREADY uses history_context):
```rust
ProviderKind::ClaudeCli => {
    let mut full_prompt = String::new();
    if !file_context.is_empty() {
        full_prompt.push_str(&file_context);
        full_prompt.push('\n');
    }
    if !history_context.is_empty() {
        full_prompt.push_str(&history_context);
    }
    full_prompt.push_str(&prompt_text);
    run_claude_cognitive_task(...full_prompt...).await
}
```

Lines 381-391 (file context from `include_context` boolean):
```rust
let file_context = if params.include_context {
    let uris = extract_resource_uris(&params.prompt);
    if uris.is_empty() { String::new() } else { read_file_context(&uris, workdir) }
} else {
    String::new()
};
```

Lines 1552-1612: `extract_resource_uris()` and `read_file_context()` already implemented.

## Problem Statement (revised based on actual code)

**The history and file context wiring is ALREADY complete for the single-agent (non-pipeline) path.** The prior prompt description was based on an older state of the code.

However, two genuine gaps remain:

### Gap 1: `include_context` is a `bool` not a `Vec<ContentBlock>`

Looking at `types.rs` line 328-329:
```rust
pub struct SessionPromptParams {
    pub session_id: String,
    pub prompt: Vec<ContentBlock>,
    #[serde(default)]
    pub include_context: bool,   // ← bool, not a resource list
}
```

The `include_context: bool` flag means "include server-side file context from Resource blocks already in `prompt`". This IS wired correctly at lines 381-391. The original prompt described it as a `Vec<ContentBlock>` which does not match reality — `include_context` is a boolean toggle.

### Gap 2: `push_assistant_turn` receives the full `assistant_text` without truncation

The `stream_result.assistant_text` can be arbitrarily large (a 100KB response would all go into history). The existing `MAX_HISTORY_CHARS=64_000` limit in `trim_history()` does truncate the history buffer as a whole — but a single turn that is 65KB will be stored as-is and immediately evict everything else.

### Gap 3: History is NOT persisted via `persist_session()`

After `push_assistant_turn()` at line 563, `persist_session()` is never called. The session is only saved in memory. If the ACP server restarts, history is lost.

## Changes Required

### Change 1: Truncate large assistant responses before storing in history

**File:** `crates/roko-acp/src/bridge_events.rs`

Find lines 560-566:
```rust
// Push assistant turn after streaming completes (skip slash commands).
match &stream_result {
    Ok(sr) if !is_slash_command && !sr.assistant_text.is_empty() => {
        session.push_assistant_turn(sr.assistant_text.clone());
    }
    _ => {}
}
```

Replace with:
```rust
// Push assistant turn after streaming completes (skip slash commands).
// Truncate to 10KB before storing to prevent history bloat from large responses.
match &stream_result {
    Ok(sr) if !is_slash_command && !sr.assistant_text.is_empty() => {
        let response_for_history = if sr.assistant_text.len() > 10_240 {
            format!("{}...[truncated]", &sr.assistant_text[..10_240])
        } else {
            sr.assistant_text.clone()
        };
        session.push_assistant_turn(response_for_history);
    }
    _ => {}
}
```

### Change 2: Persist session after each turn to preserve history across restarts

**File:** `crates/roko-acp/src/bridge_events.rs`

The `handle_session_prompt_inner` function receives `session: &mut AcpSession` and `workdir: &Path` but NOT the `SessionManager`. Session persistence requires the `SessionManager`. This means persistence can only happen at the call site in `handle_session_prompt`.

Look at how `handle_session_prompt` is called — find it in `crates/roko-acp/src/bridge_events.rs` around line 333-353:
```rust
pub async fn handle_session_prompt<R, W>(
    transport: &mut StdioTransport<R, W>,
    session: &mut AcpSession,
    params: SessionPromptParams,
    workdir: &Path,
    roko_config: &RokoConfig,
) -> Result<SessionPromptResult>
```

This function does NOT have access to `SessionManager`. The outer handler in the main ACP loop (in `crates/roko-acp/src/handler.rs` or wherever the stdio loop is) calls `handle_session_prompt` — THAT is where `persist_session()` should be called after the response.

**Step 2a:** Find the ACP handler that calls `handle_session_prompt`. Search:
```bash
grep -rn 'handle_session_prompt' crates/roko-acp/src/ --include='*.rs'
```

**Step 2b:** After that call returns successfully, call `sessions.persist_session(&session_id)`.

The `SessionManager::persist_session()` method (line 631-651 of `session.rs`) is synchronous and writes `.roko/sessions/{session_id}.json`. It already serializes `conversation_history` because `AcpSession` derives `Serialize` and `conversation_history` has no `#[serde(skip)]`.

**Step 2c:** Verify the JSON roundtrip works:
```bash
# After implementing, start ACP, send 2 prompts, restart, load session — history should persist
cargo test -p roko-acp 2>&1 | grep -E 'PASS|FAIL|error'
```

## Write Scope
- `crates/roko-acp/src/bridge_events.rs` — Truncate large responses before `push_assistant_turn`; find call site of `handle_session_prompt` in handler loop and add `persist_session` call there
- `crates/roko-acp/src/handler.rs` (or wherever the main ACP dispatch loop is) — Add `persist_session` call after successful `handle_session_prompt`

## Read-Only Context
- `crates/roko-acp/src/session.rs` — Already fully implemented, do not change
- `crates/roko-acp/src/types.rs` — `include_context` is `bool`, not `Vec<ContentBlock>`

## Acceptance Criteria
- [ ] `push_assistant_turn()` called with truncated response (≤10KB + "[truncated]" suffix)
- [ ] `push_user_turn()` called before dispatch (already done — verify it's there)
- [ ] `build_history_context_for_cli()` used in ClaudeCLI dispatch (already done — verify)
- [ ] `build_messages_array()` used in OpenAI-compat dispatch (already done — verify)
- [ ] `persist_session()` called after each successful turn in the outer handler loop
- [ ] History persists across ACP server restart (load from `.roko/sessions/{id}.json`)
- [ ] Large responses (>10KB) truncated before storing in history
- [ ] Slash commands do NOT add turns to history (already correct — verify)
- [ ] Empty history (first prompt) works exactly as before (no regression)
- [ ] `cargo check -p roko-acp && cargo clippy -p roko-acp --no-deps -- -D warnings`

## Verification
```bash
cargo check -p roko-acp && cargo clippy -p roko-acp --no-deps -- -D warnings
cargo test -p roko-acp

# Verify truncation is wired:
grep -n 'truncated\]\|10_240' crates/roko-acp/src/bridge_events.rs

# Verify push_user_turn and push_assistant_turn are called:
grep -n 'push_user_turn\|push_assistant_turn' crates/roko-acp/src/bridge_events.rs

# Verify persist_session is called somewhere in the ACP handler:
grep -rn 'persist_session' crates/roko-acp/src/
```

## Do NOT
- Reimplement `push_user_turn`, `push_assistant_turn`, `trim_history`, `build_history_context_for_cli`, or `build_messages_array` — they already exist and work
- Change `MAX_HISTORY_TURNS` (40) or `MAX_HISTORY_CHARS` (64_000) — they're already correct
- Add history to pipeline phase prompts — only the user-facing single-agent path uses history
- Store history globally — it's per-session (ACP-5)
- Change `include_context: bool` to a `Vec` — the current bool semantics are correct

---

## Current Implementation (as written by implementation agent)

### `crates/roko-acp/src/session.rs` (1558 lines — truncated)

```rust
//! ACP session state management.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{Mutex, Notify};
use uuid::Uuid;

use crate::types::{
    ClientCapabilities, CommandInput, ConfigOption, ConfigOptionType, ConfigOptionValue,
    McpServerConfig, ModeInfo, ModesInfo, SESSION_NOT_FOUND, SessionInfo, SessionListResult,
    SessionNewParams, SessionNewResult, SlashCommand,
};
use crate::workflow::WorkflowRun;

/// Shared handle to the active workflow run, updated by the runner in real time.
pub type SharedWorkflowRun = Arc<Mutex<Option<WorkflowRun>>>;

fn new_shared_run() -> SharedWorkflowRun {
    Arc::new(Mutex::new(None))
}

fn new_atomic_flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

fn new_notify() -> Arc<Notify> {
    Arc::new(Notify::new())
}

/// A lightweight cooperative cancellation token for ACP session work.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelToken {
    #[serde(skip, default = "new_atomic_flag")]
    cancelled: Arc<AtomicBool>,
    #[serde(skip, default = "new_notify")]
    notify: Arc<Notify>,
}

impl CancelToken {
    /// Creates a new uncancelled token.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cancelled: new_atomic_flag(),
            notify: new_notify(),
        }
    }

    /// Marks the token as cancelled and wakes any waiters.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.notify.notify_waiters();
    }

    /// Returns whether the token has been cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    /// Waits until the token is cancelled.
    pub async fn cancelled(&self) {
        if self.is_cancelled() {
            return;
        }

        loop {
            let notified = self.notify.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
            if self.is_cancelled() {
                return;
            }
        }
    }
}

impl Default for CancelToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Default model key when roko.toml has no models configured.
pub const FALLBACK_MODEL: &str = "sonnet";

/// Maximum number of conversation turns to retain.
const MAX_HISTORY_TURNS: usize = 40;
/// Maximum total characters across all history turns.
const MAX_HISTORY_CHARS: usize = 64_000;

// ── Mode-specific system prompts ─────────────────────────────────────

const CODE_MODE_SYSTEM_PROMPT: &str = "\
You are an expert code implementer. Your role is to write and edit code directly.

Rules:
- Make minimal, targeted changes. Don't refactor unrelated code.
- Read existing code before modifying it. Understand context first.
- Follow existing patterns and conventions in the codebase.
- Write correct, working code. Verify your changes compile.
- Be concise in explanations. Lead with the code change.";

const PLAN_MODE_SYSTEM_PROMPT: &str = "\
You are a software architect and strategist. Your role is to plan, not implement.

Rules:
- Decompose tasks into clear, actionable steps.
- Identify files that need changes and describe what changes are needed.
- Consider edge cases, dependencies, and ordering constraints.
- Do NOT write implementation code directly. Describe what to build.
- Output structured plans with numbered steps.";

const RESEARCH_MODE_SYSTEM_PROMPT: &str = "\
You are a technical researcher. Your role is to gather context and analyze options.

Rules:
- Search broadly before concluding. Check multiple sources of truth.
- Cite specific files, functions, and line numbers when referencing code.
- Compare alternatives with tradeoffs when multiple approaches exist.
- Summarize findings clearly with actionable recommendations.
- Do NOT make changes. Report what you find.";

/// A single turn in the conversation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationTurn {
    /// The role of this turn (user or assistant).
    pub role: TurnRole,
    /// The text content of this turn.
    pub content: String,
}

/// Role identifier for conversation turns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnRole {
    /// A user message.
    User,
    /// An assistant response.
    Assistant,
}

/// Session-scoped ACP configuration state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionConfigState {
    /// Active agent interaction mode.
    pub agent_mode: String,
    /// Selected model key (maps to `[models.*]` in roko.toml).
    pub model: String,
    /// Effort level: low, medium, high, max.
    pub effort: String,
    /// Temperament: cautious, balanced, aggressive.
    pub temperament: String,
    /// Routing mode: auto_override, manual, cascade.
    pub routing_mode: String,
    /// Whether clippy gate is enabled.
    pub clippy_enabled: bool,
    /// Whether test gate is enabled.
    pub tests_enabled: bool,
    /// Workflow pipeline: none, express, standard, full, auto.
    pub workflow: String,
    /// Review strictness: none, quick, standard, thorough.
    pub review_strictness: String,
    /// Maximum pipeline retry iterations (1-3).
    pub max_iterations: u32,
}

impl Default for SessionConfigState {
    fn default() -> Self {
        Self {
            agent_mode: "code".to_owned(),
            model: FALLBACK_MODEL.to_owned(),
            effort: "medium".to_owned(),
            temperament: "balanced".to_owned(),
            routing_mode: "auto_override".to_owned(),
            clippy_enabled: true,
            tests_enabled: true,
            workflow: "none".to_owned(),
            review_strictness: "none".to_owned(),
            max_iterations: 2,
        }
    }
}

impl SessionConfigState {
// ... (1158 lines omitted) ...
    }

    #[test]
    fn list_sessions_returns_expected_count() {
        let mut manager = SessionManager::new(PathBuf::from("."), Default::default());
        manager.create_session(session_params("alpha"));
        manager.create_session(session_params("beta"));

        let sessions = manager.list_sessions();
        assert_eq!(sessions.sessions.len(), 2);
    }

    #[test]
    fn missing_session_lookup_returns_none() {
        let manager = SessionManager::new(PathBuf::from("."), Default::default());
        assert!(manager.get_session("sess_missing").is_none());
    }

    #[tokio::test]
    async fn cancel_token_wakes_waiters() {
        let token = CancelToken::new();
        let waiter = token.clone();

        let handle = tokio::spawn(async move {
            waiter.cancelled().await;
            waiter.is_cancelled()
        });

        tokio::task::yield_now().await;
        token.cancel();

        assert!(handle.await.expect("waiter should complete"));
    }

    #[test]
    fn conversation_history_push_and_trim() {
        let mut session = AcpSession::new(session_params("history"));
        session.push_user_turn("hello".into());
        session.push_assistant_turn("hi there".into());
        assert_eq!(session.conversation_history.len(), 2);
        assert_eq!(session.conversation_history[0].role, TurnRole::User);
        assert_eq!(session.conversation_history[1].role, TurnRole::Assistant);
    }

    #[test]
    fn conversation_history_trims_by_count() {
        let mut session = AcpSession::new(session_params("trim"));
        for i in 0..50 {
            session.push_user_turn(format!("msg {i}"));
        }
        assert!(session.conversation_history.len() <= MAX_HISTORY_TURNS);
    }

    #[test]
    fn conversation_history_trims_by_chars() {
        let mut session = AcpSession::new(session_params("trim-chars"));
        // Push a few large messages.
        for _ in 0..5 {
            session.push_user_turn("x".repeat(20_000));
        }
        let total: usize = session
            .conversation_history
            .iter()
            .map(|t| t.content.len())
            .sum();
        assert!(total <= MAX_HISTORY_CHARS);
    }

    #[test]
    fn mode_change_clears_history() {
        let mut session = AcpSession::new(session_params("mode"));
        session.push_user_turn("hello".into());
        session.push_assistant_turn("hi".into());
        assert_eq!(session.conversation_history.len(), 2);

        session.set_mode("plan".into());
        assert!(session.conversation_history.is_empty());
    }

    #[test]
    fn system_prompt_for_mode_returns_correct_prompts() {
        let mut session = AcpSession::new(session_params("prompts"));
        assert!(
            session
                .system_prompt_for_mode()
                .contains("code implementer")
        );

        session.set_mode("plan".into());
        assert!(session.system_prompt_for_mode().contains("architect"));

        session.set_mode("research".into());
        assert!(session.system_prompt_for_mode().contains("researcher"));
    }

    #[test]
    fn build_messages_array_includes_history() {
        let mut session = AcpSession::new(session_params("messages"));
        session.push_user_turn("first".into());
        session.push_assistant_turn("response".into());

        let messages = session.build_messages_array("system text", "current prompt");
        // system + 2 history turns + current user
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "first");
        assert_eq!(messages[2]["role"], "assistant");
        assert_eq!(messages[3]["role"], "user");
        assert_eq!(messages[3]["content"], "current prompt");
    }

    #[test]
    fn build_history_context_for_cli_formats_xml() {
        let mut session = AcpSession::new(session_params("cli-ctx"));
        session.push_user_turn("hello".into());
        session.push_assistant_turn("world".into());

        let ctx = session.build_history_context_for_cli();
        assert!(ctx.contains("<conversation_history>"));
        assert!(ctx.contains("<user>\nhello\n</user>"));
        assert!(ctx.contains("<assistant>\nworld\n</assistant>"));
    }

    #[test]
    fn empty_history_produces_empty_cli_context() {
        let session = AcpSession::new(session_params("empty"));
        assert!(session.build_history_context_for_cli().is_empty());
    }

    #[test]
    fn persist_and_load_session_round_trips() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let workdir = tmp.path().to_path_buf();

        let mut manager = SessionManager::new(workdir, Default::default());
        let result = manager.create_session(session_params("persist-test"));
        let session_id = result.session_id.clone();

        // Add some history.
        {
            let session = manager.get_session_mut(&session_id).unwrap();
            session.push_user_turn("hello".into());
            session.push_assistant_turn("hi there".into());
        }

        manager.persist_session(&session_id);

        // Create a fresh manager to verify disk loading.
        let mut manager2 = SessionManager::new(tmp.path().to_path_buf(), Default::default());
        let loaded = manager2.load_session(&session_id);
        assert!(loaded.is_ok(), "should load from disk");

        // Verify the session is now in memory with history.
        let loaded_session = manager2.get_session(&session_id).unwrap();
        assert_eq!(loaded_session.conversation_history.len(), 2);
    }

    #[test]
    fn list_sessions_includes_persisted() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let workdir = tmp.path().to_path_buf();

        let mut manager = SessionManager::new(workdir.clone(), Default::default());
        let result = manager.create_session(session_params("in-memory"));

        // Persist it, then create a new manager without that session.
        manager.persist_session(&result.session_id);
        let manager2 = SessionManager::new(workdir, Default::default());
        let list = manager2.list_sessions_with_persisted();
        assert!(
            list.sessions
                .iter()
                .any(|s| s.session_id == result.session_id),
            "persisted session should appear in list"
        );
    }

    #[test]
    fn slash_commands_include_new_commands() {
        let commands = build_slash_commands();
        let names: Vec<&str> = commands.iter().map(|c| c.name.as_str()).collect();
        for expected in [
            "plan-show",
            "plan-resume",
            "analyze",
            "review",
            "agent-start",
            "agent-stop",
            "knowledge-gc",
            "knowledge-backup",
            "audit",
        ] {
            assert!(
                names.contains(&expected),
                "missing slash command: {expected}"
            );
        }
    }
}
```

### `crates/roko-acp/src/bridge_events.rs` (2032 lines — truncated)

```rust
//! Cognitive event to session/update streaming.
//!
//! Bridges Roko's provider system (via `roko-agent`) to ACP
//! `session/update` notifications.
//! All cognitive workflow dispatch now goes through
//! [`crate::runner::run_with_workflow_engine`], which uses `ModelCallService`
//! for provider-agnostic model calls.

use std::path::{Path, PathBuf};

use roko_agent::StreamChunk;
use roko_agent::streaming::parse_sse_line;
use roko_core::agent::{ProviderKind, resolve_model};
use roko_core::config::schema::RokoConfig;
use serde::Deserialize;
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt as _, AsyncRead, AsyncWrite},
    sync::mpsc,
};
use tracing::{debug, error, info, warn};

use crate::runner::run_with_workflow_engine;
use crate::{
    session::{AcpSession, CancelToken},
    transport::{StdioTransport, TransportError, TransportResult},
    types::{
        ContentBlock, JsonRpcMessage, PlanEntry, SESSION_BUSY, SessionCancelParams,
        SessionPromptParams, SessionPromptResult, SessionUpdate, StopReason, ToolCallKind,
        ToolCallStatus, UsageInfo,
    },
};

// ── Claude CLI stream-json wire types (kept for claude_cli fallback) ──

/// Top-level stream event from `claude --output-format stream-json`.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClaudeStreamEvent {
    System(ClaudeSystemEvent),
    Assistant(ClaudeAssistantEvent),
    Tool(ClaudeToolEvent),
    Result(ClaudeResultEvent),
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ClaudeSystemEvent {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub model: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ClaudeAssistantEvent {
    pub message: ClaudeMessage,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ClaudeMessage {
    #[serde(default)]
    pub content: Vec<ClaudeContentBlock>,
    #[serde(default)]
    pub usage: Option<ClaudeUsage>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClaudeContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String },
    Thinking { thinking: String },
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ClaudeToolEvent {
    #[serde(default, rename = "tool_name")]
    pub _tool_name: String,
    #[serde(default)]
    pub tool_use_id: String,
    #[serde(default)]
    pub content: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ClaudeResultEvent {
    #[serde(default)]
    pub total_cost_usd: Option<f64>,
    #[serde(default, rename = "is_error")]
    pub _is_error: bool,
    #[serde(default)]
    pub usage: Option<ClaudeUsage>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ClaudeUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

// ── Error types ──────────────────────────────────────────────────────

/// Errors produced while bridging cognitive events to ACP session updates.
#[derive(Debug, Error)]
pub enum BridgeEventsError {
    /// The target session already has an active prompt in flight.
    #[error("session '{0}' already has an active prompt")]
    SessionBusy(String),
    /// JSON serialization for an outbound session update failed.
    #[error("failed to serialize ACP session update: {0}")]
    Serialize(#[from] serde_json::Error),
    /// Writing to the ACP stdio transport failed.
    #[error("failed to send ACP session update: {0}")]
    Transport(#[from] TransportError),
    /// The spawned cognitive task terminated unexpectedly.
    #[error("ACP cognitive task failed: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),
    /// A pipeline runner error.
    #[error("ACP pipeline error: {0}")]
    Pipeline(#[from] anyhow::Error),
}

impl BridgeEventsError {
    /// Returns a JSON-RPC error tuple when the failure maps to a client-visible ACP error.
    #[must_use]
    pub fn rpc_error(&self) -> Option<(i32, String)> {
        match self {
            Self::SessionBusy(session_id) => Some((
                SESSION_BUSY,
                format!("session '{session_id}' already has an active prompt"),
            )),
            Self::Serialize(_) | Self::Transport(_) | Self::TaskJoin(_) | Self::Pipeline(_) => None,
        }
    }
}

/// Result alias for ACP event bridge operations.
pub type Result<T> = std::result::Result<T, BridgeEventsError>;

/// Maximum assistant response bytes stored in one history turn.
const MAX_HISTORY_ASSISTANT_BYTES: usize = 10_240;

// ── Cognitive events ─────────────────────────────────────────────────

/// Events emitted by the cognitive loop and mapped to ACP session updates.
#[derive(Debug, Clone)]
pub enum CognitiveEvent {
    /// A streamed agent-visible text chunk.
    TokenChunk(String),
    /// A streamed internal reasoning chunk.
    ThinkingChunk(String),
    /// A tool call has started running.
    ToolCallStart {
        tool_call_id: String,
        title: String,
        kind: ToolCallKind,
    },
    /// A tool call has finished with rendered content.
    ToolCallComplete {
        tool_call_id: String,
        status: ToolCallStatus,
        content: Vec<ContentBlock>,
    },
    /// A plan update with structured entries (shown as progress in editor).
    PlanUpdate { entries: Vec<PlanEntry> },
    /// Prompt execution completed normally.
    Complete {
        stop_reason: StopReason,
        usage: Option<UsageInfo>,
    },
    /// Prompt execution stopped because the token budget was exhausted.
    MaxTokens,
}

// ── Stream events → editor ───────────────────────────────────────────

/// Result of streaming events: the prompt result plus accumulated assistant text.
pub struct StreamResult {
    pub prompt_result: SessionPromptResult,
    /// Accumulated assistant text from TokenChunk events.
    pub assistant_text: String,
}

fn truncate_assistant_history(text: &str) -> String {
    if text.len() <= MAX_HISTORY_ASSISTANT_BYTES {
        return text.to_owned();
// ... (1632 lines omitted) ...

fn workflow_template_name(template: &crate::pipeline::WorkflowTemplate) -> &'static str {
    match template {
        crate::pipeline::WorkflowTemplate::Express => "express",
        crate::pipeline::WorkflowTemplate::Standard => "standard",
        crate::pipeline::WorkflowTemplate::Full => "full",
    }
}

fn text_block(text: String) -> ContentBlock {
    ContentBlock::Text { text }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tokio::io::{AsyncBufReadExt, BufReader, duplex, empty};

    use super::*;
    use crate::{
        session::AcpSession,
        transport::StdioTransport,
        types::{JsonRpcNotification, SessionNewParams},
    };

    #[tokio::test]
    async fn stream_events_to_editor_emits_notifications_and_returns_completion() {
        let (client, server) = duplex(4096);
        let mut transport = StdioTransport::from_io(empty(), server);
        let mut reader = BufReader::new(client);
        let cancel_token = CancelToken::new();
        let (sender, receiver) = mpsc::channel(8);

        sender
            .send(CognitiveEvent::TokenChunk("hello".to_owned()))
            .await
            .expect("send token chunk");
        sender
            .send(CognitiveEvent::Complete {
                stop_reason: StopReason::EndTurn,
                usage: Some(UsageInfo {
                    total_tokens: 12,
                    input_tokens: 5,
                    output_tokens: 7,
                    thought_tokens: None,
                    cached_read_tokens: None,
                    cached_write_tokens: None,
                }),
            })
            .await
            .expect("send completion");
        drop(sender);

        let result =
            stream_events_to_editor(&mut transport, "sess_test", receiver, &cancel_token).await;
        let result = result.expect("stream should succeed");

        assert_eq!(result.prompt_result.stop_reason, StopReason::EndTurn);

        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("read notification line");
        let notification: JsonRpcNotification =
            serde_json::from_str(&line).expect("deserialize notification");
        assert_eq!(notification.method, "session/update");
        assert_eq!(
            notification.params,
            Some(json!({
                "sessionId": "sess_test",
                "update": {
                    "sessionUpdate": "agent_message_chunk",
                    "content": {
                        "type": "text",
                        "text": "hello"
                    }
                }
            }))
        );
    }

    #[tokio::test]
    async fn stream_events_to_editor_returns_cancelled_when_token_is_cancelled() {
        let (_client, server) = duplex(1024);
        let mut transport = StdioTransport::from_io(empty(), server);
        let cancel_token = CancelToken::new();
        let (_sender, receiver) = mpsc::channel(1);

        cancel_token.cancel();

        let result =
            stream_events_to_editor(&mut transport, "sess_cancel", receiver, &cancel_token)
                .await
                .expect("cancelled prompt should still return a result");

        assert_eq!(result.prompt_result.stop_reason, StopReason::Cancelled);
    }

    #[tokio::test]
    async fn handle_session_prompt_rejects_busy_sessions() {
        let (_client, server) = duplex(1024);
        let mut transport = StdioTransport::from_io(empty(), server);
        let mut session = AcpSession::new(SessionNewParams {
            session_name: None,
            client_capabilities: None,
            mcp_servers: Vec::new(),
        });
        let session_id = session.session_id.clone();
        session.begin_prompt();

        let roko_config = RokoConfig::default();
        let error = handle_session_prompt(
            &mut transport,
            &mut session,
            SessionPromptParams {
                session_id: session_id.clone(),
                prompt: vec![ContentBlock::Text {
                    text: "busy".to_owned(),
                }],
                include_context: false,
            },
            Path::new("."),
            &roko_config,
        )
        .await
        .expect_err("busy session should be rejected");

        assert_eq!(
            error.rpc_error(),
            Some((
                SESSION_BUSY,
                format!("session '{session_id}' already has an active prompt")
            ))
        );
    }

    #[test]
    fn assistant_history_truncation_caps_bytes_and_preserves_boundaries() {
        let text = "é".repeat(6_000);
        let truncated = truncate_assistant_history(&text);
        let suffix = "...[truncated]";
        let prefix_len = truncated.len() - suffix.len();

        assert!(truncated.ends_with(suffix));
        assert!(truncated.len() <= MAX_HISTORY_ASSISTANT_BYTES + suffix.len());
        assert!(truncated.len() < text.len());
        assert!(truncated[..prefix_len].chars().all(|c| c == 'é'));
    }

    #[test]
    fn tool_name_mapping() {
        assert_eq!(tool_name_to_kind("Edit"), ToolCallKind::Edit);
        assert_eq!(tool_name_to_kind("Write"), ToolCallKind::Create);
        assert_eq!(tool_name_to_kind("Bash"), ToolCallKind::Terminal);
        assert_eq!(tool_name_to_kind("Read"), ToolCallKind::Other);
    }

    #[test]
    fn extract_at_mentions_supports_embedded_mentions() {
        let mentions = extract_at_mentions("fix @src/main.rs and @branch-diff, not foo@bar.com");
        assert_eq!(mentions, vec!["src/main.rs", "branch-diff"]);
    }

    #[tokio::test]
    async fn resolve_context_items_resolves_resource_and_path_mentions() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let workdir = tmp.path();
        let file_path = workdir.join("src/main.rs");
        std::fs::create_dir_all(file_path.parent().expect("parent directory")).expect("create dirs");
        std::fs::write(&file_path, "fn main() {}\n").expect("write file");

        let prompt = vec![
            ContentBlock::Resource {
                resource: crate::types::ResourceRef::File {
                    uri: format!("file://{}", file_path.display()),
                },
            },
            ContentBlock::Text {
                text: "check @src/main.rs".to_owned(),
            },
        ];

        let context = resolve_context_items(&prompt, workdir).await;
        assert!(context.contains("<file path=\"src/main.rs\">"));
        assert!(context.contains("--- src/main.rs ---"));
        assert!(context.contains("fn main() {}"));
    }

    #[test]
    fn truncate_with_limit_is_char_safe() {
        let text = "é".repeat(20_000);
        let truncated = truncate_with_limit(&text, 32_768, "... [truncated]");
        let prefix_len = truncated.len() - "... [truncated]".len();

        assert!(truncated.ends_with("... [truncated]"));
        assert!(truncated.len() < text.len());
        assert!(truncated[..prefix_len].chars().all(|c| c == 'é'));
    }
}
```

---

## Verification Commands

Run these and fix any failures:
```bash
cargo check -p roko-acp
cargo clippy -p roko-acp --no-deps -- -D warnings
```

## Do NOT

- Rewrite the entire implementation from scratch
- Add features not in the original prompt
- Modify files outside the write scope
- Skip running verification commands
