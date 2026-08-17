# PRD-02 — Evidence Collectors

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25 (revised 2026-04-29)
**Crate**: `roko-eval-browser` (primary), `roko-eval` (traits), `roko-eval-metrics` (static/AST/runtime)
**Prerequisites**: PRD-00 (System Overview), PRD-01 (Core Abstractions)
**Implementation path**: `crates/roko-eval-browser/src/`, `crates/roko-eval/src/collectors/`, `crates/roko-eval-metrics/src/`

---

## 0. Scope

This document specifies every `EvidenceCollector` implementation in the system: the
components that produce structured evidence from artifacts under test. Evidence is the
raw material that criteria evaluate. The quality of evidence bounds the quality of
evaluation (Song et al., ICLR 2025: self-improvement quality is bounded by verifier
quality; verifier quality is bounded by evidence quality).

Nine collector implementations:

| Collector | Crate | Produces | Typical Artifact |
|---|---|---|---|
| `BrowserCollector` | `roko-eval-browser` | Screenshot, DOM, A11y, Console, Network, Styles, Layout, Perf | Running web page |
| `ScreenshotCollector` | `roko-eval-browser` | Screenshot + metadata | Image file, URL, HTML file |
| `ProcessCollector` | `roko-eval` | ProcessOutput, ProcessStatus | Shell command |
| `HttpCollector` | `roko-eval` | HttpResponse | API endpoint |
| `StaticAnalysisCollector` | `roko-eval-metrics` | StaticAnalysis, DesignTokens | Source files |
| `AstCollector` | `roko-eval-metrics` | Ast, SemanticDiff | Source files (via tree-sitter) |
| `RuntimeTraceCollector` | `roko-eval-metrics` | RuntimeTrace | Instrumented test execution |
| `DiffCollector` | `roko-eval` | Diff, SemanticDiff | Git working tree |
| `CompositeCollector` | `roko-eval` | Union of sub-collectors | Any |

Plus: artifact storage layout, retention policies, performance budgets, and the
three novel evidence collection techniques: AST analysis, semantic diff, and
runtime tracing.

**Supersedes**: `tmp/visual-gate/prd/PRD-02-Browser-Runner-and-Metrics.md` Sections 2-4,
`tmp/visual-gate/04-browser-runner.md`, and the `vision_loop/screenshot.rs` prototype.

---

## 1. Browser Evidence Collector (`BrowserCollector`)

The browser collector is the most complex evidence collector. It launches a headless
browser, navigates to a URL, executes a sequence of user interactions (a "journey"),
and captures structured evidence at each stage.

### 1.1 Two Backend Options

```rust
// File: crates/roko-eval-browser/src/lib.rs

/// Backend for browser evidence collection.
///
/// Two implementations exist:
/// - `PlaywrightBackend` (MVP): shells out to Node.js Playwright
/// - `ChromiumoxideBackend` (target): Rust-native CDP client
#[async_trait]
pub trait BrowserBackend: Send + Sync {
    async fn run(&self, spec: &BrowserRunSpec) -> Result<BrowserRunResult, EvalError>;
    fn name(&self) -> &str;
    async fn self_test(&self) -> Result<(), EvalError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendSelection {
    Auto,
    Playwright,
    Chromiumoxide,
}

impl Default for BackendSelection {
    fn default() -> Self { Self::Auto }
}
```

**Recommended path**: Start with Playwright for MVP. Migrate to `chromiumoxide` for
production. Both backends implement the same trait and produce identical output.

### 1.2 Dev Server Management

```rust
// File: crates/roko-eval-browser/src/dev_server.rs

/// RAII guard for a dev server process.
///
/// Spawns the dev server command in its own process group (`setsid` on Unix).
/// On drop, kills the entire process group (`killpg`), ensuring all child
/// processes (shell, vite, next, webpack, etc.) are terminated.
///
/// # Why process groups?
///
/// `npm run dev` -> shell -> vite. Killing the shell leaves vite holding the port.
/// `setsid` + `killpg` kills the entire tree.
pub struct DevServerHandle {
    pgid: u32,
    port: u16,
    url: String,
    child: tokio::process::Child,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevServerConfig {
    pub command: String,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default = "default_health_path")]
    pub health_path: String,
    #[serde(default = "default_startup_timeout_ms")]
    pub startup_timeout_ms: u64,
}

fn default_health_path() -> String { "/".to_string() }
fn default_startup_timeout_ms() -> u64 { 30_000 }

impl DevServerHandle {
    /// Spawn a dev server and wait for it to become healthy.
    ///
    /// Port allocation: bind to port 0, get ephemeral port, inject via $PORT.
    /// Health check: exponential backoff (100ms initial, 1.5x, 2s max, 30s timeout).
    pub async fn spawn(config: &DevServerConfig) -> Result<Self, EvalError> {
        todo!()
    }

    pub fn url(&self) -> &str { &self.url }
    pub fn port(&self) -> u16 { self.port }
}

impl Drop for DevServerHandle {
    fn drop(&mut self) {
        #[cfg(unix)]
        unsafe {
            libc::killpg(self.pgid as i32, libc::SIGTERM);
            std::thread::spawn({
                let pgid = self.pgid;
                move || {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    libc::killpg(pgid as i32, libc::SIGKILL);
                }
            });
        }
    }
}
```

### 1.3 Journey Execution

A journey is an ordered sequence of user interactions that brings the application to a
testable state.

