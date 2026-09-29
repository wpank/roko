//! Static spec-quality score for roko task specs: SQS v1, linter id `sq-1` (S07.7).
//!
//! [`lint_files`] scores every `[[task]]` of a set of `tasks.toml` files against rules SQ01–SQ12
//! and the hard fails HF1–HF5 of S07 §4.2 (`tmp/cybernetic-harness/specs/S07-spec-quality.md`),
//! one [`SpecQualityRecord`] per task. `roko plan validate --spec-quality` prints the records.
//!
//! The rules are a port of speclint (`benchmarks/viabilitybench/speclint/speclint.py`), whose
//! definitions are frozen as `sq-1`. The test `spec_quality_matches_speclint_golden_fixtures`
//! holds this port to speclint's golden fixtures, vendored under `tests/fixtures/speclint/`.
//! Change a rule only together with speclint and the linter id.
//!
//! Static mode runs nothing, so SQ06 (red on base) scores 0 and HF3 is not evaluated; every
//! static record lists both under `unknown`. A caller that ran the verify steps on the unchanged
//! base (speclint's `--dynamic`) passes each task's [`RedOnBase`] to [`lint_files_with`] or
//! [`score_task`].

mod shell;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use regex::Regex;
use serde::Serialize;
use toml::{Table, Value};

pub use shell::{Scope, StepAnalysis, VerifyClass, analyze_step, vacuous_reason};

/// The linter id. The rule definitions in this module are frozen under it.
pub const LINTER: &str = "sq-1";

/// One weighted rule of the score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rule {
    /// `SQ01` … `SQ12`.
    pub id: &'static str,
    /// What the rule checks.
    pub name: &'static str,
    /// The rule's weight; the weights sum to 100.
    pub weight: u32,
}

impl Rule {
    const fn new(id: &'static str, name: &'static str, weight: u32) -> Self {
        Self { id, name, weight }
    }
}

/// The twelve rules, in score order.
pub const RULES: [Rule; 12] = [
    Rule::new("SQ01", "goal", 10),
    Rule::new("SQ02", "acceptance", 15),
    Rule::new("SQ03", "traceability", 5),
    Rule::new("SQ04", "verify strength", 15),
    Rule::new("SQ05", "specificity", 5),
    Rule::new("SQ06", "red on base", 15),
    Rule::new("SQ07", "context", 8),
    Rule::new("SQ08", "scope", 4),
    Rule::new("SQ09", "non-goals", 5),
    Rule::new("SQ10", "vagueness", 8),
    Rule::new("SQ11", "anchors", 5),
    Rule::new("SQ12", "hidden hook", 5),
];

/// The hard fails: a task with any of them is blocked whatever its score.
pub const HARD_FAILS: [(&str, &str); 5] = [
    ("HF1", "implementer task without verify"),
    ("HF2", "vacuous verify step"),
    ("HF3", "verify passes on the unchanged base"),
    ("HF4", "missing context file"),
    ("HF5", "greenfield claim"),
];

/// What static mode cannot evaluate.
pub const STATIC_UNKNOWN: [&str; 2] = ["HF3", "SQ06"];

/// SQ08 and the `--strict` limit on `files` (`plan_policy.rs` `NORMAL_MAX_FILES_PER_TASK`).
pub const MAX_FILES: usize = 32;

/// SQ10: the vague-term lexicon v1 (S07 §4.2), matched case-insensitively outside code spans.
pub const VAGUE_TERMS: [&str; 31] = [
    "improve",
    "better",
    "robust",
    "clean up",
    "as needed",
    "appropriate",
    "appropriately",
    "proper",
    "properly",
    "various",
    "some",
    "etc.",
    "TBD",
    "TODO",
    "maybe",
    "possibly",
    "nice",
    "good",
    "reasonable",
    "efficient",
    "efficiently",
    "user-friendly",
    "seamless",
    "seamlessly",
    "flexible",
    "handle",
    "handles",
    "support for",
    "if possible",
    "and/or",
    "polish",
];

/// HF5: the phrases of PLAN_033 (`plan_validate.rs` `GREENFIELD_PHRASES`), searched in `prompt`,
/// `description` and `title` when the workspace has crates.
pub const GREENFIELD_PHRASES: [&str; 8] = [
    "no rust crates exist",
    "no existing crates",
    "starting from scratch",
    "greenfield project",
    "greenfield implementation",
    "new project from scratch",
    "no existing code",
    "empty workspace",
];

fn compile_regex(pattern: &str) -> Regex {
    Regex::new(pattern).expect("valid spec-quality regex")
}

/// SQ10: one anchored, case-insensitive pattern per lexicon term, longest term first. A space in
/// a term matches any run of whitespace.
static VAGUE_TERM_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    let mut terms = VAGUE_TERMS.to_vec();
    terms.sort_by_key(|term| std::cmp::Reverse(term.len()));
    terms
        .into_iter()
        .map(|term| {
            let pattern = regex::escape(term).replace(' ', r"\s+");
            compile_regex(&format!("(?i)^(?:{pattern})"))
        })
        .collect()
});
static CODE_FENCE: LazyLock<Regex> = LazyLock::new(|| compile_regex(r"(?s)```.*?```"));
static CODE_SPAN: LazyLock<Regex> = LazyLock::new(|| compile_regex(r"`[^`\n]*`"));
/// S07 §3.3 counts a task as having acceptance criteria when it has `acceptance`,
/// `acceptance_contract` or acceptance phrasing. The phrasing test is lexical, as in the
/// prototype: it also matches incidental mentions such as "run the acceptance check".
static ACCEPTANCE_PHRASE: LazyLock<Regex> = LazyLock::new(|| {
    compile_regex(concat!(
        r"(?i)\b(?:acceptance|criteri(?:a|on)|done when|definition of done",
        r"|expected (?:output|result|behaviou?r))\b",
    ))
});
/// SQ02 reads criteria only from an explicit section: a line that starts with the marker and
/// ends in a colon or the end of the line, plus the bullet lines under it.
static ACCEPTANCE_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    compile_regex(concat!(
        r"(?i)^\s*(?:#+\s*)?(?:[-*+]\s+)?(?:\*\*)?",
        r"(?:acceptance(?:\s+criteria)?|done\s+when|success\s+criteria|definition\s+of\s+done",
        r"|expected\s+(?:output|result|behaviou?r))",
        r"(?:\*\*)?\s*(?::|$)(?:\*\*)?\s*(.*)$",
    ))
});
static BULLET: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"^\s*(?:[-*+]|\d+[.)])\s+(.*\S)\s*$"));
static AC_ID: LazyLock<Regex> = LazyLock::new(|| compile_regex(r"^\s*(AC\d+)\b"));
/// SQ02: a criterion is observable when it names a comparator, a concrete value or an
/// observable effect ("returns/raises/exits/prints" and their kin).
static OBSERVABLE: LazyLock<Regex> = LazyLock::new(|| {
    compile_regex(concat!(
        r#"(?i)==|!=|>=|<=|≥|≤|`[^`]+`|"[^"]+"|\d"#,
        r"|\b(?:equals?|equal to|exactly|at least|at most|no more than|fewer than|more than",
        r"|less than|greater than|same as|identical|unchanged|matches|contains?|returns?",
        r"|returned|raises?|raised|exits?|exited|exit code|prints?|printed|outputs?|emits?",
        r"|emitted|writes?|written|fails?|failed|passes|passed|rejects?|rejected|errors?",
        r"|panics?|responds?|logs?|lists?|shows?|displays?|reports?|produces?|creates?",
        r"|deletes?|removes?|exists?)\b",
    ))
});
/// SQ09: an explicit scope exclusion in the prose.
static NON_GOAL: LazyLock<Regex> = LazyLock::new(|| {
    compile_regex(r"(?i)\b(?:do not|don't|must not|out of scope|non-goals?|not in scope)\b")
});
/// SQ11: a concrete file, type or function.
static ANCHOR: LazyLock<Regex> = LazyLock::new(|| {
    compile_regex(concat!(
        r"`[^`\n]+`",
        r"|\b[\w.-]+/[\w./-]*\.[A-Za-z0-9]{1,8}\b",
        r"|\b[\w-]+\.(?:rs|toml|md|json|jsonl|ts|tsx|js|mjs|cjs|py|sh|ya?ml|go|txt|csv|html|css",
        r"|lock)\b",
        r"|\b\w+::\w+",
        r"|\b[A-Za-z_]\w*\(\)",
        r"|\b[A-Z][a-z0-9]+(?:[A-Z][a-z0-9]*)+\b",
        r"|\b[a-z][a-z0-9]*_[a-z0-9_]+\b",
    ))
});

