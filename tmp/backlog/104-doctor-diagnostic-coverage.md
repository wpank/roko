# 104 — Doctor Command Missing Critical Diagnostic Checks

**Priority**: P2 — reliability/UX; `roko doctor` currently misses checks that would catch
common workspace corruption before it causes obscure failures during plan execution
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/` (`roko-cli`)
**Depends on**: None

---

## Background

`roko doctor` is the workspace health command (`roko doctor` in the CLI). It is the first
thing a user should run when something goes wrong, and it is also run internally before plan
execution. Currently it runs 22 diagnostic check functions (as counted in
`crates/roko-cli/src/doctor.rs`, which lists them in `run_doctor` at line 175). The checks
cover config presence, provider API keys, disk space, toolchain versions, and serve health.

However, several high-impact diagnostic categories are entirely absent:

1. **JSONL file content validity.** The canonical data files (`engrams.jsonl`,
   `episodes.jsonl`, etc.) are checked for *existence* but never validated for parseable
   content. Silent corruption (a truncated write, a null byte, a non-JSON line) cascades
   into bad learning decisions and obscure errors during plan runs.

2. **Plan manifest validity.** Invalid `tasks.toml` files — bad TOML syntax, missing
   required fields, cyclic task dependencies — are only caught when `roko plan run` starts.
   `roko doctor` could catch these proactively. The validation logic already exists in
   `TasksFile::validate()` and `TasksFile::validate_structure()` in
   `crates/roko-cli/src/task_parser.rs`.

3. **File descriptor limits.** Low `ulimit -n` values (open file limit) cause random agent
   dispatch failures and TUI crashes. The TUI's `notify::RecommendedWatcher` needs many
   inotify watchers; concurrent agent dispatch opens many sockets and pipes. A soft limit
   below 1024 is problematic. The check requires a `#[cfg(unix)]` syscall to `getrlimit`.

4. **Git repository state.** Plans create git worktrees from the current branch. An
   uninitialized git repo, detached HEAD, or corrupted `.git/` directory causes
   `WorktreeManager::create_for_attempt` to fail with a confusing error. Doctor should
   check for this proactively.

5. **State snapshot validity.** `.roko/state/state-snapshot.json` is the runner-v2 resume
   file. The `check_state_layout_audit` function at line 1257 checks for file *existence*
   alongside other layout files, but never parses the JSON. A corrupt or truncated snapshot
   causes `event_loop.rs` to fail at startup with "load authoritative unified state snapshot"
   and then fall back to a fresh run (losing resume state silently).

## Current State

### The `run_doctor` function at line 175 dispatches these checks:

1. `check_workdir` (line 422) — workspace directory exists
2. `check_config_presence` (line 452) — roko.toml present and readable
3. `check_layout_basics` (line 531) — `.roko/` layout directories exist
4. `check_claude_cli` (line 806) — claude binary in PATH
5-N. `check_configured_provider_keys` (line 841) — API keys set per provider
N+1. `check_provider_usable` (line 579) — at least one provider usable
N+2. `check_available_providers` (line 972) — providers list non-empty
N+3. `check_default_model_configured` (line 603) — default model set
N+4. `check_rust_version` (line 1065) — rustc version meets floor
N+5. `check_node_version` (line 1115) — node version if present
N+6. `check_serve_auth` (line 655) — serve auth config consistent
N+7. `check_serve_health` (line 720) — serve HTTP endpoint up (async)
N+8. `check_dead_conductor_config` (line 291) — conductor config not stale
N+9. `check_v2_abstractions` (line 1187) — v2 layout markers present
N+10 to N+13. `check_state_layout_audit` (line 1257) — .roko/ VERSION and canonical file existence (not content validity)
N+14 to N+16. `check_config_freshness` (line 228) — config freshness JSON
N+17 to N+18. `check_harness_providers` (line 1457) — harness provider config
N+19 to N+20. `check_mcp_allowlist` (line 1535) — MCP server allowlist
N+21. `check_orphaned_tmp_files` (line 1729) — stale .tmp files in .roko/learn/
N+22. `check_plans_dir_conflict` (line 2127) — plans/ at wrong location
N+23. `check_disk_health` (line 1898) — disk free space and stale targets (async)
N+24. `check_target_staleness` (line 2082) — cargo target/ staleness

