//! Extension chain loader — scans `.roko/extensions/` and `plugins/` for
//! extension manifests and creates [`Extension`] implementations from them.
//!
//! Each discovered plugin manifest (`plugin.toml`) becomes a
//! [`PluginExtension`] that logs lifecycle events and can be extended to
//! execute declarative hooks (tool profiles, triggers, etc.) in the future.
//!
//! In addition, each subdirectory of `.roko/extensions/` is probed for an
//! `extension.toml` or `manifest.toml` file containing a full
//! [`ExtensionManifest`] (v2 spec §3). Manifests are validated on load and
//! any that fail validation are skipped with a warning.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;

use roko_agent::dispatcher::HandlerResolver;
use roko_agent::process::{kill_tree, set_process_group};
use roko_agent::provider::LocalToolRuntime;
use roko_core::extension::{
    Extension, ExtensionChain, ExtensionLayer, ExtensionManifest, ExtensionMeta, GateEvent,
    InferenceRequest, InferenceResponse, ManifestValidationError, PackageTier,
};
use roko_core::plugin::{PluginCapability, PluginTier};
use roko_core::tool::{
    ToolCall, ToolCategory, ToolConcurrency, ToolDef, ToolError, ToolHandler, ToolPermission,
    ToolRegistry, ToolResult, ToolSchema, ToolSource,
};
use roko_core::{Result as RokoResult, RokoError};
use roko_fs::RokoLayout;
use roko_plugin::manifest::{DeclarativeTool, LoadedPlugin, SandboxConfig as PluginSandboxConfig};
use roko_std::tool::{
    DynamicToolRegistry, SandboxConfig as RegistrySandboxConfig, ToolValidationIssue,
    validate_tool_catalog_with_handler,
};
use tokio::io::AsyncReadExt;
use tracing::{debug, info, warn};

use super::wasm_extension::WasmExtension;

// ─── LoadedExtension ────────────────────────────────────────────────────

/// A successfully loaded extension manifest together with its health state.
///
/// Returned by [`scan_extension_manifests`] for callers that need to inspect
/// which extensions were discovered before registering them with the chain.
#[derive(Debug, Clone)]
pub struct LoadedExtension {
    /// Parsed and validated manifest.
    pub manifest: ExtensionManifest,
    /// Path of the manifest file that was loaded.
    pub manifest_path: std::path::PathBuf,
    /// Whether the extension was disabled (present in `disable_extensions`).
    pub disabled: bool,
}

#[derive(Debug)]
struct ManifestScanFailure {
    extension: String,
    optional: bool,
    disabled: bool,
    message: String,
}

#[derive(Debug, Default)]
struct ManifestScanReport {
    loaded: Vec<LoadedExtension>,
    failures: Vec<ManifestScanFailure>,
}

// ─── PluginExtension (kept from original) ──────────��───────────────────────────────────────

/// An [`Extension`] backed by a discovered plugin manifest.
///
/// Currently provides logging at each hook point. When a plugin declares
/// tool profiles or triggers, this wrapper is the right place to enforce
/// them.
struct PluginExtension {
    meta: ExtensionMeta,
    /// Number of prompt templates the plugin provides.
    prompt_count: usize,
    /// Canonical declarative definitions exposed by this plugin.
    tools: Vec<ToolDef>,
}

/// Canonical resolved plugin set plus the definitions and handlers contributed
/// by its declarative tools.
///
/// Discovery, CLI status commands, provider composition, and direct dispatcher
/// tests all consume this object so a tool cannot be advertised from one scan
/// while its handler is built from another.
#[derive(Clone)]
pub struct PluginToolCatalog {
    plugins: Vec<LoadedPlugin>,
    registry: Arc<DynamicToolRegistry>,
    plugin_tools: Arc<Vec<ToolDef>>,
    handlers: Arc<HashMap<String, Arc<dyn ToolHandler>>>,
}

impl std::fmt::Debug for PluginToolCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginToolCatalog")
            .field("plugin_count", &self.plugins.len())
            .field("plugin_tool_count", &self.plugin_tools.len())
            .field("registry_tool_count", &self.registry.len())
            .finish_non_exhaustive()
    }
}

impl PluginToolCatalog {
    /// Resolved, enabled plugins in deterministic dependency order.
    #[must_use]
    pub fn plugins(&self) -> &[LoadedPlugin] {
        &self.plugins
    }

    /// Built-in and declarative definitions in the canonical dynamic registry.
    #[must_use]
    pub fn registry(&self) -> &Arc<DynamicToolRegistry> {
        &self.registry
    }

    /// Only the definitions contributed by the resolved plugin set.
    #[must_use]
    pub fn plugin_tools(&self) -> &[ToolDef] {
        &self.plugin_tools
    }

    /// Resolver for direct dispatch through the full dynamic registry.
    #[must_use]
    pub fn resolver(&self) -> Arc<dyn HandlerResolver> {
        Arc::new(self.clone())
    }

    /// Provider-ready local runtime. Providers compose this plugin-only
    /// resolver over their built-in resolver and fail closed on parity gaps.
    #[must_use]
    pub fn local_runtime(&self) -> Arc<LocalToolRuntime> {
        let handlers = Arc::clone(&self.handlers);
        let resolver: Arc<dyn HandlerResolver> =
            Arc::new(move |name: &str| handlers.get(name).cloned());
        Arc::new(LocalToolRuntime::new(
            self.plugin_tools.as_ref().clone(),
            resolver,
        ))
    }

    /// Catalog/handler parity and naming diagnostics for the composed runtime.
    #[must_use]
    pub fn validation_issues(&self) -> Vec<ToolValidationIssue> {
        validate_tool_catalog_with_handler(self.registry.as_ref(), |name| {
            self.resolve(name).is_some()
        })
    }
}

impl HandlerResolver for PluginToolCatalog {
    fn resolve(&self, name: &str) -> Option<Arc<dyn ToolHandler>> {
        self.handlers
            .get(name)
            .cloned()
            .or_else(|| roko_std::tool::handler_for(name))
    }
}

#[derive(Debug, Clone)]
struct DeclarativeToolHandler {
    name: String,
    command: String,
    working_dir: Option<PathBuf>,
    env: HashMap<String, String>,
    tier: PluginTier,
    capabilities: PluginCapability,
    sandbox: PluginSandboxConfig,
    timeout_ms: u64,
    max_output_bytes: usize,
    confinement: PluginConfinement,
}

const MAX_PLUGIN_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

/// Kernel-backed containment available for arbitrary declarative-plugin
/// subprocesses.  An unsupported host is a hard execution error: capability
/// checks and path validation are not a substitute for an OS sandbox.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PluginConfinement {
    #[cfg(target_os = "macos")]
    MacOsSeatbelt {
        executable: PathBuf,
    },
    #[cfg(target_os = "linux")]
    LinuxFirejail {
        executable: PathBuf,
    },
    Unsupported {
        reason: String,
    },
}

impl PluginConfinement {
    fn detect() -> Self {
        #[cfg(target_os = "macos")]
        {
            let executable = PathBuf::from("/usr/bin/sandbox-exec");
            if executable.is_file() {
                return Self::MacOsSeatbelt { executable };
            }
            return Self::Unsupported {
                reason: "macOS Seatbelt launcher `/usr/bin/sandbox-exec` is unavailable"
                    .to_string(),
            };
        }

        #[cfg(target_os = "linux")]
        {
            // Use fixed system paths instead of PATH lookup so a plugin cannot
            // replace the confinement launcher through its environment.
            for candidate in ["/usr/bin/firejail", "/usr/local/bin/firejail"] {
                let executable = PathBuf::from(candidate);
                if executable.is_file() {
                    return Self::LinuxFirejail { executable };
                }
            }
            return Self::Unsupported {
                reason: "Linux firejail with seccomp is unavailable at a trusted system path"
                    .to_string(),
            };
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        Self::Unsupported {
            reason: format!(
                "no plugin subprocess confinement backend is implemented for {}",
                std::env::consts::OS
            ),
        }
    }

    #[allow(dead_code)]
    fn is_supported(&self) -> bool {
        !matches!(self, Self::Unsupported { .. })
    }

    fn command(
        &self,
        shell_command: &str,
        worktree: &Path,
        capabilities: PluginCapability,
        sandbox: &PluginSandboxConfig,
    ) -> Result<tokio::process::Command, ToolError> {
        match self {
            #[cfg(target_os = "macos")]
            Self::MacOsSeatbelt { executable } => {
                let mut command = tokio::process::Command::new(executable);
                command
                    .arg("-p")
                    .arg(macos_seatbelt_profile(worktree, capabilities, sandbox))
                    .arg("/bin/sh")
                    .arg("-c")
                    .arg(shell_command);
                Ok(command)
            }
            #[cfg(target_os = "linux")]
            Self::LinuxFirejail { executable } => {
                let mut command = tokio::process::Command::new(executable);
                command.args([
                    "--quiet",
                    "--noprofile",
                    "--nonewprivs",
                    "--noroot",
                    "--caps.drop=all",
                    "--seccomp",
                    "--ipc-namespace",
                    "--rlimit-nproc=64",
                ]);
                if !capabilities.network_egress {
                    command.arg("--net=none");
                }
                command
                    .arg(format!("--private={}", worktree.display()))
                    .arg("--")
                    .arg("/bin/sh")
                    .arg("-c")
                    .arg(shell_command);
                Ok(command)
            }
            Self::Unsupported { reason } => Err(ToolError::PermissionDenied(format!(
                "plugin subprocess execution requires kernel confinement: {reason}"
            ))),
        }
    }
}

#[cfg(target_os = "macos")]
fn macos_seatbelt_profile(
    worktree: &Path,
    capabilities: PluginCapability,
    sandbox: &PluginSandboxConfig,
) -> String {
    let mut profile = String::from(
        "(version 1)\n\
         (deny default)\n\
         (allow process-exec)\n\
         (allow process-fork)\n\
         (allow signal (target self))\n\
         (allow sysctl-read)\n\
         (allow mach-lookup)\n\
         (allow file-read*)\n\
         (allow file-write* (literal \"/dev/null\"))\n",
    );
    if capabilities.filesystem_write {
        for path in seatbelt_allowed_roots(worktree, &sandbox.allowed_paths) {
            profile.push_str(&format!(
                "(allow file-write* (subpath \"{}\"))\n",
                seatbelt_escape(&path.to_string_lossy())
            ));
        }
    }
    if capabilities.network_egress {
        profile.push_str("(allow network*)\n");
    }
    profile
}

#[cfg(target_os = "macos")]
fn seatbelt_allowed_roots(worktree: &Path, patterns: &[String]) -> Vec<PathBuf> {
    let mut roots = patterns
        .iter()
        .filter_map(|pattern| {
            let prefix = pattern
                .split(['*', '?', '['])
                .next()
                .unwrap_or_default()
                .trim_end_matches('/');
            if prefix.is_empty() {
                Some(worktree.to_path_buf())
            } else {
                let candidate = worktree.join(prefix);
                Some(candidate.canonicalize().unwrap_or(candidate))
            }
        })
        .collect::<Vec<_>>();
    roots.sort();
    roots.dedup();
    roots
}

#[cfg(target_os = "macos")]
fn seatbelt_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[async_trait::async_trait]
impl ToolHandler for DeclarativeToolHandler {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, call: ToolCall, ctx: &roko_core::tool::ToolContext) -> ToolResult {
        if ctx.is_cancelled() {
            return ToolResult::err(ToolError::Cancelled);
        }

