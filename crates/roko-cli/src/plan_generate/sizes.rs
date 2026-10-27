//! The sizes a plan's source states for its tasks (gap-9e212c). A spec's
//! notes for the planner may size each task: "The rumble (`rumble.h`,
//! `rumble.cpp`): P2.04 to P2.08. … About 200 lines, so architectural." A
//! task's tier picks the model that runs it, so a task whose source states
//! its size gets the smallest tier whose line budget covers that size. Round
//! v7's phase-2 planner made all four tasks architectural, though the notes
//! sized the engine task at about 80 lines.

use std::sync::LazyLock;

use regex::Regex;
use roko_core::task::TaskTier;

use crate::plan_policy::PlanExecutionPolicy;
use crate::task_parser::{TaskDef, TasksFile};

/// The number of tasks the "Notes for the planner" of `sources` state
/// (gap-5c7f9a): a number word up to twelve, or digits, right before "task"
/// or "tasks", opening a list item, a line or a sentence ("- One task, about
/// 5 lines: …", "Eight tasks."), in that section only. A count inside a
/// sentence ("not any one task's") states nothing. `None` when no source has
/// the section, it states no count, or the counts differ: a count roko cannot
/// read is no count.
pub(super) fn stated_task_count(sources: &[&str]) -> Option<usize> {
    static COUNT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?im)(?:^[ \t]*(?:[-*+]|\d+[.)])?[ \t]*|[.;:][ \t]+)(one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|\d{1,2})[ \t]+tasks?\b(['’]s)?",
        )
        .expect("valid task count regex")
    });
    const WORDS: [&str; 12] = [
        "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
        "twelve",
    ];
    let mut counts: Vec<usize> = sources
        .iter()
        .flat_map(|source| planner_notes(source))
        .flat_map(|notes| {
            COUNT
                .captures_iter(&notes)
                .filter(|captures| captures.get(2).is_none())
                .filter_map(|captures| {
                    let word = captures[1].to_lowercase();
                    word.parse().ok().or_else(|| {
                        WORDS
                            .iter()
                            .position(|named| *named == word)
                            .map(|at| at + 1)
                    })
                })
                .collect::<Vec<usize>>()
        })
        .collect();
    counts.dedup();
    match counts[..] {
        [count] if count > 0 => Some(count),
        _ => None,
    }
}

/// The "Notes for the planner" sections of `source`: from a heading that
/// names them to the next heading of the same level or higher.
fn planner_notes(source: &str) -> Vec<String> {
    let level = |line: &str| {
        let hashes = line.chars().take_while(|c| *c == '#').count();
        (hashes > 0 && line[hashes..].starts_with(' ')).then_some(hashes)
    };
    let mut sections = Vec::new();
    let mut open: Option<(usize, String)> = None;
    for line in source.lines() {
        if let Some(depth) = level(line) {
            if let Some((section_depth, _)) = &open
                && depth <= *section_depth
            {
                sections.extend(open.take().map(|(_, text)| text));
            }
            if open.is_none() && line.to_lowercase().contains("notes for the planner") {
                open = Some((depth, String::new()));
                continue;
            }
        }
        if let Some((_, text)) = &mut open {
            text.push_str(line);
            text.push('\n');
        }
    }
    sections.extend(open.map(|(_, text)| text));
    sections
}

/// One list item of a source that states a size: the files it names, and
/// the lines it says the work takes.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Sized {
    /// The files the item names in code spans (`rumble.h`), as written.
    files: Vec<String>,
    /// Its size in lines.
    lines: u32,
}

/// The list items of `source` that state a size ("about 200 lines") and name
/// files in code spans. An item is a line that starts with a list marker
/// (`2.` or `-`), with the more indented lines under it.
fn sized_items(source: &str) -> Vec<Sized> {
    static MARKER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(\s*)(?:\d+\.|[-*])\s+").expect("valid list marker regex"));
    static SIZE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\b(?:about|around|roughly|~)\s*(\d+)\s+lines\b").expect("valid size regex")
    });
    let mut items: Vec<(usize, String)> = Vec::new();
    for line in source.lines() {
        let indent = line.len() - line.trim_start().len();
        if let Some(marker) = MARKER.captures(line) {
            items.push((marker[1].len(), line.to_string()));
        } else if let Some((item_indent, text)) = items.last_mut()
            && !line.trim().is_empty()
            && indent > *item_indent
        {
            text.push(' ');
            text.push_str(line.trim());
        } else {
            // A line that is not under the item ends it.
            items.push((usize::MAX, String::new()));
        }
    }
    items
        .into_iter()
        .filter_map(|(_, text)| {
            let lines = SIZE.captures(&text)?[1].parse().ok()?;
            let files = code_span_files(&text);
            (!files.is_empty()).then_some(Sized { files, lines })
        })
        .collect()
}

