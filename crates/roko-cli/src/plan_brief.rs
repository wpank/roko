//! A plan's companion documents (gap-d6fd85, gap-c56341).
//!
//! `roko plan prepare <plan>` writes `brief.md` beside the plan's
//! `tasks.toml`: where the plan's artifacts are, a map of its tasks, and the
//! risks a reader should know about. When `[meta] source_prd` names a PRD
//! that exists, it also copies that PRD into `prd-extract.md`. Both are
//! derived from files alone; no model runs. Dispatch puts the brief into the
//! prompt of each of the plan's tasks (`dispatch::prompt_builder`).
//!
//! With `--full` ([`prepare_full`]), the planner model first writes
//! `decomposition.md`, the plan as numbered steps with checkpoints, and then
//! `rubric.md`, the criteria a review may block on and those it may not. The
//! brief lists both.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context as _, Result};
use roko_core::config::schema::RokoConfig;

use crate::agent_exec::AgentCapture;
use crate::dispatch::SharedAgentFactory;
use crate::dispatch_v2::AgentDispatchRequest;
use crate::plan_authoring::AuthoringSpend;
use crate::task_parser::{TaskDef, TaskMeta, TasksFile};

/// The plan's brief, beside its `tasks.toml`.
pub const BRIEF_FILE: &str = "brief.md";

/// The plan's source PRD, copied beside its `tasks.toml`.
pub const PRD_EXTRACT_FILE: &str = "prd-extract.md";

/// The plan as numbered steps with checkpoints, written by a model
/// (`roko plan prepare --full`).
pub const DECOMPOSITION_FILE: &str = "decomposition.md";

/// The plan's review rubric, written by a model (`roko plan prepare --full`).
pub const RUBRIC_FILE: &str = "rubric.md";

/// The documents a model writes with `--full`, each with its row in the
/// brief's artifact table.
const MODEL_DOCUMENTS: [(&str, &str); 2] = [
    ("Decomposition", DECOMPOSITION_FILE),
    ("Review rubric", RUBRIC_FILE),
];

/// How long one `--full` model call may run.
const MODEL_CALL_TIMEOUT_MS: u64 = 600_000;

/// The tools a `--full` model call may use: it reads the repository and
/// writes nothing.
const READ_ONLY_TOOLS: &str = "Read,Grep,Glob";

/// The system prompt of the model that writes `decomposition.md`.
const DECOMPOSITION_SYSTEM_PROMPT: &str = "You write decomposition.md for a roko plan: the \
     step-by-step sequence an implementer follows. Reply with the document only, in Markdown, \
     without a code fence around it.\n\n\
     Structure:\n\
     1. `# Decomposition: <plan id>`.\n\
     2. `## Preamble`: the plan's key constraints, and notes on the existing code it changes. \
     You may read the repository to ground them; change nothing.\n\
     3. `## Steps`: numbered steps, `### Step N: <title> (task <id>)`, in an order the tasks' \
     `depends_on` allows. Each step names the files it touches, what it creates or changes and \
     the action to take, and ends with a **Checkpoint**: one command, such as \
     `cargo check -p <crate>` or `test -f <path>`, that shows the step is done. Keep each step \
     atomic.\n\
     4. `## Phases`: a table of the phases, their steps and their tasks.\n\n\
     Stay within the plan. Add no task, file or requirement it does not state; where it is \
     unclear, say so in the step instead of guessing.";

/// The system prompt of the model that writes `rubric.md`.
const RUBRIC_SYSTEM_PROMPT: &str = "You write rubric.md for a roko plan: the single source of \
     truth for the implementer's self-check and for the blocking scope of its reviewers. Reply \
     with the document only, in Markdown, without a code fence around it.\n\n\
     Structure:\n\
     1. `# Review rubric: <plan id>`.\n\
     2. A table of roles and rules: the implementer confirms every blocking item before a task \
     is done; a reviewer blocks only on a violated item.\n\
     3. `## Blocking checklist`: four to eight `- [ ]` items, each objective and checkable: a \
     command that must pass (the tasks' verify steps first), an artifact that must exist, or \
     an interface the plan pins down that must match exactly.\n\
     4. `## Explicitly non-blocking`: what a reviewer must not block on, such as style the \
     configured linters accept, prose, and tests beyond those the plan names.\n\n\
     Derive every item from the plan, its tasks and the decomposition; invent no requirement.";