        if let Err(error) = self.validate_execution_policy(ctx) {
            return ToolResult::err(error);
        }

        let working_dir = match resolve_working_dir(ctx.worktree(), self.working_dir.as_deref()) {
            Ok(path) => path,
            Err(error) => return ToolResult::err(error),
        };
        let arguments = match serde_json::to_string(&call.arguments) {
            Ok(arguments) => arguments,
            Err(error) => {
                return ToolResult::err(ToolError::Other(format!(
                    "failed to serialize arguments for `{}`: {error}",
                    self.name
                )));
            }
        };

        let canonical_worktree = match std::fs::canonicalize(ctx.worktree()) {
            Ok(path) => path,
            Err(error) => {
                return ToolResult::err(ToolError::Other(format!(
                    "failed to resolve plugin worktree `{}` before confinement: {error}",
                    ctx.worktree().display()
                )));
            }
        };
        let mut command = match self.confinement.command(
            &self.command,
            &canonical_worktree,
            self.capabilities,
            &self.sandbox,
        ) {
            Ok(command) => command,
            Err(error) => return ToolResult::err(error),
        };
        command
            .current_dir(working_dir)
            .env_clear()
            .env("PATH", "/usr/local/bin:/usr/bin:/bin")
            .envs(&self.env)
            .env("ROKO_TOOL_ARGS", arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if self.capabilities.secrets {
            for name in &self.sandbox.env_allowlist {
                if !self.env.contains_key(name)
                    && let Some(value) = std::env::var_os(name)
                {
                    command.env(name, value);
                }
            }
        }
        set_process_group(&mut command);

        let timeout = std::time::Duration::from_millis(self.timeout_ms).min(ctx.timeout);
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                return ToolResult::err(ToolError::Other(format!(
                    "failed to execute plugin tool `{}`: {error}",
                    self.name
                )));
            }
        };
        let mut process_guard = ProcessGroupGuard::new(child.id());
        let Some(stdout) = child.stdout.take() else {
            terminate_plugin_process(&mut child, &process_guard).await;
            process_guard.disarm();
            return ToolResult::err(ToolError::Other(format!(
                "plugin tool `{}` did not provide a stdout pipe",
                self.name
            )));
        };
        let Some(stderr) = child.stderr.take() else {
            terminate_plugin_process(&mut child, &process_guard).await;
            process_guard.disarm();
            return ToolResult::err(ToolError::Other(format!(
                "plugin tool `{}` did not provide a stderr pipe",
                self.name
            )));
        };
        let captured = match tokio::time::timeout(
            timeout,
            capture_bounded_output(&mut child, stdout, stderr, self.max_output_bytes),
        )
        .await
        {
            Ok(Ok(output)) => output,
            Ok(Err(CaptureError::OutputLimitExceeded)) => {
                terminate_plugin_process(&mut child, &process_guard).await;
                process_guard.disarm();
                return ToolResult::err(ToolError::Other(format!(
                    "plugin tool `{}` exceeded its combined stdout/stderr limit of {} bytes",
                    self.name, self.max_output_bytes
                )));
            }
            Ok(Err(CaptureError::Io(error))) => {
                terminate_plugin_process(&mut child, &process_guard).await;
                process_guard.disarm();
                return ToolResult::err(ToolError::Other(format!(
                    "failed to read output from plugin tool `{}`: {error}",
                    self.name
                )));
            }
            Err(_) => {
                terminate_plugin_process(&mut child, &process_guard).await;
                process_guard.disarm();
                return ToolResult::err(ToolError::Timeout {
                    after_ms: u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX),
                });
            }
        };
        process_guard.disarm();
        let rendered = render_process_output(&captured.stdout, &captured.stderr);
        if captured.status.success() {
            ToolResult::text(rendered)
        } else {
            ToolResult::err(ToolError::Other(format!(
                "plugin tool `{}` exited with {}{}",
                self.name,
                captured.status,
                if rendered.is_empty() {
                    String::new()
                } else {
                    format!(": {rendered}")
                }
            )))
        }
    }
}

impl DeclarativeToolHandler {
    fn validate_execution_policy(
        &self,
        ctx: &roko_core::tool::ToolContext,
    ) -> Result<(), ToolError> {
        let denied = self.capabilities.denied_by(self.tier);
        if !denied.is_empty() {
            return Err(ToolError::PermissionDenied(format!(
                "plugin tool `{}` exceeds tier {}: {}",
                self.name,
                self.tier.label(),
                denied.join(", ")
            )));
        }
        if !self.capabilities.exec || !self.tier.allows_exec() {
            return Err(ToolError::PermissionDenied(format!(
                "plugin tool `{}` is not granted subprocess execution at tier {}",
                self.name,
                self.tier.label()
            )));
        }
        let mut missing_runtime_grants = Vec::new();
        if !ctx.capabilities.exec {
            missing_runtime_grants.push("exec");
        }
        if self.capabilities.filesystem_read && !ctx.capabilities.read {
            missing_runtime_grants.push("filesystem_read");
        }
        if self.capabilities.filesystem_write && !ctx.capabilities.write {
            missing_runtime_grants.push("filesystem_write");
        }
        if self.capabilities.network_egress && !ctx.capabilities.network {
            missing_runtime_grants.push("network_egress");
        }
        if !missing_runtime_grants.is_empty() {
            return Err(ToolError::PermissionDenied(format!(
                "plugin tool `{}` is missing runtime grants: {}",
                self.name,
                missing_runtime_grants.join(", ")
            )));
        }

        if !self.sandbox.env_allowlist.is_empty() && !self.capabilities.secrets {
            return Err(ToolError::PermissionDenied(format!(
                "plugin tool `{}` must declare the `secrets` capability before reading allowlisted environment values",
                self.name
            )));
        }

        if !self.sandbox.allow_shell_metacharacters
            && let Err(error) = RegistrySandboxConfig::validate_command(&self.command)
        {
            return Err(ToolError::CommandNotAllowed(error.to_string()));
        }

        for name in &self.sandbox.env_allowlist {
            if name.to_ascii_uppercase().contains("PROXY")
                && (!self.capabilities.network_egress || !ctx.capabilities.network)
            {
                return Err(ToolError::NetworkBlocked(format!(
                    "proxy environment `{name}` exceeds plugin network capability"
                )));
            }
        }
        for name in self.env.keys() {
            if !self
                .sandbox
                .env_allowlist
                .iter()
                .any(|allowed| allowed == name)
            {
                return Err(ToolError::PermissionDenied(format!(
                    "plugin tool `{}` environment variable `{name}` is not in env_allowlist",
                    self.name
                )));
            }
        }

        if let Some(working_dir) = &self.working_dir
            && !sandbox_allows_path(&self.sandbox.allowed_paths, working_dir)
        {
            return Err(ToolError::PathOutsideWorktree(working_dir.clone()));
        }
        Ok(())
    }
}

fn sandbox_allows_path(patterns: &[String], relative: &Path) -> bool {
    patterns.iter().any(|pattern| {
        glob::Pattern::new(pattern).is_ok_and(|compiled| compiled.matches_path(relative))
            || pattern
                .strip_suffix("/**")
                .is_some_and(|root| Path::new(root) == relative)
    })
}

fn resolve_working_dir(root: &Path, relative: Option<&Path>) -> Result<PathBuf, ToolError> {
    let canonical_root = std::fs::canonicalize(root).map_err(|error| {
        ToolError::Other(format!(
            "failed to resolve plugin worktree `{}`: {error}",
            root.display()
        ))
    })?;
    let Some(relative) = relative else {
        return Ok(canonical_root);
    };
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(ToolError::PathOutsideWorktree(relative.to_path_buf()));
    }
    let candidate = std::fs::canonicalize(canonical_root.join(relative)).map_err(|error| {
        ToolError::Other(format!(
            "failed to resolve plugin working directory `{}`: {error}",
            relative.display()
        ))
    })?;
    if !candidate.starts_with(&canonical_root) {
        return Err(ToolError::PathOutsideWorktree(relative.to_path_buf()));
    }
    Ok(candidate)
}

#[derive(Debug)]
struct CapturedProcessOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[derive(Debug)]
enum CaptureError {
    Io(std::io::Error),
    OutputLimitExceeded,
}

async fn capture_bounded_output(
    child: &mut tokio::process::Child,
    mut stdout: tokio::process::ChildStdout,
    mut stderr: tokio::process::ChildStderr,
    max_bytes: usize,
) -> Result<CapturedProcessOutput, CaptureError> {
    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();
    let mut stdout_buffer = [0_u8; 8 * 1024];
    let mut stderr_buffer = [0_u8; 8 * 1024];
    let mut stdout_open = true;
    let mut stderr_open = true;
    let mut status = None;
    let mut captured_bytes = 0_usize;

    while stdout_open || stderr_open || status.is_none() {
        tokio::select! {
            result = stdout.read(&mut stdout_buffer), if stdout_open => {
                let read = result.map_err(CaptureError::Io)?;
                if read == 0 {
                    stdout_open = false;
                } else {
                    append_bounded(
                        &mut stdout_bytes,
                        &stdout_buffer[..read],
                        &mut captured_bytes,
                        max_bytes,
                    )?;
                }
            }
            result = stderr.read(&mut stderr_buffer), if stderr_open => {
                let read = result.map_err(CaptureError::Io)?;
                if read == 0 {
                    stderr_open = false;
                } else {
                    append_bounded(
                        &mut stderr_bytes,
                        &stderr_buffer[..read],
                        &mut captured_bytes,
                        max_bytes,
                    )?;
                }
            }
            result = child.wait(), if status.is_none() => {
                status = Some(result.map_err(CaptureError::Io)?);
            }
        }
    }

    Ok(CapturedProcessOutput {
        status: status.expect("child status is populated before capture completes"),
        stdout: stdout_bytes,
        stderr: stderr_bytes,
    })
}

