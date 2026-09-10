//! Session construction, persistence, model helpers, and history.

use std::io::Write as _;
use std::time::Instant;

use super::types::{ChatInlineDispatchError, ChatSession, SessionSnapshot};
use crate::auth_detect::AuthMethod;
use crate::chat_session::ChatAgentSession;
use crate::dispatch_v2::{DispatchResult, ToolOutput};

use roko_learn::cost_table::CostTable;

// ---------------------------------------------------------------------------
// History persistence
// ---------------------------------------------------------------------------

/// Path to the persistent chat history file.
pub(crate) fn history_path() -> std::path::PathBuf {
    let roko_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(".roko");
    roko_dir.join("chat_history")
}

/// Load history entries from disk (one per line, last 500).
pub(crate) fn load_history() -> Vec<String> {
    let path = history_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => content
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.replace("\\n", "\n"))
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Append a single history entry to disk.
pub(crate) fn save_history_entry(entry: &str) {
    let path = history_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let escaped = entry.replace('\n', "\\n");
        let _ = writeln!(f, "{escaped}");
    }
    // Trim to 500 entries periodically
    if let Ok(content) = std::fs::read_to_string(&path) {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() > 600 {
            let trimmed: Vec<&str> = lines[lines.len() - 500..].to_vec();
            let _ = std::fs::write(&path, trimmed.join("\n") + "\n");
        }
    }
}

// ---------------------------------------------------------------------------
// Session auto-save
// ---------------------------------------------------------------------------

/// Directory for session snapshots.
fn sessions_dir() -> std::path::PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(".roko")
        .join("sessions")
}

/// Save current session state to disk.
pub(crate) fn save_session(session: &ChatSession) {
    if session.conversation.is_empty() {
        return;
    }
    let dir = sessions_dir();
    let _ = std::fs::create_dir_all(&dir);
    let first_user = session
        .conversation
        .iter()
        .find(|m| m.role == "user")
        .map(|m| {
            let s = m.text.replace('\n', " ");
            if s.len() > 80 {
                format!("{}...", &s[..77])
            } else {
                s
            }
        });
    let snapshot = SessionSnapshot {
        turn_count: session.turn_count,
        total_cost: session.cost.total_cost,
        input_tokens: session.cost.input_tokens,
        output_tokens: session.cost.output_tokens,
        model: active_model_name(session),
        agent_id: session.agent_id.clone(),
        messages: session.conversation.clone(),
        system_message: active_system_prompt(session),
        saved_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        first_user_message: first_user,
    };
    let path = dir.join("last.json");
    let _ = std::fs::write(
        &path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_default(),
    );
}