/// A task whose `max_loc` is above this is flagged as a large change.
const LARGE_MAX_LOC: u32 = 500;

/// What [`prepare`] did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Prepared {
    /// The companion documents it wrote.
    pub written: Vec<PathBuf>,
    /// Those it kept, because they exist and it was not asked to overwrite
    /// them.
    pub kept: Vec<PathBuf>,
}

/// Write the companion documents of the plan in `plan_dir`, whose workspace
/// is `workdir`: `brief.md`, and `prd-extract.md` when the plan names a
/// source PRD that exists under `workdir/.roko/prd/`. A document that
/// exists is kept unless `force` is set.
///
/// # Errors
///
/// Returns an error when the plan's `tasks.toml` cannot be read or parsed,
/// or a document cannot be written.
pub fn prepare(plan_dir: &Path, workdir: &Path, force: bool) -> Result<Prepared> {
    let tasks = TasksFile::parse(&plan_dir.join("tasks.toml"))?;
    let plan_md = std::fs::read_to_string(plan_dir.join("plan.md")).ok();
    let prd = tasks
        .meta
        .source_prd
        .as_deref()
        .and_then(|slug| prd_path(workdir, slug).map(|path| (slug, path)));

    let model_documents: Vec<(&str, &str)> = MODEL_DOCUMENTS
        .into_iter()
        .filter(|(_, file)| plan_dir.join(file).is_file())
        .collect();
    let brief = brief_md(
        &tasks.meta,
        &tasks.tasks,
        plan_md.as_deref(),
        prd.is_some(),
        &model_documents,
    );
    let mut documents = vec![(plan_dir.join(BRIEF_FILE), brief)];
    if let Some((slug, path)) = prd {
        let prd = std::fs::read_to_string(&path)
            .with_context(|| format!("read the source PRD {}", path.display()))?;
        let extract = prd_extract_md(&tasks.meta.plan, slug, &prd);
        documents.push((plan_dir.join(PRD_EXTRACT_FILE), extract));
    }

    let mut prepared = Prepared::default();
    for (path, text) in documents {
        if path.exists() && !force {
            prepared.kept.push(path);
            continue;
        }
        std::fs::write(&path, text).with_context(|| format!("write {}", path.display()))?;
        prepared.written.push(path);
    }
    Ok(prepared)
}

