//! B1, the hidden suite (S05 §4.4, 7124).
//!
//! A suite is written from the task's spec only: its title, description,
//! goal, acceptance criteria, `[task.hidden]` hook and the public
//! signatures of the files it names, as the base tree holds them; never the
//! diff or the agent's output. Its author is a configured model of another
//! family than the implementer's (`AuditConfig::family_of`); with none, B1
//! is null (decision 7102). The suite is drafted in the vault with its
//! canary and validated in an audit worktree of the base tree: it builds
//! there, a test of it fails there, and it gives the same verdict twice.
//! Otherwise it is rejected and B1 is null. A validated suite is active,
//! and later audits of the task reuse it until it rotates. On the result
//! tree it runs three times: failing all three gives Y = 1. The suite is
//! only ever in the vault and in audit worktrees.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use roko_core::audit_types::AuditLabels;
use roko_core::config::audit::AuditConfig;
use roko_gate::audit::hidden::{HiddenStore, SuiteDraft, SuiteMeta, SuiteState};
use roko_gate::audit::ledger::AuditLedger;
use serde_json::json;
use sha2::{Digest, Sha256};

use super::git;
use super::rerun::{Check, rerun, target_dir};
use super::worker::{AuditTask, CheckOutcome, PhaseBCheck, UnitAudit};
use super::worktree::AuditWorktree;
use crate::dispatch::SharedAgentFactory;
use crate::dispatch_v2::AgentDispatchRequest;

/// The most signature lines the prompt quotes from one file.
const MAX_SIGNATURES: usize = 40;

/// A model that writes hidden suites.
#[async_trait::async_trait]
pub trait SuiteAuthor: Send + Sync {
    /// The model's name, which names its family.
    fn model(&self) -> &str;

    /// Answer `prompt`: the reply's text and what the call cost, in USD.
    ///
    /// # Errors
    ///
    /// The call failed.
    async fn write(&self, prompt: &str) -> anyhow::Result<(String, f64)>;
}

/// A suite author that calls a configured model through the agent factory,
/// from a scratch directory away from the workspace.
pub struct FactoryAuthor {
    factory: Arc<SharedAgentFactory>,
    model_key: String,
    slug: String,
    timeout_ms: u64,
}

impl FactoryAuthor {
    /// The model `model_key` (slug `slug`), with calls of at most
    /// `timeout_ms`.
    #[must_use]
    pub const fn new(
        factory: Arc<SharedAgentFactory>,
        model_key: String,
        slug: String,
        timeout_ms: u64,
    ) -> Self {
        Self {
            factory,
            model_key,
            slug,
            timeout_ms,
        }
    }
}

#[async_trait::async_trait]
impl SuiteAuthor for FactoryAuthor {
    fn model(&self) -> &str {
        &self.slug
    }

    async fn write(&self, prompt: &str) -> anyhow::Result<(String, f64)> {
        let scratch = tempfile::tempdir()?;
        let request = AgentDispatchRequest {
            model_key: self.model_key.clone(),
            prompt: prompt.to_string(),
            system_prompt: String::new(),
            workdir: scratch.path().to_path_buf(),
            immune_root: None,
            agent_id: "roko-audit-b1".to_string(),
            command: None,
            timeout_ms: Some(self.timeout_ms),
            mcp_config: None,
            env: vec![],
            extra_args: vec![],
            effort: None,
            tools: None,
            agent_contract: None,
            bare_mode: false,
            dangerously_skip_permissions: false,
            max_turns: None,
            live_output: None,
            attempt_key: None,
        };
        let dispatch = self.factory.run_shared_agent_bridge(request).await?;
        let cost_usd = f64::from(dispatch.result.usage.cost_usd);
        anyhow::ensure!(dispatch.result.success, "the suite author's call failed");
        let text = dispatch.result.output.body.as_text();
        let text = text.map_err(|_| anyhow::anyhow!("the suite author's reply is not text"))?;
        Ok((text.to_string(), cost_usd))
    }
}