// --------------------------------------------------------------------------------------------
// The task view.

/// The fields of one `[[task]]` that the rules read: a neutral view that any task source can
/// fill. [`SpecTask::from_toml`] reads a raw `tasks.toml` table the way speclint does: a field of
/// the wrong type counts as absent, and list items are trimmed, with empty items dropped.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpecTask {
    /// `id`, as written.
    pub id: String,
    /// `role`, trimmed; `implementer` when absent or empty.
    pub role: String,
    /// `title`.
    pub title: String,
    /// `goal` (TSS v1).
    pub goal: String,
    /// `description`.
    pub description: String,
    /// `prompt`.
    pub prompt: String,
    /// `acceptance`: the task's acceptance criteria.
    pub acceptance: Vec<String>,
    /// Whether the task has a non-empty `acceptance_contract` table.
    pub has_contract: bool,
    /// The machine-checked entries of `acceptance_contract`: its gates, the `no_stub`,
    /// `agent_output` and `review_verdict` requirements that are set, and its parity rows.
    pub contract_criteria: usize,
    /// The `verify` steps.
    pub verify: Vec<SpecVerifyStep>,
    /// The files the task writes: `files` and `write_files`.
    pub outputs: BTreeSet<String>,
    /// `context.read_files`.
    pub read_files: Vec<SpecReadFile>,
    /// `context.symbols`.
    pub symbols: Vec<String>,
    /// `context.anti_patterns`.
    pub anti_patterns: Vec<String>,
    /// `non_goals` (TSS v1).
    pub non_goals: Vec<String>,
    /// `depends_on`: ids of tasks in the same plan.
    pub depends_on: Vec<String>,
    /// `depends_on_plan`: ids of prerequisite plans.
    pub depends_on_plan: Vec<String>,
    /// `max_loc`, when it is a positive integer.
    pub max_loc: Option<i64>,
    /// `hidden.interface` when `[task.hidden]` is a table (TSS v1); `None` without the table.
    pub hidden_interface: Option<Vec<String>>,
}

/// One `verify` step.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpecVerifyStep {
    /// The shell command.
    pub command: String,
    /// The acceptance criteria (`AC1` …) the step covers (TSS v1).
    pub covers: Vec<String>,
    /// `expect`; `pass_on_base` marks a regression step that already passes on the base (TSS v1).
    pub expect: String,
}

/// One `context.read_files` entry; a bare string is a path without a `why`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpecReadFile {
    /// The path, trimmed.
    pub path: String,
    /// Why the task reads the file.
    pub why: String,
}

impl SpecTask {
    /// Read one `[[task]]` table the way speclint does.
    pub fn from_toml(task: &Table) -> Self {
        let context = task.get("context").and_then(Value::as_table);
        let contract = task.get("acceptance_contract").and_then(Value::as_table);
        let role = text(task.get("role"));
        let role = role.trim();
        Self {
            id: text(task.get("id")),
            role: if role.is_empty() { "implementer" } else { role }.to_string(),
            title: text(task.get("title")),
            goal: text(task.get("goal")),
            description: text(task.get("description")),
            prompt: text(task.get("prompt")),
            acceptance: strings(task.get("acceptance")),
            has_contract: contract.is_some_and(|contract| !contract.is_empty()),
            contract_criteria: contract.map_or(0, contract_criteria),
            verify: tables(task.get("verify"))
                .map(SpecVerifyStep::from_toml)
                .collect(),
            outputs: strings(task.get("files"))
                .into_iter()
                .chain(strings(task.get("write_files")))
                .collect(),
            read_files: read_files(context),
            symbols: strings(context.and_then(|context| context.get("symbols"))),
            anti_patterns: strings(context.and_then(|context| context.get("anti_patterns"))),
            non_goals: strings(task.get("non_goals")),
            depends_on: strings(task.get("depends_on")),
            depends_on_plan: strings(task.get("depends_on_plan")),
            max_loc: task
                .get("max_loc")
                .and_then(Value::as_integer)
                .filter(|max_loc| *max_loc > 0),
            hidden_interface: task
                .get("hidden")
                .and_then(Value::as_table)
                .map(|hidden| strings(hidden.get("interface"))),
        }
    }
}

impl SpecVerifyStep {
    fn from_toml(step: &Table) -> Self {
        Self {
            command: text(step.get("command")),
            covers: strings(step.get("covers")),
            expect: text(step.get("expect")),
        }
    }
}

/// A string field; any other value reads as empty.
fn text(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// The non-empty trimmed strings of a list field (`plan_validate.rs` `string_array`).
fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

/// The tables of a list field.
fn tables(value: Option<&Value>) -> impl Iterator<Item = &Table> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_table)
}

