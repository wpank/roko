//! Runner-side T0 reflex matching, execution, and T2 promotion tracking.
//!
//! The persisted rule store lives in `roko-learn`. This module owns the
//! runner boundary: it builds deterministic task observations, validates and
//! authorizes the deliberately small executable action surface, and records
//! explicit successful T2 promotion candidates without inferring tool
//! arguments from prose.

use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::{self, Write as _};
use std::path::Component;
use std::path::{Path, PathBuf};
use std::time::Duration;

use roko_agent::SafetyLayer;
use roko_agent::safety::contract::AgentContract;
use roko_core::tool::{
    CancelToken, NeverCancel, NoopAuditSink, NoopMetricsSink, NoopTraceSink, ToolCall, ToolContext,
    ToolHandler, ToolResult,
};
use roko_learn::reflex_store::{
    PromotionCandidate, ReflexAction, ReflexCondition, ReflexObservation,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::task_parser::TaskDef;

/// Prefix for an explicit, machine-readable reflex promotion proposal.
pub(super) const CANDIDATE_MARKER: &str = "ROKO_REFLEX_CANDIDATE:";

/// Optional instruction appended only to T2/Premium prompts.
pub(super) const CANDIDATE_PROMPT: &str = r#"## Optional T0 reflex proposal
If this task reveals a deterministic shell action that should be reused for the
same observation, put exactly one single-line marker in your final response:
ROKO_REFLEX_CANDIDATE: {"action":{"tool":"bash","args":"command"}}
Omit the marker unless the command is precise, deterministic, and safe. Roko
derives the exact task condition itself, then authorizes and replays the command
from this attempt's clean pre-agent snapshot. It is only learned when replay
reproduces the successful attempt and both replay and task gates succeed."#;

const MAX_MARKER_BYTES: usize = 8 * 1024;
const MAX_CONDITION_FIELD_BYTES: usize = 1_024;
const MAX_ACTION_ARGS_BYTES: usize = 16 * 1024;
const MAX_LEDGER_LINE_BYTES: usize = 32 * 1024;
const MESSAGE_TYPE_TASK: &str = "task";

/// Build the stable observation checked before model routing or provider use.
pub(super) fn observation_for_task(
    task: &TaskDef,
    previous_gate_output: &str,
) -> ReflexObservation {
    // The core matcher intentionally treats condition context as a substring.
    // Always emitting one fixed-length opaque token therefore makes an
    // autonomously learned full token equivalent to exact equality. Hash the
    // complete serialized task (including contracts/tools/verification) and
    // complete prior gate output with explicit length framing so neither
    // truncation nor field-boundary ambiguity can alias distinct tasks.
    let serialized_task = serde_json::to_vec(task).unwrap_or_else(|_| {
        // TaskDef serialization is infallible for its current data model. Keep
        // the fallback deterministic and distinct if that invariant changes.
        b"<task-serialization-failed>".to_vec()
    });
    let mut digest = Sha256::new();
    digest.update(b"roko-task-observation-v1\0");
    update_length_framed(&mut digest, &serialized_task);
    update_length_framed(&mut digest, previous_gate_output.as_bytes());
    let context = format!("roko-task-v1:{:x}", digest.finalize());

    let mut file_exts = task
        .files
        .iter()
        .filter_map(|file| Path::new(file).extension())
        .filter_map(|extension| extension.to_str())
        .filter(|extension| !extension.is_empty())
        .map(|extension| format!(".{extension}"))
        .collect::<Vec<_>>();
    file_exts.sort();
    file_exts.dedup();

    ReflexObservation {
        tool: None,
        args: None,
        context: Some(context),
        message_type: Some(MESSAGE_TYPE_TASK.to_string()),
        file_exts,
    }
}

fn update_length_framed(digest: &mut Sha256, value: &[u8]) {
    digest.update(u64::try_from(value.len()).unwrap_or(u64::MAX).to_le_bytes());
    digest.update(value);
}

/// Validate and authorize a persisted action at the exact execution boundary.
///
/// Only local shell actions are supported. Every hit is rechecked through the
/// effective task contract and safety policy before it may touch the isolated
/// attempt worktree; persisted data is never treated as authority by itself.
pub(super) fn authorize_action(
    action: &ReflexAction,
    safety: Option<&SafetyLayer>,
    contract: AgentContract,
    task: &TaskDef,
    worktree: &Path,
    immune_root: &Path,
    timeout: Duration,
    cancel_token: Option<std::sync::Arc<dyn CancelToken>>,
) -> Result<(ToolCall, ToolContext, SafetyLayer), String> {
    let canonical = canonical_action(action)?;
    validate_action_paths(&canonical, worktree)?;
    let definition = roko_std::tool::builtin::bash::tool_def();
    let call = ToolCall::new(
        "t0-reflex",
        definition.name.clone(),
        serde_json::json!({"command": canonical.args}),
    );
    let tool_context = ToolContext::new(
        worktree,
        timeout,
        definition.permission,
        std::sync::Arc::new(NoopAuditSink),
        std::sync::Arc::new(NoopTraceSink),
        std::sync::Arc::new(NoopMetricsSink),
        cancel_token.unwrap_or_else(|| std::sync::Arc::new(NeverCancel)),
    )
    .with_immune_root(immune_root)
    .with_allowed_tools(task.allowed_tools.clone())
    .with_denied_tools(task.denied_tools.clone());

    let safety = safety
        .cloned()
        .unwrap_or_else(SafetyLayer::with_defaults)
        .with_contract(contract);
    safety
        .check_pre_execution_with_def(&definition, &call, &tool_context)
        .map_err(|error| format!("reflex action rejected by safety policy: {error}"))?;
    Ok((call, tool_context, safety))
}

/// Execute an already authorized local shell action.
pub(super) async fn execute_action(
    call: ToolCall,
    context: ToolContext,
    safety: SafetyLayer,
) -> (bool, String) {
    let cancel = std::sync::Arc::clone(&context.cancel_token);
    let result = tokio::select! {
        result = roko_std::tool::builtin::bash::Handler.execute(call, &context) => result,
        () = cancel.cancelled() => ToolResult::err(roko_core::tool::ToolError::Cancelled),
    };
    let result = safety.scrub_output(result);
    let result = match safety.check_recovery(&result) {
        Ok(()) => result,
        Err(error) => ToolResult::err(error),
    };
    match result {
        ToolResult::Ok { .. } => (true, result.text_content()),
        ToolResult::Err(error) => (false, error.to_string()),
    }
}

/// Extract an explicit promotion candidate from a completed T2 response.
///
/// The marker must be one bounded line, occur exactly once, contain only the
/// documented fields, name a supported action, and describe a condition that
/// actually matched the source attempt's own observation.
pub(super) fn candidate_from_output(
    output: &str,
    episode_id: &str,
    observation: &ReflexObservation,
) -> Option<PromotionCandidate> {
    if episode_id.trim().is_empty() || episode_id.len() > 1_024 {
        return None;
    }

    let mut encoded = None;
    for line in output.lines() {
        let Some(json) = line.strip_prefix(CANDIDATE_MARKER) else {
            continue;
        };
        if encoded.is_some() || json.is_empty() || json.len() > MAX_MARKER_BYTES {
            return None;
        }
        encoded = Some(json.trim());
    }
    let envelope: StrictCandidate = serde_json::from_str(encoded?).ok()?;
    let condition = ReflexCondition {
        tool: None,
        args_pattern: None,
        context: Some(observation.context.as_deref()?.to_string()),
        message_type: Some(MESSAGE_TYPE_TASK.to_string()),
        file_ext: None,
    };
    let action = canonical_action(&envelope.action.into_action()?).ok()?;
    if condition.context.as_deref().is_none_or(str::is_empty) || !condition.matches(observation) {
        return None;
    }

    Some(PromotionCandidate {
        episode_id: episode_id.to_string(),
        condition,
        action,
    })
}

fn canonical_action(action: &ReflexAction) -> Result<ReflexAction, String> {
    let tool = match action.tool.trim().to_ascii_lowercase().as_str() {
        "bash" | "shell" | "sh" => "bash",
        other => return Err(format!("unsupported reflex tool `{other}`")),
    };
    if action.args.trim().is_empty() {
        return Err("reflex shell command is empty".to_string());
    }
    if action.args.len() > MAX_ACTION_ARGS_BYTES || action.args.contains('\0') {
        return Err("reflex shell command is malformed or too large".to_string());
    }
    validate_simple_local_command(&action.args)?;
    Ok(ReflexAction {
        tool: tool.to_string(),
        args: action.args.clone(),
    })
}

/// Return whether a validated reflex uses the narrow command class whose
/// failure cannot mutate either the checkout or repository metadata.
pub(super) fn action_is_provably_read_only(action: &ReflexAction) -> bool {
    canonical_action(action).ok().is_some_and(|canonical| {
        canonical
            .args
            .split_ascii_whitespace()
            .next()
            .is_some_and(|program| {
                matches!(
                    program,
                    "pwd" | "ls" | "cat" | "head" | "tail" | "wc" | "rg"
                )
            })
    })
}

/// Keep learned actions deliberately narrower than the general interactive
/// shell tool. A reflex is unattended persisted input, so it may only invoke
/// one local read/build/test command and cannot use shell composition,
/// redirection, substitution, network clients, or paths outside the worktree.
fn validate_simple_local_command(command: &str) -> Result<(), String> {
    const SHELL_CONTROL: [&str; 20] = [
        "\n", "\r", ";", "|", "&", ">", "<", "`", "$(", "${", "\\", "'", "\"", "*", "?", "[", "]",
        "{", "}", "!",
    ];
    if SHELL_CONTROL.iter().any(|needle| command.contains(needle)) {
        return Err("reflex command contains unsupported shell syntax".to_string());
    }

    let tokens = command.split_ascii_whitespace().collect::<Vec<_>>();
    let Some(program) = tokens.first().copied() else {
        return Err("reflex shell command is empty".to_string());
    };
    let allowed = matches!(
        program,
        "cargo" | "rg" | "ls" | "pwd" | "cat" | "head" | "tail" | "wc" | "git"
    );
    if !allowed {
        return Err(format!("unsupported reflex command `{program}`"));
    }
    if program == "git" {
        let subcommand = tokens.get(1).copied().unwrap_or_default();
        if !matches!(
            subcommand,
            "status" | "diff" | "log" | "show" | "rev-parse" | "ls-files"
        ) {
            return Err(format!(
                "unsupported mutating or ambiguous git reflex `{subcommand}`"
            ));
        }
        validate_git_args(subcommand, &tokens[2..])?;
    }
    if program == "cargo" {
        let subcommand = tokens.get(1).copied().unwrap_or_default();
        if !matches!(
            subcommand,
            "build" | "check" | "clippy" | "doc" | "metadata" | "test" | "tree"
        ) {
            return Err(format!("unsupported cargo reflex `{subcommand}`"));
        }
        validate_cargo_args(&tokens[2..])?;
    }
    if program == "rg" {
        validate_rg_args(&tokens[1..])?;
    }
    for token in &tokens[1..] {
        for pathish in token.split('=') {
            if Path::new(pathish).components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            }) {
                return Err("reflex command may not access paths outside its worktree".to_string());
            }
            if pathish.starts_with('/')
                || pathish.starts_with('~')
                || pathish == ".."
                || pathish.starts_with("../")
                || pathish.contains("/../")
                || pathish.contains("://")
                || pathish.contains("@/")
                || pathish.contains('$')
            {
                return Err("reflex command may not access paths outside its worktree".to_string());
            }
            let name = Path::new(pathish)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            if matches!(name, ".env" | "id_rsa" | "id_ed25519")
                || name.ends_with(".pem")
                || name.ends_with(".key")
            {
                return Err("reflex command may not read credential files".to_string());
            }
        }
    }
    Ok(())
}

