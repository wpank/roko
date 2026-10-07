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
    /// A rendered evaluation cannot fail: it has no `#[test]` function, or one
    /// whose body is empty, comments-only, tautological or a placeholder macro.
    VacuousTest {
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
            Self::VacuousTest {
                template_name,
                reason,
            } => {
                write!(
                    f,
                    "template '{template_name}' renders a test that cannot fail: {reason}"
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
        self.gate_types()
            .iter()
            .flat_map(|gate_type| self.generate(task_title, gate_type, crate_name, files))
            .collect()
    }

    /// [`Self::generate_checked`] for every gate type the templates cover: the
    /// evaluations that pass validation, and the error of each gate type
    /// whose templates did not.
    #[must_use]
    pub fn generate_checked_all(
        &self,
        task_title: &str,
        crate_name: &str,
        files: &[String],
        property_body: Option<&str>,
    ) -> (Vec<Evaluation>, Vec<EvalGenerationError>) {
        let mut evals = Vec::new();
        let mut rejected = Vec::new();
        for gate_type in self.gate_types() {
            let request = EvalGenerationRequest {
                task_title: task_title.to_string(),
                gate_type,
                crate_name: crate_name.to_string(),
                files: files.to_vec(),
                property_body: property_body.map(str::to_string),
            };
            match self.generate_checked(&request) {
                Ok(generated) => evals.extend(generated),
                Err(error) => rejected.push(error),
            }
        }
        (evals, rejected)
    }

    /// The distinct gate types the templates target, sorted.
    fn gate_types(&self) -> Vec<String> {
        self.templates
            .iter()
            .map(|t| t.gate_type.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Generate evaluations with full validation. `PropertyBased` templates
    /// require a non-empty, non-vacuous `property_body` in the request, and
    /// every rendered evaluation must hold a `#[test]` function that can
    /// fail.
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
                if let Some(reason) = detect_vacuous_test(&test_source) {
                    return Err(EvalGenerationError::VacuousTest {
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
                // Non-property templates need no body, but must still render
                // a test that can fail.
                let test_source = template
                    .template_body
                    .replace("{task_title}", &request.task_title)
                    .replace("{crate_name}", &request.crate_name)
                    .replace("{files}", &files_str);
                if let Some(reason) = detect_vacuous_test(&test_source) {
                    return Err(EvalGenerationError::VacuousTest {
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

/// Detect a rendered evaluation that cannot fail: one with no `#[test]`
/// function, or with a `#[test]` function whose body is vacuous (see
/// [`detect_vacuous_body`]).
fn detect_vacuous_test(source: &str) -> Option<String> {
    let bodies = test_fn_bodies(source);
    if bodies.is_empty() {
        return Some("rendered source has no #[test] function".into());
    }
    bodies
        .iter()
        .map(String::as_str)
        .find_map(detect_vacuous_body)
        .map(|reason| format!("a #[test] function's {reason}"))
}

/// The bodies of the `#[test]` functions in `source`, comments stripped.
fn test_fn_bodies(source: &str) -> Vec<String> {
    let stripped = strip_comments(source);
    let mut bodies = Vec::new();
    let mut rest = stripped.as_str();
    while let Some(at) = rest.find("#[test]") {
        rest = &rest[at + "#[test]".len()..];
        let Some(open) = rest.find('{') else {
            break;
        };
        let after = &rest[open + 1..];
        let mut depth = 1_usize;
        let close = after.char_indices().find_map(|(index, c)| {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            (depth == 0).then_some(index)
        });
        let Some(close) = close else {
            break;
        };
        bodies.push(after[..close].to_string());
        rest = &after[close + 1..];
    }
    bodies
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

/// Built-in evaluation templates.
///
/// Only the property template: it renders the caller's validated assertion
/// body. Compile, clippy and test-suite checks belong to their gates; the
/// empty-bodied tests that once stood in for them proved nothing
/// (bug-017c2d).
fn builtin_templates() -> Vec<EvalTemplate> {
    vec![EvalTemplate {
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
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A non-property template whose test asserts something.
    fn asserting_template(name: &str, gate_type: &str) -> EvalTemplate {
        EvalTemplate {
            name: name.into(),
            gate_type: gate_type.into(),
            strategy: EvalStrategy::ExampleBased,
            expected_behavior: "The task's file exists".into(),
            template_body: concat!(
                "// Generated eval: {task_title} ({crate_name})\n",
                "#[test]\n",
                "fn gen_file_exists() {\n",
                "    assert!(std::path::Path::new(\"{files}\").exists());\n",
                "}\n",
            )
            .into(),
        }
    }

    fn request(gate_type: &str, property_body: Option<&str>) -> EvalGenerationRequest {
        EvalGenerationRequest {
            task_title: "Task".into(),
            gate_type: gate_type.into(),
            crate_name: "roko-core".into(),
            files: vec![],
            property_body: property_body.map(str::to_string),
        }
    }

    #[test]
    fn default_generator_has_builtin_templates() {
        let generator = EvalGenerator::new();
        assert!(!generator.templates.is_empty());
    }

    #[test]
    fn generate_for_compile_gate() {
        let template = asserting_template("compile-check", "compile");
        let generator = EvalGenerator::with_templates(vec![template]);
        let evals = generator.generate("Add Demurrage trait", "compile", "roko-core", &[]);
        assert_eq!(evals.len(), 1);
        assert!(evals[0].name.starts_with("gen_compile_check"));
        assert!(evals[0].test_source.contains("Demurrage"));
        assert!(evals[0].expect_pre_failure);
    }

    #[test]
    fn generate_all_covers_multiple_gate_types() {
        let mut templates = builtin_templates();
        templates.push(asserting_template("compile-check", "compile"));
        templates.push(asserting_template("test-file", "test"));
        let generator = EvalGenerator::with_templates(templates);
        let evals =
            generator.generate_all("Wire foraging", "roko-compose", &["foraging.rs".into()]);
        // One per non-property template: the property template is excluded by
        // the compatibility filter.
        assert_eq!(evals.len(), 2, "got {} evals", evals.len());
        let gate_types: Vec<&str> = evals.iter().map(|e| e.gate_type.as_str()).collect();
        assert!(gate_types.contains(&"compile"));
        assert!(gate_types.contains(&"test"));
    }

    #[test]
    fn generate_skips_property_templates() {
        let generator = EvalGenerator::new();
        let evals = generator.generate("Task", "test", "roko-core", &[]);
        // The only built-in "test" template is property-based.
        assert!(evals.is_empty(), "{evals:?}");
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
        // The property-invariant template, the only built-in "test" one.
        assert_eq!(evals.len(), 1);
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
        // A non-property template needs no property_body.
        let template = asserting_template("compile-check", "compile");
        let generator = EvalGenerator::with_templates(vec![template]);
        let evals = generator
            .generate_checked(&request("compile", None))
            .unwrap();
        assert_eq!(evals.len(), 1);
        assert_eq!(evals[0].strategy, EvalStrategy::ExampleBased);
    }

    /// bug-017c2d: no built-in template renders a test without a body, so
    /// generation without a property body writes nothing.
    #[test]
    fn builtin_templates_render_no_placeholder_tests() {
        let (evals, rejected) =
            EvalGenerator::new().generate_checked_all("Task", "roko-core", &[], None);
        assert!(evals.is_empty(), "{evals:?}");
        assert!(
            matches!(
                rejected.as_slice(),
                [EvalGenerationError::MissingPropertyBody { .. }]
            ),
            "{rejected:?}"
        );
        for template in builtin_templates() {
            assert!(
                template.template_body.contains("{property_body}"),
                "{}",
                template.name
            );
        }
    }

    /// bug-017c2d: a checked evaluation must hold a `#[test]` that can fail.
    #[test]
    fn generate_checked_rejects_a_test_that_cannot_fail() {
        let empty = EvalTemplate {
            name: "empty".into(),
            gate_type: "compile".into(),
            strategy: EvalStrategy::ExampleBased,
            expected_behavior: "Nothing".into(),
            template_body: "#[test]\nfn gen_empty() {\n    // verified elsewhere\n}\n".into(),
        };
        let no_test = EvalTemplate {
            name: "no-test".into(),
            template_body: "// Generated eval: {task_title}\n".into(),
            ..empty.clone()
        };
        for template in [empty, no_test] {
            let generator = EvalGenerator::with_templates(vec![template]);
            let err = generator
                .generate_checked(&request("compile", None))
                .unwrap_err();
            assert!(
                matches!(err, EvalGenerationError::VacuousTest { .. }),
                "expected VacuousTest, got: {err:?}"
            );
        }
    }

    #[test]
    fn generate_checked_all_returns_valid_evals_and_rejections() {
        let mut templates = builtin_templates();
        templates.push(asserting_template("compile-check", "compile"));
        let generator = EvalGenerator::with_templates(templates);

        let (evals, rejected) =
            generator.generate_checked_all("Task", "roko-core", &["lib.rs".into()], None);
        assert_eq!(evals.len(), 1);
        assert_eq!(evals[0].gate_type, "compile");
        assert_eq!(rejected.len(), 1, "the property template has no body");

        let body = Some("assert!(value >= 0);");
        let (evals, rejected) = generator.generate_checked_all("Task", "roko-core", &[], body);
        assert_eq!(evals.len(), 2);
        assert!(rejected.is_empty(), "{rejected:?}");
    }

    #[test]
    fn test_fn_bodies_skips_comments_and_keeps_nested_braces() {
        let src = concat!(
            "#[test]\nfn a() {\n    if x { y(); }\n}\n",
            "// #[test] fn c() {}\n",
            "#[test]\nfn b() {}\n",
        );
        let bodies = test_fn_bodies(src);
        assert_eq!(bodies.len(), 2, "{bodies:?}");
        assert!(bodies[0].contains("if x { y(); }"));
        assert!(bodies[1].trim().is_empty());
        assert!(detect_vacuous_test(src).is_some());
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

        let cannot_fail = EvalGenerationError::VacuousTest {
            template_name: "empty".into(),
            reason: "no #[test]".into(),
        };
        assert!(cannot_fail.to_string().contains("cannot fail"));
        assert!(cannot_fail.to_string().contains("empty"));
    }
}
