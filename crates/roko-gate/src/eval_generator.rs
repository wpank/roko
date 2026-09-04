//! Autonomous evaluation generation pipeline (doc 10 -- Autonomous Eval Generation).
//!
//! Before an implementation agent starts, this module generates targeted test
//! cases from task specs. Three strategies are supported:
//!
//! - **Example-based**: concrete input/output pairs
//! - **Property-based**: invariants (proptest-style)
//! - **Mutation-based**: mutant detection
//!
//! Generated evaluations are validated against the current codebase and
//! registered with the `GeneratedTestGate` artifact store.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Strategy for generating evaluation test cases.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EvalStrategy {
    /// Concrete input/output pairs derived from the task spec.
    ExampleBased,
    /// Invariant assertions (proptest-style properties).
    PropertyBased,
    /// Mutation-based: ensure the implementation detects seeded faults.
    MutationBased,
}

/// Error returned when eval generation fails validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalGenerationError {
    /// A `PropertyBased` template requires a non-empty, non-vacuous `property_body`.
    MissingPropertyBody {
        /// The template name that failed.
        template_name: String,
    },
    /// The supplied property body is vacuous (comments-only, `assert!(true)`,
    /// `todo!()`, `unimplemented!()`).
    VacuousPropertyBody {
        /// The template name that failed.
        template_name: String,
        /// Human-readable explanation of the rejection.
        reason: String,
    },
}

impl fmt::Display for EvalGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPropertyBody { template_name } => {
                write!(
                    f,
                    "property template '{template_name}' requires a non-empty property_body"
                )
            }
            Self::VacuousPropertyBody {
                template_name,
                reason,
            } => {
                write!(
                    f,
                    "property template '{template_name}' has vacuous body: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for EvalGenerationError {}

/// Structured request for checked eval generation.
#[derive(Clone, Debug)]
pub struct EvalGenerationRequest {
    /// The task being evaluated.
    pub task_title: String,
    /// The gate type to generate evaluations for (e.g. `"compile"`, `"test"`).
    pub gate_type: String,
    /// The crate under test.
    pub crate_name: String,
    /// Relevant source files.
    pub files: Vec<String>,
    /// For `PropertyBased` templates, the executable assertion body to
    /// substitute into the `{property_body}` placeholder. Must be non-empty
    /// and non-vacuous for property templates.
    pub property_body: Option<String>,
}

/// A single evaluation template that can generate test cases for a gate type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvalTemplate {
    /// Human-readable template name (e.g. "compile-gate-basic").
    pub name: String,
    /// Verify type this template targets (e.g. "compile", "test", "clippy").
    pub gate_type: String,
    /// Strategy used for test generation.
    pub strategy: EvalStrategy,
    /// Description of expected behavior to validate.
    pub expected_behavior: String,
    /// Template body with placeholders for task-specific values.
    /// Placeholders: `{task_title}`, `{crate_name}`, `{files}`.
    pub template_body: String,
}

/// A generated evaluation case ready for registration with the artifact store.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Evaluation {
    /// Evaluation name derived from template + task.
    pub name: String,
    /// Verify type being evaluated.
    pub gate_type: String,
    /// Strategy used.
    pub strategy: EvalStrategy,
    /// Generated test source code.
    pub test_source: String,
    /// Whether this test is expected to fail before implementation (new feature test).
    pub expect_pre_failure: bool,
}

/// Generator that produces evaluation cases from task descriptions.
#[derive(Clone, Debug)]
pub struct EvalGenerator {
    /// Available templates for generating evaluations.
    pub templates: Vec<EvalTemplate>,
}

impl Default for EvalGenerator {
    fn default() -> Self {
        Self {
            templates: builtin_templates(),
        }
    }
}

impl EvalGenerator {
    /// Create a generator with the builtin template set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a generator with custom templates.
    #[must_use]
    pub fn with_templates(templates: Vec<EvalTemplate>) -> Self {
        Self { templates }
    }