fn validate_action_paths(action: &ReflexAction, worktree: &Path) -> Result<(), String> {
    let root = std::fs::canonicalize(worktree)
        .map_err(|error| format!("reflex worktree is unavailable: {error}"))?;
    for token in action.args.split_ascii_whitespace().skip(1) {
        let value = token.split_once('=').map_or_else(
            || (!token.starts_with('-')).then_some(token),
            |(_, value)| Some(value),
        );
        if let Some(value) = value {
            let candidate = worktree.join(value);
            if !candidate.exists() {
                continue;
            }
            let resolved = std::fs::canonicalize(&candidate)
                .map_err(|error| format!("reflex path cannot be resolved safely: {error}"))?;
            if !resolved.starts_with(&root) {
                return Err(format!(
                    "reflex path `{value}` resolves outside the isolated worktree"
                ));
            }
        }
    }
    Ok(())
}

fn validate_cargo_args(args: &[&str]) -> Result<(), String> {
    const FLAG_ONLY: [&str; 20] = [
        "--workspace",
        "--all",
        "--all-features",
        "--no-default-features",
        "--no-fail-fast",
        "--release",
        "--locked",
        "--offline",
        "--frozen",
        "--quiet",
        "-q",
        "--verbose",
        "-v",
        "--lib",
        "--bins",
        "--examples",
        "--tests",
        "--benches",
        "--doc",
        "--no-run",
    ];
    const VALUE_FLAGS: [&str; 13] = [
        "-p",
        "--package",
        "--exclude",
        "--features",
        "--jobs",
        "-j",
        "--profile",
        "--bin",
        "--example",
        "--test",
        "--bench",
        "--message-format",
        "--color",
    ];
    let mut needs_value = false;
    for arg in args {
        if needs_value {
            if !safe_selector(arg) {
                return Err("cargo reflex contains an unsafe option value".to_string());
            }
            needs_value = false;
            continue;
        }
        if FLAG_ONLY.contains(arg) {
            continue;
        }
        if VALUE_FLAGS.contains(arg) {
            needs_value = true;
            continue;
        }
        if let Some((flag, value)) = arg.split_once('=')
            && VALUE_FLAGS.contains(&flag)
            && safe_selector(value)
        {
            continue;
        }
        if !arg.starts_with('-') && safe_selector(arg) {
            continue;
        }
        return Err(format!("unsupported cargo reflex argument `{arg}`"));
    }
    if needs_value {
        return Err("cargo reflex option is missing its value".to_string());
    }
    Ok(())
}