fn append_bounded(
    destination: &mut Vec<u8>,
    chunk: &[u8],
    captured_bytes: &mut usize,
    max_bytes: usize,
) -> Result<(), CaptureError> {
    if chunk.len() > max_bytes.saturating_sub(*captured_bytes) {
        return Err(CaptureError::OutputLimitExceeded);
    }
    destination.extend_from_slice(chunk);
    *captured_bytes += chunk.len();
    Ok(())
}

#[derive(Debug)]
struct ProcessGroupGuard {
    pid: Option<u32>,
}

impl ProcessGroupGuard {
    fn new(pid: Option<u32>) -> Self {
        Self { pid }
    }

    fn disarm(&mut self) {
        self.pid = None;
    }

    fn kill_now(&self) {
        #[cfg(unix)]
        if let Some(pid) = self
            .pid
            .and_then(|pid| i32::try_from(pid).ok())
            .and_then(rustix::process::Pid::from_raw)
        {
            let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
        }
    }
}

impl Drop for ProcessGroupGuard {
    fn drop(&mut self) {
        self.kill_now();
    }
}

async fn terminate_plugin_process(
    child: &mut tokio::process::Child,
    process_guard: &ProcessGroupGuard,
) {
    if let Err(error) = kill_tree(child, std::time::Duration::ZERO).await {
        warn!(error = %error, "failed to terminate plugin process tree cleanly");
        process_guard.kill_now();
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
}

fn render_process_output(stdout: &[u8], stderr: &[u8]) -> String {
    let mut output = Vec::with_capacity(stdout.len().saturating_add(stderr.len()));
    output.extend_from_slice(stdout);
    if !stderr.is_empty() {
        if !output.is_empty() && !output.ends_with(b"\n") {
            output.push(b'\n');
        }
        output.extend_from_slice(stderr);
    }
    String::from_utf8_lossy(&output).trim_end().to_string()
}

#[async_trait::async_trait]
impl Extension for PluginExtension {
    fn name(&self) -> &str {
        &self.meta.name
    }

    fn layer(&self) -> ExtensionLayer {
        self.meta.layer
    }

    fn meta(&self) -> ExtensionMeta {
        self.meta.clone()
    }

    async fn on_init(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            extension = %self.meta.name,
            prompts = self.prompt_count,
            tools = self.tools.len(),
            "plugin extension initialized"
        );
        Ok(())
    }

    async fn on_shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(extension = %self.meta.name, "plugin extension shutting down");
        Ok(())
    }

    async fn pre_inference(
        &self,
        request: &mut InferenceRequest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            extension = %self.meta.name,
            plan_id = %request.plan_id,
            task = %request.task,
            "plugin pre_inference hook"
        );
        Ok(())
    }

    async fn post_inference(
        &self,
        response: &mut InferenceResponse,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            extension = %self.meta.name,
            plan_id = %response.plan_id,
            task = %response.task,
            success = response.success,
            "plugin post_inference hook"
        );
        Ok(())
    }

    async fn on_gate(
        &self,
        event: &mut GateEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            extension = %self.meta.name,
            gate = %event.gate_name,
            passed = event.passed,
            "plugin on_gate hook"
        );
        Ok(())
    }
}

// ─── ManifestExtension ──────────────────────────────────────────────────

/// An [`Extension`] backed by a declarative [`ExtensionManifest`] TOML file.
///
/// Handles the non-executable `Prompts`, `Config`, and `Declarative` manifest
/// tiers. WASM manifests are loaded by [`WasmExtension`]; out-of-tree native
/// Rust manifests are rejected instead of being represented as executable.
struct ManifestExtension {
    meta: ExtensionMeta,
    tier: PackageTier,
    timeout_ms: Option<u64>,
}

/// A required extension failure discovered before lifecycle initialization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionStartupFailure {
    pub extension: String,
    pub stage: &'static str,
    pub message: String,
}

impl std::fmt::Display for ExtensionStartupFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "required extension `{}` failed during {}: {}",
            self.extension, self.stage, self.message
        )
    }
}

/// Result of a startup-aware extension discovery pass.
#[derive(Clone, Debug, Default)]
pub struct ExtensionLoadReport {
    pub loaded: usize,
    pub required_failures: Vec<ExtensionStartupFailure>,
}

struct StartupFailureExtension {
    meta: ExtensionMeta,
    failure: ExtensionStartupFailure,
}

#[async_trait::async_trait]
impl Extension for StartupFailureExtension {
    fn name(&self) -> &str {
        &self.meta.name
    }

    fn layer(&self) -> ExtensionLayer {
        self.meta.layer
    }

    fn meta(&self) -> ExtensionMeta {
        self.meta.clone()
    }

    async fn on_init(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err(self.failure.to_string().into())
    }
}

#[async_trait::async_trait]
impl Extension for ManifestExtension {
    fn name(&self) -> &str {
        &self.meta.name
    }

    fn layer(&self) -> ExtensionLayer {
        self.meta.layer
    }

    fn meta(&self) -> ExtensionMeta {
        self.meta.clone()
    }

    async fn on_init(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            extension = %self.meta.name,
            tier = ?self.tier,
            timeout_ms = ?self.timeout_ms,
            "manifest extension initialized"
        );
        Ok(())
    }

    async fn on_shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(extension = %self.meta.name, "manifest extension shutting down");
        Ok(())
    }
}

// ─── Manifest loading ───────────────────────────────────────────────────

/// Load an [`ExtensionManifest`] from a TOML file.
///
/// The TOML may use an `[extension]` wrapper section or have all fields at the
/// top level. At minimum `name`, `version`, `layer`, and `tier` are required.
///
/// # Errors
///
/// Returns an error if the file cannot be read, the TOML is malformed, or the
/// manifest fails schema validation.
pub fn load_extension_manifest(
    path: &Path,
) -> Result<ExtensionManifest, Box<dyn std::error::Error + Send + Sync>> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        format!(
            "failed to read extension manifest at {}: {e}",
            path.display()
        )
    })?;
    parse_extension_manifest(&content).map_err(|e| e.to_string().into())
}

/// Parse an [`ExtensionManifest`] from a TOML string.
///
/// Accepts both a flat manifest (fields at top level) and one with an
/// `[extension]` wrapper section.
///
/// # Errors
///
/// Returns a [`ManifestValidationError`] if the TOML is invalid or validation fails.
pub fn parse_extension_manifest(
    content: &str,
) -> Result<ExtensionManifest, ManifestValidationError> {
    // Try parsing as a wrapped `[extension]` table first.
    let manifest = if let Ok(wrapped) = toml::from_str::<WrappedManifest>(content) {
        wrapped.extension
    } else {
        // Fall back to flat parsing (all fields at top level).
        toml::from_str::<ExtensionManifest>(content).map_err(|e| ManifestValidationError {
            message: format!("failed to parse TOML: {e}"),
        })?
    };

    manifest.validate()?;
    Ok(manifest)
}

/// Intermediate deserialization type for `[extension]`-wrapped TOML files.
#[derive(serde::Deserialize)]
struct WrappedManifest {
    extension: ExtensionManifest,
}

// ─── Directory scanning ─────────────────────────────────────────────────

/// Scan `.roko/extensions/` subdirectories for `extension.toml` or
/// `manifest.toml` files and return the successfully parsed manifests.
///
/// Subdirectories are scanned in alphabetical order for deterministic load
/// ordering. Manifests that fail validation are skipped with a warning.
///
/// The `disable_extensions` list marks matching extensions as disabled
/// (field `LoadedExtension::disabled = true`). Callers decide whether to skip
/// them before adding to the chain.
pub fn scan_extension_manifests(
    extensions_dir: &Path,
    disable_extensions: &[String],
) -> Vec<LoadedExtension> {
    scan_extension_manifests_report(extensions_dir, disable_extensions).loaded
}

fn scan_extension_manifests_report(
    extensions_dir: &Path,
    disable_extensions: &[String],
) -> ManifestScanReport {
    let mut report = ManifestScanReport::default();

    if !extensions_dir.exists() {
        debug!(
            dir = %extensions_dir.display(),
            "extensions directory does not exist, skipping manifest scan"
        );
        return report;
    }

    let entries = match std::fs::read_dir(extensions_dir) {
        Ok(e) => e,
        Err(err) => {
            warn!(
                dir = %extensions_dir.display(),
                error = %err,
                "failed to read extensions directory"
            );
            return report;
        }
    };

    // Collect and sort for deterministic load order.
    let mut dirs: Vec<_> = entries.flatten().filter(|e| e.path().is_dir()).collect();
    dirs.sort_by_key(|e| e.file_name());

    for entry in dirs {
        let dir = entry.path();

        // Probe candidate filenames in preference order.
        let candidates = [dir.join("extension.toml"), dir.join("manifest.toml")];

        let found = candidates.iter().find(|p| p.exists());
        let (manifest_path, manifest) = match found {
            None => {
                debug!(dir = %dir.display(), "no extension.toml or manifest.toml found");
                continue;
            }
            Some(p) => match load_extension_manifest(p) {
                Ok(m) => (p.clone(), m),
                Err(e) => {
                    warn!(
                        path = %p.display(),
                        error = %e,
                        "skipping invalid extension manifest"
                    );
                    let raw = std::fs::read_to_string(p).unwrap_or_default();
                    let parsed = toml::from_str::<toml::Value>(&raw).ok();
                    let table = parsed
                        .as_ref()
                        .and_then(|value| value.get("extension"))
                        .or(parsed.as_ref());
                    let extension = table
                        .and_then(|value| value.get("name"))
                        .and_then(toml::Value::as_str)
                        .filter(|name| !name.trim().is_empty())
                        .map(str::to_string)
                        .or_else(|| entry.file_name().to_str().map(str::to_string))
                        .unwrap_or_else(|| "<unknown>".to_string());
                    let optional = table
                        .and_then(|value| value.get("optional"))
                        .and_then(toml::Value::as_bool)
                        .unwrap_or(false);
                    report.failures.push(ManifestScanFailure {
                        disabled: disable_extensions
                            .iter()
                            .any(|disabled| disabled == &extension),
                        extension,
                        optional,
                        message: e.to_string(),
                    });
                    continue;
                }
            },
        };

        let disabled = disable_extensions.iter().any(|n| n == &manifest.name);

        if disabled {
            debug!(extension = %manifest.name, "extension is in disable_extensions list");
        } else {
            info!(
                extension = %manifest.name,
                version = %manifest.version,
                layer = ?manifest.layer,
                tier = ?manifest.tier,
                path = %manifest_path.display(),
                "loaded extension manifest"
            );
        }

        report.loaded.push(LoadedExtension {
            manifest,
            manifest_path,
            disabled,
        });
    }

    report
}

// ─── Declarative tool catalog ──────────────────────────────────────────

