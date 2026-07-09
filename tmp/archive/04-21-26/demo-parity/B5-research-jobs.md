# B5: Research job execution pipeline

## Context

**Repo:** `/Users/will/dev/nunchi/roko/roko`
**Branch:** `demo-backend`
**Language:** Rust (workspace with ~29 crates)
**Key crate paths:**
- CLI + orchestrator: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/`
- Core types: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/`
- HTTP server: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/`
- Agent dispatch: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/`

**Key files:**
- Orchestrator (20K lines): `crates/roko-cli/src/orchestrate.rs`
- CLI entry: `crates/roko-cli/src/main.rs`
- Server routes: `crates/roko-serve/src/routes/mod.rs`
- Server state: `crates/roko-serve/src/state.rs`
- Server events: `crates/roko-serve/src/events.rs`
- Server WS: `crates/roko-serve/src/routes/ws.rs`

**Architecture:**
- `roko-serve` is an axum HTTP server on port 6677 with ~85 REST routes + WebSocket
- `AppState` uses `tokio::sync::RwLock` -- all lock ops are `.read().await` / `.write().await` (NOT `.unwrap()`)
- Event bus: `state.event_bus.publish(event)` -- always present, no Option wrapping
- The TUI gets data two ways: (1) StateHub push via `watch<DashboardSnapshot>` channel, (2) file polling via `DashboardData::tick()` reading `.roko/` files

### Pre-commit (MANDATORY)

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

## What this task does

Create a `JobRunner` that polls the roko-serve API for open jobs and executes research jobs by spawning `roko research topic` as a subprocess. The runner handles the full lifecycle: claim the job (assign), start it, spawn the research subprocess with a timeout, collect output, evaluate quality, and submit results back.

## Prerequisites

B1 (job types) and B2 (job routes) must be complete and the server must accept job API requests.

## Key design decisions

- The runner polls `GET /api/jobs?state=open` every 30 seconds (configurable).
- For research jobs: spawns `cargo run -p roko-cli -- research topic "<title>"` as a subprocess. This avoids lifetime issues from calling the research function directly -- the subprocess approach is simpler and more robust.
- Subprocess execution has a hard 10-minute timeout. Jobs that exceed it are submitted as failures.
- Output quality is evaluated heuristically: word count > 100 AND at least one citation marker (`http://`, `https://`, `[1]`, `[2]`, etc.).
- Uses `sha2` for content hashing (already in workspace deps). Do NOT use `md5`.
- The runner is designed to be called from `orchestrate.rs` as a periodic side-effect, not as a standalone binary.
- The poll interval is a named constant (`POLL_INTERVAL`) and the timeout is a named constant (`SUBPROCESS_TIMEOUT`).

## Steps

- [ ] **Read existing code for subprocess patterns.** Check how `roko-cli` spawns subprocesses elsewhere:
  ```
  grep -rn "tokio::process::Command\|std::process::Command" crates/roko-cli/src/ --include='*.rs' | head -20
  ```
  Follow the same pattern.

- [ ] **Create the job_runner module.** Create `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/job_runner.rs` with the complete contents below.

- [ ] **Register the module.** Find the module declarations in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/lib.rs` or `main.rs`. Add `pub mod job_runner;` among the other module declarations.

  Search first:
  ```
  grep -n "pub mod" crates/roko-cli/src/lib.rs crates/roko-cli/src/main.rs | head -30
  ```

- [ ] **Full contents of `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/job_runner.rs`:**

```rust
//! Job runner: polls the roko-serve API for open jobs and executes them.
//!
//! The runner is intended to be called periodically from the orchestration
//! loop. Each tick checks for open jobs, claims one if available, and
//! dispatches execution based on the job type.
//!
//! # Lifecycle
//!
//! ```text
//! poll open jobs -> claim (assign) -> start -> spawn subprocess
//!                                              (with timeout)
//!                                           -> evaluate output
//!                                           -> submit results
//! ```

use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

/// Server base URL used to reach `roko serve`.
const DEFAULT_SERVER_URL: &str = "http://localhost:6677"; // MOCK: make configurable via roko.toml

/// How often to poll for new jobs.
const POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Hard timeout for any single subprocess. Jobs that exceed this are
/// submitted as failures so they do not block the runner indefinitely.
const SUBPROCESS_TIMEOUT: Duration = Duration::from_secs(600); // 10 minutes

/// Minimum word count for a research output to be considered passing.
const RESEARCH_MIN_WORDS: usize = 100;