fn validate_git_args(subcommand: &str, args: &[&str]) -> Result<(), String> {
    let allowed_flag = |arg: &str| match subcommand {
        "status" => matches!(
            arg,
            "-s" | "--short"
                | "-b"
                | "--branch"
                | "--porcelain"
                | "--porcelain=v1"
                | "--porcelain=v2"
                | "--show-stash"
                | "--ahead-behind"
                | "--no-ahead-behind"
                | "-uno"
                | "-unormal"
                | "-uall"
                | "--untracked-files=no"
                | "--untracked-files=normal"
                | "--untracked-files=all"
        ),
        "diff" => {
            matches!(
                arg,
                "-p" | "--patch"
                    | "--stat"
                    | "--numstat"
                    | "--shortstat"
                    | "--name-only"
                    | "--name-status"
                    | "--check"
                    | "--summary"
                    | "--compact-summary"
                    | "--binary"
                    | "--cached"
                    | "--staged"
                    | "--no-color"
            ) || arg
                .strip_prefix("-U")
                .is_some_and(|value| value.bytes().all(|byte| byte.is_ascii_digit()))
        }
        "log" | "show" => matches!(
            arg,
            "--oneline"
                | "--decorate"
                | "--stat"
                | "--shortstat"
                | "--name-only"
                | "--name-status"
                | "--no-color"
                | "--no-patch"
        ),
        "rev-parse" => matches!(
            arg,
            "--verify"
                | "--show-toplevel"
                | "--show-prefix"
                | "--is-inside-work-tree"
                | "--abbrev-ref"
        ),
        "ls-files" => matches!(
            arg,
            "-z" | "--cached"
                | "--modified"
                | "--deleted"
                | "--others"
                | "--ignored"
                | "--stage"
                | "--unmerged"
                | "--killed"
                | "--exclude-standard"
                | "--error-unmatch"
        ),
        _ => false,
    };
    for arg in args {
        if arg == &"--" || !arg.starts_with('-') || allowed_flag(arg) {
            continue;
        }
        return Err(format!(
            "unsupported git {subcommand} reflex argument `{arg}`"
        ));
    }
    Ok(())
}

