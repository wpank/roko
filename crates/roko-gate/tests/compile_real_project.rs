//! Integration test: `CompileGate` against a real cargo project.
//!
//! Creates a tiny cargo crate in a tempdir, runs `CompileGate` against it,
//! verifies the verdict. Then corrupts the source and runs again to verify
//! the failure path works too.

use async_trait::async_trait;
use roko_core::{Body, Context, Kind, Signal, Store, Verify};
use roko_gate::{BuildSystem, CompileGate, GatePayload};
use roko_std::MemorySubstrate;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tempfile::TempDir;
use tokio::fs;

async fn scaffold_cargo_project(root: &Path, lib_rs: &str) {
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "roko_gate_fixture"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .await
    .unwrap();
    fs::create_dir_all(root.join("src")).await.unwrap();
    fs::write(root.join("src/lib.rs"), lib_rs).await.unwrap();
}

fn payload_signal(working_dir: &Path) -> Signal {
    let payload = GatePayload::in_dir(working_dir).with_label("test-fixture");
    Signal::builder(Kind::Task)
        .body(Body::from_json(&payload).unwrap())
        .build()
}

#[tokio::test]
async fn compile_gate_passes_on_valid_project() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "pub fn greet() -> &'static str { \"hello\" }\n").await;

    let gate = CompileGate::new(BuildSystem::Cargo).with_timeout_ms(120_000);
    let signal = payload_signal(tmp.path());

    let verdict = gate.verify(&signal, &Context::now()).await;

    assert!(
        verdict.passed,
        "valid project should pass; reason={} detail={:?}",
        verdict.reason, verdict.detail
    );
    assert_eq!(verdict.gate, "compile:cargo");
    assert!(verdict.duration_ms > 0);
}

#[tokio::test]
async fn compile_gate_fails_on_syntax_error() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "pub fn broken() -> { missing body }\n").await;

    let gate = CompileGate::new(BuildSystem::Cargo).with_timeout_ms(120_000);
    let signal = payload_signal(tmp.path());

    let verdict = gate.verify(&signal, &Context::now()).await;

    assert!(!verdict.passed, "syntax error should fail the gate");
    assert!(
        verdict.reason.contains("error"),
        "reason should mention error: {}",
        verdict.reason
    );
    // Detail should contain stderr with the actual rustc error.
    let detail = verdict.detail.as_deref().unwrap_or("");
    assert!(
        detail.contains("error"),
        "detail should contain compiler error: {detail}"
    );
}