```rust
// File: crates/roko-eval-browser/src/journey.rs

/// A UI journey: an ordered sequence of user interactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiJourney {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub start_url: Option<String>,
    pub steps: Vec<UiStep>,
    #[serde(default)]
    pub screenshot_policy: ScreenshotPolicy,
    #[serde(default)]
    pub assertions: Vec<UiAssertion>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScreenshotPolicy {
    #[default]
    FinalOnly,
    BeforeAndAfter,
    EveryStep,
    Manual,
}

/// A single interaction step within a UI journey.
/// Steps map 1:1 to Playwright actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum UiStep {
    Goto { url: String, #[serde(default = "default_wait_until")] wait_until: String },
    Click { #[serde(flatten)] target: LocatorTarget },
    Fill { #[serde(flatten)] target: LocatorTarget, value: String },
    Press { key: String },
    Hover { #[serde(flatten)] target: LocatorTarget },
    SelectOption { #[serde(flatten)] target: LocatorTarget, value: String },
    Check { #[serde(flatten)] target: LocatorTarget },
    Uncheck { #[serde(flatten)] target: LocatorTarget },
    WaitForSelector { selector: String, #[serde(default)] state: String },
    WaitForTimeout { ms: u64 },
    Screenshot { name: String },
    ScrollTo { #[serde(flatten)] target: LocatorTarget },
    EvaluateJs { expression: String },
}

fn default_wait_until() -> String { "load".into() }

/// How to locate an element for interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocatorTarget {
    Css { selector: String },
    TestId { test_id: String },
    Role { role: String, name: Option<String> },
    Text { text: String, exact: Option<bool> },
    Label { label: String },
    Placeholder { placeholder: String },
    Xpath { xpath: String },
}
```

### 1.4 BrowserCollector Implementation

```rust
// File: crates/roko-eval-browser/src/lib.rs

/// Configuration for the browser evidence collector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserCollectorConfig {
    pub backend: BackendSelection,
    #[serde(default)]
    pub dev_server: Option<DevServerConfig>,
    #[serde(default)]
    pub viewports: Vec<Viewport>,
    #[serde(default)]
    pub journeys: Vec<UiJourney>,
    /// Maximum time for the entire browser run (ms). Default: 120_000.
    #[serde(default = "default_browser_timeout")]
    pub timeout_ms: u64,
    /// Whether to capture network logs.
    #[serde(default = "default_true")]
    pub capture_network: bool,
    /// Whether to capture console logs.
    #[serde(default = "default_true")]
    pub capture_console: bool,
    /// Whether to capture computed styles.
    #[serde(default = "default_true")]
    pub capture_styles: bool,
    /// Whether to capture accessibility tree.
    #[serde(default = "default_true")]
    pub capture_a11y: bool,
    /// Whether to capture layout metrics.
    #[serde(default = "default_true")]
    pub capture_layout: bool,
    /// Whether to capture performance traces.
    #[serde(default)]
    pub capture_performance: bool,
}

fn default_browser_timeout() -> u64 { 120_000 }
fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub name: String,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub device_scale_factor: Option<f64>,
    #[serde(default)]
    pub is_mobile: bool,
}

impl Default for BrowserCollectorConfig {
    fn default() -> Self {
        Self {
            backend: BackendSelection::Auto,
            dev_server: None,
            viewports: vec![
                Viewport { name: "desktop".into(), width: 1280, height: 720,
                           device_scale_factor: None, is_mobile: false },
            ],
            journeys: Vec::new(),
            timeout_ms: default_browser_timeout(),
            capture_network: true,
            capture_console: true,
            capture_styles: true,
            capture_a11y: true,
            capture_layout: true,
            capture_performance: false,
        }
    }
}

pub struct BrowserCollector {
    config: BrowserCollectorConfig,
    backend: Box<dyn BrowserBackend>,
}

#[async_trait]
impl EvidenceCollector for BrowserCollector {
    async fn collect(
        &self,
        artifact: &ArtifactRef,
        ctx: &Context,
    ) -> Result<EvidenceBag, EvalError> {
        let url = artifact.url.as_deref()
            .ok_or_else(|| EvalError::Infrastructure {
                component: "browser_collector".into(),
                message: "artifact has no URL for browser collection".into(),
                retriable: false,
            })?;

        // 1. Start dev server if configured
        let _dev_server = if let Some(ref ds_config) = self.config.dev_server {
            Some(DevServerHandle::spawn(ds_config).await?)
        } else {
            None
        };

        let mut bag = EvidenceBag::new();

        // 2. For each viewport, run the browser
        for viewport in &self.config.viewports {
            let spec = BrowserRunSpec {
                url: url.to_string(),
                viewport: viewport.clone(),
                journeys: self.config.journeys.clone(),
                timeout_ms: self.config.timeout_ms,
                capture_network: self.config.capture_network,
                capture_console: self.config.capture_console,
                capture_styles: self.config.capture_styles,
                capture_a11y: self.config.capture_a11y,
                capture_layout: self.config.capture_layout,
                capture_performance: self.config.capture_performance,
            };

            let result = self.backend.run(&spec).await?;

            // 3. Convert BrowserRunResult to Evidence entries
            if let Some(screenshot) = result.screenshot {
                bag.insert(Evidence {
                    kind: EvidenceKind::Screenshot,
                    data: EvidenceData::Bytes {
                        content: screenshot,
                        mime: Some("image/png".into()),
                    },
                    collected_at_ms: chrono::Utc::now().timestamp_millis(),
                    collection_duration_ms: result.screenshot_duration_ms,
                    collector: "browser".into(),
                    metadata: btree_map! {
                        "viewport" => viewport.name.clone(),
                    },
                });
            }

            if let Some(dom) = result.dom {
                bag.insert(Evidence {
                    kind: EvidenceKind::Dom,
                    data: EvidenceData::Text { content: dom },
                    collected_at_ms: chrono::Utc::now().timestamp_millis(),
                    collection_duration_ms: 0,
                    collector: "browser".into(),
                    metadata: btree_map! { "viewport" => viewport.name.clone() },
                });
            }

            // ... similarly for a11y, console, network, styles, layout, perf
        }

        Ok(bag)
    }

    fn produces(&self) -> &[EvidenceKind] {
        &[
            EvidenceKind::Screenshot,
            EvidenceKind::Dom,
            EvidenceKind::AccessibilityTree,
            EvidenceKind::ConsoleLog,
            EvidenceKind::NetworkLog,
            EvidenceKind::ComputedStyles,
            EvidenceKind::LayoutMetrics,
            EvidenceKind::PerformanceTrace,
        ]
    }

    fn name(&self) -> &str { "browser" }

    fn estimated_duration_ms(&self) -> u64 {
        5000 * self.config.viewports.len() as u64
    }
}
```

---

## 2. Process Evidence Collector (`ProcessCollector`)

The process collector runs a shell command and captures stdout, stderr, exit code, and
timing. This is the workhorse for code gates: compile, lint, test, format, security scan.

