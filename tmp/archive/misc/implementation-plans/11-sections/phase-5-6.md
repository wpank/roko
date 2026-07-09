<!-- Master Plan: Tier 4, Sections 4A-4D -->
<!-- Status: Not started -->
<!-- Depends on: Tier 2C, Tier 3B -->

# Phase 5: Daemon Mode & Deployment

> **Master Plan Reference**: Tier 4, Sections 4A-4D
> **Status**: Not started
> **Depends on**: Tier 2C, Tier 3B
> **Blocks**: Production deployment
>
> ### What Already Exists in Codebase
> - `crates/roko-cli/src/daemon.rs` — daemon scaffold (DaemonState, DaemonInfo, socket paths)
> - `crates/roko-cli/src/serve/deploy/` — Railway deployment scaffold
> - `crates/roko-cli/src/serve/deploy/railway_api.rs` — Railway API client
> - `crates/roko-cli/src/serve/deploy/railway_cli.rs` — Railway CLI wrapper
>
> ### Reference Material
> - Mori daemon: not applicable (new feature)
> - Railway docs: external

---

## 5.1 Wire daemon to full serve + dispatch

**File:** `crates/roko-cli/src/daemon.rs` (extend existing scaffold)

The daemon scaffold has `DaemonState`, `DaemonInfo`, socket paths. Wire it to the real runtime:

### Startup sequence

```rust
pub async fn daemon_start(foreground: bool, port: u16) -> Result<()> {
    // 1. Check if already running
    if let Some(info) = read_daemon_info()? {
        if is_process_alive(info.pid) {
            anyhow::bail!("daemon already running (pid {})", info.pid);
        }
    }

    // 2. Fork to background (unless --foreground)
    if !foreground {
        let exe = std::env::current_exe()?;
        let child = std::process::Command::new(exe)
            .args(["daemon", "start", "--foreground", "--port", &port.to_string()])
            .stdout(std::fs::File::create(log_path("daemon.log"))?)
            .stderr(std::fs::File::create(log_path("daemon.err"))?)
            .spawn()?;
        println!("daemon started (pid {})", child.id());
        return Ok(());
    }

    // 3. Write PID file
    let pid = std::process::id();
    write_pidfile(pid)?;

    // 4. Load config
    let workdir = std::env::current_dir()?;
    let config = roko_core::config::load_config(&workdir)?;

    // 5. Build AppState (loads subscriptions, templates from all repos)
    let state = Arc::new(AppState::new(workdir.clone(), config.clone()).await?);

    // 6. Start HTTP server (for webhook reception)
    let http_handle = tokio::spawn({
        let state = Arc::clone(&state);
        async move {
            roko_serve::run_server_with_state(state, "0.0.0.0", port).await
        }
    });

    // 7. Start cron scheduler
    let scheduler = roko_serve::scheduler::start_scheduler(Arc::clone(&state)).await?;

    // 8. Start file watchers
    let watchers = roko_serve::fswatcher::start_watchers(Arc::clone(&state)).await?;

    // 9. Start dispatch loop
    let dispatch_handle = roko_serve::dispatch::start_dispatch_loop(Arc::clone(&state));

    // 10. Start feedback collection loop
    let feedback_handle = roko_serve::feedback::start_feedback_loop(Arc::clone(&state));

    // 11. Start Unix socket for IPC
    let ipc_handle = start_ipc_server(Arc::clone(&state)).await?;

    // 12. Write daemon info
    let info = DaemonInfo {
        session_id: uuid::Uuid::new_v4().to_string(),
        state: DaemonState::Running,
        socket_path: socket_path().to_string_lossy().to_string(),
        signals_processed: 0,
        pid,
    };
    write_daemon_info(&info)?;

    tracing::info!(pid = pid, port = port, "daemon running");

    // 13. Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    tracing::info!("shutdown signal received");

    // 14. Graceful shutdown
    state.shutdown().await;
    scheduler.shutdown().await?;
    remove_pidfile()?;
    write_daemon_info(&DaemonInfo { state: DaemonState::Stopped, ..info })?;

    Ok(())
}
```

### IPC server (Unix socket)