/// Load the last session summary (not full restore -- just for display).
pub(crate) fn load_last_session_summary() -> Option<(String, u32, f64, String)> {
    let path = sessions_dir().join("last.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let snap: SessionSnapshot = serde_json::from_str(&content).ok()?;
    let saved_at = snap.saved_at;
    let topic = snap.first_user_message.unwrap_or_default();
    Some((saved_at, snap.turn_count, snap.total_cost, topic))
}

// ---------------------------------------------------------------------------
// Model / session helpers
// ---------------------------------------------------------------------------

pub(crate) fn active_model_name(session: &ChatSession) -> String {
    if let Some(agent_session) = session.agent_session.as_ref() {
        return agent_session.model.clone();
    }

    match &session.dispatch {
        super::types::DispatchMode::Direct { auth } => match auth {
            AuthMethod::AnthropicApi { model, .. } => {
                model.as_deref().unwrap_or("claude-sonnet-4-6").to_string()
            }
            AuthMethod::ClaudeCli => "claude CLI".to_string(),
            AuthMethod::CliProvider { label } => format!("{label} CLI"),
            AuthMethod::OpenAiCompat { model, .. } => {
                model.as_deref().unwrap_or_default().to_string()
            }
            AuthMethod::NeedsSetup => String::new(),
        },
        super::types::DispatchMode::Http { .. } => "HTTP backend".to_string(),
        super::types::DispatchMode::Session => session
            .agent_session
            .as_ref()
            .map(|s| s.model.clone())
            .unwrap_or_else(|| "session".to_string()),
    }
}

/// Format the resolved model selection for display in the startup banner.
///
/// Returns a compact label like `"claude-sonnet-4-6 (claude-cli)"` that reflects
/// what dispatch will actually use, not what auth detection reported upstream.
pub(crate) fn session_banner_label(
    selection: &crate::model_selection::EffectiveModelSelection,
) -> String {
    format!(
        "{} ({})",
        selection.effective_model_key, selection.provider_kind
    )
}

pub(crate) fn active_system_prompt(session: &ChatSession) -> Option<String> {
    session
        .agent_session
        .as_ref()
        .map(|agent_session| agent_session.system_prompt.clone())
        .or_else(|| session.system_message.clone())
}

/// Get the current model name for display.
pub(crate) fn current_model_name(session: &ChatSession) -> String {
    active_model_name(session)
}

// ---------------------------------------------------------------------------
// Model switching
// ---------------------------------------------------------------------------

/// Apply a `/model <arg>` switch atomically.
///
/// In `DispatchMode::Session`, the new model is first resolved against
/// the current workdir's config so a successful return guarantees a
/// fresh `EffectiveModelSelection`. Both `agent_session.model` and
/// `agent_session.model_selection` are then committed together. On
/// failure neither field is touched and the caller surfaces a single
/// error to the user.
///
/// In `DispatchMode::Direct`, the auth model is the only mutable
/// piece; we still validate the argument is non-empty / non-whitespace
/// before mutating so a typo never produces a half-applied state.
pub(crate) fn apply_model_switch(
    session: &mut ChatSession,
    arg: &str,
) -> Result<String, super::types::ModelSwitchError> {
    use super::types::ModelSwitchError;
    let arg = arg.trim();
    if arg.is_empty() {
        return Err(ModelSwitchError::new("model name cannot be empty"));
    }

    match &mut session.dispatch {
        super::types::DispatchMode::Direct { auth } => match auth {
            AuthMethod::AnthropicApi { model, .. } | AuthMethod::OpenAiCompat { model, .. } => {
                *model = Some(arg.to_string());
                Ok(arg.to_string())
            }
            _ => Err(ModelSwitchError::with_hint(
                "can only switch with API providers",
                "set ZAI_API_KEY, OPENAI_API_KEY, or ANTHROPIC_API_KEY",
            )),
        },
        super::types::DispatchMode::Http { .. } => Err(ModelSwitchError::new(
            "model switching not supported in HTTP mode",
        )),
        super::types::DispatchMode::Session => {
            let Some(agent_session) = session.agent_session.as_mut() else {
                return Err(ModelSwitchError::new("ChatAgentSession unavailable"));
            };
            let next_selection =
                resolve_model_selection_for_workdir(&agent_session.workdir, arg)
                    .map_err(|err| ModelSwitchError::new(err.to_string()))?;
            agent_session.model = next_selection.effective_model_key.clone();
            agent_session.model_selection = next_selection.clone();
            Ok(next_selection.effective_model_key)
        }
    }
}

/// Re-resolve the effective model for `workdir` with `requested` as the
/// CLI override. Mirrors the construction-time call site.
fn resolve_model_selection_for_workdir(
    workdir: &std::path::Path,
    requested: &str,
) -> Result<crate::model_selection::EffectiveModelSelection, crate::model_selection::Error> {
    let resolved = match crate::config::load_resolved_config(workdir) {
        Ok(resolved) => resolved.config,
        Err(_) => crate::config::Config::default(),
    };
    let mut model_config = roko_core::config::schema::RokoConfig::default();
    model_config.providers.extend(resolved.providers.clone());
    model_config.models.extend(resolved.models.clone());
    if let Some(model) = resolved.agent.model.clone() {
        model_config.agent.default_model = model;
    }
    let role = {
        let role = resolved.prompt.role.trim();
        (!role.is_empty()).then(|| role.to_string())
    };
    crate::model_selection::resolve_effective_model(
        Some(requested.to_string()),
        None,
        role,
        None,
        &model_config,
        None,
    )
}

// ---------------------------------------------------------------------------
// Session construction helpers
// ---------------------------------------------------------------------------

pub(crate) fn clone_chat_agent_session(session: &ChatAgentSession) -> ChatAgentSession {
    ChatAgentSession {
        workdir: session.workdir.clone(),
        model: session.model.clone(),
        model_selection: session.model_selection.clone(),
        effort: session.effort.clone(),
        system_prompt: session.system_prompt.clone(),
        allowed_tools_csv: session.allowed_tools_csv.clone(),
        mcp_config: session.mcp_config.clone(),
        session_id: session.session_id.clone(),
        api_history: session.api_history.clone(),
        settings_json: session.settings_json.clone(),
        timeout: session.timeout,
        provider_base_url: session.provider_base_url.clone(),
        provider_api_key_env: session.provider_api_key_env.clone(),
    }
}

pub(crate) fn turn_result_to_dispatch_result(
    turn: crate::chat_session::TurnResult,
) -> DispatchResult {
    DispatchResult {
        text: turn.text,
        model: turn.model,
        input_tokens: turn.input_tokens,
        output_tokens: turn.output_tokens,
        tool_outputs: turn
            .tool_calls
            .into_iter()
            .map(|tool_call| ToolOutput {
                tool_name: Some(tool_call.name),
                content: if tool_call.input_abbrev.is_empty() {
                    if tool_call.success {
                        "done".to_string()
                    } else {
                        "failed".to_string()
                    }
                } else {
                    tool_call.input_abbrev
                },
            })
            .collect(),
        session_id: turn.session_id,
    }
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

/// Thinking phase label based on elapsed time.
pub(crate) fn thinking_label(elapsed_s: f64) -> &'static str {
    if elapsed_s < 2.0 {
        "Connecting..."
    } else if elapsed_s < 8.0 {
        "Thinking..."
    } else if elapsed_s < 15.0 {
        "Still thinking..."
    } else {
        "Deep in thought..."
    }
}

/// Truncate a string to fit within `max` columns, adding "..." if needed.
pub(crate) fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max > 3 {
        format!("{}...", &s[..max - 3])
    } else {
        s[..max].to_string()
    }
}

