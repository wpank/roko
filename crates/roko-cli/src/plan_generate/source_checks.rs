//! The checks a plan's source states (gap-dc5d7c), so that generation can
//! ask the planner to keep each one as a verify step or say why it drops it,
//! and report what became of each.
//!
//! A check is a list item, a table row, an invariant block (an
//! `<!-- INV-1: … -->` line and the fields listed under it) or a command of a
//! shell block, under a heading such as Verification, Acceptance (Criteria),
//! Invariants, Testing Checkpoint or Checklist; or a `CHECKPOINT: …` line
//! anywhere. Prose is not a check, and neither is what a heading of another
//! kind holds, even under a Verification heading (Regression Anchors,
//! Academic References). Checks are numbered `SC1`, `SC2`, … in the order the
//! source states them. A line that names a check as the plan's own ("`tools/check.sh` is
//! the plan's final check") is one too, wherever it stands: `[[meta.verify]]` runs it, not
//! a task (bug-96115c).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Component, Path};
use std::sync::LazyLock;

use regex::Regex;
use roko_gate::spec_quality::ac_id;
use roko_learn::runtime_feedback::{GenerationSourceCheck, GenerationSourceChecks};

use super::runnable::{Probe, Unchecked, Unrunnable, check_command, command_words, only_reads};
use crate::task_parser::{TaskDef, TasksFile, VerifyStep};

/// The most of one check that a prompt or a report quotes, in characters.
const CHECK_CHARS: usize = 300;

/// Words that make a heading a check heading, as its first or last word.
const CHECK_HEADING_WORDS: &[&str] = &[
    "acceptance",
    "checklist",
    "checkpoint",
    "checkpoints",
    "checks",
    "criteria",
    "invariant",
    "invariants",
    "verification",
    "verify",
];

/// Shell block tags whose lines are commands.
const SHELL_TAGS: &[&str] = &["", "bash", "console", "sh", "shell", "zsh"];

/// Extensions of the scripts a check may quote by path (`tools/spec.sh`).
const SCRIPT_EXTENSIONS: &[&str] = &["bash", "pl", "py", "rb", "sh", "zsh"];

/// One check the source states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourceCheck {
    /// `SC1`, `SC2`, … in source order.
    pub id: String,
    /// The check, on one line.
    pub text: String,
    /// The command that runs it, when roko can tell: the unittest ids it
    /// names, expanded by roko rather than left to the planner (bug-8fe462),
    /// or the one command it quotes (bug-3cdc58).
    pub run: Option<String>,
    /// Whether the source names it as the plan's own final check, which
    /// `[[meta.verify]]` runs once every task has passed (bug-96115c).
    pub plan_level: bool,
}

/// The checks `source` states, numbered in order.
pub(super) fn source_checks(source: &str) -> Vec<SourceCheck> {
    let mut reader = Reader::default();
    let lines: Vec<&str> = source.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        reader.line(line, lines.get(index + 1).copied());
    }
    reader.finish();
    reader
        .checks
        .into_iter()
        .enumerate()
        .map(|(index, text)| SourceCheck {
            id: format!("SC{}", index + 1),
            text,
            run: None,
            plan_level: reader.plan_checks.contains(&index),
        })
        .collect()
}

/// A fenced block being read.
struct Fence {
    /// The fence that closes it: three or more backticks or tildes.
    marker: String,
    /// Whether its lines are commands to collect.
    collect: bool,
    lines: Vec<String>,
}

/// The source, read line by line.
#[derive(Default)]
struct Reader {
    checks: Vec<String>,
    /// Whether the latest heading is a check heading.
    collecting: bool,
    fence: Option<Fence>,
    /// The list item or invariant block being read, and the indent of the
    /// item (`None` for an invariant block, whose items are its details).
    open: Option<(String, Option<usize>)>,
    /// The indices of the checks the source names as the plan's own.
    plan_checks: BTreeSet<usize>,
}

impl Reader {
    fn line(&mut self, line: &str, next: Option<&str>) {
        let trimmed = line.trim();
        if let Some(fence) = &mut self.fence {
            if trimmed.starts_with(fence.marker.as_str())
                && trimmed.trim_start_matches(['`', '~']).trim().is_empty()
            {
                let fence = self.fence.take().expect("an open fence");
                if fence.collect {
                    self.checks.extend(shell_checks(&fence.lines));
                }
            } else {
                fence.lines.push(line.to_string());
            }
            return;
        }
        if let Some(marker) = fence_marker(trimmed) {
            self.close();
            let tag = trimmed.trim_start_matches(['`', '~']).trim().to_lowercase();
            self.fence = Some(Fence {
                marker,
                collect: self.collecting && SHELL_TAGS.contains(&tag.as_str()),
                lines: Vec::new(),
            });
            return;
        }
        if let Some(title) = heading(line) {
            self.close();
            self.collecting = is_check_heading(title);
            return;
        }
        if let Some(text) = checkpoint_line(trimmed) {
            self.close();
            self.checks.push(one_line(&text));
            return;
        }
        if let Some(text) = plan_check_line(line) {
            self.close();
            self.plan_checks.insert(self.checks.len());
            self.checks.push(text);
            return;
        }
        if !self.collecting {
            return;
        }
        if trimmed.is_empty() {
            self.close();
        } else if let Some(text) = html_comment(trimmed) {
            self.close();
            self.open = Some((text.to_string(), None));
        } else if let Some((indent, text)) = list_item(line) {
            match &mut self.open {
                // An invariant's fields, or a nested item: details.
                Some((check, None)) => detail(check, text),
                Some((check, Some(open))) if indent > *open => detail(check, text),
                _ => {
                    self.close();
                    self.open = Some((text.to_string(), Some(indent)));
                }
            }
        } else if trimmed.starts_with('|') {
            self.close();
            let header = next.is_some_and(|next| is_table_separator(next.trim()));
            if !header && !is_table_separator(trimmed) {
                self.checks
                    .push(one_line(&table_cells(trimmed).join(" | ")));
            }
        } else if line.starts_with([' ', '\t'])
            && let Some((check, Some(_))) = &mut self.open
        {
            // A list item's continuation line.
            check.push(' ');
            check.push_str(trimmed);
        } else {
            // Prose ends a list.
            self.close();
        }
    }

    /// Keep the list item or invariant block being read.
    fn close(&mut self) {
        if let Some((check, _)) = self.open.take() {
            let check = one_line(&check);
            if !check.is_empty() {
                self.checks.push(check);
            }
        }
    }

    fn finish(&mut self) {
        self.close();
        if let Some(fence) = self.fence.take()
            && fence.collect
        {
            self.checks.extend(shell_checks(&fence.lines));
        }
    }
}

/// Whether a heading's `title` names a section of checks: its first or last
/// word is one of [`CHECK_HEADING_WORDS`], or it says "done when". A title
/// with code in it names a file or a symbol.
fn is_check_heading(title: &str) -> bool {
    if title.contains('`') {
        return false;
    }
    let lower = title.to_lowercase();
    if lower.contains("done when") || lower.contains("definition of done") {
        return true;
    }
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let named = |word: &&str| CHECK_HEADING_WORDS.contains(word);
    words.first().is_some_and(named) || words.last().is_some_and(named)
}

/// A Markdown heading's title.
fn heading(line: &str) -> Option<&str> {
    let rest = line.trim_start_matches('#');
    let level = line.len() - rest.len();
    ((1..=6).contains(&level) && rest.starts_with([' ', '\t'])).then(|| rest.trim())
}

/// The marker of a line that opens a fenced block.
fn fence_marker(trimmed: &str) -> Option<String> {
    ["```", "~~~"].iter().find_map(|fence| {
        trimmed.starts_with(fence).then(|| {
            let symbol = fence.chars().next().unwrap_or('`');
            trimmed.chars().take_while(|c| *c == symbol).collect()
        })
    })
}

/// A `CHECKPOINT: …` line (any case, bold allowed), without its bold.
fn checkpoint_line(trimmed: &str) -> Option<String> {
    static CHECKPOINT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^[*_]*checkpoint\b[^:]{0,40}:[*_]*\s*\S").expect("valid checkpoint regex")
    });
    CHECKPOINT
        .is_match(trimmed)
        .then(|| trimmed.replace("**", ""))
}

/// A line that names a check as the plan's own (bug-96115c), as its text: it
/// quotes code and says "the plan's final check", "the whole-plan check", "a
/// plan-level check" or "the final check of the plan". A task's last step ("as
/// a final check") is not one.
fn plan_check_line(line: &str) -> Option<String> {
    static PLAN_CHECK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)\bplan['’]?s\s+(?:own\s+|final\s+|last\s+|whole\s+)*check\b|\b(?:whole[- ]plan|plan[- ]level)\s+check\b|\bfinal\s+check\s+of\s+the\s+plan\b",
        )
        .expect("valid plan check regex")
    });
    let text = list_item(line).map_or(line.trim(), |(_, text)| text);
    (text.contains('`') && PLAN_CHECK.is_match(text)).then(|| one_line(text))
}

/// The text of a one-line HTML comment that names something
/// (`<!-- INV-001: Fitness stays in range -->`).
fn html_comment(trimmed: &str) -> Option<&str> {
    let text = trimmed.strip_prefix("<!--")?.strip_suffix("-->")?.trim();
    (!text.is_empty()).then_some(text)
}

/// A list item's indent and text, without a `[ ]` or `[x]` box.
fn list_item(line: &str) -> Option<(usize, &str)> {
    static ITEM: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(\s*)(?:[-*+]|\d{1,3}[.)])\s+(?:\[[ xX]\]\s+)?(\S.*)$")
            .expect("valid list item regex")
    });
    let captures = ITEM.captures(line)?;
    let indent = captures.get(1).map_or(0, |indent| {
        indent
            .as_str()
            .chars()
            .map(|c| if c == '\t' { 4 } else { 1 })
            .sum()
    });
    Some((indent, captures.get(2)?.as_str()))
}

/// Add `text` to a check as a detail.
fn detail(check: &mut String, text: &str) {
    check.push_str(if check.ends_with(':') { " " } else { "; " });
    check.push_str(text);
}

/// A table row's cells, trimmed: split at each `|` that is not escaped
/// (`\|`), which is then shown as a plain `|`.
fn table_cells(trimmed: &str) -> Vec<String> {
    let mut cells = vec![String::new()];
    let mut escaped = false;
    for c in trimmed.chars() {
        if c == '|' && !escaped {
            cells.push(String::new());
        } else if let Some(cell) = cells.last_mut() {
            cell.push(c);
        }
        escaped = c == '\\' && !escaped;
    }
    if trimmed.starts_with('|') {
        cells.remove(0);
    }
    if cells.last().is_some_and(|cell| cell.trim().is_empty()) {
        cells.pop();
    }
    cells
        .iter()
        .map(|cell| cell.trim().replace("\\|", "|"))
        .collect()
}

/// Whether a table row is the line under its header (`|---|:---:|`).
fn is_table_separator(trimmed: &str) -> bool {
    trimmed.starts_with('|')
        && trimmed.contains('-')
        && trimmed
            .chars()
            .all(|c| matches!(c, '|' | '-' | ':' | ' ' | '\t'))
}