/// Convert a plugin manifest's command declaration into the canonical tool
/// definition advertised to agents.
#[must_use]
pub fn declarative_to_tool_def(tool: &DeclarativeTool, plugin_name: &str) -> ToolDef {
    let name = if tool.name.contains('.') {
        tool.name.clone()
    } else {
        format!("{plugin_name}.{}", tool.name)
    };
    let mut definition = ToolDef::new(
        name,
        tool.description.clone(),
        ToolCategory::Exec,
        ToolPermission {
            read: false,
            write: false,
            exec: true,
            git: false,
            network: false,
        },
    )
    .with_parameters(ToolSchema::any_object())
    .with_timeout_ms(tool.timeout_ms)
    .with_concurrency(ToolConcurrency::Serial);
    definition.source = ToolSource::Plugin {
        name: plugin_name.to_string(),
    };
    definition.metadata = Some(serde_json::json!({
        "command": tool.command,
        "working_dir": tool.working_dir,
    }));
    definition
}

/// Discover and resolve active plugin manifests from every supported project
/// root, then build their canonical definitions and executable handlers.
///
/// The root order also defines same-version precedence:
/// `.roko/extensions`, `plugins`, then `.roko/plugins`.
pub fn resolve_plugin_tool_catalog(
    workdir: &Path,
    extension_names: &[String],
    disable_extensions: &[String],
) -> RokoResult<PluginToolCatalog> {
    let layout = RokoLayout::for_project(workdir);
    let plugin_dirs = [
        layout.extensions_dir(),
        workdir.join("plugins"),
        workdir.join(".roko").join("plugins"),
    ];
    let mut discovered = Vec::new();

    for dir in &plugin_dirs {
        match roko_plugin::manifest::discover_plugins(dir) {
            Ok(plugins) => discovered.extend(plugins),
            Err(error) => warn!(
                dir = %dir.display(),
                error = %error,
                "failed to scan extension directory"
            ),
        }
    }
    discovered.retain(|plugin| {
        let name = &plugin.manifest.plugin.name;
        plugin.manifest.is_enabled()
            && (extension_names.is_empty() || extension_names.iter().any(|enabled| enabled == name))
            && !disable_extensions.iter().any(|disabled| disabled == name)
    });

    let plugins = roko_plugin::manifest::resolve_plugins(discovered)?;
    let mut registry = DynamicToolRegistry::new();
    let mut handlers = HashMap::<String, Arc<dyn ToolHandler>>::new();

    for plugin in &plugins {
        let plugin_name = &plugin.manifest.plugin.name;
        let mut definitions = Vec::with_capacity(plugin.manifest.tools.len());
        for tool in &plugin.manifest.tools {
            let capabilities = plugin.manifest.capabilities();
            let mut definition = declarative_to_tool_def(tool, plugin_name);
            definition.permission.read = capabilities.filesystem_read;
            definition.permission.write = capabilities.filesystem_write;
            definition.permission.exec = capabilities.exec;
            definition.permission.network = capabilities.network_egress;
            let sandbox = plugin.manifest.sandbox_for_tool(tool);
            let max_output_bytes = usize::try_from(sandbox.max_output_bytes)
                .unwrap_or(MAX_PLUGIN_OUTPUT_BYTES)
                .min(MAX_PLUGIN_OUTPUT_BYTES);
            let handler: Arc<dyn ToolHandler> = Arc::new(DeclarativeToolHandler {
                name: definition.name.clone(),
                command: tool.command.clone(),
                working_dir: tool.working_dir.as_deref().map(PathBuf::from),
                env: tool.env.clone(),
                tier: plugin.manifest.tier(),
                capabilities,
                sandbox,
                timeout_ms: tool.timeout_ms,
                max_output_bytes,
                confinement: PluginConfinement::detect(),
            });
            if handlers.insert(definition.name.clone(), handler).is_some() {
                warn!(
                    tool = %definition.name,
                    plugin = %plugin_name,
                    "plugin tool handler overrides an earlier declaration"
                );
            }
            definitions.push(definition);
        }

        let mut registry_sandbox =
            RegistrySandboxConfig::for_tier_level(plugin.manifest.tier().level());
        let effective_sandbox = plugin.manifest.effective_sandbox();
        if !effective_sandbox.allowed_paths.is_empty() {
            registry_sandbox.allowed_paths = effective_sandbox.allowed_paths;
        }
        registry_sandbox.network_access = effective_sandbox.network;
        registry.register_plugin(plugin_name, definitions, registry_sandbox);
    }

    let plugin_tools = registry
        .all()
        .iter()
        .filter(|definition| matches!(&definition.source, ToolSource::Plugin { .. }))
        .cloned()
        .collect::<Vec<_>>();
    let catalog = PluginToolCatalog {
        plugins,
        registry: Arc::new(registry),
        plugin_tools: Arc::new(plugin_tools),
        handlers: Arc::new(handlers),
    };
    let missing_handlers = catalog.local_runtime().missing_handlers();
    if !missing_handlers.is_empty() {
        return Err(RokoError::config(format!(
            "plugin tool catalog contains definitions without handlers: {}",
            missing_handlers.join(", ")
        )));
    }
    Ok(catalog)
}

// ─── Loader ───────────────────────────���─────────────────────────────────

/// Scan extension directories and populate the given [`ExtensionChain`].
///
/// Scans (in order):
/// 1. `<workdir>/.roko/extensions/`
/// 2. `<workdir>/plugins/`
/// 3. `<workdir>/.roko/plugins/` (the `roko plugin install` destination)
///
/// Each directory is probed via [`roko_plugin::manifest::discover_plugins`].
/// Discovered plugins are wrapped as [`PluginExtension`] and added to the
/// chain sorted by layer.
///
/// Additionally, if `extension_names` is non-empty (from `roko.toml`
/// `[agent].extensions`), only extensions whose name appears in that list are
/// loaded. An empty list means "load all discovered extensions".
///
/// Also scans for `extension.toml` / `manifest.toml` files in
/// `.roko/extensions/` subdirectories. See [`scan_extension_manifests`].
pub fn load_extensions(
    workdir: &Path,
    extension_names: &[String],
    chain: &mut ExtensionChain,
) -> usize {
    load_extensions_internal(workdir, extension_names, &[], chain, None).loaded
}

/// Like [`load_extensions`] but also accepts an explicit `disable_extensions`
/// list from the agent config (`[agent].disable_extensions` in `roko.toml`).
pub fn load_extensions_with_disabled(
    workdir: &Path,
    extension_names: &[String],
    disable_extensions: &[String],
    chain: &mut ExtensionChain,
) -> usize {
    load_extensions_internal(workdir, extension_names, disable_extensions, chain, None).loaded
}

/// Load extensions for process startup, preserving every required failure.
///
/// `initial_failures` carries earlier registry-stage failures. Each required
/// failure is also represented by a synthetic Foundation extension whose
/// `on_init` returns the error, allowing the async runner boundary to fail
/// before any task dispatch. Optional manifest failures are logged and
/// isolated without entering this report.
pub fn load_extensions_for_startup(
    workdir: &Path,
    extension_names: &[String],
    disable_extensions: &[String],
    chain: &mut ExtensionChain,
    initial_failures: Vec<ExtensionStartupFailure>,
) -> ExtensionLoadReport {
    load_extensions_internal(
        workdir,
        extension_names,
        disable_extensions,
        chain,
        Some(initial_failures),
    )
}