/// The file names in the code spans of `text`: a span with an extension of
/// letters and digits, with a letter, and no space (`rumble.h`,
/// `core/CMakeLists.txt`), and the short form `PluginProcessor.h/.cpp` for its
/// two files. A test filter (`P2.04*`) is no file.
fn code_span_files(text: &str) -> Vec<String> {
    static SPAN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"`([^`\s]+)`").expect("valid code span regex"));
    let mut files = Vec::new();
    for span in SPAN
        .captures_iter(text)
        .filter_map(|captures| captures.get(1))
    {
        let span = span.as_str();
        let (first, others) = match span.split_once("/.") {
            Some((first, others)) => (first, Some(others)),
            None => (span, None),
        };
        let Some((stem, extension)) = first.rsplit_once('.') else {
            continue;
        };
        let extension_like = extension.chars().all(|c| c.is_ascii_alphanumeric())
            && extension.contains(|c: char| c.is_ascii_alphabetic());
        if stem.is_empty() || !extension_like {
            continue;
        }
        files.push(first.to_string());
        for extension in others.into_iter().flat_map(|others| others.split("/.")) {
            files.push(format!("{stem}.{extension}"));
        }
    }
    files
}

/// Whether the plan file `file` is the one a source calls `named`: the same
/// path, or a path that ends with it.
fn same_file(file: &str, named: &str) -> bool {
    file == named || file.ends_with(&format!("/{named}"))
}

/// Give each task of `plan` whose size its source states ([`sized_items`])
/// the smallest tier whose line budget covers that size, and whose other
/// limits (files, verify steps, description) the task keeps, when the
/// planner's tier is larger, with its `max_loc` within that tier's budget.
/// A planner's smaller tier stays as written; [`raise_tiers`] judges it. An
/// item goes to the task that writes most of the files it names, as a share
/// of that task's files; a tie goes to no task. Returns one line per repair.
pub(super) fn repair_tiers(
    plan: &mut TasksFile,
    sources: &[&str],
    policy: PlanExecutionPolicy,
) -> Vec<String> {
    let mut repairs = Vec::new();
    for item in sources.iter().flat_map(|source| sized_items(source)) {
        let shares: Vec<(usize, f64)> = plan
            .tasks
            .iter()
            .enumerate()
            .filter(|(_, task)| !task.files.is_empty())
            .map(|(index, task)| {
                let named = task
                    .files
                    .iter()
                    .filter(|file| item.files.iter().any(|named| same_file(file, named)))
                    .count();
                (index, named as f64 / task.files.len() as f64)
            })
            .filter(|(_, share)| *share > 0.0)
            .collect();
        let best = shares
            .iter()
            .map(|(_, share)| *share)
            .fold(0.0_f64, f64::max);
        let mut owners = shares
            .iter()
            .filter(|(_, share)| (*share - best).abs() < 1e-9);
        let (Some(&(owner, _)), None) = (owners.next(), owners.next()) else {
            continue;
        };
        let task = &mut plan.tasks[owner];
        // gap-4dd77d: never below a tier whose other limits the task breaks,
        // which would only send the draft back.
        let without_loc = TaskDef {
            max_loc: None,
            ..task.clone()
        };
        let Some(fits) = TaskTier::ALL.into_iter().find(|tier| {
            let limits = policy.tier_size_limits(*tier);
            limits.max_loc >= item.lines && limits.exceeded_by(&without_loc).is_empty()
        }) else {
            continue;
        };
        let tier = task.tier_class();
        if fits >= tier {
            continue;
        }
        let limit = policy.tier_size_limits(fits).max_loc;
        repairs.push(format!(
            "{}: tier {} became {}, the smallest whose {limit} lines cover the about {} lines \
             its source states",
            task.id,
            tier.label(),
            fits.label(),
            item.lines
        ));
        task.tier = fits.label().to_string();
        if task.max_loc.is_some_and(|max_loc| max_loc > limit) {
            task.max_loc = Some(limit);
        }
    }
    repairs
}