It replaces the subprocess spawning that is currently embedded in each gate implementation
(e.g., `CompileGate::verify()` spawns `cargo check` directly).

```rust
// File: crates/roko-eval/src/collectors/process.rs

/// Runs a shell command and captures process evidence.
///
/// This collector replaces the subprocess spawning that is currently
/// fused into individual gate implementations (CompileGate, ClippyGate,
/// TestGate, etc.).
///
/// # Separation of concerns
///
/// In the current system, CompileGate::verify() does two things:
/// 1. Spawns `cargo check` (evidence collection)
/// 2. Interprets the exit code (judgment)
///
/// In the new system, ProcessCollector handles (1) and CompileCriterion
/// handles (2). This enables evidence reuse (multiple criteria can
/// interpret the same process output) and testability (criteria can be
/// tested with synthetic evidence).
pub struct ProcessCollector {
    /// The command to run.
    pub command: String,
    /// Arguments to the command.
    pub args: Vec<String>,
    /// Working directory.
    pub cwd: Option<PathBuf>,
    /// Environment variables.
    pub env: BTreeMap<String, String>,
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
    /// Human-readable name for this collector.
    pub name_str: String,
}

impl ProcessCollector {
    /// Create a process collector for a cargo command.
    ///
    /// Maps to the existing command construction in CompileGate, ClippyGate, etc.
    pub fn cargo(subcommand: &str, extra_args: &[&str], workdir: impl Into<PathBuf>) -> Self {
        let mut args = vec![subcommand.to_string()];
        args.extend(extra_args.iter().map(|s| s.to_string()));
        Self {
            command: "cargo".into(),
            args,
            cwd: Some(workdir.into()),
            env: BTreeMap::new(),
            timeout_ms: 600_000, // 10 minutes, matching CompileGate default
            name_str: format!("process:cargo-{subcommand}"),
        }
    }

    /// Create the specific process collectors that replace existing gates.
    pub fn for_compile(workdir: impl Into<PathBuf>, build_system: BuildSystem) -> Self {
        match build_system {
            BuildSystem::Cargo => Self::cargo("check", &["--workspace"], workdir),
            BuildSystem::Npm => Self {
                command: "npm".into(),
                args: vec!["run".into(), "build".into()],
                cwd: Some(workdir.into()),
                env: BTreeMap::new(),
                timeout_ms: 600_000,
                name_str: "process:npm-build".into(),
            },
            BuildSystem::Go => Self {
                command: "go".into(),
                args: vec!["build".into(), "./...".into()],
                cwd: Some(workdir.into()),
                env: BTreeMap::new(),
                timeout_ms: 600_000,
                name_str: "process:go-build".into(),
            },
        }
    }

    pub fn for_lint(workdir: impl Into<PathBuf>, build_system: BuildSystem) -> Self {
        match build_system {
            BuildSystem::Cargo => {
                Self::cargo("clippy", &["--workspace", "--no-deps", "--", "-D", "warnings"], workdir)
            }
            _ => todo!("lint collectors for non-Cargo build systems"),
        }
    }

    pub fn for_test(workdir: impl Into<PathBuf>, build_system: BuildSystem) -> Self {
        match build_system {
            BuildSystem::Cargo => Self::cargo("test", &["--workspace"], workdir),
            BuildSystem::Npm => Self {
                command: "npm".into(),
                args: vec!["test".into()],
                cwd: Some(workdir.into()),
                env: BTreeMap::new(),
                timeout_ms: 600_000,
                name_str: "process:npm-test".into(),
            },
            _ => todo!(),
        }
    }

    pub fn for_format_check(workdir: impl Into<PathBuf>, build_system: BuildSystem) -> Self {
        match build_system {
            BuildSystem::Cargo => Self::cargo("fmt", &["--check"], workdir),
            _ => todo!(),
        }
    }

    pub fn for_security_scan(workdir: impl Into<PathBuf>) -> Self {
        Self {
            command: "cargo".into(),
            args: vec!["audit".into()],
            cwd: Some(workdir.into()),
            env: BTreeMap::new(),
            timeout_ms: 120_000,
            name_str: "process:cargo-audit".into(),
        }
    }
}

#[async_trait]
impl EvidenceCollector for ProcessCollector {
    async fn collect(
        &self,
        artifact: &ArtifactRef,
        _ctx: &Context,
    ) -> Result<EvidenceBag, EvalError> {
        let cwd = self.cwd.clone()
            .or(artifact.path.clone())
            .unwrap_or_else(|| PathBuf::from("."));

        let started = std::time::Instant::now();
        let mut cmd = tokio::process::Command::new(&self.command);
        cmd.args(&self.args)
            .current_dir(&cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        for (k, v) in &self.env {
            cmd.env(k, v);
        }

        let output = tokio::time::timeout(
            std::time::Duration::from_millis(self.timeout_ms),
            cmd.output(),
        ).await
            .map_err(|_| EvalError::Infrastructure {
                component: self.name_str.clone(),
                message: format!("command timed out after {}ms", self.timeout_ms),
                retriable: false,
            })?
            .map_err(|e| EvalError::Infrastructure {
                component: self.name_str.clone(),
                message: format!("failed to spawn command: {e}"),
                retriable: false,
            })?;

        let duration_ms = started.elapsed().as_millis() as u64;
        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let combined = format!("{stdout}\n{stderr}");

        let mut bag = EvidenceBag::new();
        let now = chrono::Utc::now().timestamp_millis();

        // ProcessStatus evidence
        bag.insert(Evidence {
            kind: EvidenceKind::ProcessStatus,
            data: EvidenceData::Json {
                value: serde_json::json!({
                    "exit_code": exit_code,
                    "duration_ms": duration_ms,
                    "command": format!("{} {}", self.command, self.args.join(" ")),
                }),
            },
            collected_at_ms: now,
            collection_duration_ms: duration_ms,
            collector: self.name_str.clone(),
            metadata: BTreeMap::new(),
        });

        // ProcessOutput evidence
        bag.insert(Evidence {
            kind: EvidenceKind::ProcessOutput,
            data: EvidenceData::Text { content: combined },
            collected_at_ms: now,
            collection_duration_ms: duration_ms,
            collector: self.name_str.clone(),
            metadata: {
                let mut m = BTreeMap::new();
                m.insert("stdout_len".into(), stdout.len().to_string());
                m.insert("stderr_len".into(), stderr.len().to_string());
                m
            },
        });

        Ok(bag)
    }

    fn produces(&self) -> &[EvidenceKind] {
        &[EvidenceKind::ProcessOutput, EvidenceKind::ProcessStatus]
    }

    fn name(&self) -> &str {
        &self.name_str
    }

    fn estimated_duration_ms(&self) -> u64 {
        // Heuristic: compile ~30s, lint ~20s, test ~60s, fmt ~5s
        match self.name_str.as_str() {
            n if n.contains("test") => 60_000,
            n if n.contains("check") || n.contains("build") => 30_000,
            n if n.contains("clippy") => 20_000,
            _ => 10_000,
        }
    }
}
```

