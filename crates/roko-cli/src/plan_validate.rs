use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use indexmap::IndexMap;
use roko_core::AgentRole;
use roko_core::config::schema::ModelProfile;
use roko_gate::AcceptanceContract;
use roko_orchestrator::detect_cycle_nodes;
use serde::Serialize;
use toml::Value;

use roko_cli::task_parser::normalize_model_alias;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warn",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub rule_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanDiagnostics {
    pub plan_id: String,
    pub path: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct Totals {
    pub errors: usize,
    pub warnings: usize,
    pub plans_checked: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationReport {
    pub plans: Vec<PlanDiagnostics>,
    pub totals: Totals,
}

impl ValidationReport {
    #[must_use]
    pub fn exit_code(&self, strict: bool) -> i32 {
        if self.totals.errors > 0 || (strict && self.totals.warnings > 0) {
            1
        } else {
            0
        }
    }
}

#[derive(Debug, Clone)]
struct TaskSnapshot {
    ordinal: usize,
    task_id: Option<String>,
    title: Option<String>,
    role: Option<String>,
    model: Option<String>,
    depends_on: Vec<String>,
    has_depends_on_field: bool,
    gate_rung: Option<u32>,
    gate_rung_invalid: bool,
    has_files: bool,
    has_context_read_files: bool,
    has_verify_steps: bool,
    acceptance_contract: Option<Value>,
    has_required_parity_ledger_rows: bool,
    deferral_missing_fields: Vec<&'static str>,
}

impl TaskSnapshot {
    fn label(&self) -> String {
        self.task_id
            .clone()
            .unwrap_or_else(|| format!("task #{}", self.ordinal))
    }
}

pub fn validate_plans_dir(
    dir: &Path,
    models: Option<&IndexMap<String, ModelProfile>>,
) -> Result<ValidationReport> {
    validate_plans_dir_impl(dir, models, None)
}

/// Validate plans in `dir` with optional file-reference checking against `workdir`.
///
/// When `workdir` is provided, each `tasks.toml` file is scanned for declared
/// `files` and `write_files` entries and those paths are checked against the
/// workspace filesystem.
pub fn validate_plans_dir_with_workdir(
    dir: &Path,
    models: Option<&IndexMap<String, ModelProfile>>,
    workdir: Option<&Path>,
) -> Result<ValidationReport> {
    validate_plans_dir_impl(dir, models, workdir)
}

fn validate_plans_dir_impl(
    dir: &Path,
    models: Option<&IndexMap<String, ModelProfile>>,
    workdir: Option<&Path>,
) -> Result<ValidationReport> {
    let tasks_files = collect_tasks_files(dir)?;
    let plan_output_paths = collect_plan_output_paths(&tasks_files);
    let mut plans = Vec::with_capacity(tasks_files.len());
    let mut totals = Totals {
        plans_checked: tasks_files.len(),
        ..Totals::default()
    };

    for tasks_path in tasks_files {
        let mut plan = validate_tasks_file(&tasks_path, models)
            .with_context(|| format!("validate {}", tasks_path.display()))?;
        if let Some(workdir) = workdir {
            let ref_diagnostics = if plan_output_paths.len() <= 1 {
                validate_file_references(&tasks_path, workdir)
            } else {
                validate_file_references_with_plan_outputs(&tasks_path, workdir, &plan_output_paths)
            };
            if let Ok(ref_diagnostics) = ref_diagnostics {
                plan.diagnostics.extend(ref_diagnostics);
            }

            let existing_crates = collect_workspace_package_names(workdir, "crates");
            if !existing_crates.is_empty()
                && let Ok(content) = std::fs::read_to_string(&tasks_path)
                && let Ok(parsed) = toml::from_str::<Value>(&content)
            {
                let gf_plan_id = parsed
                    .get("meta")
                    .and_then(Value::as_table)
                    .and_then(|meta| meta.get("plan"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("unknown-plan");
                let gf_diagnostics =
                    validate_no_greenfield_duplicates(&parsed, gf_plan_id, &existing_crates);
                plan.diagnostics.extend(gf_diagnostics);
            }

            plan.diagnostics.sort_by(|left, right| {
                left.severity
                    .cmp(&right.severity)
                    .then_with(|| left.rule_id.cmp(&right.rule_id))
                    .then_with(|| left.task_id.cmp(&right.task_id))
                    .then_with(|| left.message.cmp(&right.message))
            });
        }
        for diagnostic in &plan.diagnostics {
            match diagnostic.severity {
                Severity::Error => totals.errors += 1,
                Severity::Warning => totals.warnings += 1,
            }
        }
        if !plan.diagnostics.is_empty() {
            plans.push(plan);
        }
    }

    Ok(ValidationReport { plans, totals })
}

pub fn render_text(report: &ValidationReport) -> String {
    let mut out = String::new();
    let mut printed_plan = false;

    for plan in report
        .plans
        .iter()
        .filter(|plan| !plan.diagnostics.is_empty())
    {
        if printed_plan {
            out.push('\n');
        }
        printed_plan = true;

        let _ = writeln!(out, "{}", plan.path);
        for diagnostic in &plan.diagnostics {
            let _ = writeln!(
                out,
                "  {:<5} {:<8} {}",
                diagnostic.severity.label(),
                diagnostic.rule_id,
                diagnostic.message
            );
        }
    }

    let diagnostic_count = report.totals.errors + report.totals.warnings;
    let plan_word = if report.totals.plans_checked == 1 {
        "plan"
    } else {
        "plans"
    };
    let _ = write!(
        out,
        "{diagnostic_count} diagnostics in {} {plan_word}",
        report.totals.plans_checked
    );
    out
}

pub fn render_json(report: &ValidationReport) -> Result<String> {
    serde_json::to_string_pretty(report).context("serialize plan validation report")
}

fn collect_tasks_files(dir: &Path) -> Result<Vec<PathBuf>> {
    if dir.is_file() {
        if dir.file_name().is_some_and(|name| name == "tasks.toml") {
            return Ok(vec![dir.to_path_buf()]);
        }
        bail!(
            "{} is not a plans directory or tasks.toml file",
            dir.display()
        );
    }

    if !dir.exists() {
        bail!("{} does not exist", dir.display());
    }
    if !dir.is_dir() {
        bail!("{} is not a directory", dir.display());
    }

    let mut out = Vec::new();
    collect_tasks_files_recursive(dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect_tasks_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries = std::fs::read_dir(dir)
        .with_context(|| format!("read directory {}", dir.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("read directory entries for {}", dir.display()))?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_tasks_files_recursive(&path, out)?;
        } else if path.is_file() && path.file_name().is_some_and(|name| name == "tasks.toml") {
            out.push(path);
        }
    }

    Ok(())
}

fn validate_tasks_file(
    tasks_path: &Path,
    models: Option<&IndexMap<String, ModelProfile>>,
) -> Result<PlanDiagnostics> {
    let content = std::fs::read_to_string(tasks_path)
        .with_context(|| format!("read {}", tasks_path.display()))?;
    let path = tasks_path.display().to_string();
    let fallback_plan_id = tasks_path
        .parent()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown-plan".to_string());

    let parsed: Value = match toml::from_str(&content) {
        Ok(value) => value,
        Err(error) => {
            return Ok(PlanDiagnostics {
                plan_id: fallback_plan_id.clone(),
                path,
                diagnostics: vec![Diagnostic {
                    severity: Severity::Error,
                    rule_id: "PLAN_001".to_string(),
                    plan_id: Some(fallback_plan_id),
                    task_id: None,
                    message: format!("failed to parse TOML: {error}"),
                }],
            });
        }
    };

    let plan_id = parsed
        .get("meta")
        .and_then(Value::as_table)
        .and_then(|meta| meta.get("plan"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or(fallback_plan_id);
    let is_architecture_queue = parsed
        .get("meta")
        .and_then(Value::as_table)
        .is_some_and(is_architecture_queue_meta);

    let mut diagnostics = Vec::new();

    // Try parsing with the runtime parser — if this fails, `plan run` would
    // also fail on the same file.  Report any deserialization error so that
    // `plan validate` and `plan run` agree on whether a file is acceptable.
    match roko_cli::task_parser::TasksFile::parse_str(&content) {
        Ok(tasks_file) => {
            for schema_issue in tasks_file.validate_against_schema() {
                diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    rule_id: "PLAN_035".to_string(),
                    plan_id: Some(plan_id.clone()),
                    task_id: None,
                    message: format!("schema validation failed: {schema_issue}"),
                });
            }
        }
        Err(runtime_err) => {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                rule_id: "PLAN_034".to_string(),
                plan_id: Some(plan_id.clone()),
                task_id: None,
                message: format!("plan would fail at runtime: {runtime_err}"),
            });
        }
    }
    let tasks = parsed
        .get("task")
        .and_then(Value::as_array)
        .map(|array| {
            array
                .iter()
                .enumerate()
                .map(|(index, task)| snapshot_task(index + 1, task))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if tasks.is_empty() {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            rule_id: "PLAN_002".to_string(),
            plan_id: Some(plan_id.clone()),
            task_id: None,
            message: "task array is missing or empty".to_string(),
        });
    }

    for task in &tasks {
        emit_required_field(
            &mut diagnostics,
            &plan_id,
            task,
            "id",
            task.task_id.as_deref(),
        );
        emit_required_field(
            &mut diagnostics,
            &plan_id,
            task,
            "title",
            task.title.as_deref(),
        );
        emit_required_field(
            &mut diagnostics,
            &plan_id,
            task,
            "role",
            task.role.as_deref(),
        );

        if task.gate_rung_invalid || task.gate_rung.is_some_and(|gate_rung| gate_rung > 6) {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                rule_id: "PLAN_007".to_string(),
                plan_id: Some(plan_id.clone()),
                task_id: task.task_id.clone(),
                message: format!(
                    "task '{}' uses invalid gate_rung; expected an integer in 0..=6",
                    task.label()
                ),
            });
        }

        if task.gate_rung == Some(0) && !task.has_verify_steps {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                rule_id: "PLAN_011".to_string(),
                plan_id: Some(plan_id.clone()),
                task_id: task.task_id.clone(),
                message: format!(
                    "task '{}' sets gate_rung = 0 but has no verify steps",
                    task.label()
                ),
            });
        }

        if let Some(contract_value) = &task.acceptance_contract {
            match contract_value.clone().try_into::<AcceptanceContract>() {
                Ok(contract) => {
                    let decision = contract.validate_contract();
                    for issue in decision.issues {
                        diagnostics.push(Diagnostic {
                            severity: if issue.blocking {
                                Severity::Error
                            } else {
                                Severity::Warning
                            },
                            rule_id: issue.code,
                            plan_id: Some(plan_id.clone()),
                            task_id: task.task_id.clone(),
                            message: format!(
                                "task '{}' has invalid acceptance_contract: {}",
                                task.label(),
                                issue.message
                            ),
                        });
                    }
                }
                Err(error) => diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    rule_id: "PLAN_012".to_string(),
                    plan_id: Some(plan_id.clone()),
                    task_id: task.task_id.clone(),
                    message: format!(
                        "task '{}' has malformed acceptance_contract: {error}",
                        task.label()
                    ),
                }),
            }
        }

        if is_architecture_queue {
            validate_architecture_queue_task(&mut diagnostics, &plan_id, task);
        }
    }

    let mut seen_ids = HashSet::new();
    let mut duplicate_ids = BTreeSet::new();
    let known_ids = tasks
        .iter()
        .filter_map(|task| normalized_field(task.task_id.as_deref()))
        .map(ToOwned::to_owned)
        .collect::<HashSet<_>>();

    for task in &tasks {
        if let Some(task_id) = normalized_field(task.task_id.as_deref()) {
            if !seen_ids.insert(task_id.to_string()) {
                duplicate_ids.insert(task_id.to_string());
            }
        }
    }

    for task_id in duplicate_ids {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            rule_id: "PLAN_004".to_string(),
            plan_id: Some(plan_id.clone()),
            task_id: Some(task_id.clone()),
            message: format!("task id '{task_id}' is duplicated within this plan"),
        });
    }

    for task in &tasks {
        for dependency in &task.depends_on {
            if !known_ids.contains(dependency.as_str()) {
                diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    rule_id: "PLAN_005".to_string(),
                    plan_id: Some(plan_id.clone()),
                    task_id: task.task_id.clone(),
                    message: format!(
                        "task '{}' depends on unknown task '{}'",
                        task.label(),
                        dependency
                    ),
                });
            }
        }
    }

    let mut valid_deps = BTreeMap::<String, BTreeSet<String>>::new();
    for task in &tasks {
        if let Some(task_id) = normalized_field(task.task_id.as_deref()) {
            valid_deps.insert(
                task_id.to_string(),
                task.depends_on
                    .iter()
                    .filter(|dependency| known_ids.contains(dependency.as_str()))
                    .cloned()
                    .collect(),
            );
        }
    }

    let cycle_nodes = detect_cycle_nodes(&valid_deps);
    for cycle_node in &cycle_nodes {
        let peers = cycle_nodes
            .iter()
            .filter(|candidate| *candidate != cycle_node)
            .cloned()
            .collect::<Vec<_>>();
        let message = if peers.is_empty() {
            format!("task '{cycle_node}' participates in a cycle")
        } else {
            format!(
                "task '{cycle_node}' forms a cycle with {}",
                peers
                    .iter()
                    .map(|peer| format!("'{peer}'"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            rule_id: "PLAN_006".to_string(),
            plan_id: Some(plan_id.clone()),
            task_id: Some(cycle_node.clone()),
            message,
        });
    }

    let reachable = reachable_from_roots(&tasks, &known_ids);
    for task in &tasks {
        if let Some(task_id) = normalized_field(task.task_id.as_deref())
            && !reachable.contains(task_id)
        {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                rule_id: "PLAN_010".to_string(),
                plan_id: Some(plan_id.clone()),
                task_id: Some(task_id.to_string()),
                message: format!("task '{task_id}' is reachable from no root"),
            });
        }
    }

    for task in &tasks {
        if let Some(role) = normalized_field(task.role.as_deref()) {
            let has_template = parse_task_role(role).is_some_and(role_has_compose_template);
            if !has_template {
                diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    rule_id: "PLAN_008".to_string(),
                    plan_id: Some(plan_id.clone()),
                    task_id: task.task_id.clone(),
                    message: format!(
                        "task '{}' uses role '{}' which has no template",
                        task.label(),
                        role
                    ),
                });
            }
        }

        if let Some(model) = normalized_field(task.model.as_deref()) {
            let normalized = normalize_model_alias(model);
            if normalized != model {
                diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    rule_id: "PLAN_012".to_string(),
                    plan_id: Some(plan_id.clone()),
                    task_id: task.task_id.clone(),
                    message: format!(
                        "task '{}' uses model alias '{}'; prefer the full name '{}'",
                        task.label(),
                        model,
                        normalized
                    ),
                });
            }
            if let Some(known_models) = models {
                if !model_is_known(normalized, known_models) {
                    diagnostics.push(Diagnostic {
                        severity: Severity::Warning,
                        rule_id: "PLAN_009".to_string(),
                        plan_id: Some(plan_id.clone()),
                        task_id: task.task_id.clone(),
                        message: format!(
                            "task '{}' uses model '{}' which is not configured in roko.toml",
                            task.label(),
                            model
                        ),
                    });
                }
            }
        }
    }

    diagnostics.sort_by(|left, right| {
        left.severity
            .cmp(&right.severity)
            .then_with(|| left.rule_id.cmp(&right.rule_id))
            .then_with(|| left.task_id.cmp(&right.task_id))
            .then_with(|| left.message.cmp(&right.message))
    });

    Ok(PlanDiagnostics {
        plan_id,
        path,
        diagnostics,
    })
}

