//! `roko backlog import` — batch import backlog specs as PRD ideas.
//!
//! Reads markdown specs from `tmp/backlog/<N>-*.md` files and creates PRD
//! ideas in `.roko/prd/ideas/`. Optionally chains through draft, plan, and
//! execution steps.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::{BacklogCmd, Cli, resolve_workdir};

/// Dispatch backlog subcommands.
pub(crate) async fn cmd_backlog(cli: &Cli, cmd: BacklogCmd) -> Result<i32> {
    match cmd {
        BacklogCmd::Import {
            path,
            draft,
            plan,
            execute,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_backlog_import(&wd, &path, draft, plan, execute).await
        }
        BacklogCmd::List { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_backlog_list(&wd)
        }
    }
}

/// List backlog items and their import status.
fn cmd_backlog_list(workdir: &Path) -> Result<i32> {
    let backlog_dir = workdir.join("tmp/backlog");
    if !backlog_dir.is_dir() {
        println!("No backlog directory found at {}", backlog_dir.display());
        return Ok(0);
    }

    let mut entries: Vec<(u32, String, PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(&backlog_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "md")
            && let Some(fname) = path.file_stem().and_then(|s| s.to_str())
        {
            // Skip the index file
            if fname == "00-INDEX" {
                continue;
            }
            // Parse leading number
            if let Some(num) = fname.split('-').next().and_then(|n| n.parse::<u32>().ok()) {
                entries.push((num, fname.to_string(), path.clone()));
            }
        }
    }

    entries.sort_by_key(|(num, _, _)| *num);

    // Check which have been imported
    let ideas_dir = workdir.join(".roko/prd/ideas");
    let ideas_exist = ideas_dir.is_dir();

    println!("Backlog specs ({} items):", entries.len());
    println!("{:<6} {:<50} {}", "ID", "Slug", "Status");
    println!("{}", "-".repeat(70));

    for (num, slug, _path) in &entries {
        let imported = if ideas_exist {
            // Check if an idea file references this backlog number
            has_imported_idea(&ideas_dir, *num)
        } else {
            false
        };
        let status = if imported { "imported" } else { "-" };
        println!("#{:<5} {:<50} {}", num, slug, status);
    }

    Ok(0)
}

/// Check if an idea referencing a backlog number already exists.
fn has_imported_idea(ideas_dir: &Path, backlog_num: u32) -> bool {
    if let Ok(content) = std::fs::read_to_string(ideas_dir.join("ideas.md")) {
        let marker = format!("[backlog#{}]", backlog_num);
        return content.contains(&marker);
    }
    false
}

/// Import backlog spec(s) as PRD ideas.
async fn cmd_backlog_import(
    workdir: &Path,
    path: &Path,
    draft: bool,
    plan: bool,
    execute: bool,
) -> Result<i32> {
    let files = collect_backlog_files(workdir, path)?;

    if files.is_empty() {
        println!("No backlog spec files found at {}", path.display());
        return Ok(1);
    }

    println!("Importing {} backlog spec(s)...\n", files.len());

    let mut imported = 0;
    let mut skipped = 0;

    for (num, slug, filepath) in &files {
        // Read the spec title from the first heading
        let content = std::fs::read_to_string(filepath)
            .with_context(|| format!("read {}", filepath.display()))?;
        let title = extract_title(&content).unwrap_or_else(|| slug.clone());

        // Create the PRD idea
        let idea_text = format!("[backlog#{}] {}", num, title);
        match roko_cli::prd::cmd_idea(workdir, &idea_text) {
            Ok(()) => {
                imported += 1;
                println!("  #{}: {}", num, title);
            }
            Err(e) => {
                eprintln!("  #{}: failed: {}", num, e);
                skipped += 1;
                continue;
            }
        }

        if draft || plan || execute {
            println!(
                "    note: --draft/--plan/--execute require agent dispatch; \
                 use `roko prd draft new` or `roko develop` for each imported idea"
            );
        }
    }

    println!(
        "\nImported: {}, Skipped: {}, Total: {}",
        imported,
        skipped,
        files.len()
    );

    if imported > 0 {
        crate::commands::util::print_next_step_hint(
            "Next: roko prd list (or roko develop 'your idea' to plan+execute)",
        );
    }

    Ok(0)
}

/// Collect backlog files from a path (single file or directory).
fn collect_backlog_files(workdir: &Path, path: &Path) -> Result<Vec<(u32, String, PathBuf)>> {
    let resolved = if path.is_relative() {
        workdir.join(path)
    } else {
        path.to_path_buf()
    };

    let mut files = Vec::new();

    if resolved.is_file() {
        if let Some(parsed) = parse_backlog_filename(&resolved) {
            files.push(parsed);
        }
    } else if resolved.is_dir() {
        for entry in std::fs::read_dir(&resolved)? {
            let entry = entry?;
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "md")
                && let Some(parsed) = parse_backlog_filename(&p)
            {
                files.push(parsed);
            }
        }
        files.sort_by_key(|(num, _, _)| *num);
    }

    Ok(files)
}

/// Parse a backlog filename like `65-cli-verb-consolidation.md` -> (65, "cli-verb-consolidation", path).
fn parse_backlog_filename(path: &Path) -> Option<(u32, String, PathBuf)> {
    let stem = path.file_stem()?.to_str()?;
    if stem == "00-INDEX" {
        return None;
    }
    let dash_pos = stem.find('-')?;
    let num: u32 = stem[..dash_pos].parse().ok()?;
    let slug = stem[dash_pos + 1..].to_string();
    Some((num, slug, path.to_path_buf()))
}

/// Extract the title from a markdown file (first # heading).
fn extract_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            return Some(heading.trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_backlog_filename() {
        let path = PathBuf::from("tmp/backlog/65-cli-verb-consolidation.md");
        let (num, slug, _) = parse_backlog_filename(&path).unwrap();
        assert_eq!(num, 65);
        assert_eq!(slug, "cli-verb-consolidation");
    }

    #[test]
    fn test_parse_index_file_returns_none() {
        let path = PathBuf::from("tmp/backlog/00-INDEX.md");
        assert!(parse_backlog_filename(&path).is_none());
    }

    #[test]
    fn test_extract_title() {
        let content = "# CLI Verb Consolidation\n\nReduce verb sprawl...";
        assert_eq!(
            extract_title(content),
            Some("CLI Verb Consolidation".to_string())
        );
    }

    #[test]
    fn test_extract_title_no_heading() {
        let content = "No heading here\nJust text";
        assert_eq!(extract_title(content), None);
    }
}