---

## 3. Diff Evidence Collector (`DiffCollector`)

Collects the code diff from git, producing both raw unified diff and semantic diff evidence.

```rust
// File: crates/roko-eval/src/collectors/diff.rs

/// Collects diff evidence from the artifact's git working tree.
///
/// Produces two evidence kinds:
/// - `EvidenceKind::Diff` -- raw unified diff from `git diff`
/// - `EvidenceKind::SemanticDiff` -- AST-level change classification
///   (requires tree-sitter, produced only if source files are available)
pub struct DiffCollector {
    /// Compare against this git ref (default: HEAD).
    pub base_ref: String,
    /// Only include diffs for these file patterns.
    pub include_patterns: Vec<String>,
    /// Exclude diffs matching these patterns.
    pub exclude_patterns: Vec<String>,
}

impl Default for DiffCollector {
    fn default() -> Self {
        Self {
            base_ref: "HEAD".into(),
            include_patterns: Vec::new(),
            exclude_patterns: vec![
                "*.lock".into(),
                "*.min.js".into(),
                "*.min.css".into(),
            ],
        }
    }
}

#[async_trait]
impl EvidenceCollector for DiffCollector {
    async fn collect(
        &self,
        artifact: &ArtifactRef,
        _ctx: &Context,
    ) -> Result<EvidenceBag, EvalError> {
        let workdir = artifact.path.as_deref().unwrap_or(Path::new("."));
        let started = std::time::Instant::now();

        // Run git diff
        let mut cmd = tokio::process::Command::new("git");
        cmd.args(["diff", &self.base_ref, "--stat", "--unified=3"])
            .current_dir(workdir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let output = cmd.output().await
            .map_err(|e| EvalError::Infrastructure {
                component: "diff_collector".into(),
                message: format!("git diff failed: {e}"),
                retriable: false,
            })?;

        let diff_text = String::from_utf8_lossy(&output.stdout).into_owned();
        let duration_ms = started.elapsed().as_millis() as u64;

        let mut bag = EvidenceBag::new();
        let now = chrono::Utc::now().timestamp_millis();

        bag.insert(Evidence {
            kind: EvidenceKind::Diff,
            data: EvidenceData::Text { content: diff_text.clone() },
            collected_at_ms: now,
            collection_duration_ms: duration_ms,
            collector: "diff".into(),
            metadata: {
                let mut m = BTreeMap::new();
                m.insert("base_ref".into(), self.base_ref.clone());
                m
            },
        });

        // If the artifact has source files, also produce SemanticDiff evidence
        // by parsing the before/after versions with tree-sitter and classifying
        // changes at the AST level. See Section 5 (AST Collector).
        if !artifact.source_files.is_empty() {
            if let Ok(semantic) = compute_semantic_diff(workdir, &self.base_ref, &artifact.source_files) {
                bag.insert(Evidence {
                    kind: EvidenceKind::SemanticDiff,
                    data: EvidenceData::Json { value: serde_json::to_value(&semantic).unwrap() },
                    collected_at_ms: now,
                    collection_duration_ms: started.elapsed().as_millis() as u64 - duration_ms,
                    collector: "diff:semantic".into(),
                    metadata: BTreeMap::new(),
                });
            }
        }

        Ok(bag)
    }

    fn produces(&self) -> &[EvidenceKind] {
        &[EvidenceKind::Diff, EvidenceKind::SemanticDiff]
    }

    fn name(&self) -> &str { "diff" }
}
```

---

## 4. HTTP Evidence Collector (`HttpCollector`)

```rust
// File: crates/roko-eval/src/collectors/http.rs

/// Collects HTTP response evidence from an API endpoint.
pub struct HttpCollector {
    pub timeout_ms: u64,
    pub follow_redirects: bool,
}

impl Default for HttpCollector {
    fn default() -> Self {
        Self { timeout_ms: 30_000, follow_redirects: true }
    }
}

#[async_trait]
impl EvidenceCollector for HttpCollector {
    async fn collect(
        &self,
        artifact: &ArtifactRef,
        _ctx: &Context,
    ) -> Result<EvidenceBag, EvalError> {
        let endpoint = artifact.http_endpoint.as_ref()
            .ok_or_else(|| EvalError::Infrastructure {
                component: "http_collector".into(),
                message: "artifact has no HTTP endpoint".into(),
                retriable: false,
            })?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(self.timeout_ms))
            .redirect(if self.follow_redirects {
                reqwest::redirect::Policy::limited(10)
            } else {
                reqwest::redirect::Policy::none()
            })
            .build()
            .map_err(|e| EvalError::Infrastructure {
                component: "http_collector".into(),
                message: format!("failed to build HTTP client: {e}"),
                retriable: false,
            })?;

        let started = std::time::Instant::now();
        let mut builder = match endpoint.method.to_uppercase().as_str() {
            "GET" => client.get(&endpoint.url),
            "POST" => client.post(&endpoint.url),
            "PUT" => client.put(&endpoint.url),
            "DELETE" => client.delete(&endpoint.url),
            "PATCH" => client.patch(&endpoint.url),
            m => return Err(EvalError::Configuration {
                message: format!("unsupported HTTP method: {m}"),
            }),
        };

        for (k, v) in &endpoint.headers {
            builder = builder.header(k.as_str(), v.as_str());
        }

        if let Some(body) = &endpoint.body {
            builder = builder.body(body.clone());
        }

        let response = builder.send().await
            .map_err(|e| EvalError::Infrastructure {
                component: "http_collector".into(),
                message: format!("HTTP request failed: {e}"),
                retriable: e.is_timeout() || e.is_connect(),
            })?;

        let status = response.status().as_u16();
        let headers: BTreeMap<String, String> = response.headers().iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = response.text().await.unwrap_or_default();
        let duration_ms = started.elapsed().as_millis() as u64;

        let mut bag = EvidenceBag::new();
        bag.insert(Evidence {
            kind: EvidenceKind::HttpResponse,
            data: EvidenceData::Json {
                value: serde_json::json!({
                    "status": status,
                    "headers": headers,
                    "body": body,
                    "duration_ms": duration_ms,
                }),
            },
            collected_at_ms: chrono::Utc::now().timestamp_millis(),
            collection_duration_ms: duration_ms,
            collector: "http".into(),
            metadata: BTreeMap::new(),
        });

        Ok(bag)
    }

    fn produces(&self) -> &[EvidenceKind] {
        &[EvidenceKind::HttpResponse]
    }

    fn name(&self) -> &str { "http" }
}
```