/// The checks of a shell block in a check section: one per numbered comment
/// (`# 1. Workspace builds`, with the commands and the `# Expected:` line
/// under it), or one per command when nothing is numbered.
fn shell_checks(lines: &[String]) -> Vec<String> {
    static NUMBERED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^#\s*\d{1,3}[.)]\s+(\S.*)$").expect("valid numbered comment regex")
    });
    // Commands, with continuation lines joined.
    let mut commands: Vec<String> = Vec::new();
    let mut continued = false;
    let mut numbered: Vec<String> = Vec::new();
    // Whether the latest numbered check has a command yet.
    let mut has_command = false;
    for line in lines.iter().map(|line| line.trim()) {
        if let Some(title) = NUMBERED.captures(line).and_then(|captures| captures.get(1)) {
            numbered.push(title.as_str().to_string());
            continued = false;
            has_command = false;
            continue;
        }
        if line.is_empty() {
            continued = false;
            continue;
        }
        if let Some(comment) = line.strip_prefix('#') {
            let comment = comment.trim();
            if let (Some(check), Some(expected)) = (
                numbered.last_mut(),
                comment
                    .strip_prefix("Expected:")
                    .or_else(|| comment.strip_prefix("expected:")),
            ) {
                let _ = write!(check, " (expected: {})", expected.trim());
            }
            continue;
        }
        let command = line.strip_prefix("$ ").unwrap_or(line);
        let (command, continues) = match command.strip_suffix('\\') {
            Some(start) => (start.trim_end(), true),
            None => (command, false),
        };
        if let Some(check) = numbered.last_mut() {
            check.push_str(match (continued, has_command) {
                (true, _) => " ",
                (false, true) => "; ",
                (false, false) => ": ",
            });
            check.push_str(command);
            has_command = true;
        } else if continued && let Some(last) = commands.last_mut() {
            last.push(' ');
            last.push_str(command);
        } else {
            commands.push(command.to_string());
        }
        continued = continues;
    }
    let checks = if numbered.is_empty() {
        commands
    } else {
        numbered
    };
    checks.iter().map(|check| one_line(check)).collect()
}

/// `text` on one line, its whitespace collapsed, cut at [`CHECK_CHARS`].
fn one_line(text: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    match text.char_indices().nth(CHECK_CHARS) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text,
    }
}

/// The source check ids a text names (`AC2: SC5: …`).
fn named_checks(text: &str) -> Vec<String> {
    static ID: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\bSC\d+\b").expect("valid source check id regex"));
    ID.find_iter(text)
        .map(|found| found.as_str().to_string())
        .collect()
}

/// Commands that only look at files or move around: a verify step made of
/// nothing else passes when the behavior a check states is wrong.
const SHALLOW_COMMANDS: &[&str] = &[
    "[", "cat", "cd", "diff", "echo", "egrep", "export", "fgrep", "file", "find", "grep", "head",
    "ls", "printf", "rg", "stat", "tail", "test", "true", "wc",
];

/// Whether a command's `words` only build or lint: a compile check passes
/// whatever the code does.
fn builds_only(words: &[&str]) -> bool {
    matches!(
        words,
        ["cargo", "check" | "build" | "clippy" | "fmt", ..]
            | ["python" | "python3", "-m", "py_compile" | "compileall", ..]
            | ["tsc" | "eslint" | "ruff" | "mypy", ..]
            | ["npx", "tsc" | "eslint", ..]
            | ["go", "build" | "vet", ..]
    )
}

/// The commands of a shell line, split at `&&`, `||`, `|`, `;` and line
/// breaks outside quotes, without `2>&1`-style redirections.
pub(super) fn shell_segments(command: &str) -> Vec<String> {
    let mut segments = vec![String::new()];
    let mut quote: Option<char> = None;
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(open), _) if c == open => quote = None,
            (None, '\'' | '"') => quote = Some(c),
            (None, '>' | '<') if chars.peek() == Some(&'&') => {
                // A redirection to a file descriptor (`2>&1`), not a split:
                // it goes, with the descriptor before it.
                if let Some(segment) = segments.last_mut() {
                    while segment.ends_with(|c: char| c.is_ascii_digit()) {
                        segment.pop();
                    }
                }
                chars.next();
                while chars.peek().is_some_and(char::is_ascii_digit) {
                    chars.next();
                }
                continue;
            }
            // `&>file` sends both streams to a file: a redirection.
            (None, '&') if chars.peek() == Some(&'>') => continue,
            (None, '&' | '|' | ';' | '\n') => {
                segments.push(String::new());
                continue;
            }
            _ => {}
        }
        if let Some(segment) = segments.last_mut() {
            segment.push(c);
        }
    }
    segments
        .into_iter()
        .map(|segment| segment.trim().to_string())
        .filter(|segment| !segment.is_empty())
        .collect()
}

/// Whether a verify step's `command` only greps, tests files, lists or
/// builds (gap-dc5d7c): such a step keeps a check's wording, not what it
/// states, since it passes when the behavior is wrong.
fn is_shallow(command: &str) -> bool {
    shell_segments(command).iter().all(|segment| {
        let words: Vec<&str> = segment
            .split_whitespace()
            .skip_while(|word| *word == "!" || (word.contains('=') && !word.starts_with('-')))
            .collect();
        words
            .first()
            .is_none_or(|first| SHALLOW_COMMANDS.contains(first))
            || builds_only(&words)
    })
}

/// Whether a drop's `reason` only restates its `check` (gap-dc5d7c): most of
/// its words are the check's own, so it says nothing about why no command
/// can check it.
fn is_restatement(reason: &str, check: &str) -> bool {
    let words = |text: &str| -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|word| word.len() > 2)
            .map(str::to_string)
            .collect()
    };
    let (reason, check) = (words(reason), words(check));
    let shared = reason.iter().filter(|word| check.contains(word)).count();
    !reason.is_empty() && shared * 10 >= reason.len() * 7
}

/// Point verify steps whose `covers` name a source check (`covers =
/// ["SC3"]`) at an acceptance criterion of their task that states it, and
/// add `ACn: SC3: <the check>` when the task has none (gap-dc5d7c): a step's
/// `covers` names its own task's criteria (PLAN_044). Returns one line per
/// cover it moved, for the log.
pub(super) fn normalize_covers(plan: &mut TasksFile, checks: &[SourceCheck]) -> Vec<String> {
    let mut moved = Vec::new();
    for task in &mut plan.tasks {
        for step in 0..task.verify.len() {
            for cover in 0..task.verify[step].covers.len() {
                let id = task.verify[step].covers[cover].trim().to_string();
                let Some(check) = checks.iter().find(|check| check.id == id) else {
                    continue;
                };
                let stated = task
                    .acceptance
                    .iter()
                    .enumerate()
                    .find(|(_, item)| named_checks(item).contains(&id))
                    .map(|(index, item)| ac_id(item, index));
                let criterion = stated.unwrap_or_else(|| {
                    let taken: Vec<String> = task
                        .acceptance
                        .iter()
                        .enumerate()
                        .map(|(index, item)| ac_id(item, index))
                        .collect();
                    let mut n = task.acceptance.len() + 1;
                    while taken.contains(&format!("AC{n}")) {
                        n += 1;
                    }
                    task.acceptance
                        .push(format!("AC{n}: {}: {}", check.id, check.text));
                    format!("AC{n}")
                });
                moved.push(format!("{}: `covers` {id} became {criterion}", task.id));
                task.verify[step].covers[cover] = criterion;
            }
        }
    }
    moved
}

/// What became of a source's checks in a plan.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct SourceCoverage {
    checks: Vec<SourceCheck>,
    /// By check id: the task, and the acceptance criterion a verify step
    /// that runs a check of it covers.
    kept: BTreeMap<String, (String, String)>,
    /// By check id: the task, and the command of the step that covers it,
    /// when every such step only greps, tests files or builds.
    shallow: BTreeMap<String, (String, String)>,
    /// By check id: the task, and why its step cannot pass as written, when
    /// no step that can run checks it (gap-ecafde).
    cannot_run: BTreeMap<String, (String, String)>,
    /// By check id: the task, and the command of a step that covers a check
    /// whose own command the source gives, but runs another one (bug-72dc0d:
    /// a `cargo test` step covering `tools/spec.sh 2 rumble 'P2.04*'`), or
    /// more than the source's commands (bug-56a806: a `grep -q` before it).
    elsewhere: BTreeMap<String, (String, String)>,
    /// By check id: why the plan drops it (`[meta] dropped_checks`).
    dropped: BTreeMap<String, String>,
}

impl SourceCoverage {
    /// What became of `checks` in `plan`, whose `unrunnable` steps cannot
    /// pass as written. A check is kept when a verify step that can run
    /// covers it (or an acceptance criterion that names it), or runs its own
    /// command (bug-3cdc58), and runs more than a grep, a file test or a
    /// build; a check whose own command the source gives is kept only by a
    /// step that runs it (bug-72dc0d), as the step's whole command or in a
    /// `&&` join of the source's commands and nothing else (bug-56a806), and
    /// a step that covers it but runs another command keeps it no more than
    /// its acceptance text does. It cannot run when only steps that cannot
    /// pass cover it otherwise; shallow when only grep, file test or build
    /// steps do; dropped when `[meta] dropped_checks` gives a reason for it
    /// that does more than restate it; and missing otherwise.
    pub(super) fn of(checks: &[SourceCheck], plan: &TasksFile, unrunnable: &[Unrunnable]) -> Self {
        let text_of = |id: &str| {
            checks
                .iter()
                .find(|check| check.id == id)
                .map(|check| check.text.as_str())
        };
        // bug-96115c: a check of the plan's own is kept by `[[meta.verify]]`,
        // never by a task's step, which runs before the other tasks' work.
        let plan_level = |id: &str| {
            checks
                .iter()
                .any(|check| check.id == id && check.plan_level)
        };
        let run_of = |id: &str| {
            checks
                .iter()
                .find(|check| check.id == id)
                .and_then(|check| check.run.as_deref())
        };
        let (task_runs, plan_runs) = (source_runs(checks, false), source_runs(checks, true));
        let mut kept = BTreeMap::new();
        let mut shallow = BTreeMap::new();
        let mut cannot_run = BTreeMap::new();
        let mut elsewhere = BTreeMap::new();
        for task in &plan.tasks {
            // Each check a step covers: through a criterion, or named.
            let mut covering: Vec<(String, String, &str, usize)> = Vec::new();
            for (number, step) in (1..).zip(&task.verify) {
                for cover in step.covers.iter().map(|cover| cover.trim()) {
                    let named = task
                        .acceptance
                        .iter()
                        .enumerate()
                        .filter(|(index, item)| ac_id(item, *index) == cover)
                        .flat_map(|(_, item)| named_checks(item))
                        .chain(named_checks(cover));
                    for id in named {
                        covering.push((id, cover.to_string(), step.command.as_str(), number));
                    }
                }
                // bug-3cdc58: a step that runs a check's own command keeps it,
                // whether or not anything names the check, when it runs the
                // source's commands alone (bug-56a806).
                for check in checks.iter().filter(|check| {
                    !check.plan_level
                        && check
                            .run
                            .as_deref()
                            .is_some_and(|run| runs_command(&step.command, run))
                }) {
                    let criterion = step.covers.first().map_or_else(
                        || format!("step {number}"),
                        |cover| cover.trim().to_string(),
                    );
                    covering.push((check.id.clone(), criterion, step.command.as_str(), number));
                }
            }
            for (id, criterion, command, number) in covering {
                if text_of(&id).is_none() || kept.contains_key(&id) || plan_level(&id) {
                    continue;
                }
                let stuck = unrunnable
                    .iter()
                    .find(|step| step.task == task.id && step.step == number);
                let other_command =
                    run_of(&id).is_some_and(|run| !runs_exactly(command, run, &task_runs));
                if let Some(stuck) = stuck {
                    cannot_run
                        .entry(id)
                        .or_insert_with(|| (task.id.clone(), stuck.why.clone()));
                } else if other_command {
                    elsewhere
                        .entry(id)
                        .or_insert_with(|| (task.id.clone(), command.to_string()));
                } else if is_shallow(command) {
                    shallow
                        .entry(id)
                        .or_insert_with(|| (task.id.clone(), command.to_string()));
                } else {
                    shallow.remove(&id);
                    cannot_run.remove(&id);
                    elsewhere.remove(&id);
                    kept.insert(id, (task.id.clone(), criterion));
                }
            }
        }
        for check in checks.iter().filter(|check| check.plan_level) {
            let ran = check.run.as_deref().is_some_and(|run| {
                plan.meta
                    .verify
                    .iter()
                    .any(|step| runs_exactly(&step.command, run, &plan_runs))
            });
            if ran {
                kept.insert(
                    check.id.clone(),
                    ("plan".to_string(), "[[meta.verify]]".to_string()),
                );
            }
        }
        shallow.retain(|id, _| !cannot_run.contains_key(id) && !elsewhere.contains_key(id));
        elsewhere.retain(|id, _| !cannot_run.contains_key(id) && !kept.contains_key(id));
        let dropped = plan
            .meta
            .dropped_checks
            .iter()
            .filter_map(|entry| {
                let (id, reason) = entry.split_once(':')?;
                let (id, reason) = (id.trim(), reason.trim());
                let check = text_of(id)?;
                (!reason.is_empty() && !kept.contains_key(id) && !is_restatement(reason, check))
                    .then(|| (id.to_string(), reason.to_string()))
            })
            .collect::<BTreeMap<_, _>>();
        shallow.retain(|id, _| !dropped.contains_key(id));
        cannot_run.retain(|id, _| !dropped.contains_key(id));
        elsewhere.retain(|id, _| !dropped.contains_key(id));
        Self {
            checks: checks.to_vec(),
            kept,
            shallow,
            cannot_run,
            elsewhere,
            dropped,
        }
    }