/// `roko plan prepare --full`: have a model write the plan's
/// `decomposition.md` and then its `rubric.md`, which it writes from the
/// decomposition too, and then write the documents of [`prepare`], whose brief
/// lists them. The model is `model`, or the workspace's planner model. It runs
/// through the shared agent factory, and each call's spend is recorded against
/// the plan. A document that exists is kept unless `force` is set; when both
/// model-written documents are kept, no model runs.
///
/// # Errors
///
/// Returns an error when the plan cannot be read, no planner model resolves,
/// a model call fails or returns nothing, or a document cannot be written.
pub async fn prepare_full(
    plan_dir: &Path,
    workdir: &Path,
    force: bool,
    model: Option<String>,
) -> Result<Prepared> {
    let tasks_path = plan_dir.join("tasks.toml");
    let tasks_toml = std::fs::read_to_string(&tasks_path)
        .with_context(|| format!("read {}", tasks_path.display()))?;
    let tasks = TasksFile::parse_str(&tasks_toml)
        .with_context(|| format!("parse {}", tasks_path.display()))?;
    let plan_id = tasks.meta.plan.as_str();
    let plan_md = std::fs::read_to_string(plan_dir.join("plan.md")).ok();
    let prd = tasks
        .meta
        .source_prd
        .as_deref()
        .and_then(|slug| prd_path(workdir, slug).map(|path| (slug, path)));
    let prd = match prd {
        Some((slug, path)) => {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("read the source PRD {}", path.display()))?;
            Some((slug, text))
        }
        None => None,
    };
    let prd = prd.as_ref().map(|(slug, text)| (*slug, text.as_str()));
    let sources = plan_sources(plan_md.as_deref(), &tasks_toml, prd);

    let mut prepared = Prepared::default();
    let decomposition_path = plan_dir.join(DECOMPOSITION_FILE);
    let rubric_path = plan_dir.join(RUBRIC_FILE);
    let write_decomposition = force || !decomposition_path.exists();
    let write_rubric = force || !rubric_path.exists();
    if write_decomposition || write_rubric {
        let writer = DocumentWriter::new(workdir, plan_id, model).await?;
        let decomposition = if write_decomposition {
            let prompt = format!(
                "Task: decomposition: Write {DECOMPOSITION_FILE} for plan `{plan_id}`\n\n{sources}"
            );
            let document = writer
                .write(DECOMPOSITION_FILE, DECOMPOSITION_SYSTEM_PROMPT, prompt)
                .await?;
            write_document(&decomposition_path, &document)?;
            prepared.written.push(decomposition_path);
            document
        } else {
            let document = std::fs::read_to_string(&decomposition_path)
                .with_context(|| format!("read {}", decomposition_path.display()))?;
            prepared.kept.push(decomposition_path);
            document
        };
        if write_rubric {
            let prompt = format!(
                "Task: rubric: Write {RUBRIC_FILE} for plan `{plan_id}`\n\n{sources}\n\
                 ## {DECOMPOSITION_FILE}\n\n{decomposition}"
            );
            let document = writer
                .write(RUBRIC_FILE, RUBRIC_SYSTEM_PROMPT, prompt)
                .await?;
            write_document(&rubric_path, &document)?;
            prepared.written.push(rubric_path);
        } else {
            prepared.kept.push(rubric_path);
        }
    } else {
        prepared.kept.push(decomposition_path);
        prepared.kept.push(rubric_path);
    }

    let documents = prepare(plan_dir, workdir, force)?;
    prepared.written.extend(documents.written);
    prepared.kept.extend(documents.kept);
    Ok(prepared)
}

/// The plan's own files, for a `--full` prompt: its `plan.md`, its
/// `tasks.toml`, and its source PRD, `(slug, text)`, when it has one.
fn plan_sources(plan_md: Option<&str>, tasks_toml: &str, prd: Option<(&str, &str)>) -> String {
    let mut out = String::new();
    if let Some(plan_md) = plan_md {
        let _ = write!(out, "## plan.md\n\n{plan_md}\n\n");
    }
    let _ = write!(out, "## tasks.toml\n\n```toml\n{tasks_toml}\n```\n");
    if let Some((slug, prd)) = prd {
        let _ = write!(out, "\n## Source PRD `{slug}`\n\n{prd}\n");
    }
    out
}

fn write_document(path: &Path, text: &str) -> Result<()> {
    std::fs::write(path, text).with_context(|| format!("write {}", path.display()))
}

/// The document in a model's `reply`: its text, without a code fence around
/// all of it. `None` when the reply is empty.
fn document_from_reply(reply: &str) -> Option<String> {
    let reply = reply.trim();
    let unfenced = reply
        .strip_prefix("```")
        .and_then(|rest| rest.split_once('\n'))
        .and_then(|(_, body)| body.trim_end().strip_suffix("```"))
        .map_or(reply, str::trim);
    (!unfenced.is_empty()).then(|| format!("{unfenced}\n"))
}

/// Has a model write `--full` documents through the shared agent factory,
/// and records the spend of each call against the plan.
struct DocumentWriter {
    factory: SharedAgentFactory,
    config: Arc<RokoConfig>,
    spend: AuthoringSpend,
    workdir: PathBuf,
    model_key: String,
}