```rust
async fn start_ipc_server(state: Arc<AppState>) -> Result<tokio::task::JoinHandle<()>> {
    let socket = socket_path();
    if socket.exists() {
        std::fs::remove_file(&socket)?;
    }

    let listener = tokio::net::UnixListener::bind(&socket)?;

    Ok(tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("ipc accept error: {e}");
                    continue;
                }
            };

            let state = Arc::clone(&state);
            tokio::spawn(async move {
                handle_ipc_command(stream, &state).await;
            });
        }
    }))
}

async fn handle_ipc_command(mut stream: tokio::net::UnixStream, state: &AppState) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await.unwrap_or(0);
    let cmd = String::from_utf8_lossy(&buf[..n]);

    let response = match cmd.trim() {
        "status" => {
            let agents = state.supervisor.list().await;
            let subs = state.subscriptions.read().await;
            serde_json::json!({
                "state": "running",
                "pid": std::process::id(),
                "active_agents": agents.len(),
                "subscriptions": subs.len(),
                "uptime_secs": state.started_at.elapsed().as_secs(),
            }).to_string()
        }
        "reload" => {
            // Re-scan subscriptions and templates
            let new_subs = load_subscriptions(&state.config, &state.workdir).await;
            *state.subscriptions.write().await = new_subs;
            let count = state.subscriptions.read().await.len();
            format!("{{\"reloaded\": true, \"subscriptions\": {count}}}")
        }
        "stop" => {
            state.shutdown().await;
            "{\"stopping\": true}".into()
        }
        _ => "{\"error\": \"unknown command\"}".into(),
    };

    let _ = stream.write_all(response.as_bytes()).await;
}
```

### CLI commands

Add to `main.rs`:

```rust
Daemon(DaemonCmd),

#[derive(Subcommand)]
enum DaemonCmd {
    /// Start the daemon (background unless --foreground)
    Start {
        #[arg(long)]
        foreground: bool,
        #[arg(long, default_value = "9090")]
        port: u16,
    },
    /// Stop the running daemon
    Stop,
    /// Show daemon status
    Status,
    /// Follow daemon logs
    Logs {
        #[arg(long, short = 'f')]
        follow: bool,
        #[arg(long, short = 'n', default_value = "50")]
        lines: usize,
    },
    /// Reload subscriptions and templates without restart
    Reload,
    /// Stop and restart the daemon
    Restart {
        #[arg(long, default_value = "9090")]
        port: u16,
    },
    /// Install as a system service (launchd on macOS)
    Install,
    /// Uninstall system service
    Uninstall,
}
```

### Checklist — 5.1

- [ ] Implement `daemon_start()` with full startup sequence
- [ ] Implement daemon fork + pidfile for background mode
- [ ] Implement Unix socket IPC server
- [ ] Implement `status`, `reload`, `stop` IPC commands
- [ ] Add `daemon` subcommand with all variants to `main.rs`
- [ ] Implement `daemon stop` (sends stop via IPC or SIGTERM to pid)
- [ ] Implement `daemon status` (queries IPC or reads daemon info)
- [ ] Implement `daemon logs` (tails log file)
- [ ] Implement `daemon reload` (sends reload via IPC)
- [ ] Implement `daemon restart` (stop + start)
- [ ] **Verify:** `roko daemon start` — starts in background, pidfile written
- [ ] **Verify:** `roko daemon status` — shows running state, agent count, subscription count
- [ ] **Verify:** `roko daemon reload` — re-scans subscriptions without restart
- [ ] **Verify:** `roko daemon stop` — graceful shutdown, agents drained
- [ ] **Verify:** `roko daemon logs --follow` — streams daemon log output
- [ ] **Verify:** Send webhook while daemon is running → agent spawns

---

## 5.2 Launchd plist generation (macOS)

**File:** `crates/roko-cli/src/daemon/launchd.rs`