    /// The checks the plan neither keeps nor drops with a reason, and no
    /// step covers.
    pub(super) fn missing(&self) -> Vec<&SourceCheck> {
        self.checks
            .iter()
            .filter(|check| {
                !self.kept.contains_key(&check.id)
                    && !self.shallow.contains_key(&check.id)
                    && !self.cannot_run.contains_key(&check.id)
                    && !self.elsewhere.contains_key(&check.id)
                    && !self.dropped.contains_key(&check.id)
            })
            .collect()
    }

    /// How many checks the plan neither keeps nor drops with a reason: the
    /// missing ones, the shallow ones and the ones whose step cannot run.
    pub(super) fn unkept(&self) -> usize {
        self.checks.len() - self.kept.len() - self.dropped.len()
    }

    /// One line for the person: how many checks the plan keeps.
    pub(super) fn summary_line(&self) -> String {
        let mut line = format!(
            "Source checks: {} of {} kept as verify steps",
            self.kept.len(),
            self.checks.len()
        );
        if !self.shallow.is_empty() {
            let _ = write!(
                line,
                ", {} checked only by a grep, a file test or a build",
                self.shallow.len()
            );
        }
        if !self.cannot_run.is_empty() {
            let _ = write!(
                line,
                ", {} checked by a step that cannot pass as written",
                self.cannot_run.len()
            );
        }
        if !self.elsewhere.is_empty() {
            let _ = write!(
                line,
                ", {} covered by a step that runs another command",
                self.elsewhere.len()
            );
        }
        if !self.dropped.is_empty() {
            let _ = write!(line, ", {} dropped with a reason", self.dropped.len());
        }
        let missing = self.missing().len();
        if missing > 0 {
            let _ = write!(line, ", {missing} in no verify step");
        }
        line
    }

    /// A line for each check the plan does not keep: what covers it only
    /// shallowly, why it was dropped, or that it is missing.
    pub(super) fn detail_lines(&self) -> Vec<String> {
        self.checks
            .iter()
            .filter_map(|check| {
                if self.kept.contains_key(&check.id) {
                    None
                } else if let Some((task, why)) = self.cannot_run.get(&check.id) {
                    Some(format!(
                        "cannot run {}: {} ({task}: {why})",
                        check.id, check.text
                    ))
                } else if let Some((task, command)) = self.elsewhere.get(&check.id) {
                    Some(format!(
                        "another command for {}: {} ({task} runs `{}`)",
                        check.id,
                        check.text,
                        one_line(command)
                    ))
                } else if let Some((task, command)) = self.shallow.get(&check.id) {
                    Some(format!(
                        "shallow {}: {} ({task} runs only `{}`)",
                        check.id,
                        check.text,
                        one_line(command)
                    ))
                } else if let Some(reason) = self.dropped.get(&check.id) {
                    Some(format!("dropped {}: {} ({reason})", check.id, check.text))
                } else {
                    Some(format!("missing {}: {}", check.id, check.text))
                }
            })
            .collect()
    }

    /// The coverage as the generation's outcome records it.
    pub(super) fn record(&self, regenerated: bool) -> GenerationSourceChecks {
        let checks = self
            .checks
            .iter()
            .map(|check| {
                let kept = self.kept.get(&check.id);
                let shallow = self.shallow.get(&check.id);
                let stuck = self.cannot_run.get(&check.id);
                let elsewhere = self.elsewhere.get(&check.id);
                let dropped = self.dropped.get(&check.id);
                let status = if kept.is_some() {
                    "kept"
                } else if stuck.is_some() {
                    "cannot_run"
                } else if shallow.is_some() {
                    "shallow"
                } else if dropped.is_some() {
                    "dropped"
                } else {
                    "missing"
                };
                GenerationSourceCheck {
                    id: check.id.clone(),
                    check: check.text.clone(),
                    status: status.to_string(),
                    task: kept
                        .or(stuck)
                        .or(shallow)
                        .or(elsewhere)
                        .map(|(task, _)| task.clone()),
                    criterion: kept.map(|(_, criterion)| criterion.clone()),
                    reason: dropped
                        .cloned()
                        .or_else(|| stuck.map(|(_, why)| why.clone()))
                        .or_else(|| {
                            shallow.map(|(_, command)| format!("only `{}`", one_line(command)))
                        })
                        .or_else(|| {
                            elsewhere.map(|(_, command)| {
                                format!("runs `{}`, not its own command", one_line(command))
                            })
                        }),
                }
            })
            .collect();
        GenerationSourceChecks {
            total: self.checks.len(),
            kept: self.kept.len(),
            shallow: self.shallow.len(),
            cannot_run: self.cannot_run.len(),
            dropped: self.dropped.len(),
            missing: self.missing().len() + self.elsewhere.len(),
            regenerated,
            checks,
            documents: Vec::new(),
        }
    }
}

/// Extensions of the documents a request may name for roko to read.
const NAMED_DOCUMENT_EXTENSIONS: &[&str] = &["markdown", "md"];

/// The largest document a request names that roko reads, in bytes.
const NAMED_DOCUMENT_BYTES: u64 = 256 * 1024;

/// A document of the workspace that a request names by its path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NamedDocument {
    /// Its path, relative to the workspace.
    pub path: String,
    /// Its text.
    pub text: String,
}

/// The markdown documents of the workspace that `request` names by path, in
/// the order it names them (round v7, phase 0). Asked naturally, a chat host
/// that read a spec sends roko a summary that names it ("Create a roko plan
/// for phase 0 from docs/specs/phase-0-foundation.md") and none of its checks,
/// so roko reads the spec itself. A path counts when it is relative, or
/// absolute inside `workdir`, and names a file of at most 256 KiB inside the
/// workspace.
pub(super) fn named_documents(request: &str, workdir: &Path) -> Vec<NamedDocument> {
    let root = workdir
        .canonicalize()
        .unwrap_or_else(|_| workdir.to_path_buf());
    let mut documents: Vec<NamedDocument> = Vec::new();
    let words = request.split(|c: char| c.is_whitespace() || "`\"'()[]<>*,;|".contains(c));
    for word in words {
        let word = word.trim_end_matches(['.', ':', '!', '?']);
        let Some(path) = document_path(word, workdir, &root) else {
            continue;
        };
        let file = root.join(&path);
        let inside = file
            .canonicalize()
            .is_ok_and(|real| real.starts_with(&root));
        let small = std::fs::metadata(&file)
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() <= NAMED_DOCUMENT_BYTES);
        if !inside || !small || documents.iter().any(|document| document.path == path) {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&file) {
            documents.push(NamedDocument { path, text });
        }
    }
    documents
}

/// `word` as the workspace path of a markdown document, when it names one
/// inside the workspace `workdir`, whose real path is `root`.
fn document_path(word: &str, workdir: &Path, root: &Path) -> Option<String> {
    let path = Path::new(word);
    let extension = path.extension()?.to_str()?;
    if !NAMED_DOCUMENT_EXTENSIONS.contains(&extension) {
        return None;
    }
    let relative = if path.is_absolute() {
        path.strip_prefix(workdir)
            .or_else(|_| path.strip_prefix(root))
            .ok()?
    } else {
        path
    };
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?),
            Component::CurDir => {}
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// The checks `documents` state, numbered `SC1`, `SC2`, … across them, in
/// order.
pub(super) fn document_checks(documents: &[NamedDocument]) -> Vec<SourceCheck> {
    let mut checks: Vec<SourceCheck> = documents
        .iter()
        .flat_map(|document| source_checks(&document.text))
        .collect();
    for (index, check) in checks.iter_mut().enumerate() {
        check.id = format!("SC{}", index + 1);
    }
    checks
}

/// Who states the source checks, as the planner block's first words: the
/// source itself (`The prompt states`), or the documents it names.
pub(super) fn checks_stated_by(kind: &str, documents: &[NamedDocument]) -> String {
    match documents {
        [] => format!("The {kind} states"),
        [document] => format!("`{}`, which the {kind} names, states", document.path),
        _ => format!("The documents the {kind} names state"),
    }
}

/// The planner prompt's copy of `documents`, the documents a request names
/// whose checks are the source checks, within `chars` characters in all.
pub(super) fn documents_block(documents: &[NamedDocument], kind: &str, chars: usize) -> String {
    let mut block = format!(
        "## Documents the {kind} names\n\n\
         The {kind} states no checks itself; it names the documents below, which roko read for \
         you. Plan from them as much as from the {kind}, and do not open them again.\n"
    );
    let mut left = chars;
    for document in documents {
        let end = document.text.floor_char_boundary(left);
        let text = document.text[..end].trim_end();
        let fence = "`".repeat(longest_backtick_run(text).max(3) + 1);
        let _ = write!(
            block,
            "\n### `{}`\n\n{fence}markdown\n{text}\n{fence}\n",
            document.path
        );
        if end < document.text.len() {
            let _ = writeln!(
                block,
                "\n`{}` is cut here, at {end} characters: read the rest of it yourself.",
                document.path
            );
        }
        left -= end;
    }
    block
}

/// The longest run of backticks in `text`.
fn longest_backtick_run(text: &str) -> usize {
    text.split(|c| c != '`').map(str::len).max().unwrap_or(0)
}

/// What a source check's own command must be, since roko runs it alone
/// (bug-56a806): the planner hears it with the checks, and again when asked
/// for a better plan (gap-115c2a).
const SELF_CONTAINED: &str = "A source check's command must be self-contained and runs exactly as \
                              written, so a check that needs a build includes the build in its own \
                              command: a build put before it in a step is dropped.";

