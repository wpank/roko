//! A plan's companion documents (gap-d6fd85).
//!
//! `roko plan prepare <plan>` writes `brief.md` beside the plan's
//! `tasks.toml`: where the plan's artifacts are, a map of its tasks, and the
//! risks a reader should know about. When `[meta] source_prd` names a PRD
//! that exists, it also copies that PRD into `prd-extract.md`. Both are
//! derived from files alone; no model runs. Dispatch puts the brief into the
//! prompt of each of the plan's tasks (`dispatch::prompt_builder`).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};

use crate::task_parser::{TaskDef, TaskMeta, TasksFile};

/// The plan's brief, beside its `tasks.toml`.
pub const BRIEF_FILE: &str = "brief.md";

/// The plan's source PRD, copied beside its `tasks.toml`.
pub const PRD_EXTRACT_FILE: &str = "prd-extract.md";

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

    let brief = brief_md(&tasks.meta, &tasks.tasks, plan_md.as_deref(), prd.is_some());
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
/// `prd_extract` says whether a `prd-extract.md` goes with it.
#[must_use]
pub fn brief_md(
    meta: &TaskMeta,
    tasks: &[TaskDef],
    plan_md: Option<&str>,
    prd_extract: bool,
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