```rust
//! Generate and manage launchd plist for macOS.

use std::path::{Path, PathBuf};
use anyhow::Result;

const LABEL: &str = "dev.nunchi.roko";

fn plist_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join("Library/LaunchAgents").join(format!("{LABEL}.plist"))
}

pub fn generate_plist(port: u16) -> String {
    let exe = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("/usr/local/bin/roko"));
    let home = std::env::var("HOME").unwrap_or_default();

    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
        <string>daemon</string>
        <string>start</string>
        <string>--foreground</string>
        <string>--port</string>
        <string>{port}</string>
    </array>
    <key>KeepAlive</key>
    <true/>
    <key>RunAtLoad</key>
    <true/>
    <key>WorkingDirectory</key>
    <string>{home}</string>
    <key>StandardOutPath</key>
    <string>{home}/.roko/logs/daemon.log</string>
    <key>StandardErrorPath</key>
    <string>{home}/.roko/logs/daemon.err</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:{home}/.cargo/bin</string>
    </dict>
</dict>
</plist>"#, exe = exe.display())
}

pub fn install(port: u16) -> Result<()> {
    let plist = generate_plist(port);
    let path = plist_path();

    // Create logs directory
    let home = std::env::var("HOME")?;
    std::fs::create_dir_all(format!("{home}/.roko/logs"))?;

    // Write plist
    std::fs::write(&path, plist)?;
    println!("wrote {}", path.display());

    // Load with launchctl
    let status = std::process::Command::new("launchctl")
        .args(["load", "-w"])
        .arg(&path)
        .status()?;

    if status.success() {
        println!("loaded into launchd — roko daemon will start automatically");
    } else {
        anyhow::bail!("launchctl load failed");
    }
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let path = plist_path();
    if !path.exists() {
        anyhow::bail!("plist not found at {}", path.display());
    }

    let _ = std::process::Command::new("launchctl")
        .args(["unload"])
        .arg(&path)
        .status();

    std::fs::remove_file(&path)?;
    println!("uninstalled — roko daemon will no longer auto-start");
    Ok(())
}
```

### Checklist — 5.2

- [ ] Create `daemon/launchd.rs`
- [ ] Implement plist generation with correct paths
- [ ] Implement `install()` — write plist + launchctl load
- [ ] Implement `uninstall()` — launchctl unload + remove plist
- [ ] Wire into `roko daemon install` / `roko daemon uninstall`
- [ ] **Verify:** `roko daemon install` — plist written, `launchctl list | grep roko` shows it
- [ ] **Verify:** `roko daemon status` — shows running after install
- [ ] **Verify:** `roko daemon uninstall` — plist removed, daemon stops
- [ ] **Verify:** After reboot, daemon auto-starts (launchd KeepAlive)

---

## 5.3 Cloud deployment with webhook ingress

### Architecture

The existing `roko serve deploy railway` infrastructure deploys the HTTP server to Railway. For webhook ingress:

1. **After deploy:** `DeploymentStatus::Ready { url }` returns the public URL
2. **Auto-register webhooks:** Use GitHub API to register the webhook URL

### GitHub webhook auto-registration

```rust
async fn register_github_webhook(
    github: &octocrab::Octocrab,
    owner: &str,
    repo: &str,
    webhook_url: &str,
    secret: &str,
) -> Result<()> {
    let hook = github
        .repos(owner, repo)
        .create_hook(
            "web",
            serde_json::json!({
                "url": format!("{webhook_url}/api/hooks/github"),
                "content_type": "json",
                "secret": secret,
            }),
            vec![
                "push", "pull_request", "issues", "issue_comment",
                "pull_request_review", "check_run",
            ],
        )
        .await?;

    tracing::info!(hook_id = hook.id, "registered GitHub webhook for {owner}/{repo}");
    Ok(())
}
```

### Deploy config additions

```toml
# roko.toml
[serve.deploy]
provider = "railway"
environment = [
    "GITHUB_TOKEN",
    "GITHUB_WEBHOOK_SECRET",
    "SLACK_BOT_TOKEN",
    "SLACK_SIGNING_SECRET",
]

# Auto-register webhooks after deploy
[[serve.deploy.webhooks]]
provider = "github"
owner = "nunchi"
repo = "collaboration"

[[serve.deploy.webhooks]]
provider = "github"
owner = "nunchi"
repo = "knowledge-base"

[[serve.deploy.webhooks]]
provider = "github"
owner = "nunchi"
repo = "roko"
```

### Checklist — 5.3

- [ ] Implement `register_github_webhook()` function
- [ ] Add webhook registration to post-deploy flow
- [ ] Add `[serve.deploy]` config section to schema
- [ ] **Verify:** `roko serve deploy railway` — server deploys, URL returned
- [ ] **Verify:** Webhook registered on configured repos (check GitHub Settings > Webhooks)
- [ ] **Verify:** Push to repo → webhook hits deployed server → agent spawns

---

## 5.4 Remote orchestrator agents

> The key new concept: roko running in the cloud, autonomously picking up plans and implementing them.