/// The planner prompt's list of the source's `checks`, with how to keep
/// each one; empty when the source states none. `states` says who states
/// them ([`checks_stated_by`]).
pub(super) fn planner_block(checks: &[SourceCheck], states: &str) -> String {
    if checks.is_empty() {
        return String::new();
    }
    let mut block = format!(
        "## Source checks\n\n\
         {states} the {} checks below. Keep each one as a verify step of the task that \
         delivers it: a step whose `covers` names the check (`covers = [\"SC3\"]`) and whose \
         command checks what the check states, by running a test the repository has or a test \
         the task writes. Name a test the task writes in its description, and put a `grep -q` \
         for it before the run, so the step fails until the task is done. A grep, a file test \
         or a compile check alone does not keep a check: it passes when the behavior is wrong. \
         One command may cover several checks when one run checks them all. Drop a check only \
         when no command can check it (it needs a person, hardware, or another plan's work \
         first), and then list it in `[meta] dropped_checks` as `\"SCk: <why no command can \
         check it>\"`. Never merge two checks, reword one away, or leave one out.\n\n",
        checks.len()
    );
    if checks.iter().any(|check| check.run.is_some()) {
        block.push_str(
            "A check given with `run:` has its command already: that command, unchanged, is its \
             step's whole command. Put no `grep -q`, file test or other command before or after \
             it; roko runs the source's commands alone. ",
        );
        block.push_str(SELF_CONTAINED);
        block.push_str("\n\n");
    }
    if checks.iter().any(|check| check.plan_level) {
        block.push_str(
            "A check marked as the plan's own runs once, after every task has passed: put it in \
             `[[meta.verify]]` with its command unchanged, never in a task's verify steps.\n\n",
        );
    }
    for check in checks {
        let _ = write!(block, "- {}: {}", check.id, check.text);
        if check.plan_level {
            block.push_str(" (the plan's own check: `[[meta.verify]]`)");
        }
        if let Some(run) = &check.run {
            let _ = write!(block, " (run: `{run}`)");
        }
        block.push('\n');
    }
    block
}

/// Give each of `checks` the command that runs it, when roko can tell: the
/// unittest ids it names, wildcards expanded from the workspace's test files
/// (bug-8fe462), or else the one command it quotes (bug-3cdc58), as a spec's
/// table of checks gives each check's command (`| P0.01 | `tools/spec.sh 0
/// params 'P0.01*'` | … |`).
pub(super) fn attach_runs(checks: &mut [SourceCheck], workdir: &Path, probe: &Probe) {
    for check in checks {
        check.run = check_command(&check.text, workdir, probe)
            .or_else(|| quoted_command(&check.text, workdir, probe));
    }
}

/// The one command `text` quotes in a code span (bug-3cdc58); `None` when no
/// span is a command ([`is_command`]), or more than one is.
fn quoted_command(text: &str, workdir: &Path, probe: &Probe) -> Option<String> {
    static SPAN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"`([^`]+)`").expect("valid code span regex"));
    let mut commands = SPAN
        .captures_iter(text)
        .filter_map(|captures| captures.get(1))
        .map(|span| span.as_str().trim())
        .filter(|span| is_command(span, workdir, probe));
    let command = commands.next()?;
    commands.next().is_none().then(|| command.to_string())
}

/// Whether a code span is a command: a script given by its path (with a
/// script's extension, or a file of the workspace), or a program this
/// machine has, with the words it is given. A file name, a symbol or a
/// signature is not one.
fn is_command(span: &str, workdir: &Path, probe: &Probe) -> bool {
    let words = command_words(span);
    let Some(program) = words.first() else {
        return false;
    };
    if program.contains('/') {
        let path = program.trim_start_matches("./");
        let script = Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| SCRIPT_EXTENSIONS.contains(&extension));
        return script || workdir.join(path).is_file();
    }
    words.len() > 1 && probe.command(program)
}

/// The commands of a shell line, each as its words ([`command_words`]).
fn commands_of(text: &str) -> Vec<Vec<String>> {
    shell_segments(text)
        .iter()
        .map(|segment| command_words(segment))
        .filter(|words| !words.is_empty())
        .collect()
}

/// Whether `command`, a verify step's, runs `run`, a check's command: each
/// of `run`'s commands is one of `command`'s, word for word, quotes aside
/// (bug-3cdc58).
fn runs_command(command: &str, run: &str) -> bool {
    let (have, wanted) = (commands_of(command), commands_of(run));
    !wanted.is_empty() && wanted.iter().all(|words| have.contains(words))
}

/// The commands the source gives `checks`: those of the plan's own checks
/// when `plan_level`, else those of the checks its tasks keep.
fn source_runs(checks: &[SourceCheck], plan_level: bool) -> Vec<&str> {
    checks
        .iter()
        .filter(|check| check.plan_level == plan_level)
        .filter_map(|check| check.run.as_deref())
        .collect()
}