/// Python truthiness, which speclint applies to the contract's requirement fields.
fn truthy(value: Option<&Value>) -> bool {
    match value {
        None => false,
        Some(Value::String(text)) => !text.is_empty(),
        Some(Value::Integer(number)) => *number != 0,
        Some(Value::Float(number)) => *number != 0.0,
        Some(Value::Boolean(flag)) => *flag,
        Some(Value::Datetime(_)) => true,
        Some(Value::Array(items)) => !items.is_empty(),
        Some(Value::Table(table)) => !table.is_empty(),
    }
}

fn contract_criteria(contract: &Table) -> usize {
    let gates = tables(contract.get("gates")).count();
    let requirements = ["no_stub", "agent_output", "review_verdict"]
        .into_iter()
        .filter(|key| truthy(contract.get(*key)))
        .count();
    let rows = tables(
        contract
            .get("parity_ledger")
            .and_then(Value::as_table)
            .and_then(|ledger| ledger.get("rows")),
    )
    .count();
    gates + requirements + rows
}

fn read_files(context: Option<&Table>) -> Vec<SpecReadFile> {
    context
        .and_then(|context| context.get("read_files"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| match item {
            Value::String(path) => Some(SpecReadFile {
                path: path.trim().to_string(),
                why: String::new(),
            }),
            Value::Table(entry) => Some(SpecReadFile {
                path: text(entry.get("path")).trim().to_string(),
                why: text(entry.get("why")),
            }),
            _ => None,
        })
        .collect()
}

// --------------------------------------------------------------------------------------------
// The plan and the workspace.

/// Whether a task's verify steps fail on the unchanged base (SQ06, HF3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RedOnBase {
    /// A step fails on the base in every run: SQ06 scores 1.
    Fail,
    /// Every step passes on the base: HF3 for an implementer that expects a step to turn green.
    Pass,
    /// Not checked (static mode), or the runs disagree.
    #[default]
    Unknown,
}

impl RedOnBase {
    /// `fail`, `pass` or `unknown`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fail => "fail",
            Self::Pass => "pass",
            Self::Unknown => "unknown",
        }
    }
}

/// The workspace a plan runs in: which files exist (HF4), whether it has crates (HF5) and the
/// text of context files (SQ07).
#[derive(Debug)]
pub struct Workspace {
    root: PathBuf,
    crate_count: usize,
    texts: Mutex<HashMap<String, Option<Arc<str>>>>,
}

impl Workspace {
    /// The workspace rooted at `root`.
    pub fn new(root: &Path) -> Self {
        let root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        let crate_count = std::fs::read_dir(root.join("crates")).map_or(0, |entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().is_dir())
                .count()
        });
        Self {
            root,
            crate_count,
            texts: Mutex::default(),
        }
    }

    /// The canonical root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The number of directories under `crates/`.
    pub fn crate_count(&self) -> usize {
        self.crate_count
    }

    /// Whether `rel` exists under the root.
    pub fn exists(&self, rel: &str) -> bool {
        self.root.join(rel).exists()
    }

    /// The UTF-8 text of the file at `rel`, when it is a file inside the root. Absolute paths and
    /// globs have none.
    pub fn text(&self, rel: &str) -> Option<Arc<str>> {
        let mut texts = self.texts.lock().unwrap_or_else(PoisonError::into_inner);
        texts
            .entry(rel.to_string())
            .or_insert_with(|| self.read(rel))
            .clone()
    }

    fn read(&self, rel: &str) -> Option<Arc<str>> {
        if rel.starts_with('/') || rel.contains(['\\', '*', '?', '[', ']']) {
            return None;
        }
        let resolved = std::fs::canonicalize(self.root.join(rel)).ok()?;
        if !resolved.is_file() || !resolved.starts_with(&self.root) {
            return None;
        }
        std::fs::read_to_string(resolved).ok().map(Arc::from)
    }
}

/// What scoring a task needs to know about its plan.
#[derive(Debug)]
pub struct PlanContext<'a> {
    /// The workspace the plan runs in.
    pub workspace: &'a Workspace,
    /// `meta.plan`, or the name of the plan's directory.
    pub plan_id: String,
    /// The `tasks.toml` path, relative to the workspace root when it is inside it.
    pub plan_path: String,
    /// Whether the plan sits under an `archive/` directory.
    pub archived: bool,
    /// The plan's tasks by trimmed id.
    pub tasks_by_id: HashMap<&'a str, &'a SpecTask>,
    /// The files each plan's tasks write, by plan id, for `depends_on_plan`.
    pub plan_outputs: &'a HashMap<String, BTreeSet<String>>,
}

/// Files written by the task's transitive dependencies and its prerequisite plans (PLAN_031).
fn dependency_outputs(task: &SpecTask, ctx: &PlanContext<'_>) -> BTreeSet<String> {
    let mut created = BTreeSet::new();
    let mut seen = HashSet::new();
    let mut pending: Vec<&str> = task.depends_on.iter().map(String::as_str).collect();
    while let Some(dep_id) = pending.pop() {
        if !seen.insert(dep_id) {
            continue;
        }
        if let Some(dep) = ctx.tasks_by_id.get(dep_id) {
            created.extend(dep.outputs.iter().cloned());
            pending.extend(dep.depends_on.iter().map(String::as_str));
        }
    }
    for plan_id in &task.depends_on_plan {
        if let Some(outputs) = ctx.plan_outputs.get(plan_id) {
            created.extend(outputs.iter().cloned());
        }
    }
    created
}

// --------------------------------------------------------------------------------------------
// Scoring.

/// The `spec.quality` record of one task (S07 §5), as speclint writes it without `spec_hash`,
/// `spec_origin`, `critic`, `ambiguity` and `ts`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpecQualityRecord {
    /// Always `spec.quality`.
    pub ev: &'static str,
    /// [`LINTER`].
    pub linter: &'static str,
    /// `static`, or `dynamic` when the red-on-base result is known.
    pub mode: &'static str,
    /// The plan id.
    pub plan_id: String,
    /// The `tasks.toml` path, relative to the workspace root when it is inside it.
    pub plan_path: String,
    /// Whether the plan sits under an `archive/` directory.
    pub archived: bool,
    /// The task id, as written.
    pub task_id: String,
    /// The task's role.
    pub role: String,
    /// The score, 0–100: the weighted sum of the rule scores, rounded to 2 places.
    pub score: f64,
    /// `A` (≥ 80), `B` (≥ 60), `C` (≥ 40) or `D`.
    pub band: &'static str,
    /// The hard fails, in HF order.
    pub hard_fail: Vec<&'static str>,
    /// What caused HF2 (the vacuous steps), HF4 (the missing files) and HF5 (the claims).
    pub hard_fail_detail: BTreeMap<&'static str, Vec<String>>,
    /// What this mode could not evaluate: `HF3` and `SQ06` in static mode.
    pub unknown: Vec<&'static str>,
    /// Each rule's score in [0, 1], rounded to 4 places.
    pub rules: BTreeMap<&'static str, f64>,
    /// The class of each verify step.
    pub verify_classes: Vec<VerifyClass>,
    /// The red-on-base result the score used.
    pub red_on_base: RedOnBase,
    /// The spec features for the M3 model (S07 §4.4).
    pub features: SpecFeatures,
}

