#[allow(clippy::items_after_test_module)]
mod plan_validation {
    #![allow(dead_code, unused_imports)]

    include!("../src/plan_validate.rs");

    #[cfg(test)]
    mod plan_validation_checks {
        use super::*;
        use indexmap::IndexMap;
        use roko_core::config::schema::ModelProfile;
        use std::collections::{HashMap, HashSet};
        use std::fs;
        use std::path::{Path, PathBuf};
        use tempfile::TempDir;
        use toml::Value;

        fn write_tasks_file(root: &Path, plan_id: &str, tasks_toml: &str) -> PathBuf {
            let plan_dir = root.join("plans").join(plan_id);
            fs::create_dir_all(&plan_dir).unwrap();
            let tasks_path = plan_dir.join("tasks.toml");
            fs::write(&tasks_path, tasks_toml).unwrap();
            tasks_path
        }

        fn task_toml(plan_id: &str, title: &str, role: Option<&str>, extra: &str) -> String {
            let role_line = role
                .map(|value| format!("role = \"{value}\"\n"))
                .unwrap_or_default();
            let files_line = if extra.contains("files =") || extra.contains("write_files =") {
                String::new()
            } else {
                "files = [\"src/lib.rs\"]\n".to_string()
            };
            let verify_line = if extra.contains("verify =") {
                String::new()
            } else {
                "verify = [{ phase = \"compile\", command = \"cargo check -p roko-cli\" }]\n"
                    .to_string()
            };
            format!(
                "[meta]\nplan = \"{plan_id}\"\n\n[[task]]\nid = \"T1\"\ntitle = \"{title}\"\n{role_line}depends_on = []\n{files_line}{verify_line}{extra}"
            )
        }

        fn validate_plan_report(
            root: &Path,
            plan_id: &str,
            tasks_toml: &str,
            models: Option<&IndexMap<String, ModelProfile>>,
        ) -> ValidationReport {
            write_tasks_file(root, plan_id, tasks_toml);
            validate_plans_dir(&root.join("plans"), models).unwrap()
        }

        fn known_models() -> IndexMap<String, ModelProfile> {
            let mut models = IndexMap::new();
            for slug in ["claude-haiku-4-5", "claude-sonnet-4-6", "claude-opus-4-6"] {
                models.insert(
                    slug.to_string(),
                    ModelProfile {
                        provider: "claude_cli".to_string(),
                        slug: slug.to_string(),
                        ..Default::default()
                    },
                );
            }
            models
        }

        fn greenfield_diagnostics(tasks_toml: &str, existing_crates: &[&str]) -> Vec<Diagnostic> {
            let parsed: Value = toml::from_str(tasks_toml).unwrap();
            let existing_crates = existing_crates
                .iter()
                .map(|crate_name| (*crate_name).to_string())
                .collect::<HashSet<_>>();
            validate_no_greenfield_duplicates(&parsed, "demo-plan", &existing_crates)
        }

        fn first_diagnostics(report: &ValidationReport) -> &[Diagnostic] {
            &report
                .plans
                .first()
                .expect("expected diagnostics for this validation case")
                .diagnostics
        }

        #[test]
        fn plan_validation_missing_role_produces_error() {
            let temp = TempDir::new().unwrap();
            let report = validate_plan_report(
                temp.path(),
                "missing-role",
                &task_toml("missing-role", "Missing role", None, ""),
                None,
            );

            assert_eq!(report.totals.plans_checked, 1);
            assert_eq!(report.totals.errors, 1);
            assert_eq!(report.totals.warnings, 0);
            let diagnostics = first_diagnostics(&report);
            assert!(
                diagnostics.iter().any(|diag| {
                    diag.rule_id == "PLAN_003"
                        && diag.task_id.as_deref() == Some("T1")
                        && diag.message.contains("missing required field 'role'")
                }),
                "missing role should be rejected: {diagnostics:?}"
            );
        }

        #[test]
        fn plan_validation_known_role_passes() {
            let temp = TempDir::new().unwrap();
            let report = validate_plan_report(
                temp.path(),
                "known-role",
                &task_toml("known-role", "Known role", Some("implementer"), ""),
                None,
            );

            assert_eq!(report.totals.plans_checked, 1);
            assert_eq!(report.totals.errors, 0);
            assert_eq!(report.totals.warnings, 0);
            assert!(
                report.plans.is_empty(),
                "unexpected diagnostics: {report:?}"
            );
        }