### What `check_state_layout_audit` does (line 1257, but NOT what is missing)

This function at line 1257 checks the `.roko/` VERSION file and then uses the following
canonical paths array (lines 1340-1353):

```rust
let canonical_paths: &[(&str, PathBuf)] = &[
    ("episodes.jsonl", layout.root_episodes_path()),
    ("gate-verdicts.jsonl", layout.root().join("gate-verdicts.jsonl")),
    ("engrams.jsonl", layout.engrams_path()),
    ("events.jsonl", layout.events_jsonl_path()),
    ("learn/gate-thresholds.json", layout.gate_thresholds_path()),
    ("state/state-snapshot.json", layout.state_dir().join("state-snapshot.json")),
];
```

For each path it checks only `path.exists()` (line 1358). It never opens the file, reads
any lines, or parses the JSON. An absent file is reported as "normal for new workspaces"
(line 1380). The `state-snapshot.json` file is only checked for existence here.

### What `TasksFile::validate()` does (task_parser.rs line 791)

The `TasksFile` struct (defined at line 699) has a `validate()` method at line 791 that
checks tiers, verify steps, and context fields, then calls `validate_structure()` at line
957 which detects cyclic dependencies via `detect_cycle_nodes`. This is the exact logic that
`roko plan validate` calls. Doctor should call the same function on every `tasks.toml` found
in the `plans/` directory.

## Implementation Plan

All new check functions follow the existing pattern in `doctor.rs`: return `DoctorCheck`
(for single result) or `Vec<DoctorCheck>` (for multiple). The `DoctorCheck` struct (line 52)
has fields: `id: String`, `status: DoctorStatus`, `message: String`, `detail: Option<String>`,
`path: Option<String>`, `url: Option<String>`, `fix: Option<String>`.

Add all new check calls inside `run_doctor` at line 175, after the existing checks and before
the `DoctorSummary::from_checks` call at line 210.

### Check B1: JSONL content sampling

Add a new private function after the `check_state_layout_audit` function (around line 1420):

```rust
fn check_jsonl_integrity(path: &Path, label: &str) -> DoctorCheck {
    use std::io::{BufRead, BufReader};

    let id = format!("jsonl_integrity_{}", label.replace(['/', '.'], "_"));
    let path_str = path.display().to_string();

    if !path.exists() {
        return DoctorCheck {
            id,
            status: DoctorStatus::Ok,
            message: format!("{label}: not present (normal for new workspaces)"),
            detail: None,
            path: Some(path_str),
            url: None,
            fix: None,
        };
    }

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            return DoctorCheck {
                id,
                status: DoctorStatus::Fail,
                message: format!("{label}: cannot open: {e}"),
                detail: None,
                path: Some(path_str),
                url: None,
                fix: Some("Check file permissions on .roko/".to_string()),
            };
        }
    };

    let reader = BufReader::new(file);
    let mut total = 0usize;
    let mut corrupt = 0usize;
    let mut first_corrupt_line: Option<usize> = None;

    for (i, line) in reader.lines().enumerate() {
        total += 1;
        match line {
            Err(_) => {
                corrupt += 1;
                first_corrupt_line.get_or_insert(i + 1);
            }
            Ok(text) if text.trim().is_empty() => {} // allow blank lines
            Ok(text) => {
                if serde_json::from_str::<serde_json::Value>(&text).is_err() {
                    corrupt += 1;
                    first_corrupt_line.get_or_insert(i + 1);
                }
            }
        }
        // Sample up to 10_000 lines for large files.
        if total >= 10_000 {
            break;
        }
    }

    if corrupt > 0 {
        DoctorCheck {
            id,
            status: DoctorStatus::Warn,
            message: format!("{label}: {corrupt}/{total} sampled lines are invalid JSON"),
            detail: first_corrupt_line.map(|n| format!("first invalid line: {n}")),
            path: Some(path_str),
            url: None,
            fix: Some(format!(
                "Inspect {label} for corruption; consider moving it aside and running roko init"
            )),
        }
    } else {
        DoctorCheck {
            id,
            status: DoctorStatus::Ok,
            message: format!("{label}: {total} lines sampled, all valid JSON"),
            detail: None,
            path: Some(path_str),
            url: None,
            fix: None,
        }
    }
}
```