/// Run git in `dir`, failing the test when it fails.
fn git(dir: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Where [`a_shared_target_dir_does_not_pass_another_checkouts_build`] tells
/// its child test process to make the repository.
const SHARED_TARGET_CHILD: &str = "ROKO_GATE_SHARED_TARGET_CHILD";

/// bug-be2a53: the gates of two checkouts of one repository inherit one
/// `CARGO_TARGET_DIR`, the main checkout's `target/`, as when it is exported
/// for every worktree. Attempt `b` broke the crate before attempt `a`'s gate
/// built `a`'s valid edit, so `b`'s sources are older than that build. Cargo
/// names the crate's build alike in both checkouts, so it took `a`'s build as
/// fresh for `b`, and the gate passed code that does not compile. Each gate
/// builds in its own checkout's target dir instead, and fails `b`.
///
/// The gate inherits roko's own environment, so the shared target dir is set
/// for a child test process.
#[test]
fn a_shared_target_dir_does_not_pass_another_checkouts_build() {
    let root = TempDir::new().expect("tempdir");
    let repo = root.path().join("repo");
    let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .args(["--exact", "shared_target_dir_child", "--nocapture"])
        .env(SHARED_TARGET_CHILD, &repo)
        .env("CARGO_TARGET_DIR", repo.join("target"))
        .output()
        .expect("run the child test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(
        stdout.contains("1 passed"),
        "the child test did not run: {stdout}"
    );
}

/// Run by [`a_shared_target_dir_does_not_pass_another_checkouts_build`] only.
#[tokio::test]
async fn shared_target_dir_child() {
    let Some(repo) = std::env::var_os(SHARED_TARGET_CHILD).map(PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&repo).expect("repo dir");
    scaffold_cargo_project(&repo, "pub fn greet() -> &'static str { \"hello\" }\n").await;
    git(&repo, &["init", "--quiet"]);
    git(&repo, &["add", "-A"]);
    git(
        &repo,
        &[
            "-c",
            "user.name=roko",
            "-c",
            "user.email=roko@localhost",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "seed",
        ],
    );
    // Two attempt worktrees, where a plan run puts them.
    let worktrees = repo.join(".roko").join("worktrees");
    let (a, b) = (worktrees.join("a"), worktrees.join("b"));
    for checkout in [&a, &b] {
        let path = checkout.to_str().expect("a UTF-8 path");
        git(&repo, &["worktree", "add", "--quiet", "--detach", path]);
    }
    // Attempt b breaks the crate first. Dating its edit an hour back stands
    // for "before a's build" on file systems with coarse timestamps too.
    let broken = b.join("src/lib.rs");
    std::fs::write(&broken, "pub fn greet() -> &'static str { missing }\n").expect("b's edit");
    let earlier = SystemTime::now() - Duration::from_secs(3600);
    std::fs::File::options()
        .write(true)
        .open(&broken)
        .and_then(|file| file.set_modified(earlier))
        .expect("date b's edit");
    std::fs::write(
        a.join("src/lib.rs"),
        "pub fn greet() -> &'static str { \"hi\" }\n",
    )
    .expect("a's edit");
    let gate = CompileGate::new(BuildSystem::Cargo).with_timeout_ms(120_000);
    let built = gate.verify(&payload_signal(&a), &Context::now()).await;
    assert!(
        built.passed,
        "attempt a compiles: reason={} detail={:?}",
        built.reason, built.detail
    );

    let verdict = gate.verify(&payload_signal(&b), &Context::now()).await;

    assert!(
        !verdict.passed,
        "attempt b's gate passed code that does not compile, on attempt a's build: {:?}",
        verdict.detail
    );
    let detail = verdict.detail.as_deref().unwrap_or("");
    assert!(detail.contains("missing"), "b's own error: {detail}");
    // Each attempt built in its own checkout's target dir, not the shared one.
    for checkout in [&a, &b] {
        assert!(
            checkout.join("target").is_dir(),
            "{} has no target dir of its own",
            checkout.display()
        );
    }
}

#[tokio::test]
async fn compile_gate_fails_on_missing_dir() {
    let gate = CompileGate::new(BuildSystem::Cargo).with_timeout_ms(10_000);
    let payload = GatePayload::in_dir("/nonexistent/path/xyz");
    let signal = Signal::builder(Kind::Task)
        .body(Body::from_json(&payload).unwrap())
        .build();

    let verdict = gate.verify(&signal, &Context::now()).await;
    assert!(!verdict.passed);
}

#[tokio::test]
async fn compile_gate_rejects_malformed_payload() {
    let gate = CompileGate::new(BuildSystem::Cargo);
    // A signal with no body, or a non-GatePayload body, should be rejected.
    let signal = Signal::builder(Kind::Task)
        .body(Body::text("not a payload"))
        .build();

    let verdict = gate.verify(&signal, &Context::now()).await;
    assert!(!verdict.passed);
    assert!(verdict.reason.contains("not a GatePayload"));
}

/// End-to-end: gate → verdict → persist as Signal → re-query.
///
/// This demonstrates the architectural flow: a gate's verdict becomes a
/// signal that lives in a substrate, ready to be consumed by policies.
#[tokio::test]
async fn verdict_flows_back_into_substrate_as_signal() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "pub fn x() -> i32 { 1 }\n").await;

    let substrate = MemorySubstrate::named("verdicts");
    let gate = CompileGate::new(BuildSystem::Cargo).with_timeout_ms(120_000);

    // 1. Task signal.
    let task_signal = payload_signal(tmp.path());
    substrate.put(task_signal.clone()).await.unwrap();

    // 2. Verify verifies.
    let verdict = gate.verify(&task_signal, &Context::now()).await;
    assert!(verdict.passed);

    // 3. Wrap the verdict as a GateVerdict signal derived from the task.
    let verdict_signal = task_signal
        .derive(Kind::GateVerdict, Body::from_json(&verdict).unwrap())
        .tag("gate", &verdict.gate)
        .tag("passed", &verdict.passed.to_string())
        .build();
    substrate.put(verdict_signal.clone()).await.unwrap();

    // 4. Query for gate verdicts — the verdict is now substrate-queryable.
    let verdicts = substrate
        .query(
            &roko_core::Query::of_kind(Kind::GateVerdict),
            &Context::now(),
        )
        .await
        .unwrap();

    assert_eq!(verdicts.len(), 1);
    assert_eq!(verdicts[0].lineage, vec![task_signal.id]);
    assert_eq!(verdicts[0].tag("passed"), Some("true"));
}

/// A wrapper that upgrades a Verify into a Store-writing policy — showing
/// how gates and substrates naturally compose.
struct PersistingGate<G: Verify> {
    inner: G,
    substrate: std::sync::Arc<dyn Store>,
}

impl<G: Verify> roko_core::Cell for PersistingGate<G> {
    fn cell_id(&self) -> &str {
        "persisting-gate"
    }

    fn cell_name(&self) -> &str {
        "PersistingGate"
    }

    fn protocols(&self) -> Vec<roko_core::ProtocolId> {
        vec![roko_core::ProtocolId::Verify]
    }

    fn capabilities(&self) -> roko_core::Capabilities {
        let mut capabilities = self.inner.capabilities();
        capabilities.store_write = true;
        capabilities
    }
}

#[async_trait]
impl<G: Verify> Verify for PersistingGate<G> {
    async fn verify(&self, signal: &Signal, ctx: &Context) -> roko_core::Verdict {
        let verdict = self.inner.verify(signal, ctx).await;
        let verdict_signal = signal
            .derive(
                Kind::GateVerdict,
                Body::from_json(&verdict).unwrap_or(Body::Empty),
            )
            .tag("gate", &verdict.gate)
            .tag("passed", &verdict.passed.to_string())
            .build();
        let _ = self.substrate.put(verdict_signal).await;
        verdict
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}

#[tokio::test]
async fn gate_composes_with_substrate_as_adapter() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "pub fn y() -> bool { true }\n").await;

    let substrate: std::sync::Arc<dyn Store> = std::sync::Arc::new(MemorySubstrate::new());
    let gate = PersistingGate {
        inner: CompileGate::new(BuildSystem::Cargo).with_timeout_ms(120_000),
        substrate: substrate.clone(),
    };

    let signal = payload_signal(tmp.path());
    let verdict = gate.verify(&signal, &Context::now()).await;
    assert!(verdict.passed);

    // The gate persisted its own verdict as a signal.
    let stored = substrate
        .query(
            &roko_core::Query::of_kind(Kind::GateVerdict),
            &Context::now(),
        )
        .await
        .unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].tag("gate"), Some("compile:cargo"));
}