fn load_extensions_internal(
    workdir: &Path,
    extension_names: &[String],
    disable_extensions: &[String],
    chain: &mut ExtensionChain,
    startup_failures: Option<Vec<ExtensionStartupFailure>>,
) -> ExtensionLoadReport {
    let layout = RokoLayout::for_project(workdir);
    let extensions_dir = layout.extensions_dir();
    let mut loaded = 0usize;
    let strict_startup = startup_failures.is_some();
    let mut failures = startup_failures
        .unwrap_or_default()
        .into_iter()
        .map(|failure| (failure.extension.clone(), failure))
        .collect::<BTreeMap<_, _>>();
    let mut resolved_names = HashSet::new();

    // ── 1. Plugin discovery (plugin.toml files) ──────────────────────────
    let catalog = match resolve_plugin_tool_catalog(workdir, extension_names, disable_extensions) {
        Ok(catalog) => catalog,
        Err(err) => {
            warn!(error = %err, "plugin version/dependency resolution failed; skipping plugin manifests");
            PluginToolCatalog {
                plugins: Vec::new(),
                registry: Arc::new(DynamicToolRegistry::new()),
                plugin_tools: Arc::new(Vec::new()),
                handlers: Arc::new(HashMap::new()),
            }
        }
    };

    for plugin in catalog.plugins() {
        let name = &plugin.manifest.plugin.name;
        resolved_names.insert(name.clone());
        failures.remove(name);
        let tools = catalog
            .registry()
            .by_extension(name)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();

        let ext = PluginExtension {
            meta: ExtensionMeta {
                name: name.clone(),
                layer: ExtensionLayer::Cognition, // default layer for plugin extensions
                // Explicitly configured plugins are required because the
                // plugin manifest has no `optional` field of its own.
                optional: !extension_names.iter().any(|configured| configured == name),
                depends_on: plugin
                    .manifest
                    .dependencies
                    .iter()
                    .map(|d| d.name.clone())
                    .collect(),
                soft_depends_on: Vec::new(),
                version: plugin.manifest.plugin.version.clone(),
                tier: PackageTier::Declarative,
            },
            prompt_count: plugin.manifest.prompts.len(),
            tools,
        };

        info!(
            plugin = %name,
            version = %plugin.manifest.plugin.version,
            prompts = ext.prompt_count,
            tools = ext.tools.len(),
            dir = %plugin.base_dir.display(),
            "loaded plugin extension"
        );

        chain.add(Box::new(ext));
        loaded += 1;
    }

    // ── 2. Extension manifest discovery (extension.toml / manifest.toml) ─
    let manifest_report = scan_extension_manifests_report(&extensions_dir, disable_extensions);
    for failure in manifest_report.failures {
        if failure.disabled
            || (!extension_names.is_empty()
                && !extension_names
                    .iter()
                    .any(|configured| configured == &failure.extension))
        {
            continue;
        }
        resolved_names.insert(failure.extension.clone());
        failures.remove(&failure.extension);
        if strict_startup && !failure.optional {
            failures.insert(
                failure.extension.clone(),
                ExtensionStartupFailure {
                    extension: failure.extension,
                    stage: "manifest_validation",
                    message: failure.message,
                },
            );
        }
    }
    for loaded_ext in manifest_report.loaded {
        // Skip disabled extensions.
        if loaded_ext.disabled {
            debug!(
                extension = %loaded_ext.manifest.name,
                "skipping disabled extension"
            );
            continue;
        }

        let name = &loaded_ext.manifest.name;

        // If an allow-list is configured, skip extensions not in it.
        if !extension_names.is_empty() && !extension_names.iter().any(|n| n == name) {
            debug!(
                extension = %name,
                "extension not in configured extensions list, skipping"
            );
            continue;
        }

        let timeout_ms = loaded_ext.manifest.timeout_ms;
        let tier = loaded_ext.manifest.tier;
        let timeout = std::time::Duration::from_millis(timeout_ms.unwrap_or(5_000));
        let meta = loaded_ext.manifest.clone().into_meta();

        let extension: Box<dyn Extension> = match tier {
            PackageTier::Wasm => match WasmExtension::load(
                meta,
                &loaded_ext.manifest_path,
                &loaded_ext.manifest.config,
                timeout,
            ) {
                Ok(extension) => {
                    info!(extension = %name, "loaded fuel-metered WASM extension");
                    Box::new(extension)
                }
                Err(error) => {
                    warn!(
                        extension = %name,
                        optional = loaded_ext.manifest.optional,
                        error = %error,
                        "WASM extension failed sandbox validation or instantiation; skipping"
                    );
                    resolved_names.insert(name.clone());
                    if strict_startup && !loaded_ext.manifest.optional {
                        failures.insert(
                            name.clone(),
                            ExtensionStartupFailure {
                                extension: name.clone(),
                                stage: "wasm_load",
                                message: error.to_string(),
                            },
                        );
                    }
                    continue;
                }
            },
            PackageTier::NativeRust => {
                warn!(
                    extension = %name,
                    "out-of-tree NativeRust execution is unsupported; refusing to register a no-op"
                );
                resolved_names.insert(name.clone());
                if strict_startup && !loaded_ext.manifest.optional {
                    failures.insert(
                        name.clone(),
                        ExtensionStartupFailure {
                            extension: name.clone(),
                            stage: "native_load",
                            message: "out-of-tree native Rust extensions are unsupported"
                                .to_string(),
                        },
                    );
                }
                continue;
            }
            PackageTier::Prompt | PackageTier::ConfigProfile | PackageTier::Declarative => {
                Box::new(ManifestExtension {
                    meta,
                    tier,
                    timeout_ms,
                })
            }
        };

        if let Some(timeout_ms) = timeout_ms {
            chain.set_timeout_override(name.clone(), std::time::Duration::from_millis(timeout_ms));
        }
        chain.add(extension);
        resolved_names.insert(name.clone());
        failures.remove(name);
        loaded += 1;
    }

    if strict_startup {
        for name in extension_names {
            if disable_extensions.iter().any(|disabled| disabled == name)
                || resolved_names.contains(name)
                || failures.contains_key(name)
            {
                continue;
            }
            failures.insert(
                name.clone(),
                ExtensionStartupFailure {
                    extension: name.clone(),
                    stage: "discovery",
                    message: "configured extension was not found in any local or registry source"
                        .to_string(),
                },
            );
        }

        for failure in failures.values() {
            chain.add(Box::new(StartupFailureExtension {
                meta: ExtensionMeta {
                    name: failure.extension.clone(),
                    layer: ExtensionLayer::Foundation,
                    optional: false,
                    depends_on: Vec::new(),
                    soft_depends_on: Vec::new(),
                    version: String::new(),
                    tier: PackageTier::ConfigProfile,
                },
                failure: failure.clone(),
            }));
        }
    }

    if loaded > 0 || !failures.is_empty() {
        chain.sort_by_layer();
        info!(
            count = loaded,
            "extension chain populated from discovered plugins and manifests"
        );
    } else {
        debug!("no plugin extensions discovered");
    }

    ExtensionLoadReport {
        loaded,
        required_failures: failures.into_values().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_agent::dispatcher::ToolDispatcher;
    use roko_core::tool::ToolContext;

    fn confinement_available() -> bool {
        PluginConfinement::detect().is_supported()
    }

    fn write_plugin(
        root: &Path,
        directory: &str,
        name: &str,
        version: &str,
        dependency: Option<(&str, &str)>,
    ) {
        let plugin_dir = root.join(directory);
        std::fs::create_dir_all(&plugin_dir).unwrap();
        let dependency_toml = dependency.map_or_else(String::new, |(dependency, version)| {
            format!("\n[[dependencies]]\nname = \"{dependency}\"\nversion = \"{version}\"\n")
        });
        std::fs::write(
            plugin_dir.join("plugin.toml"),
            format!("[plugin]\nname = \"{name}\"\nversion = \"{version}\"\n{dependency_toml}"),
        )
        .unwrap();
    }

    #[test]
    fn declarative_conversion_uses_canonical_plugin_name_and_exec_contract() {
        let tool = DeclarativeTool {
            name: "lint".to_string(),
            description: "run lint".to_string(),
            command: "cargo clippy".to_string(),
            timeout_ms: 1_234,
            working_dir: None,
            env: HashMap::new(),
            sandbox: None,
        };
        let definition = declarative_to_tool_def(&tool, "quality");
        assert_eq!(definition.name, "quality.lint");
        assert_eq!(definition.category, ToolCategory::Exec);
        assert_eq!(definition.timeout_ms, 1_234);
        assert!(definition.permission.exec);
        assert!(!definition.permission.read);
        assert!(matches!(
            definition.source,
            ToolSource::Plugin { ref name } if name == "quality"
        ));

        let mut qualified = tool;
        qualified.name = "shared.lint".to_string();
        assert_eq!(
            declarative_to_tool_def(&qualified, "quality").name,
            "shared.lint"
        );
    }

    #[tokio::test]
    async fn discovered_declarative_tool_is_advertised_and_executable() {
        if !confinement_available() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let plugin_dir = tmp.path().join(".roko/plugins/example");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join("plugin.toml"),
            r#"
tier = "standard"

[capabilities]
exec = true
filesystem_read = true
filesystem_write = true

[plugin]
name = "example"
version = "1.0.0"

[[tools]]
name = "echo"
description = "print a fixed marker"
command = "printf plugin-ok"
timeout_ms = 1000
"#,
        )
        .unwrap();

        let catalog = resolve_plugin_tool_catalog(tmp.path(), &[], &[]).unwrap();
        let definition = catalog
            .registry()
            .get("example.echo")
            .expect("declarative definition is registered");
        assert!(definition.permission.exec);
        assert!(definition.permission.read);
        assert!(definition.permission.write);
        assert!(!definition.permission.network);
        assert!(
            catalog
                .plugin_tools()
                .iter()
                .any(|tool| tool.name == "example.echo")
        );
        assert!(catalog.local_runtime().missing_handlers().is_empty());
        assert!(!catalog.validation_issues().iter().any(|issue| {
            matches!(
                issue,
                ToolValidationIssue::UnhandledTool { name } if name == "example.echo"
            )
        }));

        let dispatcher = ToolDispatcher::new(
            Arc::clone(catalog.registry()) as Arc<dyn ToolRegistry>,
            catalog.resolver(),
        );
        let result = dispatcher
            .dispatch(
                ToolCall::new("plugin-call", "example.echo", serde_json::json!({})),
                &ToolContext::testing(tmp.path()),
            )
            .await;
        match &result {
            ToolResult::Ok { content, .. } => {
                let text: String = content.iter().filter_map(|c| c.as_text()).collect();
                assert_eq!(text, "plugin-ok");
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn declarative_handler_rechecks_tier_before_spawn() {
        let handler = DeclarativeToolHandler {
            name: "example.denied".to_string(),
            command: "printf should-not-run".to_string(),
            working_dir: None,
            env: HashMap::new(),
            tier: PluginTier::Sandboxed,
            capabilities: PluginCapability::declarative_tools(),
            sandbox: PluginSandboxConfig::default(),
            timeout_ms: 1_000,
            max_output_bytes: 1024,
            confinement: PluginConfinement::detect(),
        };
        let result = handler
            .execute(
                ToolCall::new("denied", "example.denied", serde_json::json!({})),
                &ToolContext::testing("."),
            )
            .await;
        assert!(matches!(
            result,
            ToolResult::Err(ToolError::PermissionDenied(_))
        ));
    }

    #[tokio::test]
    async fn declarative_handler_enforces_its_own_timeout() {
        if !confinement_available() {
            return;
        }
        let handler = DeclarativeToolHandler {
            name: "example.slow".to_string(),
            command: "sleep 1".to_string(),
            working_dir: None,
            env: HashMap::new(),
            tier: PluginTier::Standard,
            capabilities: PluginCapability::declarative_tools(),
            sandbox: PluginSandboxConfig::default(),
            timeout_ms: 10,
            max_output_bytes: 1024,
            confinement: PluginConfinement::detect(),
        };
        let result = handler
            .execute(
                ToolCall::new("slow", "example.slow", serde_json::json!({})),
                &ToolContext::testing("."),
            )
            .await;
        assert!(matches!(result, ToolResult::Err(ToolError::Timeout { .. })));
    }

    #[tokio::test]
    async fn declarative_handler_rechecks_runtime_capabilities_before_spawn() {
        let tmp = tempfile::tempdir().unwrap();
        let marker = tmp.path().join("should-not-exist");
        let handler = DeclarativeToolHandler {
            name: "example.context-denied".to_string(),
            command: "touch should-not-exist".to_string(),
            working_dir: None,
            env: HashMap::new(),
            tier: PluginTier::Standard,
            capabilities: PluginCapability {
                filesystem_write: true,
                ..PluginCapability::declarative_tools()
            },
            sandbox: PluginSandboxConfig::default(),
            timeout_ms: 1_000,
            max_output_bytes: 1024,
            confinement: PluginConfinement::detect(),
        };
        let mut ctx = ToolContext::testing(tmp.path());
        ctx.capabilities.write = false;

        let result = handler
            .execute(
                ToolCall::new(
                    "context-denied",
                    "example.context-denied",
                    serde_json::json!({}),
                ),
                &ctx,
            )
            .await;

        assert!(matches!(
            result,
            ToolResult::Err(ToolError::PermissionDenied(ref message))
                if message.contains("filesystem_write")
        ));
        assert!(!marker.exists(), "denied command must not be spawned");
    }

    #[tokio::test]
    async fn declarative_handler_requires_runtime_network_grant() {
        let handler = DeclarativeToolHandler {
            name: "example.network-denied".to_string(),
            command: "printf should-not-run".to_string(),
            working_dir: None,
            env: HashMap::new(),
            tier: PluginTier::Trusted,
            capabilities: PluginCapability {
                network_egress: true,
                ..PluginCapability::declarative_tools()
            },
            sandbox: PluginSandboxConfig::default(),
            timeout_ms: 1_000,
            max_output_bytes: 1024,
            confinement: PluginConfinement::detect(),
        };

        let result = handler
            .execute(
                ToolCall::new(
                    "network-denied",
                    "example.network-denied",
                    serde_json::json!({}),
                ),
                &ToolContext::testing("."),
            )
            .await;

        assert!(matches!(
            result,
            ToolResult::Err(ToolError::PermissionDenied(ref message))
                if message.contains("network_egress")
        ));
    }

    #[tokio::test]
    async fn declarative_handler_only_inherits_allowlisted_environment() {
        if !confinement_available() {
            return;
        }
        let host_path = std::env::var("PATH").expect("test process has PATH");
        let handler = DeclarativeToolHandler {
            name: "example.environment".to_string(),
            command: "printf '%s:%s' \"${PATH-unset}\" \"${HOME-unset}\"".to_string(),
            working_dir: None,
            env: HashMap::new(),
            tier: PluginTier::Trusted,
            capabilities: PluginCapability {
                secrets: true,
                ..PluginCapability::declarative_tools()
            },
            sandbox: PluginSandboxConfig {
                env_allowlist: vec!["PATH".to_string()],
                allow_shell_metacharacters: true,
                ..PluginSandboxConfig::default()
            },
            timeout_ms: 1_000,
            max_output_bytes: 16 * 1024,
            confinement: PluginConfinement::detect(),
        };

        let result = handler
            .execute(
                ToolCall::new("environment", "example.environment", serde_json::json!({})),
                &ToolContext::testing("."),
            )
            .await;

        match &result {
            ToolResult::Ok { content, .. } => {
                let text: String = content.iter().filter_map(|c| c.as_text()).collect();
                assert_eq!(text, format!("{host_path}:unset"));
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn declarative_handler_rejects_environment_without_secret_capability() {
        let handler = DeclarativeToolHandler {
            name: "example.environment-denied".to_string(),
            command: "printf should-not-run".to_string(),
            working_dir: None,
            env: HashMap::new(),
            tier: PluginTier::Standard,
            capabilities: PluginCapability::declarative_tools(),
            sandbox: PluginSandboxConfig {
                env_allowlist: vec!["HOME".to_string()],
                ..PluginSandboxConfig::default()
            },
            timeout_ms: 1_000,
            max_output_bytes: 1024,
            confinement: PluginConfinement::detect(),
        };

        let result = handler
            .execute(
                ToolCall::new(
                    "environment-denied",
                    "example.environment-denied",
                    serde_json::json!({}),
                ),
                &ToolContext::testing("."),
            )
            .await;

        assert!(matches!(
            result,
            ToolResult::Err(ToolError::PermissionDenied(ref message))
                if message.contains("`secrets` capability")
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn declarative_handler_rejects_symlink_working_directory_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), tmp.path().join("escape")).unwrap();
        let handler = DeclarativeToolHandler {
            name: "example.symlink".to_string(),
            command: "printf should-not-run".to_string(),
            working_dir: Some(PathBuf::from("escape")),
            env: HashMap::new(),
            tier: PluginTier::Standard,
            capabilities: PluginCapability::declarative_tools(),
            sandbox: PluginSandboxConfig {
                allowed_paths: vec!["escape/**".to_string()],
                ..PluginSandboxConfig::default()
            },
            timeout_ms: 1_000,
            max_output_bytes: 1024,
            confinement: PluginConfinement::detect(),
        };

        let result = handler
            .execute(
                ToolCall::new("symlink", "example.symlink", serde_json::json!({})),
                &ToolContext::testing(tmp.path()),
            )
            .await;

        assert!(matches!(
            result,
            ToolResult::Err(ToolError::PathOutsideWorktree(ref path)) if path == Path::new("escape")
        ));
    }

    #[tokio::test]
    async fn declarative_handler_stops_at_combined_output_limit() {
        if !confinement_available() {
            return;
        }
        let handler = DeclarativeToolHandler {
            name: "example.output-flood".to_string(),
            command: "yes flood".to_string(),
            working_dir: None,
            env: HashMap::new(),
            tier: PluginTier::Standard,
            capabilities: PluginCapability::declarative_tools(),
            sandbox: PluginSandboxConfig::default(),
            timeout_ms: 2_000,
            max_output_bytes: 257,
            confinement: PluginConfinement::detect(),
        };

        let result = handler
            .execute(
                ToolCall::new(
                    "output-flood",
                    "example.output-flood",
                    serde_json::json!({}),
                ),
                &ToolContext::testing("."),
            )
            .await;

        assert!(matches!(
            result,
            ToolResult::Err(ToolError::Other(ref message))
                if message.contains("combined stdout/stderr limit of 257 bytes")
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn declarative_handler_timeout_removes_descendant_processes() {
        if !confinement_available() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let handler = DeclarativeToolHandler {
            name: "example.process-tree".to_string(),
            command: "sleep 30 & child=$!; printf '%s' \"$child\" > child.pid; wait".to_string(),
            working_dir: None,
            env: HashMap::new(),
            tier: PluginTier::Standard,
            capabilities: PluginCapability {
                filesystem_write: true,
                ..PluginCapability::declarative_tools()
            },
            sandbox: PluginSandboxConfig {
                allowed_paths: vec!["**".to_string()],
                allow_shell_metacharacters: true,
                ..PluginSandboxConfig::default()
            },
            timeout_ms: 200,
            max_output_bytes: 1024,
            confinement: PluginConfinement::detect(),
        };

        let result = handler
            .execute(
                ToolCall::new(
                    "process-tree",
                    "example.process-tree",
                    serde_json::json!({}),
                ),
                &ToolContext::testing(tmp.path()),
            )
            .await;
        assert!(matches!(result, ToolResult::Err(ToolError::Timeout { .. })));

        let child_pid = std::fs::read_to_string(tmp.path().join("child.pid"))
            .expect("shell wrote descendant pid")
            .parse::<i32>()
            .expect("descendant pid is numeric");
        let child_pid = rustix::process::Pid::from_raw(child_pid).expect("positive pid");
        assert!(
            rustix::process::test_kill_process(child_pid).is_err(),
            "timed-out plugin descendant is still alive"
        );
    }

    #[tokio::test]
    async fn unsupported_confinement_denies_before_spawning_plugin_binary() {
        let tmp = tempfile::tempdir().unwrap();
        let marker = tmp.path().join("must-not-exist");
        let handler = DeclarativeToolHandler {
            name: "example.unsupported-host".to_string(),
            command: "touch must-not-exist".to_string(),
            working_dir: None,
            env: HashMap::new(),
            tier: PluginTier::Standard,
            capabilities: PluginCapability {
                filesystem_write: true,
                ..PluginCapability::declarative_tools()
            },
            sandbox: PluginSandboxConfig::default(),
            timeout_ms: 1_000,
            max_output_bytes: 1_024,
            confinement: PluginConfinement::Unsupported {
                reason: "test host has no kernel sandbox".to_string(),
            },
        };

        let result = handler
            .execute(
                ToolCall::new(
                    "unsupported-host",
                    "example.unsupported-host",
                    serde_json::json!({}),
                ),
                &ToolContext::testing(tmp.path()),
            )
            .await;

        assert!(matches!(
            result,
            ToolResult::Err(ToolError::PermissionDenied(ref message))
                if message.contains("requires kernel confinement")
        ));
        assert!(!marker.exists(), "unconfined plugin command was spawned");
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn seatbelt_denies_signalling_processes_outside_plugin_sandbox() {
        let mut outside = std::process::Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .expect("spawn outside process");
        let outside_pid = outside.id();
        let handler = DeclarativeToolHandler {
            name: "example.signal".to_string(),
            command: format!("kill -TERM {outside_pid}"),
            working_dir: None,
            env: HashMap::new(),
            tier: PluginTier::Standard,
            capabilities: PluginCapability::declarative_tools(),
            sandbox: PluginSandboxConfig::default(),
            timeout_ms: 1_000,
            max_output_bytes: 1_024,
            confinement: PluginConfinement::detect(),
        };

        let result = handler
            .execute(
                ToolCall::new("signal", "example.signal", serde_json::json!({})),
                &ToolContext::testing("."),
            )
            .await;
        assert!(matches!(result, ToolResult::Err(ToolError::Other(_))));
        assert!(
            outside
                .try_wait()
                .expect("inspect outside process")
                .is_none(),
            "plugin sandbox signalled an external process"
        );
        let _ = outside.kill();
        let _ = outside.wait();
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn seatbelt_denies_loopback_network_without_network_capability() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback listener");
        let address = listener.local_addr().expect("listener address");
        let accept = tokio::spawn(async move {
            tokio::time::timeout(std::time::Duration::from_millis(750), listener.accept()).await
        });
        let handler = DeclarativeToolHandler {
            name: "example.network".to_string(),
            command: format!("/usr/bin/curl --silent --max-time 0.5 http://{address}/"),
            working_dir: None,
            env: HashMap::new(),
            tier: PluginTier::Standard,
            capabilities: PluginCapability::declarative_tools(),
            sandbox: PluginSandboxConfig::default(),
            timeout_ms: 1_000,
            max_output_bytes: 1_024,
            confinement: PluginConfinement::detect(),
        };

        let result = handler
            .execute(
                ToolCall::new("network", "example.network", serde_json::json!({})),
                &ToolContext::testing("."),
            )
            .await;
        assert!(matches!(result, ToolResult::Err(ToolError::Other(_))));
        assert!(
            accept.await.expect("accept task").is_err(),
            "plugin without network capability reached a loopback socket"
        );
    }

    #[test]
    fn load_from_nonexistent_dirs_returns_zero() {
        let mut chain = ExtensionChain::new();
        let count = load_extensions(Path::new("/nonexistent/workspace"), &[], &mut chain);
        assert_eq!(count, 0);
        assert!(chain.is_empty());
    }

    #[test]
    fn load_with_empty_dir_returns_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();
        std::fs::create_dir_all(&ext_dir).unwrap();

        let mut chain = ExtensionChain::new();
        let count = load_extensions(tmp.path(), &[], &mut chain);
        assert_eq!(count, 0);
    }

    #[test]
    fn load_discovers_plugin_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path())
            .extensions_dir()
            .join("test-ext");
        std::fs::create_dir_all(&ext_dir).unwrap();

        std::fs::write(
            ext_dir.join("plugin.toml"),
            r#"
[plugin]
name = "test-ext"
version = "0.1.0"
description = "A test extension"

[[prompts]]
name = "test-prompt"
template = "Hello, world!"
"#,
        )
        .unwrap();

        let mut chain = ExtensionChain::new();
        let count = load_extensions(tmp.path(), &[], &mut chain);
        assert_eq!(count, 1);
        assert_eq!(chain.len(), 1);

        let meta = chain.metadata();
        assert_eq!(meta[0].name, "test-ext");
        assert_eq!(meta[0].version, "0.1.0");
        assert!(meta[0].optional);
    }

    #[test]
    fn runtime_loader_resolves_versions_across_all_roots_and_orders_dependencies() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = RokoLayout::for_project(tmp.path());
        write_plugin(
            &layout.extensions_dir(),
            "shared-old",
            "shared",
            "1.5.0",
            None,
        );
        write_plugin(
            &tmp.path().join(".roko/plugins"),
            "shared-installed",
            "shared",
            "2.1.0",
            None,
        );
        write_plugin(
            &tmp.path().join("plugins"),
            "consumer",
            "consumer",
            "1.0.0",
            Some(("shared", ">=2.0.0")),
        );

        let mut chain = ExtensionChain::new();
        let count = load_extensions(tmp.path(), &[], &mut chain);

        assert_eq!(count, 2, "duplicate plugin names must resolve once");
        let metadata = chain.metadata();
        assert_eq!(metadata[0].name, "shared");
        assert_eq!(metadata[0].version, "2.1.0");
        assert_eq!(metadata[1].name, "consumer");
        assert_eq!(metadata[1].depends_on, vec!["shared"]);
    }

    #[test]
    fn runtime_loader_skips_unresolved_plugin_set_when_dependency_is_missing() {
        let tmp = tempfile::tempdir().unwrap();
        write_plugin(
            &tmp.path().join(".roko/plugins"),
            "consumer",
            "consumer",
            "1.0.0",
            Some(("missing", "1.0.0")),
        );

        let mut chain = ExtensionChain::new();
        let count = load_extensions(tmp.path(), &[], &mut chain);

        assert_eq!(count, 0);
        assert!(chain.is_empty());
    }

    #[test]
    fn runtime_loader_skips_manifest_disabled_plugins() {
        let tmp = tempfile::tempdir().unwrap();
        write_plugin(
            &tmp.path().join(".roko/plugins"),
            "disabled",
            "disabled",
            "1.0.0",
            None,
        );
        let manifest_path = tmp.path().join(".roko/plugins/disabled/plugin.toml");
        std::fs::write(
            manifest_path,
            "[plugin]\nname = \"disabled\"\nversion = \"1.0.0\"\nenabled = false\n",
        )
        .unwrap();

        let mut chain = ExtensionChain::new();
        let count = load_extensions(tmp.path(), &[], &mut chain);

        assert_eq!(count, 0);
        assert!(chain.is_empty());
    }

    #[test]
    fn runtime_loader_rejects_enabled_plugin_with_disabled_dependency() {
        let tmp = tempfile::tempdir().unwrap();
        write_plugin(
            &tmp.path().join("plugins"),
            "dependency",
            "dependency",
            "1.0.0",
            None,
        );
        std::fs::write(
            tmp.path().join("plugins/dependency/plugin.toml"),
            "[plugin]\nname = \"dependency\"\nversion = \"1.0.0\"\nenabled = false\n",
        )
        .unwrap();
        write_plugin(
            &tmp.path().join("plugins"),
            "consumer",
            "consumer",
            "1.0.0",
            Some(("dependency", "1.0.0")),
        );

        let mut chain = ExtensionChain::new();
        let count = load_extensions(tmp.path(), &[], &mut chain);

        assert_eq!(count, 0);
        assert!(chain.is_empty());
    }

    #[test]
    fn load_respects_allow_list() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = RokoLayout::for_project(tmp.path());
        let ext_dir = layout.extensions_dir().join("allowed");
        std::fs::create_dir_all(&ext_dir).unwrap();
        std::fs::write(
            ext_dir.join("plugin.toml"),
            r#"
[plugin]
name = "allowed-ext"
version = "0.1.0"
"#,
        )
        .unwrap();

        let skip_dir = layout.extensions_dir().join("skipped");
        std::fs::create_dir_all(&skip_dir).unwrap();
        std::fs::write(
            skip_dir.join("plugin.toml"),
            r#"
[plugin]
name = "skipped-ext"
version = "0.1.0"
"#,
        )
        .unwrap();

        let mut chain = ExtensionChain::new();
        let count = load_extensions(tmp.path(), &["allowed-ext".to_string()], &mut chain);
        assert_eq!(count, 1);
        assert_eq!(chain.metadata()[0].name, "allowed-ext");
    }

    // ── parse_extension_manifest ──────────────────────────────────────────

    #[test]
    fn parse_extension_manifest_wrapped_section() {
        let toml_str = r#"
[extension]
name = "my-ext"
version = "1.0.0"
layer = "action"
tier = "declarative"
optional = true
timeout_ms = 10000
"#;
        let manifest = parse_extension_manifest(toml_str).unwrap();
        assert_eq!(manifest.name, "my-ext");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.layer, ExtensionLayer::Action);
        assert_eq!(manifest.tier, PackageTier::Declarative);
        assert!(manifest.optional);
        assert_eq!(manifest.timeout_ms, Some(10000));
    }

    #[test]
    fn parse_extension_manifest_flat() {
        let toml_str = r#"
name = "flat-ext"
version = "0.2.0"
layer = "cognition"
tier = "config"
"#;
        let manifest = parse_extension_manifest(toml_str).unwrap();
        assert_eq!(manifest.name, "flat-ext");
        assert_eq!(manifest.tier, PackageTier::ConfigProfile);
    }

    #[test]
    fn parse_extension_manifest_with_tags_and_depends() {
        let toml_str = r#"
[extension]
name = "tagged-ext"
version = "1.1.0"
layer = "meta"
tier = "prompts"
description = "A tagged extension"
tags = ["observability", "metrics"]
depends_on = ["base-ext"]
"#;
        let manifest = parse_extension_manifest(toml_str).unwrap();
        assert_eq!(manifest.tags, vec!["observability", "metrics"]);
        assert_eq!(manifest.depends_on, vec!["base-ext"]);
        assert_eq!(manifest.description, "A tagged extension");
    }

    #[test]
    fn parse_extension_manifest_rejects_empty_name() {
        let toml_str = r#"
[extension]
name = ""
version = "1.0.0"
layer = "action"
tier = "declarative"
"#;
        let result = parse_extension_manifest(toml_str);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("name must not be empty"));
    }

    #[test]
    fn parse_extension_manifest_rejects_invalid_semver() {
        let toml_str = r#"
[extension]
name = "bad-version"
version = "not-semver"
layer = "action"
tier = "declarative"
"#;
        let result = parse_extension_manifest(toml_str);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("semver"));
    }

    #[test]
    fn parse_extension_manifest_rejects_invalid_name_chars() {
        let toml_str = r#"
[extension]
name = "bad name!"
version = "1.0.0"
layer = "action"
tier = "declarative"
"#;
        let result = parse_extension_manifest(toml_str);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid characters")
        );
    }

    // ── load_extension_manifest (file I/O) ───────────────────────────────

    #[test]
    fn load_extension_manifest_from_file() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest_path = tmp.path().join("extension.toml");
        std::fs::write(
            &manifest_path,
            r#"
[extension]
name = "file-ext"
version = "2.0.0"
layer = "social"
tier = "prompts"
"#,
        )
        .unwrap();

        let manifest = load_extension_manifest(&manifest_path).unwrap();
        assert_eq!(manifest.name, "file-ext");
        assert_eq!(manifest.version, "2.0.0");
        assert_eq!(manifest.tier, PackageTier::Prompt);
    }

    #[test]
    fn load_extension_manifest_nonexistent_file() {
        let result = load_extension_manifest(Path::new("/nonexistent/extension.toml"));
        assert!(result.is_err());
    }

    // ── scan_extension_manifests ─────────────────────────────────────────

    #[test]
    fn scan_extension_manifests_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();
        std::fs::create_dir_all(&ext_dir).unwrap();

        let results = scan_extension_manifests(&ext_dir, &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn scan_extension_manifests_nonexistent_dir() {
        let results = scan_extension_manifests(Path::new("/nonexistent/extensions"), &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn scan_extension_manifests_discovers_extension_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();
        let ext_subdir = ext_dir.join("my-ext");
        std::fs::create_dir_all(&ext_subdir).unwrap();

        std::fs::write(
            ext_subdir.join("extension.toml"),
            r#"
[extension]
name = "my-ext"
version = "1.0.0"
layer = "action"
tier = "declarative"
"#,
        )
        .unwrap();

        let results = scan_extension_manifests(&ext_dir, &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].manifest.name, "my-ext");
        assert!(!results[0].disabled);
        assert!(results[0].manifest_path.ends_with("extension.toml"));
    }

    #[test]
    fn scan_extension_manifests_discovers_manifest_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();
        let ext_subdir = ext_dir.join("other-ext");
        std::fs::create_dir_all(&ext_subdir).unwrap();

        std::fs::write(
            ext_subdir.join("manifest.toml"),
            r#"
[extension]
name = "other-ext"
version = "0.5.0"
layer = "memory"
tier = "config"
"#,
        )
        .unwrap();

        let results = scan_extension_manifests(&ext_dir, &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].manifest.name, "other-ext");
        assert!(results[0].manifest_path.ends_with("manifest.toml"));
    }

    #[test]
    fn scan_extension_manifests_prefers_extension_toml_over_manifest_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();
        let ext_subdir = ext_dir.join("dual-ext");
        std::fs::create_dir_all(&ext_subdir).unwrap();

        // Write both files — extension.toml should win.
        std::fs::write(
            ext_subdir.join("extension.toml"),
            r#"
[extension]
name = "extension-wins"
version = "1.0.0"
layer = "action"
tier = "declarative"
"#,
        )
        .unwrap();
        std::fs::write(
            ext_subdir.join("manifest.toml"),
            r#"
[extension]
name = "manifest-loses"
version = "1.0.0"
layer = "action"
tier = "declarative"
"#,
        )
        .unwrap();

        let results = scan_extension_manifests(&ext_dir, &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].manifest.name, "extension-wins");
    }

    #[test]
    fn scan_extension_manifests_marks_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();
        let ext_subdir = ext_dir.join("disabled-ext");
        std::fs::create_dir_all(&ext_subdir).unwrap();

        std::fs::write(
            ext_subdir.join("extension.toml"),
            r#"
[extension]
name = "disabled-ext"
version = "1.0.0"
layer = "action"
tier = "declarative"
"#,
        )
        .unwrap();

        let results = scan_extension_manifests(&ext_dir, &["disabled-ext".to_string()]);
        assert_eq!(results.len(), 1);
        assert!(results[0].disabled);
    }

    #[test]
    fn scan_extension_manifests_skips_invalid_manifests() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();

        // Write an invalid manifest (empty name).
        let bad_subdir = ext_dir.join("bad-ext");
        std::fs::create_dir_all(&bad_subdir).unwrap();
        std::fs::write(
            bad_subdir.join("extension.toml"),
            r#"
[extension]
name = ""
version = "1.0.0"
layer = "action"
tier = "declarative"
"#,
        )
        .unwrap();

        // Write a valid manifest.
        let good_subdir = ext_dir.join("good-ext");
        std::fs::create_dir_all(&good_subdir).unwrap();
        std::fs::write(
            good_subdir.join("extension.toml"),
            r#"
[extension]
name = "good-ext"
version = "1.0.0"
layer = "action"
tier = "declarative"
"#,
        )
        .unwrap();

        let results = scan_extension_manifests(&ext_dir, &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].manifest.name, "good-ext");
    }

    // ── load_extensions_with_disabled (manifest integration) ─────────────

    #[test]
    fn load_extensions_discovers_extension_toml_manifests() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();
        let ext_subdir = ext_dir.join("my-manifest-ext");
        std::fs::create_dir_all(&ext_subdir).unwrap();

        std::fs::write(
            ext_subdir.join("extension.toml"),
            r#"
[extension]
name = "my-manifest-ext"
version = "1.0.0"
layer = "cognition"
tier = "declarative"
optional = true
"#,
        )
        .unwrap();

        let mut chain = ExtensionChain::new();
        let count = load_extensions(tmp.path(), &[], &mut chain);
        assert_eq!(count, 1);
        let meta = chain.metadata();
        assert_eq!(meta[0].name, "my-manifest-ext");
        assert!(meta[0].optional);
    }

    #[test]
    fn load_extensions_with_disabled_skips_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();
        let ext_subdir = ext_dir.join("disabled-manifest-ext");
        std::fs::create_dir_all(&ext_subdir).unwrap();

        std::fs::write(
            ext_subdir.join("extension.toml"),
            r#"
[extension]
name = "disabled-manifest-ext"
version = "1.0.0"
layer = "action"
tier = "declarative"
"#,
        )
        .unwrap();

        let mut chain = ExtensionChain::new();
        let count = load_extensions_with_disabled(
            tmp.path(),
            &[],
            &["disabled-manifest-ext".to_string()],
            &mut chain,
        );
        assert_eq!(count, 0);
        assert!(chain.is_empty());
    }

    #[test]
    fn load_extensions_allow_list_filters_manifest_extensions() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = RokoLayout::for_project(tmp.path()).extensions_dir();

        // Write two manifests.
        for (name, version) in [("allowed-mext", "1.0.0"), ("other-mext", "1.0.0")] {
            let subdir = ext_dir.join(name);
            std::fs::create_dir_all(&subdir).unwrap();
            std::fs::write(
                subdir.join("extension.toml"),
                format!(
                    r#"
[extension]
name = "{name}"
version = "{version}"
layer = "action"
tier = "declarative"
"#
                ),
            )
            .unwrap();
        }

        let mut chain = ExtensionChain::new();
        let count = load_extensions(tmp.path(), &["allowed-mext".to_string()], &mut chain);
        assert_eq!(count, 1);
        assert_eq!(chain.metadata()[0].name, "allowed-mext");
    }

    fn write_wasm_extension(root: &Path, name: &str, wat: &str, optional: bool, config: &str) {
        let extension_dir = RokoLayout::for_project(root).extensions_dir().join(name);
        std::fs::create_dir_all(&extension_dir).unwrap();
        std::fs::write(
            extension_dir.join("hook.wasm"),
            wat::parse_str(wat).unwrap(),
        )
        .unwrap();
        std::fs::write(
            extension_dir.join("extension.toml"),
            format!(
                r#"
[extension]
name = "{name}"
version = "1.0.0"
layer = "cognition"
tier = "wasm"
timeout_ms = 5000
optional = {optional}

[extension.config]
module = "hook.wasm"
{config}
"#
            ),
        )
        .unwrap();
    }

    fn typed_hook_wat(hook: &str, output: &str) -> String {
        let escaped = output
            .as_bytes()
            .iter()
            .map(|byte| format!("\\{byte:02x}"))
            .collect::<String>();
        format!(
            r#"(module
                (memory (export "memory") 1)
                (func (export "roko_alloc") (param i32) (result i32) i32.const 4096)
                (data (i32.const 0) "{escaped}")
                (func (export "{hook}") (param i32 i32) (result i64)
                    i64.const {output_len}))"#,
            output_len = output.len()
        )
    }

    #[tokio::test]
    async fn wasm_extension_loads_and_executes_exported_hook() {
        let tmp = tempfile::tempdir().unwrap();
        write_wasm_extension(
            tmp.path(),
            "wasm-ok",
            &typed_hook_wat("on_init", "null"),
            false,
            "fuel = 10000\nmemory_mb = 1\nhooks = [\"on_init\"]",
        );

        let mut chain = ExtensionChain::new();
        assert_eq!(load_extensions(tmp.path(), &[], &mut chain), 1);
        assert!(chain.init_all().await.is_empty());
    }

    #[tokio::test]
    async fn wasm_extension_infinite_hook_exhausts_fuel() {
        let tmp = tempfile::tempdir().unwrap();
        write_wasm_extension(
            tmp.path(),
            "wasm-fuel",
            r#"(module
                (memory (export "memory") 1)
                (func (export "roko_alloc") (param i32) (result i32) i32.const 4096)
                (func (export "on_init") (param i32 i32) (result i64)
                    (loop br 0) i64.const 0))"#,
            false,
            "fuel = 100\nmemory_mb = 1\nhooks = [\"on_init\"]",
        );

        let mut chain = ExtensionChain::new();
        assert_eq!(load_extensions(tmp.path(), &[], &mut chain), 1);
        let errors = chain.init_all().await;
        assert_eq!(errors.len(), 1);
        assert!(errors[0].1.to_string().contains("fuel"));
    }

    #[test]
    fn wasm_extension_rejects_wasi_imports_and_excess_memory() {
        let tmp = tempfile::tempdir().unwrap();
        write_wasm_extension(
            tmp.path(),
            "wasm-wasi",
            r#"(module
                (import "wasi_snapshot_preview1" "fd_write"
                    (func $fd_write (param i32 i32 i32 i32) (result i32))))"#,
            false,
            "fuel = 10000\nmemory_mb = 1\nhooks = [\"on_init\"]",
        );
        write_wasm_extension(
            tmp.path(),
            "wasm-memory",
            r#"(module
                (memory (export "memory") 32)
                (func (export "roko_alloc") (param i32) (result i32) i32.const 4096)
                (func (export "on_init") (param i32 i32) (result i64) i64.const 4))"#,
            false,
            "fuel = 10000\nmemory_mb = 1\nhooks = [\"on_init\"]",
        );

        let mut chain = ExtensionChain::new();
        assert_eq!(load_extensions(tmp.path(), &[], &mut chain), 0);
        assert!(chain.is_empty());
    }

    #[tokio::test]
    async fn configured_required_wasm_load_failure_becomes_fatal_init_error() {
        let tmp = tempfile::tempdir().unwrap();
        write_wasm_extension(
            tmp.path(),
            "required-wasm",
            &typed_hook_wat("on_shutdown", "null"),
            false,
            "fuel = 10000\nmemory_mb = 1\nhooks = [\"on_init\"]",
        );

        let mut chain = ExtensionChain::new();
        let report = load_extensions_for_startup(
            tmp.path(),
            &["required-wasm".to_string()],
            &[],
            &mut chain,
            Vec::new(),
        );
        assert_eq!(report.loaded, 0);
        assert_eq!(report.required_failures.len(), 1);
        let errors = chain.init_all().await;
        assert_eq!(errors.len(), 1);
        assert!(errors[0].1.to_string().contains("export is absent"));
    }

    #[tokio::test]
    async fn configured_optional_wasm_load_failure_is_isolated() {
        let tmp = tempfile::tempdir().unwrap();
        write_wasm_extension(
            tmp.path(),
            "optional-wasm",
            &typed_hook_wat("on_shutdown", "null"),
            true,
            "fuel = 10000\nmemory_mb = 1\nhooks = [\"on_init\"]",
        );

        let mut chain = ExtensionChain::new();
        let report = load_extensions_for_startup(
            tmp.path(),
            &["optional-wasm".to_string()],
            &[],
            &mut chain,
            Vec::new(),
        );
        assert!(report.required_failures.is_empty());
        assert!(chain.init_all().await.is_empty());
        assert!(chain.is_empty());
    }

    #[test]
    fn configured_required_manifest_validation_failure_is_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let extension_dir = RokoLayout::for_project(tmp.path())
            .extensions_dir()
            .join("invalid-required");
        std::fs::create_dir_all(&extension_dir).unwrap();
        std::fs::write(
            extension_dir.join("extension.toml"),
            r#"[extension]
name = "invalid-required"
version = "1.0.0"
layer = "not-a-layer"
tier = "declarative"
optional = false
"#,
        )
        .unwrap();

        let mut chain = ExtensionChain::new();
        let report = load_extensions_for_startup(
            tmp.path(),
            &["invalid-required".to_string()],
            &[],
            &mut chain,
            Vec::new(),
        );
        assert_eq!(report.required_failures.len(), 1);
        assert_eq!(report.required_failures[0].stage, "manifest_validation");
        assert_eq!(report.required_failures[0].extension, "invalid-required");
    }

    #[test]
    fn configured_optional_manifest_validation_failure_is_isolated() {
        let tmp = tempfile::tempdir().unwrap();
        let extension_dir = RokoLayout::for_project(tmp.path())
            .extensions_dir()
            .join("invalid-optional");
        std::fs::create_dir_all(&extension_dir).unwrap();
        std::fs::write(
            extension_dir.join("extension.toml"),
            r#"[extension]
name = "invalid-optional"
version = "1.0.0"
layer = "not-a-layer"
tier = "declarative"
optional = true
"#,
        )
        .unwrap();

        let mut chain = ExtensionChain::new();
        let report = load_extensions_for_startup(
            tmp.path(),
            &["invalid-optional".to_string()],
            &[],
            &mut chain,
            Vec::new(),
        );
        assert!(report.required_failures.is_empty());
        assert!(chain.is_empty());
    }

    #[test]
    fn out_of_tree_native_manifest_is_not_registered_as_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let extension_dir = RokoLayout::for_project(tmp.path())
            .extensions_dir()
            .join("native-unavailable");
        std::fs::create_dir_all(&extension_dir).unwrap();
        std::fs::write(
            extension_dir.join("extension.toml"),
            r#"
[extension]
name = "native-unavailable"
version = "1.0.0"
layer = "action"
tier = "native_rust"
"#,
        )
        .unwrap();

        let mut chain = ExtensionChain::new();
        assert_eq!(load_extensions(tmp.path(), &[], &mut chain), 0);
        assert!(chain.is_empty());
    }
}