/// The spec features of a task (S07 §4.4), with speclint's names.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpecFeatures {
    /// The strongest verify step's class, or `none` without steps.
    pub verify_max_class: &'static str,
    /// The number of verify steps.
    pub n_verify: usize,
    /// Steps classed compile because of `cargo test --no-run`.
    pub n_no_run: usize,
    /// Steps that run a program and check only its exit code.
    pub n_self_exit: usize,
    /// Whether a step runs a test runner.
    pub has_test_verify: bool,
    /// Whether the task has acceptance criteria: fields or acceptance phrasing (S07 §3.3).
    pub has_acceptance: bool,
    /// Whether the task has `acceptance` or `acceptance_contract`.
    pub has_acceptance_fields: bool,
    /// The number of acceptance criteria SQ02 found.
    pub n_acceptance: usize,
    /// How many of them are observable.
    pub n_observable: usize,
    /// SQ03: the share of `acceptance` items a verify step covers.
    pub ac_coverage: f64,
    /// The words of `goal` and `description` (SQ01).
    pub desc_words: usize,
    /// Vague terms per 100 prose words (SQ10).
    pub vague_density: f64,
    /// The vague terms found, lowercased.
    pub vague_terms: Vec<String>,
    /// The number of `read_files` entries.
    pub n_read_files: usize,
    /// How many of them say why.
    pub n_read_files_why: usize,
    /// The number of `symbols`.
    pub n_symbols: usize,
    /// How many of them are anchored in a context file.
    pub n_symbols_anchored: usize,
    /// The number of files the task writes.
    pub n_files: usize,
    /// `max_loc`, when set.
    pub max_loc: Option<i64>,
    /// Whether the task states non-goals (SQ09).
    pub has_non_goals: bool,
    /// Whether the task has a `[task.hidden]` table.
    pub has_hidden_hook: bool,
    /// Refinement rounds; always 0 for an authored spec.
    pub refine_rounds: u32,
}

/// The band of a score: `A` ≥ 80, `B` ≥ 60, `C` ≥ 40, else `D`.
pub fn band(score: f64) -> &'static str {
    if score >= 80.0 {
        "A"
    } else if score >= 60.0 {
        "B"
    } else if score >= 40.0 {
        "C"
    } else {
        "D"
    }
}