fn snapshot_task(ordinal: usize, task: &Value) -> TaskSnapshot {
    let table = task.as_table();
    let gate_rung_value = table.and_then(|table| table.get("gate_rung"));

    TaskSnapshot {
        ordinal,
        task_id: table.and_then(|table| string_field(table.get("id"))),
        title: table.and_then(|table| string_field(table.get("title"))),
        role: table.and_then(|table| string_field(table.get("role"))),
        model: table.and_then(|table| {
            string_field(table.get("model")).or_else(|| string_field(table.get("model_hint")))
        }),
        has_depends_on_field: table.is_some_and(|table| table.contains_key("depends_on")),
        depends_on: table
            .and_then(|table| table.get("depends_on"))
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| string_field(Some(item)))
                    .collect()
            })
            .unwrap_or_default(),
        gate_rung: gate_rung_value
            .and_then(Value::as_integer)
            .and_then(|value| u32::try_from(value).ok()),
        gate_rung_invalid: gate_rung_value.is_some()
            && gate_rung_value
                .and_then(Value::as_integer)
                .and_then(|value| u32::try_from(value).ok())
                .is_none(),
        has_files: table
            .and_then(|table| table.get("files"))
            .or_else(|| table.and_then(|table| table.get("write_files")))
            .and_then(Value::as_array)
            .is_some_and(|files| files.iter().any(|file| string_field(Some(file)).is_some())),
        has_context_read_files: table
            .and_then(|table| table.get("context"))
            .and_then(Value::as_table)
            .and_then(|context| context.get("read_files"))
            .and_then(Value::as_array)
            .is_some_and(|files| {
                files.iter().any(|file| {
                    file.as_table()
                        .and_then(|table| table.get("path"))
                        .and_then(Value::as_str)
                        .is_some_and(|path| !path.trim().is_empty())
                })
            }),
        has_verify_steps: table
            .and_then(|table| table.get("verify"))
            .and_then(Value::as_array)
            .is_some_and(|steps| !steps.is_empty()),
        acceptance_contract: table
            .and_then(|table| table.get("acceptance_contract"))
            .cloned(),
        has_required_parity_ledger_rows: table.is_some_and(has_required_parity_ledger_rows),
        deferral_missing_fields: table
            .and_then(|table| table.get("deferral"))
            .and_then(Value::as_table)
            .map(deferral_missing_fields)
            .unwrap_or_default(),
    }
}