/// Raise each task of `plan` over its tier's size limits (files, `max_loc`,
/// description words, verify steps) to the smallest higher tier it fits, as
/// the size check suggests (gap-4dd77d). Round v7's phase 4.3 threw away the
/// regeneration that put back five of its spec's checks for "5 verify steps
/// (limit 4)" and wrote the draft that lacked them; phases 0 and 4.1 paid a
/// whole new draft each for the same rule. A task no tier fits stays for the
/// check, which asks for a split. Returns one line per task raised.
pub(super) fn raise_tiers(plan: &mut TasksFile, policy: PlanExecutionPolicy) -> Vec<String> {
    let mut raised = Vec::new();
    for task in &mut plan.tasks {
        let tier = task.tier_class();
        let over = policy.tier_size_limits(tier).exceeded_by(task);
        if over.is_empty() {
            continue;
        }
        let Some(fits) = TaskTier::ALL
            .into_iter()
            .filter(|candidate| *candidate > tier)
            .find(|candidate| {
                policy
                    .tier_size_limits(*candidate)
                    .exceeded_by(task)
                    .is_empty()
            })
        else {
            continue;
        };
        raised.push(format!(
            "{}: tier {} became {}, to fit its {}",
            task.id,
            tier.label(),
            fits.label(),
            over.join(", ")
        ));
        task.tier = fits.label().to_string();
    }
    raised
}

#[cfg(test)]
mod tests {
    use super::*;

    /// gap-9e212c: round v7's phase-2 spec (Undertow 3c84c86) sizes the
    /// rumble at about 200 lines, the output stage at about 160 and the engine
    /// task at about 80, and the LFO not at all. Its plan rumble-3 made all
    /// four tasks architectural. The engine task becomes integrative; the
    /// rumble and the output stage stay architectural, and so does the LFO,
    /// whose size the notes do not state.
    #[test]
    fn a_specs_stated_sizes_set_its_tasks_tiers() {
        let fixture =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p2_sizes");
        let spec = std::fs::read_to_string(fixture.join("request.md")).expect("the spec");
        let mut plan = TasksFile::parse(&fixture.join("rumble3.toml")).expect("rumble-3");
        let items = sized_items(&spec);
        let sizes: Vec<u32> = items.iter().map(|item| item.lines).collect();
        assert_eq!(sizes, [200, 160, 80], "{items:?}");
        assert_eq!(
            items[2].files,
            ["voice.h", "voice.cpp", "engine.cpp", "params.cpp"]
        );

        let repairs = repair_tiers(&mut plan, &[&spec], PlanExecutionPolicy::normal());
        assert_eq!(
            repairs,
            [
                "T4: tier architectural became integrative, the smallest whose 150 lines cover the \
              about 80 lines its source states"
            ]
        );
        let tiers: Vec<(&str, &str, Option<u32>)> = plan
            .tasks
            .iter()
            .map(|task| (task.id.as_str(), task.tier.as_str(), task.max_loc))
            .collect();
        assert_eq!(
            tiers,
            [
                ("T1", "architectural", Some(300)),
                ("T2", "architectural", Some(300)),
                ("T3", "architectural", Some(300)),
                ("T4", "integrative", Some(150)),
            ]
        );
        // A planner's smaller tier is not raised.
        let mut small = plan.clone();
        small.tasks[1].tier = "focused".to_string();
        small.tasks[1].max_loc = Some(50);
        assert!(repair_tiers(&mut small, &[&spec], PlanExecutionPolicy::normal()).is_empty());
        assert_eq!(small.tasks[1].tier, "focused");
    }