/// Score one task. `red_on_base` is [`RedOnBase::Unknown`] in static mode.
pub fn score_task(
    task: &SpecTask,
    ctx: &PlanContext<'_>,
    red_on_base: RedOnBase,
) -> SpecQualityRecord {
    // SQ01 goal: the words of goal and description; 1 if >= 40, 0.5 if 15-39, else 0.
    let desc_words = count_words(&task.goal) + count_words(&task.description);
    let sq01 = if desc_words >= 40 {
        1.0
    } else if desc_words >= 15 {
        0.5
    } else {
        0.0
    };

    // SQ02 acceptance: the share of criteria that are observable; 0 without criteria.
    let (criteria, has_acceptance) = acceptance_criteria(task);
    let n_observable = criteria.iter().filter(|observable| **observable).count();
    let sq02 = share(n_observable, criteria.len());

    // SQ03 traceability: the share of `acceptance` items (AC ids explicit or by position) that
    // some verify step `covers`.
    let covered: BTreeSet<&str> = task
        .verify
        .iter()
        .flat_map(|step| step.covers.iter().map(String::as_str))
        .collect();
    let n_covered = task
        .acceptance
        .iter()
        .enumerate()
        .filter(|(index, item)| covered.contains(ac_id(item, *index).as_str()))
        .count();
    let sq03 = share(n_covered, task.acceptance.len());

    // SQ04 verify strength: the value of the strongest step.
    let analyses: Vec<StepAnalysis> = task
        .verify
        .iter()
        .map(|step| analyze_step(&step.command, &task.outputs))
        .collect();
    let classes: Vec<VerifyClass> = analyses.iter().map(|analysis| analysis.class).collect();
    let max_class = classes.iter().min().copied();
    let sq04 = max_class.map_or(0.0, VerifyClass::value);

    // SQ05 specificity: test and compile runs scoped to a package, file or test name.
    let scopes: Vec<Scope> = analyses
        .iter()
        .flat_map(|analysis| analysis.scopes.iter().copied())
        .collect();
    let n_scoped = scopes
        .iter()
        .filter(|scope| **scope == Scope::Scoped)
        .count();
    let sq05 = if n_scoped == 0 {
        0.0
    } else if n_scoped == scopes.len() {
        1.0
    } else {
        0.5
    };

    // SQ06 red on base: dynamic; unknown scores 0 and is flagged.
    let sq06 = if red_on_base == RedOnBase::Fail {
        1.0
    } else {
        0.0
    };

    // SQ07 context: read_files, each with its own `why`, and every symbol anchored in them.
    let deps = dependency_outputs(task, ctx);
    let n_why = task
        .read_files
        .iter()
        .filter(|entry| {
            let why = entry.why.trim().to_lowercase();
            !why.is_empty() && why != "context"
        })
        .count();
    let contents: Vec<Arc<str>> = task
        .read_files
        .iter()
        .filter(|entry| !entry.path.is_empty())
        .filter_map(|entry| ctx.workspace.text(&entry.path))
        .collect();
    let deferred = task
        .read_files
        .iter()
        .any(|entry| deps.contains(&entry.path));
    let n_anchored = task
        .symbols
        .iter()
        .filter(|symbol| {
            explicit_symbol_anchor(symbol).is_some_and(|anchor| {
                deferred || contents.iter().any(|content| find_anchor(content, anchor))
            })
        })
        .count();
    let sq07 = if task.read_files.is_empty() {
        0.0
    } else if n_why == task.read_files.len() && n_anchored == task.symbols.len() {
        1.0
    } else {
        0.5
    };

    // SQ08 scope: `files` present and bounded; `max_loc` set.
    let bounded = (1..=MAX_FILES).contains(&task.outputs.len())
        && !task
            .outputs
            .iter()
            .any(|file| file.contains(['*', '?', '[', ']']));
    let sq08 = f64::from(u8::from(bounded) + u8::from(task.max_loc.is_some())) / 2.0;

    // SQ09 non-goals: `non_goals`, `anti_patterns` or an explicit exclusion in the prose.
    let has_non_goals = !task.non_goals.is_empty()
        || !task.anti_patterns.is_empty()
        || NON_GOAL.is_match(&format!("{}\n{}", task.goal, task.description));
    let sq09 = if has_non_goals { 1.0 } else { 0.0 };

    // SQ10 vagueness: lexicon hits per 100 prose words, d; max(0, 1 - d/5).
    let spec_text = [
        task.title.as_str(),
        task.goal.as_str(),
        task.description.as_str(),
    ]
    .into_iter()
    .chain(task.acceptance.iter().map(String::as_str))
    .collect::<Vec<_>>()
    .join("\n");
    let plain = prose(&spec_text);
    let vague = vague_terms(&plain);
    let n_words = count_words(&plain);
    let density = if n_words == 0 {
        0.0
    } else {
        100.0 * vague.len() as f64 / n_words as f64
    };
    let sq10 = (1.0 - density / 5.0).max(0.0);

    // SQ11 anchors: a concrete file, type or function named in the spec text.
    let sq11 = if ANCHOR.is_match(&spec_text) {
        1.0
    } else {
        0.0
    };

    // SQ12 hidden hook: `[task.hidden]` with an interface.
    let has_interface = task
        .hidden_interface
        .as_ref()
        .is_some_and(|interface| !interface.is_empty());
    let sq12 = if has_interface { 1.0 } else { 0.0 };

    let (hard_fail, hard_fail_detail) =
        hard_fails(task, ctx.workspace, &analyses, &deps, red_on_base);
    let values = [
        sq01, sq02, sq03, sq04, sq05, sq06, sq07, sq08, sq09, sq10, sq11, sq12,
    ];
    let score = round_to(
        RULES
            .iter()
            .zip(values)
            .map(|(rule, value)| f64::from(rule.weight) * value)
            .sum::<f64>(),
        2,
    );
    let is_static = red_on_base == RedOnBase::Unknown;
    SpecQualityRecord {
        ev: "spec.quality",
        linter: LINTER,
        mode: if is_static { "static" } else { "dynamic" },
        plan_id: ctx.plan_id.clone(),
        plan_path: ctx.plan_path.clone(),
        archived: ctx.archived,
        task_id: task.id.clone(),
        role: task.role.clone(),
        score,
        band: band(score),
        hard_fail,
        hard_fail_detail,
        unknown: if is_static {
            STATIC_UNKNOWN.to_vec()
        } else {
            Vec::new()
        },
        rules: RULES
            .iter()
            .zip(values)
            .map(|(rule, value)| (rule.id, round_to(value, 4)))
            .collect(),
        verify_classes: classes.clone(),
        red_on_base,
        features: SpecFeatures {
            verify_max_class: max_class.map_or("none", VerifyClass::as_str),
            n_verify: task.verify.len(),
            n_no_run: analyses.iter().filter(|analysis| analysis.no_run).count(),
            n_self_exit: analyses
                .iter()
                .filter(|analysis| analysis.self_exit)
                .count(),
            has_test_verify: classes.contains(&VerifyClass::Test),
            has_acceptance,
            has_acceptance_fields: !task.acceptance.is_empty() || task.has_contract,
            n_acceptance: criteria.len(),
            n_observable,
            ac_coverage: round_to(sq03, 4),
            desc_words,
            vague_density: round_to(density, 4),
            vague_terms: vague,
            n_read_files: task.read_files.len(),
            n_read_files_why: n_why,
            n_symbols: task.symbols.len(),
            n_symbols_anchored: n_anchored,
            n_files: task.outputs.len(),
            max_loc: task.max_loc,
            has_non_goals,
            has_hidden_hook: task.hidden_interface.is_some(),
            refine_rounds: 0,
        },
    }
}

/// The hard fails of a task, and what caused them.
fn hard_fails(
    task: &SpecTask,
    workspace: &Workspace,
    analyses: &[StepAnalysis],
    deps: &BTreeSet<String>,
    red_on_base: RedOnBase,
) -> (Vec<&'static str>, BTreeMap<&'static str, Vec<String>>) {
    let mut hard = Vec::new();
    let mut detail = BTreeMap::new();
    let implementer = task.role == "implementer";

    // HF1: an implementer task without verify steps.
    if implementer && task.verify.is_empty() {
        hard.push("HF1");
    }

    // HF2: a verify step that can never fail.
    let vacuous: Vec<String> = analyses
        .iter()
        .enumerate()
        .filter_map(|(index, analysis)| {
            let reason = analysis.vacuous.as_ref()?;
            Some(format!("step {}: {reason}", index + 1))
        })
        .collect();
    if !vacuous.is_empty() {
        hard.push("HF2");
        detail.insert("HF2", vacuous);
    }

    // HF3: every step passes on the base, though the task expects one to turn green.
    let all_pass_on_base = !task.verify.is_empty()
        && task
            .verify
            .iter()
            .all(|step| step.expect == "pass_on_base");
    if red_on_base == RedOnBase::Pass && implementer && !all_pass_on_base {
        hard.push("HF3");
    }

    // HF4: a context file that is missing and that no dependency writes first (PLAN_031).
    let mut missing: Vec<String> = Vec::new();
    for entry in &task.read_files {
        let path = &entry.path;
        if !path.is_empty()
            && !workspace.exists(path)
            && !deps.contains(path)
            && !missing.contains(path)
        {
            missing.push(path.clone());
        }
    }
    if !missing.is_empty() {
        hard.push("HF4");
        detail.insert("HF4", missing);
    }

    // HF5: a greenfield claim in a workspace that has crates (PLAN_033).
    if workspace.crate_count() > 0 {
        let mut claims = Vec::new();
        for text in [&task.prompt, &task.description, &task.title] {
            let lower = text.to_lowercase();
            for phrase in GREENFIELD_PHRASES {
                if lower.contains(phrase) {
                    claims.push(phrase.to_string());
                }
            }
        }
        if !claims.is_empty() {
            hard.push("HF5");
            detail.insert("HF5", claims);
        }
    }

    (hard, detail)
}

/// `part / whole`, or 0 when `whole` is 0.
fn share(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    }
}

/// `value` rounded to `places` decimals, ties to even, like Python's `round`.
fn round_to(value: f64, places: usize) -> f64 {
    format!("{value:.places$}").parse().unwrap_or(value)
}

