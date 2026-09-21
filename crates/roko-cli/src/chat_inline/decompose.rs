//! Inline task decomposition for the chat session.
//!
//! Splits a user message into ordered sub-tasks when it contains explicit
//! multi-task patterns (numbered lists, conjunction keywords, etc.).
//! Each sub-task is dispatched independently through the existing agent path
//! and results are merged into a single unified response before being shown.

use crate::dispatch_v2::DispatchResult;

// ---------------------------------------------------------------------------
// Sub-task representation
// ---------------------------------------------------------------------------

/// A single unit of work extracted from a user message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubTask {
    /// Index (0-based) within the original message's task list.
    pub index: usize,
    /// The text of this sub-task, trimmed and ready to dispatch.
    pub text: String,
}

// ---------------------------------------------------------------------------
// Decomposition
// ---------------------------------------------------------------------------

/// Attempt to split `msg` into ordered sub-tasks.
///
/// Returns `None` when the message should be treated as a single task (no
/// recognisable multi-task pattern was found, or the message is very short).
/// Returns `Some(vec)` with at least two elements when decomposition applies.
///
/// The splitting strategy is purely pattern-based — no LLM call is made.
///
/// Recognised patterns (in priority order):
/// 1. Numbered-list prefixes: lines that begin with `N.` or `N)` where N is a digit.
/// 2. Conjunction keywords on sentence boundaries: ` and `, ` then `, ` also `.
///
/// To keep the feature safe the function enforces several guards:
/// - Messages shorter than 20 characters are never split.
/// - Slash-command messages (`/…`) are never split.
/// - After splitting, any chunk shorter than 8 characters is dropped.
/// - If fewer than 2 chunks survive, `None` is returned.
pub(crate) fn decompose_message(msg: &str) -> Option<Vec<SubTask>> {
    let trimmed = msg.trim();

    // Guard: too short to be multi-task
    if trimmed.len() < 20 {
        return None;
    }

    // Guard: slash commands are never decomposed
    if trimmed.starts_with('/') {
        return None;
    }

    // Try numbered-list splitting first (highest confidence).
    if let Some(tasks) = try_numbered_list(trimmed) {
        return Some(tasks);
    }

    // Fall back to conjunction-based splitting.
    if let Some(tasks) = try_conjunction_split(trimmed) {
        return Some(tasks);
    }

    None
}

// ---------------------------------------------------------------------------
// Strategy 1: numbered list
// ---------------------------------------------------------------------------

/// Detect lines that begin with a decimal digit followed by `.` or `)`.
///
/// Example input:
/// ```text
/// 1. Fix the login bug
/// 2. Add tests for the auth module
/// 3. Update the README
/// ```
fn try_numbered_list(text: &str) -> Option<Vec<SubTask>> {
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if lines.len() < 2 {
        return None;
    }

    // Check whether at least half the lines start with `N.` or `N)`.
    let numbered: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| {
            let mut chars = l.chars();
            if let Some(c) = chars.next() {
                if c.is_ascii_digit() {
                    if let Some(sep) = chars.next() {
                        return sep == '.' || sep == ')';
                    }
                }
            }
            false
        })
        .collect();

    if numbered.len() < 2 || numbered.len() * 2 < lines.len() {
        return None;
    }

    // Strip the leading "N." / "N)" prefix and build sub-tasks.
    let tasks: Vec<SubTask> = numbered
        .iter()
        .enumerate()
        .filter_map(|(i, line)| {
            // Skip past the digit + separator
            let after_sep = line
                .chars()
                .enumerate()
                .find(|(_, c)| *c == '.' || *c == ')')
                .map(|(idx, _)| idx + 1)?;
            let body = line[after_sep..].trim();
            if body.len() < 8 {
                return None;
            }
            Some(SubTask {
                index: i,
                text: body.to_string(),
            })
        })
        .collect();

    if tasks.len() >= 2 { Some(tasks) } else { None }
}

// ---------------------------------------------------------------------------
// Strategy 2: conjunction-based splitting
// ---------------------------------------------------------------------------