        #[test]
        fn plan_validation_unknown_role_warns() {
            let temp = TempDir::new().unwrap();
            let report = validate_plan_report(
                temp.path(),
                "unknown-role",
                &task_toml("unknown-role", "Unknown role", Some("wizard"), ""),
                None,
            );

            assert_eq!(report.totals.plans_checked, 1);
            assert_eq!(report.totals.errors, 1);
            assert_eq!(report.totals.warnings, 1);
            let diagnostics = first_diagnostics(&report);
            assert!(
                diagnostics.iter().any(|diag| {
                    diag.rule_id == "PLAN_035"
                        && diag
                            .message
                            .contains("unknown role 'wizard' (valid: implementer")
                }),
                "unknown role should produce schema error: {diagnostics:?}"
            );
            assert!(
                diagnostics.iter().any(|diag| {
                    diag.rule_id == "PLAN_008"
                        && diag.task_id.as_deref() == Some("T1")
                        && diag
                            .message
                            .contains("uses role 'wizard' which has no template")
                }),
                "unknown role should produce PLAN_008: {diagnostics:?}"
            );
        }

        #[test]
        fn plan_validation_normalizes_known_model_aliases() {
            for (alias, canonical) in [
                ("haiku", "claude-haiku-4-5"),
                ("sonnet", "claude-sonnet-4-6"),
                ("opus", "claude-opus-4-6"),
            ] {
                assert_eq!(
                    normalize_model_alias(alias),
                    canonical,
                    "alias '{alias}' should normalize to '{canonical}'"
                );
            }
        }

        #[test]
        fn plan_validation_unknown_model_alias_does_not_normalize() {
            // Unknown aliases pass through unchanged
            assert_eq!(normalize_model_alias("gpt2"), "gpt2");
            assert_eq!(normalize_model_alias(" claude-unknown "), "claude-unknown");
        }

        #[test]
        fn plan_validation_alias_in_task_warns() {
            let temp = TempDir::new().unwrap();
            let models = known_models();
            let report = validate_plan_report(
                temp.path(),
                "alias",
                &task_toml(
                    "alias",
                    "Mechanical alias",
                    Some("implementer"),
                    "model_hint = \"sonnet\"\n",
                ),
                Some(&models),
            );

            assert_eq!(report.totals.plans_checked, 1);
            assert_eq!(report.totals.errors, 0);
            assert_eq!(report.totals.warnings, 1);
            let diagnostics = first_diagnostics(&report);
            assert!(
                diagnostics.iter().any(|diag| {
                    diag.rule_id == "PLAN_012"
                        && diag.task_id.as_deref() == Some("T1")
                        && diag.message.contains("uses model alias 'sonnet'")
                        && diag.message.contains("claude-sonnet-4-6")
                }),
                "alias should produce PLAN_012 warning: {diagnostics:?}"
            );
        }

        #[test]
        fn plan_validation_unknown_short_model_warns() {
            let temp = TempDir::new().unwrap();
            let models = known_models();
            let report = validate_plan_report(
                temp.path(),
                "mystery-model",
                &task_toml(
                    "mystery-model",
                    "Unknown model",
                    Some("implementer"),
                    "model_hint = \"gpt2\"\n",
                ),
                Some(&models),
            );

            assert_eq!(report.totals.plans_checked, 1);
            assert_eq!(report.totals.errors, 0);
            assert_eq!(report.totals.warnings, 1);
            let diagnostics = first_diagnostics(&report);
            assert!(
                diagnostics.iter().any(|diag| {
                    diag.rule_id == "PLAN_009"
                        && diag.task_id.as_deref() == Some("T1")
                        && diag
                            .message
                            .contains("uses model 'gpt2' which is not configured in roko.toml")
                }),
                "unknown model should produce PLAN_009 warning: {diagnostics:?}"
            );
        }

        #[test]
        fn plan_validation_full_model_name_passes() {
            let temp = TempDir::new().unwrap();
            let models = known_models();
            let report = validate_plan_report(
                temp.path(),
                "canonical-model",
                &task_toml(
                    "canonical-model",
                    "Canonical model",
                    Some("implementer"),
                    "model_hint = \"claude-sonnet-4-6\"\n",
                ),
                Some(&models),
            );

            assert_eq!(report.totals.plans_checked, 1);
            assert_eq!(report.totals.errors, 0);
            assert_eq!(report.totals.warnings, 0);
            assert!(
                report.plans.is_empty(),
                "unexpected diagnostics: {report:?}"
            );
        }

        #[test]
        fn plan_validation_missing_model_field_passes() {
            let temp = TempDir::new().unwrap();
            let models = known_models();
            let report = validate_plan_report(
                temp.path(),
                "missing-model",
                &task_toml("missing-model", "Missing model", Some("implementer"), ""),
                Some(&models),
            );

            assert_eq!(report.totals.plans_checked, 1);
            assert_eq!(report.totals.errors, 0);
            assert_eq!(report.totals.warnings, 0);
            assert!(
                report.plans.is_empty(),
                "unexpected diagnostics: {report:?}"
            );
        }