/// An acceptance item's id: its `ACn` prefix, or `AC<position>`.
fn ac_id(item: &str, index: usize) -> String {
    AC_ID
        .captures(item)
        .and_then(|captures| captures.get(1))
        .map_or_else(|| format!("AC{}", index + 1), |id| id.as_str().to_string())
}

/// Whitespace-separated tokens that hold a letter or a digit.
fn count_words(text: &str) -> usize {
    text.split_whitespace()
        .filter(|token| token.chars().any(char::is_alphanumeric))
        .count()
}

/// Text without fenced code blocks and inline code spans.
fn prose(text: &str) -> String {
    let without_fences = CODE_FENCE.replace_all(text, " ");
    CODE_SPAN.replace_all(&without_fences, " ").into_owned()
}

/// The lexicon terms in `text`, lowercased, each standing alone: not preceded or followed by a
/// word character or `-`.
fn vague_terms(text: &str) -> Vec<String> {
    let is_joined = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    let mut found = Vec::new();
    let mut previous: Option<char> = None;
    let mut pos = 0;
    while let Some(current) = text[pos..].chars().next() {
        if !previous.is_some_and(is_joined) {
            let rest = &text[pos..];
            let matched = VAGUE_TERM_PATTERNS.iter().find_map(|pattern| {
                let end = pattern.find(rest)?.end();
                (!rest[end..].chars().next().is_some_and(is_joined)).then_some(end)
            });
            if let Some(end) = matched {
                found.push(rest[..end].to_lowercase());
                previous = rest[..end].chars().next_back();
                pos += end;
                continue;
            }
        }
        previous = Some(current);
        pos += current.len_utf8();
    }
    found
}

/// Python's `str.splitlines`: lines split at `\n`, `\r\n`, `\r` and the other Unicode line
/// boundaries, without a trailing empty line.
fn split_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        if matches!(
            c,
            '\n' | '\r'
                | '\u{0b}'
                | '\u{0c}'
                | '\u{1c}'
                | '\u{1d}'
                | '\u{1e}'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        ) {
            lines.push(&text[start..index]);
            let mut end = index + c.len_utf8();
            if c == '\r' && chars.peek().is_some_and(|&(_, next)| next == '\n') {
                chars.next();
                end += 1;
            }
            start = end;
        }
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// SQ02: whether each acceptance criterion is observable, and the S07 §3.3 has-acceptance flag.
fn acceptance_criteria(task: &SpecTask) -> (Vec<bool>, bool) {
    let mut criteria: Vec<bool> = task
        .acceptance
        .iter()
        .map(|item| OBSERVABLE.is_match(item))
        .collect();
    // Contract entries are machine-checked, so they are observable by construction.
    criteria.extend(std::iter::repeat_n(true, task.contract_criteria));
    let text = [task.goal.as_str(), task.description.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let lines = split_lines(&text);
    let mut i = 0;
    while i < lines.len() {
        let header = ACCEPTANCE_HEADER.captures(lines[i]);
        i += 1;
        let Some(header) = header else {
            continue;
        };
        let inline = header.get(1).map_or("", |group| group.as_str());
        if !inline.trim().is_empty() {
            criteria.push(OBSERVABLE.is_match(inline));
        }
        while let Some(bullet) = lines.get(i).and_then(|line| BULLET.captures(line)) {
            let item = bullet.get(1).map_or("", |group| group.as_str());
            criteria.push(OBSERVABLE.is_match(item));
            i += 1;
        }
    }
    let has_fields = !task.acceptance.is_empty() || task.has_contract;
    (criteria, has_fields || ACCEPTANCE_PHRASE.is_match(&text))
}

/// The identifier a `context.symbols` entry names (`plan_policy.rs` `explicit_symbol_anchor`):
/// `exact:name`, a backticked name, `name — note` or a bare `a::b`; `None` for free text.
fn explicit_symbol_anchor(symbol: &str) -> Option<&str> {
    let trimmed = symbol.trim();
    let candidate = if let Some(rest) = trimmed.strip_prefix("exact:") {
        rest.trim()
    } else if let Some(rest) = trimmed.strip_prefix('`') {
        rest.split_once('`')?.0.trim()
    } else if let Some((head, _)) = trimmed.split_once('—') {
        head.trim()
    } else if !trimmed.chars().any(char::is_whitespace) {
        trimmed
    } else {
        return None;
    };
    let candidate = candidate.strip_suffix("()").unwrap_or(candidate);
    let candidate = candidate.trim_matches(['`', ',', ';']);
    let valid = !candidate.is_empty()
        && candidate.split("::").all(|segment| {
            segment
                .chars()
                .next()
                .is_some_and(|first| !first.is_ascii_digit())
                && segment.chars().all(is_ident_char)
        });
    valid.then_some(candidate)
}

const fn is_ident_char(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric()
}

/// Whether a context file names the anchored symbol: its last `::` segment as a whole
/// identifier (`plan_policy.rs` `find_anchor_line`).
fn find_anchor(content: &str, anchor: &str) -> bool {
    let ident = anchor.rsplit("::").next().unwrap_or(anchor);
    content.match_indices(ident).any(|(start, _)| {
        let before = content[..start].chars().next_back();
        let after = content[start + ident.len()..].chars().next();
        !before.is_some_and(is_ident_char) && !after.is_some_and(is_ident_char)
    })
}

// --------------------------------------------------------------------------------------------
// Plans.

/// A `tasks.toml` that could not be read or parsed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SpecParseError {
    /// The file, relative to the workspace root when it is inside it.
    pub path: String,
    /// The read or parse error.
    pub error: String,
}

/// The records of every task in a set of `tasks.toml` files.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpecQualityReport {
    /// [`LINTER`].
    pub linter: &'static str,
    /// One record per task, in file order.
    pub tasks: Vec<SpecQualityRecord>,
    /// The files that could not be scored.
    pub parse_errors: Vec<SpecParseError>,
}

impl SpecQualityReport {
    /// How many tasks have a hard fail.
    pub fn hard_fail_tasks(&self) -> usize {
        self.tasks
            .iter()
            .filter(|record| !record.hard_fail.is_empty())
            .count()
    }

    /// 1 when `strict` and some task has a hard fail, else 0. A low score alone never fails.
    pub fn exit_code(&self, strict: bool) -> i32 {
        i32::from(strict && self.hard_fail_tasks() > 0)
    }
}

/// Score every task of `files` in static mode. Context files resolve against `root`, the
/// workspace the plans run in.
pub fn lint_files(files: &[PathBuf], root: &Path) -> SpecQualityReport {
    lint_files_with(files, root, &BTreeMap::new())
}