### Architecture

```
┌─────────────────────────────────────────────┐
│            Cloud (Railway/Fly.io)             │
│                                              │
│  roko serve (daemon mode)                    │
│    ├── Webhook endpoints (receives events)   │
│    ├── Subscription dispatch                 │
│    ├── code-implementer-agent (runs here)    │
│    │   ├── Clones repo to /tmp/workspace     │
│    │   ├── Runs PlanRunner on tasks          │
│    │   ├── Gates: compile, test, clippy      │
│    │   ├── Pushes to feature branch          │
│    │   └── Creates PR via github MCP         │
│    └── Feedback collector                    │
└─────────────────────────────────────────────┘
           │                    ▲
           │ Webhook events     │ GitHub API
           ▼                    │
┌─────────────────────────────────────────────┐
│              GitHub                           │
│  PRDs merged → webhook → auto-plan agent     │
│  Plans merged → webhook → code-implementer   │
│  PR reviews → webhook → review-response       │
└─────────────────────────────────────────────┘
```

### Cloud PlanRunner modifications

The existing `PlanRunner` in `orchestrate.rs` assumes a local workspace with git worktrees. For cloud execution:

```rust
pub struct CloudExecutionConfig {
    /// Clone the repo to this directory before execution.
    pub workspace_dir: PathBuf,   // default: /tmp/roko-workspace
    /// GitHub token for cloning and pushing.
    pub github_token: String,
    /// Maximum parallel tasks.
    pub max_parallel: usize,      // default: 2
    /// Cost budget per plan execution (in cents).
    pub cost_budget_cents: u64,   // default: 5000 ($50)
    /// Maximum wall-clock time per plan.
    pub timeout_secs: u64,        // default: 3600 (1 hour)
}
```

### Execution flow

1. **Trigger:** `prd.plan_approved` signal (plan PR merged)
2. **Clone:** `git clone --depth 1 https://github.com/{owner}/{repo}.git /tmp/roko-workspace/{repo}`
3. **Branch:** `git checkout -b impl/{plan-slug}`
4. **Execute:** Run `PlanRunner` with the tasks from the plan
5. **Gate:** After each task, run gates (compile, test, clippy)
6. **Auto-fix:** If gate fails, `gate-fixer-agent` attempts repair (up to 3x)
7. **Push:** `git push origin impl/{plan-slug}`
8. **PR:** Use `github.create_pr` to open the implementation PR
9. **Cleanup:** Remove the workspace directory

### Git helper implementations

These async helpers back the execution flow above. Each wraps `tokio::process::Command`
calls to git and handles auth by embedding the GitHub token directly in the HTTPS URL.

```rust
use std::path::Path;
use anyhow::{Context, Result, bail};
use tokio::process::Command;

/// Clone a repository with token authentication.
///
/// Rewrites the clone URL to `https://x-access-token:{token}@github.com/{owner}/{repo}.git`
/// so no credential helper or SSH key is required in ephemeral cloud environments.
/// Uses `--depth 1` to minimise bandwidth and disk usage.
async fn git_clone(url: &str, workspace: &Path, token: &str) -> Result<()> {
    // Rewrite https://github.com/owner/repo → https://x-access-token:TOKEN@github.com/owner/repo
    let authed_url = if let Some(rest) = url.strip_prefix("https://github.com/") {
        format!("https://x-access-token:{token}@github.com/{rest}")
    } else if let Some(rest) = url.strip_prefix("https://") {
        // Generic HTTPS host — embed token the same way
        format!("https://x-access-token:{token}@{rest}")
    } else {
        bail!("unsupported URL scheme (expected https://): {url}");
    };

    // Ensure the parent directory exists
    if let Some(parent) = workspace.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("creating workspace parent directory")?;
    }

    let output = Command::new("git")
        .args(["clone", "--depth", "1", &authed_url])
        .arg(workspace)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .await
        .context("spawning git clone")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Scrub the token from any error output before surfacing
        let safe_stderr = stderr.replace(token, "***");
        bail!("git clone failed: {safe_stderr}");
    }

    tracing::info!(workspace = %workspace.display(), "cloned repository");
    Ok(())
}