---

## 5. AST Evidence Collector (`AstCollector`) -- Novel

The AST collector uses tree-sitter to parse source files and produce structured
evidence about the code's syntax tree. This enables criteria that go far beyond
text-level analysis: structural completeness checks, API surface verification,
complexity metrics, and dead code detection.

### 5.1 Why AST-Level Evidence

The current `SymbolGate` uses regex-based symbol extraction (`pub fn`, `pub struct`,
`impl Trait for Type`). This works for simple cases but fails on:

- Nested items (impl blocks inside modules)
- Generic parameters (`fn foo<T: Bound>(...)`)
- Macro-generated code (`#[derive(...)]` impls)
- Conditional compilation (`#[cfg(...)]`)
- Complex visibility (`pub(crate)`, `pub(super)`)

Tree-sitter parsing handles all of these correctly because it parses the actual
syntax tree rather than pattern-matching text.

### 5.2 Implementation

```rust
// File: crates/roko-eval-metrics/src/ast_analysis.rs

use tree_sitter::{Parser, Language, Tree, Node};

/// AST evidence collector using tree-sitter.
///
/// Parses source files and produces structured evidence about the code's
/// syntax tree. Used by SymbolCriterion, ComplexityCriterion, and
/// DeadCodeCriterion.
///
/// # Language support
///
/// Uses the tree-sitter grammar crates:
/// - `tree-sitter-rust` for .rs files
/// - `tree-sitter-typescript` for .ts/.tsx files
/// - `tree-sitter-javascript` for .js/.jsx files
/// - `tree-sitter-go` for .go files
pub struct AstCollector {
    /// File patterns to parse (e.g., ["*.rs", "*.ts"]).
    pub patterns: Vec<String>,
    /// Source roots to search (e.g., ["src", "crates"]).
    pub source_roots: Vec<PathBuf>,
    /// Maximum file size to parse (bytes). Default: 1MB.
    pub max_file_size: u64,
}

/// Structured AST evidence for a single source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAst {
    /// Relative path to the file.
    pub path: String,
    /// Language detected.
    pub language: String,
    /// Top-level items (functions, structs, traits, impls, modules).
    pub items: Vec<AstItem>,
    /// Cyclomatic complexity per function.
    pub complexity: Vec<FunctionComplexity>,
    /// Lines of code metrics.
    pub loc: LocMetrics,
}

/// A top-level AST item extracted from a source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstItem {
    /// Item kind: function, struct, enum, trait, impl, module, const, static, type_alias.
    pub kind: String,
    /// Item name (e.g., "ProcessCollector", "collect").
    pub name: String,
    /// Visibility: public, pub_crate, pub_super, private.
    pub visibility: String,
    /// Full path: "module::submodule::ItemName".
    pub path: String,
    /// Generic parameters (e.g., ["T: Bound", "E: Error"]).
    pub generics: Vec<String>,
    /// For functions: parameter types.
    pub params: Vec<String>,
    /// For functions: return type.
    pub return_type: Option<String>,
    /// For impl blocks: the type being implemented.
    pub impl_for: Option<String>,
    /// For impl blocks: the trait being implemented (if any).
    pub impl_trait: Option<String>,
    /// Source location: line:col start and end.
    pub span: AstSpan,
    /// Child items (for modules and impl blocks).
    pub children: Vec<AstItem>,
    /// Attributes/decorators on this item.
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstSpan {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionComplexity {
    pub name: String,
    pub path: String,
    /// McCabe cyclomatic complexity.
    pub cyclomatic: u32,
    /// Cognitive complexity (SonarQube model).
    pub cognitive: u32,
    /// Number of lines in the function body.
    pub body_lines: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocMetrics {
    pub total_lines: u32,
    pub code_lines: u32,
    pub comment_lines: u32,
    pub blank_lines: u32,
}

impl AstCollector {
    pub fn new(source_roots: Vec<PathBuf>) -> Self {
        Self {
            patterns: vec!["*.rs".into(), "*.ts".into(), "*.tsx".into(), "*.js".into()],
            source_roots,
            max_file_size: 1_000_000,
        }
    }

    fn language_for_extension(ext: &str) -> Option<Language> {
        match ext {
            "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
            "ts" | "tsx" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            "js" | "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
            "go" => Some(tree_sitter_go::LANGUAGE.into()),
            _ => None,
        }
    }

    fn parse_file(&self, path: &Path, source: &str, language: Language) -> FileAst {
        let mut parser = Parser::new();
        parser.set_language(&language).expect("language grammar should be valid");
        let tree = parser.parse(source, None).expect("parse should not fail");

        let items = self.extract_items(&tree.root_node(), source, "");
        let complexity = self.compute_complexity(&tree.root_node(), source);
        let loc = self.compute_loc(source);

        FileAst {
            path: path.to_string_lossy().into(),
            language: path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .into(),
            items,
            complexity,
            loc,
        }
    }

    fn extract_items(&self, node: &Node, source: &str, parent_path: &str) -> Vec<AstItem> {
        let mut items = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_item" | "function_definition" => {
                    if let Some(item) = self.extract_function(&child, source, parent_path) {
                        items.push(item);
                    }
                }
                "struct_item" | "struct_declaration" => {
                    if let Some(item) = self.extract_struct(&child, source, parent_path) {
                        items.push(item);
                    }
                }
                "enum_item" => {
                    if let Some(item) = self.extract_enum(&child, source, parent_path) {
                        items.push(item);
                    }
                }
                "trait_item" => {
                    if let Some(item) = self.extract_trait(&child, source, parent_path) {
                        items.push(item);
                    }
                }
                "impl_item" => {
                    if let Some(item) = self.extract_impl(&child, source, parent_path) {
                        items.push(item);
                    }
                }
                "mod_item" => {
                    if let Some(item) = self.extract_module(&child, source, parent_path) {
                        items.push(item);
                    }
                }
                _ => {}
            }
        }

        items
    }

    fn extract_function(&self, node: &Node, source: &str, parent_path: &str) -> Option<AstItem> {
        let name_node = node.child_by_field_name("name")?;
        let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();
        let path = if parent_path.is_empty() { name.clone() } else { format!("{parent_path}::{name}") };

        Some(AstItem {
            kind: "function".into(),
            name,
            visibility: self.extract_visibility(node, source),
            path,
            generics: self.extract_generics(node, source),
            params: self.extract_params(node, source),
            return_type: self.extract_return_type(node, source),
            impl_for: None,
            impl_trait: None,
            span: AstSpan {
                start_line: node.start_position().row as u32 + 1,
                start_col: node.start_position().column as u32,
                end_line: node.end_position().row as u32 + 1,
                end_col: node.end_position().column as u32,
            },
            children: Vec::new(),
            attributes: self.extract_attributes(node, source),
        })
    }

    // Similar extractors for struct, enum, trait, impl, module...
    fn extract_struct(&self, _node: &Node, _source: &str, _parent: &str) -> Option<AstItem> { todo!() }
    fn extract_enum(&self, _node: &Node, _source: &str, _parent: &str) -> Option<AstItem> { todo!() }
    fn extract_trait(&self, _node: &Node, _source: &str, _parent: &str) -> Option<AstItem> { todo!() }
    fn extract_impl(&self, _node: &Node, _source: &str, _parent: &str) -> Option<AstItem> { todo!() }
    fn extract_module(&self, _node: &Node, _source: &str, _parent: &str) -> Option<AstItem> { todo!() }
    fn extract_visibility(&self, _node: &Node, _source: &str) -> String { "private".into() }
    fn extract_generics(&self, _node: &Node, _source: &str) -> Vec<String> { Vec::new() }
    fn extract_params(&self, _node: &Node, _source: &str) -> Vec<String> { Vec::new() }
    fn extract_return_type(&self, _node: &Node, _source: &str) -> Option<String> { None }
    fn extract_attributes(&self, _node: &Node, _source: &str) -> Vec<String> { Vec::new() }

    fn compute_complexity(&self, _root: &Node, _source: &str) -> Vec<FunctionComplexity> {
        // Walk all function nodes. For each:
        // - Count branching points (if, match, for, while, &&, ||) for cyclomatic
        // - Use SonarQube cognitive model (nesting depth matters) for cognitive
        Vec::new()
    }

    fn compute_loc(&self, source: &str) -> LocMetrics {
        let total = source.lines().count() as u32;
        let blank = source.lines().filter(|l| l.trim().is_empty()).count() as u32;
        let comment = source.lines().filter(|l| {
            let t = l.trim();
            t.starts_with("//") || t.starts_with("/*") || t.starts_with("*")
        }).count() as u32;
        LocMetrics {
            total_lines: total,
            code_lines: total - blank - comment,
            comment_lines: comment,
            blank_lines: blank,
        }
    }
}

#[async_trait]
impl EvidenceCollector for AstCollector {
    async fn collect(
        &self,
        artifact: &ArtifactRef,
        _ctx: &Context,
    ) -> Result<EvidenceBag, EvalError> {
        let workdir = artifact.path.as_deref().unwrap_or(Path::new("."));
        let started = std::time::Instant::now();
        let mut file_asts = Vec::new();

        // Walk source roots and parse matching files
        for root in &self.source_roots {
            let full_root = workdir.join(root);
            if !full_root.exists() { continue; }

            for entry in walkdir::WalkDir::new(&full_root)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let Some(language) = Self::language_for_extension(ext) else { continue };

                // Check file size
                if let Ok(meta) = std::fs::metadata(path) {
                    if meta.len() > self.max_file_size { continue; }
                }

                if let Ok(source) = std::fs::read_to_string(path) {
                    let relative = path.strip_prefix(workdir).unwrap_or(path);
                    let file_ast = self.parse_file(relative, &source, language);
                    file_asts.push(file_ast);
                }
            }
        }

        let duration_ms = started.elapsed().as_millis() as u64;
        let mut bag = EvidenceBag::new();

        bag.insert(Evidence {
            kind: EvidenceKind::Ast,
            data: EvidenceData::Json {
                value: serde_json::to_value(&file_asts).unwrap_or_default(),
            },
            collected_at_ms: chrono::Utc::now().timestamp_millis(),
            collection_duration_ms: duration_ms,
            collector: "ast".into(),
            metadata: {
                let mut m = BTreeMap::new();
                m.insert("file_count".into(), file_asts.len().to_string());
                m
            },
        });

        Ok(bag)
    }

    fn produces(&self) -> &[EvidenceKind] {
        &[EvidenceKind::Ast]
    }

    fn name(&self) -> &str { "ast" }
}
```