    /// Generate evaluations for a task targeting a specific gate type.
    ///
    /// Returns all evaluations matching the gate type, with placeholders
    /// filled from `task_title`, `crate_name`, and `files`.
    ///
    /// **Compatibility note:** This method skips `PropertyBased` templates
    /// entirely -- they must go through [`generate_checked`] with a validated
    /// `property_body`. Non-property templates are unaffected.
    #[must_use]
    pub fn generate(
        &self,
        task_title: &str,
        gate_type: &str,
        crate_name: &str,
        files: &[String],
    ) -> Vec<Evaluation> {
        self.templates
            .iter()
            .filter(|template| template.gate_type == gate_type)
            // Compatibility: skip property templates from the old path.
            .filter(|template| template.strategy != EvalStrategy::PropertyBased)
            .map(|template| {
                let files_str = files.join(", ");
                let test_source = template
                    .template_body
                    .replace("{task_title}", task_title)
                    .replace("{crate_name}", crate_name)
                    .replace("{files}", &files_str);

                Evaluation {
                    name: format!(
                        "gen_{}_{}",
                        template.name.replace('-', "_"),
                        sanitize(task_title)
                    ),
                    gate_type: template.gate_type.clone(),
                    strategy: template.strategy.clone(),
                    test_source,
                    expect_pre_failure: true,
                }
            })
            .collect()
    }

    /// Generate evaluations for all gate types relevant to a task.
    ///
    /// **Compatibility note:** `PropertyBased` templates are skipped. Use
    /// [`generate_checked`] for property generation.
    #[must_use]
    pub fn generate_all(
        &self,
        task_title: &str,
        crate_name: &str,
        files: &[String],
    ) -> Vec<Evaluation> {
        let gate_types: Vec<String> = self
            .templates
            .iter()
            .map(|t| t.gate_type.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();

        gate_types
            .iter()
            .flat_map(|gate_type| self.generate(task_title, gate_type, crate_name, files))
            .collect()
    }

    /// Generate evaluations with full validation. `PropertyBased` templates
    /// require a non-empty, non-vacuous `property_body` in the request.
    ///
    /// Returns an error on the first template that fails validation rather
    /// than silently emitting a vacuous test.
    pub fn generate_checked(
        &self,
        request: &EvalGenerationRequest,
    ) -> Result<Vec<Evaluation>, EvalGenerationError> {
        let mut evals = Vec::new();
        let files_str = request.files.join(", ");

        for template in &self.templates {
            if template.gate_type != request.gate_type {
                continue;
            }

            if template.strategy == EvalStrategy::PropertyBased {
                // Property templates require a validated body.
                let body = match &request.property_body {
                    Some(b) if !b.trim().is_empty() => b,
                    _ => {
                        return Err(EvalGenerationError::MissingPropertyBody {
                            template_name: template.name.clone(),
                        });
                    }
                };

                // Validate the body is not vacuous.
                if let Some(reason) = detect_vacuous_body(body) {
                    return Err(EvalGenerationError::VacuousPropertyBody {
                        template_name: template.name.clone(),
                        reason,
                    });
                }

                let test_source = template
                    .template_body
                    .replace("{task_title}", &request.task_title)
                    .replace("{crate_name}", &request.crate_name)
                    .replace("{files}", &files_str)
                    .replace("{property_body}", body);

                // Validate the fully rendered source for vacuity as well.
                if let Some(reason) = detect_vacuous_rendered(&test_source) {
                    return Err(EvalGenerationError::VacuousPropertyBody {
                        template_name: template.name.clone(),
                        reason,
                    });
                }

                evals.push(Evaluation {
                    name: format!(
                        "gen_{}_{}",
                        template.name.replace('-', "_"),
                        sanitize(&request.task_title)
                    ),
                    gate_type: template.gate_type.clone(),
                    strategy: template.strategy.clone(),
                    test_source,
                    expect_pre_failure: true,
                });
            } else {
                // Non-property templates pass through unchanged.
                let test_source = template
                    .template_body
                    .replace("{task_title}", &request.task_title)
                    .replace("{crate_name}", &request.crate_name)
                    .replace("{files}", &files_str);

                evals.push(Evaluation {
                    name: format!(
                        "gen_{}_{}",
                        template.name.replace('-', "_"),
                        sanitize(&request.task_title)
                    ),
                    gate_type: template.gate_type.clone(),
                    strategy: template.strategy.clone(),
                    test_source,
                    expect_pre_failure: true,
                });
            }
        }

        Ok(evals)
    }
}

// ─── Vacuity detection ──────────────────────────────────────────────────────

/// Strip line/block comments and check if anything executable remains.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut in_block = false;
    let mut chars = src.chars().peekable();