/// B1: a cross-family hidden suite, validated on the base before use.
pub struct B1 {
    authors: Vec<Arc<dyn SuiteAuthor>>,
    config: AuditConfig,
}

impl B1 {
    /// B1 with `authors` as the candidate authors, in order, and `config`'s
    /// `[audit.families]`.
    #[must_use]
    pub const fn new(authors: Vec<Arc<dyn SuiteAuthor>>, config: AuditConfig) -> Self {
        Self { authors, config }
    }

    /// The suite's verdict on the unit, or why B1 is null.
    async fn audit(&self, audit: &UnitAudit<'_>) -> anyhow::Result<CheckOutcome> {
        let unit = audit.unit;
        if unit.task.hidden_suite.as_deref() == Some("none") {
            return Ok(null("the task's [task.hidden] suite is none"));
        }
        let Some(implementer) = self.config.family_of(&unit.model) else {
            return Ok(null("the implementer's model family is unknown"));
        };
        let Some(base) = unit.base_tree.as_deref() else {
            return Ok(null("the selection names no base tree"));
        };
        let store = HiddenStore::open(audit.vault)?;
        let mut ledger = AuditLedger::open(audit.vault)?
            .with_mirror(audit.repo.join(".roko/audit/audits.jsonl"));
        store.rotate(&mut ledger)?;
        let layout = Layout::for_files(&unit.task.files);
        let reusable = store
            .active_for(&unit.task_id)?
            .filter(|meta| meta.author_family != implementer);
        let (meta, cost_usd) = if let Some(meta) = reusable {
            (meta, None)
        } else {
            let authored = self.author(audit, &store, &mut ledger, &layout, implementer, base);
            match authored.await? {
                Ok((meta, cost)) => (meta, Some(cost)),
                Err(outcome) => return Ok(outcome),
            }
        };
        let placed = place(&store, &meta, &layout, audit.worktree)?;
        let budget = audit.time_left;
        let check = Check {
            label: "b1".to_string(),
            command: layout.command(&meta.suite_id),
            timeout: budget / 5,
        };
        let target = target_dir(audit.vault);
        let outcome = rerun(audit.worktree, &[check], budget, Some(&target), None).await;
        let _ = std::fs::remove_file(&placed);
        let meta = store.record_use(&mut ledger, &meta.suite_id)?;
        Ok(CheckOutcome {
            labels: AuditLabels {
                y: outcome.y,
                ..AuditLabels::default()
            },
            cost_usd: cost_usd.unwrap_or(0.0),
            detail: json!({
                "suite_id": meta.suite_id,
                "state": meta.state.label(),
                "reused": cost_usd.is_none(),
                "author_model": meta.author_model,
                "author_family": meta.author_family,
                "implementer_family": implementer,
                "y": outcome.y,
                "flaky": outcome.flaky,
                "passes": outcome.passes,
            }),
        })
    }