---

## 6. Semantic Diff Collector -- Novel

The semantic diff goes beyond textual diffing to classify changes at the AST level.
This distinguishes structural changes (new functions, changed signatures, modified
control flow) from cosmetic changes (renames, reformatting, comment edits). This
is critical for the `DiffCriterion`: a 200-line diff that is all renames should
score differently than a 200-line diff that adds new functionality.

```rust
// File: crates/roko-eval-metrics/src/semantic_diff.rs

/// Classification of a change at the AST level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticChange {
    /// File path.
    pub path: String,
    /// Kind of change.
    pub kind: SemanticChangeKind,
    /// Description of what changed.
    pub description: String,
    /// Significance score [0.0, 1.0]: how structurally important this change is.
    pub significance: f64,
    /// AST path of the changed node (e.g., "mod::struct::method").
    pub ast_path: Option<String>,
    /// Span in the after-version.
    pub span: Option<AstSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticChangeKind {
    /// New item added (function, struct, module, etc.).
    Added,
    /// Existing item removed.
    Removed,
    /// Function signature changed (params, return type, generics).
    SignatureChanged,
    /// Function body logic changed (control flow, algorithm).
    LogicChanged,
    /// Type definition changed (fields, variants).
    TypeChanged,
    /// Visibility changed (pub -> private or vice versa).
    VisibilityChanged,
    /// Renamed (item exists at same structural position with different name).
    Renamed,
    /// Moved (item exists with same name at different path).
    Moved,
    /// Documentation/comment changed only.
    DocumentationChanged,
    /// Formatting/whitespace only (no AST change).
    FormattingOnly,
    /// Import/use statement changed.
    ImportChanged,
    /// Attribute/annotation changed.
    AttributeChanged,
    /// Test added or modified.
    TestChanged,
}

impl SemanticChangeKind {
    /// Base significance for this change kind.
    pub fn base_significance(&self) -> f64 {
        match self {
            Self::Added => 0.9,
            Self::Removed => 0.8,
            Self::SignatureChanged => 0.85,
            Self::LogicChanged => 0.7,
            Self::TypeChanged => 0.8,
            Self::VisibilityChanged => 0.6,
            Self::Renamed => 0.3,
            Self::Moved => 0.3,
            Self::DocumentationChanged => 0.1,
            Self::FormattingOnly => 0.0,
            Self::ImportChanged => 0.2,
            Self::AttributeChanged => 0.4,
            Self::TestChanged => 0.5,
        }
    }
}

/// Compute semantic diff between two versions of source files.
///
/// Parses both versions with tree-sitter and compares the ASTs to
/// classify changes. Returns a list of semantic changes.
pub fn compute_semantic_diff(
    workdir: &Path,
    base_ref: &str,
    files: &[PathBuf],
) -> Result<Vec<SemanticChange>, EvalError> {
    let mut changes = Vec::new();

    for file in files {
        // Get the base version from git
        let base_source = get_git_file_contents(workdir, base_ref, file)?;
        // Get the current version from disk
        let current_source = std::fs::read_to_string(workdir.join(file))
            .map_err(|e| EvalError::Infrastructure {
                component: "semantic_diff".into(),
                message: format!("failed to read {}: {e}", file.display()),
                retriable: false,
            })?;

        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
        let Some(language) = AstCollector::language_for_extension(ext) else { continue };

        let mut parser = Parser::new();
        parser.set_language(&language).expect("valid language");

        let base_tree = parser.parse(&base_source, None);
        let current_tree = parser.parse(&current_source, None);

        if let (Some(base), Some(current)) = (base_tree, current_tree) {
            let file_changes = diff_trees(
                &base, &base_source,
                &current, &current_source,
                file.to_string_lossy().as_ref(),
            );
            changes.extend(file_changes);
        }
    }

    Ok(changes)
}

fn get_git_file_contents(workdir: &Path, ref_: &str, file: &Path) -> Result<String, EvalError> {
    let output = std::process::Command::new("git")
        .args(["show", &format!("{ref_}:{}", file.display())])
        .current_dir(workdir)
        .output()
        .map_err(|e| EvalError::Infrastructure {
            component: "semantic_diff".into(),
            message: format!("git show failed: {e}"),
            retriable: false,
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        // File might be new (not in base ref)
        Ok(String::new())
    }
}

fn diff_trees(
    _base: &Tree, _base_source: &str,
    _current: &Tree, _current_source: &str,
    _file_path: &str,
) -> Vec<SemanticChange> {
    // Algorithm:
    // 1. Extract top-level items from both trees
    // 2. Match items by name and kind
    // 3. For matched items: compare signatures, bodies, visibility
    // 4. Unmatched base items -> Removed
    // 5. Unmatched current items -> Added
    // 6. For items where only comments/whitespace changed -> FormattingOnly
    todo!()
}
```