/// Orchestrates job polling and dispatch.
pub struct JobRunner {
    server_url: String,
    client: reqwest::Client,
    last_poll: Option<Instant>,
    workdir: std::path::PathBuf,
    /// Agent identity used when claiming jobs.
    agent_id: String,
}

impl JobRunner {
    /// Create a new runner targeting the given server.
    #[must_use]
    pub fn new(workdir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            server_url: DEFAULT_SERVER_URL.to_string(),
            client: reqwest::Client::new(),
            last_poll: None,
            workdir: workdir.into(),
            agent_id: format!("runner-{}", &uuid::Uuid::new_v4().to_string()[..8]),
        }
    }

    /// Set a custom server URL.
    #[must_use]
    pub fn with_server_url(mut self, url: impl Into<String>) -> Self {
        self.server_url = url.into();
        self
    }

    /// Set the agent identity.
    #[must_use]
    pub fn with_agent_id(mut self, id: impl Into<String>) -> Self {
        self.agent_id = id.into();
        self
    }

    /// Check if enough time has passed since the last poll.
    /// Returns `true` if we should poll now.
    fn should_poll(&self) -> bool {
        match self.last_poll {
            None => true,
            Some(last) => last.elapsed() >= POLL_INTERVAL,
        }
    }

    /// Poll for open jobs and execute one if available.
    ///
    /// Call this from the orchestration loop. It short-circuits if the
    /// poll interval has not elapsed. Errors are returned to the caller;
    /// the typical usage is to log them and continue (non-fatal).
    pub async fn maybe_poll_jobs(&mut self) -> Result<()> {
        if !self.should_poll() {
            return Ok(());
        }
        self.last_poll = Some(Instant::now());

        let jobs = self.fetch_open_jobs().await.context("fetch open jobs")?;
        if jobs.is_empty() {
            debug!("no open jobs found");
            return Ok(());
        }

        // Take the first open job.
        let job = &jobs[0];
        let job_id = job.id.as_str();
        let job_type = job.job_type.as_str();

        if job_id.is_empty() {
            anyhow::bail!("job missing id field");
        }

        info!(job_id, job_type, "picked up job");

        match job_type {
            "research" => self.execute_research_job(job_id, job).await?,
            "coding_task" => {
                // B6 will implement this.
                debug!(job_id, "skipping coding_task job -- not yet implemented");
            }
            other => {
                debug!(job_id, other, "skipping unsupported job type");
            }
        }

        Ok(())
    }

    // ------------------------------------------------------------------
    // Job execution
    // ------------------------------------------------------------------

    /// Execute a research job end-to-end.
    ///
    /// Flow: assign -> start -> spawn research subprocess (with timeout)
    ///       -> evaluate output -> hash -> submit results.
    async fn execute_research_job(&self, job_id: &str, job: &JobSummary) -> Result<()> {
        let title = job.title.as_str();

        // Step 1: Assign the job to ourselves.
        info!(job_id, title, "assigning research job");
        self.post_json(
            &format!("/api/jobs/{job_id}/assign"),
            &serde_json::json!({ "agent_id": self.agent_id }),
        )
        .await
        .context("assign job")?;

        // Step 2: Start the job.
        self.post_empty(&format!("/api/jobs/{job_id}/start"))
            .await
            .context("start job")?;

        info!(job_id, title, "spawning research subprocess");

        // Step 3: Spawn the research subprocess with a hard timeout.
        let spawn_result = tokio::time::timeout(
            SUBPROCESS_TIMEOUT,
            tokio::process::Command::new("cargo")
                .args(["run", "-p", "roko-cli", "--", "research", "topic", title])
                .current_dir(&self.workdir)
                .output(),
        )
        .await;

        let (stdout, stderr, success) = match spawn_result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                let success = output.status.success();
                (stdout, stderr, success)
            }
            Ok(Err(e)) => {
                warn!(job_id, "subprocess spawn error: {e}");
                (String::new(), e.to_string(), false)
            }
            Err(_elapsed) => {
                warn!(job_id, "research subprocess timed out after {}s", SUBPROCESS_TIMEOUT.as_secs());
                (
                    String::new(),
                    format!("timed out after {}s", SUBPROCESS_TIMEOUT.as_secs()),
                    false,
                )
            }
        };

        // Step 4: Hash the output for dedup/reference.
        let mut hasher = Sha256::new();
        hasher.update(stdout.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        let short_hash = &hash[..12];

        info!(job_id, success, short_hash, "subprocess finished");

        // Step 5: Build the result summary.
        let result_summary = if success {
            let truncated = truncate_string(&stdout, 4000);
            if stdout.len() > 4000 {
                format!("{truncated}... [truncated, sha256:{short_hash}]")
            } else {
                truncated
            }
        } else {
            format!(
                "Research subprocess failed.\nstderr:\n{}",
                truncate_string(&stderr, 2000),
            )
        };

        // Step 6: Evaluate output quality.
        let quality_passed = success && evaluate_research_quality(&stdout);

        // Step 7: Collect artifacts (files written to .roko/research/ in the last 5 min).
        let artifacts = find_recent_artifacts(&self.workdir, "research");
        info!(job_id, artifact_count = artifacts.len(), "collected artifacts");

        // Step 8: Submit.
        let submission = serde_json::json!({
            "agent_id": self.agent_id,
            "result_summary": result_summary,
            "artifacts": artifacts,
            "gate_results": [
                {
                    "gate": "research_completed",
                    "passed": success,
                    "detail": if success {
                        format!("output hash: {short_hash}")
                    } else {
                        format!("subprocess failed")
                    }
                },
                {
                    "gate": "research_quality",
                    "passed": quality_passed,
                    "detail": if quality_passed {
                        format!("word count >= {RESEARCH_MIN_WORDS} and citations present")
                    } else {
                        format!("output below quality threshold (min {RESEARCH_MIN_WORDS} words + citations)")
                    }
                }
            ]
        });

        self.post_json(&format!("/api/jobs/{job_id}/submit"), &submission)
            .await
            .context("submit job")?;

        if success && quality_passed {
            info!(job_id, title, "research job completed and accepted");
        } else if success {
            warn!(job_id, title, "research job completed but failed quality gate");
        } else {
            warn!(job_id, title, "research job failed");
        }

        Ok(())
    }

    // ------------------------------------------------------------------
    // HTTP helpers
    // ------------------------------------------------------------------

    /// Fetch open jobs from the server.
    async fn fetch_open_jobs(&self) -> Result<Vec<JobSummary>> {
        let url = format!("{}/api/jobs?state=open", self.server_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("GET /api/jobs")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("fetch jobs failed: {status} {body}");
        }

        let body: JobListResponse = response.json().await.context("parse jobs response")?;
        Ok(body.jobs)
    }

    async fn post_json(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.server_url, path);
        let response = self
            .client
            .post(&url)
            .json(body)
            .send()
            .await
            .context(format!("POST {path}"))?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            anyhow::bail!("POST {path} failed: {status} {text}");
        }

        serde_json::from_str(&text).context(format!("parse response from {path}"))
    }

    async fn post_empty(&self, path: &str) -> Result<serde_json::Value> {
        self.post_json(path, &serde_json::json!({})).await
    }
}