        #[test]
        fn plan_validation_missing_prerequisites_warn_but_missing_outputs_do_not() {
            let temp = TempDir::new().unwrap();
            fs::create_dir_all(temp.path().join("crates/roko-core/src")).unwrap();
            fs::write(
                temp.path().join("crates/roko-core/src/lib.rs"),
                "// existing crate file\n",
            )
            .unwrap();
            fs::create_dir_all(temp.path().join("packages/existing-app")).unwrap();
            fs::write(
                temp.path().join("packages/existing-app/package.json"),
                "{\n  \"name\": \"existing-app\"\n}\n",
            )
            .unwrap();
            fs::create_dir_all(temp.path().join("docs")).unwrap();
            fs::write(temp.path().join("docs/existing.md"), "# exists\n").unwrap();

            let tasks_path = write_tasks_file(
                temp.path(),
                "refs",
                &task_toml(
                    "refs",
                    "Validate file refs",
                    Some("implementer"),
                    r#"files = [
  "crates/output-crate/src/lib.rs",
  "docs/generated.md",
]

[task.context]
read_files = [
  { path = "crates/roko-core/src/lib.rs", why = "existing crate input" },
  { path = "crates/missing-crate/src/lib.rs", why = "missing crate input" },
  { path = "packages/existing-app/package.json", why = "existing package input" },
  { path = "packages/missing-app/package.json", why = "missing package input" },
  { path = "docs/existing.md", why = "existing doc input" },
  { path = "docs/missing.md", why = "missing doc input" },
]
"#,
                ),
            );

            let diagnostics = validate_file_references(&tasks_path, temp.path()).unwrap();
            assert_eq!(
                diagnostics.len(),
                3,
                "unexpected diagnostics: {diagnostics:?}"
            );
            assert!(
                diagnostics.iter().any(|diag| {
                    diag.rule_id == "PLAN_030" && diag.message.contains("missing-crate")
                }),
                "missing crate reference should be reported: {diagnostics:?}"
            );
            assert!(
                diagnostics.iter().any(|diag| {
                    diag.rule_id == "PLAN_030" && diag.message.contains("missing-app")
                }),
                "missing package reference should be reported: {diagnostics:?}"
            );
            assert!(
                diagnostics.iter().any(|diag| {
                    diag.rule_id == "PLAN_031" && diag.message.contains("docs/missing.md")
                }),
                "missing file reference should be reported: {diagnostics:?}"
            );
            assert!(
                !diagnostics
                    .iter()
                    .any(|diag| diag.message.contains("existing.md")),
                "existing file should not be reported: {diagnostics:?}"
            );
            assert!(
                !diagnostics.iter().any(|diag| {
                    diag.message.contains("output-crate") || diag.message.contains("generated.md")
                }),
                "task outputs may be created and must not be reported: {diagnostics:?}"
            );
        }

        #[test]
        fn plan_validation_existing_prerequisites_pass() {
            let temp = TempDir::new().unwrap();
            fs::create_dir_all(temp.path().join("crates/roko-core/src")).unwrap();
            fs::write(
                temp.path().join("crates/roko-core/src/lib.rs"),
                "// existing crate file\n",
            )
            .unwrap();
            fs::create_dir_all(temp.path().join("packages/existing-app")).unwrap();
            fs::write(
                temp.path().join("packages/existing-app/package.json"),
                "{\n  \"name\": \"existing-app\"\n}\n",
            )
            .unwrap();
            fs::create_dir_all(temp.path().join("docs")).unwrap();
            fs::write(temp.path().join("docs/existing.md"), "# exists\n").unwrap();

            let tasks_path = write_tasks_file(
                temp.path(),
                "refs-ok",
                &task_toml(
                    "refs-ok",
                    "Validate existing refs",
                    Some("implementer"),
                    r#"files = ["docs/generated.md"]

[task.context]
read_files = [
  { path = "crates/roko-core/src/lib.rs", why = "existing crate input" },
  { path = "packages/existing-app/package.json", why = "existing package input" },
  { path = "docs/existing.md", why = "existing doc input" },
]
"#,
                ),
            );

            let diagnostics = validate_file_references(&tasks_path, temp.path()).unwrap();
            assert!(
                diagnostics.is_empty(),
                "existing references should not be reported: {diagnostics:?}"
            );
        }

        #[test]
        fn plan_validation_dependency_created_prerequisite_passes() {
            let temp = TempDir::new().unwrap();
            let tasks_path = write_tasks_file(
                temp.path(),
                "dependency-output",
                r#"
[meta]
plan = "dependency-output"

[[task]]
id = "T1"
title = "Create input"
role = "implementer"
files = ["generated/input.md"]
depends_on = []
verify = [{ phase = "structural", command = "test -f generated/input.md" }]

[[task]]
id = "T2"
title = "Consume input"
role = "implementer"
files = ["src/lib.rs"]
depends_on = ["T1"]
verify = [{ phase = "compile", command = "echo ok" }]

[task.context]
read_files = [{ path = "generated/input.md", why = "created by T1" }]
"#,
            );

            let diagnostics = validate_file_references(&tasks_path, temp.path()).unwrap();
            assert!(
                diagnostics.is_empty(),
                "declared dependency output should satisfy prerequisite: {diagnostics:?}"
            );
        }