/// Create and switch to a new branch in the workspace.
async fn git_checkout_new_branch(workspace: &Path, branch: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["checkout", "-b", branch])
        .current_dir(workspace)
        .output()
        .await
        .context("spawning git checkout -b")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git checkout -b {branch} failed: {stderr}");
    }

    tracing::info!(branch, workspace = %workspace.display(), "created branch");
    Ok(())
}

/// Stage all changes and create a commit.
///
/// Uses `git add -A` to pick up new files, modifications, and deletions,
/// then commits with the supplied message. Returns an error if the commit
/// produces no changes (empty diff).
async fn git_commit(workspace: &Path, message: &str) -> Result<()> {
    // Stage everything
    let add_output = Command::new("git")
        .args(["add", "-A"])
        .current_dir(workspace)
        .output()
        .await
        .context("spawning git add -A")?;

    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        bail!("git add -A failed: {stderr}");
    }

    // Check whether there is anything to commit
    let diff_output = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(workspace)
        .output()
        .await
        .context("spawning git diff --cached")?;

    if diff_output.status.success() {
        // Exit code 0 means the index matches HEAD — nothing to commit
        bail!("nothing to commit (working tree clean)");
    }

    // Commit
    let commit_output = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(workspace)
        .env("GIT_AUTHOR_NAME", "roko")
        .env("GIT_AUTHOR_EMAIL", "roko@nunchi.dev")
        .env("GIT_COMMITTER_NAME", "roko")
        .env("GIT_COMMITTER_EMAIL", "roko@nunchi.dev")
        .output()
        .await
        .context("spawning git commit")?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        bail!("git commit failed: {stderr}");
    }

    tracing::info!(workspace = %workspace.display(), "committed: {message}");
    Ok(())
}

/// Push the current branch to the remote with token authentication.
///
/// Rewrites the push URL to embed the token, matching the approach used by
/// `git_clone`. This avoids mutating the repo's remote config.
async fn git_push(workspace: &Path, branch: &str, token: &str) -> Result<()> {
    // Read the current origin URL so we can rewrite it for auth
    let url_output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(workspace)
        .output()
        .await
        .context("reading origin URL")?;

    if !url_output.status.success() {
        bail!("failed to read git remote origin URL");
    }

    let origin_url = String::from_utf8_lossy(&url_output.stdout).trim().to_string();

    let authed_url = if let Some(rest) = origin_url.strip_prefix("https://github.com/") {
        format!("https://x-access-token:{token}@github.com/{rest}")
    } else if origin_url.contains("x-access-token") {
        // Already has a token embedded (from our clone step) — reuse as-is
        origin_url.clone()
    } else if let Some(rest) = origin_url.strip_prefix("https://") {
        format!("https://x-access-token:{token}@{rest}")
    } else {
        bail!("unsupported remote URL scheme: {origin_url}");
    };

    let output = Command::new("git")
        .args(["push", &authed_url, &format!("HEAD:refs/heads/{branch}")])
        .current_dir(workspace)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .await
        .context("spawning git push")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let safe_stderr = stderr.replace(token, "***");
        bail!("git push failed: {safe_stderr}");
    }

    tracing::info!(branch, workspace = %workspace.display(), "pushed to remote");
    Ok(())
}

