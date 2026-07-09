# B6: Coding job execution pipeline

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

Extend the `JobRunner` from B5 to handle `coding_task` jobs. Coding jobs write a temporary PRD, generate a plan from it, execute the plan with a timeout, parse gate results from the subprocess output using regex, clean up the temp PRD, and submit the results back to the server.

**Audit update (2026-04-22):** the current serve-owned job runner transitions coding jobs end-to-end, but `execute_coding_job` still delegates to `runtime.run_once` and does not yet collect plan artifacts or gate results into the submitted payload.

- [ ] Replace the current coding-job summary-only execution path with the full PRD/plan/run/gate/artifact collection flow described here, or update this PRD to accept the serve-owned runtime path with equivalent artifact/gate proof.

## Prerequisites

B1 (job types), B2 (job routes), and B5 (research jobs / JobRunner) must be complete.

## Steps

- [ ] **Read the existing job_runner.rs.** Open `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/job_runner.rs` (created in B5) and understand the structure before adding to it.

- [ ] **Add the `coding_task` match arm in `maybe_poll_jobs()`.** In the existing `match job_type` block, replace the placeholder:

  Find:
  ```rust
            "coding_task" => {
                // B6 will implement this.
                debug!(job_id, "skipping coding_task job -- not yet implemented");
            }
  ```

  Replace with:
  ```rust
            "coding_task" => self.execute_coding_job(job_id, job).await?,
  ```

- [ ] **Add `execute_coding_job()` to `JobRunner`.** Inside the `impl JobRunner` block, after `execute_research_job()`, add this method:

