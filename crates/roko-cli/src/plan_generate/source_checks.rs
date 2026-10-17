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
//! source states them.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::LazyLock;

use regex::Regex;
use roko_learn::runtime_feedback::{GenerationSourceCheck, GenerationSourceChecks};

use crate::task_parser::TasksFile;

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

/// One check the source states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourceCheck {
    /// `SC1`, `SC2`, … in source order.
    pub id: String,
    /// The check, on one line.
    pub text: String,
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

/// The source check ids an acceptance item names (`AC2: SC5: …`).
fn named_checks(item: &str) -> Vec<String> {
    static ID: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\bSC\d+\b").expect("valid source check id regex"));
    ID.find_iter(item)
        .map(|found| found.as_str().to_string())
        .collect()
}

/// What became of a source's checks in a plan.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct SourceCoverage {
    checks: Vec<SourceCheck>,
    /// By check id: the task and the acceptance criterion, a verify step
    /// covers, that keep it.
    kept: BTreeMap<String, (String, String)>,
    /// By check id: why the plan drops it (`[meta] dropped_checks`).
    dropped: BTreeMap<String, String>,
}

impl SourceCoverage {
    /// What became of `checks` in `plan`. A check is kept when an
    /// acceptance criterion names it and a verify step of the same task
    /// `covers` that criterion; dropped when `[meta] dropped_checks` gives a
    /// reason for it; and missing otherwise.
    pub(super) fn of(checks: &[SourceCheck], plan: &TasksFile) -> Self {
        let known = |id: &str| checks.iter().any(|check| check.id == id);
        let mut kept = BTreeMap::new();
        for task in &plan.tasks {
            for (index, item) in task.acceptance.iter().enumerate() {
                let criterion = roko_gate::spec_quality::ac_id(item, index);
                let covered = task
                    .verify
                    .iter()
                    .any(|step| step.covers.iter().any(|id| id.trim() == criterion));
                if !covered {
                    continue;
                }
                for id in named_checks(item) {
                    if known(&id) {
                        kept.entry(id)
                            .or_insert_with(|| (task.id.clone(), criterion.clone()));
                    }
                }
            }
        }
        let dropped = plan
            .meta
            .dropped_checks
            .iter()
            .filter_map(|entry| {
                let (id, reason) = entry.split_once(':')?;
                let (id, reason) = (id.trim(), reason.trim());
                (known(id) && !reason.is_empty() && !kept.contains_key(id))
                    .then(|| (id.to_string(), reason.to_string()))
            })
            .collect();
        Self {
            checks: checks.to_vec(),
            kept,
            dropped,
        }
    }

    /// The checks the plan neither keeps nor drops with a reason.
    pub(super) fn missing(&self) -> Vec<&SourceCheck> {
        self.checks
            .iter()
            .filter(|check| {
                !self.kept.contains_key(&check.id) && !self.dropped.contains_key(&check.id)
            })
            .collect()
    }

    /// One line for the person: how many checks the plan keeps.
    pub(super) fn summary_line(&self) -> String {
        let mut line = format!(
            "Source checks: {} of {} kept as verify steps",
            self.kept.len(),
            self.checks.len()
        );
        if !self.dropped.is_empty() {
            let _ = write!(line, ", {} dropped with a reason", self.dropped.len());
        }
        let missing = self.missing().len();
        if missing > 0 {
            let _ = write!(line, ", {missing} in no verify step");
        }
        line
    }

    /// A line for each check the plan does not keep: why it was dropped, or
    /// that it is missing.
    pub(super) fn detail_lines(&self) -> Vec<String> {
        self.checks
            .iter()
            .filter_map(|check| {
                if let Some(reason) = self.dropped.get(&check.id) {
                    Some(format!("dropped {}: {} ({reason})", check.id, check.text))
                } else if self.kept.contains_key(&check.id) {
                    None
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
                let reason = self.dropped.get(&check.id);
                GenerationSourceCheck {
                    id: check.id.clone(),
                    check: check.text.clone(),
                    status: if kept.is_some() {
                        "kept"
                    } else if reason.is_some() {
                        "dropped"
                    } else {
                        "missing"
                    }
                    .to_string(),
                    task: kept.map(|(task, _)| task.clone()),
                    criterion: kept.map(|(_, criterion)| criterion.clone()),
                    reason: reason.cloned(),
                }
            })
            .collect();
        GenerationSourceChecks {
            total: self.checks.len(),
            kept: self.kept.len(),
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
         The {kind} states the {} checks below. Keep every one as a verify step: put it in the \
         `acceptance` of the task that delivers it as `ACn: SCk: <the check>`, and give that \
         task a verify step whose `covers` names `ACn`. A check written as a command can be that \
         step's command. Drop a check only when no command can check it (it needs a person, \
         hardware, or another plan's work first), and then list it in `[meta] dropped_checks` \
         as `\"SCk: <why>\"`. Never merge two checks into one criterion, reword one away, or \
         leave one out.\n\n",
        checks.len()
    );
    for check in checks {
        let _ = writeln!(block, "- {}: {}", check.id, check.text);
    }
    block
}

/// The prompt that asks the planner once more for a plan that leaves
/// source checks `missing`: the original request, the plan it wrote, and
/// each such check.
pub(super) fn regeneration_prompt(
    task_prompt: &str,
    plan: &str,
    coverage: &SourceCoverage,
) -> String {
    let missing: Vec<String> = coverage
        .missing()
        .iter()
        .map(|check| format!("- {}: {}", check.id, check.text))
        .collect();
    format!(
        "{task_prompt}\n\n---\n\nYour plan below keeps {} of the {} source checks as verify \
         steps. These are in no verify step, and `[meta] dropped_checks` does not list them:\n\
         {}\n\n```toml\n{plan}\n```\n\n\
         Add each of them to the `acceptance` of the task that delivers it as \
         `ACn: SCk: <the check>`, with a verify step whose `covers` names `ACn`; or, when no \
         command can check it, list it in `[meta] dropped_checks` as `\"SCk: <why>\"`. Keep the \
         rest of the plan as it is. Output the complete plan as a single ```toml fenced block, \
         followed only by the ```accept:accept/<file> blocks of the tests it pins.",
        coverage.kept.len(),
        coverage.checks.len(),
        missing.join("\n")
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
}
