//! Tests below each spin up a throwaway git repo in a
//! [`tempfile::TempDir`]. Every test that needs `git` first checks
//! the binary via [`git_available`]; if git isn't on `$PATH`, the
//! test returns early so `cargo test` still succeeds on machines
//! without git (for instance minimal CI images).

#![allow(clippy::unwrap_used)]

use super::creation_journal::{
    CreationMarker, CreationPhase, creation_record_name, ensure_cleanup_safe, unlink_claim_file,
};
use super::git_ops::{
    RetainedOwnership, isolate_worktree_config, read_gitdir, validate_id,
    worktree_list_contains_path,
};
use super::{
    AttemptAcceptance, CREATION_MARKER_DIR, CREATION_MARKER_SCHEMA, REPOSITORY_MUTATION_LOCK,
    RUNTIME_SHUTDOWN_WAIT, TestClaimMutationBarrier, TestClaimMutationPoint, TestPhaseBarrier,
    WorktreeConfig, WorktreeError, WorktreeHealth, WorktreeManager, clear_stale_index_lock,
    format_attempt_branch_name, format_attempt_worktree_id, format_branch_name,
    validate_workspace_file_kinds_with,
};
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::time::Duration;
use tempfile::TempDir;

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn workspace_walk_never_traverses_directory_replaced_by_external_symlink() {
    use std::os::unix::fs::symlink;

    let workspace = TempDir::new().unwrap();
    let external = TempDir::new().unwrap();
    std::fs::create_dir(workspace.path().join("queued")).unwrap();
    assert!(
        StdCommand::new("mkfifo")
            .arg(external.path().join("outside.fifo"))
            .status()
            .unwrap()
            .success()
    );
    let mut replaced = false;
    let result = validate_workspace_file_kinds_with(workspace.path(), &[], 8, |relative| {
        if relative == Path::new("queued") && !replaced {
            replaced = true;
            std::fs::rename(
                workspace.path().join("queued"),
                workspace.path().join("original"),
            )
            .unwrap();
            symlink(external.path(), workspace.path().join("queued")).unwrap();
        }
    });
    assert!(replaced);
    assert!(
        !result
            .as_ref()
            .is_err_and(|error| error.to_string().contains("non-file input")),
        "walker followed replacement symlink to external FIFO: {result:?}"
    );
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn workspace_walk_rejects_deleted_and_recreated_directory() {
    let workspace = TempDir::new().unwrap();
    std::fs::create_dir(workspace.path().join("queued")).unwrap();
    let mut replaced = false;
    let error = validate_workspace_file_kinds_with(workspace.path(), &[], 8, |relative| {
        if relative == Path::new("queued") && !replaced {
            replaced = true;
            std::fs::remove_dir(workspace.path().join("queued")).unwrap();
            std::fs::create_dir(workspace.path().join("queued")).unwrap();
        }
    })
    .unwrap_err();
    assert!(replaced);
    assert!(error.to_string().contains("directory changed"), "{error}");
}

fn git_available() -> bool {
    StdCommand::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn init_repo(dir: &Path) {
    // Keep the default branch name deterministic across git
    // versions and user configs.
    let status = StdCommand::new("git")
        .current_dir(dir)
        .args(["init", "-b", "main"])
        .status()
        .unwrap();
    assert!(status.success(), "git init failed");

    // Scoped identity so the test's commit is reproducible without
    // touching the user's global git config.
    for (k, v) in [
        ("user.email", "roko@example.test"),
        ("user.name", "Roko Test"),
        ("commit.gpgsign", "false"),
    ] {
        let ok = StdCommand::new("git")
            .current_dir(dir)
            .args(["config", k, v])
            .status()
            .unwrap()
            .success();
        assert!(ok, "git config {k} failed");
    }

    let ok = StdCommand::new("git")
        .current_dir(dir)
        .args(["commit", "--allow-empty", "-m", "init"])
        .status()
        .unwrap()
        .success();
    assert!(ok, "git commit --allow-empty failed");
}

fn make_manager() -> Option<(TempDir, WorktreeManager)> {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return None;
    }
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path().join("repo");
    std::fs::create_dir_all(&repo_root).unwrap();
    init_repo(&repo_root);
    let worktrees_root = tmp.path().join("worktrees");
    let mgr = WorktreeManager::new(WorktreeConfig {
        repo_root,
        base_branch: "main".to_string(),
        worktrees_root,
        max_live: None,
        idle_ttl: Duration::from_secs(3600),
    });
    Some((tmp, mgr))
}

fn make_manager_with_budget(max_live: usize) -> Option<(TempDir, WorktreeManager)> {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return None;
    }
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path().join("repo");
    std::fs::create_dir_all(&repo_root).unwrap();
    init_repo(&repo_root);
    let worktrees_root = tmp.path().join("worktrees");
    let mgr = WorktreeManager::new(WorktreeConfig {
        repo_root,
        base_branch: "main".to_string(),
        worktrees_root,
        max_live: Some(max_live),
        idle_ttl: Duration::from_secs(3600),
    });
    Some((tmp, mgr))
}