/// Remove the workspace directory and all its contents.
///
/// Equivalent to `rm -rf workspace`. Silently succeeds if the directory
/// does not exist (idempotent cleanup).
async fn git_cleanup(workspace: &Path) -> Result<()> {
    match tokio::fs::remove_dir_all(workspace).await {
        Ok(()) => {
            tracing::info!(workspace = %workspace.display(), "cleaned up workspace");
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Already gone — not an error
            Ok(())
        }
        Err(e) => Err(e).context(format!("removing workspace {}", workspace.display())),
    }
}
```

#### Usage in `implement_plan`

These helpers compose into the execution flow from the previous section:

```rust
async fn implement_plan(
    plan: &Plan,
    config: &CloudExecutionConfig,
) -> Result<()> {
    let repo_url = &plan.repo_url;   // e.g. "https://github.com/nunchi/roko"
    let branch = format!("impl/{}", plan.slug);
    let workspace = config.workspace_dir.join(&plan.slug);

    // Steps 2-3: Clone and branch
    git_clone(repo_url, &workspace, &config.github_token).await?;
    git_checkout_new_branch(&workspace, &branch).await?;

    // Steps 4-6: Execute tasks with gates (existing PlanRunner logic)
    let runner = PlanRunner::new_cloud(&workspace, config)?;
    runner.run_all(&plan.tasks).await?;

    // Step 7: Commit and push
    git_commit(&workspace, &format!("impl({}): execute plan tasks", plan.slug)).await?;
    git_push(&workspace, &branch, &config.github_token).await?;

    // Step 8: Create PR (via GitHub MCP tool or octocrab)
    // ... (handled by the agent template's PR-creation step)

    // Step 9: Cleanup
    git_cleanup(&workspace).await?;

    Ok(())
}
```

### Resource management

```toml
[serve.cloud]
max_concurrent_plans = 2
workspace_dir = "/tmp/roko-workspace"
cost_budget_cents = 5000
plan_timeout_secs = 3600
cleanup_after_secs = 300
```

### Checklist — 5.4

- [ ] Add `CloudExecutionConfig` to config schema
- [ ] Implement repo cloning to workspace directory
- [ ] Adapt PlanRunner for non-worktree mode (direct branch in cloned repo)
- [ ] Implement push-to-branch after task completion
- [ ] Implement PR creation after all tasks complete
- [ ] Add cost budget tracking (abort if exceeded)
- [ ] Add timeout handling (abort if exceeded)
- [ ] Add workspace cleanup after execution
- [ ] **Verify:** Merge a test plan PR → code-implementer clones, implements, pushes PR
- [ ] **Verify:** Cost budget abort works (set low budget, verify abort)
- [ ] **Verify:** Timeout abort works (set low timeout, verify abort)
- [ ] **Verify:** Workspace cleaned up after execution

---

# Phase 6: Multi-Repo Subscription Configuration

---

## 6.1 Subscription config format

Full schema for `.roko/subscriptions.toml`:

```toml
# Each [[subscription]] entry maps a signal kind pattern to an agent template.

[[subscription]]
# REQUIRED: Signal kind pattern (supports prefix matching)
pattern = "webhook.github.pull_request"

# REQUIRED: Agent template name (looked up in .roko/templates/)
agent_template = "pr-review-agent"

# OPTIONAL: JSON filter on signal body. All keys must match.
# Array values mean "any of" (OR).
filter = { action = ["opened", "synchronize"] }

# OPTIONAL: Glob on changed file paths (GitHub push events only).
path_filter = "docs/**/*.md"

# OPTIONAL: Working directory for the agent.
# Defaults to the repo where this subscriptions.toml lives.
repo_context = "/path/to/repo"

# OPTIONAL: Cron schedule (only for pattern = "scheduler.cron").
schedule = "0 9 * * MON"

# OPTIONAL: Directory to watch (only for pattern = "watcher.fs_change").
watch_path = "/path/to/watch"
watch_glob = "**/*.md"

# OPTIONAL: Max concurrent agent instances (default: 3).
max_concurrent = 2

# OPTIONAL: Min seconds between triggers (default: none).
cooldown_secs = 300

# OPTIONAL: Enable/disable (default: true).
enabled = true
```

---

## 6.2 Multi-repo loading

### `roko.toml` serve section

```toml
[serve]
port = 9090
bind = "0.0.0.0"

# Additional repos to load subscriptions and templates from.
# Each repo should have a .roko/ directory.
repos = [
    "/Users/will/dev/nunchi/collaboration",
    "/Users/will/dev/nunchi/knowledge-base",
]
```

### Loading algorithm

On `AppState::new()`:

```rust
async fn load_all(config: &RokoConfig, workdir: &Path) -> (Vec<Subscription>, TemplateRegistry) {
    let mut all_subs = Vec::new();
    let mut all_templates = TemplateRegistry::new();

    // 1. Load from local .roko/
    load_repo(workdir, &mut all_subs, &mut all_templates, None).await;

    // 2. Load from each configured repo
    for repo_path in &config.serve.repos {
        load_repo(repo_path, &mut all_subs, &mut all_templates, Some(repo_path)).await;
    }

    (all_subs, all_templates)
}