fn is_architecture_queue_meta(meta: &toml::map::Map<String, Value>) -> bool {
    ["queue_kind", "queue_schema", "kind"].iter().any(|field| {
        meta.get(*field)
            .and_then(Value::as_str)
            .is_some_and(|value| value.trim() == "architecture_implementation")
    })
}

fn validate_architecture_queue_task(
    diagnostics: &mut Vec<Diagnostic>,
    plan_id: &str,
    task: &TaskSnapshot,
) {
    let requirements = [
        (
            !task.has_depends_on_field,
            "PLAN_020",
            "declares no depends_on array for dependency metadata",
        ),
        (
            !task.has_context_read_files,
            "PLAN_021",
            "declares no context.read_files source docs",
        ),
        (
            !task.has_files,
            "PLAN_022",
            "declares no files list for likely crates/artifacts",
        ),
        (
            !task.has_verify_steps,
            "PLAN_023",
            "declares no executable verify steps",
        ),
        (
            task.acceptance_contract.is_none(),
            "PLAN_024",
            "declares no typed acceptance_contract",
        ),
        (
            !task.has_required_parity_ledger_rows,
            "PLAN_025",
            "declares no required parity ledger rows",
        ),
    ];

    for (missing, rule_id, message) in requirements {
        if missing {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                rule_id: rule_id.to_string(),
                plan_id: Some(plan_id.to_string()),
                task_id: task.task_id.clone(),
                message: format!("architecture queue task '{}' {message}", task.label()),
            });
        }
    }

    for field in &task.deferral_missing_fields {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            rule_id: "PLAN_026".to_string(),
            plan_id: Some(plan_id.to_string()),
            task_id: task.task_id.clone(),
            message: format!(
                "architecture queue task '{}' has incomplete deferral metadata: missing {field}",
                task.label()
            ),
        });
    }
}