fn validate_rg_args(args: &[&str]) -> Result<(), String> {
    const FLAG_ONLY: [&str; 13] = [
        "-n",
        "--line-number",
        "-i",
        "--ignore-case",
        "-S",
        "--smart-case",
        "-F",
        "--fixed-strings",
        "-l",
        "--files-with-matches",
        "--files",
        "--hidden",
        "--json",
    ];
    const VALUE_FLAGS: [&str; 10] = [
        "-g",
        "--glob",
        "-t",
        "--type",
        "-T",
        "--type-not",
        "-m",
        "--max-count",
        "-C",
        "--context",
    ];
    let mut needs_value = false;
    for arg in args {
        if needs_value {
            if !safe_selector(arg) {
                return Err("rg reflex contains an unsafe option value".to_string());
            }
            needs_value = false;
            continue;
        }
        if FLAG_ONLY.contains(arg) {
            continue;
        }
        if VALUE_FLAGS.contains(arg) {
            needs_value = true;
            continue;
        }
        if !arg.starts_with('-') && safe_selector(arg) {
            continue;
        }
        return Err(format!("unsupported rg reflex argument `{arg}`"));
    }
    if needs_value {
        return Err("rg reflex option is missing its value".to_string());
    }
    Ok(())
}

fn safe_selector(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._,+:-/".contains(&byte))
}