---

## 7. Runtime Trace Collector -- Novel

The runtime trace collector instruments test execution to capture function call
traces, allocation patterns, and I/O activity. This evidence enables criteria that
assess runtime behavior, not just static structure.

```rust
// File: crates/roko-eval-metrics/src/runtime_trace.rs

/// Runtime trace evidence from instrumented test execution.
///
/// This collector runs the project's test suite with instrumentation enabled
/// and captures execution traces. The traces provide evidence for criteria
/// that need runtime behavior data:
///
/// - **Coverage criterion**: which functions/lines were exercised
/// - **Performance criterion**: hot paths and allocation rates
/// - **Integration criterion**: which modules interact at runtime
///
/// # Instrumentation backends
///
/// - Rust: `cargo test` with `LLVM_PROFILE_FILE` for coverage,
///   `RUST_LOG=trace` for span tracing
/// - Node.js: `NODE_OPTIONS=--experimental-vm-modules` with c8 coverage
pub struct RuntimeTraceCollector {
    /// Build system to instrument.
    pub build_system: BuildSystem,
    /// Which test selector to use (All, Quick, Patterns).
    pub test_selector: TestSelector,
    /// Whether to collect code coverage data.
    pub collect_coverage: bool,
    /// Whether to collect execution timing traces.
    pub collect_timing: bool,
    /// Timeout for instrumented test execution (ms).
    pub timeout_ms: u64,
}

/// Runtime trace evidence structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeTraceData {
    /// Code coverage summary.
    pub coverage: Option<CoverageSummary>,
    /// Function-level execution counts.
    pub execution_counts: Vec<FunctionExecution>,
    /// Hot paths (functions taking > threshold of total time).
    pub hot_paths: Vec<HotPath>,
    /// Module interaction graph (which modules called each other).
    pub module_interactions: Vec<ModuleInteraction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageSummary {
    /// Line coverage percentage (0.0 to 1.0).
    pub line_coverage: f64,
    /// Branch coverage percentage (0.0 to 1.0).
    pub branch_coverage: f64,
    /// Function coverage percentage (0.0 to 1.0).
    pub function_coverage: f64,
    /// Number of lines instrumented.
    pub total_lines: u32,
    /// Number of lines executed.
    pub covered_lines: u32,
    /// Per-file coverage.
    pub files: Vec<FileCoverage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    pub path: String,
    pub line_coverage: f64,
    pub uncovered_lines: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionExecution {
    pub name: String,
    pub module: String,
    pub call_count: u64,
    pub total_time_us: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotPath {
    pub function: String,
    pub percentage_of_total: f64,
    pub call_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInteraction {
    pub caller_module: String,
    pub callee_module: String,
    pub call_count: u64,
}

#[async_trait]
impl EvidenceCollector for RuntimeTraceCollector {
    async fn collect(
        &self,
        artifact: &ArtifactRef,
        _ctx: &Context,
    ) -> Result<EvidenceBag, EvalError> {
        let workdir = artifact.path.as_deref().unwrap_or(Path::new("."));
        let started = std::time::Instant::now();

        // Phase 1: Run instrumented tests
        let mut env = BTreeMap::new();
        if self.collect_coverage && self.build_system == BuildSystem::Cargo {
            // Use cargo-llvm-cov or LLVM_PROFILE_FILE
            env.insert("CARGO_INCREMENTAL".into(), "0".into());
            env.insert("RUSTFLAGS".into(), "-C instrument-coverage".into());
            env.insert("LLVM_PROFILE_FILE".into(),
                format!("{}/target/coverage/%p-%m.profraw", workdir.display()));
        }

        let process = ProcessCollector {
            command: "cargo".into(),
            args: vec!["test".into(), "--workspace".into()],
            cwd: Some(workdir.to_path_buf()),
            env,
            timeout_ms: self.timeout_ms,
            name_str: "process:instrumented-test".into(),
        };

        let process_bag = process.collect(artifact, _ctx).await?;

        // Phase 2: Collect coverage data
        let mut trace_data = RuntimeTraceData {
            coverage: None,
            execution_counts: Vec::new(),
            hot_paths: Vec::new(),
            module_interactions: Vec::new(),
        };

        if self.collect_coverage {
            // Parse coverage output (lcov/JSON format)
            trace_data.coverage = parse_coverage_data(workdir).ok();
        }

        let duration_ms = started.elapsed().as_millis() as u64;
        let mut bag = process_bag; // Start with the process output evidence

        bag.insert(Evidence {
            kind: EvidenceKind::RuntimeTrace,
            data: EvidenceData::Json {
                value: serde_json::to_value(&trace_data).unwrap_or_default(),
            },
            collected_at_ms: chrono::Utc::now().timestamp_millis(),
            collection_duration_ms: duration_ms,
            collector: "runtime_trace".into(),
            metadata: BTreeMap::new(),
        });

        Ok(bag)
    }

    fn produces(&self) -> &[EvidenceKind] {
        &[
            EvidenceKind::RuntimeTrace,
            EvidenceKind::ProcessOutput,
            EvidenceKind::ProcessStatus,
        ]
    }

    fn name(&self) -> &str { "runtime_trace" }

    fn estimated_duration_ms(&self) -> u64 {
        120_000 // Instrumented tests are slow
    }
}

fn parse_coverage_data(_workdir: &Path) -> Result<CoverageSummary, EvalError> {
    // Parse lcov.info or JSON coverage output from llvm-cov
    todo!()
}
```

