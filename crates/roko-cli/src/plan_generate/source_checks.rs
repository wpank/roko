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
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use roko_gate::spec_quality::ac_id;
use roko_learn::runtime_feedback::{GenerationSourceCheck, GenerationSourceChecks};

use super::runnable::{Probe, Unchecked, Unrunnable, check_command, command_words, only_reads};
use crate::task_parser::{TasksFile, VerifyStep};

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
    /// By check id: why the plan drops it (`[meta] dropped_checks`).
    dropped: BTreeMap<String, String>,
}

impl SourceCoverage {
    /// What became of `checks` in `plan`, whose `unrunnable` steps cannot
    /// pass as written. A check is kept when a verify step that can run
    /// covers it (or an acceptance criterion that names it), or runs its own
    /// command (bug-3cdc58), and runs more than a grep, a file test or a
    /// build; it cannot run when only steps that
    /// cannot pass cover it otherwise; shallow when only grep, file test or
    /// build steps do; dropped when `[meta] dropped_checks` gives a reason for
    /// it that does more than restate it; and missing otherwise.
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
        let mut kept = BTreeMap::new();
        let mut shallow = BTreeMap::new();
        let mut cannot_run = BTreeMap::new();
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
                // whether or not anything names the check.
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
                if let Some(stuck) = stuck {
                    cannot_run
                        .entry(id)
                        .or_insert_with(|| (task.id.clone(), stuck.why.clone()));
                } else if is_shallow(command) {
                    shallow
                        .entry(id)
                        .or_insert_with(|| (task.id.clone(), command.to_string()));
                } else {
                    shallow.remove(&id);
                    cannot_run.remove(&id);
                    kept.insert(id, (task.id.clone(), criterion));
                }
            }
        }
        for check in checks.iter().filter(|check| check.plan_level) {
            let ran = check.run.as_deref().is_some_and(|run| {
                plan.meta
                    .verify
                    .iter()
                    .any(|step| runs_command(&step.command, run))
            });
            if ran {
                kept.insert(
                    check.id.clone(),
                    ("plan".to_string(), "[[meta.verify]]".to_string()),
                );
            }
        }
        shallow.retain(|id, _| !cannot_run.contains_key(id));
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
        Self {
            checks: checks.to_vec(),
            kept,
            shallow,
            cannot_run,
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
                    task: kept.or(stuck).or(shallow).map(|(task, _)| task.clone()),
                    criterion: kept.map(|(_, criterion)| criterion.clone()),
                    reason: dropped
                        .cloned()
                        .or_else(|| stuck.map(|(_, why)| why.clone()))
                        .or_else(|| {
                            shallow.map(|(_, command)| format!("only `{}`", one_line(command)))
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
            missing: self.missing().len(),
            regenerated,
            checks,
        }
    }
}

/// The planner prompt's list of the source's `checks`, with how to keep
/// each one; empty when the source states none.
pub(super) fn planner_block(checks: &[SourceCheck], kind: &str) -> String {
    if checks.is_empty() {
        return String::new();
    }
    let mut block = format!(
        "## Source checks\n\n\
         The {kind} states the {} checks below. Keep each one as a verify step of the task that \
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
            "A check given with `run:` has its command already: use that command as its step's \
             command, unchanged.\n\n",
        );
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

/// Whether `command`, a verify step's, runs `run`, a check's command: each
/// of `run`'s commands is one of `command`'s, word for word, quotes aside
/// (bug-3cdc58).
fn runs_command(command: &str, run: &str) -> bool {
    let commands = |text: &str| -> Vec<Vec<String>> {
        shell_segments(text)
            .iter()
            .map(|segment| command_words(segment))
            .filter(|words| !words.is_empty())
            .collect()
    };
    let (have, wanted) = (commands(command), commands(run));
    !wanted.is_empty() && wanted.iter().all(|words| have.contains(words))
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
/// wildcard check gets the real ones. Returns one line per step changed.
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
            let named: Vec<String> = step
                .covers
                .iter()
                .map(|cover| cover.trim())
                .flat_map(|cover| {
                    task.acceptance
                        .iter()
                        .enumerate()
                        .filter(|(index, item)| ac_id(item, *index) == cover)
                        .flat_map(|(_, item)| named_checks(item))
                        .chain(named_checks(cover))
                        .collect::<Vec<_>>()
                })
                .collect();
            // Every check it covers must have a command of its own.
            let Some(mut runs) = named
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
            runs.dedup();
            if runs.is_empty() {
                continue;
            }
            let command = runs.join(" && ");
            applied.push(format!(
                "{} step {number}: `{}` became `{command}`, the source check's own tests",
                task.id, step.command
            ));
            step.command = command;
        }
    }
    applied
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
    format!(
        "{task_prompt}\n\n---\n\n{kept}{problems}\n```toml\n{plan}\n```\n\n\
         Keep each source check as a verify step whose `covers` names it and whose command runs \
         a test of what it states: a test the repository has, or one the task writes, named in \
         its description and guarded by a `grep -q` for it. When no command can check one, list \
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
    /// from it runs 12 of them verbatim, with its own AC ids in `covers` and
    /// no SC id anywhere. Each check gets its quoted command as its `run`, and
    /// a step that runs it keeps it, named or not: 12 of 15 kept, and the
    /// three no step runs (P0.07, P0.14, P0.15) missing.
    #[test]
    fn a_step_running_a_checks_command_keeps_it() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p0_source_checks");
        let request = std::fs::read_to_string(fixture.join("request.md")).expect("the request");
        let plan = TasksFile::parse(&fixture.join("tasks.toml")).expect("the plan");
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
}