fn manager_with_worktrees_root(
    manager: &WorktreeManager,
    worktrees_root: PathBuf,
) -> WorktreeManager {
    WorktreeManager::new(WorktreeConfig {
        repo_root: manager.config.repo_root.clone(),
        base_branch: manager.config.base_branch.clone(),
        worktrees_root,
        max_live: manager.config.max_live,
        idle_ttl: manager.config.idle_ttl,
    })
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn repository_lock_path(manager: &WorktreeManager) -> PathBuf {
    let common = String::from_utf8(
        StdCommand::new("git")
            .current_dir(&manager.config.repo_root)
            .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    std::fs::canonicalize(common.trim())
        .unwrap()
        .join(REPOSITORY_MUTATION_LOCK)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn test_creation_marker(manager: &WorktreeManager, id: &str) -> CreationMarker {
    let target_oid = String::from_utf8(
        StdCommand::new("git")
            .current_dir(&manager.config.repo_root)
            .args(["rev-parse", "main"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    let common_git_dir = std::fs::canonicalize(manager.config.repo_root.join(".git")).unwrap();
    CreationMarker {
        schema_version: CREATION_MARKER_SCHEMA,
        claim_id: uuid::Uuid::new_v4().simple().to_string(),
        id: id.to_string(),
        repo_root: manager.config.repo_root.clone(),
        common_git_dir: common_git_dir.clone(),
        branch: format!("feature/{id}"),
        branch_old_oid: None,
        target_oid,
        path: manager.path_for(id),
        admin_dir: common_git_dir.join(format!("worktrees/roko-{id}")),
        phase: CreationPhase::Prepared,
        previous_digest: None,
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
struct GitProcessBarrier {
    started: PathBuf,
    release: PathBuf,
    invocations: PathBuf,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn install_git_process_barrier(manager: &WorktreeManager, tempdir: &TempDir) -> GitProcessBarrier {
    use std::os::unix::fs::PermissionsExt;

    let script = tempdir.path().join("blocking-git.sh");
    let started = tempdir.path().join("git-started");
    let release = tempdir.path().join("git-release");
    let consumed = tempdir.path().join("git-barrier-consumed");
    let invocations = tempdir.path().join("git-invocations");
    let body = format!(
        "#!/bin/sh\n\
         set -eu\n\
         printf '%s\\n' \"$$\" >> '{}'\n\
         if [ ! -e '{}' ]; then\n\
           : > '{}'\n\
           : > '{}'\n\
           while [ ! -e '{}' ]; do :; done\n\
         fi\n\
         exec git \"$@\"\n",
        invocations.display(),
        consumed.display(),
        consumed.display(),
        started.display(),
        release.display(),
    );
    std::fs::write(&script, body).expect("write blocking git wrapper");
    let mut permissions = std::fs::metadata(&script)
        .expect("wrapper metadata")
        .permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&script, permissions).expect("make wrapper executable");
    manager.set_test_git_binary(script);
    GitProcessBarrier {
        started,
        release,
        invocations,
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
async fn wait_for_barrier(path: &Path) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("git process reached barrier");
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn invocation_pids(path: &Path) -> Vec<u32> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|line| line.parse().expect("wrapper pid"))
        .collect()
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn assert_processes_exited(pids: &[u32]) {
    for pid in pids {
        let status = StdCommand::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("probe wrapper pid");
        assert!(!status.success(), "git wrapper process {pid} leaked");
    }
}

fn assert_no_git_locks(root: &Path) {
    fn visit(path: &Path, locks: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit(&path, locks);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "lock")
                && path
                    .file_name()
                    .is_none_or(|name| name != std::ffi::OsStr::new(REPOSITORY_MUTATION_LOCK))
            {
                locks.push(path);
            }
        }
    }

    let mut locks = Vec::new();
    visit(&root.join(".git"), &mut locks);
    assert!(locks.is_empty(), "git lock files leaked: {locks:?}");
}

// ── Existing tests (§15.1–§15.2, §15.8) ──

#[tokio::test]
async fn create_worktree_materialises_directory() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    let handle = mgr.create("01-alpha", "feature/alpha").await.unwrap();
    assert_eq!(handle.id, "01-alpha");
    assert_eq!(handle.branch, "feature/alpha");
    assert!(handle.path.exists(), "worktree dir should exist");
    assert!(handle.path.join(".git").exists(), ".git file expected");
    assert!(handle.created_at_ms > 0);
}

#[tokio::test]
async fn list_returns_every_active_handle_sorted() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    mgr.create("02-bravo", "feature/bravo").await.unwrap();
    mgr.create("01-alpha", "feature/alpha").await.unwrap();
    let listed = mgr.list().unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, "01-alpha");
    assert_eq!(listed[1].id, "02-bravo");
}

#[tokio::test]
async fn remove_drops_handle_and_worktree() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    let handle = mgr.create("03-charlie", "feature/charlie").await.unwrap();
    assert!(handle.path.exists());
    mgr.remove("03-charlie").await.unwrap();
    assert!(mgr.list().unwrap().is_empty());
    assert!(
        !handle.path.exists(),
        "git worktree remove --force should delete the dir"
    );
}

#[tokio::test]
async fn create_remove_roundtrip_allows_reuse() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    mgr.create("04-delta", "feature/delta").await.unwrap();
    mgr.remove("04-delta").await.unwrap();
    // After removal we must be able to use the id again.
    let h2 = mgr.create("04-delta", "feature/delta").await.unwrap();
    assert_eq!(h2.id, "04-delta");
    assert_eq!(mgr.list().unwrap().len(), 1);
}

#[tokio::test]
async fn remove_nonexistent_is_not_found() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    let err = mgr.remove("nope").await.unwrap_err();
    assert!(matches!(err, WorktreeError::NotFound(ref id) if id == "nope"));
}

#[tokio::test]
async fn duplicate_id_is_rejected() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    mgr.create("05-echo", "feature/echo").await.unwrap();
    let err = mgr.create("05-echo", "feature/echo-2").await.unwrap_err();
    assert!(matches!(err, WorktreeError::AlreadyExists(ref id) if id == "05-echo"));
    // Original handle is still there, in registry and on disk.
    assert_eq!(mgr.list().unwrap().len(), 1);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_create_retains_ownership_through_registry_commit() {
    let Some((tmp, manager)) = make_manager_with_budget(1) else {
        return;
    };
    let barrier = install_git_process_barrier(&manager, &tmp);
    let cancelled_manager = manager.clone();
    let caller = tokio::spawn(async move {
        cancelled_manager
            .create("cancel-create", "feature/cancel-create")
            .await
    });
    wait_for_barrier(&barrier.started).await;
    assert_eq!(invocation_pids(&barrier.invocations).len(), 1);

    caller.abort();
    assert!(caller.await.expect_err("caller cancelled").is_cancelled());

    let contender_manager = manager.clone();
    let contender = tokio::spawn(async move {
        contender_manager
            .create("after-cancel", "feature/after-cancel")
            .await
    });
    tokio::time::sleep(Duration::from_millis(75)).await;
    assert!(
        !contender.is_finished(),
        "another clone entered while cancelled create still owned Git"
    );
    assert_eq!(
        invocation_pids(&barrier.invocations).len(),
        1,
        "a second Git mutation overlapped the blocked child"
    );

    std::fs::write(&barrier.release, "release").expect("release Git child");
    let error = tokio::time::timeout(Duration::from_secs(5), contender)
        .await
        .expect("contender completed after reconciliation")
        .expect("contender task")
        .expect_err("completed cancelled create consumes the only slot");
    assert!(matches!(error, WorktreeError::BudgetExhausted { max: 1 }));
    assert_eq!(manager.active_count(), 1);
    assert!(manager.path_for("cancel-create").exists());
    assert!(!manager.path_for("after-cancel").exists());
    assert!(manager.operations.try_lock().is_ok());
    let pids = invocation_pids(&barrier.invocations);
    assert_processes_exited(&pids);
    assert_no_git_locks(&manager.config.repo_root);

    manager
        .remove("cancel-create")
        .await
        .expect("free capacity");
    let retry = manager
        .create("after-cancel", "feature/after-cancel")
        .await
        .expect("capacity restored after reconciled removal");
    assert!(retry.path.exists());
    assert_eq!(manager.active_count(), 1);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_remove_retains_handle_until_git_then_allows_ensure() {
    let Some((tmp, manager)) = make_manager_with_budget(1) else {
        return;
    };
    manager
        .create_for_plan("cancel-remove")
        .await
        .expect("seed worktree");
    let barrier = install_git_process_barrier(&manager, &tmp);
    let cancelled_manager = manager.clone();
    let caller = tokio::spawn(async move { cancelled_manager.remove("cancel-remove").await });
    wait_for_barrier(&barrier.started).await;
    assert_eq!(manager.active_count(), 1, "remove hid active handle early");

    caller.abort();
    assert!(caller.await.expect_err("caller cancelled").is_cancelled());
    let ensure_manager = manager.clone();
    let ensure = tokio::spawn(async move { ensure_manager.ensure_for_plan("cancel-remove").await });
    tokio::time::sleep(Duration::from_millis(75)).await;
    assert!(
        !ensure.is_finished(),
        "ensure entered while cancelled remove still owned Git"
    );
    assert_eq!(manager.active_count(), 1, "in-flight handle was lost");
    assert_eq!(
        invocation_pids(&barrier.invocations).len(),
        1,
        "a second Git mutation overlapped the blocked remove"
    );

    std::fs::write(&barrier.release, "release").expect("release Git child");
    let ensured = tokio::time::timeout(Duration::from_secs(5), ensure)
        .await
        .expect("ensure completed after removal reconciliation")
        .expect("ensure task")
        .expect("ensure recreated removed worktree");
    assert_eq!(ensured.id, "cancel-remove");
    assert!(ensured.path.exists());
    assert_eq!(manager.active_count(), 1);
    assert!(manager.operations.try_lock().is_ok());
    let pids = invocation_pids(&barrier.invocations);
    assert!(
        pids.len() >= 3,
        "remove and recreated checkout must run Git"
    );
    assert_processes_exited(&pids);
    assert_no_git_locks(&manager.config.repo_root);
}

#[test]
fn runtime_shutdown_during_linked_phase_rolls_back_before_releasing_owner() {
    let Some((tmp, manager)) = make_manager_with_budget(1) else {
        return;
    };
    let started = tmp.path().join("linked-started");
    let release = tmp.path().join("linked-release");
    manager.set_test_phase_barrier(TestPhaseBarrier {
        phase: CreationPhase::LinkedNoCheckout,
        started: started.clone(),
        release,
    });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build caller runtime");
    let creating_manager = manager.clone();
    runtime.block_on(async {
        tokio::spawn(async move {
            let _ = creating_manager
                .create_for_plan("runtime-drop-create")
                .await;
        });
        wait_for_barrier(&started).await;
    });

    let shutdown_started = std::time::Instant::now();
    drop(runtime);
    assert!(
        shutdown_started.elapsed() < RUNTIME_SHUTDOWN_WAIT,
        "caller runtime shutdown exceeded its bounded ownership wait"
    );
    assert!(manager.operations.try_lock().is_ok());
    assert!(manager.get("runtime-drop-create").is_none());
    assert!(!manager.path_for("runtime-drop-create").exists());
    assert!(!manager.creation_marker_path("runtime-drop-create").exists());
    assert!(!manager.creation_claim_path("runtime-drop-create").exists());
    let listed = StdCommand::new("git")
        .current_dir(&manager.config.repo_root)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .expect("list worktrees");
    assert!(!worktree_list_contains_path(
        &listed.stdout,
        &manager.path_for("runtime-drop-create")
    ));
    assert_no_git_locks(&manager.config.repo_root);
}

#[test]
fn runtime_shutdown_after_reset_commits_registry_and_removes_marker() {
    let Some((tmp, manager)) = make_manager_with_budget(1) else {
        return;
    };
    let started = tmp.path().join("reset-started");
    manager.set_test_phase_barrier(TestPhaseBarrier {
        phase: CreationPhase::ResetComplete,
        started: started.clone(),
        release: tmp.path().join("reset-release"),
    });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build caller runtime");
    let creating_manager = manager.clone();
    runtime.block_on(async {
        tokio::spawn(async move {
            let _ = creating_manager.create_for_plan("reset-complete").await;
        });
        wait_for_barrier(&started).await;
    });
    drop(runtime);

    assert!(manager.operations.try_lock().is_ok());
    let handle = manager
        .get("reset-complete")
        .expect("completed reset reconciled into registry");
    assert!(handle.path.exists());
    assert!(!manager.creation_marker_path("reset-complete").exists());
    assert!(!manager.creation_claim_path("reset-complete").exists());
    assert_eq!(manager.active_count(), 1);
}

#[test]
fn unproved_create_cleanup_permanently_withholds_mutation_owner() {
    let Some((tmp, manager)) = make_manager_with_budget(1) else {
        return;
    };
    let started = tmp.path().join("cleanup-started");
    manager.set_test_phase_barrier(TestPhaseBarrier {
        phase: CreationPhase::LinkedNoCheckout,
        started: started.clone(),
        release: tmp.path().join("cleanup-release"),
    });
    manager.set_test_cleanup_failure();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build caller runtime");
    let creating_manager = manager.clone();
    runtime.block_on(async {
        tokio::spawn(async move {
            let _ = creating_manager.create_for_plan("cleanup-unproved").await;
        });
        wait_for_barrier(&started).await;
    });
    drop(runtime);

    // The worker hands the reservation back, retaining the repository's
    // ownership in it (bug-53475e).
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let state = loop {
        if let Ok(state) = manager.operations.try_lock() {
            break state;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the worker never handed its reservation back"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(state.retained.is_some(), "{:?}", state.retained);
    drop(state);
    assert!(manager.path_for("cleanup-unproved").exists());
    assert!(manager.creation_claim_path("cleanup-unproved").exists());
    assert_eq!(manager.active_count(), 0);
}

/// bug-109b5a: before dispatch, a stale `index.lock` in the git directory a
/// `.git` file's `gitdir:` points at is removed, from the checkout or any
/// directory below it; a fresh one, which a live git process may hold, is
/// kept, as is one younger than the caller's threshold. A plain `.git`
/// directory works the same way.
#[test]
fn stale_index_lock_is_cleared_before_dispatch_with_gitdir_indirection() {
    let tmp = TempDir::new().unwrap();
    let checkout = tmp.path().join("checkout");
    let git_dir = tmp.path().join("repo.git/worktrees/checkout");
    std::fs::create_dir_all(checkout.join("src")).unwrap();
    std::fs::create_dir_all(&git_dir).unwrap();
    std::fs::write(
        checkout.join(".git"),
        "gitdir: ../repo.git/worktrees/checkout\n",
    )
    .unwrap();
    let place_lock = |lock: &Path, age: Duration| {
        std::fs::File::create(lock)
            .unwrap()
            .set_modified(std::time::SystemTime::now() - age)
            .unwrap();
    };
    let lock = git_dir.join("index.lock");
    let worktree_threshold = Duration::from_secs(60);

    place_lock(&lock, Duration::from_secs(5));
    assert_eq!(clear_stale_index_lock(&checkout, worktree_threshold), None);
    assert_eq!(clear_stale_index_lock(&checkout, Duration::ZERO), None);
    assert!(
        lock.exists(),
        "a fresh lock may belong to a live git process"
    );

    place_lock(&lock, Duration::from_secs(120));
    assert_eq!(
        clear_stale_index_lock(&checkout, Duration::from_secs(600)),
        None
    );
    assert!(
        lock.exists(),
        "younger than the shared checkout's threshold"
    );
    let removed = clear_stale_index_lock(&checkout.join("src"), worktree_threshold);
    assert!(
        removed.is_some(),
        "the stale lock behind gitdir: is removed"
    );
    assert!(!lock.exists());
    assert_eq!(clear_stale_index_lock(&checkout, worktree_threshold), None);

    let plain = tmp.path().join("plain");
    std::fs::create_dir_all(plain.join(".git")).unwrap();
    let plain_lock = plain.join(".git").join("index.lock");
    place_lock(&plain_lock, Duration::from_secs(120));
    assert_eq!(
        clear_stale_index_lock(&plain, worktree_threshold),
        Some(plain_lock.clone())
    );
    assert!(!plain_lock.exists());
}

/// bug-53475e: while ownership retained by an unproved cleanup names a
/// live git process, or none, the next mutation is refused at once with
/// what to do, instead of waiting forever; once the named process is gone,
/// the next mutation releases the ownership and runs.
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn unproved_cleanup_fails_fast_instead_of_blocking_later_operations() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build caller runtime");
    let retain = |pids: Vec<u32>| {
        let mut state = manager.operations.try_lock().expect("reservation free");
        state.retained = Some(RetainedOwnership {
            pids,
            since: std::time::SystemTime::now(),
            repository_lock: None,
        });
    };
    let prune = || {
        runtime
            .block_on(async {
                tokio::time::timeout(Duration::from_secs(10), manager.prune()).await
            })
            .expect("the mutation answers at once")
    };

    retain(Vec::new());
    let started = std::time::Instant::now();
    let error = prune().expect_err("no named process can be proved gone");
    assert!(
        matches!(error, WorktreeError::OwnershipRetained { .. }),
        "{error}"
    );
    assert!(error.to_string().contains("restart roko"), "{error}");
    assert!(started.elapsed() < Duration::from_secs(5));

    let mut stray = StdCommand::new("sleep")
        .arg("30")
        .spawn()
        .expect("spawn sleep");
    retain(vec![stray.id()]);
    let error = prune().expect_err("the stray git process still runs");
    assert!(
        error.to_string().contains(&stray.id().to_string()),
        "{error}"
    );
    stray.kill().expect("kill the stray process");
    stray.wait().expect("reap the stray process");

    prune().expect("the mutation runs once the process is gone");
    let state = manager.operations.try_lock().expect("reservation free");
    assert!(state.retained.is_none(), "{:?}", state.retained);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn mutation_wrapper_cannot_fork_an_escaped_process() {
    use std::os::unix::fs::PermissionsExt;

    let Some((tmp, manager)) = make_manager() else {
        return;
    };
    let escaped = tmp.path().join("escaped-descendant");
    let wrapper = tmp.path().join("forking-git-wrapper.sh");
    std::fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\n( printf escaped > '{}' ) &\nexec git \"$@\"\n",
            escaped.display()
        ),
    )
    .expect("write adversarial wrapper");
    let mut permissions = std::fs::metadata(&wrapper).unwrap().permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&wrapper, permissions).unwrap();
    manager.set_test_git_binary(wrapper);

    let result = manager.create("no-escape", "feature/no-escape").await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!escaped.exists(), "wrapper fork escaped containment");
    assert!(result.is_err(), "forking wrapper should fail closed");
    assert!(!manager.path_for("no-escape").exists());
    assert!(!manager.creation_marker_path("no-escape").exists());
    assert!(!manager.creation_claim_path("no-escape").exists());
    assert!(manager.operations.try_lock().is_ok());
}

#[tokio::test]
async fn checkout_extension_is_rejected_before_creation_side_effects() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let status = StdCommand::new("git")
        .current_dir(&manager.config.repo_root)
        .args(["config", "filter.hostile.smudge", "/usr/bin/true"])
        .status()
        .expect("configure hostile filter");
    assert!(status.success());

    let error = manager
        .create("policy-reject", "feature/policy-reject")
        .await
        .expect_err("checkout extension must fail closed");
    assert!(matches!(error, WorktreeError::UnsafeGitExecution { .. }));
    assert!(!manager.path_for("policy-reject").exists());
    assert!(!manager.creation_marker_path("policy-reject").exists());
    assert!(!manager.creation_claim_path("policy-reject").exists());
    assert!(manager.operations.try_lock().is_ok());
}