    /// Have a model of another family than `implementer` write a suite for
    /// the unit's task, and validate it on `base`: the active suite and the
    /// call's cost, or B1's null outcome.
    async fn author(
        &self,
        audit: &UnitAudit<'_>,
        store: &HiddenStore,
        ledger: &mut AuditLedger,
        layout: &Layout,
        implementer: &str,
        base: &str,
    ) -> anyhow::Result<Result<(SuiteMeta, f64), CheckOutcome>> {
        let unit = audit.unit;
        let author = self.authors.iter().find_map(|author| {
            let family = self.config.family_of(author.model())?;
            (family != implementer).then_some((author, family))
        });
        let Some((author, family)) = author else {
            return Ok(Err(null("no configured model of another family")));
        };
        if audit.usd_left <= 0.0 {
            return Ok(Err(null("no audit budget is left for the author's call")));
        }
        let interface = signatures(audit.repo, base, &unit.task.files);
        let spec = prompt(&unit.task, &interface, layout);
        let (reply, cost_usd) = author.write(&spec).await?;
        let (file_name, comment) = layout.store_file();
        let draft = SuiteDraft {
            task_id: unit.task_id.clone(),
            spec_hash: format!("sha256:{}", hex(&Sha256::digest(spec.as_bytes()))),
            author_model: author.model().to_string(),
            author_family: family.to_string(),
            implementer_family: implementer.to_string(),
            file_name: file_name.to_string(),
            comment: comment.to_string(),
            body: code_of(&reply),
        };
        let meta = store.draft(ledger, draft)?;
        if let Err(why) = validate(audit, store, &meta, layout, base).await {
            store.transition(ledger, &meta.suite_id, SuiteState::Rejected, &why)?;
            return Ok(Err(CheckOutcome {
                cost_usd,
                detail: json!({
                    "suite_id": meta.suite_id,
                    "state": SuiteState::Rejected.label(),
                    "why": why,
                    "author_model": meta.author_model,
                }),
                ..CheckOutcome::default()
            }));
        }
        let id = meta.suite_id;
        store.transition(ledger, &id, SuiteState::Validated, "on_base")?;
        let meta = store.transition(ledger, &id, SuiteState::Active, "in_use")?;
        Ok(Ok((meta, cost_usd)))
    }
}

/// B1's outcome when it cannot tell: no label, and why.
fn null(why: &str) -> CheckOutcome {
    CheckOutcome {
        detail: json!({ "null": why }),
        ..CheckOutcome::default()
    }
}

#[async_trait::async_trait]
impl PhaseBCheck for B1 {
    async fn check(&self, audit: &UnitAudit<'_>) -> CheckOutcome {
        self.audit(audit)
            .await
            .unwrap_or_else(|error| CheckOutcome {
                detail: json!({ "error": error.to_string() }),
                ..CheckOutcome::default()
            })
    }
}

/// Where a suite goes in a worktree, and how it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Layout {
    /// A Rust integration test of the crate under `dir` (empty for the
    /// root package), named `package`.
    Rust {
        dir: String,
        package: Option<String>,
    },
    /// A pytest file.
    Python,
    /// A POSIX shell script.
    Shell,
}

impl Layout {
    /// The layout for a task that changes `files`.
    fn for_files(files: &[String]) -> Self {
        if let Some(rust) = files.iter().find(|file| file.ends_with(".rs")) {
            let mut parts = rust.split('/');
            return match (parts.next(), parts.next()) {
                (Some("crates"), Some(name)) => Self::Rust {
                    dir: format!("crates/{name}/"),
                    package: Some(name.to_string()),
                },
                _ => Self::Rust {
                    dir: String::new(),
                    package: None,
                },
            };
        }
        if files.iter().any(|file| file.ends_with(".py")) {
            Self::Python
        } else {
            Self::Shell
        }
    }

    /// The body's file name in the store, and the language's line comment.
    const fn store_file(&self) -> (&'static str, &'static str) {
        match self {
            Self::Rust { .. } => ("suite.rs", "//"),
            Self::Python => ("suite.py", "#"),
            Self::Shell => ("suite.sh", "#"),
        }
    }

    /// Where suite `suite_id` goes in a worktree.
    fn dest(&self, suite_id: &str) -> String {
        let name = format!("roko_hidden_{}", suite_id.replace('-', "_"));
        match self {
            Self::Rust { dir, .. } => format!("{dir}tests/{name}.rs"),
            Self::Python => format!("tests/test_{name}.py"),
            Self::Shell => format!("{name}.sh"),
        }
    }

    /// The command that runs suite `suite_id` from a worktree's root.
    fn command(&self, suite_id: &str) -> String {
        let test = format!("roko_hidden_{}", suite_id.replace('-', "_"));
        match self {
            Self::Rust {
                package: Some(package),
                ..
            } => format!("cargo test -p {package} --test {test}"),
            Self::Rust { package: None, .. } => format!("cargo test --test {test}"),
            Self::Python => format!("python3 -m pytest -q {}", self.dest(suite_id)),
            Self::Shell => format!("sh {}", self.dest(suite_id)),
        }
    }

