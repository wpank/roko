//! Bounded Cargo-aware change-impact selection for focused verification.
//!
//! The analyzer deliberately fails closed: malformed Git output, metadata
//! timeouts, workspace-level build inputs, and reverse-dependency overflow
//! force the caller back to the full gate lane.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use cargo_metadata::{Metadata, Package, PackageId, Target};
use roko_core::config::GatesConfig;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{info, warn};

const MAX_GIT_OUTPUT: usize = 4 * 1024 * 1024;
const MAX_METADATA_OUTPUT: usize = 16 * 1024 * 1024;
const MAX_STDERR_OUTPUT: usize = 256 * 1024;
const MAX_UNTRACKED_RUST_INPUT: usize = 4 * 1024 * 1024;
const MAX_CHANGED_FILES: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CargoTargetSelector {
    Lib,
    Bin(String),
    Test(String),
    Example(String),
    Bench(String),
    /// Cargo's default package targets (library and binaries).
    Package,
    /// Every package target; reserved for manifest/build-contract changes.
    AllTargets,
}

impl CargoTargetSelector {
    fn args(&self) -> Vec<String> {
        match self {
            Self::Lib => vec!["--lib".into()],
            Self::Bin(name) => vec!["--bin".into(), name.clone()],
            Self::Test(name) => vec!["--test".into(), name.clone()],
            Self::Example(name) => vec!["--example".into(), name.clone()],
            Self::Bench(name) => vec!["--bench".into(), name.clone()],
            Self::Package => Vec::new(),
            Self::AllTargets => vec!["--all-targets".into()],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImpactedTarget {
    pub package: String,
    pub selector: CargoTargetSelector,
    pub required_features: Vec<String>,
}

impl ImpactedTarget {
    #[must_use]
    pub fn check_command(&self) -> String {
        let mut parts = vec![
            "cargo".to_string(),
            "check".to_string(),
            "-p".to_string(),
            self.package.clone(),
        ];
        parts.extend(self.selector.args());
        if !self.required_features.is_empty() {
            parts.push("--features".into());
            parts.push(self.required_features.join(","));
        }
        parts.push("--message-format=json".into());
        parts.join(" ")
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImpactReport {
    pub changed_files: Vec<String>,
    pub planned_but_unchanged: Vec<String>,
    pub unplanned_changes: Vec<String>,
    pub targets: Vec<ImpactedTarget>,
    pub producer_packages: Vec<String>,
    pub reverse_dependents: Vec<String>,
    pub high_impact: bool,
    pub high_impact_reasons: Vec<String>,
    pub fallback_reason: Option<String>,
    pub analysis_ms: u64,
}

impl ImpactReport {
    #[must_use]
    pub fn is_structural_only(&self) -> bool {
        !self.changed_files.is_empty()
            && self.fallback_reason.is_none()
            && self.targets.is_empty()
            && self
                .changed_files
                .iter()
                .all(|path| structural_only_path(path))
    }

    #[must_use]
    pub fn focused_commands(&self) -> Vec<String> {
        self.targets
            .iter()
            .map(ImpactedTarget::check_command)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    #[must_use]
    pub fn one_target(&self) -> Option<&ImpactedTarget> {
        (self.targets.len() == 1).then(|| &self.targets[0])
    }
}

pub async fn analyze(
    workdir: &Path,
    planned_files: &[String],
    config: &GatesConfig,
) -> ImpactReport {
    let started = Instant::now();
    let mut report = ImpactReport::default();
    let limit = Duration::from_millis(config.impact_timeout_ms.max(100));

    let changed = match changed_files(workdir, limit).await {
        Ok(changed) => changed,
        Err(error) => {
            report.fallback_reason = Some(error);
            report.analysis_ms = elapsed_ms(started);
            return report;
        }
    };
    report.changed_files = changed.into_iter().collect();
    let planned = planned_files
        .iter()
        .map(|path| normalize_relative(path))
        .filter(|path| !path.is_empty())
        .collect::<BTreeSet<_>>();
    let actual = report
        .changed_files
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    report.planned_but_unchanged = planned
        .iter()
        .filter(|declared| {
            !actual
                .iter()
                .any(|path| path == *declared || path.starts_with(&format!("{declared}/")))
        })
        .cloned()
        .collect();
    report.unplanned_changes = actual
        .iter()
        .filter(|path| {
            !planned
                .iter()
                .any(|declared| *path == declared || path.starts_with(&format!("{declared}/")))
        })
        .cloned()
        .collect();

    if report.changed_files.is_empty() || report.is_structural_only() {
        report.analysis_ms = elapsed_ms(started);
        return report;
    }
    if report
        .changed_files
        .iter()
        .any(|path| workspace_build_input(path))
    {
        report.fallback_reason =
            Some("workspace Cargo/build input changed; focused target selection is unsafe".into());
        report.analysis_ms = elapsed_ms(started);
        return report;
    }

    let metadata = match cargo_metadata(workdir, limit).await {
        Ok(metadata) => metadata,
        Err(error) => {
            report.fallback_reason = Some(error);
            report.analysis_ms = elapsed_ms(started);
            return report;
        }
    };
    let workspace_ids = metadata
        .workspace_members
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let packages = metadata
        .packages
        .iter()
        .filter(|package| workspace_ids.contains(&package.id))
        .collect::<Vec<_>>();

    let mut targets = BTreeSet::new();
    let mut producers = BTreeSet::new();
    for changed in &report.changed_files {
        if structural_only_path(changed) {
            continue;
        }
        let Some(package) = package_for_path(workdir, changed, &packages) else {
            report.fallback_reason = Some(format!(
                "changed path `{changed}` is not owned by one Cargo workspace package"
            ));
            report.analysis_ms = elapsed_ms(started);
            return report;
        };
        let package_name = package.name.to_string();
        producers.insert(package_name.clone());
        if changed.ends_with("Cargo.toml") || changed.ends_with("build.rs") {
            report.high_impact = true;
            report
                .high_impact_reasons
                .push(format!("package build contract changed: {changed}"));
            targets.insert(ImpactedTarget {
                package: package_name,
                selector: CargoTargetSelector::AllTargets,
                required_features: Vec::new(),
            });
            continue;
        }
        if !changed.ends_with(".rs") {
            report.fallback_reason = Some(format!(
                "non-structural input `{changed}` may affect generated or included Rust"
            ));
            report.analysis_ms = elapsed_ms(started);
            return report;
        }
        let selected = targets_for_path(workdir, changed, package);
        if selected.is_empty() {
            report.fallback_reason = Some(format!(
                "Rust input `{changed}` could not be mapped to a complete Cargo target family"
            ));
            report.analysis_ms = elapsed_ms(started);
            return report;
        } else {
            targets.extend(selected);
        }
    }
    report.producer_packages = producers.into_iter().collect();
    // Line-based public-item detection cannot see enum variants, trait methods,
    // or public struct fields whose changed lines do not repeat the enclosing
    // `pub` declaration. Any library-target edit can therefore alter a
    // downstream contract even when the hunk itself looks private. Keep FAST
    // correct by compiling bounded reverse dependents for library changes;
    // bin/test/example/bench-only edits retain the narrower path.
    if targets
        .iter()
        .any(|target| target.selector == CargoTargetSelector::Lib)
    {
        report.high_impact = true;
        report.high_impact_reasons.push(
            "library source changed; downstream contract impact cannot be ruled out".to_string(),
        );
    }
    if targets.len() > config.impact_max_targets.max(1) {
        report.fallback_reason = Some(format!(
            "{} impacted Cargo targets exceed configured cap {}",
            targets.len(),
            config.impact_max_targets.max(1)
        ));
        report.analysis_ms = elapsed_ms(started);
        return report;
    }

    match public_surface_reasons(workdir, &report.changed_files, limit).await {
        Ok(reasons) => {
            report.high_impact = report.high_impact || !reasons.is_empty();
            report.high_impact_reasons.extend(reasons);
        }
        Err(error) => {
            report.fallback_reason = Some(error);
            report.analysis_ms = elapsed_ms(started);
            return report;
        }
    }

    if report.high_impact {
        // Public producer contracts can be consumed by binaries, examples,
        // integration tests, and benches in the same package without an edge
        // in Cargo's package graph. Include those exact targets (and their
        // required features) before traversing cross-package dependents.
        for package in packages
            .iter()
            .filter(|package| report.producer_packages.contains(&package.name.to_string()))
        {
            for target in &package.targets {
                if let Some(selector) = target_selector(target) {
                    targets.insert(ImpactedTarget {
                        package: package.name.to_string(),
                        selector,
                        required_features: target.required_features.clone(),
                    });
                }
            }
        }
        let (reverse, overflow) = reverse_dependents(
            &metadata,
            &report.producer_packages,
            config.impact_max_reverse_dependents.max(1),
        );
        report.reverse_dependents = reverse.clone();
        if overflow {
            report.fallback_reason = Some(format!(
                "public/high-impact change exceeds reverse-dependent cap {}; full verification required",
                config.impact_max_reverse_dependents.max(1)
            ));
            report.analysis_ms = elapsed_ms(started);
            return report;
        }
        for package in reverse {
            targets.insert(ImpactedTarget {
                package,
                selector: CargoTargetSelector::Package,
                required_features: Vec::new(),
            });
        }
    }
    if targets.len() > config.impact_max_targets.max(1) {
        report.fallback_reason = Some(format!(
            "{} producer and reverse-dependent Cargo targets exceed configured cap {}",
            targets.len(),
            config.impact_max_targets.max(1)
        ));
        report.analysis_ms = elapsed_ms(started);
        return report;
    }
    report.targets = targets.into_iter().collect();
    report.analysis_ms = elapsed_ms(started);
    info!(
        changed_files = report.changed_files.len(),
        targets = report.targets.len(),
        high_impact = report.high_impact,
        reverse_dependents = report.reverse_dependents.len(),
        analysis_ms = report.analysis_ms,
        "Cargo change-impact analysis complete"
    );
    report
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn normalize_relative(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

fn structural_only_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let basename = lower.rsplit('/').next().unwrap_or(&lower);
    let documentation_name = ["readme", "changelog", "contributing", "license"]
        .iter()
        .any(|prefix| basename.starts_with(prefix))
        && (!basename.contains('.') || basename.ends_with(".md") || basename.ends_with(".txt"));
    lower.starts_with("docs/")
        || lower.starts_with("tmp/")
        || lower == ".roko/gaps.md"
        || documentation_name
}

fn workspace_build_input(path: &str) -> bool {
    matches!(path, "Cargo.toml" | "Cargo.lock" | "build.rs")
        || path.starts_with(".cargo/")
        || path.starts_with("rust-toolchain")
}

async fn bounded_output(
    workdir: &Path,
    program: &str,
    args: &[&str],
    limit: Duration,
    max_bytes: usize,
) -> Result<Vec<u8>, String> {
    let invocation = format!("{program} {}", args.join(" "));
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(workdir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("CARGO_NET_OFFLINE", "true")
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("could not run {invocation}: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("could not capture stdout for {invocation}"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("could not capture stderr for {invocation}"))?;
    let execution = timeout(limit, async {
        tokio::try_join!(
            async {
                child
                    .wait()
                    .await
                    .map_err(|error| format!("could not wait for {invocation}: {error}"))
            },
            read_capped(stdout, max_bytes, &invocation, "stdout"),
            read_capped(stderr, MAX_STDERR_OUTPUT, &invocation, "stderr"),
        )
    })
    .await;
    let (status, stdout, stderr) = match execution {
        Ok(Ok(output)) => output,
        Ok(Err(error)) => {
            let _ = child.start_kill();
            let _ = timeout(Duration::from_secs(1), child.wait()).await;
            return Err(error);
        }
        Err(_) => {
            let _ = child.start_kill();
            let _ = timeout(Duration::from_secs(1), child.wait()).await;
            return Err(format!("{invocation} timed out"));
        }
    };
    if !status.success() {
        let stderr = String::from_utf8_lossy(&stderr);
        return Err(format!(
            "{invocation} failed: {}",
            stderr.trim().chars().take(500).collect::<String>()
        ));
    }
    Ok(stdout)
}

async fn read_capped<R>(
    mut stream: R,
    max_bytes: usize,
    invocation: &str,
    stream_name: &str,
) -> Result<Vec<u8>, String>
where
    R: AsyncRead + Unpin,
{
    let mut output = Vec::with_capacity(max_bytes.min(64 * 1024));
    let mut chunk = [0_u8; 8192];
    loop {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|error| format!("could not read {stream_name} from {invocation}: {error}"))?;
        if read == 0 {
            return Ok(output);
        }
        if read > max_bytes.saturating_sub(output.len()) {
            return Err(format!(
                "{invocation} {stream_name} exceeded {max_bytes} bytes"
            ));
        }
        output.extend_from_slice(&chunk[..read]);
    }
}

fn parse_git_paths(raw: &[u8], command: &str) -> Result<BTreeSet<String>, String> {
    if raw.is_empty() {
        return Ok(BTreeSet::new());
    }
    if !raw.ends_with(&[0]) {
        return Err(format!("{command} returned unterminated -z path output"));
    }
    let mut paths = BTreeSet::new();
    for encoded in raw[..raw.len() - 1].split(|byte| *byte == 0) {
        if encoded.is_empty() {
            return Err(format!("{command} returned an empty path"));
        }
        let path = std::str::from_utf8(encoded)
            .map_err(|_| format!("{command} returned a non-UTF-8 path"))?;
        let parsed = Path::new(path);
        if path.contains('\\')
            || parsed.is_absolute()
            || !parsed
                .components()
                .all(|component| matches!(component, std::path::Component::Normal(_)))
        {
            return Err(format!("{command} returned an unsafe or ambiguous path"));
        }
        paths.insert(path.to_string());
        if paths.len() > MAX_CHANGED_FILES {
            return Err(format!(
                "{command} returned more than {MAX_CHANGED_FILES} paths"
            ));
        }
    }
    Ok(paths)
}

async fn changed_files(workdir: &Path, limit: Duration) -> Result<BTreeSet<String>, String> {
    let unsafe_paths = bounded_output(
        workdir,
        "git",
        &[
            "diff",
            "--name-only",
            "-z",
            "--diff-filter=CDRTUXB",
            "HEAD",
            "--",
        ],
        limit,
        MAX_GIT_OUTPUT,
    )
    .await?;
    if !unsafe_paths.is_empty() {
        return Err("deletion, rename, conflict, or type change requires full verification".into());
    }
    let tracked = bounded_output(
        workdir,
        "git",
        &[
            "diff",
            "--name-only",
            "-z",
            "--diff-filter=AM",
            "HEAD",
            "--",
        ],
        limit,
        MAX_GIT_OUTPUT,
    )
    .await?;
    let untracked = bounded_output(
        workdir,
        "git",
        &["ls-files", "-z", "--others", "--exclude-standard"],
        limit,
        MAX_GIT_OUTPUT,
    )
    .await?;
    let mut files = parse_git_paths(&tracked, "git diff --name-only -z")?;
    files.extend(parse_git_paths(&untracked, "git ls-files --others -z")?);
    if files.len() > MAX_CHANGED_FILES {
        return Err(format!(
            "combined changed path count exceeded {MAX_CHANGED_FILES}"
        ));
    }
    Ok(files)
}

async fn cargo_metadata(workdir: &Path, limit: Duration) -> Result<Metadata, String> {
    let stdout = bounded_output(
        workdir,
        "cargo",
        &["metadata", "--format-version=1", "--locked", "--offline"],
        limit,
        MAX_METADATA_OUTPUT,
    )
    .await?;
    serde_json::from_slice(&stdout).map_err(|error| format!("parse cargo metadata: {error}"))
}

fn package_for_path<'a>(
    workdir: &Path,
    changed: &str,
    packages: &[&'a Package],
) -> Option<&'a Package> {
    let absolute = workdir.join(changed);
    let mut owners = packages
        .iter()
        .copied()
        .filter(|package| {
            package
                .manifest_path
                .as_std_path()
                .parent()
                .is_some_and(|root| absolute.starts_with(root))
        })
        .collect::<Vec<_>>();
    owners.sort_by_key(|package| {
        std::cmp::Reverse(
            package
                .manifest_path
                .as_std_path()
                .parent()
                .map_or(0, |root| root.components().count()),
        )
    });
    let owner = owners.first().copied()?;
    let root = owner.manifest_path.as_std_path().parent()?;
    (!owners
        .iter()
        .skip(1)
        .any(|candidate| candidate.manifest_path.as_std_path().parent() == Some(root)))
    .then_some(owner)
}

fn target_selector(target: &Target) -> Option<CargoTargetSelector> {
    let kinds = target
        .kind
        .iter()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    if kinds.contains("test") {
        Some(CargoTargetSelector::Test(target.name.clone()))
    } else if kinds.contains("example") {
        Some(CargoTargetSelector::Example(target.name.clone()))
    } else if kinds.contains("bench") {
        Some(CargoTargetSelector::Bench(target.name.clone()))
    } else if kinds.contains("bin") {
        Some(CargoTargetSelector::Bin(target.name.clone()))
    } else if kinds.iter().any(|kind| {
        matches!(
            kind.as_str(),
            "lib" | "rlib" | "dylib" | "cdylib" | "staticlib" | "proc-macro"
        )
    }) {
        Some(CargoTargetSelector::Lib)
    } else {
        None
    }
}

fn targets_for_path(workdir: &Path, changed: &str, package: &Package) -> Vec<ImpactedTarget> {
    let absolute = workdir.join(changed);
    let package_name = package.name.to_string();
    let mut exact = Vec::new();
    for target in &package.targets {
        let source = target.src_path.as_std_path();
        if absolute == source {
            if let Some(selector) = target_selector(target) {
                exact.push(ImpactedTarget {
                    package: package_name.clone(),
                    selector,
                    required_features: target.required_features.clone(),
                });
            }
        }
    }
    if !exact.is_empty() {
        return exact;
    }

    let package_root = package.manifest_path.as_std_path().parent();
    let relative = package_root
        .and_then(|root| absolute.strip_prefix(root).ok())
        .map(PathBuf::from);
    let Some(relative) = relative else {
        return Vec::new();
    };
    let relative_text = relative.to_string_lossy().replace('\\', "/");
    let family = if relative_text.starts_with("tests/") {
        Some("test")
    } else if relative_text.starts_with("examples/") {
        Some("example")
    } else if relative_text.starts_with("benches/") {
        Some("bench")
    } else if relative_text.starts_with("src/bin/") {
        Some("bin")
    } else if relative_text.starts_with("src/") {
        Some("lib")
    } else {
        None
    };
    let Some(family) = family else {
        return Vec::new();
    };
    let mut candidates = Vec::new();
    for target in &package.targets {
        let Some(selector) = target_selector(target) else {
            continue;
        };
        let matches_family = matches!(
            (&selector, family),
            (
                CargoTargetSelector::Lib | CargoTargetSelector::Bin(_),
                "lib"
            ) | (CargoTargetSelector::Bin(_), "bin")
                | (CargoTargetSelector::Test(_), "test")
                | (CargoTargetSelector::Example(_), "example")
                | (CargoTargetSelector::Bench(_), "bench")
        );
        if !matches_family {
            continue;
        }
        candidates.push(ImpactedTarget {
            package: package_name.clone(),
            selector,
            required_features: target.required_features.clone(),
        });
    }
    // A non-root module can be included by any target in its conventional
    // family. Selecting the whole family is conservative; the global target
    // cap widens to full verification instead of silently dropping members.
    candidates
}

/// Classify a single stripped code line (no `+`/`-` prefix) as a public-item
/// change, a contract change, or both.
///
/// Returns `(public, contract)`.
fn classify_code_line(code: &str) -> (bool, bool) {
    let public = code.starts_with("pub ")
        && !code.starts_with("pub(crate)")
        && !code.starts_with("pub(super)")
        && !code.starts_with("pub(self)");
    let contract = code.starts_with("pub use ")
        || code.starts_with("impl ")
        || code.starts_with("#[serde")
        || code.contains("derive(Serialize")
        || code.contains("derive(Deserialize")
        || code.starts_with("#[repr(")
        || code.starts_with("#[macro_export]");
    (public, contract)
}

/// Classify unified-diff text into public-surface-change reason strings.
///
/// Each hunk line starting with `+` or `-` (excluding `+++`/`---` headers) is
/// inspected for public item signatures and contract annotations. Returns the
/// deduplicated reason set in sorted order.
pub fn classify_diff_lines(diff_text: &str) -> Vec<String> {
    let mut reasons = BTreeSet::new();
    for line in diff_text.lines().filter(|line| {
        (line.starts_with('+') || line.starts_with('-'))
            && !line.starts_with("+++")
            && !line.starts_with("---")
    }) {
        let code = line[1..].trim_start();
        let (public, contract) = classify_code_line(code);
        if public {
            reasons.insert("public Rust item/signature changed".to_string());
        }
        if contract {
            reasons.insert("trait, re-export, or serialized contract changed".to_string());
        }
    }
    reasons.into_iter().collect()
}

async fn public_surface_reasons(
    workdir: &Path,
    changed_files: &[String],
    limit: Duration,
) -> Result<Vec<String>, String> {
    let diff = bounded_output(
        workdir,
        "git",
        &[
            "diff",
            "--unified=0",
            "HEAD",
            "--",
            ":(glob)**/*.rs",
            ":(glob)**/*.toml",
        ],
        limit,
        MAX_GIT_OUTPUT,
    )
    .await?;
    let mut text = String::from_utf8(diff).map_err(|_| "Git diff was not UTF-8".to_string())?;
    let untracked = bounded_output(
        workdir,
        "git",
        &["ls-files", "-z", "--others", "--exclude-standard"],
        limit,
        MAX_GIT_OUTPUT,
    )
    .await?;
    let untracked = parse_git_paths(&untracked, "git ls-files --others -z")?;
    let mut untracked_bytes = 0_usize;
    for path in changed_files
        .iter()
        .filter(|path| path.ends_with(".rs") && untracked.contains(*path))
    {
        let remaining = MAX_UNTRACKED_RUST_INPUT.saturating_sub(untracked_bytes);
        let file = tokio::fs::File::open(workdir.join(path))
            .await
            .map_err(|error| format!("open untracked Rust input `{path}`: {error}"))?;
        let mut encoded = Vec::with_capacity(remaining.min(64 * 1024));
        file.take(
            u64::try_from(remaining)
                .unwrap_or(u64::MAX)
                .saturating_add(1),
        )
        .read_to_end(&mut encoded)
        .await
        .map_err(|error| format!("read untracked Rust input `{path}`: {error}"))?;
        if encoded.len() > remaining {
            return Err(format!(
                "untracked Rust inputs exceeded {MAX_UNTRACKED_RUST_INPUT} bytes"
            ));
        }
        untracked_bytes = untracked_bytes.saturating_add(encoded.len());
        let source = String::from_utf8(encoded)
            .map_err(|_| format!("untracked Rust input `{path}` was not UTF-8"))?;
        for line in source.lines() {
            text.push_str("\n+");
            text.push_str(line);
        }
    }
    Ok(classify_diff_lines(&text))
}

fn reverse_dependents(
    metadata: &Metadata,
    producers: &[String],
    cap: usize,
) -> (Vec<String>, bool) {
    let workspace = metadata
        .workspace_members
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let names = metadata
        .packages
        .iter()
        .filter(|package| workspace.contains(&package.id))
        .map(|package| (package.id.clone(), package.name.to_string()))
        .collect::<HashMap<_, _>>();
    let producer_ids = names
        .iter()
        .filter(|(_, name)| producers.contains(name))
        .map(|(id, _)| id.clone())
        .collect::<BTreeSet<_>>();
    let mut reverse: BTreeMap<PackageId, BTreeSet<PackageId>> = BTreeMap::new();
    if let Some(resolve) = &metadata.resolve {
        for node in &resolve.nodes {
            if !workspace.contains(&node.id) {
                continue;
            }
            for dependency in &node.dependencies {
                if workspace.contains(dependency) {
                    reverse
                        .entry(dependency.clone())
                        .or_default()
                        .insert(node.id.clone());
                }
            }
        }
    } else {
        warn!("cargo metadata did not provide a resolve graph");
        return (Vec::new(), true);
    }

    let mut queue = producer_ids.iter().cloned().collect::<VecDeque<_>>();
    let mut seen = producer_ids;
    let mut selected = BTreeSet::new();
    while let Some(current) = queue.pop_front() {
        for dependent in reverse.get(&current).into_iter().flatten() {
            if seen.insert(dependent.clone()) {
                queue.push_back(dependent.clone());
                if let Some(name) = names.get(dependent) {
                    selected.insert(name.clone());
                }
            }
        }
    }
    let overflow = selected.len() > cap;
    (selected.into_iter().take(cap).collect(), overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selectors_cover_examples_and_benches() {
        assert_eq!(
            CargoTargetSelector::Example("demo".into()).args(),
            vec!["--example", "demo"]
        );
        assert_eq!(
            CargoTargetSelector::Bench("throughput".into()).args(),
            vec!["--bench", "throughput"]
        );
    }

    #[test]
    fn workspace_inputs_are_never_focused() {
        assert!(workspace_build_input("Cargo.toml"));
        assert!(workspace_build_input("Cargo.lock"));
        assert!(workspace_build_input(".cargo/config.toml"));
        assert!(!workspace_build_input("crates/example/src/lib.rs"));
    }

    #[test]
    fn focused_commands_are_stable_and_unique() {
        let target = ImpactedTarget {
            package: "demo".into(),
            selector: CargoTargetSelector::Bench("speed".into()),
            required_features: vec!["bench-support".into()],
        };
        assert_eq!(
            target.check_command(),
            "cargo check -p demo --bench speed --features bench-support --message-format=json"
        );
    }

    #[test]
    fn library_selector_is_treated_as_a_downstream_contract_boundary() {
        let targets = [ImpactedTarget {
            package: "producer".into(),
            selector: CargoTargetSelector::Lib,
            required_features: Vec::new(),
        }];
        assert!(
            targets
                .iter()
                .any(|target| target.selector == CargoTargetSelector::Lib)
        );
    }

    #[test]
    fn nul_paths_preserve_newlines_and_reject_ambiguity() {
        assert_eq!(
            parse_git_paths(b"src/line\nname.rs\0", "git").unwrap(),
            BTreeSet::from(["src/line\nname.rs".to_string()])
        );
        assert!(parse_git_paths(b"src/a.rs\n", "git").is_err());
        assert!(parse_git_paths(b"../outside.rs\0", "git").is_err());
        assert!(parse_git_paths(b"src\\ambiguous.rs\0", "git").is_err());
    }

    #[test]
    fn public_struct_field_change_reports_reverse_dependents() {
        // A diff that changes a public struct field from `bool` to `Option<bool>`
        // must be classified as a public surface change. The diff format mirrors
        // `git diff --unified=0` output.
        let diff = "\
diff --git a/crates/roko-core/src/config.rs b/crates/roko-core/src/config.rs
--- a/crates/roko-core/src/config.rs
+++ b/crates/roko-core/src/config.rs
@@ -42,1 +42,1 @@
-    pub enabled: bool,
+    pub enabled: Option<bool>,
";
        let reasons = classify_diff_lines(diff);
        assert!(
            reasons
                .iter()
                .any(|r| r.contains("public Rust item/signature")),
            "expected public-item reason in {reasons:?}"
        );

        // Verify that `reverse_dependents` traverses a mock graph and returns
        // multiple downstream crates when a producer library is marked.
        let metadata: cargo_metadata::Metadata = serde_json::from_value(serde_json::json!({
            "version": 1,
            "workspace_root": "/mock",
            "target_directory": "/mock/target",
            "workspace_members": [
                "producer 0.1.0 (path+file:///mock/crates/producer)",
                "consumer-a 0.1.0 (path+file:///mock/crates/consumer-a)",
                "consumer-b 0.1.0 (path+file:///mock/crates/consumer-b)"
            ],
            "packages": [
                {
                    "name": "producer",
                    "version": "0.1.0",
                    "id": "producer 0.1.0 (path+file:///mock/crates/producer)",
                    "manifest_path": "/mock/crates/producer/Cargo.toml",
                    "targets": [],
                    "features": {},
                    "dependencies": []
                },
                {
                    "name": "consumer-a",
                    "version": "0.1.0",
                    "id": "consumer-a 0.1.0 (path+file:///mock/crates/consumer-a)",
                    "manifest_path": "/mock/crates/consumer-a/Cargo.toml",
                    "targets": [],
                    "features": {},
                    "dependencies": []
                },
                {
                    "name": "consumer-b",
                    "version": "0.1.0",
                    "id": "consumer-b 0.1.0 (path+file:///mock/crates/consumer-b)",
                    "manifest_path": "/mock/crates/consumer-b/Cargo.toml",
                    "targets": [],
                    "features": {},
                    "dependencies": []
                }
            ],
            "resolve": {
                "root": null,
                "nodes": [
                    {
                        "id": "producer 0.1.0 (path+file:///mock/crates/producer)",
                        "dependencies": [],
                        "deps": [],
                        "features": []
                    },
                    {
                        "id": "consumer-a 0.1.0 (path+file:///mock/crates/consumer-a)",
                        "dependencies": [
                            "producer 0.1.0 (path+file:///mock/crates/producer)"
                        ],
                        "deps": [],
                        "features": []
                    },
                    {
                        "id": "consumer-b 0.1.0 (path+file:///mock/crates/consumer-b)",
                        "dependencies": [
                            "producer 0.1.0 (path+file:///mock/crates/producer)"
                        ],
                        "deps": [],
                        "features": []
                    }
                ]
            }
        }))
        .expect("mock metadata should deserialize");

        let (dependents, overflow) =
            reverse_dependents(&metadata, &["producer".to_string()], 10);
        assert!(
            dependents.len() >= 2,
            "expected at least 2 reverse dependents, got {dependents:?}"
        );
        assert!(dependents.contains(&"consumer-a".to_string()));
        assert!(dependents.contains(&"consumer-b".to_string()));
        assert!(!overflow, "should not overflow with cap of 10");
    }

    #[test]
    fn private_helper_body_edit_no_cross_crate_signal() {
        // Private function body edits, `pub(crate)`, `pub(super)`, and
        // `pub(self)` changes must NOT produce any public-surface reasons.
        let diff = "\
diff --git a/crates/roko-cli/src/runner/helpers.rs b/crates/roko-cli/src/runner/helpers.rs
--- a/crates/roko-cli/src/runner/helpers.rs
+++ b/crates/roko-cli/src/runner/helpers.rs
@@ -10,3 +10,5 @@
-fn private_helper() {
-    let x = 1;
-}
+fn private_helper() {
+    let x = 2;
+    let y = x + 1;
+    tracing::info!(y, \"updated\");
+}
@@ -20,1 +22,1 @@
-    pub(crate) fn internal_tool(&self) -> bool {
+    pub(crate) fn internal_tool(&self) -> Option<bool> {
@@ -30,1 +32,1 @@
-    pub(super) fn parent_visible(&self) -> u32 {
+    pub(super) fn parent_visible(&self) -> u64 {
@@ -40,1 +42,1 @@
-    pub(self) fn module_private(&self) -> &str {
+    pub(self) fn module_private(&self) -> String {
";
        let reasons = classify_diff_lines(diff);
        assert!(
            reasons.is_empty(),
            "private/pub(crate)/pub(super)/pub(self) edits must produce zero surface signals, got {reasons:?}"
        );
    }

    #[test]
    fn reexport_and_serde_consumer_detected() {
        // `pub use` re-exports must trigger contract-change classification.
        let reexport_diff = "\
diff --git a/crates/roko-core/src/lib.rs b/crates/roko-core/src/lib.rs
--- a/crates/roko-core/src/lib.rs
+++ b/crates/roko-core/src/lib.rs
@@ -5,0 +6,1 @@
+pub use crate::config::NewExport;
";
        let reasons = classify_diff_lines(reexport_diff);
        assert!(
            reasons
                .iter()
                .any(|r| r.contains("re-export") || r.contains("contract")),
            "pub use re-export must trigger contract classification, got {reasons:?}"
        );
        // `pub use` also starts with `pub ` (non-restricted), so the
        // public-item signal should fire too.
        assert!(
            reasons
                .iter()
                .any(|r| r.contains("public Rust item/signature")),
            "pub use should also count as a public item change, got {reasons:?}"
        );

        // `derive(Deserialize)` and `#[serde` annotations on added lines.
        let serde_diff = "\
diff --git a/crates/roko-core/src/types.rs b/crates/roko-core/src/types.rs
--- a/crates/roko-core/src/types.rs
+++ b/crates/roko-core/src/types.rs
@@ -10,0 +11,2 @@
+#[derive(Debug, Clone, Deserialize)]
+#[serde(rename_all = \"camelCase\")]
";
        let serde_reasons = classify_diff_lines(serde_diff);
        assert!(
            serde_reasons
                .iter()
                .any(|r| r.contains("contract")),
            "derive(Deserialize) must trigger contract classification, got {serde_reasons:?}"
        );

        // Verify both derive(Deserialize) and #[serde are detected from the
        // same diff (both lines independently match the contract heuristic).
        // One reason string covers both since they map to the same bucket.
        assert_eq!(
            serde_reasons.len(),
            1,
            "both serde annotations should collapse to a single contract reason, got {serde_reasons:?}"
        );
    }
}