/// Score every task of `files`. `red_on_base` holds dynamic results by (plan path, task id);
/// every other task is scored in static mode.
pub fn lint_files_with(
    files: &[PathBuf],
    root: &Path,
    red_on_base: &BTreeMap<(String, String), RedOnBase>,
) -> SpecQualityReport {
    let workspace = Workspace::new(root);
    let mut plans: Vec<(String, String, Vec<SpecTask>)> = Vec::new();
    let mut parse_errors = Vec::new();
    for file in files {
        let path = std::fs::canonicalize(file).unwrap_or_else(|_| file.clone());
        let plan_path = relative_path(&path, workspace.root());
        match read_tasks_toml(&path) {
            Ok(data) => {
                let tasks = tables(data.get("task")).map(SpecTask::from_toml).collect();
                plans.push((plan_path, plan_id(&path, &data), tasks));
            }
            Err(error) => parse_errors.push(SpecParseError {
                path: plan_path,
                error,
            }),
        }
    }

    let mut plan_outputs: HashMap<String, BTreeSet<String>> = HashMap::new();
    for (_, id, tasks) in &plans {
        let outputs = plan_outputs.entry(id.clone()).or_default();
        for task in tasks {
            outputs.extend(task.outputs.iter().cloned());
        }
    }

    let mut records = Vec::new();
    for (plan_path, id, tasks) in &plans {
        let ctx = PlanContext {
            workspace: &workspace,
            plan_id: id.clone(),
            plan_path: plan_path.clone(),
            archived: plan_path.split('/').any(|part| part == "archive"),
            tasks_by_id: tasks
                .iter()
                .filter(|task| !task.id.trim().is_empty())
                .map(|task| (task.id.trim(), task))
                .collect(),
            plan_outputs: &plan_outputs,
        };
        for task in tasks {
            let key = (plan_path.clone(), task.id.clone());
            let state = red_on_base.get(&key).copied().unwrap_or_default();
            records.push(score_task(task, &ctx, state));
        }
    }

    SpecQualityReport {
        linter: LINTER,
        tasks: records,
        parse_errors,
    }
}

fn read_tasks_toml(path: &Path) -> Result<Table, String> {
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    toml::from_str(&text).map_err(|error| error.to_string())
}