    /// What one run showed, from its exit and its output.
    fn run(&self, success: bool, code: Option<i32>, output: &str) -> Run {
        if success {
            return Run::Passed;
        }
        let failed = match self {
            Self::Rust { .. } => output.contains("test result: FAILED"),
            Self::Python => code == Some(1),
            Self::Shell => true,
        };
        if failed { Run::Failed } else { Run::Broken }
    }

    /// What the author is asked to write.
    const fn ask(&self) -> &'static str {
        match self {
            Self::Rust { .. } => {
                "one Rust integration test file: `#[test]` functions that call the crate's \
                 public API by its crate name"
            }
            Self::Python => "one pytest file: `test_` functions that import the code under test",
            Self::Shell => {
                "one POSIX sh script, run from the repository root, that exits 0 only when the \
                 spec holds"
            }
        }
    }
}

/// One run of a suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Run {
    /// Every test passed.
    Passed,
    /// A test failed.
    Failed,
    /// The suite did not build or collect.
    Broken,
    /// It ran out of time.
    TimedOut,
}

/// Validate suite `meta` in an audit worktree of `base`: `Err` holds why it
/// is rejected.
async fn validate(
    audit: &UnitAudit<'_>,
    store: &HiddenStore,
    meta: &SuiteMeta,
    layout: &Layout,
    base: &str,
) -> Result<(), String> {
    let path = audit
        .vault
        .worktrees_dir()
        .join(format!("{}-base", audit.unit.sel_id));
    if path.exists() {
        let _ = std::fs::remove_dir_all(&path);
        let _ = git(audit.repo, &["worktree", "prune"]);
    }
    let worktree = AuditWorktree::create(audit.repo, &path, base, "roko audit base")
        .map_err(|error| format!("no worktree of the base: {error}"))?;
    place(store, meta, layout, worktree.path())
        .map_err(|error| format!("the suite was not placed: {error}"))?;
    let command = layout.command(&meta.suite_id);
    let target = target_dir(audit.vault);
    let timeout = audit.time_left / 5;
    let first = run_once(worktree.path(), &command, timeout, &target, layout).await;
    let second = run_once(worktree.path(), &command, timeout, &target, layout).await;
    match (first, second) {
        (Run::Failed, Run::Failed) => Ok(()),
        (Run::Passed, Run::Passed) => Err("it passes on the base".to_string()),
        (Run::Broken, _) | (_, Run::Broken) => Err("it does not build on the base".to_string()),
        (Run::TimedOut, _) | (_, Run::TimedOut) => Err("it timed out on the base".to_string()),
        _ => Err("it gives the base two different verdicts".to_string()),
    }
}

/// One run of `command` in `worktree`, building in `target`.
async fn run_once(
    worktree: &Path,
    command: &str,
    timeout: Duration,
    target: &Path,
    layout: &Layout,
) -> Run {
    let mut child = tokio::process::Command::new("sh");
    child
        .arg("-c")
        .arg(command)
        .current_dir(worktree)
        .env("CARGO_TARGET_DIR", target)
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);
    match tokio::time::timeout(timeout, child.output()).await {
        Ok(Ok(output)) => {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            layout.run(output.status.success(), output.status.code(), &text)
        }
        Ok(Err(_)) => Run::Broken,
        Err(_) => Run::TimedOut,
    }
}

/// Copy suite `meta`'s body to its place in `worktree`; returns the path.
fn place(
    store: &HiddenStore,
    meta: &SuiteMeta,
    layout: &Layout,
    worktree: &Path,
) -> std::io::Result<PathBuf> {
    let dest = worktree.join(layout.dest(&meta.suite_id));
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(store.body_path(meta), &dest)?;
    Ok(dest)
}