fn has_required_parity_ledger_rows(table: &toml::map::Map<String, Value>) -> bool {
    table
        .get("acceptance_contract")
        .and_then(Value::as_table)
        .and_then(|contract| contract.get("parity_ledger"))
        .and_then(Value::as_table)
        .filter(|parity| {
            parity
                .get("required")
                .and_then(Value::as_bool)
                .unwrap_or(true)
        })
        .and_then(|parity| parity.get("rows"))
        .and_then(Value::as_array)
        .is_some_and(|rows| !rows.is_empty())
}

fn deferral_missing_fields(table: &toml::map::Map<String, Value>) -> Vec<&'static str> {
    let string_array_present = |field: &str| {
        table
            .get(field)
            .and_then(Value::as_array)
            .is_some_and(|items| {
                items
                    .iter()
                    .any(|item| item.as_str().is_some_and(|value| !value.trim().is_empty()))
            })
    };

    let mut missing = Vec::new();
    if !table
        .get("rationale")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
    {
        missing.push("deferral.rationale");
    }
    for field in [
        "prerequisite_runtime_policy_gates",
        "acceptance_gates",
        "risk_notes",
        "parity_requirements",
    ] {
        if !string_array_present(field) {
            missing.push(match field {
                "prerequisite_runtime_policy_gates" => "deferral.prerequisite_runtime_policy_gates",
                "acceptance_gates" => "deferral.acceptance_gates",
                "risk_notes" => "deferral.risk_notes",
                "parity_requirements" => "deferral.parity_requirements",
                _ => unreachable!("checked field list is exhaustive"),
            });
        }
    }
    missing
}