    /// Phase 4's notes name a header and its source in one span
    /// (`PluginProcessor.h/.cpp`); a span without an extension, or a test
    /// filter, is no file.
    #[test]
    fn a_short_form_names_two_files() {
        assert_eq!(
            code_span_files("(`engine.h`, `PluginProcessor.h/.cpp`, `P2.04*`, `cmake --preset`)"),
            ["engine.h", "PluginProcessor.h", "PluginProcessor.cpp"]
        );
        let items = sized_items(
            "- **The plumbing** (`engine.h`, `PluginProcessor.h/.cpp`): P4.01. About 70 lines.\n\
             - **Untitled**: no files here. About 30 lines.\n",
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].lines, 70);
    }

    /// gap-4dd77d: a task over its tier's size limits takes the smallest
    /// higher tier it fits (phase 4.3's focused T1 with 5 verify steps
    /// becomes integrative), and one no tier fits stays for the check. A
    /// stated size never lowers a task below a tier whose step limit it
    /// breaks.
    #[test]
    fn a_task_over_its_tier_takes_the_smallest_tier_it_fits() {
        let step = |command: &str| {
            format!("\n[[task.verify]]\nphase = \"test\"\ncommand = \"{command}\"\n")
        };
        let steps: String = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|name| step(&format!("tools/spec.sh {name}")))
            .collect();
        let plan_with = |tier: &str, files: usize| {
            let files: Vec<String> = (0..files).map(|n| format!("\"src/f{n}.cpp\"")).collect();
            TasksFile::parse_str(&format!(
                "[meta]\nplan = \"p\"\ntotal = 1\nstatus = \"ready\"\n\n\
                 [[task]]\nid = \"T1\"\ntitle = \"Fit the panels\"\nstatus = \"ready\"\n\
                 role = \"implementer\"\ntier = \"{tier}\"\nfiles = [{}]\n{steps}",
                files.join(", ")
            ))
            .expect("a plan")
        };
        let policy = PlanExecutionPolicy::generated(8);
        let mut plan = plan_with("focused", 1);
        assert_eq!(
            raise_tiers(&mut plan, policy),
            ["T1: tier focused became integrative, to fit its 5 verify steps (limit 4)"]
        );
        assert_eq!(plan.tasks[0].tier, "integrative");
        assert!(raise_tiers(&mut plan, policy).is_empty());

        // Nine files: no tier of a generated plan holds them.
        let mut wide = plan_with("focused", 9);
        assert!(raise_tiers(&mut wide, policy).is_empty());
        assert_eq!(wide.tasks[0].tier, "focused");

        // About 10 lines would make it mechanical, whose 3 steps it breaks:
        // focused holds 4, so integrative it stays.
        let mut stated = plan_with("architectural", 1);
        let source = "1. **The panels** (`src/f0.cpp`): about 10 lines.";
        assert_eq!(
            repair_tiers(&mut stated, &[source], policy),
            [
                "T1: tier architectural became integrative, the smallest whose 150 lines cover the \
              about 10 lines its source states"
            ]
        );
    }

    /// gap-5c7f9a: the task count a spec's notes for the planner state, read
    /// from that section only and where the count opens an item, a line or a
    /// sentence: phase 4.1's "One task, about 5 lines", phase 5.2's "One
    /// task:", phase 5's "Seven tasks." Phase 4.1's "phase 4's seven tasks"
    /// is outside the notes, and phase 5's "not any one task's" is inside a
    /// sentence. Two different counts, or none, are no count.
    #[test]
    fn the_notes_state_the_task_count() {
        let spec = |dir: &str| {
            std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures")
                    .join(dir)
                    .join("request.md"),
            )
            .expect("a spec")
        };
        assert_eq!(stated_task_count(&[&spec("p4_1_tasks")]), Some(1));
        assert_eq!(stated_task_count(&[&spec("p5_2_criteria")]), Some(1));
        assert_eq!(stated_task_count(&[&spec("p5_ship")]), Some(7));
        for (source, count) in [
            (
                "# P\n\n**Size**: 7 tasks.\n\n## Notes for the planner\n\n- Eight tasks. Each \
                 task's steps run.\n  1. **A** (`a.cpp`). Needs task 2.\n",
                Some(8),
            ),
            (
                "## Notes for the planner\n\n- Six tasks.\n- Two tasks own `a.cpp`.\n",
                None,
            ),
            ("## Checks\n\n- Three tasks.\n", None),
            (
                "## Notes for the planner\n\nThree tasks: a, b and c.\n\n## After\n\nFive tasks.\n",
                Some(3),
            ),
            (
                "## Notes for the planner\n\n- The check is the plan's, not any one task's.\n",
                None,
            ),
        ] {
            assert_eq!(stated_task_count(&[source]), count, "{source}");
        }
        assert_eq!(
            stated_task_count(&[
                "## Notes for the planner\n\nTwo tasks.\n",
                "## Notes for the planner\n\nThree tasks.\n"
            ]),
            None
        );
    }
}