    while let Some(c) = chars.next() {
        if in_block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
            }
            continue;
        }
        if c == '/' {
            match chars.peek() {
                Some('/') => {
                    // Line comment -- skip to end of line.
                    for c2 in chars.by_ref() {
                        if c2 == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    in_block = true;
                    continue;
                }
                _ => {}
            }
        }
        out.push(c);
    }
    out
}

/// Detect a vacuous property body (before template substitution).
///
/// Returns `Some(reason)` if the body is vacuous, `None` if acceptable.
fn detect_vacuous_body(body: &str) -> Option<String> {
    let stripped = strip_comments(body);
    let trimmed = stripped.trim();

    if trimmed.is_empty() {
        return Some("body is empty or comments-only".into());
    }

    // Normalise whitespace for pattern matching.
    let normalised: String = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");

    // Tautological assertions.
    if normalised.contains("assert!(true)")
        || normalised.contains("assert_eq!(true, true)")
        || normalised.contains("assert_eq!(1, 1)")
    {
        return Some("body contains only tautological assertions".into());
    }

    // Placeholder macros.
    if normalised == "todo!()" || normalised == "todo! ()" || normalised.starts_with("todo!(\"") {
        return Some("body is a todo!() placeholder".into());
    }
    if normalised == "unimplemented!()"
        || normalised == "unimplemented! ()"
        || normalised.starts_with("unimplemented!(\"")
    {
        return Some("body is an unimplemented!() placeholder".into());
    }

    None
}

/// Detect vacuity in a fully rendered test source.
///
/// This catches cases where the body was fine in isolation but the final
/// rendered source has no assertions beyond boilerplate.
fn detect_vacuous_rendered(source: &str) -> Option<String> {
    let stripped = strip_comments(source);

    // Check if any fn body contains only vacuous content.
    // Look for test function bodies that contain only comments/whitespace
    // after removing boilerplate.
    if stripped.contains("todo!()") || stripped.contains("todo! ()") {
        return Some("rendered source contains todo!()".into());
    }
    if stripped.contains("unimplemented!()") || stripped.contains("unimplemented! ()") {
        return Some("rendered source contains unimplemented!()".into());
    }

    None
}