Add a caller function that applies this to all canonical JSONL files:

```rust
fn check_all_jsonl_integrity(workdir: &Path) -> Vec<DoctorCheck> {
    let layout = RokoLayout::for_project(workdir);
    if !layout.root().is_dir() {
        return vec![];
    }
    let files: &[(&str, PathBuf)] = &[
        ("engrams.jsonl", layout.engrams_path()),
        ("episodes.jsonl", layout.root_episodes_path()),
        ("gate-verdicts.jsonl", layout.root().join("gate-verdicts.jsonl")),
        ("events.jsonl", layout.events_jsonl_path()),
        ("learn/provider-outcomes.jsonl", layout.root().join("learn/provider-outcomes.jsonl")),
        ("learn/efficiency.jsonl", layout.root().join("learn/efficiency.jsonl")),
    ];
    files.iter().map(|(label, path)| check_jsonl_integrity(path, label)).collect()
}
```

In `run_doctor` (line 175), add before the `DoctorSummary::from_checks` call:
```rust
checks.extend(check_all_jsonl_integrity(&workdir));
```

### Check B2: Plan manifest validation

Add a new function to doctor.rs:

```rust
fn check_plan_manifests(workdir: &Path) -> Vec<DoctorCheck> {
    use crate::task_parser::TasksFile;

    let plans_dir = workdir.join("plans");
    if !plans_dir.is_dir() {
        return vec![DoctorCheck {
            id: "plan_manifests".to_string(),
            status: DoctorStatus::Ok,
            message: "no plans/ directory (nothing to check)".to_string(),
            detail: None,
            path: Some(plans_dir.display().to_string()),
            url: None,
            fix: None,
        }];
    }

    let mut results = Vec::new();
    let entries = match std::fs::read_dir(&plans_dir) {
        Ok(e) => e,
        Err(err) => {
            return vec![DoctorCheck {
                id: "plan_manifests".to_string(),
                status: DoctorStatus::Fail,
                message: format!("cannot read plans/ directory: {err}"),
                detail: None,
                path: Some(plans_dir.display().to_string()),
                url: None,
                fix: Some("Check permissions on plans/".to_string()),
            }];
        }
    };

    for entry in entries.flatten() {
        let tasks_path = entry.path().join("tasks.toml");
        if !tasks_path.exists() {
            continue;
        }
        let path_str = tasks_path.display().to_string();
        let id = format!(
            "plan_manifest_{}",
            entry.file_name().to_string_lossy().replace(['-', '/'], "_")
        );
        match TasksFile::parse(&tasks_path) {
            Err(e) => {
                results.push(DoctorCheck {
                    id,
                    status: DoctorStatus::Fail,
                    message: format!("tasks.toml parse error in {}", entry.file_name().to_string_lossy()),
                    detail: Some(e.to_string()),
                    path: Some(path_str),
                    url: None,
                    fix: Some(format!("roko plan validate {}", entry.path().display())),
                });
            }
            Ok(tasks_file) => {
                let issues = tasks_file.validate();
                if issues.is_empty() {
                    results.push(DoctorCheck {
                        id,
                        status: DoctorStatus::Ok,
                        message: format!(
                            "{} tasks.toml: {} tasks, valid",
                            entry.file_name().to_string_lossy(),
                            tasks_file.tasks.len()
                        ),
                        detail: None,
                        path: Some(path_str),
                        url: None,
                        fix: None,
                    });
                } else {
                    results.push(DoctorCheck {
                        id,
                        status: DoctorStatus::Warn,
                        message: format!(
                            "{} tasks.toml: {} validation issue(s)",
                            entry.file_name().to_string_lossy(),
                            issues.len()
                        ),
                        detail: Some(issues.join("; ")),
                        path: Some(path_str),
                        url: None,
                        fix: Some(format!("roko plan validate {}", entry.path().display())),
                    });
                }
            }
        }
    }
    results
}
```

Add to `run_doctor`:
```rust
checks.extend(check_plan_manifests(&workdir));
```

### Check B3: File descriptor limit (Unix only)