impl DocumentWriter {
    /// A writer for the plan `plan_id` of the workspace `workdir`, on `model`
    /// or the workspace's planner model.
    async fn new(workdir: &Path, plan_id: &str, model: Option<String>) -> Result<Self> {
        let model_key =
            crate::model_selection::resolve_planner_model(workdir, model, "plan prepare --full")?;
        let config = roko_core::config::loader::load_config_unified(workdir)
            .with_context(|| format!("load the roko config of {}", workdir.display()))?;
        let config = Arc::new(config);
        let factory = SharedAgentFactory::new(Arc::clone(&config), None, None, None).await;
        Ok(Self {
            factory,
            config,
            spend: AuthoringSpend::generation(workdir, plan_id, None),
            workdir: workdir.to_path_buf(),
            model_key,
        })
    }

    /// The document the model writes as `file` for `prompt` under
    /// `system_prompt`, headed by a comment naming the model.
    async fn write(&self, file: &str, system_prompt: &str, prompt: String) -> Result<String> {
        let request = AgentDispatchRequest {
            model_key: self.model_key.clone(),
            prompt,
            system_prompt: system_prompt.to_string(),
            workdir: self.workdir.clone(),
            immune_root: None,
            agent_id: "plan-prepare".to_string(),
            command: None,
            timeout_ms: Some(MODEL_CALL_TIMEOUT_MS),
            mcp_config: None,
            env: self.config.agent.env.clone().unwrap_or_default(),
            extra_args: Vec::new(),
            effort: Some(self.config.agent.default_effort.clone()),
            tools: Some(READ_ONLY_TOOLS.to_string()),
            agent_contract: None,
            bare_mode: true,
            dangerously_skip_permissions: self.config.runner.dangerously_skip_permissions,
            max_turns: None,
            live_output: None,
            attempt_key: None,
        };
        let started = Instant::now();
        let dispatch = self
            .factory
            .run_shared_agent_bridge(request)
            .await
            .with_context(|| format!("dispatch the model that writes {file}"))?;
        let reply = dispatch.result.output.body.as_text().unwrap_or("");
        let capture = AgentCapture {
            exit_code: i32::from(!dispatch.result.success),
            output: reply.to_string(),
            usage: dispatch.result.usage,
            model: dispatch.target.model_slug.clone(),
            provider: dispatch.target.provider_id.clone(),
            duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        };
        self.spend.record(&capture).await;
        anyhow::ensure!(
            dispatch.result.success,
            "the model failed to write {file}: {}",
            reply.chars().take(500).collect::<String>()
        );
        let document = document_from_reply(reply)
            .with_context(|| format!("the model wrote an empty {file}"))?;
        Ok(format!(
            "<!-- Written by model `{}` for `roko plan prepare --full`. `tasks.toml` and \
             `plan.md` decide where they differ. -->\n\n{document}",
            dispatch.target.model_slug
        ))
    }
}