fn normalize_condition_field(value: Option<String>) -> Option<Option<String>> {
    match value {
        None => Some(None),
        Some(value) => {
            if value.trim().is_empty()
                || value.len() > MAX_CONDITION_FIELD_BYTES
                || value.contains('\0')
            {
                None
            } else {
                Some(Some(value))
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictCandidate {
    action: StrictAction,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictCondition {
    #[serde(default)]
    tool: Option<String>,
    #[serde(default)]
    args_pattern: Option<String>,
    #[serde(default)]
    context: Option<String>,
    #[serde(default)]
    message_type: Option<String>,
    #[serde(default)]
    file_ext: Option<String>,
}

impl StrictCondition {
    fn into_condition(self) -> Option<ReflexCondition> {
        let condition = ReflexCondition {
            tool: normalize_condition_field(self.tool)?,
            args_pattern: normalize_condition_field(self.args_pattern)?,
            context: normalize_condition_field(self.context)?,
            message_type: normalize_condition_field(self.message_type)?,
            file_ext: normalize_condition_field(self.file_ext)?,
        };
        if condition.tool.is_none()
            && condition.args_pattern.is_none()
            && condition.context.is_none()
            && condition.message_type.is_none()
            && condition.file_ext.is_none()
        {
            return None;
        }
        Some(condition)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictAction {
    tool: String,
    args: String,
}

impl StrictAction {
    fn into_action(self) -> Option<ReflexAction> {
        if self.tool.len() > 32 {
            return None;
        }
        Some(ReflexAction {
            tool: self.tool,
            args: self.args,
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CandidateLedgerKind {
    Success,
    Reset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateLedgerRecord {
    kind: CandidateLedgerKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    episode_id: Option<String>,
    condition: ReflexCondition,
    action: ReflexAction,
}

impl CandidateLedgerRecord {
    fn key(&self) -> Option<String> {
        serde_json::to_string(&(&self.condition, &self.action)).ok()
    }

    fn is_valid(&self) -> bool {
        let valid_episode = match (&self.kind, self.episode_id.as_deref()) {
            (CandidateLedgerKind::Success, Some(id)) => !id.trim().is_empty() && id.len() <= 1_024,
            (CandidateLedgerKind::Reset, None) => true,
            _ => false,
        };
        valid_episode
            && StrictCondition {
                tool: self.condition.tool.clone(),
                args_pattern: self.condition.args_pattern.clone(),
                context: self.condition.context.clone(),
                message_type: self.condition.message_type.clone(),
                file_ext: self.condition.file_ext.clone(),
            }
            .into_condition()
            .is_some()
            && canonical_action(&self.action).is_ok()
    }
}

/// Append-only, restart-safe count of successful explicit T2 candidates.
pub(super) struct PromotionTracker {
    path: PathBuf,
    seen_episodes: HashSet<String>,
    counts: HashMap<String, u32>,
}

impl PromotionTracker {
    pub(super) fn open(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut tracker = Self {
            path,
            seen_episodes: HashSet::new(),
            counts: HashMap::new(),
        };
        let Ok(text) = std::fs::read_to_string(&tracker.path) else {
            return tracker;
        };
        for line in text.lines() {
            if line.is_empty() || line.len() > MAX_LEDGER_LINE_BYTES {
                continue;
            }
            let Ok(record) = serde_json::from_str::<CandidateLedgerRecord>(line) else {
                continue;
            };
            if !record.is_valid() {
                continue;
            }
            let Some(key) = record.key() else {
                continue;
            };
            match record.kind {
                CandidateLedgerKind::Success => {
                    let Some(episode_id) = record.episode_id else {
                        continue;
                    };
                    if tracker.seen_episodes.insert(episode_id) {
                        let count = tracker.counts.entry(key).or_default();
                        *count = count.saturating_add(1);
                    }
                }
                CandidateLedgerKind::Reset => {
                    tracker.counts.insert(key, 0);
                }
            }
        }
        tracker
    }

    /// Durably count one successful exact attempt and return its stable count.
    /// Duplicate attempt IDs are ignored. I/O errors are returned so callers
    /// can log them without changing task or fallback behavior.
    pub(super) fn record_success(&mut self, candidate: &PromotionCandidate) -> io::Result<u32> {
        let occurrence = CandidateLedgerRecord {
            kind: CandidateLedgerKind::Success,
            episode_id: Some(candidate.episode_id.clone()),
            condition: candidate.condition.clone(),
            action: canonical_action(&candidate.action)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?,
        };
        if !occurrence.is_valid() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid reflex promotion occurrence",
            ));
        }
        let key = occurrence
            .key()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "serialize candidate key"))?;
        if self.seen_episodes.contains(&candidate.episode_id) {
            return Ok(self.counts.get(&key).copied().unwrap_or_default());
        }

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let encoded = serde_json::to_string(&occurrence)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{encoded}")?;
        file.sync_data()?;

        self.seen_episodes.insert(candidate.episode_id.clone());
        let count = self.counts.entry(key).or_default();
        *count = count.saturating_add(1);
        Ok(*count)
    }

    /// Start a fresh promotion generation after an exact rule is demoted.
    /// Historical successful episodes remain deduplicated, but no longer
    /// contribute toward the three new gate-covered occurrences required to
    /// re-promote the pair.
    pub(super) fn reset(
        &mut self,
        condition: &ReflexCondition,
        action: &ReflexAction,
    ) -> io::Result<()> {
        let record = CandidateLedgerRecord {
            kind: CandidateLedgerKind::Reset,
            episode_id: None,
            condition: condition.clone(),
            action: canonical_action(action)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?,
        };
        if !record.is_valid() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid reflex promotion reset",
            ));
        }
        let key = record
            .key()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "serialize reset key"))?;
        let result = self.append(&record);
        // Fail closed in this process even if durable bookkeeping is
        // temporarily unavailable: old successes must not immediately undo a
        // demotion.
        self.counts.insert(key, 0);
        result
    }

    fn append(&self, record: &CandidateLedgerRecord) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let encoded = serde_json::to_string(record)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{encoded}")?;
        file.sync_data()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(title: &str) -> TaskDef {
        serde_json::from_value(serde_json::json!({
            "id": "task",
            "title": title,
        }))
        .expect("minimal task definition")
    }

    fn observation() -> ReflexObservation {
        ReflexObservation {
            tool: None,
            args: None,
            context: Some("compile rust workspace".to_string()),
            message_type: Some("task".to_string()),
            file_exts: vec![".rs".to_string()],
        }
    }

    fn marker(tool: &str) -> String {
        format!(r#"{CANDIDATE_MARKER}{{"action":{{"tool":"{tool}","args":"cargo check"}}}}"#)
    }

    #[test]
    fn explicit_candidate_uses_runner_derived_exact_source_condition() {
        let source = observation();
        let candidate = candidate_from_output(&marker("bash"), "p:t:1", &source)
            .expect("matching explicit marker");
        assert_eq!(candidate.action.tool, "bash");
        assert_eq!(candidate.action.args, "cargo check");
        assert_eq!(candidate.condition.context, source.context);
        assert_eq!(candidate.condition.message_type.as_deref(), Some("task"));

        assert!(candidate_from_output(&marker("network"), "p:t:3", &source).is_none());
    }

    #[test]
    fn ambiguous_or_wildcard_markers_are_rejected() {
        let wildcard = format!(
            r#"{CANDIDATE_MARKER}{{"condition":{{}},"action":{{"tool":"bash","args":"cargo check"}}}}"#
        );
        assert!(candidate_from_output(&wildcard, "p:t:1", &observation()).is_none());

        let repeated = format!("{}\n{}", marker("bash"), marker("bash"));
        assert!(candidate_from_output(&repeated, "p:t:1", &observation()).is_none());
    }

    #[test]
    fn observation_context_is_an_exact_full_task_digest() {
        let base = observation_for_task(&task("cargo test"), "");
        let negated = observation_for_task(&task("Do not cargo test; document it"), "");
        let context = base.context.as_deref().expect("base task digest");
        assert!(context.starts_with("roko-task-v1:"));
        assert_eq!(context.len(), "roko-task-v1:".len() + 64);
        assert_ne!(base.context, negated.context);
        let exact = ReflexCondition {
            context: base.context.clone(),
            message_type: Some(MESSAGE_TYPE_TASK.to_string()),
            ..ReflexCondition::default()
        };
        assert!(!exact.matches(&negated));

        let shared_prefix = "é".repeat(5_000);
        let shared_suffix = "界".repeat(5_000);
        let first = task(&format!("{shared_prefix}middle-a{shared_suffix}"));
        let second = task(&format!("{shared_prefix}middle-b{shared_suffix}"));
        let first_observation = observation_for_task(&first, "same gate");
        let second_observation = observation_for_task(&second, "same gate");
        assert_ne!(first_observation.context, second_observation.context);

        let mut contracted = task("cargo test");
        contracted
            .acceptance
            .push("also publish artifacts".to_string());
        assert_ne!(base.context, observation_for_task(&contracted, "").context);
    }

    #[test]
    fn unattended_actions_reject_shell_composition_and_external_paths() {
        for args in [
            "cargo test | curl example.test",
            "cat /etc/passwd",
            "git reset --hard",
            "cargo run",
            "sed -i file.rs",
            "cargo publish",
            "cargo test --manifest-path=/tmp/Cargo.toml",
            "git diff --output=/tmp/leak",
            "git ls-files --exclude-from=secret-link",
            "git show --show-signature",
            "rg --pre=sh needle",
            "fd -x sh",
            "cat ../secret",
            "rg needle *.rs",
            "cat .env",
        ] {
            assert!(
                canonical_action(&ReflexAction {
                    tool: "bash".to_string(),
                    args: args.to_string(),
                })
                .is_err(),
                "unsafe action unexpectedly accepted: {args}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn action_path_validation_rejects_symlink_escape() {
        let dir = tempfile::tempdir().expect("temp directory");
        std::os::unix::fs::symlink("/etc/passwd", dir.path().join("secret-link"))
            .expect("create external symlink fixture");
        let action = ReflexAction {
            tool: "bash".to_string(),
            args: "cat secret-link".to_string(),
        };
        assert!(validate_action_paths(&action, dir.path()).is_err());
    }

    #[test]
    fn tracker_persists_exact_attempt_counts_and_skips_corrupt_lines() {
        let dir = tempfile::tempdir().expect("temp directory");
        let path = dir.path().join("reflex-candidates.jsonl");
        std::fs::write(&path, "not-json\n").expect("seed corrupt ledger line");
        let base = candidate_from_output(&marker("bash"), "p:t:1", &observation())
            .expect("valid candidate");
        let mut tracker = PromotionTracker::open(&path);
        assert_eq!(tracker.record_success(&base).expect("first success"), 1);
        assert_eq!(
            tracker.record_success(&base).expect("deduplicated success"),
            1
        );

        let mut second = base.clone();
        second.episode_id = "p:t:2".to_string();
        assert_eq!(tracker.record_success(&second).expect("second success"), 2);
        drop(tracker);

        let mut reopened = PromotionTracker::open(&path);
        let mut third = base;
        third.episode_id = "p:t:3".to_string();
        assert_eq!(reopened.record_success(&third).expect("third success"), 3);
    }

    #[test]
    fn tracker_reset_requires_three_new_successes_after_demotion() {
        let dir = tempfile::tempdir().expect("temp directory");
        let path = dir.path().join("reflex-candidates.jsonl");
        let base = candidate_from_output(&marker("bash"), "run-1:p:t:1", &observation())
            .expect("valid candidate");
        let mut tracker = PromotionTracker::open(&path);
        assert_eq!(tracker.record_success(&base).expect("first success"), 1);
        let mut second = base.clone();
        second.episode_id = "run-1:p:t:2".to_string();
        assert_eq!(tracker.record_success(&second).expect("second success"), 2);
        tracker
            .reset(&base.condition, &base.action)
            .expect("durable demotion reset");

        let mut reopened = PromotionTracker::open(&path);
        let mut post_demotion = base;
        post_demotion.episode_id = "run-2:p:t:1".to_string();
        assert_eq!(
            reopened
                .record_success(&post_demotion)
                .expect("post-demotion success"),
            1
        );
    }
}