fn string_field(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalized_field(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn emit_required_field(
    diagnostics: &mut Vec<Diagnostic>,
    plan_id: &str,
    task: &TaskSnapshot,
    field: &'static str,
    value: Option<&str>,
) {
    if value.is_some_and(|value| !value.trim().is_empty()) {
        return;
    }

    diagnostics.push(Diagnostic {
        severity: Severity::Error,
        rule_id: "PLAN_003".to_string(),
        plan_id: Some(plan_id.to_string()),
        task_id: task.task_id.clone(),
        message: format!(
            "task '{}' is missing required field '{}'",
            task.label(),
            field
        ),
    });
}

fn reachable_from_roots(tasks: &[TaskSnapshot], known_ids: &HashSet<String>) -> HashSet<String> {
    let mut dependents = HashMap::<String, Vec<String>>::new();
    let mut roots = Vec::new();

    for task in tasks {
        let Some(task_id) = normalized_field(task.task_id.as_deref()) else {
            continue;
        };

        let valid_deps = task
            .depends_on
            .iter()
            .filter(|dependency| known_ids.contains(dependency.as_str()))
            .cloned()
            .collect::<Vec<_>>();

        if valid_deps.is_empty() && task.depends_on.is_empty() {
            roots.push(task_id.to_string());
        }

        for dependency in valid_deps {
            dependents
                .entry(dependency)
                .or_default()
                .push(task_id.to_string());
        }
    }

    roots.sort();
    for children in dependents.values_mut() {
        children.sort();
    }

    let mut reachable = HashSet::new();
    let mut stack = roots;
    while let Some(task_id) = stack.pop() {
        if !reachable.insert(task_id.clone()) {
            continue;
        }
        if let Some(children) = dependents.get(&task_id) {
            for child in children.iter().rev() {
                stack.push(child.clone());
            }
        }
    }

    reachable
}

fn role_has_compose_template(role: AgentRole) -> bool {
    matches!(
        role,
        AgentRole::Strategist
            | AgentRole::Implementer
            | AgentRole::Architect
            | AgentRole::Auditor
            | AgentRole::QuickReviewer
            | AgentRole::Scribe
            | AgentRole::Critic
            | AgentRole::AutoFixer
            | AgentRole::IntegrationTester
            | AgentRole::Refactorer
    )
}

fn parse_task_role(role: &str) -> Option<AgentRole> {
    let normalized = role.trim().to_ascii_lowercase();
    let normalized = normalized
        .strip_prefix("agentrole::")
        .unwrap_or(&normalized)
        .replace(['_', ' '], "-");
    Some(match normalized.as_str() {
        "conductor" => AgentRole::Conductor,
        "strategist" => AgentRole::Strategist,
        "implementer" | "engineer" | "coder" => AgentRole::Implementer,
        "architect" => AgentRole::Architect,
        "researcher" => AgentRole::Researcher,
        "auditor" => AgentRole::Auditor,
        "quick-reviewer" | "quickreviewer" => AgentRole::QuickReviewer,
        "scribe" => AgentRole::Scribe,
        "critic" => AgentRole::Critic,
        "auto-fixer" | "autofixer" => AgentRole::AutoFixer,
        "refactorer" => AgentRole::Refactorer,
        "pre-planner" | "preplanner" => AgentRole::PrePlanner,
        "doc-verifier" | "docverifier" => AgentRole::DocVerifier,
        "integration-tester" | "integrationtester" => AgentRole::IntegrationTester,
        "merge-resolver" | "mergeresolver" => AgentRole::MergeResolver,
        "terminal-validator" | "terminalvalidator" => AgentRole::TerminalValidator,
        "golem-lifecycle-tester" | "golemlifecycletester" => AgentRole::GolemLifecycleTester,
        "spec-drift-detector" | "specdriftdetector" => AgentRole::SpecDriftDetector,
        "regression-detector" | "regressiondetector" => AgentRole::RegressionDetector,
        "performance-sentinel" | "performancesentinel" => AgentRole::PerformanceSentinel,
        "coverage-tracker" | "coveragetracker" => AgentRole::CoverageTracker,
        "plan-lifecycle-mgr" | "plan-lifecycle-manager" | "planlifecyclemanager" => {
            AgentRole::PlanLifecycleManager
        }
        "cross-system-tester" | "crosssystemtester" => AgentRole::CrossSystemTester,
        "error-diagnoser" | "errordiagnoser" => AgentRole::ErrorDiagnoser,
        "dep-validator" | "dependency-validator" | "dependencyvalidator" => {
            AgentRole::DependencyValidator
        }
        "pattern-extractor" | "patternextractor" => AgentRole::PatternExtractor,
        "snapshot-comparator" | "snapshotcomparator" => AgentRole::SnapshotComparator,
        "full-loop-validator" | "fullloopvalidator" => AgentRole::FullLoopValidator,
        _ => return None,
    })
}

fn model_is_known(model: &str, known_models: &IndexMap<String, ModelProfile>) -> bool {
    known_models.contains_key(model) || known_models.values().any(|profile| profile.slug == model)
}

/// Extract the workspace package/crate name from a task file path.
///
/// Returns `Some(name)` for paths like `crates/roko-foo/src/lib.rs` and
/// `packages/app-one/package.json`. Returns `None` for paths outside those
/// workspace roots.
fn extract_crate_from_path(path: &str) -> Option<&str> {
    let parts: Vec<&str> = path.splitn(3, '/').collect();
    if parts.len() >= 2 && (parts[0] == "crates" || parts[0] == "packages") {
        Some(parts[1])
    } else {
        None
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct TaskPathReferences {
    task_id: String,
    outputs: BTreeSet<String>,
    prerequisites: BTreeSet<String>,
    depends_on: Vec<String>,
    depends_on_plan: Vec<String>,
}

fn string_array(table: &toml::map::Map<String, Value>, field: &str) -> Vec<String> {
    table
        .get(field)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.as_str()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Classify task paths according to the executable task schema.
///
/// `files` and `write_files` are mutation outputs and may be created by the
/// task. `context.read_files` are inputs that must exist before dispatch,
/// unless a declared dependency produces the exact path first.
fn collect_task_path_references(parsed: &Value) -> Vec<TaskPathReferences> {
    let mut out = Vec::new();
    let tasks = parsed
        .get("task")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    for task in tasks {
        let table = match task.as_table() {
            Some(table) => table,
            None => continue,
        };

        let task_id = table
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("unknown")
            .to_string();

        let mut outputs = BTreeSet::new();
        for field in ["files", "write_files"] {
            outputs.extend(string_array(table, field));
        }

        let prerequisites = table
            .get("context")
            .and_then(Value::as_table)
            .and_then(|context| context.get("read_files"))
            .and_then(Value::as_array)
            .map(|files| {
                files
                    .iter()
                    .filter_map(|file| {
                        file.as_table()
                            .and_then(|table| table.get("path"))
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(ToOwned::to_owned)
                    })
                    .collect()
            })
            .unwrap_or_default();

        out.push(TaskPathReferences {
            task_id,
            outputs,
            prerequisites,
            depends_on: string_array(table, "depends_on"),
            depends_on_plan: string_array(table, "depends_on_plan"),
        });
    }

    out
}

fn parsed_plan_id(parsed: &Value) -> String {
    parsed
        .get("meta")
        .and_then(Value::as_table)
        .and_then(|meta| meta.get("plan"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown-plan")
        .to_string()
}

fn collect_plan_output_paths(tasks_files: &[PathBuf]) -> HashMap<String, BTreeSet<String>> {
    let mut outputs = HashMap::<String, BTreeSet<String>>::new();

    for tasks_path in tasks_files {
        let Ok(content) = std::fs::read_to_string(tasks_path) else {
            continue;
        };
        let Ok(parsed) = toml::from_str::<Value>(&content) else {
            continue;
        };
        let plan_outputs = outputs.entry(parsed_plan_id(&parsed)).or_default();
        for task in collect_task_path_references(&parsed) {
            plan_outputs.extend(task.outputs);
        }
    }

    outputs
}

fn dependency_created_paths(
    task: &TaskPathReferences,
    tasks_by_id: &HashMap<String, TaskPathReferences>,
    plan_output_paths: &HashMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut created = BTreeSet::new();
    let mut visited = HashSet::new();
    let mut pending = task.depends_on.clone();

    while let Some(dependency_id) = pending.pop() {
        if !visited.insert(dependency_id.clone()) {
            continue;
        }
        if let Some(dependency) = tasks_by_id.get(&dependency_id) {
            created.extend(dependency.outputs.iter().cloned());
            pending.extend(dependency.depends_on.iter().cloned());
        }
    }

    for plan_id in &task.depends_on_plan {
        if let Some(outputs) = plan_output_paths.get(plan_id) {
            created.extend(outputs.iter().cloned());
        }
    }

    created
}

fn missing_prerequisite_diagnostic(
    plan_id: &str,
    task_id: &str,
    file_path: &str,
    existing_crates: &HashSet<String>,
    existing_packages: &HashSet<String>,
) -> Diagnostic {
    let missing_workspace_root = match file_path.split_once('/') {
        Some(("crates", _)) => extract_crate_from_path(file_path)
            .filter(|crate_name| !existing_crates.contains(*crate_name))
            .map(|crate_name| ("crate", crate_name, "crates/")),
        Some(("packages", _)) => extract_crate_from_path(file_path)
            .filter(|package_name| !existing_packages.contains(*package_name))
            .map(|package_name| ("package", package_name, "packages/")),
        _ => None,
    };

    if let Some((kind, name, root)) = missing_workspace_root {
        Diagnostic {
            severity: Severity::Warning,
            rule_id: "PLAN_030".to_string(),
            plan_id: Some(plan_id.to_string()),
            task_id: Some(task_id.to_string()),
            message: format!(
                "task '{}' requires prerequisite '{}' in {} '{}' which does not exist in {}",
                task_id, file_path, kind, name, root
            ),
        }
    } else {
        Diagnostic {
            severity: Severity::Warning,
            rule_id: "PLAN_031".to_string(),
            plan_id: Some(plan_id.to_string()),
            task_id: Some(task_id.to_string()),
            message: format!(
                "task '{}' requires prerequisite '{}' which does not exist on disk and is not created by a declared dependency",
                task_id, file_path
            ),
        }
    }
}

fn validate_file_references_with_plan_outputs(
    tasks_path: &Path,
    workdir: &Path,
    plan_output_paths: &HashMap<String, BTreeSet<String>>,
) -> Result<Vec<Diagnostic>> {
    let content = std::fs::read_to_string(tasks_path)
        .with_context(|| format!("read {}", tasks_path.display()))?;
    let parsed: Value =
        toml::from_str(&content).with_context(|| format!("parse TOML {}", tasks_path.display()))?;
    let plan_id = parsed_plan_id(&parsed);
    let existing_crates = collect_workspace_package_names(workdir, "crates");
    let existing_packages = collect_workspace_package_names(workdir, "packages");
    let tasks = collect_task_path_references(&parsed);
    let tasks_by_id = tasks
        .iter()
        .cloned()
        .map(|task| (task.task_id.clone(), task))
        .collect::<HashMap<_, _>>();
    let mut diagnostics = Vec::new();

    for task in &tasks {
        let created_by_dependencies =
            dependency_created_paths(task, &tasks_by_id, plan_output_paths);
        for prerequisite in &task.prerequisites {
            if workdir.join(prerequisite).exists() || created_by_dependencies.contains(prerequisite)
            {
                continue;
            }

            diagnostics.push(missing_prerequisite_diagnostic(
                &plan_id,
                &task.task_id,
                prerequisite,
                &existing_crates,
                &existing_packages,
            ));
        }
    }

    Ok(diagnostics)
}

fn collect_workspace_package_names(workdir: &Path, root_dir_name: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    let root = workdir.join(root_dir_name);
    let Ok(entries) = std::fs::read_dir(&root) else {
        return names;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            names.insert(name.to_string());
        }
    }

    names
}

/// Validate that task prerequisites exist in the workspace.
///
/// Task `files`/`write_files` are outputs and may be created by the task.
/// `context.read_files` are prerequisites and must already exist, unless an
/// explicitly declared task dependency produces the exact path.
pub fn validate_file_references(tasks_path: &Path, workdir: &Path) -> Result<Vec<Diagnostic>> {
    validate_file_references_with_plan_outputs(tasks_path, workdir, &HashMap::new())
}

/// Phrases that indicate ungrounded generation in an existing workspace.
///
/// These phrases should never appear in a plan generated for a workspace with
/// existing crates.
const GREENFIELD_PHRASES: &[&str] = &[
    "no rust crates exist",
    "no existing crates",
    "starting from scratch",
    "greenfield project",
    "greenfield implementation",
    "new project from scratch",
    "no existing code",
    "empty workspace",
];

/// Validate that a plan does not treat an existing workspace as greenfield.
///
/// Checks:
/// 1. Task prompts for "create crate X" where X already exists -> PLAN_032 Error
/// 2. Task prompts for banned greenfield phrases in a non-empty workspace -> PLAN_033 Error
///
/// If workspace is empty (no crates dir or zero members), all checks are skipped.
fn validate_no_greenfield_duplicates(
    parsed: &Value,
    plan_id: &str,
    existing_crates: &HashSet<String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if existing_crates.is_empty() {
        return diagnostics;
    }

    let plan_id = plan_id.to_owned();
    let existing_crates_lower = existing_crates
        .iter()
        .map(|crate_name| crate_name.to_ascii_lowercase())
        .collect::<HashSet<_>>();

    let tasks = parsed
        .get("task")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    for task in tasks {
        let table = match task.as_table() {
            Some(table) => table,
            None => continue,
        };
        let task_id = table
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();

        let text_fields = [
            table.get("prompt").and_then(Value::as_str),
            table.get("description").and_then(Value::as_str),
            table.get("title").and_then(Value::as_str),
        ];

        for text in text_fields.into_iter().flatten() {
            let text_lower = text.to_ascii_lowercase();

            for pattern in &["create crate ", "new crate ", "add crate "] {
                if let Some(idx) = text_lower.find(pattern) {
                    let after_raw = &text[idx + pattern.len()..];
                    let candidate = after_raw.trim_start_matches(|c: char| {
                        c.is_whitespace()
                            || matches!(
                                c,
                                '`' | '"' | '\'' | ':' | ',' | '.' | ';' | '(' | '[' | '{' | '='
                            )
                    });
                    let proposed = candidate
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                        .collect::<String>()
                        .to_ascii_lowercase();

                    if !proposed.is_empty() && existing_crates_lower.contains(&proposed) {
                        diagnostics.push(Diagnostic {
                            severity: Severity::Error,
                            rule_id: "PLAN_032".to_string(),
                            plan_id: Some(plan_id.clone()),
                            task_id: Some(task_id.clone()),
                            message: format!(
                                "task '{}' proposes creating crate '{}' which already exists in the workspace",
                                task_id, proposed
                            ),
                        });
                    }
                }
            }

            for phrase in GREENFIELD_PHRASES {
                if text_lower.contains(phrase) {
                    diagnostics.push(Diagnostic {
                        severity: Severity::Error,
                        rule_id: "PLAN_033".to_string(),
                        plan_id: Some(plan_id.clone()),
                        task_id: Some(task_id.clone()),
                        message: format!(
                            "task '{}' contains greenfield claim '{}' but workspace has {} existing crates",
                            task_id,
                            phrase,
                            existing_crates.len()
                        ),
                    });
                }
            }
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn extract_crate_from_path_handles_workspace_roots() {
        assert_eq!(
            extract_crate_from_path("crates/roko-foo/src/lib.rs"),
            Some("roko-foo")
        );
        assert_eq!(
            extract_crate_from_path("packages/app-one/package.json"),
            Some("app-one")
        );
        assert_eq!(extract_crate_from_path("src/lib.rs"), None);
    }

    #[test]
    fn collect_task_path_references_classifies_outputs_and_prerequisites() {
        let parsed: Value = toml::from_str(
            r#"
[meta]
plan = "demo"

[[task]]
id = "T1"
files = ["src/lib.rs", "docs/guide.md"]
write_files = ["crates/roko-cli/src/plan_validate.rs", " "]
depends_on = []

[task.context]
read_files = [
  { path = "Cargo.toml", why = "workspace manifest" },
  { path = " ", why = "ignored empty path" },
]

[[task]]
id = "T2"
write_files = ["docs/guide.md", "packages/app-one/package.json"]
depends_on = ["T1"]
depends_on_plan = ["foundation"]
"#,
        )
        .unwrap();

        let refs = collect_task_path_references(&parsed);
        assert_eq!(refs.len(), 2);
        assert_eq!(
            refs[0].outputs,
            BTreeSet::from([
                "crates/roko-cli/src/plan_validate.rs".to_string(),
                "docs/guide.md".to_string(),
                "src/lib.rs".to_string(),
            ])
        );
        assert_eq!(
            refs[0].prerequisites,
            BTreeSet::from(["Cargo.toml".to_string()])
        );
        assert_eq!(refs[1].depends_on, vec!["T1"]);
        assert_eq!(refs[1].depends_on_plan, vec!["foundation"]);
    }

    #[test]
    fn validate_file_references_warns_for_missing_workspace_paths() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::create_dir_all(root.join("plans/demo")).unwrap();
        fs::create_dir_all(root.join("crates/existing-crate/src")).unwrap();
        fs::write(
            root.join("crates/existing-crate/Cargo.toml"),
            "[package]\nname = \"existing-crate\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(root.join("crates/existing-crate/src/lib.rs"), "// exists\n").unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/existing.md"), "# exists\n").unwrap();
        fs::create_dir_all(root.join("packages/existing-app")).unwrap();
        fs::write(
            root.join("packages/existing-app/package.json"),
            "{\n  \"name\": \"existing-app\"\n}\n",
        )
        .unwrap();

        let tasks_path = root.join("plans/demo/tasks.toml");
        fs::write(
            &tasks_path,
            r#"
[meta]
plan = "demo"

[[task]]
id = "T1"
title = "Validate file refs"
role = "implementer"
files = [
  "crates/output-crate/src/lib.rs",
  "docs/generated.md",
]
depends_on = []
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]

[task.context]
read_files = [
  { path = "crates/existing-crate/src/lib.rs", why = "existing input" },
  { path = "crates/existing-crate/src/missing.rs", why = "missing file in existing crate" },
  { path = "crates/missing-crate/src/lib.rs", why = "missing crate input" },
  { path = "docs/existing.md", why = "existing input" },
  { path = "docs/missing.md", why = "missing input" },
  { path = "packages/existing-app/package.json", why = "existing package input" },
  { path = "packages/missing-app/package.json", why = "missing package input" },
]
"#,
        )
        .unwrap();

        let plans_dir = root.join("plans");

        let no_workdir = validate_plans_dir(plans_dir.as_path(), None).unwrap();
        assert_eq!(no_workdir.totals.errors, 0);
        assert_eq!(no_workdir.totals.warnings, 0);

        let diagnostics = validate_file_references(&tasks_path, root).unwrap();
        assert_eq!(diagnostics.len(), 4);
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.rule_id == "PLAN_030" && diag.message.contains("missing-crate"))
        );
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.rule_id == "PLAN_030" && diag.message.contains("missing-app"))
        );
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.rule_id == "PLAN_031" && diag.message.contains("docs/missing.md"))
        );
        assert!(diagnostics.iter().any(|diag| {
            diag.rule_id == "PLAN_031" && diag.message.contains("existing-crate/src/missing.rs")
        }));
        assert!(
            !diagnostics
                .iter()
                .any(|diag| diag.message.contains("existing-crate/src/lib.rs"))
        );
        assert!(
            !diagnostics
                .iter()
                .any(|diag| diag.message.contains("existing-app/package.json"))
        );
        assert!(
            !diagnostics
                .iter()
                .any(|diag| diag.message.contains("output-crate")
                    || diag.message.contains("generated.md"))
        );

        let with_workdir =
            validate_plans_dir_with_workdir(plans_dir.as_path(), None, Some(root)).unwrap();
        assert_eq!(with_workdir.totals.errors, 0);
        assert_eq!(with_workdir.totals.warnings, 4);
        assert_eq!(with_workdir.plans.len(), 1);
        assert!(
            with_workdir.plans[0]
                .diagnostics
                .iter()
                .any(|diag| diag.rule_id == "PLAN_030")
        );
        assert!(
            with_workdir.plans[0]
                .diagnostics
                .iter()
                .any(|diag| diag.rule_id == "PLAN_031")
        );
    }

    #[test]
    fn validate_file_references_allows_only_declared_dependency_outputs() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("plans/demo")).unwrap();
        let tasks_path = root.join("plans/demo/tasks.toml");
        fs::write(
            &tasks_path,
            r#"
[meta]
plan = "demo"

[[task]]
id = "T1"
title = "Create generated input"
role = "implementer"
files = ["generated/input.md"]
depends_on = []
verify = [{ phase = "structural", command = "test -f generated/input.md" }]

[[task]]
id = "T2"
title = "Read generated input after its producer"
role = "implementer"
files = ["src/consumer.rs"]
depends_on = ["T1"]
verify = [{ phase = "compile", command = "echo ok" }]

[task.context]
read_files = [{ path = "generated/input.md", why = "created by T1" }]

[[task]]
id = "T3"
title = "Read generated input without declaring its producer"
role = "implementer"
files = ["src/other.rs"]
depends_on = []
verify = [{ phase = "compile", command = "echo ok" }]

[task.context]
read_files = [{ path = "generated/input.md", why = "undeclared producer" }]
"#,
        )
        .unwrap();

        let diagnostics = validate_file_references(&tasks_path, root).unwrap();
        assert_eq!(
            diagnostics.len(),
            1,
            "unexpected diagnostics: {diagnostics:?}"
        );
        assert_eq!(diagnostics[0].task_id.as_deref(), Some("T3"));
        assert_eq!(diagnostics[0].rule_id, "PLAN_031");
    }

    #[test]
    fn validate_no_greenfield_duplicates_flags_existing_crate_and_phrases() {
        let parsed: Value = toml::from_str(
            r#"
[meta]
plan = "demo-plan"

[[task]]
id = "T1"
prompt = "Please Create Crate ROKO-COMPOSE."
title = "Build the bridge"

[[task]]
id = "T2"
description = "This is a GREENFIELD PROJECT starting from scratch."
title = "Rewrite the pipeline"

[[task]]
id = "T3"
title = "Add crate ROKO-CORE"
"#,
        )
        .unwrap();
        let existing_crates = ["roko-compose".to_string(), "roko-core".to_string()]
            .into_iter()
            .collect::<HashSet<_>>();

        let diagnostics = validate_no_greenfield_duplicates(&parsed, "demo-plan", &existing_crates);

        assert_eq!(
            diagnostics
                .iter()
                .filter(|diag| diag.rule_id == "PLAN_032")
                .count(),
            2
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diag| diag.rule_id == "PLAN_033")
                .count(),
            2
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag.rule_id == "PLAN_032" && diag.task_id.as_deref() == Some("T1")
            })
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag.rule_id == "PLAN_033" && diag.task_id.as_deref() == Some("T2")
            })
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag.rule_id == "PLAN_032" && diag.task_id.as_deref() == Some("T3")
            })
        );
    }

    #[test]
    fn validate_plans_dir_with_workdir_rejects_greenfield_duplicates() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::create_dir_all(root.join("plans/demo")).unwrap();
        fs::create_dir_all(root.join("crates/roko-compose")).unwrap();
        fs::create_dir_all(root.join("crates/roko-core")).unwrap();

        let tasks_path = root.join("plans/demo/tasks.toml");
        fs::write(
            &tasks_path,
            r#"
[meta]
plan = "demo-plan"

[[task]]
id = "T1"
title = "Create crate ROKO-COMPOSE"
prompt = "Please create crate ROKO-COMPOSE for the workspace."
role = "implementer"
depends_on = []
files = ["crates/roko-compose/src/lib.rs"]
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]