/// The PRD that `slug` names under `workdir/.roko/prd/`, if it exists.
fn prd_path(workdir: &Path, slug: &str) -> Option<PathBuf> {
    let prd = workdir.join(".roko").join("prd");
    let file = format!("{slug}.md");
    [
        prd.join("published").join(&file),
        prd.join("drafts").join(&file),
        prd.join("draft").join(&file),
        prd.join(&file),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

/// The plan's `brief.md`: where its artifacts are, its task map, its risk
/// flags, and the `## Quick Reference` of its `plan_md`, if it has one.
/// `prd_extract` says whether a `prd-extract.md` goes with it, and
/// `model_documents` lists, as (artifact, file), the model-written documents
/// beside it.
#[must_use]
pub fn brief_md(
    meta: &TaskMeta,
    tasks: &[TaskDef],
    plan_md: Option<&str>,
    prd_extract: bool,
    model_documents: &[(&str, &str)],
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Plan brief: `{}`\n", meta.plan);
    out.push_str(
        "Written by `roko plan prepare` from the plan's files; no model wrote it. It orients \
         you; `tasks.toml` and `plan.md` decide.\n\n",
    );

    out.push_str("## Artifacts\n\n| Artifact | Path |\n|---|---|\n| Tasks | `tasks.toml` |\n");
    if plan_md.is_some() {
        out.push_str("| Plan | `plan.md` |\n");
    }
    if prd_extract {
        let _ = writeln!(out, "| Source PRD | `{PRD_EXTRACT_FILE}` |");
    }
    for (artifact, file) in model_documents {
        let _ = writeln!(out, "| {artifact} | `{file}` |");
    }

    out.push_str("\n## Tasks\n\n| ID | Title | Role | Tier | Depends on | Files |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for task in tasks {
        let files = match task.files.len() {
            0 => "—".to_string(),
            1..=3 => task.files.join(", "),
            n => format!("{}, +{}", task.files[..3].join(", "), n - 3),
        };
        let depends_on = if task.depends_on.is_empty() {
            "—".to_string()
        } else {
            task.depends_on.join(", ")
        };
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} |",
            cell(&task.id),
            cell(&task.title),
            cell(task.role.as_deref().unwrap_or("implementer")),
            cell(&task.tier),
            cell(&depends_on),
            cell(&files)
        );
    }

    out.push_str("\n## Risks\n\n");
    let unverified: Vec<&str> = tasks
        .iter()
        .filter(|task| task.verify.is_empty() && !task.has_accept_tests())
        .map(|task| task.id.as_str())
        .collect();
    let large: Vec<&str> = tasks
        .iter()
        .filter(|task| task.max_loc.is_some_and(|loc| loc > LARGE_MAX_LOC))
        .map(|task| task.id.as_str())
        .collect();
    let mut risks = Vec::new();
    if !unverified.is_empty() {
        risks.push(format!("No verify step: {}", unverified.join(", ")));
    }
    if !large.is_empty() {
        risks.push(format!(
            "`max_loc` over {LARGE_MAX_LOC}, a large change: {}",
            large.join(", ")
        ));
    }
    if meta.skip_enrichment {
        risks.push("`skip_enrichment = true`: tasks run unenriched".to_string());
    }
    if risks.is_empty() {
        risks.push("None found.".to_string());
    }
    for risk in risks {
        let _ = writeln!(out, "- {risk}");
    }

    if let Some(quick_reference) = plan_md.and_then(quick_reference) {
        out.push_str("\n## Quick Reference (from `plan.md`)\n\n");
        out.push_str(&quick_reference);
    }
    out
}

/// The `## Quick Reference` section of `plan_md`, without its heading.
fn quick_reference(plan_md: &str) -> Option<String> {
    let (_, rest) = plan_md.split_once("## Quick Reference\n")?;
    let section = rest.split("\n## ").next().unwrap_or(rest).trim();
    (!section.is_empty()).then(|| format!("{section}\n"))
}

/// `text` as one Markdown table cell.
fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