/// `meta.plan`, or the name of the plan's directory.
fn plan_id(path: &Path, data: &Table) -> String {
    data.get("meta")
        .and_then(Value::as_table)
        .and_then(|meta| meta.get("plan"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|plan| !plan.is_empty())
        .map_or_else(
            || {
                path.parent()
                    .and_then(Path::file_name)
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default()
            },
            str::to_string,
        )
}

/// `path` relative to `root`, or the whole path when it is outside.
fn relative_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

// --------------------------------------------------------------------------------------------
// Text report.

/// The text `plan validate --spec-quality` prints: per plan, one line per task with its score,
/// band, rule scores and hard fails, the hard-fail details under it, and a summary line.
pub fn render_text(report: &SpecQualityReport) -> String {
    let mut out = String::new();
    let mode = if report.tasks.iter().any(|record| record.mode == "dynamic") {
        "dynamic".to_string()
    } else {
        format!("static: {} not evaluated", STATIC_UNKNOWN.join(" and "))
    };
    let _ = writeln!(out, "spec quality ({LINTER}, {mode})");
    for error in &report.parse_errors {
        let _ = writeln!(out, "  parse error: {}: {}", error.path, error.error);
    }
    let width = report
        .tasks
        .iter()
        .map(|record| record.task_id.chars().count())
        .max()
        .unwrap_or(0);
    let mut plan_path: Option<&str> = None;
    for record in &report.tasks {
        if plan_path != Some(record.plan_path.as_str()) {
            let _ = writeln!(out, "{}", record.plan_path);
            plan_path = Some(record.plan_path.as_str());
        }
        let rules = record
            .rules
            .iter()
            .map(|(rule, value)| format!("{rule}={}", rule_value(*value)))
            .collect::<Vec<_>>()
            .join(" ");
        let hard = if record.hard_fail.is_empty() {
            String::new()
        } else {
            format!("  hard={}", record.hard_fail.join(","))
        };
        let _ = writeln!(
            out,
            "  {:<width$} {:>6.2} {}  {rules}{hard}",
            record.task_id, record.score, record.band
        );
        for (hard_fail, details) in &record.hard_fail_detail {
            for detail in details {
                let _ = writeln!(out, "  {:<width$}   {hard_fail}: {detail}", "");
            }
        }
    }
    let bands = ["A", "B", "C", "D"].map(|band| {
        report
            .tasks
            .iter()
            .filter(|record| record.band == band)
            .count()
    });
    let _ = write!(
        out,
        "{} tasks: {} A, {} B, {} C, {} D; {} with hard fails",
        report.tasks.len(),
        bands[0],
        bands[1],
        bands[2],
        bands[3],
        report.hard_fail_tasks()
    );
    out
}

/// A rule score with at most two decimals and no trailing zeros: `1`, `0.5`, `0.25`.
fn rule_value(value: f64) -> String {
    let text = format!("{value:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_dirs() -> Vec<PathBuf> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/speclint");
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
            .expect("read the speclint fixtures")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|dir| dir.join("tasks.toml").is_file())
            .collect();
        dirs.sort();
        dirs
    }

    fn close(left: f64, right: f64, tolerance: f64) -> bool {
        (left - right).abs() <= tolerance
    }

    /// S07.9 parity: speclint's golden fixtures (vendored from
    /// `benchmarks/viabilitybench/speclint/fixtures/`) score the same here, within 0.5 points,
    /// with identical hard fails, bands, rule scores and verify classes.
    #[test]
    fn spec_quality_matches_speclint_golden_fixtures() {
        let dirs = fixture_dirs();
        assert_eq!(dirs.len(), 16, "one fixture per rule and static hard fail");
        let mut focused = Vec::new();
        for dir in &dirs {
            let name = dir.display().to_string();
            let expected = std::fs::read_to_string(dir.join("expected.json"))
                .expect("read expected.json");
            let expected: serde_json::Value =
                serde_json::from_str(&expected).expect("parse expected.json");
            let report = lint_files(&[dir.join("tasks.toml")], dir);
            assert!(report.parse_errors.is_empty(), "{name}: {:?}", report.parse_errors);
            let got: HashMap<&str, &SpecQualityRecord> = report
                .tasks
                .iter()
                .map(|record| (record.task_id.as_str(), record))
                .collect();
            let want_tasks = expected["tasks"].as_object().expect("expected tasks");
            let mut got_ids: Vec<&str> = got.keys().copied().collect();
            got_ids.sort_unstable();
            let mut want_ids: Vec<&str> = want_tasks.keys().map(String::as_str).collect();
            want_ids.sort_unstable();
            assert_eq!(got_ids, want_ids, "{name}: task ids");

            for (task_id, want) in want_tasks {
                let record = got[task_id.as_str()];
                let label = format!("{name} {task_id}");
                let score = want["score"].as_f64().expect("expected score");
                assert!(
                    close(record.score, score, 0.5),
                    "{label}: score {} vs {score}",
                    record.score
                );
                assert_eq!(record.band, want["band"], "{label}: band");
                let hard: Vec<&str> = want["hard_fail"]
                    .as_array()
                    .expect("expected hard_fail")
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .collect();
                assert_eq!(record.hard_fail, hard, "{label}: hard fails");
                let classes: Vec<&str> = record
                    .verify_classes
                    .iter()
                    .map(|class| class.as_str())
                    .collect();
                let want_classes: Vec<&str> = want["verify_classes"]
                    .as_array()
                    .expect("expected verify_classes")
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .collect();
                assert_eq!(classes, want_classes, "{label}: verify classes");
                let rules = want["rules"].as_object().expect("expected rules");
                assert_eq!(record.rules.len(), rules.len(), "{label}: rule count");
                for (rule, value) in rules {
                    let value = value.as_f64().expect("expected rule score");
                    let got_value = record.rules[rule.as_str()];
                    assert!(
                        close(got_value, value, 1e-3),
                        "{label}: {rule} {got_value} vs {value}"
                    );
                }
                assert_eq!(record.unknown, STATIC_UNKNOWN, "{label}: unknown");
                assert_eq!(record.red_on_base, RedOnBase::Unknown, "{label}");
                assert_eq!(record.mode, "static", "{label}");
            }

            let rule = expected["focus"]["rule"].as_str().expect("focus rule");
            focused.push(rule.to_string());
            let focus = expected["focus"]["expect"].as_object().expect("focus");
            for (task_id, want) in focus {
                let record = got[task_id.as_str()];
                if rule.starts_with("SQ") {
                    let want = want.as_f64().expect("focus score");
                    assert!(
                        close(record.rules[rule], want, 1e-3),
                        "{name} {task_id}: focus {rule}"
                    );
                } else {
                    let want = want.as_bool().expect("focus flag");
                    assert_eq!(
                        record.hard_fail.contains(&rule),
                        want,
                        "{name} {task_id}: focus {rule}"
                    );
                }
            }
        }

        focused.sort();
        let mut all: Vec<String> = RULES.iter().map(|rule| rule.id.to_string()).collect();
        all.extend(["HF1", "HF2", "HF4", "HF5"].map(String::from));
        all.sort();
        assert_eq!(focused, all, "the fixtures cover every rule and static hard fail");
    }

    #[test]
    fn red_on_base_scores_sq06_and_hf3_when_supplied() {
        let dir = fixture_dirs()
            .into_iter()
            .find(|dir| dir.ends_with("sq06-red-on-base"))
            .expect("the sq06 fixture");
        let path = dir.join("tasks.toml");
        let plan_path = "tasks.toml".to_string();
        let red_on_base = BTreeMap::from([
            ((plan_path.clone(), "T1".to_string()), RedOnBase::Fail),
            ((plan_path, "T2".to_string()), RedOnBase::Pass),
        ]);
        let report = lint_files_with(&[path], &dir, &red_on_base);
        let t1 = &report.tasks[0];
        assert_eq!(t1.rules["SQ06"], 1.0);
        assert!(t1.unknown.is_empty());
        assert_eq!(t1.mode, "dynamic");
        let t2 = &report.tasks[1];
        assert_eq!(t2.hard_fail, ["HF3"]);
    }

    #[test]
    fn weights_sum_to_one_hundred() {
        assert_eq!(RULES.iter().map(|rule| rule.weight).sum::<u32>(), 100);
    }

    #[test]
    fn vague_terms_stand_alone_and_skip_code() {
        assert_eq!(
            vague_terms("Improve the retry logic and handle some edge cases as needed."),
            ["improve", "handle", "some", "as needed"]
        );
        assert!(vague_terms("the properly-named handles_x").is_empty());
        assert_eq!(vague_terms("clean\n  up etc. now"), ["clean\n  up", "etc."]);
        assert!(vague_terms(&prose("Set the `better` flag.")).is_empty());
    }

    #[test]
    fn symbol_anchors_follow_plan_policy() {
        assert_eq!(explicit_symbol_anchor("parse_config"), Some("parse_config"));
        assert_eq!(explicit_symbol_anchor("`Config::load()` loads it"), Some("Config::load"));
        assert_eq!(explicit_symbol_anchor("exact: run_gate_once"), Some("run_gate_once"));
        assert_eq!(explicit_symbol_anchor("the Config struct"), None);
        assert!(find_anchor("pub fn load() {}", "Config::load"));
        assert!(!find_anchor("pub fn loader() {}", "load"));
    }

    #[test]
    fn rounding_and_lines_match_python() {
        // 2.675 is stored just below the tie; 0.125 and 0.375 are exact ties, which go to even.
        assert_eq!(round_to(2.675, 2), 2.67);
        assert_eq!(round_to(0.125, 2), 0.12);
        assert_eq!(round_to(0.375, 2), 0.38);
        assert_eq!(round_to(2.0 / 3.0, 4), 0.6667);
        assert_eq!(split_lines("a\r\nb\rc\n"), ["a", "b", "c"]);
        assert_eq!(rule_value(0.5), "0.5");
        assert_eq!(rule_value(1.0), "1");
        assert_eq!(rule_value(0.0), "0");
    }

    #[test]
    fn render_text_lists_scores_rules_and_hard_fails() {
        let dir = fixture_dirs()
            .into_iter()
            .find(|dir| dir.ends_with("hf2-vacuous-verify"))
            .expect("the hf2 fixture");
        let report = lint_files(&[dir.join("tasks.toml")], &dir);
        let text = render_text(&report);
        assert!(text.starts_with("spec quality (sq-1, static: HF3 and SQ06 not evaluated)\n"));
        assert!(text.contains("\ntasks.toml\n"), "{text}");
        assert!(
            text.contains("T3  17.00 D  SQ01=0 SQ02=0 SQ03=0 SQ04=0 SQ05=0 SQ06=0 SQ07=0 SQ08=1"),
            "{text}"
        );
        assert!(text.contains("hard=HF2"), "{text}");
        assert!(text.contains("HF2: step 1: the step only runs `echo done`"), "{text}");
        assert!(text.ends_with("6 tasks: 0 A, 0 B, 0 C, 6 D; 4 with hard fails"), "{text}");
        assert_eq!(report.exit_code(false), 0);
        assert_eq!(report.exit_code(true), 1);
    }
}