/// The source's commands, of `runs`, that `command`, a verify step's, runs
/// ([`runs_command`]), in the order it runs them, each once (bug-56a806):
/// two with the same words are one, and one whose commands another of them
/// runs too is part of that one.
fn runs_in<'a>(command: &str, runs: &[&'a str]) -> Vec<&'a str> {
    let have = commands_of(command);
    // Each one it runs: where it starts in `command`, and its commands.
    let mut found: Vec<(usize, &'a str, Vec<Vec<String>>)> = Vec::new();
    for &run in runs {
        let wanted = commands_of(run);
        let start = wanted
            .iter()
            .map(|words| have.iter().position(|have| have == words))
            .collect::<Option<Vec<usize>>>()
            .and_then(|at| at.into_iter().min());
        if let Some(start) = start
            && !found.iter().any(|(_, _, other)| *other == wanted)
        {
            found.push((start, run, wanted));
        }
    }
    let mut whole: Vec<(usize, &'a str)> = found
        .iter()
        .filter(|(_, _, wanted)| {
            !found.iter().any(|(_, _, other)| {
                other.len() > wanted.len() && wanted.iter().all(|words| other.contains(words))
            })
        })
        .map(|(start, run, _)| (*start, *run))
        .collect();
    whole.sort_by_key(|(start, _)| *start);
    whole.into_iter().map(|(_, run)| run).collect()
}

/// `command`, a verify step's, cut down to the source's commands of `runs`
/// it runs ([`runs_in`]), joined by `&&` and written as the source writes
/// them; `None` when it runs none of them.
fn exact_command(command: &str, runs: &[&str]) -> Option<String> {
    let found = runs_in(command, runs);
    (!found.is_empty()).then(|| found.join(" && "))
}

/// Whether `command`, a verify step's, runs `run`, a check's command, and
/// nothing but the source's commands of `runs`, as the source writes them
/// (bug-56a806): a guard of the planner's own before them is another
/// command, which can fail on correct work.
fn runs_exactly(command: &str, run: &str, runs: &[&str]) -> bool {
    runs_command(command, run)
        && exact_command(command, runs).is_some_and(|exact| exact == command.trim())
}

/// Give the plan a `[[meta.verify]]` step for each check the source names as
/// the plan's own final check (bug-96115c) that no such step runs yet: its
/// command, unchanged, which runs once every task has passed. Returns one
/// line per step added.
pub(super) fn add_plan_checks(plan: &mut TasksFile, checks: &[SourceCheck]) -> Vec<String> {
    let mut added = Vec::new();
    for check in checks.iter().filter(|check| check.plan_level) {
        let Some(run) = &check.run else {
            continue;
        };
        if plan
            .meta
            .verify
            .iter()
            .any(|step| runs_command(&step.command, run))
        {
            continue;
        }
        plan.meta.verify.push(VerifyStep {
            phase: "test".to_string(),
            command: run.clone(),
            fail_msg: Some(format!("{}: {}", check.id, check.text)),
            timeout_ms: crate::task_parser::default_verify_timeout(),
            scope: Vec::new(),
            covers: Vec::new(),
            expect: None,
        });
        added.push(format!(
            "added `[[meta.verify]]` running `{run}`: the source names it the plan's final check \
             ({})",
            check.id
        ));
    }
    added
}

/// Run each source check that has a command with that command (bug-8fe462),
/// in every verify step that covers only such checks and cannot pass as
/// written (`unrunnable`): a planner that invented test names for a
/// wildcard check gets the real ones. The step runs those commands alone
/// (bug-56a806). Returns one line per step changed.
pub(super) fn apply_runs(
    plan: &mut TasksFile,
    checks: &[SourceCheck],
    unrunnable: &[Unrunnable],
) -> Vec<String> {
    let mut applied = Vec::new();
    for task in &mut plan.tasks {
        for (number, step) in (1..).zip(task.verify.iter_mut()) {
            if !unrunnable
                .iter()
                .any(|stuck| stuck.task == task.id && stuck.step == number)
            {
                continue;
            }
            // A check of the plan's own runs in `[[meta.verify]]` (bug-96115c).
            let named: Vec<String> = step_checks(&task.acceptance, step)
                .into_iter()
                .filter(|id| {
                    !checks
                        .iter()
                        .any(|check| &check.id == id && check.plan_level)
                })
                .collect();
            // Every check it covers must have a command of its own.
            let Some(runs) = named
                .iter()
                .map(|id| {
                    checks
                        .iter()
                        .find(|check| &check.id == id)
                        .and_then(|check| check.run.as_deref())
                })
                .collect::<Option<Vec<&str>>>()
            else {
                continue;
            };
            let Some(command) = exact_command(&runs.join(" && "), &runs) else {
                continue;
            };
            applied.push(format!(
                "{} step {number}: `{}` became `{command}`, the source check's own tests",
                task.id, step.command
            ));
            step.command = command;
        }
    }
    applied
}

/// The source checks a verify step covers: named in its `covers`, or by an
/// acceptance criterion of its task (`acceptance`) that its `covers` names.
fn step_checks(acceptance: &[String], step: &VerifyStep) -> Vec<String> {
    step.covers
        .iter()
        .map(|cover| cover.trim())
        .flat_map(|cover| {
            acceptance
                .iter()
                .enumerate()
                .filter(|(index, item)| ac_id(item, *index) == cover)
                .flat_map(|(_, item)| named_checks(item))
                .chain(named_checks(cover))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Run each source check whose own command roko knows, and that no step
/// runs, in the task that owns it (bug-72dc0d). The planner groups the work
/// and words it; roko runs the source's commands, so how many of them a plan
/// keeps does not turn on the planner retyping each one ([`owner_of`]). A
/// task's checks run as one step: in place of its first step that cannot
/// pass as written (`unrunnable`); else as a step of its own while the task
/// has fewer than `max_steps(task)` steps; else in its last step that runs
/// source commands, or else its last step. A step they join runs the
/// source's commands alone (bug-56a806): those it ran already, then these.
/// Returns one line per step changed or added.
pub(super) fn assign_runs(
    plan: &mut TasksFile,
    checks: &[SourceCheck],
    unrunnable: &[Unrunnable],
    max_steps: impl Fn(&TaskDef) -> usize,
) -> Vec<String> {
    let task_runs = source_runs(checks, false);
    let mut owned: BTreeMap<usize, Vec<&SourceCheck>> = BTreeMap::new();
    for check in checks.iter().filter(|check| !check.plan_level) {
        let Some(run) = check.run.as_deref() else {
            continue;
        };
        let ran = plan
            .tasks
            .iter()
            .flat_map(|task| &task.verify)
            .any(|step| runs_command(&step.command, run));
        if let (false, Some(owner)) = (ran, owner_of(plan, check, run)) {
            owned.entry(owner).or_default().push(check);
        }
    }
    let mut assigned = Vec::new();
    for (index, owned) in owned {
        let limit = max_steps(&plan.tasks[index]);
        let task = &mut plan.tasks[index];
        let mut runs: Vec<&str> = Vec::new();
        let mut covers: Vec<String> = Vec::new();
        for check in &owned {
            if let Some(run) = check.run.as_deref()
                && !runs.contains(&run)
            {
                runs.push(run);
            }
            if let Some(criterion) = criterion_naming(&task.acceptance, check)
                && !covers.contains(&criterion)
            {
                covers.push(criterion);
            }
        }
        let command = runs.join(" && ");
        let ids = owned
            .iter()
            .map(|check| check.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let stuck = (1..=task.verify.len()).find(|number| {
            unrunnable
                .iter()
                .any(|step| step.task == task.id && step.step == *number)
        });
        // At the step limit: the last step that runs source commands, which
        // runs these too, else the last step.
        let last = (task.verify.len() >= limit.max(1)).then(|| {
            (1..=task.verify.len())
                .rev()
                .find(|number| {
                    exact_command(&task.verify[number - 1].command, &task_runs).is_some()
                })
                .unwrap_or(task.verify.len())
        });
        if let Some(number) = stuck.or(last) {
            let step = &mut task.verify[number - 1];
            let was = one_line(&step.command);
            // The source's commands the step runs already stay, before these.
            let both = format!("{} && {command}", step.command);
            step.command = exact_command(&both, &task_runs).unwrap_or(command);
            for cover in covers {
                if !step.covers.contains(&cover) {
                    step.covers.push(cover);
                }
            }
            assigned.push(format!(
                "{} step {number}: `{was}` became `{}`, to run the source's own commands for \
                 {ids}",
                task.id,
                one_line(&step.command)
            ));
        } else {
            assigned.push(format!(
                "{}: added a step running `{command}`, the source's own commands for {ids}",
                task.id
            ));
            task.verify.push(VerifyStep {
                phase: "test".to_string(),
                command,
                fail_msg: Some(format!("the source's checks {ids}")),
                timeout_ms: crate::task_parser::default_verify_timeout(),
                scope: Vec::new(),
                covers,
                expect: None,
            });
        }
    }
    assigned
}

/// Make each verify step that runs a source check's command run the
/// source's commands it runs alone, each once, as the source writes them
/// (bug-56a806). Planners put guards of their own before them (`grep -q
/// 'AllocationCounter' core/src/engine.cpp && tools/spec.sh 2 engine
/// 'P2.18*,P2.19*'`), which fail on correct work; those go, and so does any
/// other command around them. A task's steps run its checks' commands, and
/// `[[meta.verify]]` the plan's own; a step that runs none is left alone. A
/// step that comes out the same as an earlier one of its task joins it, with
/// its `covers`: a generated plan may not repeat a check
/// (`PLAN_VERIFY_DUPLICATE`). Returns one line per step changed.
pub(super) fn run_exactly(plan: &mut TasksFile, checks: &[SourceCheck]) -> Vec<String> {
    let (task_runs, plan_runs) = (source_runs(checks, false), source_runs(checks, true));
    let mut changed = Vec::new();
    for task in &mut plan.tasks {
        changed.extend(exact_steps(&mut task.verify, &task_runs, &task.id));
    }
    changed.extend(exact_steps(
        &mut plan.meta.verify,
        &plan_runs,
        "`[[meta.verify]]`",
    ));
    changed
}

/// [`run_exactly`] for one list of verify `steps`, a task's or the plan's,
/// which the lines it returns call `owner`'s.
fn exact_steps(steps: &mut Vec<VerifyStep>, runs: &[&str], owner: &str) -> Vec<String> {
    let mut changed = Vec::new();
    // The number in the planner's list of each step kept so far.
    let mut numbers: Vec<usize> = Vec::new();
    let mut index = 0;
    for number in 1..=steps.len() {
        let Some(exact) = exact_command(&steps[index].command, runs) else {
            numbers.push(number);
            index += 1;
            continue;
        };
        let was = one_line(&steps[index].command);
        if let Some(earlier) = steps[..index].iter().position(|step| step.command == exact) {
            let step = steps.remove(index);
            let into = &mut steps[earlier];
            for cover in step.covers {
                if !into.covers.contains(&cover) {
                    into.covers.push(cover);
                }
            }
            into.timeout_ms = into.timeout_ms.max(step.timeout_ms);
            changed.push(format!(
                "{owner} step {number}: `{was}` became `{exact}`, the source's own commands \
                 alone, which step {} runs already, so it joined that step",
                numbers[earlier]
            ));
            continue;
        }
        if exact != steps[index].command {
            changed.push(format!(
                "{owner} step {number}: `{was}` became `{exact}`, the source's own commands alone"
            ));
            steps[index].command = exact;
        }
        numbers.push(number);
        index += 1;
    }
    changed
}

/// The one task of `plan` that owns `check`, whose command is `run`, by the
/// first of these rules that names any task: a step of the task covers the
/// check; the task's title, description or acceptance names it (by its id,
/// by its own label in the source such as `P2.04`, or by its command); its
/// command names one of the task's files (`rumble` in `tools/spec.sh 2 rumble
/// 'P2.04*'` for `core/src/rumble.cpp`). `None` when no rule names a task, or
/// the first that does names several.
fn owner_of(plan: &TasksFile, check: &SourceCheck, run: &str) -> Option<usize> {
    let label = check_label(&check.text);
    let covers = |task: &TaskDef| {
        task.verify
            .iter()
            .any(|step| step_checks(&task.acceptance, step).contains(&check.id))
    };
    let names = |task: &TaskDef| {
        std::iter::once(task.title.as_str())
            .chain(task.description.as_deref())
            .chain(task.acceptance.iter().map(String::as_str))
            .any(|text| {
                named_checks(text).contains(&check.id)
                    || label.is_some_and(|label| contains_token(text, label))
                    || text.contains(run)
            })
    };
    let files = |task: &TaskDef| names_files_of(run, task);
    let rules: [&dyn Fn(&TaskDef) -> bool; 3] = [&covers, &names, &files];
    for rule in rules {
        let mut owners = plan
            .tasks
            .iter()
            .enumerate()
            .filter(|(_, task)| rule(task))
            .map(|(index, _)| index);
        match (owners.next(), owners.next()) {
            (Some(owner), None) => return Some(owner),
            (Some(_), Some(_)) => return None,
            (None, _) => {}
        }
    }
    None
}

/// The acceptance criterion of a task (its `acceptance`) that names `check`:
/// by the check's id, its label in the source, or its command.
fn criterion_naming(acceptance: &[String], check: &SourceCheck) -> Option<String> {
    let label = check_label(&check.text);
    acceptance
        .iter()
        .enumerate()
        .find(|(_, item)| {
            named_checks(item).contains(&check.id)
                || label.is_some_and(|label| contains_token(item, label))
                || check.run.as_deref().is_some_and(|run| item.contains(run))
        })
        .map(|(index, item)| ac_id(item, index))
}

/// The source's own label for a check stated as a table row: its first cell,
/// when that is one short word with a letter and a digit (`P2.04` of `P2.04 |
/// `tools/spec.sh 2 rumble 'P2.04*'` | …`).
fn check_label(text: &str) -> Option<&str> {
    let (first, _) = text.split_once(" | ")?;
    let first = first.trim();
    (first.len() <= 24
        && !first.contains(char::is_whitespace)
        && first.contains(|c: char| c.is_ascii_alphabetic())
        && first.contains(|c: char| c.is_ascii_digit()))
    .then_some(first)
}

/// Whether `text` holds `token` as a word of its own: not inside a longer
/// word or number (`P2.01` is not in `P2.010`).
fn contains_token(text: &str, token: &str) -> bool {
    text.match_indices(token).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + token.len()..].chars().next();
        !before.is_some_and(|c| c.is_alphanumeric() || c == '.')
            && !after.is_some_and(char::is_alphanumeric)
    })
}

/// Words too common in commands and file names to tell one task's files
/// from another's.
const COMMON_WORDS: &[&str] = &[
    "core", "index", "lib", "main", "mod", "spec", "src", "test", "tests", "util", "utils",
];

/// Whether `run`, a check's command, names one of `task`'s files: a word of
/// its arguments is the name of a file the task writes, without its
/// extension, or a part of that name (`output` of `output_stage.cpp`).
fn names_files_of(run: &str, task: &TaskDef) -> bool {
    let words: Vec<String> = command_words(run)
        .iter()
        .skip(1)
        .filter(|word| !word.starts_with('-'))
        .flat_map(|word| {
            word.split(|c: char| !c.is_ascii_alphanumeric())
                .map(str::to_ascii_lowercase)
                .collect::<Vec<_>>()
        })
        .filter(|word| {
            word.len() >= 3
                && word.chars().all(|c| c.is_ascii_alphabetic())
                && !COMMON_WORDS.contains(&word.as_str())
        })
        .collect();
    task.files
        .iter()
        .filter_map(|file| Path::new(file).file_stem()?.to_str())
        .map(str::to_ascii_lowercase)
        .any(|stem| {
            words
                .iter()
                .any(|word| *word == stem || stem.split(['_', '-']).any(|part| part == word))
        })
}

/// The prompt that asks the planner once more for a plan that leaves some of
/// the source's checks unkept, or has verify steps that cannot pass as
/// written (gap-ecafde): the original request, the plan it wrote, each check
/// it leaves out or checks only shallowly, and each step that cannot run.
pub(super) fn regeneration_prompt(
    task_prompt: &str,
    plan: &str,
    coverage: &SourceCoverage,
    unrunnable: &[Unrunnable],
    unchecked: &[Unchecked],
) -> String {
    let mut problems = String::new();
    let missing = coverage.missing();
    if !missing.is_empty() {
        problems.push_str(
            "These are in no verify step, and `[meta] dropped_checks` gives no reason why no \
             command can check them:\n",
        );
        for check in missing {
            let _ = writeln!(problems, "- {}: {}", check.id, check.text);
        }
    }
    if !coverage.shallow.is_empty() {
        problems.push_str(
            "These are checked only by a grep, a file test or a build, which passes when the \
             behavior is wrong:\n",
        );
        for check in &coverage.checks {
            if let Some((task, command)) = coverage.shallow.get(&check.id) {
                let _ = writeln!(
                    problems,
                    "- {}: {} ({task} runs only `{}`)",
                    check.id,
                    check.text,
                    one_line(command)
                );
            }
        }
    }
    if !coverage.elsewhere.is_empty() {
        problems.push_str(
            "These checks come with their own command, but the step that covers them runs \
             another one; run each check's own command, unchanged:\n",
        );
        for check in &coverage.checks {
            if let Some((task, command)) = coverage.elsewhere.get(&check.id) {
                let _ = writeln!(
                    problems,
                    "- {}: {} ({task} runs `{}`)",
                    check.id,
                    check.text,
                    one_line(command)
                );
            }
        }
    }
    if !unrunnable.is_empty() {
        problems.push_str(
            "These verify steps cannot pass as written, whatever the work does; use a runner \
             this machine has (see Test runners), name only tests that exist or that the task \
             writes, put each test file a task writes in its `files`, and check files the task \
             writes: a check of files no task of the plan writes is decided before it runs:\n",
        );
        for step in unrunnable {
            let _ = writeln!(problems, "- {} step {}: {}", step.task, step.step, step.why);
        }
    }
    if !unchecked.is_empty() {
        problems.push_str(
            "No check of these tasks can see their work: every verify step of theirs reads \
             only files no task of the plan writes and already passes, so it passes whatever \
             the task does. Give each task a role that writes and checks that read what it \
             writes; a spec, a design or notes is a `scribe` task that writes them to a file in \
             its `files`:\n",
        );
        for task in unchecked {
            let _ = writeln!(problems, "- {}: {}", task.task, only_reads(&task.reads));
        }
    }
    let kept = if coverage.checks.is_empty() {
        String::new()
    } else {
        format!(
            "Your plan below keeps {} of the {} source checks as verify steps that check them.\n",
            coverage.kept.len(),
            coverage.checks.len()
        )
    };
    let own = if coverage.checks.iter().any(|check| check.run.is_some()) {
        format!(" {SELF_CONTAINED}")
    } else {
        String::new()
    };
    format!(
        "{task_prompt}\n\n---\n\n{kept}{problems}\n```toml\n{plan}\n```\n\n\
         Keep each source check as a verify step whose `covers` names it and whose command runs \
         a test of what it states: the check's own command alone, unchanged, when it comes with \
         one; else a test the repository has, or one the task writes, named in its description \
         and guarded by a `grep -q` for it.{own} When no command can check one, list \
         it in `[meta] dropped_checks` as `\"SCk: <why no command can check it>\"`. Fix each \
         step that cannot pass and each task no check can see, changing a task's role and \
         `files` where that takes it. Keep the rest of the plan as it is. Output the complete \
         plan as a single ```toml fenced block, followed only by the ```accept:accept/<file> \
         blocks of the tests it pins."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(source: &str) -> Vec<String> {
        source_checks(source)
            .into_iter()
            .map(|check| check.text)
            .collect()
    }

    /// The bullets of a Verification section are its checks; the numbered
    /// steps of a section of another kind, and prose, are not.
    #[test]
    fn verification_bullets_are_checks() {
        let source = "# Plan 08g\n\n## What to build\n\n1. `TuringMachine` module:\n   - 16-bit \
                      register\n\n## Verification\n\n- `TuringMachine` at `change=0.0`: \
                      identical\n  sequence on every cycle\n- `ClockDivider` `div4` fires once \
                      per 4 pulses\n\nThe end.\n";
        assert_eq!(
            texts(source),
            [
                "`TuringMachine` at `change=0.0`: identical sequence on every cycle",
                "`ClockDivider` `div4` fires once per 4 pulses",
            ]
        );
        let checks = source_checks(source);
        assert_eq!(checks[1].id, "SC2");
    }

    /// A table under a check heading gives a check per row, not its header;
    /// the prose and the rubric around it give none.
    #[test]
    fn invariant_table_rows_are_checks() {
        let source = "## Verification\n\nThe tests are the criteria.\n\n### Invariants\n\n\
                      | INV | Test | Constraint |\n|---|---|---|\n\
                      | INV-1 | `tests.test_osc.test_a` | RMS > 0.3 |\n\
                      | INV-2 | `tests.test_osc.test_b` | \\|a − b\\| > 0.05 |\n\n\
                      ## Rubric (for a reviewer)\n\n- Keeps its state.\n";
        assert_eq!(
            texts(source),
            [
                "INV-1 | `tests.test_osc.test_a` | RMS > 0.3",
                "INV-2 | `tests.test_osc.test_b` | |a − b| > 0.05",
            ]
        );
    }

    /// bardo's plan docs: an invariant is a comment with its fields under
    /// it; a Testing Checkpoint block numbers its commands; a `CHECKPOINT:`
    /// line counts anywhere; and the reference table and anchor list under
    /// Verification are not checks.
    #[test]
    fn bardo_invariants_checkpoints_and_commands_are_checks() {
        let source = "## Testing Checkpoint\n\nRun these:\n\n```bash\n# 1. Members\n\
                      cargo metadata --no-deps \\\n  --format-version 1\n# Expected: 25\n\n\
                      # 2. Check\ncargo check --workspace\n```\n\n\
                      CHECKPOINT: cargo test -p golem-tools -- bridge\n\n\
                      ## Verification\n\n### Invariants\n\n\
                      <!-- INV-001: Fitness range -->\n- **type**: numeric_range\n\
                      - **constraint**: `0.0 ≤ fitness ≤ 1.0`\n\n\
                      <!-- INV-002: Decay -->\n- **test_fn**: `test_decay`\n\n\
                      ### Regression Anchors\n\n`test_frame_duration_60fps`\n\n\
                      ### Academic References Verified\n\n| Reference | Notes |\n|---|---|\n\
                      | 60 FPS | standard |\n\n```rust\nfn main() {}\n```\n";
        assert_eq!(
            texts(source),
            [
                "Members: cargo metadata --no-deps --format-version 1 (expected: 25)",
                "Check: cargo check --workspace",
                "CHECKPOINT: cargo test -p golem-tools -- bridge",
                "INV-001: Fitness range; **type**: numeric_range; **constraint**: \
                 `0.0 ≤ fitness ≤ 1.0`",
                "INV-002: Decay; **test_fn**: `test_decay`",
            ]
        );
    }

    /// A step that only greps, tests files, lists or builds keeps no check;
    /// one that runs a test does, whatever greps guard it.
    #[test]
    fn grep_file_test_and_build_steps_are_shallow() {
        for command in [
            "grep -q 'struct TuringMachine' src/turing_machine.rs && echo ok",
            "test -f src/widget.rs",
            "ls synth/ | grep -c osc",
            "cargo check -p golem-sonification 2>&1",
            "python3 -m py_compile synth/osc.py",
            "grep -q 'a|b' src/x.rs",
        ] {
            assert!(is_shallow(command), "{command}");
        }
        for command in [
            "grep -q 'fn locked_loop' src/tm.rs && cargo test -p golem-sonification locked_loop",
            "python3 -m unittest tests.test_osc.OscillatorTest.test_pitch_tracks_midi",
            "cd synth && pytest -q tests/test_osc.py",
            "RUST_LOG=info cargo test quantizer 2>&1",
        ] {
            assert!(!is_shallow(command), "{command}");
        }
    }

    /// A drop's reason must say why no command can check it, not restate
    /// the check.
    #[test]
    fn a_restated_check_is_no_reason() {
        let check = "`ProbabilityGate` at `probability=0.0`: blocks all gates (0 outputs)";
        assert!(is_restatement(
            "ProbabilityGate at probability=0.0 blocks all gates.",
            check
        ));
        assert!(!is_restatement(
            "needs a person to listen to the output",
            check
        ));
    }

    /// A step that covers a source check by its id is pointed at a criterion
    /// of its task that states the check, which is added when missing; the
    /// coverage then keeps the check, by that criterion.
    #[test]
    fn covers_naming_a_check_get_a_criterion_and_keep_it() {
        let checks = source_checks("## Verification\n\n- the widget spins\n- the widget stops\n");
        let mut plan = TasksFile::parse_str(
            "[meta]\nplan = \"p\"\ntotal = 1\nstatus = \"ready\"\n\n\
             [[task]]\nid = \"T1\"\ntitle = \"t\"\nstatus = \"ready\"\nrole = \"implementer\"\n\
             tier = \"focused\"\nacceptance = [\"AC1: SC2: the widget stops\"]\n\n\
             [[task.verify]]\nphase = \"test\"\ncommand = \"pytest tests/test_w.py::test_spins\"\n\
             covers = [\"SC1\"]\n\n\
             [[task.verify]]\nphase = \"test\"\ncommand = \"grep -q stop src/w.py\"\n\
             covers = [\"SC2\"]\n",
        )
        .expect("a plan");
        let moved = normalize_covers(&mut plan, &checks);
        assert_eq!(
            moved,
            ["T1: `covers` SC1 became AC2", "T1: `covers` SC2 became AC1"]
        );
        let task = &plan.tasks[0];
        assert_eq!(task.acceptance[1], "AC2: SC1: the widget spins");
        assert_eq!(task.verify[0].covers, ["AC2"]);

        let coverage = SourceCoverage::of(&checks, &plan, &[]);
        assert_eq!(coverage.kept.len(), 1, "{coverage:?}");
        assert_eq!(coverage.shallow.len(), 1, "{coverage:?}");
        assert_eq!(coverage.unkept(), 1);
        assert_eq!(
            coverage.summary_line(),
            "Source checks: 1 of 2 kept as verify steps, 1 checked only by a grep, a file test \
             or a build"
        );

        // gap-ecafde: a check whose only test step cannot run is not kept.
        let stuck = [Unrunnable {
            task: "T1".to_string(),
            step: 1,
            why: "`pytest` is not installed".to_string(),
        }];
        let coverage = SourceCoverage::of(&checks, &plan, &stuck);
        assert!(coverage.kept.is_empty(), "{coverage:?}");
        assert_eq!(coverage.unkept(), 2);
        let record = coverage.record(false);
        assert_eq!(
            (
                record.checks[0].status.as_str(),
                record.checks[0].reason.as_deref()
            ),
            ("cannot_run", Some("`pytest` is not installed"))
        );
    }

    /// A heading is a check heading by its first or last word, never by a
    /// word in the middle or a file name.
    #[test]
    fn check_headings_are_named_by_their_first_or_last_word() {
        for title in [
            "Verification",
            "Acceptance Criteria",
            "Testing Checkpoint",
            "Key Invariants",
            "5. Verification",
            "Done when",
        ] {
            assert!(is_check_heading(title), "{title}");
        }
        for title in [
            "Academic References Verified",
            "Unit 7: Old Plan Cleanup (Verification Only)",
            "`verification/summary.rs`",
            "2.3 CheckpointManager",
            "What to build",
        ] {
            assert!(!is_check_heading(title), "{title}");
        }
    }

    /// bug-3cdc58: round v7's phase-0 spec states its 15 checks as a table
    /// whose Command column holds one command per check, and the plan written
    /// from it runs 12 of them, with its own AC ids in `covers` and no SC id
    /// anywhere. Each check gets its quoted command as its `run`, and a step
    /// that runs it keeps it, named or not: 12 of 15 kept, and the three no
    /// step runs (P0.07, P0.14, P0.15) missing. bug-56a806: each of the 12
    /// steps runs a grep of the planner's own first, so it keeps its check
    /// only once roko runs the spec's command alone.
    #[test]
    fn a_step_running_a_checks_command_keeps_it() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p0_source_checks");
        let request = std::fs::read_to_string(fixture.join("request.md")).expect("the request");
        let mut plan = TasksFile::parse(&fixture.join("tasks.toml")).expect("the plan");
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("tools")).expect("tools");
        for script in ["spec.sh", "pluginval.sh"] {
            std::fs::write(temp.path().join("tools").join(script), "#!/bin/sh\n")
                .expect("a script");
        }
        let mut checks = source_checks(&request);
        assert_eq!(checks.len(), 15);
        attach_runs(&mut checks, temp.path(), &Probe::with(&[], &[]));
        assert_eq!(
            checks[0].run.as_deref(),
            Some("tools/spec.sh 0 params 'P0.01*'")
        );
        assert_eq!(checks[13].run.as_deref(), Some("tools/pluginval.sh 5"));
        assert_eq!(checks[14].run.as_deref(), Some("tools/check.sh --plugin"));

        let guarded = SourceCoverage::of(&checks, &plan, &[]);
        assert!(guarded.kept.is_empty(), "{:?}", guarded.detail_lines());
        assert_eq!(guarded.elsewhere.len(), 12);
        assert_eq!(run_exactly(&mut plan, &checks).len(), 12);
        let coverage = SourceCoverage::of(&checks, &plan, &[]);
        let missing: Vec<&str> = coverage
            .missing()
            .iter()
            .map(|check| check.id.as_str())
            .collect();
        assert_eq!(missing, ["SC7", "SC14", "SC15"]);
        assert_eq!(coverage.kept.len(), 12, "{:?}", coverage.detail_lines());
        assert_eq!(coverage.unkept(), 3);
        assert_eq!(
            coverage.kept.get("SC1"),
            Some(&("T2".to_string(), "AC1".to_string()))
        );
    }

    /// bug-3cdc58: a code span is a check's command when it runs a workspace
    /// script or a program this machine has, with what it is given; a file
    /// name, a symbol or a signature is not one, and neither is either of two
    /// commands a check quotes. A step runs a check's command when one of its
    /// commands is that command word for word, quotes aside.
    #[test]
    fn a_check_quoting_one_command_runs_it() {
        let temp = tempfile::tempdir().expect("tempdir");
        let probe = Probe::with(&[("make", true), ("cmake", true), ("ctest", true)], &[]);
        let run = |text: &str| quoted_command(text, temp.path(), &probe);
        assert_eq!(
            run("P0.14 | `tools/pluginval.sh 5` | passes").as_deref(),
            Some("tools/pluginval.sh 5")
        );
        assert_eq!(run("- `make test` passes").as_deref(), Some("make test"));
        assert_eq!(run("`./run.sh` prints ok").as_deref(), Some("./run.sh"));
        for text in [
            "`docs/PRD.md` section 4 lists them",
            "`kNumParams` is 57",
            "`process(float* left, float* right, int n)` writes every sample",
            "`make` alone",
            "`cmake --preset core` builds, then `ctest --preset core` passes",
            "`make test` and `make lint` pass",
        ] {
            assert_eq!(run(text), None, "{text}");
        }
        assert!(runs_command(
            "grep -q 'kNumParams' core/params.h && tools/spec.sh 0 params \"P0.01*\"",
            "tools/spec.sh 0 params 'P0.01*'"
        ));
        assert!(!runs_command(
            "tools/spec.sh 0 params 'P0.02*'",
            "tools/spec.sh 0 params 'P0.01*'"
        ));
    }

    /// bug-96115c: round v7's phase-1 spec says, in its notes for the
    /// planner, "`tools/check.sh` (every present spec test, phases 0 and 1)
    /// is the plan's final check, not any one task's", and the plan written
    /// from it had no `[[meta.verify]]`, so nothing in the run checked the two
    /// phases together. The sentence is lifted as a check of the plan's own,
    /// missing from that plan; generation adds `[[meta.verify]]` running its
    /// command, which keeps it, and a task step that runs it would not.
    #[test]
    fn a_specs_named_final_check_becomes_meta_verify() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p1_final_check");
        let request = std::fs::read_to_string(fixture.join("request.md")).expect("the request");
        let mut plan = TasksFile::parse(&fixture.join("tasks.toml")).expect("the plan");
        let temp = tempfile::tempdir().expect("tempdir");
        let mut checks = source_checks(&request);
        attach_runs(&mut checks, temp.path(), &Probe::with(&[], &[]));
        let final_checks: Vec<&SourceCheck> =
            checks.iter().filter(|check| check.plan_level).collect();
        assert_eq!(final_checks.len(), 1, "{checks:?}");
        let final_check = final_checks[0];
        assert_eq!(final_check.run.as_deref(), Some("tools/check.sh"));
        assert_eq!(checks.len(), 21, "the 20 table checks and the final one");

        let coverage = SourceCoverage::of(&checks, &plan, &[]);
        let missing: Vec<&str> = coverage
            .missing()
            .iter()
            .map(|check| check.id.as_str())
            .collect();
        assert_eq!(missing, [final_check.id.as_str()]);

        let added = add_plan_checks(&mut plan, &checks);
        assert_eq!(
            added,
            [format!(
                "added `[[meta.verify]]` running `tools/check.sh`: the source names it the \
                 plan's final check ({})",
                final_check.id
            )]
        );
        assert_eq!(plan.meta.verify.len(), 1);
        assert_eq!(plan.meta.verify[0].command, "tools/check.sh");
        let coverage = SourceCoverage::of(&checks, &plan, &[]);
        assert_eq!(coverage.unkept(), 0, "{:?}", coverage.detail_lines());
        assert!(add_plan_checks(&mut plan, &checks).is_empty(), "added once");
    }

    /// bug-96115c: the phrases that make a line the plan's own check, and
    /// one that does not: a task's last step is a task's.
    #[test]
    fn a_line_naming_the_plans_own_check_is_lifted() {
        for text in [
            "Run `make check` as the plan's final check.",
            "`tools/check.sh --plugin` is the whole-plan check.",
            "The plan-level check: `npm test`.",
            "Use `cargo test --workspace` as the final check of the plan.",
        ] {
            let checks = source_checks(&format!("# Spec\n\n## Notes\n\n{text}\n"));
            assert_eq!(checks.len(), 1, "{text}: {checks:?}");
            assert!(checks[0].plan_level, "{text}");
        }
        let checks = source_checks("# Spec\n\n## Notes\n\nAs a final check, run `make lint`.\n");
        assert!(checks.is_empty(), "{checks:?}");
    }

    /// bug-72dc0d: round v7's second phase-2 plan write (op 9fa7849f)
    /// checked every task with cargo in a CMake repo, and kept 1 of the
    /// spec's 20 checks. Each check's own command now runs in the task that
    /// owns it, found by the file its command names: T1 runs the LFO checks,
    /// and so on. The third write (op 87df1c1b) quoted every check in its
    /// criteria while its steps ran cargo: that keeps none of them, where it
    /// counted 10.
    #[test]
    fn the_source_commands_run_in_the_tasks_that_own_them() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p2_spec_checks");
        let request = std::fs::read_to_string(fixture.join("request.md")).expect("the spec");
        let temp = tempfile::tempdir().expect("tempdir");
        let mut checks = source_checks(&request);
        attach_runs(&mut checks, temp.path(), &Probe::with(&[], &[]));
        assert_eq!(checks.len(), 20);
        assert_eq!(
            checks[3].run.as_deref(),
            Some("tools/spec.sh 2 rumble 'P2.04*'")
        );
        let cargo_steps = |plan: &TasksFile| -> Vec<Unrunnable> {
            plan.tasks
                .iter()
                .flat_map(|task| {
                    (1..)
                        .zip(&task.verify)
                        .filter(|(_, step)| step.command.contains("cargo "))
                        .map(|(step, _)| Unrunnable {
                            task: task.id.clone(),
                            step,
                            why: "it runs `cargo`, which needs a `Cargo.toml`".to_string(),
                        })
                })
                .collect()
        };

        let mut plan = TasksFile::parse(&fixture.join("plan2.toml")).expect("plan 2");
        let stuck = cargo_steps(&plan);
        assert_eq!(stuck.len(), 4);
        let before = SourceCoverage::of(&checks, &plan, &stuck);
        assert_eq!(before.kept.len(), 1, "only [[meta.verify]]");
        let assigned = assign_runs(&mut plan, &checks, &stuck, |_| 4);
        assert_eq!(assigned.len(), 4, "{assigned:#?}");
        let command = |id: &str| {
            plan.tasks
                .iter()
                .find(|task| task.id == id)
                .expect("a task")
                .verify[0]
                .command
                .clone()
        };
        // bug-56a806: the steps run the spec's commands alone, without the
        // file tests and greps of the steps they replace.
        assert_eq!(
            command("T1"),
            "tools/spec.sh 2 lfo 'P2.01*' && tools/spec.sh 2 lfo 'P2.02*' && tools/spec.sh 2 lfo \
             'P2.03*'"
        );
        assert_eq!(
            command("T2"),
            "tools/spec.sh 2 rumble 'P2.04*' && tools/spec.sh 2 rumble 'P2.05*' && tools/spec.sh \
             2 rumble 'P2.06*' && tools/spec.sh 2 rumble 'P2.07*' && tools/spec.sh 2 rumble \
             'P2.08*'"
        );
        assert!(command("T3").contains("tools/spec.sh 2 output 'P2.12*'"));
        assert!(command("T4").starts_with("tools/spec.sh 2 engine 'P2.13*' && "));
        assert!(
            plan.tasks
                .iter()
                .flat_map(|task| &task.verify)
                .all(|step| !step.command.contains("cargo"))
        );
        let after = SourceCoverage::of(&checks, &plan, &[]);
        assert_eq!(after.kept.len(), 20, "{:#?}", after.detail_lines());

        let mut plan = TasksFile::parse(&fixture.join("plan3.toml")).expect("plan 3");
        let quoted = SourceCoverage::of(&checks, &plan, &[]);
        assert_eq!(quoted.kept.len(), 1, "{:#?}", quoted.detail_lines());
        assert_eq!(quoted.elsewhere.len(), 19);
        assert!(
            quoted
                .summary_line()
                .ends_with("19 covered by a step that runs another command"),
            "{}",
            quoted.summary_line()
        );
        // Its criteria name the checks, so apply_runs gives each step the
        // commands of the checks it covers, and nothing else (bug-56a806):
        // T1's one step covers P2.01 to P2.03.
        let stuck = cargo_steps(&plan);
        let applied = apply_runs(&mut plan, &checks, &stuck);
        assert_eq!(applied.len(), 4, "{applied:#?}");
        assert_eq!(
            plan.tasks[0].verify[0].command,
            "tools/spec.sh 2 lfo 'P2.01*' && tools/spec.sh 2 lfo 'P2.02*' && tools/spec.sh 2 lfo \
             'P2.03*'"
        );
        let applied = SourceCoverage::of(&checks, &plan, &[]);
        assert_eq!(applied.kept.len(), 20, "{:#?}", applied.detail_lines());
    }

    /// Round v7, phase 0: asked naturally, the chat sent roko a 10-line
    /// summary that names the spec and states no check. The spec it names
    /// is read, and its checks are the 15 the spec states, as when the spec
    /// itself is the source. A path outside the workspace, a missing file or
    /// a file that is no markdown document is not one.
    #[test]
    fn a_summary_names_the_spec_whose_checks_it_keeps() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p0_source_checks");
        let spec = std::fs::read_to_string(fixture.join("request.md")).expect("the spec");
        let summary = std::fs::read_to_string(fixture.join("summary.md")).expect("the summary");
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        std::fs::create_dir_all(workdir.join("docs/specs")).expect("docs");
        std::fs::write(workdir.join("docs/specs/phase-0-foundation.md"), &spec).expect("spec");
        assert!(source_checks(&summary).is_empty());

        let documents = named_documents(&summary, workdir);
        let paths: Vec<&str> = documents
            .iter()
            .map(|document| document.path.as_str())
            .collect();
        assert_eq!(paths, ["docs/specs/phase-0-foundation.md"]);
        assert_eq!(document_checks(&documents), source_checks(&spec));
        assert_eq!(document_checks(&documents).len(), 15);

        std::fs::write(workdir.join("CMakeLists.txt"), "# Checks\n").expect("cmake");
        std::fs::write(workdir.join("README.md"), "# Undertow\n").expect("readme");
        let outside = tempfile::tempdir().expect("tempdir");
        std::fs::write(outside.path().join("other.md"), &spec).expect("outside");
        let request = format!(
            "Plan from `{}`, see README.md, ../other.md, {}, docs/specs/missing.md, \
             CMakeLists.txt and (docs/specs/phase-0-foundation.md).",
            workdir.join("docs/specs/phase-0-foundation.md").display(),
            outside.path().join("other.md").display(),
        );
        let paths: Vec<String> = named_documents(&request, workdir)
            .into_iter()
            .map(|document| document.path)
            .collect();
        assert_eq!(paths, ["docs/specs/phase-0-foundation.md", "README.md"]);
    }

    /// Round v7, phase 3: rows of a spec's table can share one command
    /// (`tools/spec.sh 3 sequencer 'P3.01*,P3.02*'`). The task that owns them
    /// runs it in one step, once, and that step keeps every row.
    #[test]
    fn checks_that_share_a_command_run_once() {
        let source = "# Phase 3\n\n## Acceptance checks\n\n| Id | Command | Check |\n|---|---|---|\n\
                      | P3.01 | `tools/spec.sh 3 sequencer 'P3.01*,P3.02*'` | the steps play in order |\n\
                      | P3.02 | `tools/spec.sh 3 sequencer 'P3.01*,P3.02*'` | a tie slides on |\n";
        let temp = tempfile::tempdir().expect("tempdir");
        let mut checks = source_checks(source);
        attach_runs(&mut checks, temp.path(), &Probe::with(&[], &[]));
        assert_eq!(checks.len(), 2, "{checks:?}");
        let mut plan = TasksFile::parse_str(
            "[meta]\nplan = \"p3\"\ntotal = 1\nstatus = \"ready\"\n\n\
             [[task]]\nid = \"T1\"\ntitle = \"The sequencer\"\nstatus = \"ready\"\n\
             role = \"implementer\"\ntier = \"focused\"\nfiles = [\"core/src/sequencer.cpp\"]\n",
        )
        .expect("a plan");
        let assigned = assign_runs(&mut plan, &checks, &[], |_| 4);
        assert_eq!(assigned.len(), 1, "{assigned:?}");
        let commands: Vec<&str> = plan.tasks[0]
            .verify
            .iter()
            .map(|step| step.command.as_str())
            .collect();
        assert_eq!(commands, ["tools/spec.sh 3 sequencer 'P3.01*,P3.02*'"]);
        assert_eq!(SourceCoverage::of(&checks, &plan, &[]).kept.len(), 2);
    }

    /// bug-56a806: round v7's third phase-2 plan write (op 1a965a97) runs
    /// each of the spec's 15 commands after a grep the planner made up, such
    /// as T4's `grep -q 'AllocationCounter' core/src/engine.cpp`, a name only
    /// the spec's tests use. Read as written, those steps keep none of the 23
    /// table checks: each runs another command than its check's. Through
    /// generation's repairs, every step runs one of the spec's commands alone,
    /// and each check is kept by the step that runs exactly its command, the
    /// plan's own check by `[[meta.verify]]`.
    #[test]
    fn a_step_runs_the_spec_commands_exactly() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p2_exact_commands");
        let request = std::fs::read_to_string(fixture.join("request.md")).expect("the spec");
        let mut plan = TasksFile::parse(&fixture.join("tasks.toml")).expect("the plan");
        let temp = tempfile::tempdir().expect("tempdir");
        let mut checks = source_checks(&request);
        attach_runs(&mut checks, temp.path(), &Probe::with(&[], &[]));
        assert_eq!(checks.len(), 24);
        let commands: BTreeSet<&str> = source_runs(&checks, false).into_iter().collect();
        assert_eq!(commands.len(), 15, "{commands:#?}");
        assert_eq!(source_runs(&checks, true), ["tools/check.sh"]);

        let guarded = SourceCoverage::of(&checks, &plan, &[]);
        assert_eq!(
            guarded.kept.keys().collect::<Vec<_>>(),
            ["SC24"],
            "{:#?}",
            guarded.detail_lines()
        );
        assert_eq!(
            guarded.summary_line(),
            "Source checks: 1 of 24 kept as verify steps, 23 covered by a step that runs another \
             command"
        );

        // Generation's repairs, in order; none of the steps is stuck.
        let mut repairs = apply_runs(&mut plan, &checks, &[]);
        repairs.extend(assign_runs(&mut plan, &checks, &[], |_| 6));
        repairs.extend(run_exactly(&mut plan, &checks));
        assert_eq!(repairs.len(), 15, "{repairs:#?}");
        assert_eq!(
            repairs[14],
            "T4 step 4: `grep -q 'AllocationCounter' core/src/engine.cpp && tools/spec.sh 2 \
             engine 'P2.18*,P2.19*'` became `tools/spec.sh 2 engine 'P2.18*,P2.19*'`, the \
             source's own commands alone"
        );
        let steps: Vec<&VerifyStep> = plan.tasks.iter().flat_map(|task| &task.verify).collect();
        assert_eq!(steps.len(), 15);
        for step in steps {
            assert!(commands.contains(step.command.as_str()), "{}", step.command);
        }
        let exact = SourceCoverage::of(&checks, &plan, &[]);
        assert_eq!(exact.kept.len(), 24, "{:#?}", exact.detail_lines());
        assert_eq!(
            exact.kept.get("SC19"),
            Some(&("T4".to_string(), "AC2".to_string()))
        );
        assert_eq!(
            exact.summary_line(),
            "Source checks: 24 of 24 kept as verify steps"
        );
        assert!(run_exactly(&mut plan, &checks).is_empty(), "exact once");
    }

    /// bug-56a806: a step runs a source command exactly when it is that
    /// command, as the source writes it, or a `&&` join of source commands.
    /// Run exactly, steps lose any other command around them (a guard, a
    /// pipe); one that comes out the same as an earlier step joins it, since
    /// a generated plan may not repeat a check; a step that runs no source
    /// command stays as it is. At its step limit, a task's unassigned checks
    /// join its last step that runs source commands.
    #[test]
    fn steps_run_the_source_commands_alone_and_once() {
        let source = "# Phase 3\n\n## Acceptance checks\n\n| Id | Command | Check |\n|---|---|---|\n\
                      | P3.01 | `tools/spec.sh 3 sequencer 'P3.01*,P3.02*'` | the steps play in order |\n\
                      | P3.02 | `tools/spec.sh 3 sequencer 'P3.01*,P3.02*'` | a tie slides on |\n\
                      | P3.03 | `tools/spec.sh 3 sequencer 'P3.03*'` | a rest is silent |\n\n\
                      ## Notes\n\n`tools/check.sh` is the plan's final check.\n";
        let temp = tempfile::tempdir().expect("tempdir");
        let mut checks = source_checks(source);
        attach_runs(&mut checks, temp.path(), &Probe::with(&[], &[]));
        assert_eq!(checks.len(), 4, "{checks:?}");
        let runs = source_runs(&checks, false);
        let (pair, rest) = (runs[0], runs[2]);
        assert!(runs_exactly(rest, rest, &runs));
        assert!(runs_exactly(&format!("{pair} && {rest}"), rest, &runs));
        for command in [
            format!("grep -q rest core/src/sequencer.cpp && {rest}"),
            format!("{rest} && make lint"),
            "tools/spec.sh 3 sequencer \"P3.03*\"".to_string(),
            pair.to_string(),
        ] {
            assert!(!runs_exactly(&command, rest, &runs), "{command}");
        }

        let task = "[meta]\nplan = \"p3\"\ntotal = 1\nstatus = \"ready\"\n\n\
                    [[meta.verify]]\nphase = \"test\"\n\
                    command = \"test -x tools/check.sh && tools/check.sh\"\n\n\
                    [[task]]\nid = \"T1\"\ntitle = \"The sequencer\"\nstatus = \"ready\"\n\
                    role = \"implementer\"\ntier = \"focused\"\n\
                    files = [\"core/src/sequencer.cpp\"]\n\
                    acceptance = [\"AC1: steps in order\", \"AC2: ties\", \"AC3: rests\"]\n";
        let step = |command: &str, cover: &str| {
            format!(
                "\n[[task.verify]]\nphase = \"test\"\ncommand = {command:?}\ncovers = \
                 [\"{cover}\"]\n"
            )
        };
        let mut plan = TasksFile::parse_str(&format!(
            "{task}{}{}{}{}",
            step(
                "grep -q 'class Sequencer' core/src/sequencer.cpp && tools/spec.sh 3 sequencer \
                 \"P3.01*,P3.02*\"",
                "AC1"
            ),
            step(
                "grep -q tie core/src/sequencer.cpp && tools/spec.sh 3 sequencer 'P3.01*,P3.02*'",
                "AC2"
            ),
            step(
                "cmake --build --preset core && ctest --preset core -R rest",
                "AC3"
            ),
            step("tools/spec.sh 3 sequencer 'P3.03*' 2>&1 | tail -20", "AC3"),
        ))
        .expect("a plan");
        let changed = run_exactly(&mut plan, &checks);
        assert_eq!(
            changed,
            [
                format!(
                    "T1 step 1: `grep -q 'class Sequencer' core/src/sequencer.cpp && \
                     tools/spec.sh 3 sequencer \"P3.01*,P3.02*\"` became `{pair}`, the source's \
                     own commands alone"
                ),
                format!(
                    "T1 step 2: `grep -q tie core/src/sequencer.cpp && {pair}` became `{pair}`, \
                     the source's own commands alone, which step 1 runs already, so it joined \
                     that step"
                ),
                format!(
                    "T1 step 4: `{rest} 2>&1 | tail -20` became `{rest}`, the source's own \
                     commands alone"
                ),
                "`[[meta.verify]]` step 1: `test -x tools/check.sh && tools/check.sh` became \
                 `tools/check.sh`, the source's own commands alone"
                    .to_string(),
            ]
        );
        let steps: Vec<(&str, Vec<&str>)> = plan.tasks[0]
            .verify
            .iter()
            .map(|step| {
                (
                    step.command.as_str(),
                    step.covers.iter().map(String::as_str).collect(),
                )
            })
            .collect();
        assert_eq!(
            steps,
            [
                (pair, vec!["AC1", "AC2"]),
                (
                    "cmake --build --preset core && ctest --preset core -R rest",
                    vec!["AC3"]
                ),
                (rest, vec!["AC3"]),
            ]
        );
        assert_eq!(plan.meta.verify[0].command, "tools/check.sh");
        let coverage = SourceCoverage::of(&checks, &plan, &[]);
        assert_eq!(coverage.kept.len(), 4, "{:#?}", coverage.detail_lines());

        // At the step limit, P3.03 joins the step that runs P3.01 and P3.02;
        // the planner's own build step stays.
        let mut plan = TasksFile::parse_str(&format!(
            "{task}{}{}",
            step(
                &format!("grep -q tie core/src/sequencer.cpp && {pair}"),
                "AC1"
            ),
            step("cmake --build --preset core", "AC2"),
        ))
        .expect("a plan");
        let assigned = assign_runs(&mut plan, &checks, &[], |_| 2);
        assert_eq!(assigned.len(), 1, "{assigned:?}");
        let commands: Vec<&str> = plan.tasks[0]
            .verify
            .iter()
            .map(|step| step.command.as_str())
            .collect();
        assert_eq!(
            commands,
            [
                format!("{pair} && {rest}").as_str(),
                "cmake --build --preset core"
            ]
        );
    }

    /// gap-115c2a: roko runs a source check's own command alone (bug-56a806),
    /// so a check that needs a build must build in its own command. The
    /// planner hears it with the checks and when asked again; checks stated
    /// in prose, which have no command, do not bring it.
    #[test]
    fn the_planner_is_told_a_check_command_runs_as_written() {
        let source = "# Widget\n\n## Acceptance checks\n\n| Id | Command | Check |\n|---|---|---|\n\
                      | W.01 | `tools/spec.sh widget spin` | the widget spins |\n";
        let temp = tempfile::tempdir().expect("tempdir");
        let mut checks = source_checks(source);
        attach_runs(&mut checks, temp.path(), &Probe::with(&[], &[]));
        assert_eq!(checks[0].run.as_deref(), Some("tools/spec.sh widget spin"));
        assert!(SELF_CONTAINED.contains("must be self-contained"));
        let block = planner_block(&checks, "The prompt states");
        assert!(block.contains(SELF_CONTAINED), "{block}");
        let plan = TasksFile::parse_str(
            "[meta]\nplan = \"w\"\ntotal = 1\nstatus = \"ready\"\n\n\
             [[task]]\nid = \"T1\"\ntitle = \"Spin\"\nstatus = \"ready\"\nrole = \"implementer\"\n\
             tier = \"focused\"\n",
        )
        .expect("a plan");
        let coverage = SourceCoverage::of(&checks, &plan, &[]);
        let again = regeneration_prompt("Spin the widget.", "[meta]", &coverage, &[], &[]);
        assert!(again.contains(SELF_CONTAINED), "{again}");

        let prose = source_checks("## Verification\n\n- the widget spins\n");
        assert!(!planner_block(&prose, "The prompt states").contains(SELF_CONTAINED));
    }
}