[[task]]
id = "T2"
title = "Rewrite the pipeline"
description = "This is a greenfield implementation starting from scratch."
role = "implementer"
depends_on = []
files = ["crates/roko-core/src/lib.rs"]
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]
"#,
        )
        .unwrap();

        let report =
            validate_plans_dir_with_workdir(root.join("plans").as_path(), None, Some(root))
                .unwrap();

        assert_eq!(report.totals.errors, 4);
        assert!(report.plans.iter().any(|plan| {
            plan.diagnostics
                .iter()
                .any(|diag| diag.rule_id == "PLAN_032" && diag.task_id.as_deref() == Some("T1"))
        }));
        assert!(report.plans.iter().any(|plan| {
            plan.diagnostics
                .iter()
                .any(|diag| diag.rule_id == "PLAN_033" && diag.task_id.as_deref() == Some("T2"))
        }));
    }

    #[test]
    fn validate_plans_dir_with_workdir_skips_greenfield_checks_for_empty_workspace() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::create_dir_all(root.join("plans/demo")).unwrap();
        let tasks_path = root.join("plans/demo/tasks.toml");
        fs::write(
            &tasks_path,
            r#"
[meta]
plan = "demo-plan"

[[task]]
id = "T1"
title = "Create crate roko-compose"
prompt = "Please create crate roko-compose."
role = "implementer"
depends_on = []
files = ["crates/roko-compose/src/lib.rs"]
verify = [{ phase = "compile", command = "cargo check -p roko-cli" }]
"#,
        )
        .unwrap();

        let report =
            validate_plans_dir_with_workdir(root.join("plans").as_path(), None, Some(root))
                .unwrap();

        assert_eq!(report.totals.errors, 0);
        assert!(report.plans.iter().all(|plan| {
            plan.diagnostics
                .iter()
                .all(|diag| diag.rule_id != "PLAN_032" && diag.rule_id != "PLAN_033")
        }));
    }
}
