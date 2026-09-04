//! Cargo command parsing, fingerprinting, and profile utilities.
//!
//! Extracted from `gate_dispatch.rs` to keep command-level helpers separate
//! from the gate execution pipeline.

use std::collections::BTreeSet;
use std::path::Path;

use roko_core::config::{GateRungConfig, GatesConfig};
use roko_gate::rung_dispatch::GatePipelineBuilder;
use roko_gate::PlanComplexity;

use crate::task_parser::VerifyStep;

// ── Cargo target selector ───────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum CargoTargetSelector {
    Lib,
    Bin(String),
    Test(String),
}

impl CargoTargetSelector {
    pub(super) fn command_args(&self) -> (&'static str, Option<&str>) {
        match self {
            Self::Lib => ("--lib", None),
            Self::Bin(name) => ("--bin", Some(name)),
            Self::Test(name) => ("--test", Some(name)),
        }
    }
}

// ── Targeted cargo check ────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TargetedCargoCheck {
    pub(super) package: String,
    pub(super) target: CargoTargetSelector,
    pub(super) command: String,
}

// ── Cargo command fingerprint ───────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CargoCommandFingerprint {
    pub(super) action: String,
    pub(super) arguments: Vec<String>,
    pub(super) tool_arguments: Vec<String>,
}

// ── Utility helpers ─────────────────────────────────────────────────────

pub(super) fn safe_cargo_name(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

pub(super) fn simple_command_tokens(command: &str) -> Option<Vec<&str>> {
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

pub(super) fn command_uses_cargo(command: &str) -> bool {
    cargo_command_fingerprint(command).is_some()
        || command
            .split(|character: char| character.is_ascii_whitespace() || ";&|()".contains(character))
            .any(|token| token == "cargo")
}

// ── Profile insertion ───────────────────────────────────────────────────

/// Add an explicit Cargo profile to a simple runner-owned gate command.
///
/// Environment variables such as `CARGO_PROFILE_DEV_*` only configure a
/// profile; Cargo does not select that profile unless the command includes
/// `--profile`. Shell composition and quoted commands are intentionally left
/// untouched because rewriting them safely requires a shell parser.
pub(super) fn cargo_command_with_profile(command: &str, profile: &str) -> Option<String> {
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

pub(super) fn cargo_profile_available(workdir: &Path, profile: &str) -> bool {
    std::fs::read_to_string(workdir.join("Cargo.toml"))
        .ok()
        .and_then(|manifest| toml::from_str::<toml::Value>(&manifest).ok())
        .and_then(|manifest| manifest.get("profile").cloned())
        .and_then(|profiles| profiles.get(profile).cloned())
        .is_some_and(|profile| profile.is_table())
}

// ── Cargo command fingerprint ───────────────────────────────────────────

/// Normalize simple Cargo verification commands while deliberately rejecting
/// shell composition.  Only presentation/cache flags are ignored; flags that
/// can change what is compiled remain part of the fingerprint.
pub(super) fn cargo_command_fingerprint(command: &str) -> Option<CargoCommandFingerprint> {
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

// ── Scope and canonical commands ────────────────────────────────────────

pub(super) fn default_cargo_scope(target_crates: &[String]) -> String {
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

pub(super) fn canonical_verify_commands(
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

// ── Verify step deduplication ───────────────────────────────────────────

pub(super) fn deduplicate_verify_steps(
    task_id: &str,
    verify_steps: Vec<VerifyStep>,
    canonical_commands: &[String],
) -> Vec<VerifyStep> {
    use tracing::info;

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

// ── Git changed files ───────────────────────────────────────────────────

pub(super) fn git_changed_files(workdir: &Path) -> Option<Vec<String>> {
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

// ── Manifest target resolution ──────────────────────────────────────────

pub(super) fn cargo_manifest_for_file(
    workdir: &Path,
    file: &str,
) -> Option<(std::path::PathBuf, String)> {
    let relative = std::path::Path::new(file);
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

pub(super) fn manifest_target_for_path(
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

// ── Targeted cargo check ────────────────────────────────────────────────

/// Select a single Cargo target only when FAST mode can prove every changed
/// Rust file belongs to that exact target.  Module files are intentionally
/// ambiguous because Cargo metadata does not reveal which roots include them.
pub(super) fn targeted_cargo_check(
    workdir: &Path,
    target_crates: &[String],
) -> Option<TargetedCargoCheck> {
    if !super::gate_dispatch::fast_mode_enabled() {
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

    let mut selected: Option<(std::path::PathBuf, CargoTargetSelector)> = None;
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

// ── Targeted compile rung ───────────────────────────────────────────────

pub(super) fn with_targeted_compile_rung(
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

// ── Focused/scoped verify steps ─────────────────────────────────────────

pub(super) fn scoped_test_command(
    workdir: &Path,
    command: &str,
    report: &super::impact_analysis::ImpactReport,
) -> Option<String> {
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
        super::impact_analysis::CargoTargetSelector::Test(name) => {
            Some(format!("{} --test {name}", command.trim()))
        }
        super::impact_analysis::CargoTargetSelector::Lib => {
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

pub(super) fn scope_authored_verify_steps(
    workdir: &Path,
    task_id: &str,
    steps: Vec<VerifyStep>,
    report: &super::impact_analysis::ImpactReport,
) -> Vec<VerifyStep> {
    use tracing::info;

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

pub(super) fn focused_verify_steps(
    report: &super::impact_analysis::ImpactReport,
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