async fn load_repo(
    repo: &Path,
    subs: &mut Vec<Subscription>,
    templates: &mut TemplateRegistry,
    repo_context: Option<&Path>,
) {
    // Subscriptions
    let sub_path = repo.join(".roko/subscriptions.toml");
    if let Ok(content) = tokio::fs::read_to_string(&sub_path).await {
        if let Ok(file) = toml::from_str::<SubscriptionsFile>(&content) {
            for mut sub in file.subscription {
                if sub.repo_context.is_none() {
                    sub.repo_context = repo_context.map(|p| p.to_path_buf());
                }
                subs.push(sub);
            }
        }
    }

    // Templates
    let template_dir = repo.join(".roko/templates");
    if template_dir.is_dir() {
        if let Err(e) = templates.scan_directory(&template_dir) {
            tracing::warn!(repo = %repo.display(), error = %e, "failed to load templates");
        }
    }
}
```

### Template namespacing

Templates from different repos may have the same name. Resolution:
1. If unique: use directly by name
2. If conflict: prefix with repo basename (`collaboration:digest-agent`)
3. Subscriptions can use either form

---

## 6.3 Per-repo `.roko/` initialization

### `roko init --repo <path>`

Creates the `.roko/` directory structure in a target repo:

```
.roko/
├── roko.toml             # Repo-specific config
├── subscriptions.toml    # Event → agent mappings
├── templates/            # Agent template directory
├── scripts.toml          # Script wrapper config (if scripts/ exists)
└── mcp-servers.toml      # MCP server registry
```

### Collaboration repo `.roko/roko.toml`

```toml
schema_version = 2

[project]
name = "nunchi-collaboration"
root = "/Users/will/dev/nunchi/collaboration"

[agent]
default_model = "claude-haiku-4-5-20251001"
default_backend = "claude_cli"
```

### Knowledge-base repo `.roko/roko.toml`

```toml
schema_version = 2

[project]
name = "nunchi-knowledge-base"
root = "/Users/will/dev/nunchi/knowledge-base"

[agent]
default_model = "claude-haiku-4-5-20251001"
default_backend = "claude_cli"
```

### Checklist — 6.3

- [ ] Implement `roko init --repo <path>` command
- [ ] Auto-detect `scripts/` directory → generate `scripts.toml`
- [ ] Create `.roko/roko.toml` with repo-specific defaults
- [ ] Create empty `subscriptions.toml` and `templates/` directory
- [ ] **Verify:** `roko init --repo /Users/will/dev/nunchi/collaboration` creates structure
- [ ] **Verify:** `roko init --repo /Users/will/dev/nunchi/knowledge-base` creates structure
- [ ] **Verify:** `roko serve` loads subscriptions from both repos

---

## 6.4 Secret management

### Environment variable interpolation

All config files support `${VAR_NAME}` syntax:

```toml
[serve.webhooks]
github_secret = "${GITHUB_WEBHOOK_SECRET}"
```

Resolved at load time via `std::env::var()`.

### `.env` file loading

On startup, `roko serve` and `roko daemon start` load `.env` from:
1. Current directory (`.env`)
2. `~/.roko/.env` (global)

Using the `dotenvy` crate:
```toml
dotenvy = "0.15"
```

```rust
fn load_dotenv() {
    // Local .env
    let _ = dotenvy::dotenv();
    // Global .env
    if let Ok(home) = std::env::var("HOME") {
        let global = PathBuf::from(home).join(".roko/.env");
        let _ = dotenvy::from_path(&global);
    }
}
```

### Secret namespaces

Secrets are organized by service:

```env
# GitHub
GITHUB_TOKEN=ghp_...
GITHUB_WEBHOOK_SECRET=whsec_...

# Slack
SLACK_BOT_TOKEN=xoxb-...
SLACK_SIGNING_SECRET=...

# External services
LINEAR_API_KEY=lin_api_...
NOTION_TOKEN=ntn_...
FIREFLIES_API_KEY=...
ANTHROPIC_API_KEY=sk-ant-...
```

### How secrets flow

1. **Config files** reference secrets via `${VAR}` → resolved from env
2. **MCP servers** get secrets via `env` field in `mcp-servers.toml`
3. **Scripts** get secrets via `env` field in `scripts.toml`
4. **Agents** inherit the MCP server's env (no direct access to secrets)

### Checklist — 6.4

- [ ] Add `dotenvy` dependency
- [ ] Implement `.env` loading on startup (local + global)
- [ ] Implement `${VAR}` interpolation in config parser
- [ ] Document required env vars per integration
- [ ] **Verify:** Set env var, verify it resolves in webhook config
- [ ] **Verify:** `.env` file loaded on `roko serve` startup
- [ ] **Verify:** MCP servers receive configured env vars