```rust
    /// Execute a coding job end-to-end.
    ///
    /// Flow:
    /// 1. Assign + start the job.
    /// 2. Write a temporary PRD from the job description.
    /// 3. Generate a plan from the PRD: `roko prd plan <slug>`.
    /// 4. Execute the plan with a hard timeout: `roko plan run <plan_dir>`.
    /// 5. Parse gate results from subprocess output.
    /// 6. Clean up the temp PRD.
    /// 7. Submit results back.
    async fn execute_coding_job(&self, job_id: &str, job: &JobSummary) -> Result<()> {
        let title = &job.title;
        let description = job.description.as_deref().unwrap_or(title.as_str());

        // Step 1: Assign and start.
        info!(job_id, title, "assigning coding job");
        self.post_json(
            &format!("/api/jobs/{job_id}/assign"),
            &serde_json::json!({ "agent_id": self.agent_id }),
        )
        .await
        .context("assign coding job")?;

        self.post_empty(&format!("/api/jobs/{job_id}/start"))
            .await
            .context("start coding job")?;

        info!(job_id, title, "starting coding job pipeline");

        // Step 2: Write a temporary PRD.
        // Slug: strip "job-" prefix if present, normalise to snake_case-ish.
        let raw_id = job_id.strip_prefix("job-").unwrap_or(job_id);
        let slug = format!("job-{raw_id}");
        let prd_dir = self.workdir.join(".roko").join("prd").join("drafts");
        tokio::fs::create_dir_all(&prd_dir)
            .await
            .context("create prd drafts dir")?;

        let prd_path = prd_dir.join(format!("{slug}.md"));
        let prd_content = format!(
            "---\ntitle: {title}\nstatus: draft\nsource: job-runner\njob_id: {job_id}\n---\n\n\
             # {title}\n\n\
             ## Description\n\n{description}\n\n\
             ## Acceptance criteria\n\n\
             - [ ] Implementation compiles without errors\n\
             - [ ] All existing tests pass\n\
             - [ ] No new clippy warnings\n"
        );
        tokio::fs::write(&prd_path, &prd_content)
            .await
            .context("write temp PRD")?;

        info!(job_id, %slug, "wrote temporary PRD");

        // Step 3: Generate plan from the PRD.
        let plan_result = tokio::time::timeout(
            SUBPROCESS_TIMEOUT,
            tokio::process::Command::new("cargo")
                .args(["run", "-p", "roko-cli", "--", "prd", "plan", &slug])
                .current_dir(&self.workdir)
                .output(),
        )
        .await;

        let plan_output = match plan_result {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => {
                self.submit_failure(job_id, "plan_generation", &format!("spawn error: {e}"))
                    .await?;
                cleanup_prd(&prd_path).await;
                return Ok(());
            }
            Err(_) => {
                self.submit_failure(
                    job_id,
                    "plan_generation",
                    &format!("plan generation timed out after {}s", SUBPROCESS_TIMEOUT.as_secs()),
                )
                .await?;
                cleanup_prd(&prd_path).await;
                return Ok(());
            }
        };

        if !plan_output.status.success() {
            let stderr = String::from_utf8_lossy(&plan_output.stderr);
            warn!(job_id, "plan generation failed");
            self.submit_failure(
                job_id,
                "plan_generation",
                &format!("Plan generation failed:\n{}", truncate_string(&stderr, 2000)),
            )
            .await?;
            cleanup_prd(&prd_path).await;
            return Ok(());
        }

        // Step 4: Find the generated plan directory and execute it.
        let plans_dir = self.workdir.join(".roko").join("plans");
        let plan_dir = find_plan_dir_for_slug(&plans_dir, &slug).unwrap_or_else(|| plans_dir.clone());

        info!(job_id, plan_dir = %plan_dir.display(), "executing plan");

        let exec_result = tokio::time::timeout(
            SUBPROCESS_TIMEOUT,
            tokio::process::Command::new("cargo")
                .args([
                    "run",
                    "-p",
                    "roko-cli",
                    "--",
                    "plan",
                    "run",
                    &plan_dir.to_string_lossy(),
                ])
                .current_dir(&self.workdir)
                .output(),
        )
        .await;

        let (exec_stdout, exec_stderr, plan_success) = match exec_result {
            Ok(Ok(out)) => {
                let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
                let ok = out.status.success();
                (stdout, stderr, ok)
            }
            Ok(Err(e)) => (String::new(), e.to_string(), false),
            Err(_) => (
                String::new(),
                format!("plan execution timed out after {}s", SUBPROCESS_TIMEOUT.as_secs()),
                false,
            ),
        };

        // Step 5: Parse gate results from output.
        let gate_results = parse_gate_results(&exec_stdout, &exec_stderr);
        let all_passed = gate_results.iter().all(|g| g.passed);

        // Step 6: Clean up the temp PRD.
        cleanup_prd(&prd_path).await;

        // Step 7: Build result summary.
        let mut summary = format!("## Coding job: {title}\n\n");
        summary.push_str(&format!(
            "Plan execution: {}\nGate results: {}\n\n",
            if plan_success { "success" } else { "failed" },
            if all_passed { "all passed" } else { "some failed" }
        ));
        for gr in &gate_results {
            summary.push_str(&format!(
                "- {}: {}\n",
                gr.gate,
                if gr.passed { "PASS" } else { "FAIL" }
            ));
            if !gr.detail.is_empty() {
                summary.push_str(&format!("  {}\n", truncate_string(&gr.detail, 200)));
            }
        }
        if !exec_stdout.is_empty() {
            let tail = tail_lines(&exec_stdout, 20);
            summary.push_str(&format!("\n### stdout (last 20 lines)\n```\n{tail}\n```\n"));
        }

        // Collect artifacts.
        let mut artifacts = find_recent_artifacts(&self.workdir, "plans");
        artifacts.extend(find_recent_artifacts(&self.workdir, "prd/drafts"));

        // Step 8: Submit.
        let submission = serde_json::json!({
            "agent_id": self.agent_id,
            "result_summary": truncate_string(&summary, 8000),
            "artifacts": artifacts,
            "gate_results": gate_results.iter().map(|g| serde_json::json!({
                "gate": g.gate,
                "passed": g.passed,
                "detail": truncate_string(&g.detail, 500),
            })).collect::<Vec<_>>(),
        });

        self.post_json(&format!("/api/jobs/{job_id}/submit"), &submission)
            .await
            .context("submit coding job")?;

        if plan_success && all_passed {
            info!(job_id, "coding job completed successfully");
        } else {
            warn!(job_id, plan_success, all_passed, "coding job completed with failures");
        }

        Ok(())
    }

    /// Submit a failure result for a job that could not complete a pipeline stage.
    async fn submit_failure(&self, job_id: &str, gate_name: &str, detail: &str) -> Result<()> {
        let submission = serde_json::json!({
            "agent_id": self.agent_id,
            "result_summary": format!("Job failed at stage: {gate_name}"),
            "artifacts": [],
            "gate_results": [{
                "gate": gate_name,
                "passed": false,
                "detail": truncate_string(detail, 2000),
            }],
        });

        self.post_json(&format!("/api/jobs/{job_id}/submit"), &submission)
            .await
            .context("submit failure")?;
        Ok(())
    }
```