/// The plan's `prd-extract.md`: the PRD `slug` names, under a header that
/// says where it came from.
#[must_use]
pub fn prd_extract_md(plan_id: &str, slug: &str, prd: &str) -> String {
    format!(
        "<!-- The source PRD of plan `{plan_id}` (PRD `{slug}`), copied from .roko/prd by `roko plan prepare`. \
         Edit the PRD, not this copy. -->\n\n{prd}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const TASKS: &str = r#"
[meta]
plan = "brief-demo"
source_prd = "brief-demo"
skip_enrichment = true

[[task]]
id = "T1"
title = "Parse the config"
role = "implementer"
files = ["src/config.rs"]
verify = [{ phase = "test", command = "cargo test -p demo config" }]

[[task]]
id = "T2"
title = "Document a|b"
role = "scribe"
max_loc = 800
depends_on = ["T1"]
files = ["a.md", "b.md", "c.md", "d.md"]
"#;

    fn plan(root: &Path) -> PathBuf {
        let plan_dir = root.join("plans/brief-demo");
        std::fs::create_dir_all(&plan_dir).unwrap();
        std::fs::write(plan_dir.join("tasks.toml"), TASKS).unwrap();
        std::fs::write(
            plan_dir.join("plan.md"),
            "# Plan\n\n## Quick Reference\n\nRun T1 first.\n\n## Details\n\nMore.\n",
        )
        .unwrap();
        plan_dir
    }

    /// gap-d6fd85: `prepare` writes the brief and the PRD extract once, and
    /// keeps them on a second run unless forced.
    #[test]
    fn prepare_writes_the_companion_documents_and_keeps_them() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let plan_dir = plan(root);
        std::fs::create_dir_all(root.join(".roko/prd/published")).unwrap();
        std::fs::write(
            root.join(".roko/prd/published/brief-demo.md"),
            "# PRD\n\nThe spec.\n",
        )
        .unwrap();
        let brief = plan_dir.join(BRIEF_FILE);
        let extract = plan_dir.join(PRD_EXTRACT_FILE);

        let first = prepare(&plan_dir, root, false).unwrap();
        assert_eq!(first.written, [brief.clone(), extract.clone()]);
        let text = std::fs::read_to_string(&brief).unwrap();
        assert!(text.starts_with("# Plan brief: `brief-demo`\n"), "{text}");
        assert!(text.contains("| Source PRD | `prd-extract.md` |"), "{text}");
        assert!(
            text.contains("| T1 | Parse the config | implementer | focused | — | src/config.rs |"),
            "{text}"
        );
        assert!(
            text.contains("| T2 | Document a\\|b | scribe | focused | T1 | a.md, b.md, c.md, +1 |"),
            "{text}"
        );
        assert!(text.contains("- No verify step: T2\n"), "{text}");
        assert!(text.contains("a large change: T2\n"), "{text}");
        assert!(text.contains("\n\nRun T1 first.\n"), "{text}");
        assert!(!text.contains("More."), "{text}");
        let extract_text = std::fs::read_to_string(&extract).unwrap();
        assert!(extract_text.ends_with("# PRD\n\nThe spec.\n"));

        std::fs::write(&brief, "edited").unwrap();
        let second = prepare(&plan_dir, root, false).unwrap();
        assert_eq!(second.kept, [brief.clone(), extract.clone()]);
        assert_eq!(std::fs::read_to_string(&brief).unwrap(), "edited");

        let forced = prepare(&plan_dir, root, true).unwrap();
        assert_eq!(forced.written.len(), 2);
        assert_ne!(std::fs::read_to_string(&brief).unwrap(), "edited");
    }

    /// gap-c56341: the brief lists the documents `--full` had a model write,
    /// when they are there.
    #[test]
    fn prepare_lists_the_model_written_documents() {
        let temp = tempfile::tempdir().unwrap();
        let plan_dir = plan(temp.path());
        std::fs::write(plan_dir.join(DECOMPOSITION_FILE), "# Decomposition\n").unwrap();
        std::fs::write(plan_dir.join(RUBRIC_FILE), "# Review rubric\n").unwrap();

        prepare(&plan_dir, temp.path(), false).unwrap();

        let text = std::fs::read_to_string(plan_dir.join(BRIEF_FILE)).unwrap();
        assert!(
            text.contains("| Decomposition | `decomposition.md` |"),
            "{text}"
        );
        assert!(text.contains("| Review rubric | `rubric.md` |"), "{text}");
    }

    /// A model's reply is the document; a code fence around all of it goes.
    #[test]
    fn document_from_reply_drops_a_fence_around_the_reply() {
        assert_eq!(
            document_from_reply("```markdown\n# Rubric\n\n- [ ] builds\n```\n").as_deref(),
            Some("# Rubric\n\n- [ ] builds\n")
        );
        assert_eq!(
            document_from_reply("  # Steps\n\n```sh\ncargo check\n```\n").as_deref(),
            Some("# Steps\n\n```sh\ncargo check\n```\n")
        );
        assert_eq!(document_from_reply(" \n"), None);
    }

    /// Without its source PRD, a plan gets a brief and no extract.
    #[test]
    fn prepare_writes_no_extract_without_the_source_prd() {
        let temp = tempfile::tempdir().unwrap();
        let plan_dir = plan(temp.path());

        let prepared = prepare(&plan_dir, temp.path(), false).unwrap();

        assert_eq!(prepared.written, [plan_dir.join(BRIEF_FILE)]);
        let text = std::fs::read_to_string(plan_dir.join(BRIEF_FILE)).unwrap();
        assert!(!text.contains(PRD_EXTRACT_FILE), "{text}");
    }
}