---

## 8. Evidence Storage Layout

All evidence artifacts are stored under `.roko/eval/` with a predictable directory structure.

```
.roko/eval/
  traces/                           # EvalTrace JSONL files
    2026-04-29.jsonl                # One file per day
  artifacts/                        # Raw evidence artifacts
    {task_id}/
      {attempt}/
        screenshots/
          desktop.png
          mobile.png
        dom/
          desktop.html
        a11y/
          desktop.json
        ast/
          file_asts.json
        coverage/
          lcov.info
        diff.patch
        process_output.txt
  profiles/                         # User-authored profile TOML files
    rust-strict.toml
    web-visual.toml
```

---

## 9. Performance Budgets

| Collector | Target Latency | Maximum | Notes |
|---|---|---|---|
| ProcessCollector (compile) | 30s | 600s | Depends on project size |
| ProcessCollector (lint) | 20s | 120s | |
| ProcessCollector (test) | 60s | 600s | |
| ProcessCollector (format) | 5s | 30s | |
| DiffCollector | 1s | 10s | |
| AstCollector | 5s | 30s | Scales with file count |
| BrowserCollector | 10s | 120s | Per viewport |
| HttpCollector | 2s | 30s | |
| RuntimeTraceCollector | 120s | 600s | Instrumented tests are slow |

The `EvalService` enforces per-collector timeouts and aggregates total evaluation
time against a configurable budget. Collectors that exceed their budget are terminated
and produce an infrastructure failure finding.

---

## 10. Implementation Checklist

| # | File | What |
|---|---|---|
| 1 | `crates/roko-eval/src/collectors/mod.rs` | Module declarations |
| 2 | `crates/roko-eval/src/collectors/process.rs` | ProcessCollector |
| 3 | `crates/roko-eval/src/collectors/diff.rs` | DiffCollector |
| 4 | `crates/roko-eval/src/collectors/http.rs` | HttpCollector |
| 5 | `crates/roko-eval-browser/Cargo.toml` | New crate |
| 6 | `crates/roko-eval-browser/src/lib.rs` | BrowserCollector, BrowserBackend trait |
| 7 | `crates/roko-eval-browser/src/dev_server.rs` | DevServerHandle |
| 8 | `crates/roko-eval-browser/src/journey.rs` | UiJourney, UiStep, LocatorTarget |
| 9 | `crates/roko-eval-browser/src/playwright.rs` | PlaywrightBackend |
| 10 | `crates/roko-eval-metrics/Cargo.toml` | New crate |
| 11 | `crates/roko-eval-metrics/src/ast_analysis.rs` | AstCollector (tree-sitter) |
| 12 | `crates/roko-eval-metrics/src/semantic_diff.rs` | SemanticDiff computation |
| 13 | `crates/roko-eval-metrics/src/runtime_trace.rs` | RuntimeTraceCollector |
