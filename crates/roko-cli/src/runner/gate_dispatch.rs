//! Verify dispatch — runs gate rungs as background tokio tasks and sends
//! results through a channel.

use std::collections::{BTreeSet, HashMap};
use std::fs::OpenOptions;
use std::io::Read;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Instant;

use futures::FutureExt;
use roko_core::config::{GateMode, GateRungConfig, GatesConfig};
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
use tokio::process::Command;
use tokio::sync::{Semaphore, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{Duration, timeout};
use tracing::{error, info, warn};

use crate::task_parser::VerifyStep;

use super::types::{
    GateCompletion, GateCompletionKind, GateEffectRef, GateVerdictSummary, RunnerFailureKind,
};
use super::{impact_analysis, impact_analysis::ImpactReport};

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
    /// File scope authorized by the plan, compared with the actual diff.
    pub planned_files: Vec<String>,
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
            planned_files: td.files.clone(),
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

fn effective_gate_mode(configured: GateMode, fast_mode: bool, task_verify_only: bool) -> GateMode {
    // Backward-compatible fail-safe for callers of the original FAST switch.
    // The supported wrapper now sets `ROKO_GATE_MODE=focused` explicitly.
    if fast_mode && task_verify_only && configured == GateMode::Full {
        GateMode::Focused
    } else {
        configured
    }
}

fn command_uses_cargo(command: &str) -> bool {
    cargo_command_fingerprint(command).is_some()
        || command
            .split(|character: char| character.is_ascii_whitespace() || ";&|()".contains(character))
            .any(|token| token == "cargo")
}

#[derive(Default)]
struct CompileCoordinatorRegistry {
    repositories: HashMap<PathBuf, Weak<Semaphore>>,
    workdir_keys: HashMap<PathBuf, PathBuf>,
}

type CompileCoordinators = Mutex<CompileCoordinatorRegistry>;

async fn compile_coordinator(workdir: &Path, permits: usize) -> Arc<Semaphore> {
    static COORDINATORS: OnceLock<CompileCoordinators> = OnceLock::new();
    let coordinators =
        COORDINATORS.get_or_init(|| Mutex::new(CompileCoordinatorRegistry::default()));
    let workdir_key = workdir.to_path_buf();
    {
        let coordinators = coordinators
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if let Some(repository) = coordinators.workdir_keys.get(&workdir_key)
            && let Some(existing) = coordinators
                .repositories
                .get(repository)
                .and_then(Weak::upgrade)
        {
            return existing;
        }
    }

    let common_dir = timeout(
        Duration::from_secs(2),
        Command::new("git")
            .args(["rev-parse", "--git-common-dir"])
            .current_dir(workdir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .ok()
    .and_then(Result::ok)
    .filter(|output| output.status.success())
    .and_then(|output| String::from_utf8(output.stdout).ok())
    .map(|path| {
        let path = PathBuf::from(path.trim());
        if path.is_absolute() {
            path
        } else {
            workdir.join(path)
        }
    })
    .unwrap_or_else(|| workdir.to_path_buf());
    let repository = tokio::fs::canonicalize(&common_dir)
        .await
        .unwrap_or(common_dir);

    let mut coordinators = coordinators
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    coordinators
        .workdir_keys
        .insert(workdir_key, repository.clone());
    if let Some(existing) = coordinators
        .repositories
        .get(&repository)
        .and_then(Weak::upgrade)
    {
        return existing;
    }
    coordinators
        .repositories
        .retain(|_, coordinator| coordinator.strong_count() > 0);
    let coordinator = Arc::new(Semaphore::new(permits.max(1)));
    coordinators
        .repositories
        .insert(repository, Arc::downgrade(&coordinator));
    coordinator
}

async fn acquire_compile_ownership(
    workdir: &Path,
    permits: usize,
    max_wait: Duration,
    plan_id: &str,
    task_id: &str,
    command: &str,
) -> Result<tokio::sync::OwnedSemaphorePermit, String> {
    let started = Instant::now();
    let coordinator = compile_coordinator(workdir, permits).await;
    let permit = timeout(max_wait, coordinator.acquire_owned())
        .await
        .map_err(|_| format!("compile ownership timed out for `{command}`"))?
        .map_err(|_| "compile ownership semaphore closed".to_string())?;
    info!(
        plan_id,
        task_id,
        command,
        wait_ms = elapsed_millis(started),
        compile_concurrency = permits.max(1),
        "compile ownership acquired"
    );
    Ok(permit)
}

fn elapsed_millis(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn cargo_cache_counts(verdict: &Verdict) -> (u64, u64) {
    verdict
        .detail
        .as_deref()
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|message| {
            message.get("reason").and_then(serde_json::Value::as_str) == Some("compiler-artifact")
        })
        .fold((0_u64, 0_u64), |(hits, misses), message| {
            if message
                .get("fresh")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                (hits.saturating_add(1), misses)
            } else {
                (hits, misses.saturating_add(1))
            }
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

/// Add an explicit Cargo profile to a simple runner-owned gate command.
///
/// Environment variables such as `CARGO_PROFILE_DEV_*` only configure a
/// profile; Cargo does not select that profile unless the command includes
/// `--profile`. Shell composition and quoted commands are intentionally left
/// untouched because rewriting them safely requires a shell parser.
fn cargo_command_with_profile(command: &str, profile: &str) -> Option<String> {
    let tokens = simple_command_tokens(command)?;
    if tokens.first().copied() != Some("cargo")
        || !matches!(tokens.get(1).copied(), Some("check" | "clippy" | "test"))
    {
        return None;
    }
    if tokens
        .iter()
        .any(|token| *token == "--profile" || token.strip_prefix("--profile=").is_some())
    {
        return Some(command.trim().to_string());
    }

    let mut selected = tokens.into_iter().map(str::to_string).collect::<Vec<_>>();
    let insertion = selected
        .iter()
        .position(|token| token == "--")
        .unwrap_or(selected.len());
    selected.splice(
        insertion..insertion,
        ["--profile".to_string(), profile.to_string()],
    );
    Some(selected.join(" "))
}

fn cargo_profile_available(workdir: &Path, profile: &str) -> bool {
    std::fs::read_to_string(workdir.join("Cargo.toml"))
        .ok()
        .and_then(|manifest| toml::from_str::<toml::Value>(&manifest).ok())
        .and_then(|manifest| manifest.get("profile").cloned())
        .and_then(|profiles| profiles.get(profile).cloned())
        .is_some_and(|profile| profile.is_table())
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

fn scoped_test_command(workdir: &Path, command: &str, report: &ImpactReport) -> Option<String> {
    let target = report.one_target()?;
    let tokens = simple_command_tokens(command)?;
    if tokens.get(1).copied() != Some("test")
        || tokens.contains(&"--")
        || tokens.iter().any(|token| {
            matches!(
                *token,
                "--lib" | "--bin" | "--test" | "--example" | "--bench" | "--all-targets"
            ) || token.starts_with("--bin=")
                || token.starts_with("--test=")
                || token.starts_with("--example=")
                || token.starts_with("--bench=")
        })
    {
        return None;
    }
    let selected_package = tokens
        .windows(2)
        .find_map(|pair| matches!(pair[0], "-p" | "--package").then_some(pair[1]));
    if selected_package != Some(target.package.as_str()) {
        return None;
    }

    match &target.selector {
        impact_analysis::CargoTargetSelector::Test(name) => {
            Some(format!("{} --test {name}", command.trim()))
        }
        impact_analysis::CargoTargetSelector::Lib => {
            let changed = report
                .changed_files
                .iter()
                .filter(|path| path.ends_with(".rs"))
                .collect::<Vec<_>>();
            let path = changed.as_slice().first().copied()?;
            if changed.len() != 1 {
                return None;
            }
            let source = std::fs::read_to_string(workdir.join(path)).ok()?;
            if !source.contains("#[test]") && !source.contains("mod tests") {
                return None;
            }
            let module = path
                .split("/src/")
                .nth(1)?
                .strip_suffix(".rs")?
                .trim_end_matches("/mod")
                .replace('/', "::");
            if module.is_empty() || matches!(module.as_str(), "lib" | "main") {
                return None;
            }
            Some(format!("{} --lib -- {module}::", command.trim()))
        }
        _ => None,
    }
}

fn scope_authored_verify_steps(
    workdir: &Path,
    task_id: &str,
    steps: Vec<VerifyStep>,
    report: &ImpactReport,
) -> Vec<VerifyStep> {
    steps
        .into_iter()
        .map(|mut step| {
            if let Some(scoped) = scoped_test_command(workdir, &step.command, report) {
                info!(
                    task_id,
                    original_command = %step.command,
                    scoped_command = %scoped,
                    "focused gate scoped authored Cargo test"
                );
                step.command = scoped;
            }
            step
        })
        .collect()
}

fn focused_verify_steps(
    report: &ImpactReport,
    task_id: &str,
    authored: Vec<VerifyStep>,
    timeout_secs: u64,
    cargo_profile: Option<&str>,
) -> Vec<VerifyStep> {
    let commands = report
        .focused_commands()
        .into_iter()
        .map(|command| {
            cargo_profile
                .and_then(|profile| cargo_command_with_profile(&command, profile))
                .unwrap_or(command)
        })
        .collect::<Vec<_>>();
    let authored = deduplicate_verify_steps(task_id, authored, &commands);
    let mut selected = commands
        .into_iter()
        .map(|command| VerifyStep {
            phase: "impact-compile".into(),
            command,
            fail_msg: Some("impact-selected Cargo check failed".into()),
            timeout_ms: timeout_secs.max(1).saturating_mul(1_000),
        })
        .collect::<Vec<_>>();
    selected.extend(authored);
    selected
}

fn with_targeted_compile_rung(
    gates_config: &GatesConfig,
    complexity: PlanComplexity,
    targeted: Option<&TargetedCargoCheck>,
    timeout_secs: u64,
    cargo_profile: Option<&str>,
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
                cargo_profile
                    .and_then(|profile| cargo_command_with_profile(&targeted.command, profile))
                    .unwrap_or_else(|| targeted.command.clone())
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

/// Fetch the `git diff HEAD` output for the LlmJudge gate.
///
/// Runs `git diff HEAD -- .` in a blocking task with a bounded 5 s timeout.
/// Returns `None` on any error or timeout so the caller can fall back to
/// description-only evaluation.
async fn fetch_git_diff(workdir: &Path) -> Option<String> {
    let workdir = workdir.to_path_buf();
    tokio::time::timeout(
        Duration::from_secs(5),
        tokio::task::spawn_blocking(move || {
            std::process::Command::new("git")
                .args(["diff", "HEAD", "--", "."])
                .current_dir(&workdir)
                .env("GIT_TERMINAL_PROMPT", "0")
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout).ok()
                    } else {
                        None
                    }
                })
        }),
    )
    .await
    .ok()
    .and_then(|r| r.ok())
    .flatten()
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

fn gate_input_fingerprint_id(snapshot: &GateInputSnapshot) -> String {
    let mut identity = Sha256::new();
    hash_part(&mut identity, snapshot.0.as_bytes());
    identity.update(snapshot.1);
    identity.update([u8::from(snapshot.2)]);
    format!("{:x}", identity.finalize())
}

/// Combined identity of the immutable base plus every tracked/untracked byte
/// and mode in a task checkout.
pub(super) async fn owned_input_fingerprint_id(workdir: PathBuf) -> Result<String, String> {
    let snapshot = gate_input_snapshot(workdir).await?;
    snapshot
        .2
        .then(|| gate_input_fingerprint_id(&snapshot))
        .ok_or_else(|| "worktree has no owned diff to fingerprint".to_string())
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

fn normalized_failure_fingerprint(digest: Option<&str>) -> Option<String> {
    fn remove_volatile(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                object.remove("duration_ms");
                for child in object.values_mut() {
                    remove_volatile(child);
                }
            }
            serde_json::Value::Array(array) => {
                for child in array {
                    remove_volatile(child);
                }
            }
            _ => {}
        }
    }
    let digest = digest?.trim();
    if digest.is_empty() {
        return None;
    }
    let mut value = serde_json::from_str::<serde_json::Value>(digest).ok()?;
    remove_volatile(&mut value);
    serde_json::to_string(&value).ok()
}

fn filter_preexisting_failures(
    task_id: &str,
    verdicts: &mut [Verdict],
    baseline: Option<&[GateVerdictSummary]>,
) {
    let Some(baseline) = baseline else {
        return;
    };
    for verdict in verdicts
        .iter_mut()
        .filter(|verdict| !verdict.passed && !verdict.skipped)
    {
        let current_name = raw_gate_name(&verdict.gate);
        let current_fingerprint = normalized_failure_fingerprint(verdict.error_digest.as_deref());
        let unchanged = baseline.iter().any(|prior| {
            !prior.passed
                && raw_gate_name(&prior.gate_name) == current_name
                && current_fingerprint.is_some()
                && current_fingerprint
                    == normalized_failure_fingerprint(prior.error_digest.as_deref())
        });
        if unchanged {
            let original = verdict.gate.clone();
            verdict.passed = true;
            verdict.gate = format!("pre-existing-filtered:{original}");
            verdict.reason = "unchanged pre-existing verification failure filtered".into();
            info!(
                task_id,
                gate = %original,
                "filtered unchanged pre-existing gate failure"
            );
        }
    }
}

fn gate_failure_input(
    kind: GateCompletionKind,
    before: &GateInputSnapshot,
    baseline_failed_gates: Option<&[GateVerdictSummary]>,
    gate: &str,
) -> &'static str {
    match (kind, before.2, baseline_failed_gates) {
        (GateCompletionKind::Preflight, _, _) | (GateCompletionKind::Gate, false, _) => "baseline",
        (GateCompletionKind::Gate, true, Some(failures))
            if failures
                .iter()
                .any(|failure| raw_gate_name(&failure.gate_name) == raw_gate_name(gate)) =>
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
///
/// When `gate_adapter` is provided, the worker body delegates through the
/// shared [`RunnerProductionGateAdapter`] instead of calling `run_gate_once`
/// directly. This is production redirect #2 from #275.
pub fn spawn_gate(
    effect: GateEffectRef,
    plan_id: String,
    task_id: String,
    rung: u32,
    workdir: PathBuf,
    gates_config: GatesConfig,
    complexity: PlanComplexity,
    verify_steps: Vec<VerifyStep>,
    baseline_failed_gates: Option<Vec<GateVerdictSummary>>,
    timeout_secs: u64,
    gate_tx: mpsc::Sender<GateCompletion>,
    gate_sem: Arc<Semaphore>,
    target_crates: Vec<String>,
    verdict_publisher: Option<VerdictPublisher>,
    task_context: Option<GateTaskContext>,
    telemetry_sink: Option<Arc<dyn TelemetryEventSink>>,
    main_target_dir: Option<PathBuf>,
    expected_input_fingerprint: Option<String>,
    gate_adapter: Option<Arc<RunnerProductionGateAdapter>>,
    line_sink: Option<mpsc::UnboundedSender<String>>,
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
            if let Some(expected) = expected_input_fingerprint.as_deref() {
                let observed = owned_input_fingerprint_id(workdir.clone()).await?;
                if observed != expected {
                    return Err(
                        "timeout salvage input changed before ordinary gate start; refusing attribution"
                            .to_string(),
                    );
                }
            }
            // #275 redirect: when a shared gate adapter is available, delegate
            // through it instead of calling `run_gate_once` inline.
            let completion = if let Some(adapter) = gate_adapter {
                adapter
                    .run(
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
                        task_context,
                    )
                    .await
            } else {
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
                    line_sink,
                )
                .await
            };
            Ok::<_, String>(completion)
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

    let _compile_permit = acquire_compile_ownership(
        workdir,
        1,
        Duration::from_secs(300),
        "auto-fix",
        gate_name,
        &command_str,
    )
    .await?;

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
    baseline_failed_gates: Option<Vec<GateVerdictSummary>>,
    timeout_secs: u64,
    target_crates: Vec<String>,
    verdict_publisher: Option<VerdictPublisher>,
    task_context: Option<GateTaskContext>,
    telemetry_sink: Option<Arc<dyn TelemetryEventSink>>,
    main_target_dir: Option<PathBuf>,
    line_sink: Option<mpsc::UnboundedSender<String>>,
) -> GateCompletion {
    let start = Instant::now();
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

    let fast_mode = fast_mode_enabled();
    let cargo_profile =
        (fast_mode && cargo_profile_available(&workdir, "dev-fast")).then_some("dev-fast");
    let task_verify_only = task_verify_only_enabled();
    let mut task_verify_contract_error =
        fast_task_verify_contract_error(fast_mode, task_verify_only, verify_steps.len());
    let configured_mode = effective_gate_mode(gates_config.mode, fast_mode, task_verify_only);
    let planned_files = task_context
        .as_ref()
        .map(|context| context.planned_files.as_slice())
        .unwrap_or_default();
    let mut impact_report = if configured_mode == GateMode::Focused {
        let impact_limit = Duration::from_millis(gates_config.impact_timeout_ms.max(100));
        Some(
            match timeout(
                impact_limit,
                impact_analysis::analyze(&workdir, planned_files, &gates_config),
            )
            .await
            {
                Ok(report) => report,
                Err(_) => ImpactReport {
                    fallback_reason: Some(format!(
                        "impact analysis exceeded {} ms; full verification required",
                        gates_config.impact_timeout_ms.max(100)
                    )),
                    analysis_ms: gates_config.impact_timeout_ms.max(100),
                    ..ImpactReport::default()
                },
            },
        )
    } else {
        None
    };
    if gates_config.has_custom_rungs()
        && let Some(report) = impact_report.as_mut()
    {
        report.fallback_reason = Some(
            "custom required gate rungs cannot be replaced by focused inference; full verification required"
                .into(),
        );
    }
    let focused_fallback = impact_report
        .as_ref()
        .is_some_and(|report| report.fallback_reason.is_some());
    let gate_mode = if focused_fallback {
        GateMode::Full
    } else if impact_report
        .as_ref()
        .is_some_and(ImpactReport::is_structural_only)
    {
        GateMode::Structural
    } else {
        configured_mode
    };
    if let Some(report) = impact_report.as_ref() {
        info!(
            plan_id = %plan_id,
            task_id = %task_id,
            configured_mode = %configured_mode,
            effective_mode = %gate_mode,
            high_impact = report.high_impact,
            high_impact_reasons = ?report.high_impact_reasons,
            reverse_dependents = ?report.reverse_dependents,
            unplanned_changes = ?report.unplanned_changes,
            fallback_reason = ?report.fallback_reason,
            analysis_ms = report.analysis_ms,
            "gate impact policy selected"
        );
    } else {
        info!(
            plan_id = %plan_id,
            task_id = %task_id,
            configured_mode = %configured_mode,
            effective_mode = %gate_mode,
            "gate mode selected"
        );
    }

    if gate_mode == GateMode::None {
        let duration_ms = start.elapsed().as_millis() as u64;
        info!(
            plan_id = %plan_id,
            task_id = %task_id,
            rung,
            duration_ms,
            "gate disabled; completed without filesystem snapshots"
        );
        return GateCompletion {
            kind: effect.kind,
            attempt: Some(effect.attempt.clone()),
            effect: Some(effect),
            plan_id,
            task_id,
            rung,
            passed: true,
            failure_kind: None,
            verdicts: Vec::new(),
            output: String::new(),
            duration_ms,
            selected_rungs: Vec::new(),
        };
    }

    // A focused-analysis failure must not retain a producer-only payload from
    // the optimistic lane. Empty Cargo scope restores the historical
    // workspace-wide canonical gates and makes the fallback genuinely full.
    let gate_target_crates = if focused_fallback {
        Vec::new()
    } else {
        target_crates.clone()
    };
    let signal = gate_signal(
        &plan_id,
        &task_id,
        rung,
        &workdir,
        &gate_target_crates,
        main_target_dir.as_deref(),
    );

    let execute_pipeline = gate_mode == GateMode::Full && (!task_verify_only || focused_fallback);
    let targeted_check =
        (execute_pipeline && !focused_fallback && !gates_config.has_custom_rungs())
            .then(|| targeted_cargo_check(&workdir, &gate_target_crates))
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
    let mut canonical_commands = if !execute_pipeline {
        Vec::new()
    } else {
        canonical_verify_commands(
            &gates_config,
            complexity,
            &gate_target_crates,
            targeted_check.as_ref(),
        )
    };
    if !gates_config.has_custom_rungs()
        && let Some(profile) = cargo_profile
    {
        canonical_commands = canonical_commands
            .into_iter()
            .map(|command| cargo_command_with_profile(&command, profile).unwrap_or(command))
            .collect();
    }
    let verify_steps = if let Some(profile) = cargo_profile {
        verify_steps
            .into_iter()
            .map(|mut step| {
                if let Some(command) = cargo_command_with_profile(&step.command, profile) {
                    step.command = command;
                }
                step
            })
            .collect()
    } else {
        verify_steps
    };
    let authored_verify_steps = if fast_mode {
        deduplicate_verify_steps(&task_id, verify_steps, &canonical_commands)
    } else {
        verify_steps
    };
    let verify_steps = match gate_mode {
        GateMode::None => Vec::new(),
        GateMode::Structural => authored_verify_steps
            .into_iter()
            .filter(|step| {
                matches!(
                    step.phase.trim().to_ascii_lowercase().as_str(),
                    "structural" | "format" | "parse" | "diff"
                )
            })
            .collect(),
        GateMode::Focused => {
            let report = impact_report
                .as_ref()
                .expect("focused gate mode must have an impact report");
            let authored =
                scope_authored_verify_steps(&workdir, &task_id, authored_verify_steps, report);
            focused_verify_steps(report, &task_id, authored, timeout_secs, cargo_profile)
        }
        GateMode::Full => authored_verify_steps,
    };
    if task_verify_contract_error.is_none()
        && fast_mode
        && gate_mode == GateMode::Structural
        && verify_steps.is_empty()
    {
        task_verify_contract_error = Some(
            "FAST structural mode selected no structural/format/parse/diff verification; fail closed"
                .into(),
        );
    }
    let gates_config = with_targeted_compile_rung(
        &gates_config,
        complexity,
        targeted_check.as_ref(),
        timeout_secs,
        cargo_profile,
    );
    let selected_rungs = match gate_mode {
        GateMode::None => Vec::new(),
        GateMode::Structural => vec!["structural".to_string()],
        GateMode::Focused => {
            let mut labels = Vec::new();
            if verify_steps
                .iter()
                .any(|step| step.phase == "impact-compile")
            {
                labels.push("impact-compile".to_string());
            }
            if verify_steps
                .iter()
                .any(|step| step.phase != "impact-compile")
            {
                labels.push("task-verify".to_string());
            }
            labels
        }
        GateMode::Full => GatePipelineBuilder::selected_rung_labels(&gates_config, complexity),
    };
    let canonical_uses_cargo = canonical_commands
        .iter()
        .any(|command| command_uses_cargo(command))
        || (roko_gate::BuildSystem::detect(&workdir) == roko_gate::BuildSystem::Cargo
            && selected_rungs.iter().any(|rung| {
                matches!(
                    rung.as_str(),
                    "compile"
                        | "build"
                        | "check"
                        | "lint"
                        | "clippy"
                        | "test"
                        | "tests"
                        | "generated-test"
                        | "generated-tests"
                        | "gen-test"
                        | "property-test"
                        | "property-tests"
                        | "prop-test"
                        | "integration"
                        | "integration-test"
                )
            }));
    let baseline_verify_steps = (gate_mode == GateMode::Focused)
        .then(|| {
            verify_steps
                .iter()
                .filter(|step| step.phase != "impact-compile")
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

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

        let mut verdicts = Vec::new();
        if execute_pipeline {
            // Fetch the git diff for the LlmJudge gate so it can evaluate the
            // actual implementation rather than just the task description.
            let diff_text = fetch_git_diff(&workdir_for_run).await;
            let inputs = build_rung_execution_inputs(
                &gate_target_crates,
                task_context.as_ref(),
                diff_text.as_deref(),
            );
            let config = build_rung_execution_config(
                &workdir_for_run,
                timeout_secs,
                &verify_steps,
                verdict_publisher.clone(),
                line_sink.clone(),
            );
            let pipeline = GatePipelineBuilder::from_config_with_execution(
                &gates_config,
                complexity,
                inputs,
                config,
            );
            let command = canonical_commands.join(" && ");
            let compile_permit = if canonical_uses_cargo {
                match acquire_compile_ownership(
                    &workdir_for_run,
                    gates_config.compile_concurrency,
                    limit,
                    &plan_id,
                    &task_id,
                    &command,
                )
                .await
                {
                    Ok(permit) => Some(permit),
                    Err(error) => {
                        return vec![proof_failure!(
                            "compile-ownership",
                            error,
                            "canonical compile ownership unavailable",
                        )];
                    }
                }
            } else {
                None
            };
            let pipeline_started = Instant::now();
            let pipeline_verdict = pipeline.verify(&signal, &ctx).await;
            let (cache_hits, cache_misses) = cargo_cache_counts(&pipeline_verdict);
            verdicts.push(pipeline_verdict);
            drop(compile_permit);
            info!(
                plan_id = %plan_id,
                task_id = %task_id,
                command = %command,
                elapsed_ms = elapsed_millis(pipeline_started),
                cache_hits,
                cache_misses,
                cache_mode = if main_target_dir.is_some() {
                    "shared-target"
                } else {
                    "worktree-target"
                },
                "canonical gate command span complete"
            );
        }
        verdicts.extend(
            run_verify_steps(
                &signal,
                &ctx,
                &plan_id,
                &task_id,
                verify_steps,
                gates_config.compile_concurrency,
                line_sink.clone(),
            )
            .await,
        );
        verdicts
    };

    let checked = async {
        let before = gate_input_snapshot(workdir.clone()).await?;
        let mut verdicts = run.await;
        let lazy_baseline = if baseline_failed_gates.is_none()
            && gate_mode == GateMode::Focused
            && verdicts.iter().any(|verdict| {
                !verdict.passed
                    && !verdict.skipped
                    && verdict.gate.starts_with(&format!("task-verify:{task_id}:"))
            }) {
            run_focused_baseline_verify(
                &workdir,
                &plan_id,
                &task_id,
                rung,
                baseline_verify_steps,
                &gate_target_crates,
                main_target_dir.as_deref(),
                gates_config.compile_concurrency,
            )
            .await
        } else {
            None
        };
        let after = gate_input_snapshot(workdir.clone()).await?;

        // E45-T02: If the first run produced any failures, attempt cargo auto-fix
        // before we finalise. If the fix applied cleanly, rerun the gate pipeline
        // once and replace the verdicts so the caller sees a pass instead.
        // Gated on `gates_config.cargo_fix_enabled` (default: true).
        let first_run_failed = verdicts.iter().any(|v| !v.passed && !v.skipped);
        if first_run_failed && before == after && gates_config.cargo_fix_enabled && !fast_mode {
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
                    let mut retry_verdicts = Vec::new();
                    if execute_pipeline {
                        let diff_text_retry = fetch_git_diff(&workdir).await;
                        let inputs_retry = build_rung_execution_inputs(
                            &gate_target_crates,
                            task_context.as_ref(),
                            diff_text_retry.as_deref(),
                        );
                        let config_retry = build_rung_execution_config(
                            &workdir,
                            timeout_secs,
                            &verify_steps_for_retry,
                            verdict_publisher.clone(),
                            line_sink.clone(),
                        );
                        let pipeline_retry = GatePipelineBuilder::from_config_with_execution(
                            &gates_config,
                            complexity,
                            inputs_retry,
                            config_retry,
                        );
                        let command = canonical_commands.join(" && ");
                        let compile_permit = if canonical_uses_cargo {
                            match acquire_compile_ownership(
                                &workdir,
                                gates_config.compile_concurrency,
                                limit,
                                &plan_id,
                                &task_id,
                                &command,
                            )
                            .await
                            {
                                Ok(permit) => Some(permit),
                                Err(error) => {
                                    retry_verdicts.push(proof_failure!(
                                        "compile-ownership",
                                        error,
                                        "retry compile ownership unavailable",
                                    ));
                                    None
                                }
                            }
                        } else {
                            None
                        };
                        if retry_verdicts.is_empty() {
                            retry_verdicts.push(pipeline_retry.verify(&signal, &ctx).await);
                        }
                        drop(compile_permit);
                    }
                    retry_verdicts.extend(
                        run_verify_steps(
                            &signal,
                            &ctx,
                            &plan_id,
                            &task_id,
                            verify_steps_for_retry,
                            gates_config.compile_concurrency,
                            line_sink.clone(),
                        )
                        .await,
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

        Ok::<_, String>((before, after, verdicts, lazy_baseline))
    };
    let (input_before, mut verdicts, lazy_baseline) = match timeout(limit, checked).await {
        Ok(Ok((before, after, mut verdicts, lazy_baseline))) => {
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
            (Some(before), verdicts, lazy_baseline)
        }
        Ok(Err(error)) => (
            None,
            vec![proof_failure!(
                "unattributed:input-snapshot",
                format!("could not prove immutable gate input: {error}"),
                "gate input identity unavailable",
            )],
            None,
        ),
        Err(_) => (
            None,
            vec![proof_failure!(
                format!("unattributed:gate-timeout:rung-{rung}"),
                format!("gate timed out after {timeout_secs}s"),
                format!("timeout: gate rung {rung} exceeded {timeout_secs}s"),
            )],
            None,
        ),
    };
    let baseline_evidence = baseline_failed_gates
        .as_deref()
        .or(lazy_baseline.as_deref());
    filter_preexisting_failures(&task_id, &mut verdicts, baseline_evidence);
    if let Some(before) = input_before.as_ref() {
        for verdict in verdicts
            .iter_mut()
            .filter(|verdict| !verdict.passed && !verdict.gate.starts_with("unattributed:"))
        {
            let input = gate_failure_input(effect.kind, before, baseline_evidence, &verdict.gate);
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
    // appends it to engrams.jsonl.
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
    line_sink: Option<mpsc::UnboundedSender<String>>,
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

            let line_sink_for_run = line_sink;
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
                    all.extend(
                        run_verify_steps(&signal, &ctx, &plan_id_for_run, &task_id, steps, 1, line_sink_for_run.clone()).await,
                    );
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
    diff_text: Option<&str>,
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
    // When diff_text is available, the LlmJudgeGate can evaluate
    // whether the implementation matches the description. Without
    // it, the gate degrades gracefully (judges description only).
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
                diff: diff_text.unwrap_or("").to_string(),
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
/// task context. The `fact_check_oracle` is populated when the workspace
/// provides a Perplexity API key; other oracle fields remain `None` and the
/// rung dispatch fails closed with explicit skipped/not-wired verdicts when
/// required oracles are absent.
fn build_rung_execution_config(
    workdir: &Path,
    timeout_secs: u64,
    verify_steps: &[VerifyStep],
    verdict_publisher: Option<VerdictPublisher>,
    line_sink: Option<mpsc::UnboundedSender<String>>,
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

    // Wire the FactCheck oracle when a Perplexity API key is available.
    // Checks the environment directly (PERPLEXITY_API_KEY); the oracle is
    // `None` when absent, causing the gate to return Skipped as before.
    let fact_check_oracle: Option<Arc<dyn roko_gate::fact_check::SearchOracle>> =
        std::env::var("PERPLEXITY_API_KEY")
            .ok()
            .filter(|key| !key.is_empty())
            .map(|key| {
                Arc::new(super::gate_oracles::PerplexitySearchOracle::new(key))
                    as Arc<dyn roko_gate::fact_check::SearchOracle>
            });

    RungExecutionConfig {
        source_roots: Some(vec![workdir.to_path_buf()]),
        timeout_ms: Some(timeout_secs.saturating_mul(1000)),
        integration_test_pattern,
        integration_build_system,
        generated_test_artifacts,
        verdict_publisher,
        fact_check_oracle,
        line_sink,
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

    if fast_mode_enabled() {
        // Tauri's build script otherwise invokes a frontend build from Cargo,
        // duplicating work that the evidence owner runs explicitly.
        payload = payload.with_env("SKIP_FRONTEND_BUILD", "1");
        if cargo_profile_available(workdir, "dev-fast") {
            payload = payload.with_cargo_profile("dev-fast");
        }
    }

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
    plan_id: &str,
    task_id: &str,
    verify_steps: Vec<VerifyStep>,
    compile_concurrency: usize,
    line_sink: Option<mpsc::UnboundedSender<String>>,
) -> Vec<Verdict> {
    let payload = signal.body.as_json::<GatePayload>().ok();
    let workdir = payload
        .as_ref()
        .map(|payload| payload.working_dir.as_path())
        .unwrap_or_else(|| Path::new("."));
    let mut verdicts = Vec::new();
    for (i, step) in verify_steps.iter().enumerate() {
        let mut effective_step = step.clone();
        if let Some(profile) = payload
            .as_ref()
            .and_then(|payload| payload.cargo_profile.as_deref())
            && let Some(command) = cargo_command_with_profile(&step.command, profile)
        {
            effective_step.command = command;
        }
        let step_start = Instant::now();
        let compile_permit = if command_uses_cargo(&effective_step.command) {
            match acquire_compile_ownership(
                workdir,
                compile_concurrency,
                Duration::from_millis(effective_step.timeout_ms.max(1)),
                plan_id,
                task_id,
                &effective_step.command,
            )
            .await
            {
                Ok(permit) => Some(permit),
                Err(error) => {
                    verdicts.push(
                        Verdict::fail(format!("task-verify:{task_id}:compile-ownership"), error)
                            .with_error_digest("compile ownership unavailable"),
                    );
                    break;
                }
            }
        } else {
            None
        };
        let gate = verify_step_gate(task_id, &effective_step, line_sink.clone());
        let verdict = gate.verify(signal, ctx).await;
        let (cache_hits, cache_misses) = cargo_cache_counts(&verdict);
        drop(compile_permit);
        info!(
            plan_id,
            task_id = %task_id,
            step = i + 1,
            total_steps = verify_steps.len(),
            phase = %effective_step.phase,
            command = %effective_step.command,
            timeout_ms = effective_step.timeout_ms,
            passed = verdict.passed,
            elapsed_ms = elapsed_millis(step_start),
            cache_mode = if payload.as_ref().is_some_and(|payload| payload.target_dir.is_some()) {
                "shared-target"
            } else {
                "worktree-target"
            },
            cache_hits,
            cache_misses,
            "verify step completed"
        );
        verdicts.push(verdict);
    }
    verdicts
}

async fn run_focused_baseline_verify(
    workdir: &Path,
    plan_id: &str,
    task_id: &str,
    rung: u32,
    steps: Vec<VerifyStep>,
    target_crates: &[String],
    main_target_dir: Option<&Path>,
    compile_concurrency: usize,
) -> Option<Vec<GateVerdictSummary>> {
    let steps = steps
        .into_iter()
        .filter(|step| {
            cargo_command_fingerprint(&step.command)
                .is_some_and(|fingerprint| fingerprint.action == "test")
        })
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return None;
    }
    let parent = tempfile::Builder::new()
        .prefix("roko-gate-baseline-")
        .tempdir()
        .ok()?;
    let mut baseline_guard = RegisteredBaselineWorktree::new(workdir, parent);
    let baseline = baseline_guard.checkout.clone();
    // `git worktree add` can be interrupted after registration but before a
    // successful exit. Arm cleanup before spawning so timeout/cancellation
    // cannot leave a stale Git worktree record behind.
    baseline_guard.cleanup_required = true;
    let add = timeout(
        Duration::from_secs(10),
        Command::new("git")
            .args(["worktree", "add", "--detach"])
            .arg(&baseline)
            .arg("HEAD")
            .current_dir(workdir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    if !add.status.success() {
        return None;
    }

    let signal = gate_signal(
        plan_id,
        task_id,
        rung,
        &baseline,
        target_crates,
        main_target_dir,
    );
    let ctx = roko_core::Context::now();
    let verdicts =
        run_verify_steps(&signal, &ctx, plan_id, task_id, steps, compile_concurrency, None).await;
    let removal = timeout(
        Duration::from_secs(10),
        Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(&baseline)
            .current_dir(workdir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .output(),
    )
    .await;
    if matches!(removal, Ok(Ok(ref output)) if output.status.success()) {
        baseline_guard.cleanup_required = false;
    } else {
        warn!(path = %baseline.display(), "failed to remove temporary baseline worktree cleanly");
    }
    Some(
        verdicts
            .into_iter()
            .map(|verdict| GateVerdictSummary {
                gate_name: verdict.gate,
                passed: verdict.passed,
                skipped: verdict.skipped,
                summary: verdict.reason,
                error_digest: verdict.error_digest,
                failure_kind: None,
                rung_index: None,
            })
            .collect(),
    )
}

/// Cancellation-safe owner for a temporary registered Git worktree.
///
/// The normal path uses bounded async cleanup above. Drop is only a fallback
/// for timeout, cancellation, panic, or a failed async removal. If Git refuses
/// the fallback removal, the checkout is preserved rather than recursively
/// deleted while Git may still consider it registered.
struct RegisteredBaselineWorktree {
    repository: PathBuf,
    checkout: PathBuf,
    parent: Option<tempfile::TempDir>,
    cleanup_required: bool,
}

impl RegisteredBaselineWorktree {
    fn new(repository: &Path, parent: tempfile::TempDir) -> Self {
        let checkout = parent.path().join("checkout");
        Self {
            repository: repository.to_path_buf(),
            checkout,
            parent: Some(parent),
            cleanup_required: false,
        }
    }
}

impl Drop for RegisteredBaselineWorktree {
    fn drop(&mut self) {
        if !self.cleanup_required {
            return;
        }
        let removed = std::process::Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(&self.checkout)
            .current_dir(&self.repository)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if removed {
            return;
        }

        if self.checkout.exists() {
            if let Some(parent) = self.parent.take() {
                let retained = parent.keep();
                warn!(
                    path = %retained.display(),
                    "preserving temporary baseline worktree after cleanup failure"
                );
            }
        } else {
            // The checkout may have disappeared during a partially completed
            // add/remove. Prune only stale administrative records before the
            // TempDir owner removes its now-unregistered parent.
            let _ = std::process::Command::new("git")
                .args(["worktree", "prune", "--expire", "now"])
                .current_dir(&self.repository)
                .env("GIT_TERMINAL_PROMPT", "0")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
    }
}

fn verify_step_gate(
    task_id: &str,
    step: &VerifyStep,
    line_sink: Option<mpsc::UnboundedSender<String>>,
) -> ShellGate {
    let mut gate = ShellGate::new(
        "bash",
        vec![
            "-o".into(),
            "pipefail".into(),
            "-c".into(),
            step.command.clone(),
        ],
    )
    .with_name(format!("task-verify:{}:{}", task_id, step.phase))
    .with_timeout_ms(step.timeout_ms);
    if let Some(sink) = line_sink {
        gate = gate.with_line_sink(sink);
    }
    gate
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
                    | RunnerFailureKind::ContextOverflow
                    | RunnerFailureKind::Unknown => RunnerFailureKind::Structural,
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RunnerProductionGateAdapter (#275)
// ═══════════════════════════════════════════════════════════════════════════

/// Adapter that converts Runner-v2 gate parameters into a
/// [`ProductionGateRequest`], calls the injected
/// [`ProductionGateRunner`], and converts the
/// [`ProductionGateVerdictV1`] back into a [`GateCompletion`].
///
/// This is the single point of conversion between the Runner-v2 types
/// (which own event-loop integration, attempt ownership, and TUI events)
/// and the shared production gate service (which owns rung selection,
/// execution, and verdict normalization).
///
/// ## Call-site manifest
///
/// Four production boundaries redirect through this adapter:
///
/// 1. `run_gate_once` — delegates to `Self::run` instead of inline rung execution.
/// 2. `spawn_gate` worker body — the spawned task calls `Self::run`.
/// 3. Preflight spawn branch in `event_loop.rs` — injects the same shared service.
/// 4. Normal/plan-verify spawn branch in `event_loop.rs` — injects the same shared service.
pub struct RunnerProductionGateAdapter {
    /// The injected shared gate service.
    service: Arc<dyn roko_gate::production_service::ProductionGateRunner>,
}

impl std::fmt::Debug for RunnerProductionGateAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnerProductionGateAdapter")
            .finish_non_exhaustive()
    }
}

impl RunnerProductionGateAdapter {
    /// Create an adapter wrapping the given shared service.
    pub fn new(service: Arc<dyn roko_gate::production_service::ProductionGateRunner>) -> Self {
        Self { service }
    }

    /// Convert Runner-v2 parameters into a `ProductionGateRequest`.
    fn build_request(
        effect: &GateEffectRef,
        plan_id: &str,
        task_id: &str,
        workdir: &Path,
        gates_config: &GatesConfig,
        verify_steps: &[VerifyStep],
        timeout_secs: u64,
        target_crates: &[String],
        task_context: Option<&GateTaskContext>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> roko_gate::ProductionGateRequest {
        // Convert CLI VerifyStep -> neutral VerifyStepSpec.
        let verify_step_specs: Vec<roko_gate::VerifyStepSpec> = verify_steps
            .iter()
            .map(|step| {
                roko_gate::VerifyStepSpec::from_command(&step.command)
                    .with_phase(&step.phase)
                    .with_timeout_ms(step.timeout_ms)
            })
            .collect();

        // Convert GateTaskContext -> GateTaskContextSpec.
        let task_context_spec = task_context
            .map(|ctx| roko_gate::GateTaskContextSpec {
                title: ctx.task_title.clone(),
                description: ctx.task_description.clone(),
                symbols: ctx.symbols.clone(),
                acceptance: ctx.acceptance.clone(),
            })
            .unwrap_or_default();

        // Compute workspace fingerprint synchronously from the workdir.
        let workspace_fingerprint = format!("{}:{}:{}", plan_id, task_id, effect.generation);

        roko_gate::ProductionGateRequest {
            run_id: format!("{}:{}", plan_id, effect.generation),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            attempt: effect.attempt.attempt,
            workspace: workdir.to_path_buf(),
            workspace_fingerprint,
            changed_files: target_crates.to_vec(),
            verify_steps: verify_step_specs,
            gates_config: gates_config.clone(),
            task_context: task_context_spec,
            timeout_secs,
            cancel,
            baseline_fingerprint: None,
            adaptive_thresholds: None,
        }
    }

    /// Convert a `ProductionGateVerdictV1` back into a `GateCompletion`.
    fn verdict_to_completion(
        effect: GateEffectRef,
        plan_id: String,
        task_id: String,
        rung: u32,
        verdict: &roko_gate::ProductionGateVerdictV1,
    ) -> GateCompletion {
        let passed = verdict.passed();

        // Map per-rung verdicts to GateVerdictSummary.
        let summaries: Vec<GateVerdictSummary> = verdict
            .rung_verdicts
            .iter()
            .map(|rv| {
                let failure_kind = if rv.skipped() || rv.passed() {
                    None
                } else {
                    rv.failure_classification
                        .as_ref()
                        .map(|fc| match fc.recommended_action {
                            roko_gate::GateFailureAction::Blocked => RunnerFailureKind::Resource,
                            roko_gate::GateFailureAction::NeedsHuman => {
                                RunnerFailureKind::Permanent
                            }
                            roko_gate::GateFailureAction::NeedsReplan => {
                                RunnerFailureKind::Structural
                            }
                            roko_gate::GateFailureAction::Retry => RunnerFailureKind::Transient,
                        })
                        .or(Some(RunnerFailureKind::Unknown))
                };
                GateVerdictSummary {
                    gate_name: rv.gate_name.clone(),
                    passed: rv.passed(),
                    skipped: rv.skipped(),
                    summary: rv.diagnostic.chars().take(500).collect(),
                    error_digest: rv
                        .failure_classification
                        .as_ref()
                        .map(|fc| format!("{:?}", fc.primary)),
                    failure_kind,
                    rung_index: Some(rv.rung.as_index()),
                }
            })
            .collect();

        let selected_rungs: Vec<String> = verdict
            .rung_verdicts
            .iter()
            .filter(|rv| !rv.skipped())
            .map(|rv| rv.rung.label().to_string())
            .collect();

        let failure_kind = if !passed {
            summaries
                .iter()
                .find_map(|s| s.failure_kind)
                .or(Some(RunnerFailureKind::Unknown))
        } else {
            None
        };

        // Collect output from rung diagnostics.
        let output: String = verdict
            .rung_verdicts
            .iter()
            .filter(|rv| !rv.diagnostic.is_empty())
            .map(|rv| format!("{}: {}", rv.gate_name, rv.diagnostic))
            .collect::<Vec<_>>()
            .join("; ");

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
            duration_ms: verdict.total_duration.as_millis() as u64,
            selected_rungs,
        }
    }

    /// Run the production gate pipeline through the shared service and return
    /// a `GateCompletion` compatible with the Runner-v2 event loop.
    ///
    /// This is the primary entry point that replaces the inline execution in
    /// `run_gate_once`. The existing `run_gate_once` delegates to this method
    /// when a `RunnerProductionGateAdapter` is available.
    pub async fn run(
        &self,
        effect: GateEffectRef,
        plan_id: String,
        task_id: String,
        rung: u32,
        workdir: PathBuf,
        gates_config: GatesConfig,
        _complexity: PlanComplexity,
        verify_steps: Vec<VerifyStep>,
        _baseline_failed_gates: Option<Vec<GateVerdictSummary>>,
        timeout_secs: u64,
        target_crates: Vec<String>,
        task_context: Option<GateTaskContext>,
    ) -> GateCompletion {
        let cancel = tokio_util::sync::CancellationToken::new();
        let request = Self::build_request(
            &effect,
            &plan_id,
            &task_id,
            &workdir,
            &gates_config,
            &verify_steps,
            timeout_secs,
            &target_crates,
            task_context.as_ref(),
            cancel,
        );

        let progress = Arc::new(roko_gate::production_service::NoopProgressSink);
        match self.service.run(request, progress).await {
            Ok(verdict) => Self::verdict_to_completion(effect, plan_id, task_id, rung, &verdict),
            Err(err) => {
                error!(%err, "production gate service error");
                failed_gate_completion(
                    effect,
                    plan_id,
                    task_id,
                    rung,
                    format!("production gate service error: {err}"),
                )
            }
        }
    }
}

/// Create a default `RunnerProductionGateAdapter` with the production service.
///
/// Used by the event loop when no custom service is injected.
pub fn default_gate_adapter() -> RunnerProductionGateAdapter {
    RunnerProductionGateAdapter::new(Arc::new(
        roko_gate::production_service::ProductionGateService::new(),
    )
        as Arc<dyn roko_gate::production_service::ProductionGateRunner>)
}

// ── Generated-test artifact store ───────────────────────────────────────

/// Filesystem-backed store for generated test artifacts, keyed by plan.
#[derive(Clone, Debug)]
pub(crate) struct FsGeneratedArtifactStore {
    root: PathBuf,
}

impl FsGeneratedArtifactStore {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn artifact_dir(&self) -> PathBuf {
        self.root.join("generated-tests")
    }

    pub(crate) fn matching_entries(&self, prefix: &str) -> Vec<String> {
        let dir = self.artifact_dir();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };

        let mut names: Vec<String> = entries
            .filter_map(std::result::Result::ok)
            .filter_map(|entry| {
                entry.file_type().ok().filter(|kind| kind.is_file())?;
                let name = entry.file_name().to_string_lossy().into_owned();
                let logical = format!("generated-tests/{name}");
                logical.starts_with(prefix).then_some(logical)
            })
            .collect();
        names.sort();
        names
    }
}

impl GeneratedArtifactStore for FsGeneratedArtifactStore {
    fn list(&self, _plan: &str, prefix: &str) -> Vec<String> {
        self.matching_entries(prefix)
    }

    fn read(&self, _plan: &str, name: &str) -> Option<Vec<u8>> {
        let relative = name.strip_prefix("generated-tests/")?;
        if relative.contains("..") || relative.contains('/') {
            return None;
        }
        std::fs::read(self.artifact_dir().join(relative)).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;
    use std::fs::File;
    use std::sync::Mutex;

    use super::super::types::TaskAttemptRef;

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

    #[test]
    fn fast_profile_is_inserted_before_tool_arguments() {
        assert_eq!(
            cargo_command_with_profile("cargo clippy -p roko-cli -- -D warnings", "dev-fast")
                .as_deref(),
            Some("cargo clippy -p roko-cli --profile dev-fast -- -D warnings")
        );
        assert_eq!(
            cargo_command_with_profile("cargo test -p roko-cli --profile custom", "dev-fast")
                .as_deref(),
            Some("cargo test -p roko-cli --profile custom")
        );
        assert!(cargo_command_with_profile("cargo test && echo done", "dev-fast").is_none());
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
            None, // expected_input_fingerprint
            None, // gate_adapter
            None, // line_sink
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
            None, // line_sink
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
            None, // line_sink
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
            None, // expected_input_fingerprint
            None, // gate_adapter
            None, // line_sink
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

        let verdicts = run_verify_steps(&signal, &ctx, "plan", "T01", vec![step], 1, None).await;

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

        let verdicts = run_verify_steps(&signal, &ctx, "plan", "T01", vec![step], 1, None).await;

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
    async fn unchanged_preexisting_failure_is_filtered() {
        let dir = git_repo();
        let gates = GatesConfig {
            cargo_fix_enabled: false,
            custom_rungs: vec![roko_core::config::GateRungConfig {
                name: "fixture-pass".into(),
                command: "true".into(),
                timeout_secs: 10,
                required: true,
                parallel_with: Vec::new(),
            }],
            ..GatesConfig::default()
        };
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
            gates.clone(),
            PlanComplexity::Trivial,
            vec![step.clone()],
            None,
            10,
            Vec::new(),
            None,
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
            .cloned()
            .collect();
        std::fs::write(dir.path().join("candidate.txt"), b"owned\n").unwrap();
        let candidate = run_gate_once(
            gate_effect(GateCompletionKind::Gate),
            "plan".into(),
            "task".into(),
            1,
            dir.path().to_path_buf(),
            gates,
            PlanComplexity::Trivial,
            vec![step],
            Some(baseline_failures),
            10,
            Vec::new(),
            None,
            None,
            None,
            None,
            None,
        )
        .await;
        assert!(candidate.passed);
        assert!(candidate.verdicts.iter().any(|verdict| {
            verdict.passed && verdict.gate_name.starts_with("pre-existing-filtered:")
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

    // ─── RunnerProductionGateAdapter tests (#275) ────────────────────────

    /// Fake gate runner that returns a canned verdict for adapter tests.
    #[derive(Debug)]
    struct FakeGateRunner {
        passed: bool,
    }

    #[async_trait::async_trait]
    impl roko_gate::production_service::ProductionGateRunner for FakeGateRunner {
        async fn run(
            &self,
            request: roko_gate::ProductionGateRequest,
            _progress: Arc<dyn roko_gate::production_service::ProgressSink>,
        ) -> roko_core::Result<roko_gate::ProductionGateVerdictV1> {
            use roko_gate::production_verdict::{
                EvidenceRef, PipelineOutcome, ProductionGateRungVerdict as ProdRV, RungState,
                VERDICT_SCHEMA_VERSION,
            };
            use roko_gate::rung_selector::Rung;

            let state = if self.passed {
                RungState::Passed
            } else {
                RungState::Failed
            };
            Ok(roko_gate::ProductionGateVerdictV1 {
                schema_version: VERDICT_SCHEMA_VERSION,
                request_fingerprint: request.workspace_fingerprint.clone(),
                workspace_fingerprint: request.workspace_fingerprint,
                rung_verdicts: vec![ProdRV {
                    rung: Rung::Compile,
                    gate_name: "compile".into(),
                    state,
                    failure_classification: None,
                    diagnostic: if self.passed {
                        "all good".into()
                    } else {
                        "error[E0433]".into()
                    },
                    evidence: EvidenceRef::default(),
                    duration: std::time::Duration::from_millis(42),
                    test_counts: None,
                    input_fingerprint: String::new(),
                    skip_reason: None,
                }],
                outcome: if self.passed {
                    PipelineOutcome::Passed
                } else {
                    PipelineOutcome::Failed
                },
                mostly_passing: false,
                total_duration: std::time::Duration::from_millis(42),
                adaptive_snapshot: None,
            })
        }
    }

    #[test]
    fn adapter_build_request_converts_verify_steps() {
        let effect = gate_effect(GateCompletionKind::Gate);
        let steps = vec![VerifyStep {
            phase: "test".into(),
            command: "cargo test".into(),
            fail_msg: Some("tests failed".into()),
            timeout_ms: 60_000,
        }];
        let cancel = tokio_util::sync::CancellationToken::new();
        let request = RunnerProductionGateAdapter::build_request(
            &effect,
            "plan-1",
            "task-1",
            Path::new("/tmp/ws"),
            &GatesConfig::default(),
            &steps,
            600,
            &["roko-core".to_string()],
            None,
            cancel,
        );
        assert_eq!(request.plan_id, "plan-1");
        assert_eq!(request.task_id, "task-1");
        assert_eq!(request.verify_steps.len(), 1);
        assert_eq!(request.verify_steps[0].phase, "test");
        assert_eq!(request.verify_steps[0].command, "cargo test");
        assert_eq!(request.timeout_secs, 600);
    }

    #[test]
    fn adapter_build_request_converts_task_context() {
        let effect = gate_effect(GateCompletionKind::Gate);
        let ctx = GateTaskContext {
            plan_id: "p1".into(),
            symbols: vec!["Foo::bar".into()],
            acceptance: vec!["Must compile".into()],
            task_description: Some("Implement bar".into()),
            task_title: "Bar task".into(),
            planned_files: vec!["src/lib.rs".into()],
        };
        let cancel = tokio_util::sync::CancellationToken::new();
        let request = RunnerProductionGateAdapter::build_request(
            &effect,
            "plan-1",
            "task-1",
            Path::new("/tmp/ws"),
            &GatesConfig::default(),
            &[],
            600,
            &[],
            Some(&ctx),
            cancel,
        );
        assert_eq!(request.task_context.title, "Bar task");
        assert_eq!(
            request.task_context.description.as_deref(),
            Some("Implement bar")
        );
        assert_eq!(request.task_context.symbols, vec!["Foo::bar"]);
        assert_eq!(request.task_context.acceptance, vec!["Must compile"]);
    }

    #[test]
    fn adapter_verdict_to_completion_pass() {
        use roko_gate::production_verdict::{
            EvidenceRef, PipelineOutcome, ProductionGateRungVerdict as ProdRV, RungState,
            VERDICT_SCHEMA_VERSION,
        };
        use roko_gate::rung_selector::Rung;

        let effect = gate_effect(GateCompletionKind::Gate);
        let verdict = roko_gate::ProductionGateVerdictV1 {
            schema_version: VERDICT_SCHEMA_VERSION,
            request_fingerprint: "fp".into(),
            workspace_fingerprint: "fp".into(),
            rung_verdicts: vec![
                ProdRV {
                    rung: Rung::Compile,
                    gate_name: "compile".into(),
                    state: RungState::Passed,
                    failure_classification: None,
                    diagnostic: "ok".into(),
                    evidence: EvidenceRef::default(),
                    duration: std::time::Duration::from_millis(10),
                    test_counts: None,
                    input_fingerprint: String::new(),
                    skip_reason: None,
                },
                ProdRV {
                    rung: Rung::Lint,
                    gate_name: "clippy".into(),
                    state: RungState::Skipped,
                    failure_classification: None,
                    diagnostic: String::new(),
                    evidence: EvidenceRef::default(),
                    duration: std::time::Duration::ZERO,
                    test_counts: None,
                    input_fingerprint: String::new(),
                    skip_reason: Some("adaptive skip".into()),
                },
            ],
            outcome: PipelineOutcome::Passed,
            mostly_passing: false,
            total_duration: std::time::Duration::from_millis(10),
            adaptive_snapshot: None,
        };

        let completion = RunnerProductionGateAdapter::verdict_to_completion(
            effect,
            "plan-1".into(),
            "task-1".into(),
            2,
            &verdict,
        );
        assert!(completion.passed);
        assert_eq!(completion.verdicts.len(), 2);
        assert!(completion.verdicts[0].passed);
        assert!(completion.verdicts[1].skipped);
        assert!(completion.failure_kind.is_none());
        assert_eq!(completion.selected_rungs, vec!["compile"]);
    }

    #[test]
    fn adapter_verdict_to_completion_fail() {
        use roko_gate::production_verdict::{
            EvidenceRef, PipelineOutcome, ProductionGateRungVerdict as ProdRV, RungState,
            VERDICT_SCHEMA_VERSION,
        };
        use roko_gate::rung_selector::Rung;

        let effect = gate_effect(GateCompletionKind::Gate);
        let verdict = roko_gate::ProductionGateVerdictV1 {
            schema_version: VERDICT_SCHEMA_VERSION,
            request_fingerprint: "fp".into(),
            workspace_fingerprint: "fp".into(),
            rung_verdicts: vec![ProdRV {
                rung: Rung::Test,
                gate_name: "test".into(),
                state: RungState::Failed,
                failure_classification: None,
                diagnostic: "test failed".into(),
                evidence: EvidenceRef::default(),
                duration: std::time::Duration::from_millis(500),
                test_counts: Some(roko_core::TestCount::new(10, 2, 0)),
                input_fingerprint: String::new(),
                skip_reason: None,
            }],
            outcome: PipelineOutcome::Failed,
            mostly_passing: false,
            total_duration: std::time::Duration::from_millis(500),
            adaptive_snapshot: None,
        };

        let completion = RunnerProductionGateAdapter::verdict_to_completion(
            effect,
            "plan-1".into(),
            "task-1".into(),
            2,
            &verdict,
        );
        assert!(!completion.passed);
        assert!(completion.failure_kind.is_some());
        assert_eq!(completion.verdicts.len(), 1);
        assert!(!completion.verdicts[0].passed);
        assert_eq!(completion.selected_rungs, vec!["test"]);
    }

    #[tokio::test]
    async fn adapter_run_delegates_to_service() {
        let adapter = RunnerProductionGateAdapter::new(Arc::new(FakeGateRunner { passed: true }));
        let effect = gate_effect(GateCompletionKind::Gate);
        let completion = adapter
            .run(
                effect,
                "plan-1".into(),
                "task-1".into(),
                2,
                PathBuf::from("/tmp/ws"),
                GatesConfig::default(),
                PlanComplexity::Trivial,
                vec![],
                None,
                600,
                vec![],
                None,
            )
            .await;
        assert!(completion.passed);
        assert_eq!(completion.plan_id, "plan-1");
        assert_eq!(completion.task_id, "task-1");
    }

    #[tokio::test]
    async fn adapter_run_failing_service() {
        let adapter = RunnerProductionGateAdapter::new(Arc::new(FakeGateRunner { passed: false }));
        let effect = gate_effect(GateCompletionKind::Gate);
        let completion = adapter
            .run(
                effect,
                "plan-fail".into(),
                "task-fail".into(),
                2,
                PathBuf::from("/tmp/ws"),
                GatesConfig::default(),
                PlanComplexity::Trivial,
                vec![],
                None,
                600,
                vec![],
                None,
            )
            .await;
        assert!(!completion.passed);
        assert!(completion.failure_kind.is_some());
    }

    #[test]
    fn default_gate_adapter_creates_valid_adapter() {
        let adapter = default_gate_adapter();
        let debug = format!("{adapter:?}");
        assert!(debug.contains("RunnerProductionGateAdapter"));
    }
}