Add a new function:

```rust
fn check_resource_limits() -> Vec<DoctorCheck> {
    let mut results = Vec::new();

    #[cfg(unix)]
    {
        use std::mem::MaybeUninit;

        // SAFETY: getrlimit is safe to call with a valid pointer.
        let mut rlim = MaybeUninit::<libc::rlimit>::uninit();
        let rc = unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, rlim.as_mut_ptr()) };
        if rc == 0 {
            let rlim = unsafe { rlim.assume_init() };
            let soft = rlim.rlim_cur;
            let id = "fd_limit".to_string();
            if soft != libc::RLIM_INFINITY && soft < 1024 {
                results.push(DoctorCheck {
                    id,
                    status: DoctorStatus::Warn,
                    message: format!("open file limit is {soft} (recommended: ≥4096)"),
                    detail: Some(
                        "Low fd limits cause random failures in concurrent agent dispatch and TUI file watching".to_string(),
                    ),
                    path: None,
                    url: None,
                    fix: Some("ulimit -n 4096  # add to shell profile for persistence".to_string()),
                });
            } else {
                results.push(DoctorCheck {
                    id,
                    status: DoctorStatus::Ok,
                    message: format!("open file limit is {soft}"),
                    detail: None,
                    path: None,
                    url: None,
                    fix: None,
                });
            }
        }
    }

    results
}
```

This requires adding `libc` as a dependency to `roko-cli`'s `Cargo.toml`. Check if `libc`
is already a transitive dependency with `cargo tree -p roko-cli | grep libc`. If it is,
add it directly as a direct dev dependency or non-dev dependency. The `#[cfg(unix)]` guard
makes this a no-op on Windows.

Add to `run_doctor`:
```rust
checks.extend(check_resource_limits());
```

### Check B4: Git repository state

Add a new function:

```rust
fn check_git_state(workdir: &Path) -> DoctorCheck {
    let git_dir = workdir.join(".git");
    if !git_dir.exists() {
        return DoctorCheck {
            id: "git_state".to_string(),
            status: DoctorStatus::Warn,
            message: "workspace is not a git repository".to_string(),
            detail: Some("plan execution creates git worktrees; a git repo is required".to_string()),
            path: Some(workdir.display().to_string()),
            url: None,
            fix: Some("git init && git commit --allow-empty -m 'init'".to_string()),
        };
    }

    // Check for detached HEAD by reading .git/HEAD.
    let head_path = git_dir.join("HEAD");
    let head_content = match std::fs::read_to_string(&head_path) {
        Ok(c) => c,
        Err(e) => {
            return DoctorCheck {
                id: "git_state".to_string(),
                status: DoctorStatus::Fail,
                message: format!("cannot read .git/HEAD: {e}"),
                detail: Some("git repository may be corrupted".to_string()),
                path: Some(head_path.display().to_string()),
                url: None,
                fix: Some("git fsck".to_string()),
            };
        }
    };

    if head_content.trim().starts_with("ref: ") {
        // Normal branch reference.
        let branch = head_content.trim().trim_start_matches("ref: refs/heads/");
        DoctorCheck {
            id: "git_state".to_string(),
            status: DoctorStatus::Ok,
            message: format!("git HEAD is on branch: {branch}"),
            detail: None,
            path: Some(workdir.display().to_string()),
            url: None,
            fix: None,
        }
    } else {
        // Detached HEAD (contains a SHA).
        DoctorCheck {
            id: "git_state".to_string(),
            status: DoctorStatus::Warn,
            message: "git HEAD is detached".to_string(),
            detail: Some(format!("HEAD: {}", head_content.trim())),
            path: Some(workdir.display().to_string()),
            url: None,
            fix: Some("git checkout main  # or your primary branch".to_string()),
        }
    }
}
```

Add to `run_doctor`:
```rust
checks.push(check_git_state(&workdir));
```

### Check B5: State snapshot validity

Extend the existing `check_state_layout_audit` function (line 1257) or add a separate
`check_snapshot_validity` function. The separate function is cleaner:

```rust
fn check_snapshot_validity(workdir: &Path) -> DoctorCheck {
    let layout = RokoLayout::for_project(workdir);
    let snapshot_path = layout.state_dir().join("state-snapshot.json");
    let path_str = snapshot_path.display().to_string();

    if !snapshot_path.exists() {
        return DoctorCheck {
            id: "snapshot_validity".to_string(),
            status: DoctorStatus::Ok,
            message: "no state snapshot (fresh workspace or clean start)".to_string(),
            detail: None,
            path: Some(path_str),
            url: None,
            fix: None,
        };
    }

    match std::fs::read_to_string(&snapshot_path) {
        Err(e) => DoctorCheck {
            id: "snapshot_validity".to_string(),
            status: DoctorStatus::Fail,
            message: format!("cannot read state snapshot: {e}"),
            detail: None,
            path: Some(path_str),
            url: None,
            fix: Some("rm .roko/state/state-snapshot.json  # forces fresh start".to_string()),
        },
        Ok(content) if content.trim().is_empty() => DoctorCheck {
            id: "snapshot_validity".to_string(),
            status: DoctorStatus::Warn,
            message: "state snapshot is empty".to_string(),
            detail: None,
            path: Some(path_str),
            url: None,
            fix: Some("rm .roko/state/state-snapshot.json  # forces fresh start".to_string()),
        },
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(_) => DoctorCheck {
                id: "snapshot_validity".to_string(),
                status: DoctorStatus::Ok,
                message: "state snapshot is valid JSON".to_string(),
                detail: None,
                path: Some(path_str),
                url: None,
                fix: None,
            },
            Err(e) => DoctorCheck {
                id: "snapshot_validity".to_string(),
                status: DoctorStatus::Fail,
                message: format!("state snapshot is corrupt: {e}"),
                detail: Some("runner resume will fail; delete to start fresh".to_string()),
                path: Some(path_str),
                url: None,
                fix: Some("rm .roko/state/state-snapshot.json".to_string()),
            },
        },
    }
}
```

Add to `run_doctor`:
```rust
checks.push(check_snapshot_validity(&workdir));
```

## Acceptance Criteria

1. `roko doctor` samples each canonical JSONL file and reports a `[warn]` if any of the
   first 10,000 lines are not valid JSON. The check ID starts with `jsonl_integrity_`.
2. `roko doctor` parses every `plans/*/tasks.toml` and reports `[warn]` for validation
   issues (missing fields, cycles) and `[fail]` for unparseable TOML. The check includes
   a `fix` field pointing to `roko plan validate <path>`.
3. On Unix systems, `roko doctor` checks the soft file descriptor limit and warns if it is
   below 1024, with a suggested `ulimit` command.
4. `roko doctor` checks for a git repository at the workspace root and warns if `.git` is
   absent or if HEAD is detached.
5. `roko doctor` attempts to parse `state-snapshot.json` as JSON and reports `[fail]` if
   it is corrupt or unreadable, with a `fix` command to delete it.
6. All existing `cargo test -p roko-cli` tests pass.
7. New tests added: (a) corrupt JSONL produces `[warn]`; (b) detached HEAD produces `[warn]`;
   (c) corrupt snapshot produces `[fail]`; (d) invalid tasks.toml produces `[fail]`.

## Verification Checklist

- [ ] Run `roko doctor` on the workspace and confirm new check IDs appear in the output
- [ ] Create a test JSONL with one corrupt line and confirm `jsonl_integrity_*` check reports `[warn]`
- [ ] Create a tasks.toml with a circular dependency and confirm `plan_manifest_*` check reports `[warn]`
- [ ] On macOS/Linux, lower the fd limit with `ulimit -n 256` in a subshell and run `roko doctor`; confirm `[warn]` on `fd_limit`
- [ ] Run `cargo test -p roko-cli -- doctor` and confirm all doctor tests pass including the new ones
- [ ] Run `cargo clippy -p roko-cli --no-deps -- -D warnings` and confirm clean
- [ ] If `libc` was added to Cargo.toml, run `cargo build -p roko-cli` and confirm it compiles

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/doctor.rs` | Add `check_all_jsonl_integrity`, `check_plan_manifests`, `check_resource_limits`, `check_git_state`, `check_snapshot_validity` functions; call each from `run_doctor` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/Cargo.toml` | Add `libc` as a dependency (check if already transitive first) |