#[tokio::test]
async fn repository_fsmonitor_is_disabled_per_managed_git_invocation() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let status = StdCommand::new("git")
        .current_dir(&manager.config.repo_root)
        .args(["config", "core.fsmonitor", "true"])
        .status()
        .expect("configure repository fsmonitor");
    assert!(status.success());

    let handle = manager
        .create("fsmonitor-compatible", "feature/fsmonitor-compatible")
        .await
        .expect("managed worktree commands must override repository fsmonitor");
    assert!(handle.path.exists());
    assert_eq!(
        manager
            .check_health("fsmonitor-compatible")
            .await
            .expect("health probe must override repository fsmonitor"),
        WorktreeHealth::Ok
    );

    manager
        .remove("fsmonitor-compatible")
        .await
        .expect("managed removal must also override repository fsmonitor");
    assert!(!handle.path.exists());

    let configured = StdCommand::new("git")
        .current_dir(&manager.config.repo_root)
        .args(["config", "--get", "core.fsmonitor"])
        .output()
        .expect("read repository fsmonitor");
    assert!(configured.status.success());
    assert_eq!(String::from_utf8_lossy(&configured.stdout).trim(), "true");
}

#[tokio::test]
async fn direct_create_preserves_outstanding_marker_and_owned_objects() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let id = "outstanding-marker";
    let path = manager.path_for(id);
    let admin_dir = manager
        .config
        .repo_root
        .join(".git/worktrees/roko-outstanding-marker");
    std::fs::create_dir_all(&path).unwrap();
    std::fs::create_dir_all(&admin_dir).unwrap();
    std::fs::write(path.join("owned-sentinel"), b"old path object").unwrap();
    std::fs::write(admin_dir.join("owned-sentinel"), b"old admin object").unwrap();
    let marker_path = manager.creation_marker_path(id);
    std::fs::create_dir_all(marker_path.parent().unwrap()).unwrap();
    let original_bytes = format!(
        "{{\n  \"id\": \"{id}\",\n  \"branch\": \"old/branch\",\n  \"path\": \"{}\",\n  \"admin_dir\": \"{}\",\n  \"phase\": \"linked_no_checkout\"\n}}\n",
        path.display(),
        admin_dir.display()
    )
    .into_bytes();
    std::fs::write(&marker_path, &original_bytes).unwrap();

    for result in [
        manager.create(id, "new/branch").await.map(|_| ()),
        manager.create_for_plan(id).await.map(|_| ()),
    ] {
        assert!(matches!(
            result,
            Err(WorktreeError::ReattachRejected { .. })
        ));
        assert_eq!(std::fs::read(&marker_path).unwrap(), original_bytes);
        assert_eq!(
            std::fs::read(path.join("owned-sentinel")).unwrap(),
            b"old path object"
        );
        assert_eq!(
            std::fs::read(admin_dir.join("owned-sentinel")).unwrap(),
            b"old admin object"
        );
    }
    assert!(manager.operations.try_lock().is_ok());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn dangling_creation_marker_rejects_direct_create() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let id = "dangling-marker";
    let marker_path = manager.creation_marker_path(id);
    std::fs::create_dir_all(marker_path.parent().unwrap()).unwrap();
    let missing_target = marker_path.with_extension("missing");
    std::os::unix::fs::symlink(&missing_target, &marker_path).unwrap();

    let error = manager
        .create_for_plan(id)
        .await
        .expect_err("dangling marker must retain the claim");
    assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
    assert!(
        std::fs::symlink_metadata(&marker_path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(!missing_target.exists());
    assert!(!manager.path_for(id).exists());
}

#[test]
fn creation_marker_transitions_require_exact_identity_and_prior_phase() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
    let target_oid = String::from_utf8(
        StdCommand::new("git")
            .current_dir(&manager.config.repo_root)
            .args(["rev-parse", "main"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    let common_git_dir = std::fs::canonicalize(manager.config.repo_root.join(".git")).unwrap();
    let marker = CreationMarker {
        schema_version: CREATION_MARKER_SCHEMA,
        claim_id: uuid::Uuid::new_v4().simple().to_string(),
        id: "phase-identity".to_string(),
        repo_root: manager.config.repo_root.clone(),
        common_git_dir: common_git_dir.clone(),
        branch: "feature/phase-identity".to_string(),
        branch_old_oid: None,
        target_oid,
        path: manager.path_for("phase-identity"),
        admin_dir: common_git_dir.join("worktrees/roko-phase-identity"),
        phase: CreationPhase::Prepared,
        previous_digest: None,
    };
    let mut claim = manager.publish_creation_marker(marker).unwrap();
    let claim_path = manager.creation_claim_path(&claim.marker.id);
    let prepared_path = claim_path.join(creation_record_name(
        &claim.marker.claim_id,
        CreationPhase::Prepared,
    ));
    let prepared_bytes = std::fs::read(&prepared_path).unwrap();

    assert!(
        manager
            .transition_creation_marker(&mut claim, CreationPhase::ResetComplete)
            .is_err()
    );
    assert_eq!(std::fs::read(&prepared_path).unwrap(), prepared_bytes);

    manager
        .transition_creation_marker(&mut claim, CreationPhase::LinkedNoCheckout)
        .unwrap();
    manager
        .transition_creation_marker(&mut claim, CreationPhase::ResetComplete)
        .unwrap();
    let linked = std::fs::read(claim_path.join(creation_record_name(
        &claim.marker.claim_id,
        CreationPhase::LinkedNoCheckout,
    )))
    .unwrap();
    let reset: CreationMarker = serde_json::from_slice(
        &std::fs::read(claim_path.join(creation_record_name(
            &claim.marker.claim_id,
            CreationPhase::ResetComplete,
        )))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        reset.previous_digest,
        Some(blake3::hash(&linked).to_hex().to_string())
    );
    // This unit test does not seed the completed worktree filesystem;
    // rollback cleanup exercises exact fd-relative removal instead.
    manager.remove_creation_claim_if_exact(&claim).unwrap();
    assert!(std::fs::symlink_metadata(claim_path).is_err());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn transition_path_swap_returns_error_and_preserves_foreign_claim_bytes() {
    use std::os::unix::fs::PermissionsExt;

    let Some((tmp, manager)) = make_manager() else {
        return;
    };
    std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
    let claim = manager
        .publish_creation_marker(test_creation_marker(&manager, "transition-swap"))
        .unwrap();
    let public = manager.creation_claim_path("transition-swap");
    let detached = tmp.path().join("detached-transition-claim");
    let started = tmp.path().join("transition-swap-started");
    let release = tmp.path().join("transition-swap-release");
    manager.set_test_claim_mutation_barrier(TestClaimMutationBarrier {
        point: TestClaimMutationPoint::BeforeTransitionWrite,
        started: started.clone(),
        release: release.clone(),
    });
    let worker_manager = manager.clone();
    let worker = std::thread::spawn(move || {
        let mut claim = claim;
        worker_manager.transition_creation_marker(&mut claim, CreationPhase::LinkedNoCheckout)
    });
    wait_for_file_sync(&started);
    std::fs::rename(&public, &detached).unwrap();
    std::fs::create_dir(&public).unwrap();
    std::fs::set_permissions(&public, std::fs::Permissions::from_mode(0o700)).unwrap();
    let foreign = public.join("foreign-claim.json");
    std::fs::write(&foreign, b"foreign transition claim\n").unwrap();
    std::fs::set_permissions(&foreign, std::fs::Permissions::from_mode(0o600)).unwrap();
    let foreign_bytes = std::fs::read(&foreign).unwrap();
    std::fs::write(&release, b"release").unwrap();
    assert!(worker.join().unwrap().is_err());
    assert_eq!(std::fs::read(&foreign).unwrap(), foreign_bytes);
    assert_eq!(std::fs::read_dir(&public).unwrap().count(), 1);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn removal_path_swap_returns_error_and_preserves_foreign_claim_bytes() {
    use std::os::unix::fs::PermissionsExt;

    let Some((tmp, manager)) = make_manager() else {
        return;
    };
    std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
    let mut claim = manager
        .publish_creation_marker(test_creation_marker(&manager, "removal-swap"))
        .unwrap();
    manager
        .transition_creation_marker(&mut claim, CreationPhase::LinkedNoCheckout)
        .unwrap();
    manager
        .transition_creation_marker(&mut claim, CreationPhase::ResetComplete)
        .unwrap();
    let public = manager.creation_claim_path("removal-swap");
    let detached = tmp.path().join("detached-removal-claim");
    let started = tmp.path().join("removal-swap-started");
    let release = tmp.path().join("removal-swap-release");
    manager.set_test_claim_mutation_barrier(TestClaimMutationBarrier {
        point: TestClaimMutationPoint::BeforeRemovalCleanup,
        started: started.clone(),
        release: release.clone(),
    });
    let worker_manager = manager.clone();
    let worker =
        std::thread::spawn(move || worker_manager.remove_completed_creation_marker(&claim));
    wait_for_file_sync(&started);
    std::fs::rename(&public, &detached).unwrap();
    std::fs::create_dir(&public).unwrap();
    std::fs::set_permissions(&public, std::fs::Permissions::from_mode(0o700)).unwrap();
    let foreign = public.join("foreign-claim.json");
    std::fs::write(&foreign, b"foreign removal claim\n").unwrap();
    std::fs::set_permissions(&foreign, std::fs::Permissions::from_mode(0o600)).unwrap();
    let foreign_bytes = std::fs::read(&foreign).unwrap();
    std::fs::write(&release, b"release").unwrap();
    assert!(worker.join().unwrap().is_err());
    assert_eq!(std::fs::read(&foreign).unwrap(), foreign_bytes);
    assert_eq!(std::fs::read_dir(&public).unwrap().count(), 1);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn parent_root_swap_returns_error_and_preserves_foreign_root_bytes() {
    use std::os::unix::fs::PermissionsExt;

    let Some((tmp, manager)) = make_manager() else {
        return;
    };
    std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
    let claim = manager
        .publish_creation_marker(test_creation_marker(&manager, "root-swap"))
        .unwrap();
    let public_root = manager.config.worktrees_root.join(CREATION_MARKER_DIR);
    let detached = tmp.path().join("detached-marker-root");
    let started = tmp.path().join("root-swap-started");
    let release = tmp.path().join("root-swap-release");
    manager.set_test_claim_mutation_barrier(TestClaimMutationBarrier {
        point: TestClaimMutationPoint::BeforeTransitionWrite,
        started: started.clone(),
        release: release.clone(),
    });
    let worker_manager = manager.clone();
    let worker = std::thread::spawn(move || {
        let mut claim = claim;
        worker_manager.transition_creation_marker(&mut claim, CreationPhase::LinkedNoCheckout)
    });
    wait_for_file_sync(&started);
    std::fs::rename(&public_root, &detached).unwrap();
    std::fs::create_dir(&public_root).unwrap();
    std::fs::set_permissions(&public_root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let foreign = public_root.join("foreign-root-record");
    std::fs::write(&foreign, b"foreign root bytes\n").unwrap();
    std::fs::set_permissions(&foreign, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::write(&release, b"release").unwrap();
    assert!(worker.join().unwrap().is_err());
    assert_eq!(std::fs::read(&foreign).unwrap(), b"foreign root bytes\n");
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn wait_for_file_sync(path: &Path) {
    let started = std::time::Instant::now();
    while !path.exists() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "barrier timeout"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn prepared_claim_is_restart_fail_closed_and_byte_preserved() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
    let claim = manager
        .publish_creation_marker(test_creation_marker(&manager, "prepared-restart"))
        .unwrap();
    let claim_path = manager.creation_claim_path("prepared-restart");
    let prepared = claim_path.join(creation_record_name(
        &claim.marker.claim_id,
        CreationPhase::Prepared,
    ));
    let bytes = std::fs::read(&prepared).unwrap();
    drop(claim);

    let error = manager
        .ensure_for_plan("prepared-restart")
        .await
        .expect_err("Prepared restart state must remain fail closed");
    assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
    assert_eq!(std::fs::read(prepared).unwrap(), bytes);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn empty_claim_publication_window_is_recovered_under_repository_lock() {
    use std::os::unix::fs::PermissionsExt;

    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
    let _lock = manager.acquire_repository_mutation_lock().unwrap();
    let _root = manager.open_creation_marker_root(true).unwrap();
    let empty = manager.creation_claim_path("empty-restart");
    std::fs::create_dir(&empty).unwrap();
    std::fs::set_permissions(&empty, std::fs::Permissions::from_mode(0o700)).unwrap();
    manager
        .recover_or_reject_creation_claim("empty-restart")
        .await
        .unwrap();
    assert!(std::fs::symlink_metadata(empty).is_err());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_owner_prevents_empty_claim_recovery() {
    use std::os::unix::fs::PermissionsExt;

    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
    let owner = manager.acquire_repository_mutation_lock().unwrap();
    let _root = manager.open_creation_marker_root(true).unwrap();
    let empty = manager.creation_claim_path("live-empty");
    std::fs::create_dir(&empty).unwrap();
    std::fs::set_permissions(&empty, std::fs::Permissions::from_mode(0o700)).unwrap();
    let contender = WorktreeManager::new((*manager.config).clone());
    let task = tokio::spawn(async move { contender.ensure_for_plan("live-empty").await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !task.is_finished(),
        "live owner did not exclude empty recovery"
    );
    assert!(empty.is_dir());
    drop(owner);
    let recovered = task.await.unwrap().unwrap();
    assert!(recovered.path.exists());
    assert!(std::fs::symlink_metadata(empty).is_err());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_owner_prevents_reset_complete_recovery() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let handle = manager.create_for_plan("live-reset").await.unwrap();
    let owner = manager.acquire_repository_mutation_lock().unwrap();
    let mut marker = test_creation_marker(&manager, "live-reset");
    marker.branch = handle.branch.clone();
    marker.path = handle.path.clone();
    marker.admin_dir = read_gitdir(&handle.path).unwrap();
    marker.branch_old_oid = Some(marker.target_oid.clone());
    let mut claim = manager.publish_creation_marker(marker).unwrap();
    manager
        .transition_creation_marker(&mut claim, CreationPhase::LinkedNoCheckout)
        .unwrap();
    manager
        .transition_creation_marker(&mut claim, CreationPhase::ResetComplete)
        .unwrap();
    let claim_path = manager.creation_claim_path("live-reset");
    let contender = WorktreeManager::new((*manager.config).clone());
    let task = tokio::spawn(async move { contender.ensure_for_plan("live-reset").await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !task.is_finished(),
        "live owner did not exclude ResetComplete recovery"
    );
    assert!(claim_path.is_dir());
    drop(claim);
    drop(owner);
    let recovered = task.await.unwrap().unwrap();
    assert_eq!(recovered.path, handle.path);
    assert!(std::fs::symlink_metadata(claim_path).is_err());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn record_symlink_is_rejected_without_following_or_removal() {
    let Some((tmp, manager)) = make_manager() else {
        return;
    };
    std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
    let claim = manager
        .publish_creation_marker(test_creation_marker(&manager, "record-symlink"))
        .unwrap();
    let record = manager
        .creation_claim_path("record-symlink")
        .join(creation_record_name(
            &claim.marker.claim_id,
            CreationPhase::Prepared,
        ));
    let outside = tmp.path().join("outside-record");
    std::fs::write(&outside, b"outside bytes\n").unwrap();
    let outside_bytes = std::fs::read(&outside).unwrap();
    std::fs::remove_file(&record).unwrap();
    std::os::unix::fs::symlink(&outside, &record).unwrap();
    drop(claim);

    assert!(manager.ensure_for_plan("record-symlink").await.is_err());
    assert!(
        std::fs::symlink_metadata(&record)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(std::fs::read(outside).unwrap(), outside_bytes);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn hard_linked_record_is_rejected_and_preserved() {
    use std::os::unix::fs::MetadataExt;

    let Some((tmp, manager)) = make_manager() else {
        return;
    };
    std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
    let claim = manager
        .publish_creation_marker(test_creation_marker(&manager, "record-hardlink"))
        .unwrap();
    let record = manager
        .creation_claim_path("record-hardlink")
        .join(creation_record_name(
            &claim.marker.claim_id,
            CreationPhase::Prepared,
        ));
    let outside = tmp.path().join("outside-hardlink");
    std::fs::hard_link(&record, &outside).unwrap();
    let bytes = std::fs::read(&record).unwrap();
    drop(claim);

    assert!(manager.ensure_for_plan("record-hardlink").await.is_err());
    assert_eq!(std::fs::read(&record).unwrap(), bytes);
    assert_eq!(std::fs::read(&outside).unwrap(), bytes);
    assert_eq!(std::fs::metadata(record).unwrap().nlink(), 2);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn mixed_uuid_record_is_rejected_and_preserved() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
    let claim = manager
        .publish_creation_marker(test_creation_marker(&manager, "mixed-uuid"))
        .unwrap();
    let record = manager
        .creation_claim_path("mixed-uuid")
        .join(creation_record_name(
            &claim.marker.claim_id,
            CreationPhase::Prepared,
        ));
    let mut foreign = claim.marker.clone();
    foreign.claim_id = uuid::Uuid::new_v4().simple().to_string();
    let mut foreign_bytes = serde_json::to_vec(&foreign).unwrap();
    foreign_bytes.push(b'\n');
    std::fs::write(&record, &foreign_bytes).unwrap();
    drop(claim);

    assert!(manager.ensure_for_plan("mixed-uuid").await.is_err());
    assert_eq!(std::fs::read(record).unwrap(), foreign_bytes);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn branch_compare_and_swap_rejects_drift_and_preserves_foreign_ref() {
    let Some((tmp, manager)) = make_manager() else {
        return;
    };
    let started = tmp.path().join("branch-cas-started");
    let release = tmp.path().join("branch-cas-release");
    manager.set_test_claim_mutation_barrier(TestClaimMutationBarrier {
        point: TestClaimMutationPoint::BeforeBranchCas,
        started: started.clone(),
        release: release.clone(),
    });
    let creating = manager.clone();
    let task = tokio::spawn(async move { creating.create("cas-drift", "feature/cas-drift").await });
    wait_for_barrier(&started).await;
    assert!(
        StdCommand::new("git")
            .current_dir(&manager.config.repo_root)
            .args(["commit", "--allow-empty", "-m", "foreign drift"])
            .status()
            .unwrap()
            .success()
    );
    let foreign_oid = String::from_utf8(
        StdCommand::new("git")
            .current_dir(&manager.config.repo_root)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    assert!(
        StdCommand::new("git")
            .current_dir(&manager.config.repo_root)
            .args(["update-ref", "refs/heads/feature/cas-drift", &foreign_oid,])
            .status()
            .unwrap()
            .success()
    );
    std::fs::write(&release, b"release").unwrap();
    assert!(task.await.unwrap().is_err());
    let actual = String::from_utf8(
        StdCommand::new("git")
            .current_dir(&manager.config.repo_root)
            .args(["rev-parse", "refs/heads/feature/cas-drift"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert_eq!(actual.trim(), foreign_oid);
    assert!(!manager.path_for("cas-drift").exists());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn legacy_marker_kinds_and_bytes_never_migrate_automatically() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    for (id, bytes) in [
        ("legacy-prepared", b"{malformed prepared\n".as_slice()),
        (
            "legacy-reset",
            b"{\"phase\":\"reset_complete\"}\n".as_slice(),
        ),
    ] {
        let path = manager.creation_marker_path(id);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(manager.ensure_for_plan(id).await.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(!manager.creation_claim_path(id).exists());
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn cleanup_safe_restart_reproves_checkout_and_removes_terminal_claim() {
    use std::os::unix::fs::PermissionsExt;

    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let handle = manager
        .create_for_plan("cleanup-restart")
        .await
        .expect("seed completed checkout");
    let mut marker = test_creation_marker(&manager, "cleanup-restart");
    marker.branch = handle.branch.clone();
    marker.path = handle.path.clone();
    marker.admin_dir = read_gitdir(&handle.path).unwrap();
    marker.branch_old_oid = Some(marker.target_oid.clone());
    let mut claim = manager.publish_creation_marker(marker).unwrap();
    manager
        .transition_creation_marker(&mut claim, CreationPhase::LinkedNoCheckout)
        .unwrap();
    manager
        .transition_creation_marker(&mut claim, CreationPhase::ResetComplete)
        .unwrap();
    ensure_cleanup_safe(&claim.claim_dir_fd, &claim.marker).unwrap();
    unlink_claim_file(
        &claim.claim_dir_fd,
        &creation_record_name(&claim.marker.claim_id, CreationPhase::Prepared),
    )
    .unwrap();
    unlink_claim_file(
        &claim.claim_dir_fd,
        &creation_record_name(&claim.marker.claim_id, CreationPhase::LinkedNoCheckout),
    )
    .unwrap();
    let claim_path = manager.creation_claim_path("cleanup-restart");
    let reset_path = claim_path.join(creation_record_name(
        &claim.marker.claim_id,
        CreationPhase::ResetComplete,
    ));
    let cleanup_path = claim_path.join("cleanup-safe.json");
    let reset_bytes = std::fs::read(&reset_path).unwrap();
    let cleanup_bytes = std::fs::read(&cleanup_path).unwrap();
    let unknown_path = claim_path.join("foreign-record");
    std::fs::write(&unknown_path, b"foreign\n").unwrap();
    std::fs::set_permissions(&unknown_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    drop(claim);

    assert!(manager.ensure_for_plan("cleanup-restart").await.is_err());
    assert_eq!(std::fs::read(&reset_path).unwrap(), reset_bytes);
    assert_eq!(std::fs::read(&cleanup_path).unwrap(), cleanup_bytes);
    assert_eq!(std::fs::read(&unknown_path).unwrap(), b"foreign\n");
    std::fs::remove_file(unknown_path).unwrap();

    let recovered = manager.ensure_for_plan("cleanup-restart").await.unwrap();
    assert_eq!(recovered.path, handle.path);
    assert!(std::fs::symlink_metadata(claim_path).is_err());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn every_cleanup_unlink_crash_prefix_converges() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    for prefix_len in 0..=5 {
        let id = format!("cleanup-prefix-{prefix_len}");
        let handle = manager.create_for_plan(&id).await.unwrap();
        let mut marker = test_creation_marker(&manager, &id);
        marker.branch = handle.branch.clone();
        marker.path = handle.path.clone();
        marker.admin_dir = read_gitdir(&handle.path).unwrap();
        marker.branch_old_oid = Some(marker.target_oid.clone());
        let mut claim = manager.publish_creation_marker(marker).unwrap();
        manager
            .transition_creation_marker(&mut claim, CreationPhase::LinkedNoCheckout)
            .unwrap();
        manager
            .transition_creation_marker(&mut claim, CreationPhase::ResetComplete)
            .unwrap();
        ensure_cleanup_safe(&claim.claim_dir_fd, &claim.marker).unwrap();
        let cleanup_order = [
            creation_record_name(&claim.marker.claim_id, CreationPhase::Prepared),
            creation_record_name(&claim.marker.claim_id, CreationPhase::LinkedNoCheckout),
            creation_record_name(&claim.marker.claim_id, CreationPhase::ResetComplete),
            "claim-id".to_string(),
            "cleanup-safe.json".to_string(),
        ];
        for name in cleanup_order.iter().take(prefix_len) {
            unlink_claim_file(&claim.claim_dir_fd, name).unwrap();
        }
        rustix::fs::fsync(&claim.claim_dir_fd).unwrap();
        let claim_path = manager.creation_claim_path(&id);
        drop(claim);

        let recovered = manager.ensure_for_plan(&id).await.unwrap();
        assert_eq!(recovered.path, handle.path, "prefix {prefix_len}");
        assert!(
            std::fs::symlink_metadata(&claim_path).is_err(),
            "prefix {prefix_len} left a claim"
        );
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn unknown_journal_entry_fails_closed_without_cleanup() {
    use std::os::unix::fs::PermissionsExt;

    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    std::fs::create_dir_all(&manager.config.worktrees_root).unwrap();
    let claim = manager
        .publish_creation_marker(test_creation_marker(&manager, "unknown-entry"))
        .unwrap();
    let unknown = manager
        .creation_claim_path("unknown-entry")
        .join("foreign-record");
    std::fs::write(&unknown, b"foreign\n").unwrap();
    std::fs::set_permissions(&unknown, std::fs::Permissions::from_mode(0o600)).unwrap();
    drop(claim);
    assert!(manager.ensure_for_plan("unknown-entry").await.is_err());
    assert_eq!(std::fs::read(unknown).unwrap(), b"foreign\n");
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn repository_flock_serializes_cross_root_manager_instances() {
    let Some((tmp, manager)) = make_manager() else {
        return;
    };
    let owner = manager.acquire_repository_mutation_lock().unwrap();
    let contender = manager_with_worktrees_root(&manager, tmp.path().join("alternate-worktrees"));
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (acquired_tx, acquired_rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        let guard = contender.acquire_repository_mutation_lock().unwrap();
        acquired_tx.send(()).unwrap();
        drop(guard);
    });
    started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(
        acquired_rx
            .recv_timeout(Duration::from_millis(100))
            .is_err(),
        "cross-root manager bypassed canonical repository flock"
    );
    drop(owner);
    acquired_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    worker.join().unwrap();
    assert!(repository_lock_path(&manager).exists());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn linked_repo_root_uses_the_same_canonical_common_directory_lock() {
    let Some((tmp, manager)) = make_manager() else {
        return;
    };
    let linked_root = tmp.path().join("linked-repository-root");
    let added = StdCommand::new("git")
        .current_dir(&manager.config.repo_root)
        .args([
            "worktree",
            "add",
            "-b",
            "linked-root",
            linked_root.to_str().unwrap(),
            "main",
        ])
        .status()
        .unwrap();
    assert!(added.success());
    assert!(linked_root.join(".git").is_file());
    let linked_manager = WorktreeManager::new(WorktreeConfig {
        repo_root: linked_root,
        base_branch: "main".to_string(),
        worktrees_root: tmp.path().join("linked-manager-worktrees"),
        max_live: None,
        idle_ttl: Duration::from_secs(3600),
    });
    assert_eq!(
        repository_lock_path(&manager),
        repository_lock_path(&linked_manager)
    );

    let owner = manager.acquire_repository_mutation_lock().unwrap();
    let (acquired_tx, acquired_rx) = std::sync::mpsc::channel();
    let linked_contender = linked_manager.clone();
    let worker = std::thread::spawn(move || {
        let _guard = linked_contender.acquire_repository_mutation_lock().unwrap();
        acquired_tx.send(()).unwrap();
    });
    assert!(
        acquired_rx
            .recv_timeout(Duration::from_millis(100))
            .is_err(),
        "linked repo_root bypassed its canonical common-directory lock"
    );
    drop(owner);
    acquired_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    worker.join().unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let created = runtime
        .block_on(linked_manager.create_for_plan("linked-root-create"))
        .unwrap();
    assert!(created.path.exists());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cross_root_create_serializes_same_and_different_ids() {
    let Some((tmp, manager)) = make_manager() else {
        return;
    };
    let contender = manager_with_worktrees_root(&manager, tmp.path().join("alternate-worktrees"));

    let same_started = tmp.path().join("same-create-started");
    let same_release = tmp.path().join("same-create-release");
    manager.set_test_phase_barrier(TestPhaseBarrier {
        phase: CreationPhase::LinkedNoCheckout,
        started: same_started.clone(),
        release: same_release.clone(),
    });
    let owner_manager = manager.clone();
    let owner = tokio::spawn(async move { owner_manager.create_for_plan("cross-root-same").await });
    wait_for_barrier(&same_started).await;
    let same_contender = contender.clone();
    let raced =
        tokio::spawn(async move { same_contender.create_for_plan("cross-root-same").await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !raced.is_finished(),
        "same-id create bypassed repository lock"
    );
    std::fs::write(&same_release, b"release").unwrap();
    let first = owner.await.unwrap().unwrap();
    let error = raced.await.unwrap().unwrap_err();
    assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
    assert!(first.path.exists());
    assert!(!contender.path_for("cross-root-same").exists());

    let different_started = tmp.path().join("different-create-started");
    let different_release = tmp.path().join("different-create-release");
    manager.set_test_phase_barrier(TestPhaseBarrier {
        phase: CreationPhase::LinkedNoCheckout,
        started: different_started.clone(),
        release: different_release.clone(),
    });
    let owner_manager = manager.clone();
    let owner =
        tokio::spawn(async move { owner_manager.create_for_plan("cross-root-owner").await });
    wait_for_barrier(&different_started).await;
    let different_contender = contender.clone();
    let raced = tokio::spawn(async move {
        different_contender
            .create_for_plan("cross-root-contender")
            .await
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !raced.is_finished(),
        "different-id create bypassed repository lock"
    );
    std::fs::write(&different_release, b"release").unwrap();
    assert!(owner.await.unwrap().unwrap().path.exists());
    assert!(raced.await.unwrap().unwrap().path.exists());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cross_root_create_serializes_remove() {
    let Some((tmp, manager)) = make_manager() else {
        return;
    };
    let remover = manager_with_worktrees_root(&manager, tmp.path().join("remover-worktrees"));
    let removed_path = remover
        .create_for_plan("cross-root-remove")
        .await
        .unwrap()
        .path;
    let started = tmp.path().join("create-remove-started");
    let release = tmp.path().join("create-remove-release");
    manager.set_test_phase_barrier(TestPhaseBarrier {
        phase: CreationPhase::LinkedNoCheckout,
        started: started.clone(),
        release: release.clone(),
    });
    let owner_manager = manager.clone();
    let owner =
        tokio::spawn(async move { owner_manager.create_for_plan("cross-root-create").await });
    wait_for_barrier(&started).await;
    let remove_manager = remover.clone();
    let raced = tokio::spawn(async move { remove_manager.remove("cross-root-remove").await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !raced.is_finished(),
        "remove bypassed cross-root create owner"
    );
    assert!(removed_path.exists());
    std::fs::write(&release, b"release").unwrap();
    assert!(owner.await.unwrap().unwrap().path.exists());
    raced.await.unwrap().unwrap();
    assert!(!removed_path.exists());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn repository_flock_serializes_a_separate_process() {
    let Some((tmp, manager)) = make_manager() else {
        return;
    };
    let owner = manager.acquire_repository_mutation_lock().unwrap();
    let started = tmp.path().join("subprocess-lock-started");
    let acquired = tmp.path().join("subprocess-lock-acquired");
    let mut child = StdCommand::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "orchestrator::worktree::tests::repository_lock_process_helper",
            "--nocapture",
        ])
        .env("ROKO_TEST_REPO_ROOT", &manager.config.repo_root)
        .env(
            "ROKO_TEST_WORKTREES_ROOT",
            tmp.path().join("subprocess-worktrees"),
        )
        .env("ROKO_TEST_LOCK_STARTED", &started)
        .env("ROKO_TEST_LOCK_ACQUIRED", &acquired)
        .spawn()
        .unwrap();
    wait_for_file_sync(&started);
    std::thread::sleep(Duration::from_millis(100));
    assert!(!acquired.exists(), "subprocess bypassed repository flock");
    drop(owner);
    wait_for_file_sync(&acquired);
    assert!(child.wait().unwrap().success());
}

/// bug-53475e: a process kept waiting on another's repository mutation lock
/// fails after a bounded wait, naming the holder, instead of blocking forever.
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn a_contended_repository_lock_times_out_naming_its_holder() {
    let Some((tmp, manager)) = make_manager() else {
        return;
    };
    let _owner = manager.acquire_repository_mutation_lock().unwrap();
    let outcome = tmp.path().join("contender-outcome");
    let mut child = StdCommand::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "orchestrator::worktree::tests::repository_lock_process_helper",
            "--nocapture",
        ])
        .env("ROKO_TEST_REPO_ROOT", &manager.config.repo_root)
        .env(
            "ROKO_TEST_WORKTREES_ROOT",
            tmp.path().join("contender-worktrees"),
        )
        .env("ROKO_TEST_LOCK_ACTION", "contend")
        .env("ROKO_TEST_REPOSITORY_LOCK_WAIT_MS", "300")
        .env(
            "ROKO_TEST_LOCK_STARTED",
            tmp.path().join("contender-started"),
        )
        .env("ROKO_TEST_LOCK_ACQUIRED", &outcome)
        .spawn()
        .unwrap();
    assert!(child.wait().unwrap().success());
    let error = std::fs::read_to_string(&outcome).unwrap();
    assert!(error.contains("worktree mutations are on hold"), "{error}");
    assert!(
        error.contains(&format!("pid {} ", std::process::id())),
        "{error}"
    );
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cross_root_create_serializes_a_subprocess_prune() {
    let Some((tmp, manager)) = make_manager() else {
        return;
    };
    let started = tmp.path().join("subprocess-prune-started");
    let acquired = tmp.path().join("subprocess-prune-complete");
    let create_started = tmp.path().join("subprocess-create-started");
    let create_release = tmp.path().join("subprocess-create-release");
    manager.set_test_phase_barrier(TestPhaseBarrier {
        phase: CreationPhase::LinkedNoCheckout,
        started: create_started.clone(),
        release: create_release.clone(),
    });
    let owner_manager = manager.clone();
    let owner = tokio::spawn(async move {
        owner_manager
            .create_for_plan("subprocess-prune-owner")
            .await
    });
    wait_for_barrier(&create_started).await;
    let mut child = StdCommand::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "orchestrator::worktree::tests::repository_lock_process_helper",
            "--nocapture",
        ])
        .env("ROKO_TEST_REPO_ROOT", &manager.config.repo_root)
        .env(
            "ROKO_TEST_WORKTREES_ROOT",
            tmp.path().join("subprocess-prune-worktrees"),
        )
        .env("ROKO_TEST_LOCK_ACTION", "prune")
        .env("ROKO_TEST_LOCK_STARTED", &started)
        .env("ROKO_TEST_LOCK_ACQUIRED", &acquired)
        .spawn()
        .unwrap();
    wait_for_file_sync(&started);
    std::thread::sleep(Duration::from_millis(100));
    assert!(!acquired.exists(), "subprocess prune bypassed create owner");
    std::fs::write(&create_release, b"release").unwrap();
    assert!(owner.await.unwrap().unwrap().path.exists());
    wait_for_file_sync(&acquired);
    assert!(child.wait().unwrap().success());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn repository_lock_process_helper() {
    let Ok(repo_root) = std::env::var("ROKO_TEST_REPO_ROOT") else {
        return;
    };
    let worktrees_root = std::env::var("ROKO_TEST_WORKTREES_ROOT").unwrap();
    let started = std::env::var("ROKO_TEST_LOCK_STARTED").unwrap();
    let acquired = std::env::var("ROKO_TEST_LOCK_ACQUIRED").unwrap();
    let manager = WorktreeManager::new(WorktreeConfig {
        repo_root: PathBuf::from(repo_root),
        base_branch: "main".to_string(),
        worktrees_root: PathBuf::from(worktrees_root),
        max_live: None,
        idle_ttl: Duration::from_secs(3600),
    });
    std::fs::write(started, b"started").unwrap();
    if std::env::var("ROKO_TEST_LOCK_ACTION").as_deref() == Ok("contend") {
        let error = manager
            .acquire_repository_mutation_lock()
            .expect_err("the parent holds the lock throughout");
        std::fs::write(acquired, error.to_string()).unwrap();
        return;
    }
    if std::env::var("ROKO_TEST_LOCK_ACTION").as_deref() == Ok("prune") {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(manager.prune())
            .unwrap();
    } else {
        let _owner = manager.acquire_repository_mutation_lock().unwrap();
    }
    std::fs::write(acquired, b"acquired").unwrap();
}

#[tokio::test]
async fn remove_spawn_failure_preserves_registry_for_retry() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let handle = manager
        .create_for_plan("remove-error")
        .await
        .expect("seed worktree");
    manager.set_test_git_binary(PathBuf::from("definitely-not-a-git-binary"));

    let error = manager
        .remove("remove-error")
        .await
        .expect_err("missing Git binary must fail");
    assert!(matches!(error, WorktreeError::IoError(_)));
    assert_eq!(manager.get("remove-error"), Some(handle.clone()));
    assert!(handle.path.exists());
    assert_eq!(manager.active_count(), 1);

    manager.set_test_git_binary(PathBuf::from("git"));
    manager.remove("remove-error").await.expect("retry removal");
    assert_eq!(manager.active_count(), 0);
    assert!(!handle.path.exists());
}

#[tokio::test]
async fn path_for_does_not_touch_disk() {
    let tmp = TempDir::new().unwrap();
    let mgr = WorktreeManager::new(WorktreeConfig {
        repo_root: tmp.path().join("repo"),
        base_branch: "main".to_string(),
        worktrees_root: tmp.path().join("worktrees"),
        max_live: None,
        idle_ttl: Duration::from_secs(3600),
    });
    let p = mgr.path_for("06-foxtrot");
    assert_eq!(p, tmp.path().join("worktrees").join("06-foxtrot"));
    assert!(!p.exists(), "path_for should be pure");
    // No git calls were made, so nothing was created.
    assert!(mgr.list().unwrap().is_empty());
}

#[tokio::test]
async fn concurrent_task_attempts_are_file_and_branch_disjoint() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let (first, sibling) = tokio::join!(
        manager.create_for_attempt("plan", "first", 1),
        manager.create_for_attempt("plan", "sibling", 1),
    );
    let first = first.unwrap();
    let sibling = sibling.unwrap();
    assert_ne!(first.id, sibling.id);
    assert_ne!(first.branch, sibling.branch);
    std::fs::write(first.path.join("first-only.txt"), b"owned by first\n").unwrap();
    std::fs::write(sibling.path.join("sibling-only.txt"), b"owned by sibling\n").unwrap();
    assert!(!first.path.join("sibling-only.txt").exists());
    assert!(!sibling.path.join("first-only.txt").exists());
    assert_eq!(first.id, format_attempt_worktree_id("plan", "first", 1));
    assert_eq!(first.branch, format_attempt_branch_name("plan", "first", 1));
}

#[tokio::test]
async fn accepted_attempt_is_the_immutable_base_for_the_next_task() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let first = manager
        .create_for_attempt("plan", "first", 1)
        .await
        .unwrap();
    std::fs::write(first.path.join("accepted.txt"), b"first\n").unwrap();
    for args in [
        vec!["add", "accepted.txt"],
        vec!["commit", "-m", "accepted first attempt"],
    ] {
        assert!(
            StdCommand::new("git")
                .current_dir(&first.path)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    let accepted = manager
        .accept_attempt("plan", "first", 1, &acceptance_in("run-1"))
        .await
        .unwrap();
    std::fs::write(first.path.join("late.txt"), b"must not propagate\n").unwrap();
    for args in [
        vec!["add", "late.txt"],
        vec!["commit", "-m", "late mutation"],
    ] {
        assert!(
            StdCommand::new("git")
                .current_dir(&first.path)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }

    let next = manager.create_for_attempt("plan", "next", 1).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(next.path.join("accepted.txt")).unwrap(),
        "first\n"
    );
    assert!(!next.path.join("late.txt").exists());
    let next_oid = StdCommand::new("git")
        .current_dir(&next.path)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&next_oid.stdout).trim(),
        accepted.commit_oid
    );
    assert_eq!(manager.accepted_for_plan("plan"), Some(accepted));
    assert_ne!(first.path, next.path);
}

/// bug-056b40: a resumed run's attempts start from the plan branch its
/// earlier process accepted work onto, and re-attach the checkouts that
/// process kept. Another run continues neither.
#[tokio::test]
async fn a_resumed_run_starts_attempts_from_the_plan_branch() {
    let Some((_tmp, first_process)) = make_manager() else {
        return;
    };
    let worktrees_root = first_process.config.worktrees_root.clone();
    let repo = first_process.config.repo_root.clone();
    let base = git_in(&repo, &["rev-parse", "main"]);
    assert_eq!(
        first_process.begin_plan_run("plan", "run-1").await.unwrap(),
        None
    );
    let first = first_process
        .create_for_attempt("plan", "first", 0)
        .await
        .unwrap();
    std::fs::write(first.path.join("accepted.txt"), b"first\n").unwrap();
    let tip = first_process
        .accept_attempt("plan", "first", 0, &acceptance_in("run-1"))
        .await
        .unwrap()
        .commit_oid;
    let kept = first_process
        .create_for_attempt("plan", "second", 0)
        .await
        .unwrap();
    std::fs::write(kept.path.join("half-done.txt"), b"in progress\n").unwrap();

    // A new process of the same run.
    let resumed = manager_with_worktrees_root(&first_process, worktrees_root.clone());
    assert_eq!(
        resumed.begin_plan_run("plan", "run-1").await.unwrap(),
        Some(tip.clone())
    );
    let next = resumed
        .create_for_attempt("plan", "third", 0)
        .await
        .unwrap();
    assert_eq!(git_in(&next.path, &["rev-parse", "HEAD"]), tip);
    assert!(next.path.join("accepted.txt").exists());
    let reattached = resumed
        .create_for_attempt("plan", "second", 0)
        .await
        .expect("the run's kept checkout is re-attached");
    assert_eq!(reattached.path, kept.path);
    assert_eq!(
        std::fs::read_to_string(reattached.path.join("half-done.txt")).unwrap(),
        "in progress\n"
    );

    // Another run starts from the configured base and does not take over
    // the checkouts run-1 kept.
    let other_run = manager_with_worktrees_root(&first_process, worktrees_root);
    assert_eq!(
        other_run.begin_plan_run("plan", "run-2").await.unwrap(),
        None
    );
    let fresh = other_run
        .create_for_attempt("plan", "fourth", 0)
        .await
        .unwrap();
    assert_eq!(git_in(&fresh.path, &["rev-parse", "HEAD"]), base);
    assert!(
        other_run
            .create_for_attempt("plan", "second", 0)
            .await
            .is_err()
    );
}

fn acceptance_in(run_id: &str) -> AttemptAcceptance {
    AttemptAcceptance {
        run_id: run_id.to_string(),
        attempt_key: format!("{run_id}:plan:task:1"),
        verdict: "passed".to_string(),
        title: "Accept the attempt".to_string(),
    }
}

fn git_in(dir: &Path, args: &[&str]) -> String {
    let output = StdCommand::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// An attempt of `task` in `plan` that left `file` with `contents`,
/// uncommitted, in its own checkout.
async fn attempt_with_file(
    manager: &WorktreeManager,
    task: &str,
    file: &str,
    contents: &str,
) -> super::WorktreeHandle {
    let handle = manager.create_for_attempt("plan", task, 0).await.unwrap();
    std::fs::write(handle.path.join(file), contents).unwrap();
    handle
}

/// gap-3b5361: accepting commits each attempt's own changes and folds
/// siblings that started from the same base into one plan branch, without
/// touching the operator's checkout.
#[tokio::test]
async fn accept_folds_sibling_attempts_into_the_plan_branch() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let repo = manager.config.repo_root.clone();
    let main = git_in(&repo, &["rev-parse", "main"]);
    let first = attempt_with_file(&manager, "first", "first.txt", "first\n").await;
    let sibling = attempt_with_file(&manager, "sibling", "sibling.txt", "sibling\n").await;

    let one = manager
        .accept_attempt("plan", "first", 0, &acceptance_in("run-1"))
        .await
        .unwrap();
    let two = manager
        .accept_attempt("plan", "sibling", 0, &acceptance_in("run-1"))
        .await
        .unwrap();

    // The first fast-forwards the new plan branch; the sibling is merged in.
    assert_eq!(one.commit_oid, one.attempt_commit);
    let plan = format_branch_name("plan");
    assert_eq!(git_in(&repo, &["rev-parse", &plan]), two.commit_oid);
    let parents = git_in(
        &repo,
        &["rev-list", "--parents", "-n", "1", &two.commit_oid],
    );
    assert_eq!(
        parents,
        format!(
            "{} {} {}",
            two.commit_oid, one.commit_oid, two.attempt_commit
        )
    );
    let files = git_in(&repo, &["ls-tree", "--name-only", &plan]);
    assert!(
        files.contains("first.txt") && files.contains("sibling.txt"),
        "{files}"
    );
    // Each attempt's work is committed on its own branch, and its checkout
    // is clean.
    for handle in [&first, &sibling] {
        assert_eq!(git_in(&handle.path, &["status", "--porcelain"]), "");
    }
    let body = git_in(&repo, &["log", "-1", "--format=%B", &one.attempt_commit]);
    assert!(body.contains("Roko-Run: run-1"), "{body}");
    assert!(body.contains("Roko-Attempt: run-1:plan:task:1"), "{body}");
    // The operator's checkout never moved.
    assert_eq!(git_in(&repo, &["rev-parse", "HEAD"]), main);
    assert_eq!(git_in(&repo, &["symbolic-ref", "--short", "HEAD"]), "main");
    assert_eq!(git_in(&repo, &["status", "--porcelain"]), "");
    // Later attempts start from the plan branch.
    assert_eq!(
        manager.accepted_for_plan("plan").unwrap().commit_oid,
        two.commit_oid
    );
    let next = manager.create_for_attempt("plan", "next", 0).await.unwrap();
    assert!(next.path.join("first.txt").exists() && next.path.join("sibling.txt").exists());
}

/// gap-3b5361: a sibling whose work conflicts with what the plan branch
/// already holds is refused, and no ref moves.
#[tokio::test]
async fn accept_refuses_a_conflicting_sibling_and_moves_no_ref() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let repo = manager.config.repo_root.clone();
    attempt_with_file(&manager, "first", "same.txt", "first\n").await;
    attempt_with_file(&manager, "sibling", "same.txt", "sibling\n").await;
    let one = manager
        .accept_attempt("plan", "first", 0, &acceptance_in("run-1"))
        .await
        .unwrap();

    let error = manager
        .accept_attempt("plan", "sibling", 0, &acceptance_in("run-1"))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, WorktreeError::Conflict { paths, .. } if paths.contains("same.txt")),
        "{error}"
    );
    let plan = format_branch_name("plan");
    assert_eq!(git_in(&repo, &["rev-parse", &plan]), one.commit_oid);
    assert_eq!(manager.accepted_for_plan("plan").unwrap(), one);
}

/// gap-3b5361: a plan branch checked out anywhere is never moved.
#[tokio::test]
async fn accept_leaves_a_checked_out_plan_branch_alone() {
    let Some((tmp, manager)) = make_manager() else {
        return;
    };
    let repo = manager.config.repo_root.clone();
    let plan = format_branch_name("plan");
    git_in(&repo, &["branch", &plan, "main"]);
    let checkout = tmp.path().join("operator-plan-checkout");
    git_in(
        &repo,
        &["worktree", "add", &checkout.display().to_string(), &plan],
    );
    let tip = git_in(&repo, &["rev-parse", &plan]);
    attempt_with_file(&manager, "first", "first.txt", "first\n").await;

    let error = manager
        .accept_attempt("plan", "first", 0, &acceptance_in("run-1"))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("checked out"), "{error}");
    assert_eq!(git_in(&repo, &["rev-parse", &plan]), tip);
    assert!(!checkout.join("first.txt").exists());
}

/// gap-3b5361: a later run does not build on a plan branch another run
/// left: the old tip is kept under `refs/roko/plan-archive/`, and the plan
/// branch starts afresh. A resumed run (same run id) continues it.
#[tokio::test]
async fn accept_starts_afresh_from_another_runs_plan_branch() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let repo = manager.config.repo_root.clone();
    attempt_with_file(&manager, "old", "old.txt", "old\n").await;
    let old = manager
        .accept_attempt("plan", "old", 0, &acceptance_in("run-1"))
        .await
        .unwrap();

    // A new process: a manager with no acceptances of its own yet.
    let later = manager_with_worktrees_root(&manager, manager.config.worktrees_root.join("later"));
    attempt_with_file(&later, "new", "new.txt", "new\n").await;
    let new = later
        .accept_attempt("plan", "new", 0, &acceptance_in("run-2"))
        .await
        .unwrap();

    let plan = format_branch_name("plan");
    assert_eq!(new.commit_oid, new.attempt_commit);
    assert_eq!(git_in(&repo, &["rev-parse", &plan]), new.commit_oid);
    let archive = format!("refs/roko/plan-archive/plan/{}", old.commit_oid);
    assert_eq!(git_in(&repo, &["rev-parse", &archive]), old.commit_oid);

    // The same run, resumed in yet another process, continues the branch.
    let resumed =
        manager_with_worktrees_root(&manager, manager.config.worktrees_root.join("resumed"));
    attempt_with_file(&resumed, "more", "more.txt", "more\n").await;
    let more = resumed
        .accept_attempt("plan", "more", 0, &acceptance_in("run-2"))
        .await
        .unwrap();
    let files = git_in(&repo, &["ls-tree", "--name-only", &more.commit_oid]);
    assert!(
        files.contains("new.txt") && files.contains("more.txt"),
        "{files}"
    );
    assert!(!files.contains("old.txt"), "{files}");
}

/// gap-3b5361: the config copies roko puts in an attempt's checkout are not
/// part of the attempt's work.
#[tokio::test]
async fn accept_leaves_roko_config_copies_out_of_the_commit() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let repo = manager.config.repo_root.clone();
    std::fs::create_dir_all(repo.join(".cursor")).unwrap();
    std::fs::write(repo.join(".cursor").join("mcp.json"), "{}\n").unwrap();
    let handle = attempt_with_file(&manager, "first", "first.txt", "first\n").await;
    assert!(handle.path.join(".cursor").join("mcp.json").exists());

    let accepted = manager
        .accept_attempt("plan", "first", 0, &acceptance_in("run-1"))
        .await
        .unwrap();

    let files = git_in(
        &repo,
        &["ls-tree", "-r", "--name-only", &accepted.commit_oid],
    );
    assert!(files.contains("first.txt"), "{files}");
    assert!(!files.contains(".cursor"), "{files}");
}

#[tokio::test]
async fn removal_preserves_dirty_attempt_for_attribution_and_recovery() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let attempt = manager
        .create_for_attempt("plan", "dirty", 1)
        .await
        .unwrap();
    std::fs::write(attempt.path.join("unknown.txt"), b"do not delete\n").unwrap();

    let error = manager.remove(&attempt.id).await.unwrap_err();

    assert!(matches!(error, WorktreeError::DirtyWorktree { .. }));
    assert_eq!(
        std::fs::read_to_string(attempt.path.join("unknown.txt")).unwrap(),
        "do not delete\n"
    );
    assert_eq!(manager.get(&attempt.id), Some(attempt));
}

#[tokio::test]
async fn reflex_replay_removal_discards_only_exact_owned_dirty_checkout() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let id = "reflex-0123456789abcdef0123456789abcdef";
    let branch = format!("roko/reflex-replay/{id}");
    let replay = manager
        .create(id, &branch)
        .await
        .expect("create reflex replay checkout");
    std::fs::write(replay.path.join("reproduced.txt"), b"verified delta\n")
        .expect("write replay delta");

    manager
        .remove_reflex_replay(id)
        .await
        .expect("force-remove owned dirty replay");

    assert!(!replay.path.exists());
    assert!(manager.get(id).is_none());
    assert!(
        manager
            .remove_reflex_replay("attempt-0123456789abcdef0123456789abcdef")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn invalid_ids_are_rejected_before_git_runs() {
    let tmp = TempDir::new().unwrap();
    let mgr = WorktreeManager::new(WorktreeConfig {
        repo_root: tmp.path().join("repo"),
        base_branch: "main".to_string(),
        worktrees_root: tmp.path().join("worktrees"),
        max_live: None,
        idle_ttl: Duration::from_secs(3600),
    });
    // Neither repo nor worktrees_root exists — but we never reach
    // git because validate_id rejects these inputs first.
    for bad in [
        "",
        ".hidden",
        "-starts-with-dash",
        "has/slash",
        "has\\back",
        "white space",
        "has..dots",
        "has~tilde",
        "has^caret",
        "has:colon",
        "has?q",
        "has*star",
        "has[bracket",
        "has@at",
        "has\ttab",
    ] {
        let err = mgr.create(bad, "feature/x").await.unwrap_err();
        assert!(
            matches!(err, WorktreeError::InvalidId(_)),
            "expected InvalidId for `{bad}`, got {err:?}"
        );
    }
}

#[test]
fn validate_id_accepts_reasonable_ids() {
    for good in [
        "01-alpha",
        "plan_42",
        "08a-some-thing",
        "nested.branch-name",
    ] {
        validate_id(good).unwrap();
    }
}

// ── New tests (§15.3–§15.7, §15.9) ──

#[test]
fn format_branch_name_uses_convention() {
    assert_eq!(format_branch_name("01-alpha"), "roko/plan/01-alpha");
    assert_eq!(format_branch_name("fix_typo"), "roko/plan/fix_typo");
}

#[tokio::test]
async fn create_for_plan_uses_canonical_branch() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    let handle = mgr.create_for_plan("07-golf").await.unwrap();
    assert_eq!(handle.branch, "roko/plan/07-golf");
    assert_eq!(handle.id, "07-golf");
    assert!(handle.path.exists());
}

#[tokio::test]
async fn get_and_plan_path_return_tracked_handle() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    let handle = mgr.create_for_plan("07-golf-2").await.unwrap();
    let fetched = mgr.get("07-golf-2").unwrap();
    assert_eq!(fetched.id, "07-golf-2");
    assert_eq!(mgr.plan_path("07-golf-2"), Some(handle.path));
}

#[tokio::test]
async fn ensure_for_plan_reuses_existing_worktree() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    let first = mgr.create_for_plan("07-golf-3").await.unwrap();
    let before_count = mgr.active_count();
    let ensured = mgr.ensure_for_plan("07-golf-3").await.unwrap();
    assert_eq!(before_count, mgr.active_count());
    assert_eq!(ensured.id, first.id);
    assert_eq!(ensured.path, first.path);
    assert_eq!(ensured.branch, first.branch);
}

#[tokio::test]
async fn discover_existing_reattaches_on_disk_worktrees() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    // Create a worktree the normal way.
    let original = mgr.create_for_plan("09-resume").await.unwrap();
    let original_path = original.path.clone();
    let original_branch = original.branch.clone();
    assert!(original_path.exists());

    // Build a *fresh* manager that doesn't know about the worktree.
    let mgr2 = WorktreeManager::new(WorktreeConfig {
        repo_root: mgr.config.repo_root.clone(),
        base_branch: "main".to_string(),
        worktrees_root: original_path.parent().unwrap().to_path_buf(),
        max_live: None,
        idle_ttl: Duration::from_secs(3600),
    });
    assert_eq!(mgr2.active_count(), 0);

    // discover_existing should find the on-disk worktree.
    let discovered = mgr2.discover_existing(&["09-resume", "nonexistent"]).await;
    assert_eq!(discovered, vec!["09-resume".to_string()]);
    assert_eq!(mgr2.active_count(), 1);

    let handle = mgr2.get("09-resume").unwrap();
    assert_eq!(handle.path, original_path);
    assert_eq!(handle.branch, original_branch);
    assert!(handle.created_at_ms > 0);
    assert!(handle.last_active_ms >= handle.created_at_ms);
}

#[tokio::test]
async fn ensure_for_plan_reattaches_untracked_on_disk_worktree() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    // Create a worktree, then simulate a fresh manager (resume scenario).
    let original = mgr.create_for_plan("09-ensure").await.unwrap();
    let original_path = original.path.clone();

    let mgr2 = WorktreeManager::new(WorktreeConfig {
        repo_root: mgr.config.repo_root.clone(),
        base_branch: "main".to_string(),
        worktrees_root: original_path.parent().unwrap().to_path_buf(),
        max_live: None,
        idle_ttl: Duration::from_secs(3600),
    });
    assert!(mgr2.get("09-ensure").is_none());

    // ensure_for_plan should reattach instead of trying git worktree add.
    let ensured = mgr2.ensure_for_plan("09-ensure").await.unwrap();
    assert_eq!(ensured.path, original_path);
    assert_eq!(ensured.branch, original.branch);
}

#[tokio::test]
async fn ensure_rejects_stale_snapshot_registry_identity() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    let original = mgr.create_for_plan("09-stale-snapshot").await.unwrap();
    let mut snapshot = mgr.snapshot(42);
    snapshot.handles[0].path = original.path.with_file_name("wrong-path");
    let restored = WorktreeManager::from_snapshot((*mgr.config).clone(), snapshot).unwrap();

    let error = restored
        .ensure_for_plan("09-stale-snapshot")
        .await
        .unwrap_err();

    assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
    assert!(
        original.path.exists(),
        "valid git worktree must be preserved"
    );
}

#[tokio::test]
async fn from_snapshot_rejects_duplicate_registry_ids() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    mgr.create_for_plan("09-duplicate-snapshot").await.unwrap();
    let mut snapshot = mgr.snapshot(42);
    snapshot.handles.push(snapshot.handles[0].clone());

    let error = WorktreeManager::from_snapshot((*mgr.config).clone(), snapshot).unwrap_err();

    assert!(matches!(error, WorktreeError::AlreadyExists(ref id) if id == "09-duplicate-snapshot"));
}

#[tokio::test]
async fn concurrent_ensure_for_plan_returns_one_shared_handle() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    let first_manager = mgr.clone();
    let second_manager = mgr.clone();

    let (first, second) = tokio::join!(
        first_manager.ensure_for_plan("09-concurrent"),
        second_manager.ensure_for_plan("09-concurrent")
    );

    let first = first.expect("first ensure");
    let second = second.expect("second ensure");
    assert_eq!(first.id, second.id);
    assert_eq!(first.path, second.path);
    assert_eq!(first.branch, second.branch);
    assert_eq!(first.created_at_ms, second.created_at_ms);
    assert_eq!(mgr.active_count(), 1);

    let listed = StdCommand::new("git")
        .current_dir(&mgr.config.repo_root)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .expect("git worktree list");
    assert!(listed.status.success());
    let canonical_path = std::fs::canonicalize(&first.path).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&listed.stdout)
            .lines()
            .filter_map(|line| line.strip_prefix("worktree "))
            .filter_map(|path| std::fs::canonicalize(path).ok())
            .filter(|path| *path == canonical_path)
            .count(),
        1
    );
}

#[tokio::test]
async fn concurrent_discover_and_ensure_keep_one_active_handle() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    let original = mgr.create_for_plan("09-discover-race").await.unwrap();
    let fresh = WorktreeManager::new(WorktreeConfig {
        repo_root: mgr.config.repo_root.clone(),
        base_branch: "main".to_string(),
        worktrees_root: original.path.parent().unwrap().to_path_buf(),
        max_live: None,
        idle_ttl: Duration::from_secs(3600),
    });
    let ensure_manager = fresh.clone();
    let discover_manager = fresh.clone();

    let (ensured, discovered) = tokio::join!(
        ensure_manager.ensure_for_plan("09-discover-race"),
        discover_manager.discover_existing(&["09-discover-race"])
    );

    let ensured = ensured.expect("ensure should reuse the candidate");
    assert_eq!(ensured.path, original.path);
    assert!(discovered.is_empty() || discovered == ["09-discover-race"]);
    assert_eq!(fresh.active_count(), 1);
    assert_eq!(fresh.get("09-discover-race").unwrap().path, original.path);
}

#[tokio::test]
async fn discover_existing_rejects_wrong_branch() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    let original = mgr
        .create("09-wrong-branch", "feature/not-the-plan-branch")
        .await
        .unwrap();
    let fresh = WorktreeManager::new(WorktreeConfig {
        repo_root: mgr.config.repo_root.clone(),
        base_branch: "main".to_string(),
        worktrees_root: original.path.parent().unwrap().to_path_buf(),
        max_live: None,
        idle_ttl: Duration::from_secs(3600),
    });

    assert!(
        fresh
            .discover_existing(&["09-wrong-branch"])
            .await
            .is_empty()
    );
    assert!(fresh.get("09-wrong-branch").is_none());
    let error = fresh.ensure_for_plan("09-wrong-branch").await.unwrap_err();
    assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
}

#[tokio::test]
async fn discover_existing_rejects_foreign_repository_worktree() {
    let Some((tmp, mgr)) = make_manager() else {
        return;
    };
    let foreign_repo = tmp.path().join("foreign-repo");
    std::fs::create_dir_all(&foreign_repo).unwrap();
    init_repo(&foreign_repo);
    let candidate = mgr.path_for("09-foreign");
    std::fs::create_dir_all(candidate.parent().unwrap()).unwrap();
    let expected_branch = format_branch_name("09-foreign");
    let status = StdCommand::new("git")
        .current_dir(&foreign_repo)
        .args([
            "worktree",
            "add",
            "-b",
            &expected_branch,
            candidate.to_str().unwrap(),
            "main",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    assert!(
        mgr.discover_existing(&["09-foreign"]).await.is_empty(),
        "a canonical-looking worktree from another repository must fail closed"
    );
    assert!(mgr.get("09-foreign").is_none());
    let error = mgr.ensure_for_plan("09-foreign").await.unwrap_err();
    assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
}

#[tokio::test]
async fn inherited_git_environment_cannot_spoof_reattach_identity() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let id = "09-env-spoof";
    let expected_branch = format_branch_name(id);
    let status = StdCommand::new("git")
        .current_dir(&manager.config.repo_root)
        .args(["checkout", "-b", &expected_branch])
        .status()
        .unwrap();
    assert!(status.success());
    let candidate = manager.path_for(id);
    std::fs::create_dir_all(&candidate).unwrap();
    std::fs::write(candidate.join(".git"), b"not a linked-worktree pointer\n").unwrap();
    manager.set_test_git_probe_environment(vec![
        (
            std::ffi::OsString::from("GIT_DIR"),
            manager.config.repo_root.join(".git").into_os_string(),
        ),
        (
            std::ffi::OsString::from("GIT_WORK_TREE"),
            candidate.clone().into_os_string(),
        ),
    ]);

    assert!(manager.discover_existing(&[id]).await.is_empty());
    assert!(manager.get(id).is_none());
    let error = manager.ensure_for_plan(id).await.unwrap_err();
    assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
    let listed = StdCommand::new("git")
        .current_dir(&manager.config.repo_root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .args(["worktree", "list", "--porcelain"])
        .output()
        .unwrap();
    assert!(!worktree_list_contains_path(&listed.stdout, &candidate));
}

#[tokio::test]
async fn create_and_health_ignore_command_local_git_environment_spoofing() {
    let Some((tmp, manager)) = make_manager() else {
        return;
    };
    let foreign = tmp.path().join("probe-spoof-repo");
    std::fs::create_dir_all(&foreign).unwrap();
    init_repo(&foreign);
    manager.set_test_git_probe_environment(vec![
        (
            std::ffi::OsString::from("GIT_DIR"),
            foreign.join(".git").into_os_string(),
        ),
        (
            std::ffi::OsString::from("GIT_WORK_TREE"),
            foreign.clone().into_os_string(),
        ),
    ]);

    let handle = manager
        .create("probe-sanitized", "feature/probe-sanitized")
        .await
        .expect("sanitized create");
    let admin_dir = read_gitdir(&handle.path).expect("worktree pointer");
    let canonical_admin = std::fs::canonicalize(admin_dir).unwrap();
    let canonical_common = std::fs::canonicalize(manager.config.repo_root.join(".git"))
        .unwrap()
        .join("worktrees");
    assert_eq!(canonical_admin.parent(), Some(canonical_common.as_path()));
    assert_eq!(
        manager.check_health("probe-sanitized").await.unwrap(),
        WorktreeHealth::Ok
    );
}

#[tokio::test]
async fn discover_existing_rejects_nonreciprocal_admin_gitdir_link() {
    let Some((_tmp, manager)) = make_manager() else {
        return;
    };
    let handle = manager.create_for_plan("09-nonreciprocal").await.unwrap();
    let admin_dir = read_gitdir(&handle.path).unwrap();
    std::fs::write(
        admin_dir.join("gitdir"),
        format!("{}\n", manager.config.repo_root.join(".git").display()),
    )
    .unwrap();
    let fresh = WorktreeManager::new((*manager.config).clone());

    assert!(
        fresh
            .discover_existing(&["09-nonreciprocal"])
            .await
            .is_empty()
    );
    let error = fresh.ensure_for_plan("09-nonreciprocal").await.unwrap_err();
    assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
}

#[tokio::test]
async fn ensure_for_plan_rejects_detached_existing_worktree() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    let original = mgr.create_for_plan("09-detached").await.unwrap();
    let status = StdCommand::new("git")
        .current_dir(&original.path)
        .args(["checkout", "--detach"])
        .status()
        .unwrap();
    assert!(status.success());
    let tracked_error = mgr.ensure_for_plan("09-detached").await.unwrap_err();
    assert!(matches!(
        tracked_error,
        WorktreeError::ReattachRejected { .. }
    ));
    let fresh = WorktreeManager::new(WorktreeConfig {
        repo_root: mgr.config.repo_root.clone(),
        base_branch: "main".to_string(),
        worktrees_root: original.path.parent().unwrap().to_path_buf(),
        max_live: None,
        idle_ttl: Duration::from_secs(3600),
    });

    let error = fresh.ensure_for_plan("09-detached").await.unwrap_err();

    assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
    assert!(fresh.get("09-detached").is_none());
}

#[tokio::test]
async fn ensure_for_plan_rejects_missing_worktree_metadata() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    let original = mgr.create_for_plan("09-missing-metadata").await.unwrap();
    std::fs::remove_file(original.path.join(".git")).unwrap();
    let fresh = WorktreeManager::new(WorktreeConfig {
        repo_root: mgr.config.repo_root.clone(),
        base_branch: "main".to_string(),
        worktrees_root: original.path.parent().unwrap().to_path_buf(),
        max_live: None,
        idle_ttl: Duration::from_secs(3600),
    });

    let error = fresh
        .ensure_for_plan("09-missing-metadata")
        .await
        .unwrap_err();

    assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
    assert!(original.path.exists(), "unsafe candidate must be preserved");
    assert!(fresh.get("09-missing-metadata").is_none());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn ensure_for_plan_rejects_symlink_candidate() {
    let Some((tmp, mgr)) = make_manager() else {
        return;
    };
    let outside = tmp.path().join("outside-worktree");
    let expected_branch = format_branch_name("09-symlink");
    let status = StdCommand::new("git")
        .current_dir(&mgr.config.repo_root)
        .args([
            "worktree",
            "add",
            "-b",
            &expected_branch,
            outside.to_str().unwrap(),
            "main",
        ])
        .status()
        .unwrap();
    assert!(status.success());
    std::fs::create_dir_all(&mgr.config.worktrees_root).unwrap();
    std::os::unix::fs::symlink(&outside, mgr.path_for("09-symlink")).unwrap();

    let error = mgr.ensure_for_plan("09-symlink").await.unwrap_err();

    assert!(matches!(error, WorktreeError::ReattachRejected { .. }));
    assert!(outside.exists(), "target worktree must be preserved");
    assert!(mgr.get("09-symlink").is_none());
}

#[tokio::test]
async fn discover_rejects_invalid_ids_without_creating_paths() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };

    let discovered = mgr.discover_existing(&["../escape", "has/slash"]).await;

    assert!(discovered.is_empty());
    assert_eq!(mgr.active_count(), 0);
    assert!(!mgr.config.worktrees_root.exists());
}

#[tokio::test]
async fn remove_all_clears_every_tracked_worktree() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    mgr.create_for_plan("07-golf-4").await.unwrap();
    mgr.create_for_plan("07-golf-5").await.unwrap();

    let removed = mgr.remove_all().await.unwrap();
    assert_eq!(
        removed,
        vec!["07-golf-4".to_string(), "07-golf-5".to_string()]
    );
    assert_eq!(mgr.active_count(), 0);
}

#[tokio::test]
async fn budget_exhausted_when_max_live_reached() {
    let Some((_tmp, mgr)) = make_manager_with_budget(1) else {
        return;
    };
    mgr.create("a", "feature/a").await.unwrap();
    let err = mgr.create("b", "feature/b").await.unwrap_err();
    assert!(matches!(err, WorktreeError::BudgetExhausted { max: 1 }));
    // Original still intact.
    assert_eq!(mgr.list().unwrap().len(), 1);
}

#[tokio::test]
async fn concurrent_create_respects_max_live_budget() {
    let Some((_tmp, mgr)) = make_manager_with_budget(1) else {
        return;
    };
    let first_manager = mgr.clone();
    let second_manager = mgr.clone();

    let (first, second) = tokio::join!(
        first_manager.create("budget-a", "feature/budget-a"),
        second_manager.create("budget-b", "feature/budget-b")
    );

    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let error = first.err().or_else(|| second.err()).expect("one rejection");
    assert!(matches!(error, WorktreeError::BudgetExhausted { max: 1 }));
    assert_eq!(mgr.active_count(), 1);
}

#[tokio::test]
async fn touch_updates_last_active() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    let h = mgr.create("08-hotel", "feature/hotel").await.unwrap();
    let before = h.last_active_ms;
    tokio::time::sleep(Duration::from_millis(50)).await;
    mgr.touch("08-hotel");
    let listed = mgr.list().unwrap();
    let after = listed[0].last_active_ms;
    assert!(after > before, "touch should bump last_active_ms");
}

#[tokio::test]
async fn check_health_ok_for_live_worktree() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    mgr.create("09-india", "feature/india").await.unwrap();
    let health = mgr.check_health("09-india").await.unwrap();
    assert_eq!(health, WorktreeHealth::Ok);
}

#[tokio::test]
async fn check_health_missing_when_dir_deleted() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    let h = mgr.create("10-juliet", "feature/juliet").await.unwrap();
    std::fs::remove_dir_all(&h.path).unwrap();
    let health = mgr.check_health("10-juliet").await.unwrap();
    assert_eq!(health, WorktreeHealth::Missing);
}

#[tokio::test]
async fn reclaim_idle_evicts_stale_worktrees() {
    if !git_available() {
        return;
    }
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path().join("repo");
    std::fs::create_dir_all(&repo_root).unwrap();
    init_repo(&repo_root);
    // idle_ttl = 0 so every worktree is immediately reclaimable.
    let mgr = WorktreeManager::new(WorktreeConfig {
        repo_root,
        base_branch: "main".to_string(),
        worktrees_root: tmp.path().join("worktrees"),
        max_live: None,
        idle_ttl: Duration::ZERO,
    });
    mgr.create("11-kilo", "feature/kilo").await.unwrap();
    mgr.create("12-lima", "feature/lima").await.unwrap();
    // Small sleep so timestamps are in the past.
    tokio::time::sleep(Duration::from_millis(5)).await;
    let evicted = mgr.reclaim_idle().await.unwrap();
    assert_eq!(evicted.len(), 2);
    assert!(mgr.list().unwrap().is_empty());
}

#[tokio::test]
async fn clear_stale_locks_removes_old_lock_files() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    mgr.create("13-mike", "feature/mike").await.unwrap();

    // Plant a stale lock in the git worktrees metadata dir.
    let handle = mgr.get("13-mike").expect("tracked worktree");
    let lock_dir = read_gitdir(&handle.path).expect("linked worktree gitdir");
    assert!(
        lock_dir.exists(),
        "git should have created worktree metadata"
    );
    let lock_path = lock_dir.join("index.lock");
    std::fs::write(&lock_path, "").unwrap();

    // Backdate the lock to epoch so it appears stale (> 5 min).
    std::fs::File::options()
        .write(true)
        .open(&lock_path)
        .unwrap()
        .set_times(
            std::fs::FileTimes::new()
                .set_accessed(std::time::SystemTime::UNIX_EPOCH)
                .set_modified(std::time::SystemTime::UNIX_EPOCH),
        )
        .unwrap();

    let cleared = mgr.clear_stale_locks().unwrap();
    assert_eq!(cleared.len(), 1, "stale lock should be cleared");
    assert!(!lock_path.exists(), "lock file should be deleted");
}

#[tokio::test]
async fn create_clears_stale_main_repo_lock_before_git() {
    let Some((tmp, mgr)) = make_manager() else {
        return;
    };

    let repo_root = tmp.path().join("repo");
    let lock_path = repo_root.join(".git").join("index.lock");
    std::fs::write(&lock_path, "").unwrap();
    std::fs::File::options()
        .write(true)
        .open(&lock_path)
        .unwrap()
        .set_times(
            std::fs::FileTimes::new()
                .set_accessed(std::time::SystemTime::UNIX_EPOCH)
                .set_modified(std::time::SystemTime::UNIX_EPOCH),
        )
        .unwrap();

    let handle = mgr.create("14-november", "feature/november").await.unwrap();
    assert_eq!(handle.id, "14-november");
    assert!(
        !lock_path.exists(),
        "stale main repo lock should be removed"
    );
}

#[tokio::test]
async fn prune_runs_without_error() {
    let Some((_tmp, mgr)) = make_manager() else {
        return;
    };
    let result = mgr.prune().await;
    assert!(result.is_ok());
}

// ── G08 config isolation tests ─────────────────────────────────────

#[test]
fn isolate_worktree_config_copies_cursor_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    let wt = tmp.path().join("worktree");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::create_dir_all(&wt).unwrap();

    // Create .cursor/mcp.json in "repo"
    let cursor = repo.join(".cursor");
    std::fs::create_dir_all(&cursor).unwrap();
    std::fs::write(cursor.join("mcp.json"), r#"{"test": true}"#).unwrap();

    isolate_worktree_config(&repo, &wt);

    let wt_mcp = wt.join(".cursor").join("mcp.json");
    assert!(wt_mcp.exists(), "worktree should have .cursor/mcp.json");
    assert!(!wt_mcp.is_symlink(), "should be a copy, not a symlink");
    let content = std::fs::read_to_string(&wt_mcp).unwrap();
    assert_eq!(content, r#"{"test": true}"#);
}

#[test]
fn isolate_worktree_config_skips_when_no_source() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    let wt = tmp.path().join("worktree");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::create_dir_all(&wt).unwrap();

    // No .cursor/ in repo -- should not panic or create anything.
    isolate_worktree_config(&repo, &wt);

    assert!(!wt.join(".cursor").exists());
}

#[test]
fn isolate_worktree_config_skips_when_target_exists() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    let wt = tmp.path().join("worktree");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::create_dir_all(&wt).unwrap();

    // Create .cursor/ in both
    std::fs::create_dir_all(repo.join(".cursor")).unwrap();
    std::fs::write(repo.join(".cursor/mcp.json"), "original").unwrap();
    std::fs::create_dir_all(wt.join(".cursor")).unwrap();
    std::fs::write(wt.join(".cursor/mcp.json"), "existing").unwrap();

    isolate_worktree_config(&repo, &wt);

    // Should NOT overwrite
    let content = std::fs::read_to_string(wt.join(".cursor/mcp.json")).unwrap();
    assert_eq!(content, "existing");
}