- [ ] **Add helper functions.** After the `find_recent_artifacts` function (from B5), add:

```rust
/// Parse gate results from subprocess stdout/stderr using pattern matching.
///
/// Detects:
/// - Compile gate: `error[E` or `could not compile` in stderr.
/// - Test gate: `test result: ok` / `test result: FAILED` in stdout/stderr.
/// - Clippy gate: `warning:` count in stderr (subtracting the "N warnings emitted" summary line).
fn parse_gate_results(stdout: &str, stderr: &str) -> Vec<GateResult> {
    let mut results = Vec::new();

    // Compile gate.
    let has_compile_error = stderr.contains("error[E") || stderr.contains("could not compile");
    results.push(GateResult {
        gate: "compile".to_string(),
        passed: !has_compile_error,
        detail: if has_compile_error {
            extract_first_error(stderr).unwrap_or_else(|| "compilation errors found".to_string())
        } else {
            "compilation succeeded".to_string()
        },
    });

    // Test gate: scan both stdout and stderr since `cargo test` mixes them.
    let combined = format!("{stdout}\n{stderr}");
    let test_ok = combined.contains("test result: ok");
    let test_failed = combined.contains("test result: FAILED");
    if test_ok || test_failed {
        results.push(GateResult {
            gate: "test".to_string(),
            passed: test_ok && !test_failed,
            detail: extract_test_summary(&combined).unwrap_or_else(|| {
                if test_ok {
                    "all tests passed".to_string()
                } else {
                    "test failures detected".to_string()
                }
            }),
        });
    }

    // Clippy gate: count `warning:` occurrences in stderr, subtract the
    // trailing "N warning(s) emitted" summary line to avoid double-counting.
    let warning_lines: usize = stderr
        .lines()
        .filter(|line| line.trim_start().starts_with("warning:"))
        .count();
    // The summary line looks like "warning: 3 warnings emitted" or
    // "warning: `crate-name` (lib) generated 3 warnings". Subtract if present.
    let summary_lines: usize = stderr
        .lines()
        .filter(|line| {
            let l = line.trim_start();
            l.starts_with("warning:") && (l.contains("warnings emitted") || l.contains("generated"))
        })
        .count();
    let actual_warnings = warning_lines.saturating_sub(summary_lines);
    results.push(GateResult {
        gate: "clippy".to_string(),
        passed: actual_warnings == 0,
        detail: format!("{actual_warnings} warning(s)"),
    });

    results
}

struct GateResult {
    gate: String,
    passed: bool,
    detail: String,
}

/// Extract the first `error[Exxxx]` line from stderr.
fn extract_first_error(stderr: &str) -> Option<String> {
    stderr
        .lines()
        .find(|line| line.contains("error[E"))
        .map(|line| line.trim().to_string())
}

/// Extract the `test result: ...` summary line from output.
fn extract_test_summary(output: &str) -> Option<String> {
    output
        .lines()
        .find(|line| line.trim_start().starts_with("test result:"))
        .map(|line| line.trim().to_string())
}

/// Get the last `n` lines of a string.
fn tail_lines(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

/// Find a plan directory or file matching a slug under `plans_dir`.
fn find_plan_dir_for_slug(plans_dir: &Path, slug: &str) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(plans_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str())?;
        if path.is_dir() && name.contains(slug) {
            return Some(path);
        }
        // Also accept plan files named after the slug -- return the parent dir.
        if name.starts_with(slug) && (name.ends_with(".toml") || name.ends_with(".json")) {
            return path.parent().map(|p| p.to_path_buf());
        }
    }
    None
}

/// Remove a temporary PRD file, logging but not propagating errors.
async fn cleanup_prd(path: &std::path::Path) {
    if let Err(e) = tokio::fs::remove_file(path).await {
        tracing::debug!(path = %path.display(), "failed to remove temp PRD (non-fatal): {e}");
    }
}
```

- [ ] **Add tests for the new helpers.** In the `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn parse_gate_results_detects_compile_error() {
        let stderr = "error[E0308]: mismatched types\n  --> src/main.rs:10:5";
        let results = parse_gate_results("", stderr);
        let compile = results.iter().find(|r| r.gate == "compile").unwrap();
        assert!(!compile.passed, "compile gate should fail");
        assert!(compile.detail.contains("E0308"), "detail should include error code");
    }

    #[test]
    fn parse_gate_results_detects_test_success() {
        let stdout = "running 5 tests\ntest result: ok. 5 passed; 0 failed; 0 ignored";
        let results = parse_gate_results(stdout, "");
        let compile = results.iter().find(|r| r.gate == "compile").unwrap();
        assert!(compile.passed, "compile should pass with no error lines");
        let test = results.iter().find(|r| r.gate == "test").unwrap();
        assert!(test.passed, "test gate should pass");
        assert!(test.detail.contains("5 passed"), "detail should include pass count");
    }

    #[test]
    fn parse_gate_results_detects_test_failure() {
        let stdout = "test result: FAILED. 3 passed; 2 failed; 0 ignored";
        let results = parse_gate_results(stdout, "");
        let test = results.iter().find(|r| r.gate == "test").unwrap();
        assert!(!test.passed, "test gate should fail");
    }

    #[test]
    fn parse_gate_results_counts_clippy_warnings_correctly() {
        // 2 real warnings + 1 summary line = should report 2.
        let stderr = "warning: unused variable `x`\n\
                      warning: dead_code\n\
                      warning: `my_crate` (lib) generated 2 warnings";
        let results = parse_gate_results("", stderr);
        let clippy = results.iter().find(|r| r.gate == "clippy").unwrap();
        assert!(!clippy.passed, "clippy gate should fail with 2 warnings");
        assert!(clippy.detail.contains("2 warning"), "detail should say 2 warnings");
    }

    #[test]
    fn parse_gate_results_clean_clippy_passes() {
        let results = parse_gate_results("", "");
        let clippy = results.iter().find(|r| r.gate == "clippy").unwrap();
        assert!(clippy.passed, "clippy gate should pass with no warnings");
    }

    #[test]
    fn tail_lines_returns_last_n() {
        let text = "a\nb\nc\nd\ne";
        assert_eq!(tail_lines(text, 3), "c\nd\ne");
        assert_eq!(tail_lines(text, 10), "a\nb\nc\nd\ne");
        assert_eq!(tail_lines(text, 0), "");
    }

    #[test]
    fn truncate_string_under_limit_unchanged() {
        assert_eq!(truncate_string("hello", 10), "hello");
    }

    #[test]
    fn truncate_string_over_limit_appends_ellipsis() {
        let result = truncate_string("hello world", 5);
        assert_eq!(result, "hello...");
    }

    #[test]
    fn find_plan_dir_for_slug_returns_none_for_missing_dir() {
        let result = find_plan_dir_for_slug(Path::new("/nonexistent"), "my-job");
        assert!(result.is_none());
    }
```

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Compile
cargo check -p roko-cli 2>&1 | head -30

# Run all job_runner tests (including new gate result parsing tests)
cargo test -p roko-cli -- job_runner --nocapture

# Clippy
cargo clippy -p roko-cli --no-deps -- -D warnings 2>&1 | head -20

# Format check
cargo +nightly fmt --all -- --check
```

Expected: all unit tests pass, including the five gate result parsing tests. The `coding_task` branch in `maybe_poll_jobs()` now routes to `execute_coding_job()` instead of logging a skip.