/// Sanitize a task title into a valid Rust identifier fragment.
fn sanitize(title: &str) -> String {
    title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

/// Built-in evaluation templates for the standard gate types.
fn builtin_templates() -> Vec<EvalTemplate> {
    vec![
        EvalTemplate {
            name: "compile-check".into(),
            gate_type: "compile".into(),
            strategy: EvalStrategy::ExampleBased,
            expected_behavior: "Code compiles with no errors after implementation".into(),
            template_body: concat!(
                "// Generated eval: {task_title}\n",
                "// Verify: `cargo check -p {crate_name}` succeeds\n",
                "// Files: {files}\n",
                "#[test]\n",
                "fn gen_compiles() {\n",
                "    // This test validates that the crate compiles.\n",
                "    // The compile gate itself handles verification;\n",
                "    // this is a placeholder for the artifact store.\n",
                "}\n",
            )
            .into(),
        },
        EvalTemplate {
            name: "clippy-clean".into(),
            gate_type: "clippy".into(),
            strategy: EvalStrategy::ExampleBased,
            expected_behavior: "No new clippy warnings introduced".into(),
            template_body: concat!(
                "// Generated eval: {task_title}\n",
                "// Verify: `cargo clippy -p {crate_name}` produces no warnings\n",
                "// Files: {files}\n",
                "#[test]\n",
                "fn gen_clippy_clean() {\n",
                "    // Clippy cleanliness verified by ClippyGate.\n",
                "}\n",
            )
            .into(),
        },
        EvalTemplate {
            name: "test-pass".into(),
            gate_type: "test".into(),
            strategy: EvalStrategy::ExampleBased,
            expected_behavior: "All existing tests continue to pass".into(),
            template_body: concat!(
                "// Generated eval: {task_title}\n",
                "// Verify: `cargo test -p {crate_name}` passes\n",
                "// Files: {files}\n",
                "#[test]\n",
                "fn gen_tests_pass() {\n",
                "    // Test suite integrity verified by TestGate.\n",
                "}\n",
            )
            .into(),
        },
        EvalTemplate {
            name: "property-invariant".into(),
            gate_type: "test".into(),
            strategy: EvalStrategy::PropertyBased,
            expected_behavior: "Implementation satisfies domain invariants".into(),
            template_body: concat!(
                "// Generated property eval: {task_title}\n",
                "// Crate: {crate_name}, Files: {files}\n",
                "// Strategy: property-based (verify invariants hold)\n",
                "#[test]\n",
                "fn gen_property_invariant() {\n",
                "    {property_body}\n",
                "}\n",
            )
            .into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_generator_has_builtin_templates() {
        let generator = EvalGenerator::new();
        assert!(!generator.templates.is_empty());
    }

    #[test]
    fn generate_for_compile_gate() {
        let generator = EvalGenerator::new();
        let evals = generator.generate("Add Demurrage trait", "compile", "roko-core", &[]);
        assert_eq!(evals.len(), 1);
        assert!(evals[0].name.starts_with("gen_compile_check"));
        assert!(evals[0].test_source.contains("Demurrage"));
        assert!(evals[0].expect_pre_failure);
    }

    #[test]
    fn generate_all_covers_multiple_gate_types() {
        let generator = EvalGenerator::new();
        let evals =
            generator.generate_all("Wire foraging", "roko-compose", &["foraging.rs".into()]);
        // Should have compile (1) + clippy (1) + test (1, non-property) = 3.
        // The property template is excluded by the compatibility filter.
        assert!(evals.len() >= 3, "got {} evals", evals.len());
        let gate_types: Vec<&str> = evals.iter().map(|e| e.gate_type.as_str()).collect();
        assert!(gate_types.contains(&"compile"));
        assert!(gate_types.contains(&"test"));
    }

    #[test]
    fn generate_skips_property_templates() {
        let generator = EvalGenerator::new();
        let evals = generator.generate("Task", "test", "roko-core", &[]);
        // Only the ExampleBased "test-pass" template, not the PropertyBased one.
        assert_eq!(evals.len(), 1);
        assert_eq!(evals[0].strategy, EvalStrategy::ExampleBased);
    }

    #[test]
    fn custom_templates() {
        let generator = EvalGenerator::with_templates(vec![EvalTemplate {
            name: "custom".into(),
            gate_type: "security".into(),
            strategy: EvalStrategy::MutationBased,
            expected_behavior: "No SQL injection".into(),
            template_body: "// {task_title} in {crate_name}".into(),
        }]);
        let evals = generator.generate("Sanitize input", "security", "my-crate", &[]);
        assert_eq!(evals.len(), 1);
        assert!(evals[0].test_source.contains("Sanitize input"));
    }

    #[test]
    fn sanitize_title() {
        assert_eq!(sanitize("Add Demurrage trait"), "add_demurrage_trait");
        assert_eq!(sanitize("fix: bug #123"), "fix__bug__123");
    }

    // ─── generate_checked tests ─────────────────────────────────────

    #[test]
    fn generate_checked_with_valid_property_body() {
        let generator = EvalGenerator::new();
        let request = EvalGenerationRequest {
            task_title: "Add bounds check".into(),
            gate_type: "test".into(),
            crate_name: "roko-core".into(),
            files: vec!["bounds.rs".into()],
            property_body: Some("assert!(value >= 0 && value < max);".into()),
        };
        let evals = generator.generate_checked(&request).unwrap();
        // Should have both the ExampleBased test-pass and the PropertyBased
        // property-invariant templates.
        assert_eq!(evals.len(), 2);
        let property = evals
            .iter()
            .find(|e| e.strategy == EvalStrategy::PropertyBased)
            .expect("property eval");
        assert!(
            property.test_source.contains("assert!(value >= 0"),
            "body should be substituted into source"
        );
        assert!(
            !property.test_source.contains("{property_body}"),
            "placeholder must be replaced"
        );
    }

    #[test]
    fn generate_checked_missing_property_body_errors() {
        let generator = EvalGenerator::new();
        let request = EvalGenerationRequest {
            task_title: "Task".into(),
            gate_type: "test".into(),
            crate_name: "roko-core".into(),
            files: vec![],
            property_body: None,
        };
        let err = generator.generate_checked(&request).unwrap_err();
        assert!(
            matches!(err, EvalGenerationError::MissingPropertyBody { .. }),
            "expected MissingPropertyBody, got: {err:?}"
        );
    }

    #[test]
    fn generate_checked_empty_body_errors() {
        let generator = EvalGenerator::new();
        let request = EvalGenerationRequest {
            task_title: "Task".into(),
            gate_type: "test".into(),
            crate_name: "roko-core".into(),
            files: vec![],
            property_body: Some("   ".into()),
        };
        let err = generator.generate_checked(&request).unwrap_err();
        assert!(matches!(
            err,
            EvalGenerationError::MissingPropertyBody { .. }
        ));
    }

    #[test]
    fn generate_checked_comments_only_body_errors() {
        let generator = EvalGenerator::new();
        let request = EvalGenerationRequest {
            task_title: "Task".into(),
            gate_type: "test".into(),
            crate_name: "roko-core".into(),
            files: vec![],
            property_body: Some("// just a comment\n/* another */".into()),
        };
        let err = generator.generate_checked(&request).unwrap_err();
        assert!(
            matches!(err, EvalGenerationError::VacuousPropertyBody { .. }),
            "expected VacuousPropertyBody, got: {err:?}"
        );
    }

    #[test]
    fn generate_checked_assert_true_errors() {
        let generator = EvalGenerator::new();
        let request = EvalGenerationRequest {
            task_title: "Task".into(),
            gate_type: "test".into(),
            crate_name: "roko-core".into(),
            files: vec![],
            property_body: Some("assert!(true)".into()),
        };
        let err = generator.generate_checked(&request).unwrap_err();
        assert!(
            matches!(err, EvalGenerationError::VacuousPropertyBody { .. }),
            "expected VacuousPropertyBody, got: {err:?}"
        );
    }

    #[test]
    fn generate_checked_todo_body_errors() {
        let generator = EvalGenerator::new();
        let request = EvalGenerationRequest {
            task_title: "Task".into(),
            gate_type: "test".into(),
            crate_name: "roko-core".into(),
            files: vec![],
            property_body: Some("todo!()".into()),
        };
        let err = generator.generate_checked(&request).unwrap_err();
        assert!(matches!(
            err,
            EvalGenerationError::VacuousPropertyBody { .. }
        ));
    }

    #[test]
    fn generate_checked_unimplemented_body_errors() {
        let generator = EvalGenerator::new();
        let request = EvalGenerationRequest {
            task_title: "Task".into(),
            gate_type: "test".into(),
            crate_name: "roko-core".into(),
            files: vec![],
            property_body: Some("unimplemented!()".into()),
        };
        let err = generator.generate_checked(&request).unwrap_err();
        assert!(matches!(
            err,
            EvalGenerationError::VacuousPropertyBody { .. }
        ));
    }

    #[test]
    fn generate_checked_non_property_gate_ignores_body() {
        // When targeting compile gate, the property_body is not needed.
        let generator = EvalGenerator::new();
        let request = EvalGenerationRequest {
            task_title: "Task".into(),
            gate_type: "compile".into(),
            crate_name: "roko-core".into(),
            files: vec![],
            property_body: None,
        };
        let evals = generator.generate_checked(&request).unwrap();
        assert_eq!(evals.len(), 1);
        assert_eq!(evals[0].strategy, EvalStrategy::ExampleBased);
    }

    // ─── vacuity detection unit tests ───────────────────────────────

    #[test]
    fn detect_vacuous_empty() {
        assert!(detect_vacuous_body("").is_some());
        assert!(detect_vacuous_body("   ").is_some());
    }

    #[test]
    fn detect_vacuous_comments_only() {
        assert!(detect_vacuous_body("// line comment").is_some());
        assert!(detect_vacuous_body("/* block */").is_some());
        assert!(detect_vacuous_body("// line\n/* block */\n// more").is_some());
    }

    #[test]
    fn detect_vacuous_assert_true() {
        assert!(detect_vacuous_body("assert!(true)").is_some());
        assert!(detect_vacuous_body("  assert!(true)  ").is_some());
    }

    #[test]
    fn detect_vacuous_todo() {
        assert!(detect_vacuous_body("todo!()").is_some());
        assert!(detect_vacuous_body("todo!(\"later\")").is_some());
    }

    #[test]
    fn detect_vacuous_unimplemented() {
        assert!(detect_vacuous_body("unimplemented!()").is_some());
    }

    #[test]
    fn detect_vacuous_real_assertion_passes() {
        assert!(detect_vacuous_body("assert_eq!(result, 42);").is_none());
        assert!(detect_vacuous_body("let x = compute(); assert!(x > 0);").is_none());
    }

    #[test]
    fn strip_comments_removes_line_and_block() {
        let src = "code // comment\n/* block */ more";
        let stripped = strip_comments(src);
        assert!(!stripped.contains("comment"));
        assert!(!stripped.contains("block"));
        assert!(stripped.contains("code"));
        assert!(stripped.contains("more"));
    }

    // ─── property template no longer has TODO ───────────────────────

    #[test]
    fn builtin_property_template_has_placeholder_not_todo() {
        let templates = builtin_templates();
        let prop = templates
            .iter()
            .find(|t| t.strategy == EvalStrategy::PropertyBased)
            .expect("property template");
        assert!(
            !prop.template_body.contains("TODO"),
            "property template must not contain TODO placeholder"
        );
        assert!(
            prop.template_body.contains("{property_body}"),
            "property template must contain {{property_body}} placeholder"
        );
    }

    // ─── EvalGenerationError Display ────────────────────────────────

    #[test]
    fn eval_generation_error_display() {
        let missing = EvalGenerationError::MissingPropertyBody {
            template_name: "test".into(),
        };
        assert!(missing.to_string().contains("test"));
        assert!(missing.to_string().contains("non-empty"));

        let vacuous = EvalGenerationError::VacuousPropertyBody {
            template_name: "prop".into(),
            reason: "todo!()".into(),
        };
        assert!(vacuous.to_string().contains("vacuous"));
        assert!(vacuous.to_string().contains("todo!()"));
    }
}