        #[test]
        fn plan_validation_loaded_plan_dependency_output_passes_but_unrelated_output_fails() {
            let temp = TempDir::new().unwrap();
            write_tasks_file(
                temp.path(),
                "foundation",
                r#"
[meta]
plan = "foundation"

[[task]]
id = "F1"
title = "Create shared input"
role = "implementer"
files = ["generated/shared.md"]
depends_on = []
verify = [{ phase = "structural", command = "test -f generated/shared.md" }]
"#,
            );
            write_tasks_file(
                temp.path(),
                "consumer",
                r#"
[meta]
plan = "consumer"

[[task]]
id = "T1"
title = "Consume declared plan output"
role = "implementer"
files = ["src/declared.rs"]
depends_on = []
depends_on_plan = ["foundation"]
verify = [{ phase = "compile", command = "echo ok" }]

[task.context]
read_files = [{ path = "generated/shared.md", why = "foundation output" }]

[[task]]
id = "T2"
title = "Consume unrelated plan output"
role = "implementer"
files = ["src/undeclared.rs"]
depends_on = []
verify = [{ phase = "compile", command = "echo ok" }]

[task.context]
read_files = [{ path = "generated/shared.md", why = "producer not declared" }]
"#,
            );

            let report = validate_plans_dir_with_workdir(
                &temp.path().join("plans"),
                None,
                Some(temp.path()),
            )
            .unwrap();
            assert_eq!(report.totals.warnings, 1, "unexpected report: {report:?}");
            let diagnostics = report
                .plans
                .iter()
                .flat_map(|plan| &plan.diagnostics)
                .collect::<Vec<_>>();
            assert_eq!(diagnostics[0].task_id.as_deref(), Some("T2"));
            assert!(
                diagnostics[0]
                    .message
                    .contains("not created by a declared dependency")
            );
        }

        #[test]
        fn plan_validation_duplicate_crate_proposal_errors() {
            let tasks_toml = task_toml(
                "greenfield-dup",
                "Ground repository",
                Some("architect"),
                "description = \"Create crate roko-compose.\"\n",
            );
            let diagnostics = greenfield_diagnostics(&tasks_toml, &["roko-compose"]);

            assert_eq!(
                diagnostics.len(),
                1,
                "unexpected diagnostics: {diagnostics:?}"
            );
            assert!(
                diagnostics
                    .iter()
                    .any(|diag| diag.severity == Severity::Error && diag.rule_id == "PLAN_032"),
                "duplicate crate proposal should be rejected: {diagnostics:?}"
            );
            assert!(
                diagnostics
                    .iter()
                    .any(|diag| diag.message.contains("roko-compose")),
                "duplicate crate proposal should name the existing crate: {diagnostics:?}"
            );
        }

        #[test]
        fn plan_validation_greenfield_phrase_in_existing_workspace_errors() {
            let tasks_toml = task_toml(
                "greenfield-phrase",
                "Ground repository",
                Some("architect"),
                "description = \"This is a greenfield project.\"\n",
            );
            let diagnostics = greenfield_diagnostics(&tasks_toml, &["roko-core"]);

            assert_eq!(
                diagnostics.len(),
                1,
                "unexpected diagnostics: {diagnostics:?}"
            );
            assert!(
                diagnostics
                    .iter()
                    .any(|diag| diag.severity == Severity::Error && diag.rule_id == "PLAN_033"),
                "greenfield claim should be rejected in an existing workspace: {diagnostics:?}"
            );
        }

        #[test]
        fn plan_validation_empty_workspace_skips_greenfield_checks() {
            let tasks_toml = task_toml(
                "greenfield-empty",
                "Ground repository",
                Some("architect"),
                "description = \"This is a greenfield project.\"\n",
            );
            let diagnostics = greenfield_diagnostics(&tasks_toml, &[]);

            assert!(
                diagnostics.is_empty(),
                "empty workspaces should not flag greenfield claims: {diagnostics:?}"
            );
        }

        #[test]
        fn plan_validation_legitimate_new_crate_in_existing_workspace_passes() {
            let tasks_toml = task_toml(
                "greenfield-new",
                "Ground repository",
                Some("architect"),
                "description = \"Create crate my-new-feature.\"\n",
            );
            let diagnostics = greenfield_diagnostics(&tasks_toml, &["roko-core"]);

            assert!(
                diagnostics.is_empty(),
                "genuinely new crates should not be rejected: {diagnostics:?}"
            );
        }
    }
}