/// The author's prompt: the task's spec and the public signatures it
/// starts from, never the diff or the agent's output.
fn prompt(task: &AuditTask, signatures: &str, layout: &Layout) -> String {
    let mut prompt = String::from(
        "You write a hidden acceptance suite for a software task. You see the task's spec and \
         the public interface it starts from, never anyone's implementation.\n\n",
    );
    let _ = writeln!(prompt, "Task: {}", task.title);
    for (heading, text) in [("", &task.description), ("Goal: ", &task.goal)] {
        if !text.trim().is_empty() {
            let _ = writeln!(prompt, "{heading}{}", text.trim());
        }
    }
    for (heading, items) in [
        ("Acceptance criteria", &task.acceptance),
        ("Public surface the suite calls", &task.interface),
        ("Properties the suite checks", &task.properties),
    ] {
        if !items.is_empty() {
            let _ = writeln!(prompt, "\n{heading}:");
            for item in items {
                let _ = writeln!(prompt, "- {item}");
            }
        }
    }
    if !signatures.is_empty() {
        let _ = write!(prompt, "\nPublic signatures at the start:\n{signatures}");
    }
    let _ = write!(
        prompt,
        "\nWrite {}. Test the behaviour the spec names, so that the suite fails on code that \
         lacks it. Reply with the file's contents only, in one fenced code block.",
        layout.ask()
    );
    prompt
}

/// The lines of `files` in `tree` that declare public items, at most
/// [`MAX_SIGNATURES`] per file.
fn signatures(repo: &Path, tree: &str, files: &[String]) -> String {
    const STARTS: [&str; 10] = [
        "pub fn ",
        "pub async fn ",
        "pub struct ",
        "pub enum ",
        "pub trait ",
        "pub type ",
        "pub const ",
        "def ",
        "async def ",
        "class ",
    ];
    let mut signatures = String::new();
    for file in files {
        let Ok(text) = git(repo, &["show", &format!("{tree}:{file}")]) else {
            continue;
        };
        let lines: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|line| STARTS.iter().any(|start| line.starts_with(start)))
            .take(MAX_SIGNATURES)
            .collect();
        if !lines.is_empty() {
            let _ = writeln!(signatures, "{file}:\n{}", lines.join("\n"));
        }
    }
    signatures
}

/// The first fenced code block of `reply`, or all of it.
fn code_of(reply: &str) -> String {
    let Some(start) = reply.find("```") else {
        return format!("{}\n", reply.trim());
    };
    let fenced = &reply[start + 3..];
    let body = fenced.split_once('\n').map_or("", |(_, body)| body);
    let end = body.find("```").unwrap_or(body.len());
    format!("{}\n", body[..end].trim_end())
}