pub(crate) fn preview_text(text: &str, max_chars: usize) -> String {
    let mut chars = text
        .chars()
        .map(|c| if matches!(c, '\n' | '\r') { ' ' } else { c });
    let preview: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{preview}...")
    } else {
        preview
    }
}

pub(crate) fn format_time(instant: Instant) -> String {
    // Use system time offset from session start
    let now = chrono::Local::now();
    let _ = instant; // we use wall clock, not the instant
    now.format("%-I:%M %p").to_string()
}

/// Build unified inline agent session.
pub(crate) fn build_unified_inline_agent_session(
    workdir: std::path::PathBuf,
) -> std::result::Result<(ChatAgentSession, Option<String>, CostTable), ChatInlineDispatchError> {
    let resolved = crate::config::load_resolved_config(&workdir)
        .map_err(|source| ChatInlineDispatchError::ConfigLoad { source })?;
    let config = resolved.config;

    let model_config = roko_core::config::loader::load_config_unified(&workdir).unwrap_or_default();

    let cost_table = CostTable::from_config(&model_config.models).with_defaults();
    let role = {
        let role = config.prompt.role.trim();
        (!role.is_empty()).then(|| role.to_string())
    };
    let selection = crate::model_selection::resolve_effective_model(
        None,
        None,
        role,
        None,
        &model_config,
        None,
    )
    .map_err(|source| ChatInlineDispatchError::ModelSelection { source })?;
    let agent_session = ChatAgentSession::new(&config, workdir, selection)
        .map_err(|source| ChatInlineDispatchError::ChatSessionInit { source })?;

    tracing::debug!(
        model = %agent_session.model_selection.effective_model_key,
        provider = %agent_session.model_selection.provider_key,
        source = %agent_session.model_selection.source,
        "resolved effective model for chat session"
    );

    let system_message = Some(agent_session.system_prompt.clone());

    Ok((agent_session, system_message, cost_table))
}