// ---------------------------------------------------------------------------
// Response types (minimal, for deserialization)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct JobListResponse {
    #[serde(default)]
    jobs: Vec<JobSummary>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JobSummary {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub job_type: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub assigned_to: Option<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Evaluate whether research output meets minimum quality thresholds.
///
/// Checks:
/// - Word count >= `RESEARCH_MIN_WORDS`
/// - At least one citation marker (`http://`, `https://`, `[1]`, `[2]`, etc.)
fn evaluate_research_quality(output: &str) -> bool {
    if output.split_whitespace().count() < RESEARCH_MIN_WORDS {
        return false;
    }
    // Check for citation markers
    let has_url = output.contains("http://") || output.contains("https://");
    let has_footnote = output.contains("[1]") || output.contains("[2]") || output.contains("[3]");
    has_url || has_footnote
}

/// Scan `.roko/{subdir}/` for files modified in the last 5 minutes.
fn find_recent_artifacts(workdir: &Path, subdir: &str) -> Vec<String> {
    let dir = workdir.join(".roko").join(subdir);
    let cutoff = std::time::SystemTime::now()
        .checked_sub(Duration::from_secs(300))
        .expect("system time arithmetic");
    let mut paths = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(meta) = path.metadata() {
                if let Ok(modified) = meta.modified() {
                    if modified > cutoff {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            paths.push(format!(".roko/{subdir}/{name}"));
                        }
                    }
                }
            }
        }
    }

    paths
}

