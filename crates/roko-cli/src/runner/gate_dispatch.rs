//! Verify dispatch — runs gate rungs as background tokio tasks and sends
//! results through a channel.

use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use futures::FutureExt;
use roko_core::config::{GateRungConfig, GatesConfig};
use roko_core::{
    Body, Kind, LensScope, ObservableEvent, Provenance, Signal, SignalBuilder, TelemetryEventSink,
    Verdict, Verify,
};
use roko_fs::RokoLayout;
use roko_gate::classify_gate_failure;
use roko_gate::generated_test_gate::ArtifactStore as GeneratedArtifactStore;
use roko_gate::llm_judge_gate::JudgePayload;
use roko_gate::rung_dispatch::{GatePipelineBuilder, RungExecutionConfig, RungExecutionInputs};
use roko_gate::rung_for_gate_name;
use roko_gate::symbol_gate::{SymbolExpectation, SymbolKind, SymbolManifest, Visibility};
use roko_gate::verdict_publisher::VerdictPublisher;
use roko_gate::{GatePayload, PlanComplexity, ShellGate};
use sha2::{Digest, Sha256};
use tokio::sync::{Semaphore, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{Duration, timeout};
use tracing::{error, info};

use crate::gate_runner::FsGeneratedArtifactStore;
use crate::task_parser::VerifyStep;

use super::types::{
    GateCompletion, GateCompletionKind, GateEffectRef, GateVerdictSummary, RunnerFailureKind,
    TaskAttemptRef,
};

/// Sentinel rung value for plan-level verification (not a per-task rung).
pub const RUNG_PLAN_VERIFY: u32 = 1000;
/// Sentinel rung value for post-merge regression gates.
pub const RUNG_MERGE: u32 = 1001;

/// Compute the `CARGO_BUILD_JOBS` limit: half the available logical CPUs,
/// floored to at least 1. This prevents CPU exhaustion when multiple agents
/// run gate checks in parallel.
fn cargo_build_jobs() -> String {
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2);
    (cpus / 2).max(1).to_string()
}

/// Detect whether `sccache` is available on `PATH`. The result is cached
/// after the first call to avoid repeated filesystem lookups.
fn sccache_available() -> bool {
    use std::sync::OnceLock;
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        let path_var = std::env::var("PATH").unwrap_or_default();
        std::env::split_paths(&path_var)
            .any(|dir| dir.join("sccache").is_file() || dir.join("sccache.exe").is_file())
    })
}

/// Task-definition-derived context used by [`build_rung_execution_inputs`] and
/// [`build_rung_execution_config`] to build real `RungExecutionInputs` for
/// advanced gate rungs (Symbol, FactCheck, LlmJudge, GeneratedTest).
///
/// Constructed by the event loop from the current [`TaskDef`] before spawning
/// the gate worker.
#[derive(Clone, Debug, Default)]
pub struct GateTaskContext {
    /// Plan identifier, used as the label for symbol manifests.
    pub plan_id: String,
    /// Context symbols from the task definition (for the Symbol gate).
    pub symbols: Vec<String>,
    /// Acceptance criteria from the task definition (for the FactCheck gate).
    pub acceptance: Vec<String>,
    /// Task description (for the LlmJudge gate).
    pub task_description: Option<String>,
    /// Task title fallback when description is absent.
    pub task_title: String,
}

impl GateTaskContext {
    /// Build a `GateTaskContext` from a plan ID and optional task definition.
    ///
    /// Extracts the symbols, acceptance criteria, description, and title
    /// that the advanced gate rungs need to produce real verdicts.
    pub fn from_task_def(
        plan_id: &str,
        task_def: Option<&crate::task_parser::TaskDef>,
    ) -> Option<Self> {
        let td = task_def?;
        Some(Self {
            plan_id: plan_id.to_string(),
            symbols: td
                .context
                .as_ref()
                .map(|ctx| ctx.symbols.clone())
                .unwrap_or_default(),
            acceptance: td.acceptance.clone(),
            task_description: td.description.clone(),
            task_title: td.title.clone(),
        })
    }
}

