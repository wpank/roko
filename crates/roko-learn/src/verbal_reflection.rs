//! P4-01: LLM-generated verbal self-reflection after gate failure.
//!
//! When a gate pipeline fails, produces a structured reflection prompt
//! for the LLM to explain what went wrong and what it would do differently.
//! These reflections feed back into playbook extraction and episode metadata.

use serde::{Deserialize, Serialize};

/// Input to the reflection prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionInput {
    /// Task that failed.
    pub task_id: String,
    /// Which gate rungs failed.
    pub failed_gates: Vec<String>,
    /// Error signatures from the failed gates.
    pub error_signatures: Vec<String>,
    /// Code changes that were attempted.
    pub diff_summary: String,
    /// Model that was used.
    pub model: String,
    /// Attempt number (1-based).
    pub attempt: u32,
}

/// Structured reflection output from the LLM.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Reflection {
    /// What the agent thinks went wrong.
    pub root_cause: String,
    /// What the agent would do differently.
    pub alternative_approach: String,
    /// Specific patterns to avoid next time.
    pub avoid_patterns: Vec<String>,
    /// Confidence in the reflection (0.0 to 1.0).
    pub confidence: f64,
    /// Whether the agent thinks the task is achievable.
    pub task_achievable: bool,
}

/// Build a system prompt section for the reflection.
#[must_use]
pub fn build_reflection_prompt(input: &ReflectionInput) -> String {
    let mut prompt = String::new();
    prompt.push_str("## Self-Reflection on Gate Failure\n\n");
    prompt.push_str(&format!(
        "Task `{}` failed on attempt {} at gates: {}\n\n",
        input.task_id,
        input.attempt,
        input.failed_gates.join(", ")
    ));

    if !input.error_signatures.is_empty() {
        prompt.push_str("Error signatures:\n");
        for sig in &input.error_signatures {
            prompt.push_str(&format!("- {sig}\n"));
        }
        prompt.push('\n');
    }

    if !input.diff_summary.is_empty() {
        prompt.push_str("Changes attempted:\n");
        prompt.push_str(&input.diff_summary);
        prompt.push_str("\n\n");
    }

    prompt.push_str(
        "Analyze what went wrong. Respond with:\n\
         1. Root cause: What specifically caused the failure?\n\
         2. Alternative approach: What would you do differently?\n\
         3. Patterns to avoid: List specific anti-patterns to avoid.\n\
         4. Achievability: Is this task achievable with the current approach?\n",
    );

    prompt
}

/// Parse an LLM response into a structured `Reflection`.
///
/// Uses simple heuristic parsing of numbered sections.
#[must_use]
pub fn parse_reflection(response: &str) -> Reflection {
    let lines: Vec<&str> = response.lines().collect();
    let mut reflection = Reflection {
        confidence: 0.5,
        task_achievable: true,
        ..Default::default()
    };

    let mut current_section = 0u8;
    for line in &lines {
        let lower = line.to_lowercase();
        if lower.contains("root cause") || lower.starts_with("1.") {
            current_section = 1;
            let content = extract_after_marker(line);
            if !content.is_empty() {
                reflection.root_cause = content;
            }
        } else if lower.contains("alternative") || lower.starts_with("2.") {
            current_section = 2;
            let content = extract_after_marker(line);
            if !content.is_empty() {
                reflection.alternative_approach = content;
            }
        } else if lower.contains("pattern") || lower.contains("avoid") || lower.starts_with("3.") {
            current_section = 3;
            let content = extract_after_marker(line);
            if !content.is_empty() {
                reflection.avoid_patterns.push(content);
            }
        } else if lower.contains("achievab") || lower.starts_with("4.") {
            current_section = 4;
            if lower.contains("not achievable") || lower.contains("no") {
                reflection.task_achievable = false;
            }
        } else {
            match current_section {
                1 if !line.trim().is_empty() => {
                    if reflection.root_cause.is_empty() {
                        reflection.root_cause = line.trim().to_string();
                    } else {
                        reflection.root_cause.push(' ');
                        reflection.root_cause.push_str(line.trim());
                    }
                }
                2 if !line.trim().is_empty() => {
                    if reflection.alternative_approach.is_empty() {
                        reflection.alternative_approach = line.trim().to_string();
                    } else {
                        reflection.alternative_approach.push(' ');
                        reflection.alternative_approach.push_str(line.trim());
                    }
                }
                3 if line.trim().starts_with('-') || line.trim().starts_with('*') => {
                    let content = line.trim().trim_start_matches(['-', '*', ' ']);
                    if !content.is_empty() {
                        reflection.avoid_patterns.push(content.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    // Assess confidence based on completeness.
    let mut completeness: f64 = 0.0;
    if !reflection.root_cause.is_empty() {
        completeness += 0.3;
    }
    if !reflection.alternative_approach.is_empty() {
        completeness += 0.3;
    }
    if !reflection.avoid_patterns.is_empty() {
        completeness += 0.2;
    }
    completeness += 0.2; // Base confidence.
    reflection.confidence = completeness.clamp(0.0, 1.0);

    reflection
}

fn extract_after_marker(line: &str) -> String {
    // Strip leading number/marker and colon.
    let trimmed = line.trim();
    if let Some(pos) = trimmed.find(':') {
        trimmed[pos + 1..].trim().to_string()
    } else if let Some(pos) = trimmed.find(')') {
        trimmed[pos + 1..].trim().to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_reflection_prompt() {
        let input = ReflectionInput {
            task_id: "task-1".into(),
            failed_gates: vec!["compile".into(), "test".into()],
            error_signatures: vec!["E0308: type mismatch".into()],
            diff_summary: "Added new function foo()".into(),
            model: "claude-sonnet-4".into(),
            attempt: 2,
        };
        let prompt = build_reflection_prompt(&input);
        assert!(prompt.contains("task-1"));
        assert!(prompt.contains("compile, test"));
        assert!(prompt.contains("E0308"));
    }

    #[test]
    fn parses_structured_reflection() {
        let response = "\
1. Root cause: Missing type annotation on the return value.
2. Alternative approach: Use explicit return type annotations for all functions.
3. Patterns to avoid:
- Relying on type inference for complex functions
- Using impl Trait in argument position
4. Achievability: Yes, this is achievable.";

        let reflection = parse_reflection(response);
        assert!(!reflection.root_cause.is_empty());
        assert!(!reflection.alternative_approach.is_empty());
        assert!(!reflection.avoid_patterns.is_empty());
        assert!(reflection.task_achievable);
        assert!(reflection.confidence > 0.5);
    }
}