/// Truncate a string to at most `max_len` bytes, appending "..." if truncated.
/// Always respects UTF-8 char boundaries.
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let mut end = max_len;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_runner_new_has_defaults() {
        let runner = JobRunner::new("/tmp/test");
        assert_eq!(runner.server_url, "http://localhost:6677");
        assert!(runner.agent_id.starts_with("runner-"));
        assert!(runner.last_poll.is_none());
    }

    #[test]
    fn should_poll_returns_true_initially() {
        let runner = JobRunner::new("/tmp/test");
        assert!(runner.should_poll());
    }

    #[test]
    fn should_poll_returns_false_after_recent_poll() {
        let mut runner = JobRunner::new("/tmp/test");
        runner.last_poll = Some(Instant::now());
        assert!(!runner.should_poll());
    }

    #[test]
    fn find_recent_artifacts_returns_empty_for_missing_dir() {
        let artifacts = find_recent_artifacts(Path::new("/nonexistent"), "research");
        assert!(artifacts.is_empty());
    }

    #[test]
    fn sha256_hash_is_deterministic() {
        let mut h1 = Sha256::new();
        h1.update(b"test content");
        let d1 = format!("{:x}", h1.finalize());

        let mut h2 = Sha256::new();
        h2.update(b"test content");
        let d2 = format!("{:x}", h2.finalize());

        assert_eq!(d1, d2);
        assert_eq!(d1.len(), 64);
    }

    #[test]
    fn evaluate_research_quality_passes_with_url_and_words() {
        let long_text = "word ".repeat(150);
        let output = format!("{long_text} https://example.com/paper");
        assert!(evaluate_research_quality(&output));
    }

    #[test]
    fn evaluate_research_quality_passes_with_footnote() {
        let long_text = "word ".repeat(150);
        let output = format!("{long_text} [1] Some citation");
        assert!(evaluate_research_quality(&output));
    }

    #[test]
    fn evaluate_research_quality_fails_too_short() {
        assert!(!evaluate_research_quality("short text https://example.com"));
    }

    #[test]
    fn evaluate_research_quality_fails_no_citations() {
        let long_text = "word ".repeat(150);
        assert!(!evaluate_research_quality(&long_text));
    }

    #[test]
    fn truncate_string_under_limit_unchanged() {
        assert_eq!(truncate_string("hello", 10), "hello");
    }

    #[test]
    fn truncate_string_over_limit_appends_ellipsis() {
        let result = truncate_string("hello world", 5);
        assert!(result.ends_with("..."));
        assert!(result.len() <= 5 + 3);
    }
}
```

- [ ] **Wire into orchestrate.rs.** Open `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`. Find the main `run_all()` function. Search for the main loop:
  ```
  grep -n "loop {" crates/roko-cli/src/orchestrate.rs | head -10
  ```

  Inside the main orchestration loop, add a periodic job poll. The exact location depends on the loop structure, but the pattern is:

  At the top of the file, add:
  ```rust
  use crate::job_runner::JobRunner;
  ```

  Before the main loop starts, initialize:
  ```rust
  let mut job_runner = JobRunner::new(&workdir);  // MOCK: configure server_url from roko.toml
  ```

  Inside the loop body, after other periodic work, add:
  ```rust
  // Poll for and execute jobs from the server (fire-and-forget -- log errors but don't crash).
  if let Err(e) = job_runner.maybe_poll_jobs().await {
      tracing::debug!("job poll error (non-fatal): {e:#}");
  }
  ```

- [ ] **Ensure dependencies are available.** Check that `roko-cli` Cargo.toml includes:
  - `reqwest` with `json` feature (for HTTP client)
  - `sha2` (for hashing)
  - `uuid` with `v4` feature (for agent ID)
  - `tokio` with `process` and `time` features (for subprocess spawning + timeout)

  Search:
  ```
  grep -n "reqwest\|sha2\|uuid\|tokio" crates/roko-cli/Cargo.toml | head -10
  ```

  Add any missing deps.

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Compile
cargo check -p roko-cli 2>&1 | head -30

# Run job_runner unit tests
cargo test -p roko-cli -- job_runner --nocapture

# Clippy
cargo clippy -p roko-cli --no-deps -- -D warnings 2>&1 | head -20

# Format check
cargo +nightly fmt --all -- --check

# Integration test (requires B1+B2 to be complete):
# Terminal 1: cargo run -p roko-cli -- serve
# Terminal 2:
#   curl -s -X POST http://localhost:6677/api/jobs \
#     -H 'Content-Type: application/json' \
#     -d '{"title":"Research Uniswap v4 hooks","description":"Survey hook patterns","job_type":"research"}' | jq .
#
# Wait 30s for the runner to pick it up (or set POLL_INTERVAL to a shorter
# value during testing). The job state should transition open -> assigned ->
# in_progress -> submitted.
```

Expected: module compiles clean, unit tests pass, research jobs execute via subprocess and are submitted with quality gate results.