/// Lower-case hex of `bytes`.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use roko_core::audit_home::AuditVault;

    use super::*;
    use crate::audit::worker::AuditUnit;
    use crate::audit::worktree::tests::{repo_with, tree_of, write};

    /// A scripted author: it answers with its next suite, and counts calls.
    struct Scripted {
        model: &'static str,
        suites: parking_lot::Mutex<VecDeque<&'static str>>,
        calls: AtomicUsize,
    }

    impl Scripted {
        fn new(model: &'static str, suites: &[&'static str]) -> Arc<Self> {
            Arc::new(Self {
                model,
                suites: parking_lot::Mutex::new(suites.iter().copied().collect()),
                calls: AtomicUsize::new(0),
            })
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::Relaxed)
        }
    }

    #[async_trait::async_trait]
    impl SuiteAuthor for Scripted {
        fn model(&self) -> &str {
            self.model
        }

        async fn write(&self, prompt: &str) -> anyhow::Result<(String, f64)> {
            assert!(
                !prompt.contains("echo 43"),
                "the author never sees the attempt"
            );
            self.calls.fetch_add(1, Ordering::Relaxed);
            let suite = self.suites.lock().pop_front();
            let suite = suite.ok_or_else(|| anyhow::anyhow!("no suite left"))?;
            Ok((format!("Here it is:\n```sh\n{suite}\n```\n"), 0.01))
        }
    }

    #[tokio::test]
    async fn b1_suite_from_another_family_is_validated_before_use() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        let base = repo_with(&repo, &[("src/answer.sh", "echo 41\n")]);
        // The attempt prints the wrong answer.
        write(&repo, "src/answer.sh", "echo 43\n");
        let result = tree_of(&repo);
        let home = temp.path().join("vault");
        let vault = AuditVault::resolve_with(&repo, Some(&home), None).expect("a vault");
        let path = vault.worktrees_dir().join("sel-1");
        let worktree = AuditWorktree::create(&repo, &path, &result, "audit").expect("a worktree");
        let unit = AuditUnit {
            sel_id: "sel-1".to_string(),
            attempt_key: "run-1:plan:T1:1".to_string(),
            run_id: "run-1".to_string(),
            plan_id: "plan".to_string(),
            task_id: "T1".to_string(),
            pi: 0.5,
            base_tree: Some(base),
            result_tree: Some(result),
            model: "claude-sonnet-4-6".to_string(),
            prediction_id: None,
            task: AuditTask {
                title: "`sh src/answer.sh` prints the answer".to_string(),
                acceptance: vec!["it prints 42".to_string()],
                files: vec!["src/answer.sh".to_string()],
                kind: "code".to_string(),
                ..AuditTask::default()
            },
        };
        let audit = UnitAudit {
            unit: &unit,
            repo: &repo,
            worktree: worktree.path(),
            vault: &vault,
            findings: &[],
            phase_a: AuditLabels::default(),
            usd_left: 0.30,
            time_left: Duration::from_secs(120),
        };
        let same = Scripted::new("claude-haiku-4-5", &["exit 0"]);
        let other = Scripted::new(
            "glm-4.6",
            &[
                "[ \"$(sh src/answer.sh)\" -gt 0 ]",
                "[ \"$(sh src/answer.sh)\" = 42 ]",
            ],
        );
        let authors: Vec<Arc<dyn SuiteAuthor>> = vec![same.clone(), other.clone()];
        let b1 = B1::new(authors, AuditConfig::default());
        let store = HiddenStore::open(&vault).expect("the store");

        // A suite that passes on the base is rejected, and B1 is null.
        let rejected = b1.check(&audit).await;
        assert_eq!(rejected.labels.y, None, "{}", rejected.detail);
        assert_eq!(rejected.detail["state"], "rejected", "{}", rejected.detail);
        assert_eq!(same.calls(), 0, "never a same-family author");
        assert_eq!(other.calls(), 1);
        let states: Vec<SuiteState> = store
            .all()
            .expect("the suites")
            .iter()
            .map(|meta| meta.state)
            .collect();
        assert_eq!(states, [SuiteState::Rejected]);

        // A discriminating suite is stored active, with its canary, and the
        // failing artifact gets Y = 1.
        let failed = b1.check(&audit).await;
        assert_eq!(failed.labels.y, Some(true), "{}", failed.detail);
        let active = store
            .active_for("T1")
            .expect("the store")
            .expect("an active suite");
        assert_eq!(active.author_family, "zhipu");
        assert_eq!(active.implementer_family, "anthropic");
        let body = std::fs::read_to_string(store.body_path(&active)).expect("the body");
        let first = format!("# {}\n", active.canary);
        assert!(body.starts_with(&first), "{body}");
        let placed = worktree.path().join(Layout::Shell.dest(&active.suite_id));
        assert!(!placed.exists(), "the suite leaves the worktree");

        // A later audit of the task reuses it without a call.
        let again = b1.check(&audit).await;
        assert_eq!(again.labels.y, Some(true), "{}", again.detail);
        assert_eq!(again.detail["reused"], true);
        assert_eq!(other.calls(), 2);
        assert_eq!(store.meta(&active.suite_id).expect("its meta").uses, 2);
        worktree.remove().expect("remove");
    }
}