/// The conjunction markers we recognise (case-insensitive, surrounded by spaces
/// or at sentence boundaries).
const CONJUNCTIONS: &[&str] = &[
    " and also ",
    " and then ",
    ". then ",
    ", then ",
    " and ",
    " then ",
    " also ",
];

fn try_conjunction_split(text: &str) -> Option<Vec<SubTask>> {
    // Normalise to lowercase for matching but keep the original for output.
    let lower = text.to_lowercase();

    // Find the first conjunction that produces a valid split.
    for &conj in CONJUNCTIONS {
        if let Some(pos) = lower.find(conj) {
            let left = text[..pos].trim();
            let right = text[pos + conj.len()..].trim();

            if left.len() >= 8 && right.len() >= 8 {
                // Check if the right side itself contains another conjunction —
                // if so, recurse to get all the pieces.
                let mut tasks = vec![SubTask { index: 0, text: left.to_string() }];
                if let Some(mut sub) = try_conjunction_split(right) {
                    for t in &mut sub {
                        t.index += 1;
                    }
                    tasks.extend(sub);
                } else {
                    tasks.push(SubTask { index: 1, text: right.to_string() });
                }
                return Some(tasks);
            }
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Result merging
// ---------------------------------------------------------------------------

/// Merge an ordered list of per-sub-task `DispatchResult`s into one unified
/// response that the chat session can display as a single assistant turn.
///
/// The merge strategy:
/// - Text sections are labelled with the sub-task number and joined.
/// - Token counts are summed.
/// - The model name is taken from the first result (all sub-tasks run on the
///   same session so the model never changes mid-turn).
/// - Tool outputs from every sub-task are concatenated in order.
/// - The session_id is taken from the last result that provides one.
pub(crate) fn merge_results(results: Vec<(SubTask, DispatchResult)>) -> DispatchResult {
    if results.is_empty() {
        return DispatchResult {
            text: String::new(),
            model: String::new(),
            input_tokens: 0,
            output_tokens: 0,
            tool_outputs: Vec::new(),
            session_id: None,
        };
    }

    if results.len() == 1 {
        let (_, r) = results.into_iter().next().unwrap();
        return r;
    }

    let total = results.len();
    let mut text_parts: Vec<String> = Vec::with_capacity(total);
    let mut input_tokens: u64 = 0;
    let mut output_tokens: u64 = 0;
    let mut model = String::new();
    let mut tool_outputs = Vec::new();
    let mut session_id: Option<String> = None;

    for (task, result) in results {
        // Label each section by its sub-task number (1-based for humans).
        text_parts.push(format!("**Task {}:** {}", task.index + 1, result.text));
        input_tokens = input_tokens.saturating_add(result.input_tokens);
        output_tokens = output_tokens.saturating_add(result.output_tokens);
        if model.is_empty() {
            model = result.model;
        }
        tool_outputs.extend(result.tool_outputs);
        if result.session_id.is_some() {
            session_id = result.session_id;
        }
    }

    DispatchResult {
        text: text_parts.join("\n\n---\n\n"),
        model,
        input_tokens,
        output_tokens,
        tool_outputs,
        session_id,
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- decompose_message ---

    #[test]
    fn short_message_not_decomposed() {
        assert!(decompose_message("fix the bug").is_none());
    }

    #[test]
    fn slash_command_not_decomposed() {
        assert!(decompose_message("/help and also show /version").is_none());
    }

    #[test]
    fn single_sentence_not_decomposed() {
        assert!(
            decompose_message("please summarise the architecture of this project for me").is_none()
        );
    }

    #[test]
    fn numbered_list_two_items() {
        let tasks =
            decompose_message("1. Fix the login bug\n2. Add tests for the auth module").unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].text, "Fix the login bug");
        assert_eq!(tasks[1].text, "Add tests for the auth module");
        assert_eq!(tasks[0].index, 0);
        assert_eq!(tasks[1].index, 1);
    }

    #[test]
    fn numbered_list_three_items() {
        let msg = "1. Fix the bug in X\n2. Add tests for Y\n3. Update the README";
        let tasks = decompose_message(msg).unwrap();
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[2].text, "Update the README");
    }

    #[test]
    fn numbered_list_paren_separator() {
        let msg = "1) Fix the login bug\n2) Add tests for the auth module";
        let tasks = decompose_message(msg).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].text, "Fix the login bug");
    }

    #[test]
    fn conjunction_and_splits_two() {
        let msg = "fix the bug in the login module and add tests for the auth flow";
        let tasks = decompose_message(msg).unwrap();
        assert_eq!(tasks.len(), 2);
        assert!(tasks[0].text.contains("login module"));
        assert!(tasks[1].text.contains("auth flow"));
    }

    #[test]
    fn conjunction_then_splits_two() {
        let msg = "update the README for the project then run cargo clippy to check lints";
        let tasks = decompose_message(msg).unwrap();
        assert_eq!(tasks.len(), 2);
        assert!(tasks[0].text.contains("README"));
        assert!(tasks[1].text.contains("clippy"));
    }

    #[test]
    fn conjunction_also_splits_two() {
        let msg = "check the gate configuration in roko.toml also verify the provider health registry";
        let tasks = decompose_message(msg).unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn conjunction_chain_three_parts() {
        let msg = "fix the login bug and add tests for auth and update the README file";
        let tasks = decompose_message(msg).unwrap();
        assert_eq!(tasks.len(), 3);
    }

    #[test]
    fn conjunction_and_also_preferred_over_plain_and() {
        let msg = "fix the login module and also add tests for the auth flow";
        let tasks = decompose_message(msg).unwrap();
        assert_eq!(tasks.len(), 2);
        assert!(tasks[0].text.contains("login module"));
        assert!(tasks[1].text.contains("auth flow"));
    }

    // --- merge_results ---

    fn make_result(text: &str, model: &str, input: u64, output: u64) -> DispatchResult {
        DispatchResult {
            text: text.to_string(),
            model: model.to_string(),
            input_tokens: input,
            output_tokens: output,
            tool_outputs: Vec::new(),
            session_id: None,
        }
    }

    fn make_task(index: usize, text: &str) -> SubTask {
        SubTask { index, text: text.to_string() }
    }

    #[test]
    fn merge_empty_returns_empty() {
        let r = merge_results(vec![]);
        assert!(r.text.is_empty());
        assert_eq!(r.input_tokens, 0);
        assert_eq!(r.output_tokens, 0);
    }

    #[test]
    fn merge_single_passthrough() {
        let result = make_result("hello", "claude-sonnet-4-6", 10, 20);
        let task = make_task(0, "say hello");
        let merged = merge_results(vec![(task, result)]);
        assert_eq!(merged.text, "hello");
        assert_eq!(merged.model, "claude-sonnet-4-6");
        assert_eq!(merged.input_tokens, 10);
        assert_eq!(merged.output_tokens, 20);
    }

    #[test]
    fn merge_two_sums_tokens() {
        let r1 = make_result("Answer to task 1.", "claude-sonnet-4-6", 10, 20);
        let r2 = make_result("Answer to task 2.", "claude-sonnet-4-6", 15, 30);
        let merged = merge_results(vec![
            (make_task(0, "task 1"), r1),
            (make_task(1, "task 2"), r2),
        ]);
        assert_eq!(merged.input_tokens, 25);
        assert_eq!(merged.output_tokens, 50);
        assert!(merged.text.contains("Task 1"));
        assert!(merged.text.contains("Task 2"));
        assert!(merged.text.contains("---"));
    }

    #[test]
    fn merge_takes_model_from_first() {
        let r1 = make_result("r1", "model-a", 1, 1);
        let r2 = make_result("r2", "model-b", 1, 1);
        let merged = merge_results(vec![
            (make_task(0, "t1"), r1),
            (make_task(1, "t2"), r2),
        ]);
        assert_eq!(merged.model, "model-a");
    }

    #[test]
    fn merge_session_id_from_last() {
        let mut r1 = make_result("r1", "m", 1, 1);
        r1.session_id = Some("sess-1".to_string());
        let mut r2 = make_result("r2", "m", 1, 1);
        r2.session_id = Some("sess-2".to_string());
        let merged = merge_results(vec![
            (make_task(0, "t1"), r1),
            (make_task(1, "t2"), r2),
        ]);
        assert_eq!(merged.session_id.as_deref(), Some("sess-2"));
    }
}