fn fast_mode_enabled() -> bool {
    std::env::var("ROKO_FAST_MODE").is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn task_verify_only_enabled() -> bool {
    std::env::var("ROKO_TASK_VERIFY_ONLY").is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn fast_task_verify_contract_error(
    fast_mode: bool,
    task_verify_only: bool,
    authored_verify_count: usize,
) -> Option<String> {
    (fast_mode && task_verify_only && authored_verify_count != 1).then(|| {
        format!(
            "FAST task-owned verification requires exactly one authored verify step; found {authored_verify_count}"
        )
    })
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum CargoTargetSelector {
    Lib,
    Bin(String),
    Test(String),
}

impl CargoTargetSelector {
    fn command_args(&self) -> (&'static str, Option<&str>) {
        match self {
            Self::Lib => ("--lib", None),
            Self::Bin(name) => ("--bin", Some(name)),
            Self::Test(name) => ("--test", Some(name)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TargetedCargoCheck {
    package: String,
    target: CargoTargetSelector,
    command: String,
}

fn safe_cargo_name(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn git_changed_files(workdir: &Path) -> Option<Vec<String>> {
    fn collect_git_paths(workdir: &Path, args: &[&str], paths: &mut BTreeSet<String>) -> bool {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(workdir)
            .output();
        let Ok(output) = output else {
            return false;
        };
        if !output.status.success() {
            return false;
        }
        paths.extend(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(ToOwned::to_owned),
        );
        true
    }

    // Deletions, renames/copies, type changes, conflicts, and unknown status
    // cannot be represented by one positive Cargo target. Fail closed to the
    // original broad gate whenever any such change is present.
    let mut unsafe_paths = BTreeSet::new();
    if !collect_git_paths(
        workdir,
        &["diff", "--name-only", "--diff-filter=CDRTUXB", "HEAD", "--"],
        &mut unsafe_paths,
    ) || !unsafe_paths.is_empty()
    {
        return None;
    }

    let mut paths = BTreeSet::new();
    if !collect_git_paths(
        workdir,
        &["diff", "--name-only", "--diff-filter=AM", "HEAD", "--"],
        &mut paths,
    ) || !collect_git_paths(
        workdir,
        &["ls-files", "--others", "--exclude-standard"],
        &mut paths,
    ) {
        return None;
    }
    Some(paths.into_iter().collect())
}

fn cargo_manifest_for_file(workdir: &Path, file: &str) -> Option<(PathBuf, String)> {
    let relative = Path::new(file);
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return None;
    }
    let absolute_file = workdir.join(relative);
    let mut cursor = absolute_file.parent()?.to_path_buf();
    loop {
        if cursor.join("Cargo.toml").is_file() {
            let within_package = absolute_file.strip_prefix(&cursor).ok()?;
            return within_package
                .to_str()
                .map(|path| (cursor, path.to_string()));
        }
        if cursor == workdir || !cursor.pop() || !cursor.starts_with(workdir) {
            return None;
        }
    }
}

fn manifest_target_for_path(
    manifest: &toml::Value,
    package: &str,
    path: &str,
) -> Option<CargoTargetSelector> {
    let normalize = |value: &str| {
        value
            .replace('\\', "/")
            .trim_start_matches("./")
            .to_string()
    };
    let path = normalize(path);
    let package_config = manifest.get("package").and_then(toml::Value::as_table);
    let auto_lib = package_config
        .and_then(|table| table.get("autolib"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    let auto_bins = package_config
        .and_then(|table| table.get("autobins"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    let auto_tests = package_config
        .and_then(|table| table.get("autotests"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    let mut matches = BTreeSet::new();

    if let Some(lib) = manifest.get("lib").and_then(toml::Value::as_table) {
        let lib_path = lib
            .get("path")
            .and_then(toml::Value::as_str)
            .map_or_else(|| "src/lib.rs".to_string(), normalize);
        if path == lib_path {
            matches.insert(CargoTargetSelector::Lib);
        }
    } else if auto_lib && path == "src/lib.rs" {
        matches.insert(CargoTargetSelector::Lib);
    }

    if let Some(bins) = manifest.get("bin").and_then(toml::Value::as_array) {
        for bin in bins.iter().filter_map(toml::Value::as_table) {
            let Some(name) = bin.get("name").and_then(toml::Value::as_str) else {
                continue;
            };
            let bin_path = bin
                .get("path")
                .and_then(toml::Value::as_str)
                .map_or_else(|| format!("src/bin/{name}.rs"), normalize);
            if path == bin_path {
                if !safe_cargo_name(name) {
                    return None;
                }
                matches.insert(CargoTargetSelector::Bin(name.to_string()));
            }
        }
    }
    if auto_bins && path == "src/main.rs" && safe_cargo_name(package) {
        matches.insert(CargoTargetSelector::Bin(package.to_string()));
    }
    if auto_bins && let Some(rest) = path.strip_prefix("src/bin/") {
        if let Some(name) = rest
            .strip_suffix("/main.rs")
            .or_else(|| rest.strip_suffix(".rs"))
            && !name.contains('/')
            && safe_cargo_name(name)
        {
            matches.insert(CargoTargetSelector::Bin(name.to_string()));
        }
    }

    if let Some(tests) = manifest.get("test").and_then(toml::Value::as_array) {
        for test in tests.iter().filter_map(toml::Value::as_table) {
            let Some(name) = test.get("name").and_then(toml::Value::as_str) else {
                continue;
            };
            let test_path = test
                .get("path")
                .and_then(toml::Value::as_str)
                .map_or_else(|| format!("tests/{name}.rs"), normalize);
            if path == test_path {
                if !safe_cargo_name(name) {
                    return None;
                }
                matches.insert(CargoTargetSelector::Test(name.to_string()));
            }
        }
    }
    if auto_tests && let Some(rest) = path.strip_prefix("tests/") {
        if let Some(name) = rest
            .strip_suffix("/main.rs")
            .or_else(|| rest.strip_suffix(".rs"))
            && !name.contains('/')
            && safe_cargo_name(name)
        {
            matches.insert(CargoTargetSelector::Test(name.to_string()));
        }
    }
    if matches.len() != 1 {
        return None;
    }
    matches.into_iter().next()
}

/// Select a single Cargo target only when FAST mode can prove every changed
/// Rust file belongs to that exact target.  Module files are intentionally
/// ambiguous because Cargo metadata does not reveal which roots include them.
fn targeted_cargo_check(workdir: &Path, target_crates: &[String]) -> Option<TargetedCargoCheck> {
    if !fast_mode_enabled() {
        return None;
    }
    let packages = target_crates
        .iter()
        .filter(|package| package.as_str() != "workspace")
        .collect::<BTreeSet<_>>();
    let package = packages.iter().next()?.as_str().to_string();
    if packages.len() != 1 || !safe_cargo_name(&package) {
        return None;
    }

    let files = git_changed_files(workdir)?;
    // A non-Rust input may affect build scripts, generated source, features,
    // or `include_*` data. Without dependency metadata that is ambiguous, so
    // target narrowing fails closed even for apparently harmless side files.
    if files.is_empty() || files.iter().any(|file| !file.ends_with(".rs")) {
        return None;
    }

    let mut selected: Option<(PathBuf, CargoTargetSelector)> = None;
    let mut saw_rust = false;
    for file in files.iter().filter(|file| file.ends_with(".rs")) {
        saw_rust = true;
        let (package_root, package_path) = cargo_manifest_for_file(workdir, file)?;
        let manifest_text = std::fs::read_to_string(package_root.join("Cargo.toml")).ok()?;
        let manifest = toml::from_str::<toml::Value>(&manifest_text).ok()?;
        let manifest_package = manifest
            .get("package")
            .and_then(toml::Value::as_table)
            .and_then(|package| package.get("name"))
            .and_then(toml::Value::as_str)?;
        if manifest_package != package {
            return None;
        }
        let target = manifest_target_for_path(&manifest, manifest_package, &package_path)?;
        match &selected {
            Some((root, prior)) if root != &package_root || prior != &target => return None,
            None => selected = Some((package_root, target)),
            _ => {}
        }
    }
    if !saw_rust {
        return None;
    }
    let (_, target) = selected?;
    let (target_flag, target_name) = target.command_args();
    let mut command = format!("cargo check -p {package} {target_flag}");
    if let Some(target_name) = target_name {
        command.push(' ');
        command.push_str(target_name);
    }
    command.push_str(" --message-format=json");
    Some(TargetedCargoCheck {
        package,
        target,
        command,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CargoCommandFingerprint {
    action: String,
    arguments: Vec<String>,
    tool_arguments: Vec<String>,
}

fn simple_command_tokens(command: &str) -> Option<Vec<&str>> {
    if command.trim().is_empty()
        || command.chars().any(|ch| {
            matches!(
                ch,
                '\n' | '\r' | '\'' | '"' | '`' | '$' | '|' | ';' | '&' | '<' | '>'
            )
        })
    {
        return None;
    }
    Some(command.split_ascii_whitespace().collect())
}

/// Normalize simple Cargo verification commands while deliberately rejecting
/// shell composition.  Only presentation/cache flags are ignored; flags that
/// can change what is compiled remain part of the fingerprint.
fn cargo_command_fingerprint(command: &str) -> Option<CargoCommandFingerprint> {
    let tokens = simple_command_tokens(command)?;
    if tokens.first().copied()? != "cargo" {
        return None;
    }
    let action = tokens.get(1).copied()?;
    if !matches!(action, "check" | "clippy" | "test") {
        return None;
    }

    let mut arguments = Vec::new();
    let mut tool_arguments = Vec::new();
    let mut index = 2;
    let mut after_separator = false;
    while index < tokens.len() {
        let token = tokens[index];
        if after_separator {
            tool_arguments.push(token.to_string());
            index += 1;
            continue;
        }
        if token == "--" {
            after_separator = true;
            index += 1;
            continue;
        }
        if matches!(token, "-q" | "--quiet" | "-v" | "--verbose") {
            index += 1;
            continue;
        }
        if matches!(
            token,
            "--color" | "--message-format" | "-j" | "--jobs" | "--target-dir"
        ) {
            index += 2;
            if index > tokens.len() {
                return None;
            }
            continue;
        }
        if ["--color=", "--message-format=", "--jobs=", "--target-dir="]
            .iter()
            .any(|prefix| token.starts_with(prefix))
        {
            index += 1;
            continue;
        }

        let canonical_value_flag = match token {
            "-p" | "--package" => Some("--package"),
            "--bin" => Some("--bin"),
            "--test" => Some("--test"),
            "--example" => Some("--example"),
            "--bench" => Some("--bench"),
            "--features" => Some("--features"),
            "--profile" => Some("--profile"),
            "--target" => Some("--target"),
            "--manifest-path" => Some("--manifest-path"),
            _ => None,
        };
        if let Some(flag) = canonical_value_flag {
            let value = *tokens.get(index + 1)?;
            let value = if flag == "--features" {
                let mut features = value.split(',').collect::<Vec<_>>();
                features.sort_unstable();
                features.join(",")
            } else {
                value.to_string()
            };
            arguments.push(format!("{flag}={value}"));
            index += 2;
            continue;
        }
        if let Some((flag, value)) = token.split_once('=') {
            let flag = match flag {
                "-p" | "--package" => "--package",
                "--bin" => "--bin",
                "--test" => "--test",
                "--example" => "--example",
                "--bench" => "--bench",
                "--features" => "--features",
                "--profile" => "--profile",
                "--target" => "--target",
                "--manifest-path" => "--manifest-path",
                _ => flag,
            };
            let value = if flag == "--features" {
                let mut features = value.split(',').collect::<Vec<_>>();
                features.sort_unstable();
                features.join(",")
            } else {
                value.to_string()
            };
            arguments.push(format!("{flag}={value}"));
        } else {
            arguments.push(token.to_string());
        }
        index += 1;
    }
    arguments.sort();
    Some(CargoCommandFingerprint {
        action: action.to_string(),
        arguments,
        tool_arguments,
    })
}

fn default_cargo_scope(target_crates: &[String]) -> String {
    let packages = target_crates
        .iter()
        .filter(|package| !package.is_empty() && package.as_str() != "workspace")
        .collect::<BTreeSet<_>>();
    if packages.is_empty() {
        "--workspace".to_string()
    } else {
        packages
            .into_iter()
            .map(|package| format!("-p {package}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn canonical_verify_commands(
    gates_config: &GatesConfig,
    complexity: PlanComplexity,
    target_crates: &[String],
    targeted_check: Option<&TargetedCargoCheck>,
) -> Vec<String> {
    if gates_config.has_custom_rungs() {
        return gates_config
            .effective_rungs()
            .into_iter()
            .filter(|rung| rung.required)
            .map(|rung| rung.command)
            .filter(|command| !command.trim().is_empty())
            .collect();
    }

    let selected = GatePipelineBuilder::selected_rung_labels(gates_config, complexity);
    let scope = default_cargo_scope(target_crates);
    selected
        .iter()
        .filter_map(|rung| match rung.as_str() {
            "compile" => Some(targeted_check.map_or_else(
                || format!("cargo check {scope} --lib --message-format=json"),
                |targeted| targeted.command.clone(),
            )),
            "lint" => Some(format!(
                "cargo clippy {scope} --lib --no-deps -- -D warnings"
            )),
            "test" => Some(format!("cargo test {scope}")),
            _ => None,
        })
        .collect()
}

fn deduplicate_verify_steps(
    task_id: &str,
    verify_steps: Vec<VerifyStep>,
    canonical_commands: &[String],
) -> Vec<VerifyStep> {
    let covered = canonical_commands
        .iter()
        .filter_map(|command| cargo_command_fingerprint(command))
        .collect::<BTreeSet<_>>();
    let total = verify_steps.len();
    let mut retained = Vec::with_capacity(total);
    for step in verify_steps {
        let fingerprint = cargo_command_fingerprint(&step.command);
        let exact_duplicate = fingerprint
            .as_ref()
            .is_some_and(|fingerprint| covered.contains(fingerprint));
        if exact_duplicate {
            info!(
                task_id = %task_id,
                phase = %step.phase,
                command = %step.command,
                reason = "semantic-duplicate",
                "skipping redundant authored verify command"
            );
            continue;
        }
        retained.push(step);
    }
    if retained.len() != total {
        info!(
            task_id = %task_id,
            original_steps = total,
            retained_steps = retained.len(),
            "verify command deduplication complete"
        );
    }
    retained
}

fn with_targeted_compile_rung(
    gates_config: &GatesConfig,
    complexity: PlanComplexity,
    targeted: Option<&TargetedCargoCheck>,
    timeout_secs: u64,
) -> GatesConfig {
    let Some(targeted) = targeted else {
        return gates_config.clone();
    };
    if gates_config.has_custom_rungs() {
        return gates_config.clone();
    }
    let mut optimized = gates_config.clone();
    optimized.custom_rungs = GatePipelineBuilder::selected_rung_labels(gates_config, complexity)
        .into_iter()
        .map(|name| GateRungConfig {
            command: if name == "compile" {
                targeted.command.clone()
            } else {
                String::new()
            },
            name,
            timeout_secs: timeout_secs.max(1),
            required: true,
            parallel_with: Vec::new(),
        })
        .collect();
    optimized
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GateInputSnapshot(String, [u8; 32], bool);
const MAX_UNTRACKED_FILES: usize = 1024;
const MAX_UNTRACKED_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_GATE_INPUT_BYTES: u64 = 32 * 1024 * 1024;
fn hash_part(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}
#[cfg(unix)]
fn metadata_unchanged(before: &std::fs::Metadata, after: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    before.file_type() == after.file_type()
        && before.len() == after.len()
        && before.modified().ok() == after.modified().ok()
        && before.dev() == after.dev()
        && before.ino() == after.ino()
        && before.mode() == after.mode()
}

#[cfg(unix)]
fn metadata_mode(metadata: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    metadata.mode()
}

#[cfg(not(unix))]
fn metadata_mode(_metadata: &std::fs::Metadata) -> u32 {
    0
}
fn gate_input_snapshot_blocking(workdir: &Path) -> Result<GateInputSnapshot, String> {
    #[cfg(not(unix))]
    return Err("stable gate input identity is unavailable on this platform".into());
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(workdir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(|error| error.to_string())?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    };
    let base_commit = String::from_utf8_lossy(&git(&["rev-parse", "HEAD"])?)
        .trim()
        .to_string();
    let diff = git(&["diff", "--binary", "HEAD", "--"])?;
    if diff.len() as u64 > MAX_GATE_INPUT_BYTES {
        return Err("tracked diff exceeds gate input byte limit".into());
    }
    let status = git(&[
        "status",
        "--porcelain=v1",
        "-z",
        "--ignored=matching",
        "-uall",
    ])?;
    crate::orchestrator::worktree::validate_workspace_file_kinds(workdir, &status)
        .map_err(|error| error.to_string())?;
    let untracked = git(&["ls-files", "--others", "--exclude-standard", "-z"])?;
    let mut hasher = Sha256::new();
    hash_part(&mut hasher, base_commit.as_bytes());
    hash_part(&mut hasher, &diff);
    let mut total_bytes = diff.len() as u64;
    for (index, raw_path) in untracked
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .enumerate()
    {
        if index >= MAX_UNTRACKED_FILES {
            return Err("untracked file count exceeds input limit".into());
        }
        let relative = std::str::from_utf8(raw_path).map_err(|error| error.to_string())?;
        let path = workdir.join(relative);
        let before = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        hash_part(&mut hasher, raw_path);
        hasher.update(metadata_mode(&before).to_le_bytes());
        if before.file_type().is_symlink() {
            let target_path = std::fs::read_link(&path).map_err(|error| error.to_string())?;
            let target = target_path.as_os_str().as_encoded_bytes();
            total_bytes = total_bytes.saturating_add(target.len() as u64);
            if target.len() as u64 > MAX_UNTRACKED_FILE_BYTES || total_bytes > MAX_GATE_INPUT_BYTES
            {
                return Err("untracked symlink exceeds input limit".into());
            }
            hasher.update([b'l']);
            hash_part(&mut hasher, target);
            if std::fs::read_link(&path).ok().as_ref() != Some(&target_path) {
                return Err("untracked symlink changed while hashing".into());
            }
        } else if before.is_file() {
            if before.len() > MAX_UNTRACKED_FILE_BYTES
                || total_bytes.saturating_add(before.len()) > MAX_GATE_INPUT_BYTES
            {
                return Err("untracked file exceeds input limit".into());
            }
            let mut options = OpenOptions::new();
            options.read(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
            }
            let mut file = options.open(&path).map_err(|error| error.to_string())?;
            let opened = file.metadata().map_err(|error| error.to_string())?;
            if !opened.is_file() || !metadata_unchanged(&before, &opened) {
                return Err("untracked file changed before hashing".into());
            }
            hasher.update([b'f']);
            hasher.update(before.len().to_le_bytes());
            let read_bytes = std::io::copy(&mut (&mut file).take(before.len() + 1), &mut hasher)
                .map_err(|error| error.to_string())?;
            let after = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
            if read_bytes != before.len() || !metadata_unchanged(&before, &after) {
                return Err("untracked file changed while hashing".into());
            }
            total_bytes += read_bytes;
        } else {
            return Err("untracked path is not a regular file or symlink".into());
        }
    }
    let owned_diff: [u8; 32] = hasher.finalize().into();
    let has_owned_diff = !diff.is_empty() || !untracked.is_empty();
    Ok(GateInputSnapshot(base_commit, owned_diff, has_owned_diff))
}
async fn gate_input_snapshot(workdir: PathBuf) -> Result<GateInputSnapshot, String> {
    tokio::task::spawn_blocking(move || gate_input_snapshot_blocking(&workdir))
        .await
        .map_err(|error| error.to_string())?
}

/// Stable identity of a task worktree's base commit plus all tracked and
/// untracked owned bytes. Reflex promotion reuses the same attribution proof
/// as the gate so an isolated replay can be compared with the Premium source
/// attempt without inventing a weaker diff format.
pub(super) async fn reflex_input_fingerprint(
    workdir: PathBuf,
) -> Result<(String, [u8; 32], bool), String> {
    let GateInputSnapshot(base, digest, has_owned_diff) = gate_input_snapshot(workdir).await?;
    Ok((base, digest, has_owned_diff))
}
async fn accepted_input_snapshot(
    workdir: PathBuf,
    expected_oid: &str,
) -> Result<GateInputSnapshot, String> {
    let snapshot = gate_input_snapshot(workdir).await?;
    (snapshot.0 == expected_oid && !snapshot.2)
        .then_some(snapshot)
        .ok_or_else(|| "accepted plan input differs from immutable commit".into())
}
fn raw_gate_name(name: &str) -> &str {
    name.strip_prefix("baseline+owned:")
        .or_else(|| name.strip_prefix("baseline:"))
        .or_else(|| name.strip_prefix("owned-diff:"))
        .or_else(|| name.strip_prefix("unattributed:"))
        .unwrap_or(name)
}
fn gate_failure_input(
    kind: GateCompletionKind,
    before: &GateInputSnapshot,
    baseline_failed_gates: Option<&[String]>,
    gate: &str,
) -> &'static str {
    match (kind, before.2, baseline_failed_gates) {
        (GateCompletionKind::Preflight, _, _) | (GateCompletionKind::Gate, false, _) => "baseline",
        (GateCompletionKind::Gate, true, Some(failures))
            if failures.iter().any(|name| name == raw_gate_name(gate)) =>
        {
            "baseline+owned"
        }
        (GateCompletionKind::Gate, true, Some(_)) => "owned-diff",
        (GateCompletionKind::Gate, true, None) => "unattributed",
        (GateCompletionKind::PlanVerify, _, _) => "accepted-plan",
        (GateCompletionKind::Merge, _, _) => "post-merge",
    }
}
macro_rules! proof_failure {
    ($gate:expr, $reason:expr, $digest:expr $(,)?) => {
        Verdict::fail($gate, $reason).with_error_digest($digest)
    };
}
/// Spawn a gate rung as a background task. Sends `GateCompletion` when done.
pub fn spawn_gate(
    effect: GateEffectRef,
    plan_id: String,
    task_id: String,
    rung: u32,
    workdir: PathBuf,
    gates_config: GatesConfig,
    complexity: PlanComplexity,
    verify_steps: Vec<VerifyStep>,
    baseline_failed_gates: Option<Vec<String>>,
    timeout_secs: u64,
    gate_tx: mpsc::Sender<GateCompletion>,
    gate_sem: Arc<Semaphore>,
    target_crates: Vec<String>,
    verdict_publisher: Option<VerdictPublisher>,
    task_context: Option<GateTaskContext>,
    telemetry_sink: Option<Arc<dyn TelemetryEventSink>>,
    main_target_dir: Option<PathBuf>,
) -> (JoinHandle<()>, oneshot::Sender<()>) {
    let (start_tx, start_rx) = oneshot::channel();
    let handle = tokio::spawn(async move {
        if start_rx.await.is_err() {
            return;
        }
        let failure_effect = effect.clone();
        let failure_plan = plan_id.clone();
        let failure_task = task_id.clone();
        let worker = AssertUnwindSafe(async move {
            let t_wait = Instant::now();
            let _permit = gate_sem
                .acquire_owned()
                .await
                .map_err(|_| "gate semaphore closed before acquisition".to_string())?;
            let wait_ms = t_wait.elapsed().as_millis() as u64;
            if wait_ms > 10 {
                info!(plan_id = %plan_id, task_id = %task_id, rung, wait_ms,
                    "gate semaphore acquired");
            }
            Ok::<_, String>(
                run_gate_once(
                    effect,
                    plan_id,
                    task_id,
                    rung,
                    workdir,
                    gates_config,
                    complexity,
                    verify_steps,
                    baseline_failed_gates,
                    timeout_secs,
                    target_crates,
                    verdict_publisher,
                    task_context,
                    telemetry_sink,
                    main_target_dir,
                )
                .await,
            )
        })
        .catch_unwind()
        .await;
        let completion = match worker {
            Ok(Ok(completion)) => completion,
            Ok(Err(message)) => {
                failed_gate_completion(failure_effect, failure_plan, failure_task, rung, message)
            }
            Err(_) => failed_gate_completion(
                failure_effect,
                failure_plan,
                failure_task,
                rung,
                "gate producer panicked".to_string(),
            ),
        };

        if let Err(e) = gate_tx.send(completion).await {
            error!(err = %e, "failed to send gate completion — channel closed");
            return;
        }
    });
    (handle, start_tx)
}

fn failed_gate_completion(
    effect: GateEffectRef,
    plan_id: String,
    task_id: String,
    rung: u32,
    message: String,
) -> GateCompletion {
    GateCompletion {
        kind: effect.kind,
        attempt: Some(effect.attempt.clone()),
        effect: Some(effect),
        plan_id,
        task_id,
        rung,
        passed: false,
        failure_kind: Some(RunnerFailureKind::Resource),
        verdicts: Vec::new(),
        output: message,
        duration_ms: 0,
        selected_rungs: Vec::new(),
    }
}

/// Outcome of a single `attempt_auto_fix` call — used for tracking and telemetry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutoFixOutcome {
    /// Name of the gate that triggered the auto-fix attempt.
    pub gate_name: String,
    /// Whether the classifier judged the failure as a `cargo fix` candidate.
    pub was_candidate: bool,
    /// Whether `cargo fix` (or `cargo clippy --fix`) exited successfully.
    pub fix_applied: bool,
    /// Whether the gate was re-run after the fix and the retry passed.
    pub gate_passed_after_fix: bool,
    /// The fix command that was executed, e.g. `"cargo fix --allow-dirty"`.
    pub command: Option<String>,
}

impl AutoFixOutcome {
    /// Construct a "not a candidate" outcome — no fix was attempted.
    fn not_candidate(gate_name: impl Into<String>) -> Self {
        Self {
            gate_name: gate_name.into(),
            was_candidate: false,
            fix_applied: false,
            gate_passed_after_fix: false,
            command: None,
        }
    }
}

/// Attempt to auto-fix compile or clippy gate failures using `cargo fix`.
///
/// For "compile" gates: runs `cargo fix --allow-dirty` then `cargo fmt`.
/// For "clippy" gates: runs `cargo clippy --fix --allow-dirty`.
///
/// Returns `Ok(AutoFixOutcome)` describing what happened. Returns `Err` only
/// on internal failures (spawn error, etc).
///
/// Per spec: never use `--allow-staged`, only `--allow-dirty`.
pub async fn attempt_auto_fix(
    workdir: &Path,
    gate_name: &str,
    error_output: &str,
) -> Result<AutoFixOutcome, String> {
    let classification = roko_gate::classify_gate_failure(gate_name, error_output);
    if !classification.cargo_fix_candidate {
        return Ok(AutoFixOutcome::not_candidate(gate_name));
    }

    let raw = raw_gate_name(gate_name);
    let (program, args): (&str, &[&str]) = if raw.starts_with("compile") {
        ("cargo", &["fix", "--allow-dirty"])
    } else if raw.starts_with("clippy") {
        ("cargo", &["clippy", "--fix", "--allow-dirty"])
    } else {
        return Ok(AutoFixOutcome::not_candidate(gate_name));
    };

    let command_str = format!("{program} {}", args.join(" "));

    info!(
        gate = %gate_name,
        command = %command_str,
        "attempting cargo auto-fix before agent retry"
    );

    let mut fix_cmd = tokio::process::Command::new(program);
    fix_cmd
        .args(args)
        .current_dir(workdir)
        .env("CARGO_BUILD_JOBS", cargo_build_jobs());
    if sccache_available() {
        fix_cmd.env("RUSTC_WRAPPER", "sccache");
    }
    let fix_status = fix_cmd
        .output()
        .await
        .map_err(|e| format!("failed to spawn {program}: {e}"))?;

    if !fix_status.status.success() {
        info!(
            gate = %gate_name,
            exit_code = ?fix_status.status.code(),
            "cargo auto-fix exited non-zero — falling through to agent"
        );
        return Ok(AutoFixOutcome {
            gate_name: gate_name.to_string(),
            was_candidate: true,
            fix_applied: false,
            gate_passed_after_fix: false,
            command: Some(command_str),
        });
    }

    // For compile fixes, also run cargo fmt to keep formatting clean.
    if raw.starts_with("compile") {
        let _ = tokio::process::Command::new("cargo")
            .env("CARGO_BUILD_JOBS", cargo_build_jobs())
            .args(["fmt"])
            .current_dir(workdir)
            .output()
            .await;
    }

    info!(gate = %gate_name, "cargo auto-fix applied — will retry gate");
    Ok(AutoFixOutcome {
        gate_name: gate_name.to_string(),
        was_candidate: true,
        fix_applied: true,
        gate_passed_after_fix: false, // updated by caller after retry
        command: Some(command_str),
    })
}

/// Run a gate rung to completion and return its summary.
pub async fn run_gate_once(
    effect: GateEffectRef,
    plan_id: String,
    task_id: String,
    rung: u32,
    workdir: PathBuf,
    gates_config: GatesConfig,
    complexity: PlanComplexity,
    verify_steps: Vec<VerifyStep>,
    baseline_failed_gates: Option<Vec<String>>,
    timeout_secs: u64,
    target_crates: Vec<String>,
    verdict_publisher: Option<VerdictPublisher>,
    task_context: Option<GateTaskContext>,
    telemetry_sink: Option<Arc<dyn TelemetryEventSink>>,
    main_target_dir: Option<PathBuf>,
) -> GateCompletion {
    let start = Instant::now();
    let signal = gate_signal(
        &plan_id,
        &task_id,
        rung,
        &workdir,
        &target_crates,
        main_target_dir.as_deref(),
    );
    let ctx = roko_core::Context::now();
    let limit = Duration::from_secs(timeout_secs.max(1));

    info!(
        plan_id = %plan_id,
        task_id = %task_id,
        rung,
        timeout_secs,
        verify_step_count = verify_steps.len(),
        "gate rung starting"
    );

    // E05-T04: Compute the labels of the canonical rungs that were selected
    // for this gate run BEFORE building the pipeline, so we can thread them
    // through the GateCompletion for callers.
    let selected_rungs = GatePipelineBuilder::selected_rung_labels(&gates_config, complexity);

    let fast_mode = fast_mode_enabled();
    let task_verify_only = task_verify_only_enabled();
    let task_verify_contract_error =
        fast_task_verify_contract_error(fast_mode, task_verify_only, verify_steps.len());
    let targeted_check = (!task_verify_only && !gates_config.has_custom_rungs())
        .then(|| targeted_cargo_check(&workdir, &target_crates))
        .flatten();
    if let Some(targeted) = targeted_check.as_ref() {
        info!(
            plan_id = %plan_id,
            task_id = %task_id,
            package = %targeted.package,
            target = ?targeted.target,
            command = %targeted.command,
            "FAST mode selected target-aware canonical compile"
        );
    }
    let canonical_commands = if task_verify_only {
        Vec::new()
    } else {
        canonical_verify_commands(
            &gates_config,
            complexity,
            &target_crates,
            targeted_check.as_ref(),
        )
    };
    let verify_steps = if fast_mode {
        deduplicate_verify_steps(&task_id, verify_steps, &canonical_commands)
    } else {
        verify_steps
    };
    let gates_config = with_targeted_compile_rung(
        &gates_config,
        complexity,
        targeted_check.as_ref(),
        timeout_secs,
    );

    // E45-T02: Clone verify_steps so we can use them for the auto-fix retry
    // pass if the first run fails and cargo fix is applicable.
    let verify_steps_for_retry = verify_steps.clone();

    let workdir_for_run = workdir.clone();
    let run = async {
        if let Some(reason) = task_verify_contract_error.as_deref() {
            return vec![proof_failure!(
                "task-verify:contract",
                reason.to_string(),
                "invalid FAST task-owned verification contract",
            )];
        }

        let inputs = build_rung_execution_inputs(&target_crates, task_context.as_ref());
        let config = build_rung_execution_config(
            &workdir_for_run,
            timeout_secs,
            &verify_steps,
            verdict_publisher.clone(),
        );
        let pipeline = if gates_config.has_custom_rungs() && targeted_check.is_none() {
            GatePipelineBuilder::from_config(&gates_config, complexity)
        } else {
            GatePipelineBuilder::from_config_with_execution(
                &gates_config,
                complexity,
                inputs,
                config,
            )
        };

        let mut verdicts = if task_verify_only {
            Vec::new()
        } else {
            vec![pipeline.verify(&signal, &ctx).await]
        };
        verdicts.extend(run_verify_steps(&signal, &ctx, &task_id, verify_steps).await);
        verdicts
    };

    let checked = async {
        let before = gate_input_snapshot(workdir.clone()).await?;
        let mut verdicts = run.await;
        let after = gate_input_snapshot(workdir.clone()).await?;

        // E45-T02: If the first run produced any failures, attempt cargo auto-fix
        // before we finalise. If the fix applied cleanly, rerun the gate pipeline
        // once and replace the verdicts so the caller sees a pass instead.
        // Gated on `gates_config.cargo_fix_enabled` (default: true).
        let first_run_failed = verdicts.iter().any(|v| !v.passed && !v.skipped);
        if first_run_failed && before == after && gates_config.cargo_fix_enabled {
            let first_output = render_output(&verdicts);
            // Find the first failing gate name to drive the fix heuristic.
            let failing_gate = verdicts
                .iter()
                .find(|v| !v.passed && !v.skipped)
                .map(|v| v.gate.as_str())
                .unwrap_or("compile");
            match attempt_auto_fix(&workdir, failing_gate, &first_output).await {
                Ok(mut outcome) if outcome.fix_applied => {
                    // Fix applied — rerun the pipeline with a fresh snapshot pair.
                    let before_retry = gate_input_snapshot(workdir.clone()).await?;
                    let inputs_retry =
                        build_rung_execution_inputs(&target_crates, task_context.as_ref());
                    let config_retry = build_rung_execution_config(
                        &workdir,
                        timeout_secs,
                        &verify_steps_for_retry,
                        verdict_publisher.clone(),
                    );
                    let pipeline_retry =
                        if gates_config.has_custom_rungs() && targeted_check.is_none() {
                            GatePipelineBuilder::from_config(&gates_config, complexity)
                        } else {
                            GatePipelineBuilder::from_config_with_execution(
                                &gates_config,
                                complexity,
                                inputs_retry,
                                config_retry,
                            )
                        };
                    let mut retry_verdicts = if task_verify_only {
                        Vec::new()
                    } else {
                        vec![pipeline_retry.verify(&signal, &ctx).await]
                    };
                    retry_verdicts.extend(
                        run_verify_steps(&signal, &ctx, &task_id, verify_steps_for_retry).await,
                    );
                    let after_retry = gate_input_snapshot(workdir.clone()).await?;
                    if before_retry == after_retry {
                        // Immutable input confirmed after retry — use retry verdicts.
                        let retry_passed = retry_verdicts.iter().all(|v| v.passed || v.skipped);
                        outcome.gate_passed_after_fix = retry_passed;
                        info!(
                            gate = %outcome.gate_name,
                            command = ?outcome.command,
                            gate_passed_after_fix = outcome.gate_passed_after_fix,
                            "auto-fix retry complete"
                        );
                        verdicts = retry_verdicts;
                    }
                }
                Ok(_) | Err(_) => {
                    // No fix applied, not a candidate, or fix failed — fall through to agent.
                }
            }
        }

        Ok::<_, String>((before, after, verdicts))
    };
    let (input_before, mut verdicts) = match timeout(limit, checked).await {
        Ok(Ok((before, after, mut verdicts))) => {
            if before != after {
                verdicts.push(proof_failure!(
                    "unattributed:immutable-input",
                    format!(
                        "gate input changed during verification (base {} -> {})",
                        before.0, after.0
                    ),
                    "gate input mutation invalidates attribution",
                ));
            }
            (Some(before), verdicts)
        }
        Ok(Err(error)) => (
            None,
            vec![proof_failure!(
                "unattributed:input-snapshot",
                format!("could not prove immutable gate input: {error}"),
                "gate input identity unavailable",
            )],
        ),
        Err(_) => (
            None,
            vec![proof_failure!(
                format!("unattributed:gate-timeout:rung-{rung}"),
                format!("gate timed out after {timeout_secs}s"),
                format!("timeout: gate rung {rung} exceeded {timeout_secs}s"),
            )],
        ),
    };
    if let Some(before) = input_before.as_ref() {
        for verdict in verdicts
            .iter_mut()
            .filter(|verdict| !verdict.passed && !verdict.gate.starts_with("unattributed:"))
        {
            let input = gate_failure_input(
                effect.kind,
                before,
                baseline_failed_gates.as_deref(),
                &verdict.gate,
            );
            verdict.gate = format!("{input}:{}", verdict.gate);
            verdict.reason = format!("{input} failure: {}", verdict.reason);
        }
    }
    let duration_ms = start.elapsed().as_millis() as u64;

    // E05-T03: Skipped verdicts are neutral — filter them out of the
    // pass/fail decision. A gate run whose only verdicts are skipped stubs
    // is considered passed (no real gate disagreed).
    let real_verdicts: Vec<&Verdict> = verdicts.iter().filter(|v| !v.skipped).collect();
    let passed = real_verdicts.iter().all(|v| v.passed);
    let all_skipped = real_verdicts.is_empty() && !verdicts.is_empty();
    let output = render_output(&verdicts);
    let failure_kind = (!passed && !all_skipped).then(|| classify_failure_kind(&verdicts, &output));

    if let Some(sink) = telemetry_sink.as_ref() {
        let ancestry = [
            LensScope::Cell(task_id.clone()),
            LensScope::Graph(plan_id.clone()),
        ];
        for verdict in &real_verdicts {
            let verified = ObservableEvent::SignalVerified(signal.id.to_hex(), (*verdict).clone());
            if let Err(error) = sink.emit(&verified, &ancestry).await {
                error!(%error, "gate SignalVerified telemetry delivery failed");
            }
            if effect.kind == GateCompletionKind::Preflight {
                let pre_result = ObservableEvent::VerifyPreResult {
                    block: task_id.clone(),
                    verdict: (*verdict).clone(),
                    evidence: verdict.error_digest.iter().cloned().collect(),
                };
                if let Err(error) = sink.emit(&pre_result, &ancestry).await {
                    error!(%error, "gate VerifyPreResult telemetry delivery failed");
                }
            }
        }
    }

    // E05-T08: Publish non-skipped verdicts through VerdictPublisher as
    // Kind::GateVerdict signals. The publisher callback (set up by the
    // caller in event_loop.rs) graduates each Pulse to a Signal and
    // appends it to signals.jsonl.
    if let Some(ref publisher) = verdict_publisher {
        let real: Vec<Verdict> = verdicts.iter().filter(|v| !v.skipped).cloned().collect();
        if !real.is_empty() {
            publisher.publish_all(&real, Some(rung));
        }
    }

    let summaries: Vec<GateVerdictSummary> = verdicts
        .iter()
        .map(|v| {
            // E05-T04: Resolve each verdict's gate name to its canonical rung
            // index so callers can observe per-rung EMA thresholds. Strip
            // attribution prefixes (baseline:, owned-diff:, etc.) before lookup.
            let raw_name = raw_gate_name(&v.gate);
            let rung_index = rung_for_gate_name(raw_name).map(|r| r.as_index());
            GateVerdictSummary {
                gate_name: v.gate.clone(),
                passed: v.passed,
                skipped: v.skipped,
                summary: v.reason.clone(),
                error_digest: v.error_digest.clone(),
                failure_kind: (!v.passed && !v.skipped)
                    .then(|| classify_failure_kind(std::slice::from_ref(v), &v.reason)),
                rung_index,
            }
        })
        .collect();

    let preview_limit = if passed { 500 } else { 4_000 };
    let output_preview: String = output.chars().take(preview_limit).collect();
    let verdict_names: Vec<&str> = summaries.iter().map(|v| v.gate_name.as_str()).collect();
    info!(
        plan_id = %plan_id,
        task_id = %task_id,
        rung,
        passed,
        duration_ms,
        verdict_count = summaries.len(),
        verdicts = ?verdict_names,
        output_preview = %output_preview,
        "gate completed"
    );

    GateCompletion {
        kind: effect.kind,
        attempt: Some(effect.attempt.clone()),
        effect: Some(effect),
        plan_id,
        task_id,
        rung,
        passed,
        failure_kind,
        verdicts: summaries,
        output,
        duration_ms,
        selected_rungs,
    }
}

/// Spawn plan-level verify steps as a background task.
pub fn spawn_plan_verify(
    effect: GateEffectRef,
    plan_id: String,
    workdir: PathBuf,
    expected_oid: String,
    verify_steps: Vec<(String, Vec<VerifyStep>)>,
    timeout_secs: u64,
    gate_tx: mpsc::Sender<GateCompletion>,
    gate_sem: Arc<Semaphore>,
    main_target_dir: Option<PathBuf>,
) -> (JoinHandle<()>, oneshot::Sender<()>) {
    let (start_tx, start_rx) = oneshot::channel();
    let handle = tokio::spawn(async move {
        if start_rx.await.is_err() {
            return;
        }
        let failure_effect = effect.clone();
        let failure_plan = plan_id.clone();
        let worker = AssertUnwindSafe(async move {
            let t_wait = Instant::now();
            let _permit = gate_sem
                .acquire_owned()
                .await
                .map_err(|_| "plan verify semaphore closed before acquisition".to_string())?;
            let wait_ms = t_wait.elapsed().as_millis() as u64;
            if wait_ms > 10 {
                info!(
                    plan_id = %plan_id,
                    wait_ms,
                    "plan verify semaphore acquired"
                );
            }
            let start = Instant::now();
            let ctx = roko_core::Context::now();
            let limit = Duration::from_secs(timeout_secs.max(1));
            let plan_id_for_run = plan_id.clone();
            let workdir_for_run = workdir.clone();

            let run = async move {
                let before =
                    match accepted_input_snapshot(workdir_for_run.clone(), &expected_oid).await {
                        Ok(snapshot) => snapshot,
                        Err(error) => return vec![Verdict::fail("accepted-plan:input", error)],
                    };
                let mut all = Vec::new();
                for (task_id, steps) in verify_steps {
                    let signal = gate_signal(
                        &plan_id_for_run,
                        &task_id,
                        RUNG_PLAN_VERIFY,
                        &workdir_for_run,
                        &[], // plan-level verify runs workspace-wide
                        main_target_dir.as_deref(),
                    );
                    all.extend(run_verify_steps(&signal, &ctx, &task_id, steps).await);
                }
                if accepted_input_snapshot(workdir_for_run, &expected_oid).await != Ok(before) {
                    all.push(Verdict::fail(
                        "accepted-plan:immutable-input",
                        "accepted plan input changed during verification",
                    ));
                }
                all
            };

            let verdicts = match timeout(limit, run).await {
                Ok(verdicts) => verdicts,
                Err(_) => vec![
                    Verdict::fail(
                        "plan-verify-timeout",
                        format!("plan verify timed out after {timeout_secs}s"),
                    )
                    .with_error_digest(format!("timeout: plan verify exceeded {timeout_secs}s")),
                ],
            };
            let duration_ms = start.elapsed().as_millis() as u64;
            let real_verdicts: Vec<&Verdict> = verdicts.iter().filter(|v| !v.skipped).collect();
            let passed = real_verdicts.iter().all(|v| v.passed);
            let output = render_output(&verdicts);
            let failure_kind = (!passed).then(|| classify_failure_kind(&verdicts, &output));
            let summaries = verdicts
                .iter()
                .map(|v| GateVerdictSummary {
                    gate_name: v.gate.clone(),
                    passed: v.passed,
                    skipped: v.skipped,
                    summary: v.reason.clone(),
                    error_digest: v.error_digest.clone(),
                    failure_kind: (!v.passed && !v.skipped)
                        .then(|| classify_failure_kind(std::slice::from_ref(v), &v.reason)),
                    rung_index: None, // plan-verify steps are not canonical rungs
                })
                .collect();

            info!(
                plan_id = %plan_id,
                passed,
                duration_ms,
                "plan verify completed"
            );

            Ok::<_, String>(GateCompletion {
                kind: GateCompletionKind::PlanVerify,
                attempt: Some(effect.attempt.clone()),
                effect: Some(effect),
                plan_id,
                task_id: "plan-verify".to_string(),
                rung: RUNG_PLAN_VERIFY,
                passed,
                failure_kind,
                verdicts: summaries,
                output,
                duration_ms,
                selected_rungs: Vec::new(), // sentinel: no canonical rungs for plan-verify
            })
        })
        .catch_unwind()
        .await;
        let completion = match worker {
            Ok(Ok(completion)) => completion,
            Ok(Err(message)) => failed_gate_completion(
                failure_effect,
                failure_plan,
                "plan-verify".to_string(),
                RUNG_PLAN_VERIFY,
                message,
            ),
            Err(_) => failed_gate_completion(
                failure_effect,
                failure_plan,
                "plan-verify".to_string(),
                RUNG_PLAN_VERIFY,
                "plan verify producer panicked".to_string(),
            ),
        };

        if let Err(e) = gate_tx.send(completion).await {
            error!(err = %e, "failed to send plan verify completion — channel closed");
        }
    });
    (handle, start_tx)
}

/// Build enriched [`RungExecutionInputs`] from available task context.
///
/// E05-T05: Populates real signal fields from the task definition so that
/// advanced gate rungs (Symbol, FactCheck, LlmJudge) receive genuine inputs
/// instead of defaulting to `None` and immediately returning skipped stubs.
///
/// - `symbol_signal`: Built from task context symbols as a `SymbolManifest`.
/// - `fact_check_signal`: Built from task acceptance criteria as text.
/// - `llm_judge_signal`: Built from task description as a `JudgePayload`
///    (diff is not available synchronously; the gate degrades gracefully
///    with an empty diff).
/// - `code_intel_hints`: Target crate names for focused verification.
fn build_rung_execution_inputs(
    target_crates: &[String],
    task_ctx: Option<&GateTaskContext>,
) -> RungExecutionInputs {
    let code_intel_hints = target_crates.to_vec();

    let Some(ctx) = task_ctx else {
        return RungExecutionInputs {
            code_intel_hints,
            ..Default::default()
        };
    };

    // Build SymbolManifest signal from task context symbols (rung 3).
    let symbol_signal = if ctx.symbols.is_empty() {
        None
    } else {
        let mut manifest = SymbolManifest::new(&ctx.plan_id);
        for sym in &ctx.symbols {
            let (module_path, name) = match sym.rsplit_once("::") {
                Some((module, name)) => (module.to_string(), name.to_string()),
                None => (String::new(), sym.clone()),
            };
            manifest.expectations.push(SymbolExpectation {
                name,
                kind: SymbolKind::Struct,
                visibility: Visibility::Pub,
                module_path,
                signature: None,
            });
        }
        Some(
            SignalBuilder::new(Kind::Task)
                .body(Body::from_json(&manifest).unwrap_or_else(|_| Body::empty()))
                .provenance(Provenance::trusted("runner"))
                .build(),
        )
    };

    // Build fact-check signal from acceptance criteria (rung 5).
    let fact_check_signal = if ctx.acceptance.is_empty() {
        None
    } else {
        let claims = ctx.acceptance.join("\n");
        Some(
            SignalBuilder::new(Kind::Task)
                .body(Body::text(&claims))
                .provenance(Provenance::trusted("runner"))
                .build(),
        )
    };

    // Build LLM judge signal from task description (rung 6).
    // The diff is left empty here because we cannot run `git diff`
    // synchronously in this context. The LlmJudgeGate degrades
    // gracefully when the diff is empty (judges description only).
    let llm_judge_signal = {
        let task_description = ctx
            .task_description
            .as_deref()
            .unwrap_or(ctx.task_title.as_str());
        if task_description.is_empty() {
            None
        } else {
            let payload = JudgePayload {
                task_description: task_description.to_string(),
                diff: String::new(),
            };
            Some(
                SignalBuilder::new(Kind::Task)
                    .body(Body::from_json(&payload).unwrap_or_else(|_| Body::empty()))
                    .provenance(Provenance::trusted("runner"))
                    .build(),
            )
        }
    };

    RungExecutionInputs {
        symbol_signal,
        fact_check_signal,
        llm_judge_signal,
        code_intel_hints,
    }
}

/// Build enriched [`RungExecutionConfig`] from task workdir and verify steps.
///
/// E05-T05: Populates `source_roots`, `timeout_ms`, `integration_test_pattern`,
/// `integration_build_system`, and `generated_test_artifacts` from available
/// task context. Oracle fields (fact-check, llm-judge) remain `None` — the
/// rung dispatch fails closed with explicit skipped/not-wired verdicts when
/// required oracles are absent, rather than producing silent passes.
fn build_rung_execution_config(
    workdir: &Path,
    timeout_secs: u64,
    verify_steps: &[VerifyStep],
    verdict_publisher: Option<VerdictPublisher>,
) -> RungExecutionConfig {
    let integration_test_pattern = verify_steps
        .iter()
        .find(|v| v.phase.eq_ignore_ascii_case("integration"))
        .map(|step| step.command.clone());

    let integration_build_system = if integration_test_pattern.is_some() {
        Some(roko_gate::BuildSystem::detect(workdir))
    } else {
        None
    };

    // E05-T05: Wire generated_test_artifacts when the workdir contains
    // generated test files. This allows the GeneratedTest rung to run
    // real tests instead of returning a skipped stub.
    let generated_test_artifacts: Option<Arc<dyn GeneratedArtifactStore>> = {
        let store = FsGeneratedArtifactStore::new(workdir.to_path_buf());
        if store.matching_entries("generated-tests/gen_").is_empty() {
            None
        } else {
            Some(Arc::new(store))
        }
    };

    RungExecutionConfig {
        source_roots: Some(vec![workdir.to_path_buf()]),
        timeout_ms: Some(timeout_secs.saturating_mul(1000)),
        integration_test_pattern,
        integration_build_system,
        generated_test_artifacts,
        verdict_publisher,
        ..Default::default()
    }
}

fn gate_signal(
    plan_id: &str,
    task_id: &str,
    rung: u32,
    workdir: &std::path::Path,
    target_crates: &[String],
    main_target_dir: Option<&Path>,
) -> Signal {
    let attempt_sentinel = RokoLayout::for_project(workdir)
        .gate_attempts_dir()
        .join(format!(
            "{}-{}-{rung}.seen",
            sanitize_gate_env_segment(plan_id),
            sanitize_gate_env_segment(task_id)
        ));
    let mut payload = GatePayload::in_dir(workdir)
        .with_label(format!("{plan_id}:{task_id}:rung-{rung}"))
        .with_target_crates(target_crates.to_vec())
        .with_env("ROKO_GATE_PLAN_ID", plan_id)
        .with_env("ROKO_GATE_TASK_ID", task_id)
        .with_env("ROKO_GATE_RUNG", rung.to_string())
        .with_env(
            "ROKO_GATE_ATTEMPT_SENTINEL",
            attempt_sentinel.to_string_lossy().to_string(),
        )
        // Limit build parallelism to nproc/2 to prevent CPU exhaustion
        // when multiple agents run gate checks concurrently (#206).
        .with_env("CARGO_BUILD_JOBS", cargo_build_jobs());

    // Shared FAST-mode targets rely on Cargo's incremental artifacts. Rust
    // incremental crates are not sccache-cacheable, and combining the two was
    // producing wrapper overhead with zero hits. Keep the existing sccache
    // behavior for isolated/normal runs.
    if sccache_available() && !(fast_mode_enabled() && main_target_dir.is_some()) {
        payload = payload.with_env("RUSTC_WRAPPER", "sccache");
    }

    // Share the main workspace build cache with worktree gate commands so
    // that `cargo check`/`cargo clippy`/`cargo test` inside a task worktree
    // reuse incremental artifacts instead of rebuilding all crates from
    // scratch.
    if let Some(target_dir) = main_target_dir {
        payload = payload.with_target_dir(target_dir);
    }

    SignalBuilder::new(Kind::Task)
        .body(Body::from_json(&payload).unwrap_or_else(|_| Body::empty()))
        .provenance(Provenance::trusted("runner"))
        .tag("plan_id", plan_id.to_string())
        .tag("task_id", task_id.to_string())
        .tag("rung", rung.to_string())
        .build()
}

fn sanitize_gate_env_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if sanitized.is_empty() {
        "unknown".to_string()
    } else {
        sanitized
    }
}

async fn run_verify_steps(
    signal: &Signal,
    ctx: &roko_core::Context,
    task_id: &str,
    verify_steps: Vec<VerifyStep>,
) -> Vec<Verdict> {
    let mut verdicts = Vec::new();
    for (i, step) in verify_steps.iter().enumerate() {
        let step_start = Instant::now();
        let gate = verify_step_gate(task_id, step);
        let verdict = gate.verify(signal, ctx).await;
        info!(
            task_id = %task_id,
            step = i + 1,
            total_steps = verify_steps.len(),
            phase = %step.phase,
            command = %step.command,
            timeout_ms = step.timeout_ms,
            passed = verdict.passed,
            elapsed_ms = step_start.elapsed().as_millis() as u64,
            "verify step completed"
        );
        verdicts.push(verdict);
    }
    verdicts
}

fn verify_step_gate(task_id: &str, step: &VerifyStep) -> ShellGate {
    ShellGate::new(
        "bash",
        vec![
            "-o".into(),
            "pipefail".into(),
            "-c".into(),
            step.command.clone(),
        ],
    )
    .with_name(format!("task-verify:{}:{}", task_id, step.phase))
    .with_timeout_ms(step.timeout_ms)
}

fn render_output(verdicts: &[Verdict]) -> String {
    verdicts
        .iter()
        .map(render_verdict_output)
        .collect::<Vec<_>>()
        .join("; ")
}

fn render_verdict_output(v: &Verdict) -> String {
    let status = if v.skipped {
        "SKIP"
    } else if v.passed {
        "pass"
    } else {
        "FAIL"
    };
    let detail = v.detail.as_deref().unwrap_or("").trim();
    let digest = v.error_digest.as_deref().unwrap_or("").trim();
    let reason = v.reason.trim();

    let message = if v.passed {
        first_non_empty([detail, reason, digest])
    } else if !detail.is_empty() && !digest.is_empty() {
        format!("{detail}\n\nclassification:\n{digest}")
    } else {
        first_non_empty([detail, reason, digest])
    };

    if message.is_empty() {
        format!("{}: {status}", v.gate)
    } else {
        format!("{}: {status} — {message}", v.gate)
    }
}

fn first_non_empty<const N: usize>(values: [&str; N]) -> String {
    values
        .into_iter()
        .find(|value| !value.is_empty())
        .unwrap_or("")
        .to_string()
}

fn classify_failure_kind(verdicts: &[Verdict], output: &str) -> RunnerFailureKind {
    let combined = verdicts
        .iter()
        .filter(|v| !v.passed)
        .map(|v| {
            format!(
                "{}\n{}\n{}",
                v.reason,
                v.detail.as_deref().unwrap_or(""),
                v.error_digest.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let text = if combined.trim().is_empty() {
        output
    } else {
        &combined
    };
    let classification = classify_gate_failure("runner", text);
    let rendered = serde_json::to_string(&classification).unwrap_or_default();
    let fallback = RunnerFailureKind::from_output(text);
    match classification.recommended_action {
        roko_gate::GateFailureAction::Blocked => RunnerFailureKind::Resource,
        roko_gate::GateFailureAction::NeedsHuman => RunnerFailureKind::Permanent,
        roko_gate::GateFailureAction::NeedsReplan => RunnerFailureKind::Structural,
        roko_gate::GateFailureAction::Retry => {
            if rendered.contains("external_environment") {
                RunnerFailureKind::Transient
            } else {
                match fallback {
                    RunnerFailureKind::Resource | RunnerFailureKind::Transient => fallback,
                    RunnerFailureKind::Permanent
                    | RunnerFailureKind::Structural
                    | RunnerFailureKind::Unknown => RunnerFailureKind::Structural,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;
    use std::sync::Mutex;

    #[test]
    fn cargo_verify_fingerprint_ignores_only_non_semantic_flags_and_order() {
        let canonical =
            cargo_command_fingerprint("cargo check -p roko-cli --bin roko --message-format=json");
        let authored =
            cargo_command_fingerprint("cargo check --quiet --bin=roko --package roko-cli");
        assert_eq!(canonical, authored);
        assert!(cargo_command_fingerprint("cargo check -p roko-cli && echo pass").is_none());
        assert_ne!(
            canonical,
            cargo_command_fingerprint("cargo check -p roko-cli --lib")
        );
    }

    fn verify_step(command: &str) -> VerifyStep {
        VerifyStep {
            phase: "compile".to_string(),
            command: command.to_string(),
            fail_msg: None,
            timeout_ms: 1_000,
        }
    }

    #[test]
    fn fast_dedupe_removes_only_required_exact_canonical_commands() {
        let exact = "cargo check -p roko-cli --bin roko";
        let broad = "cargo check -p roko-cli";
        let retained = deduplicate_verify_steps(
            "task",
            vec![verify_step(exact), verify_step(broad)],
            &["cargo check --package roko-cli --bin=roko".to_string()],
        );
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].command, broad);

        let repeated =
            deduplicate_verify_steps("task", vec![verify_step(exact), verify_step(exact)], &[]);
        assert_eq!(repeated.len(), 2, "authored repetitions remain intentional");

        let mut gates = GatesConfig::default();
        gates.custom_rungs = vec![GateRungConfig {
            name: "compile".to_string(),
            command: exact.to_string(),
            timeout_secs: 30,
            required: false,
            parallel_with: Vec::new(),
        }];
        assert!(
            canonical_verify_commands(&gates, PlanComplexity::Trivial, &[], None).is_empty(),
            "optional canonical work cannot cover required authored verification"
        );
    }

    #[test]
    fn fast_task_verify_only_requires_exactly_one_authored_step() {
        assert!(fast_task_verify_contract_error(true, true, 0).is_some());
        assert!(fast_task_verify_contract_error(true, true, 2).is_some());
        assert!(fast_task_verify_contract_error(true, true, 1).is_none());

        assert!(
            fast_task_verify_contract_error(false, true, 0).is_none(),
            "default mode preserves the existing verify-only behavior"
        );
        assert!(
            fast_task_verify_contract_error(true, false, 0).is_none(),
            "FAST canonical verification is unaffected"
        );
    }

    #[test]
    fn manifest_root_paths_map_to_exact_cargo_targets() {
        let manifest = toml::from_str::<toml::Value>(
            r#"
[package]
name = "roko-cli"
autobins = false

[[bin]]
name = "roko"
path = "src/main.rs"

[[test]]
name = "runner_smoke"
path = "tests/runner_smoke.rs"
"#,
        )
        .expect("manifest parses");
        assert_eq!(
            manifest_target_for_path(&manifest, "roko-cli", "src/main.rs"),
            Some(CargoTargetSelector::Bin("roko".to_string()))
        );
        assert_eq!(
            manifest_target_for_path(&manifest, "roko-cli", "src/lib.rs"),
            Some(CargoTargetSelector::Lib)
        );
        assert_eq!(
            manifest_target_for_path(&manifest, "roko-cli", "tests/runner_smoke.rs"),
            Some(CargoTargetSelector::Test("runner_smoke".to_string()))
        );
        assert_eq!(
            manifest_target_for_path(&manifest, "roko-cli", "src/runner/mod.rs"),
            None,
            "module ownership is ambiguous and must fall back"
        );

        let auto_disabled = toml::from_str::<toml::Value>(
            r#"
[package]
name = "manual-targets"
autolib = false
autobins = false
autotests = false
"#,
        )
        .expect("manifest parses");
        for path in ["src/lib.rs", "src/main.rs", "tests/implicit.rs"] {
            assert_eq!(
                manifest_target_for_path(&auto_disabled, "manual-targets", path),
                None
            );
        }
    }

    #[test]
    fn duplicate_target_paths_are_ambiguous() {
        let manifest = toml::from_str::<toml::Value>(
            r#"
[package]
name = "ambiguous-targets"
autolib = false
autobins = false
autotests = false

[lib]
path = "src/shared.rs"

[[bin]]
name = "shared-bin"
path = "src/shared.rs"

[[test]]
name = "shared-test"
path = "src/shared.rs"
"#,
        )
        .expect("manifest parses");

        assert_eq!(
            manifest_target_for_path(&manifest, "ambiguous-targets", "src/shared.rs"),
            None,
            "one source path owned by multiple Cargo targets must fall back"
        );
    }

    #[test]
    fn deletion_or_rename_disables_fast_target_narrowing() {
        let dir = git_repo();
        std::fs::create_dir(dir.path().join("src")).expect("src directory");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname='fixture'\nversion='0.1.0'\n",
        )
        .expect("manifest");
        std::fs::write(dir.path().join("src/lib.rs"), "pub fn value() {}\n").expect("lib");
        std::fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").expect("main");
        let commit = std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .status()
            .expect("git add");
        assert!(commit.success());
        let commit = std::process::Command::new("git")
            .args(["commit", "-m", "fixture"])
            .current_dir(dir.path())
            .status()
            .expect("git commit");
        assert!(commit.success());

        std::fs::remove_file(dir.path().join("src/lib.rs")).expect("delete lib");
        std::fs::write(
            dir.path().join("src/main.rs"),
            "fn main() { println!(\"x\"); }\n",
        )
        .expect("modify main");
        assert_eq!(git_changed_files(dir.path()), None);
    }

    struct StateHubTelemetryTestSink(roko_runtime::StateHubSender);

    #[async_trait::async_trait]
    impl TelemetryEventSink for StateHubTelemetryTestSink {
        async fn emit(
            &self,
            event: &ObservableEvent,
            ancestry: &[LensScope],
        ) -> roko_core::Result<Vec<Signal>> {
            let errors = self.0.emit_observable(event, ancestry);
            if errors.is_empty() {
                Ok(Vec::new())
            } else {
                Err(roko_core::RokoError::invalid(errors.join("; ")))
            }
        }
    }

    struct RecordingGateLens {
        name: String,
        scope: LensScope,
        observes: Vec<roko_core::ObservableEventKind>,
        seen: Arc<Mutex<Vec<(String, ObservableEvent)>>>,
    }

    #[async_trait::async_trait]
    impl roko_core::TelemetryObserve for RecordingGateLens {
        async fn observe(&self, event: &ObservableEvent) -> roko_core::Result<Vec<Signal>> {
            self.seen
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push((self.name.clone(), event.clone()));
            Ok(Vec::new())
        }

        fn observes(&self) -> &[roko_core::ObservableEventKind] {
            &self.observes
        }

        fn scope(&self) -> LensScope {
            self.scope.clone()
        }
    }

    fn git_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for args in [
            vec!["init", "-b", "main"],
            vec!["config", "user.name", "Roko Test"],
            vec!["config", "user.email", "roko@example.invalid"],
            vec!["commit", "--allow-empty", "-m", "base"],
        ] {
            let output = std::process::Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git setup failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        dir
    }

    fn gate_effect(kind: GateCompletionKind) -> GateEffectRef {
        GateEffectRef {
            attempt: TaskAttemptRef::new("plan", "task", 1),
            kind,
            rung: 1,
            generation: 1,
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn preflight_producer_reaches_state_hub_scoped_lenses_once() -> roko_core::Result<()> {
        use roko_core::{LensConfig, LensRegistry, ObservableEventKind};
        use roko_runtime::{LensExecutor, LensQueueConfig};

        let dir = git_repo();
        let hub = roko_runtime::SharedStateHub::new_in_process();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut registry = LensRegistry::new();
        for (name, scope, kind) in [
            (
                "signal-verification-recorder",
                "graph:plan",
                ObservableEventKind::SignalLifecycle,
            ),
            (
                "preflight-recorder",
                "cell:task",
                ObservableEventKind::VerifyLifecycle,
            ),
        ] {
            registry.register_with_observes(
                LensConfig {
                    name: name.to_string(),
                    block: "test:recording-gate-lens".to_string(),
                    scope: scope.to_string(),
                    params: BTreeMap::new(),
                },
                vec![kind],
            )?;
        }
        let mut executor = LensExecutor::new(registry.clone())?.with_projection(hub.sender());
        for registration in registry.registrations() {
            executor.register(
                registration.config.name.clone(),
                Arc::new(RecordingGateLens {
                    name: registration.config.name.clone(),
                    scope: registration.scope.clone(),
                    observes: registration.observes.clone(),
                    seen: Arc::clone(&seen),
                }),
            )?;
        }
        let queue = executor.into_queued("gate-producer-test", LensQueueConfig::default())?;
        let telemetry_sink: Arc<dyn TelemetryEventSink> =
            Arc::new(StateHubTelemetryTestSink(hub.sender()));
        let gates = GatesConfig {
            custom_rungs: vec![roko_core::config::GateRungConfig {
                name: "preflight-test".into(),
                command: "true".into(),
                timeout_secs: 10,
                required: true,
                parallel_with: Vec::new(),
            }],
            ..GatesConfig::default()
        };

        let completion = run_gate_once(
            gate_effect(GateCompletionKind::Preflight),
            "plan".into(),
            "task".into(),
            1,
            dir.path().to_path_buf(),
            gates,
            PlanComplexity::Trivial,
            Vec::new(),
            None,
            10,
            Vec::new(),
            None,
            None,
            Some(telemetry_sink),
            None,
        )
        .await;
        assert!(completion.passed, "preflight should pass: {completion:#?}");

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if seen
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .len()
                    == 2
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("gate telemetry observations");
        assert!(queue.wait_idle(Duration::from_secs(5)).await);

        let observations = seen
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        assert_eq!(observations.len(), 2, "one event per production variant");
        assert!(observations.iter().any(|(lens, event)| {
            lens == "signal-verification-recorder"
                && matches!(event, ObservableEvent::SignalVerified(_, verdict) if verdict.passed)
        }));
        assert!(observations.iter().any(|(lens, event)| {
            lens == "preflight-recorder"
                && matches!(event, ObservableEvent::VerifyPreResult { block, verdict, evidence }
                    if block == "task" && verdict.passed && evidence.is_empty())
        }));

        Ok(())
    }

    fn barrier_gate() -> (
        JoinHandle<()>,
        oneshot::Sender<()>,
        mpsc::Receiver<GateCompletion>,
    ) {
        let dir = tempfile::tempdir().expect("tempdir");
        let workdir = dir.keep();
        let (tx, rx) = mpsc::channel(1);
        let effect = GateEffectRef {
            attempt: TaskAttemptRef::new("plan", "task", 1),
            kind: GateCompletionKind::Gate,
            rung: 1,
            generation: 99,
        };
        let (handle, start) = spawn_gate(
            effect,
            "plan".to_string(),
            "task".to_string(),
            1,
            workdir,
            GatesConfig::default(),
            PlanComplexity::Trivial,
            Vec::new(),
            None,
            1,
            tx,
            Arc::new(Semaphore::new(1)),
            Vec::new(),
            None,
            None,
            None,
            None, // main_target_dir
        );
        (handle, start, rx)
    }

    #[tokio::test]
    async fn gate_producer_waits_for_owner_start_barrier() {
        let (handle, start, mut rx) = barrier_gate();
        tokio::task::yield_now().await;
        assert!(matches!(
            rx.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        drop(start);
        handle.await.expect("barrier cancellation should be clean");
    }

    #[tokio::test]
    async fn gate_start_reports_failure_after_producer_abort() {
        let (handle, start, _rx) = barrier_gate();
        handle.abort();
        let _ = handle.await;
        assert!(start.send(()).is_err());
    }

    #[tokio::test]
    async fn plan_verify_is_barriered_and_preserves_exact_effect() {
        let shared_root = git_repo();
        std::fs::write(shared_root.path().join("unrelated.txt"), b"dirty root\n").unwrap();
        let dir = git_repo();
        let expected_oid = String::from_utf8_lossy(
            &std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(dir.path())
                .output()
                .unwrap()
                .stdout,
        )
        .trim()
        .to_string();
        let effect = GateEffectRef {
            attempt: TaskAttemptRef::new("plan-a", "plan-verify", 1),
            kind: GateCompletionKind::PlanVerify,
            rung: RUNG_PLAN_VERIFY,
            generation: 501,
        };
        let (tx, mut rx) = mpsc::channel(1);
        let (handle, start) = spawn_plan_verify(
            effect.clone(),
            "plan-a".to_string(),
            dir.path().to_path_buf(),
            expected_oid,
            Vec::new(),
            1,
            tx,
            Arc::new(Semaphore::new(1)),
            None, // main_target_dir
        );
        tokio::task::yield_now().await;
        assert!(matches!(
            rx.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        start.send(()).unwrap();
        let completion = rx.recv().await.unwrap();
        handle.await.unwrap();
        assert!(completion.passed);
        assert_eq!(completion.effect, Some(effect));
        assert!(shared_root.path().join("unrelated.txt").exists());
    }

    #[tokio::test]
    async fn closed_plan_verify_semaphore_emits_exact_resource_failure() {
        let dir = tempfile::tempdir().unwrap();
        let effect = GateEffectRef {
            attempt: TaskAttemptRef::new("plan-b", "plan-verify", 1),
            kind: GateCompletionKind::PlanVerify,
            rung: RUNG_PLAN_VERIFY,
            generation: 502,
        };
        let semaphore = Arc::new(Semaphore::new(0));
        semaphore.close();
        let (tx, mut rx) = mpsc::channel(1);
        let (handle, start) = spawn_plan_verify(
            effect.clone(),
            "plan-b".to_string(),
            dir.path().to_path_buf(),
            "unused".to_string(),
            Vec::new(),
            1,
            tx,
            semaphore,
            None, // main_target_dir
        );
        start.send(()).unwrap();
        let completion = rx.recv().await.unwrap();
        handle.await.unwrap();
        assert!(!completion.passed);
        assert_eq!(completion.failure_kind, Some(RunnerFailureKind::Resource));
        assert_eq!(completion.effect, Some(effect));
    }

    #[tokio::test]
    async fn closed_semaphore_emits_exact_failed_preflight_completion() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (tx, mut rx) = mpsc::channel(1);
        let semaphore = Arc::new(Semaphore::new(1));
        semaphore.close();
        let effect = GateEffectRef {
            attempt: TaskAttemptRef::new("plan", "task", 2),
            kind: GateCompletionKind::Preflight,
            rung: 3,
            generation: 101,
        };
        let (handle, start) = spawn_gate(
            effect.clone(),
            "plan".to_string(),
            "task".to_string(),
            3,
            dir.path().to_path_buf(),
            GatesConfig::default(),
            PlanComplexity::Trivial,
            Vec::new(),
            None,
            1,
            tx,
            semaphore,
            Vec::new(),
            None,
            None,
            None,
            None, // main_target_dir
        );

        start.send(()).expect("owner starts producer");
        let completion = rx.recv().await.expect("structured failure completion");
        handle.await.expect("supervisor exits cleanly");
        assert!(!completion.passed);
        assert_eq!(completion.kind, GateCompletionKind::Preflight);
        assert_eq!(completion.attempt.as_ref(), Some(&effect.attempt));
        assert_eq!(completion.effect.as_ref(), Some(&effect));
        assert_eq!(completion.failure_kind, Some(RunnerFailureKind::Resource));
        assert!(completion.output.contains("semaphore closed"));
    }

    #[test]
    fn retry_recommended_gate_digest_remains_retryable() {
        let digest = r#"{
  "gate": "task-verify:C01:structural",
  "primary": "unknown",
  "failure_kind": "permanent",
  "retry_policy": {
    "retryable": true,
    "cooldown_secs": 0,
    "include_error_digest": true,
    "generate_reflection": true,
    "regenerate_verify": false
  },
  "summary": "exit code: 1",
  "classes": ["unknown"],
  "compile_errors": [],
  "error_count": 0,
  "warning_count": 0,
  "cargo_fix_candidate": false,
  "agent_retry_needed": true,
  "recommended_action": "retry",
  "replan_candidate": false,
  "blocking_findings": [],
  "duration_ms": 10,
  "raw_excerpt": ""
}"#;
        let verdict =
            Verdict::fail("task-verify:C01:structural", "exit code: 1").with_error_digest(digest);

        let kind = classify_failure_kind(&[verdict], "");

        assert_eq!(kind, RunnerFailureKind::Structural);
        assert!(kind.is_retryable());
    }

    #[test]
    fn failed_gate_output_prefers_command_detail_before_classification() {
        let verdict = Verdict::fail("task-verify:V03:test", "exit code: 1")
            .with_detail("failures:\n    workspace_tests::regression\n")
            .with_error_digest(r#"{"recommended_action":"retry"}"#);

        let rendered = render_output(&[verdict]);

        assert!(rendered.contains("workspace_tests::regression"));
        assert!(rendered.contains("classification:"));
        assert!(
            rendered.find("workspace_tests::regression").unwrap()
                < rendered.find("classification:").unwrap()
        );
    }

    #[tokio::test]
    async fn verify_steps_fail_when_a_piped_command_fails_before_tail() {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        let signal = gate_signal("plan", "task", 2, tempdir.path(), &[], None);
        let ctx = roko_core::Context::now();
        let step = VerifyStep {
            phase: "test".to_string(),
            command: "false | tail -1".to_string(),
            fail_msg: None,
            timeout_ms: 10_000,
        };

        let verdicts = run_verify_steps(&signal, &ctx, "T01", vec![step]).await;

        assert_eq!(verdicts.first().map(|verdict| verdict.passed), Some(false));
    }

    #[tokio::test]
    async fn verify_steps_pass() {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        let signal = gate_signal("plan", "task", 2, tempdir.path(), &[], None);
        let ctx = roko_core::Context::now();
        let step = VerifyStep {
            phase: "structural".to_string(),
            command: "true".to_string(),
            fail_msg: None,
            timeout_ms: 10_000,
        };

        let verdicts = run_verify_steps(&signal, &ctx, "T01", vec![step]).await;

        assert_eq!(verdicts.first().map(|verdict| verdict.passed), Some(true));
    }

    #[tokio::test]
    async fn failures_identify_actual_baseline_and_owned_diff_inputs() {
        for (owned, expected) in [(false, "baseline:"), (true, "owned-diff:")] {
            let dir = git_repo();
            if owned {
                std::fs::write(dir.path().join("candidate.txt"), b"owned\n").unwrap();
            }
            let completion = run_gate_once(
                gate_effect(GateCompletionKind::Gate),
                "plan".into(),
                "task".into(),
                1,
                dir.path().to_path_buf(),
                GatesConfig::default(),
                PlanComplexity::Trivial,
                vec![VerifyStep {
                    phase: "test".into(),
                    command: "false".into(),
                    fail_msg: None,
                    timeout_ms: 10_000,
                }],
                Some(Vec::new()),
                10,
                Vec::new(),
                None,
                None,
                None,
                None,
            )
            .await;
            assert!(!completion.passed);
            assert!(
                completion
                    .verdicts
                    .iter()
                    .filter(|verdict| !verdict.passed)
                    .all(|verdict| verdict.gate_name.starts_with(expected))
            );
        }
    }

    #[tokio::test]
    async fn gate_fails_closed_when_verification_mutates_owned_input() {
        let dir = git_repo();
        std::fs::write(dir.path().join("tracked.txt"), b"before\n").unwrap();
        for args in [
            vec!["add", "tracked.txt"],
            vec!["commit", "-m", "tracked input"],
        ] {
            assert!(
                std::process::Command::new("git")
                    .args(args)
                    .current_dir(dir.path())
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let completion = run_gate_once(
            gate_effect(GateCompletionKind::Gate),
            "plan".into(),
            "task".into(),
            1,
            dir.path().to_path_buf(),
            GatesConfig::default(),
            PlanComplexity::Trivial,
            vec![VerifyStep {
                phase: "test".into(),
                command: "printf 'after\\n' > tracked.txt".into(),
                fail_msg: None,
                timeout_ms: 10_000,
            }],
            Some(Vec::new()),
            10,
            Vec::new(),
            None,
            None,
            None,
            None,
        )
        .await;

        assert!(!completion.passed);
        assert!(
            completion
                .verdicts
                .iter()
                .any(|verdict| verdict.gate_name == "unattributed:immutable-input")
        );
    }

    #[tokio::test]
    async fn failure_reproduced_on_baseline_retains_both_identities() {
        let dir = git_repo();
        let step = VerifyStep {
            phase: "test".into(),
            command: "false".into(),
            fail_msg: None,
            timeout_ms: 10_000,
        };
        let baseline = run_gate_once(
            gate_effect(GateCompletionKind::Preflight),
            "plan".into(),
            "task".into(),
            1,
            dir.path().to_path_buf(),
            GatesConfig::default(),
            PlanComplexity::Trivial,
            vec![step.clone()],
            None,
            10,
            Vec::new(),
            None,
            None,
            None,
            None,
        )
        .await;
        let baseline_failures = baseline
            .verdicts
            .iter()
            .filter(|verdict| !verdict.passed)
            .map(|verdict| raw_gate_name(&verdict.gate_name).to_string())
            .collect();
        std::fs::write(dir.path().join("candidate.txt"), b"owned\n").unwrap();
        let candidate = run_gate_once(
            gate_effect(GateCompletionKind::Gate),
            "plan".into(),
            "task".into(),
            1,
            dir.path().to_path_buf(),
            GatesConfig::default(),
            PlanComplexity::Trivial,
            vec![step],
            Some(baseline_failures),
            10,
            Vec::new(),
            None,
            None,
            None,
            None,
        )
        .await;
        assert!(candidate.verdicts.iter().any(|verdict| {
            !verdict.passed && verdict.gate_name.starts_with("baseline+owned:")
        }));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn equal_content_symlink_target_swap_invalidates_gate_input() {
        use std::os::unix::fs::symlink;

        let dir = git_repo();
        std::fs::write(dir.path().join("a"), b"same\n").unwrap();
        std::fs::write(dir.path().join("b"), b"same\n").unwrap();
        symlink("a", dir.path().join("input")).unwrap();
        let completion = run_gate_once(
            gate_effect(GateCompletionKind::Gate),
            "plan".into(),
            "task".into(),
            1,
            dir.path().to_path_buf(),
            GatesConfig::default(),
            PlanComplexity::Trivial,
            vec![VerifyStep {
                phase: "test".into(),
                command: "ln -sfn b input".into(),
                fail_msg: None,
                timeout_ms: 10_000,
            }],
            Some(Vec::new()),
            10,
            Vec::new(),
            None,
            None,
            None,
            None,
        )
        .await;
        assert!(!completion.passed);
        assert!(
            completion
                .verdicts
                .iter()
                .any(|verdict| { verdict.gate_name == "unattributed:immutable-input" })
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_cycle_is_not_traversed() {
        use std::os::unix::fs::symlink;

        let dir = git_repo();
        symlink(".", dir.path().join("cycle")).unwrap();
        gate_input_snapshot_blocking(dir.path()).expect("symlink cycle remains a link input");
    }

    #[cfg(unix)]
    #[test]
    fn equal_kind_len_mtime_inode_replacement_is_detected() {
        use std::os::unix::fs::MetadataExt;

        let dir = git_repo();
        let input = dir.path().join("input");
        let replacement = dir.path().join("replacement");
        std::fs::write(&input, b"aaaa").unwrap();
        std::fs::write(&replacement, b"bbbb").unwrap();
        let before = std::fs::symlink_metadata(&input).unwrap();
        OpenOptions::new()
            .write(true)
            .open(&replacement)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(before.modified().unwrap()))
            .unwrap();
        std::fs::rename(&replacement, &input).unwrap();
        let after = std::fs::symlink_metadata(&input).unwrap();

        assert_eq!(before.file_type(), after.file_type());
        assert_eq!(before.len(), after.len());
        assert_eq!(before.modified().unwrap(), after.modified().unwrap());
        assert_eq!(before.dev(), after.dev());
        assert_ne!(before.ino(), after.ino());
        assert!(!metadata_unchanged(&before, &after));
    }

    #[cfg(unix)]
    #[test]
    fn untracked_executable_mode_changes_gate_and_replay_fingerprint() {
        use std::os::unix::fs::PermissionsExt;

        let dir = git_repo();
        let input = dir.path().join("script.sh");
        std::fs::write(&input, b"#!/bin/sh\nexit 0\n").expect("write untracked script");
        std::fs::set_permissions(&input, std::fs::Permissions::from_mode(0o644))
            .expect("set non-executable mode");
        let before = gate_input_snapshot_blocking(dir.path()).expect("initial fingerprint");
        let metadata_before = std::fs::symlink_metadata(&input).expect("initial metadata");

        std::fs::set_permissions(&input, std::fs::Permissions::from_mode(0o755))
            .expect("set executable mode");
        let after = gate_input_snapshot_blocking(dir.path()).expect("updated fingerprint");
        let metadata_after = std::fs::symlink_metadata(&input).expect("updated metadata");

        assert_ne!(before.1, after.1);
        assert!(!metadata_unchanged(&metadata_before, &metadata_after));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn bare_fifo_is_rejected_without_scanning_ignored_build_artifacts() {
        let dir = git_repo();
        std::fs::write(dir.path().join(".gitignore"), "ignored-build/\n").unwrap();
        std::fs::create_dir(dir.path().join("ignored-build")).unwrap();
        assert!(
            std::process::Command::new("mkfifo")
                .arg("ignored-build/cache.fifo")
                .current_dir(dir.path())
                .status()
                .unwrap()
                .success()
        );
        gate_input_snapshot_blocking(dir.path()).expect("ignored artifacts are pruned");
        assert!(
            std::process::Command::new("mkfifo")
                .arg("input.fifo")
                .current_dir(dir.path())
                .status()
                .unwrap()
                .success()
        );
        assert!(
            gate_input_snapshot_blocking(dir.path())
                .unwrap_err()
                .contains("non-file input")
        );
        let completion = tokio::time::timeout(
            Duration::from_secs(2),
            run_gate_once(
                gate_effect(GateCompletionKind::Gate),
                "plan".into(),
                "task".into(),
                1,
                dir.path().to_path_buf(),
                GatesConfig::default(),
                PlanComplexity::Trivial,
                Vec::new(),
                Some(Vec::new()),
                1,
                Vec::new(),
                None,
                None,
                None,
                None,
            ),
        )
        .await
        .expect("FIFO fingerprinting must not block");
        assert!(!completion.passed);
        assert!(completion.duration_ms < 2_000, "{completion:#?}");
        assert!(completion.verdicts.iter().any(|verdict| {
            verdict.gate_name == "unattributed:input-snapshot"
                && verdict.summary.contains("non-file input")
        }));
    }

    #[tokio::test]
    async fn untracked_size_and_count_limits_are_deterministic() {
        let oversized = git_repo();
        File::create(oversized.path().join("large.bin"))
            .unwrap()
            .set_len(MAX_UNTRACKED_FILE_BYTES + 1)
            .unwrap();
        assert!(
            gate_input_snapshot_blocking(oversized.path())
                .unwrap_err()
                .contains("exceeds input limit")
        );
        let completion = run_gate_once(
            gate_effect(GateCompletionKind::Gate),
            "plan".into(),
            "task".into(),
            1,
            oversized.path().to_path_buf(),
            GatesConfig::default(),
            PlanComplexity::Trivial,
            Vec::new(),
            Some(Vec::new()),
            1,
            Vec::new(),
            None,
            None,
            None,
            None,
        )
        .await;
        assert!(
            completion
                .verdicts
                .iter()
                .any(|verdict| { verdict.gate_name == "unattributed:input-snapshot" })
        );

        let counted = git_repo();
        for index in 0..MAX_UNTRACKED_FILES {
            File::create(counted.path().join(format!("item-{index:04}"))).unwrap();
        }
        gate_input_snapshot_blocking(counted.path()).expect("count boundary is accepted");
        File::create(counted.path().join("one-too-many")).unwrap();
        assert!(
            gate_input_snapshot_blocking(counted.path())
                .unwrap_err()
                .contains("untracked file count")
        );
    }

    /// E05-T08: Prove that `run_gate_once` publishes non-skipped verdicts
    /// through the `VerdictPublisher` as `Kind::GateVerdict` pulses.
    #[tokio::test]
    async fn live_gate_verdicts_publish_signal() {
        use roko_core::Kind;
        use roko_core::config::GateRungConfig;
        use std::sync::Mutex;

        let published: Arc<Mutex<Vec<roko_core::Pulse>>> = Arc::new(Mutex::new(Vec::new()));
        let published_clone = Arc::clone(&published);
        let publisher = VerdictPublisher::new(Arc::new(move |pulse| {
            published_clone.lock().unwrap().push(pulse);
        }));

        let dir = git_repo();
        let gates = GatesConfig {
            custom_rungs: vec![GateRungConfig {
                name: "test".into(),
                command: "true".into(),
                timeout_secs: 10,
                required: true,
                parallel_with: Vec::new(),
            }],
            ..GatesConfig::default()
        };
        let completion = run_gate_once(
            gate_effect(GateCompletionKind::Gate),
            "plan-pub".into(),
            "task-pub".into(),
            2,
            dir.path().to_path_buf(),
            gates,
            PlanComplexity::Trivial,
            vec![VerifyStep {
                phase: "test".into(),
                command: "true".into(),
                fail_msg: None,
                timeout_ms: 10_000,
            }],
            None,
            10,
            Vec::new(),
            Some(publisher),
            None,
            None,
            None,
        )
        .await;

        assert!(completion.passed, "gate should pass: {completion:#?}");

        let pulses = published.lock().unwrap();
        assert!(
            !pulses.is_empty(),
            "VerdictPublisher must receive at least one pulse"
        );
        for pulse in pulses.iter() {
            assert_eq!(
                pulse.kind,
                Kind::GateVerdict,
                "published pulse must be Kind::GateVerdict"
            );
            assert_eq!(
                pulse.topic,
                roko_core::Topic::new("gate.verdict.emitted"),
                "published pulse must have gate.verdict.emitted topic"
            );
        }
    }

    // ── E45-T02: auto-fix path tests ─────────────────────────────────────────

    /// Gate output that does NOT look like a cargo_fix_candidate should result
    /// in `was_candidate = false` and no fix command attempted.
    #[tokio::test]
    async fn auto_fix_skips_non_candidate_output() {
        let dir = tempfile::tempdir().unwrap();
        // A plain test failure string — no compile errors, so cargo_fix_candidate = false.
        let non_compile_output = "test result: FAILED. 2 passed; 1 failed; 0 ignored";
        let outcome = attempt_auto_fix(dir.path(), "test", non_compile_output)
            .await
            .expect("attempt_auto_fix must not return Err for non-candidates");

        assert!(
            !outcome.was_candidate,
            "test failures are not fix candidates"
        );
        assert!(!outcome.fix_applied);
        assert!(!outcome.gate_passed_after_fix);
        assert!(outcome.command.is_none());
        assert_eq!(outcome.gate_name, "test");
    }

    /// A gate name that is neither "compile" nor "clippy" should produce a
    /// not-candidate outcome even if the output looks fixable.
    #[tokio::test]
    async fn auto_fix_skips_unknown_gate_name() {
        let dir = tempfile::tempdir().unwrap();
        // Even with compile-looking output, an unrecognised gate name is not fixable.
        let output = "error[E0433]: failed to resolve: use of undeclared crate `foo`";
        let outcome = attempt_auto_fix(dir.path(), "docs", output)
            .await
            .expect("attempt_auto_fix must not error");

        assert!(!outcome.fix_applied);
        assert_eq!(outcome.gate_name, "docs");
    }

    /// `AutoFixOutcome` must accurately record the command string when a fix is attempted.
    /// We cannot easily run real cargo fix in a unit test, but we can verify that when
    /// `cargo_fix_candidate` is false, the command field stays None.
    #[tokio::test]
    async fn auto_fix_outcome_command_is_none_for_non_candidates() {
        let dir = tempfile::tempdir().unwrap();
        let output = "nothing interesting here";
        let outcome = attempt_auto_fix(dir.path(), "compile", output)
            .await
            .unwrap();

        // classify_gate_failure("compile", ...) on empty/non-error output should
        // set cargo_fix_candidate = false.
        assert!(!outcome.fix_applied);
        assert!(
            outcome.command.is_none(),
            "no command should be recorded when fix was not attempted"
        );
    }

    /// `cargo_fix_enabled` defaults to `true` in `GatesConfig::default()`.
    #[test]
    fn gates_config_cargo_fix_enabled_default_is_true() {
        let cfg = GatesConfig::default();
        assert!(
            cfg.cargo_fix_enabled,
            "cargo_fix_enabled should default to true"
        );
    }

    /// `cargo_fix_enabled = false` must round-trip through TOML deserialization.
    #[test]
    fn gates_config_cargo_fix_enabled_toml_roundtrip() {
        use roko_core::config::schema::RokoConfig;
        let toml = r#"
[gates]
cargo_fix_enabled = false
"#;
        let config = RokoConfig::from_toml(toml).expect("config must parse");
        assert!(
            !config.gates.cargo_fix_enabled,
            "cargo_fix_enabled must deserialize to false"
        );
    }
}
