//! Structured reviewer output classification (#257).
//!
//! Accepts reviewer output in three formats, tried in order:
//! 1. Exact JSON object with `"verdict"` field.
//! 2. A single fenced JSON block (` ```json ... ``` `).
//! 3. Legacy text format with keyword detection.
//!
//! Malformed output is [`ReviewVerdict::Unclear`], never implicit approval.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Structured classification of reviewer output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum ReviewVerdict {
    /// The reviewer approved the changes without reservations.
    Approved,
    /// The reviewer requests revisions before approval.
    Revise {
        /// Specific findings that need to be addressed.
        findings: Vec<String>,
    },
    /// The reviewer rejected the changes outright.
    Rejected {
        /// Reason for rejection.
        reason: String,
    },
    /// The reviewer output could not be reliably classified.
    Unclear {
        /// Best-effort summary of the reviewer's output.
        summary: String,
    },
}

impl ReviewVerdict {
    /// Returns `true` if this verdict is an approval.
    #[must_use]
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Approved)
    }

    /// Returns `true` if this verdict requests revision.
    #[must_use]
    pub fn is_revise(&self) -> bool {
        matches!(self, Self::Revise { .. })
    }

    /// Returns `true` if this verdict is a rejection.
    #[must_use]
    pub fn is_rejected(&self) -> bool {
        matches!(self, Self::Rejected { .. })
    }

    /// Returns `true` if the output could not be classified.
    #[must_use]
    pub fn is_unclear(&self) -> bool {
        matches!(self, Self::Unclear { .. })
    }
}

// ---------------------------------------------------------------------------
// Internal JSON schema
// ---------------------------------------------------------------------------

/// Internal deserialization target for reviewer JSON output.
#[derive(Deserialize)]
struct ReviewJson {
    verdict: String,
    #[serde(default)]
    findings: Vec<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    summary: Option<String>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Parse reviewer output into a structured [`ReviewVerdict`].
///
/// Tries in order:
/// 1. Exact JSON parse.
/// 2. Extract fenced JSON block and parse.
/// 3. Legacy keyword-based text classification.
///
/// Returns [`ReviewVerdict::Unclear`] for any malformed or ambiguous output.
/// Never returns implicit approval.
#[must_use]
pub fn parse_review(raw: &str) -> ReviewVerdict {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return ReviewVerdict::Unclear {
            summary: String::new(),
        };
    }

    // 1. Try exact JSON.
    if let Some(v) = try_json(trimmed) {
        return v;
    }

    // 2. Try fenced JSON block.
    if let Some(v) = try_fenced_json(trimmed) {
        return v;
    }

    // 3. Legacy text format.
    parse_legacy_text(trimmed)
}

/// Attempt to parse the raw string as a complete JSON object.
fn try_json(s: &str) -> Option<ReviewVerdict> {
    // Quick guard: must start with `{`.
    if !s.starts_with('{') {
        return None;
    }
    let parsed: ReviewJson = serde_json::from_str(s).ok()?;
    Some(classify_json(parsed))
}

/// Extract a fenced JSON block and parse it.
fn try_fenced_json(s: &str) -> Option<ReviewVerdict> {
    let start_marker = "```json";
    let end_marker = "```";

    let json_start = s.find(start_marker)?;
    let content_start = json_start + start_marker.len();
    let rest = &s[content_start..];
    let json_end = rest.find(end_marker)?;
    let json_str = rest[..json_end].trim();

    let parsed: ReviewJson = serde_json::from_str(json_str).ok()?;
    Some(classify_json(parsed))
}

/// Map parsed JSON fields to a `ReviewVerdict`.
fn classify_json(parsed: ReviewJson) -> ReviewVerdict {
    match parsed.verdict.to_lowercase().as_str() {
        "approved" | "approve" | "lgtm" => ReviewVerdict::Approved,
        "revise" | "revision" | "changes_requested" => {
            if parsed.findings.is_empty() {
                ReviewVerdict::Revise {
                    findings: vec![parsed
                        .reason
                        .or(parsed.summary)
                        .unwrap_or_else(|| "Revisions requested".to_string())],
                }
            } else {
                ReviewVerdict::Revise {
                    findings: parsed.findings,
                }
            }
        }
        "rejected" | "reject" => ReviewVerdict::Rejected {
            reason: parsed
                .reason
                .or(parsed.summary)
                .unwrap_or_else(|| "Rejected without reason".to_string()),
        },
        _ => ReviewVerdict::Unclear {
            summary: parsed
                .summary
                .or(parsed.reason)
                .unwrap_or_else(|| format!("Unknown verdict: {}", parsed.verdict)),
        },
    }
}

/// Legacy text-based classification using keyword detection.
///
/// This is deliberately conservative: ambiguous output maps to `Unclear`.
fn parse_legacy_text(s: &str) -> ReviewVerdict {
    let lower = s.to_lowercase();

    // Check for explicit rejection first (strongest negative signal).
    if lower.contains("rejected") || lower.starts_with("reject:") {
        let reason = extract_after_keyword(s, &["rejected:", "reject:", "reason:"])
            .unwrap_or_else(|| truncate(s, 200));
        return ReviewVerdict::Rejected { reason };
    }

    // Check for revision request.
    if lower.contains("changes requested")
        || lower.contains("revise")
        || lower.contains("needs changes")
        || lower.contains("please fix")
    {
        let findings = extract_findings(s);
        return ReviewVerdict::Revise {
            findings: if findings.is_empty() {
                vec![truncate(s, 200)]
            } else {
                findings
            },
        };
    }

    // Check for approval (must be unambiguous).
    let approval_keywords = ["approved", "lgtm", "looks good", "ship it"];
    let has_approval = approval_keywords
        .iter()
        .any(|kw| lower.contains(kw));

    // Reject "approved" if there's also a qualification.
    let has_qualification = lower.contains("but ")
        || lower.contains("however")
        || lower.contains("except")
        || lower.contains("issue")
        || lower.contains("concern");

    if has_approval && !has_qualification {
        return ReviewVerdict::Approved;
    }

    // Ambiguous: classify as unclear.
    ReviewVerdict::Unclear {
        summary: truncate(s, 200),
    }
}

/// Extract text after the first occurrence of any keyword.
fn extract_after_keyword(s: &str, keywords: &[&str]) -> Option<String> {
    let lower = s.to_lowercase();
    for kw in keywords {
        if let Some(pos) = lower.find(kw) {
            let after = s[pos + kw.len()..].trim();
            if !after.is_empty() {
                return Some(truncate(after, 200));
            }
        }
    }
    None
}

/// Extract numbered or bulleted findings from text.
fn extract_findings(s: &str) -> Vec<String> {
    let mut findings = Vec::new();
    for line in s.lines() {
        let trimmed = line.trim();
        // Match patterns like "1.", "- ", "* ".
        let is_list = trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
                && trimmed.contains(". ");

        if is_list {
            let content = trimmed
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-' || c == '*')
                .trim();
            if !content.is_empty() {
                findings.push(content.to_string());
            }
        }
    }
    findings
}

/// Truncate a string to at most `max_len` characters, appending "..." if needed.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut result = s[..max_len].to_string();
        result.push_str("...");
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── JSON format tests ──────────────────────────────────────────────

    #[test]
    fn json_approved() {
        let input = r#"{"verdict": "approved"}"#;
        assert_eq!(parse_review(input), ReviewVerdict::Approved);
    }

    #[test]
    fn json_lgtm() {
        let input = r#"{"verdict": "lgtm"}"#;
        assert_eq!(parse_review(input), ReviewVerdict::Approved);
    }

    #[test]
    fn json_revise_with_findings() {
        let input = r#"{"verdict": "revise", "findings": ["Fix error handling", "Add docs"]}"#;
        let result = parse_review(input);
        assert!(result.is_revise());
        if let ReviewVerdict::Revise { findings } = result {
            assert_eq!(findings, vec!["Fix error handling", "Add docs"]);
        }
    }

    #[test]
    fn json_revise_with_reason_no_findings() {
        let input = r#"{"verdict": "revise", "reason": "needs error handling"}"#;
        let result = parse_review(input);
        if let ReviewVerdict::Revise { findings } = result {
            assert_eq!(findings, vec!["needs error handling"]);
        } else {
            panic!("expected Revise, got {result:?}");
        }
    }

    #[test]
    fn json_rejected() {
        let input = r#"{"verdict": "rejected", "reason": "security vulnerability"}"#;
        let result = parse_review(input);
        assert!(result.is_rejected());
        if let ReviewVerdict::Rejected { reason } = result {
            assert_eq!(reason, "security vulnerability");
        }
    }

    #[test]
    fn json_unknown_verdict() {
        let input = r#"{"verdict": "maybe"}"#;
        assert!(parse_review(input).is_unclear());
    }

    // ── Fenced JSON tests ──────────────────────────────────────────────

    #[test]
    fn fenced_json_approved() {
        let input = "Some preamble text\n```json\n{\"verdict\": \"approved\"}\n```\nMore text";
        assert_eq!(parse_review(input), ReviewVerdict::Approved);
    }

    #[test]
    fn fenced_json_revise() {
        let input = r#"Here is my review:
```json
{"verdict": "revise", "findings": ["Missing tests"]}
```"#;
        let result = parse_review(input);
        if let ReviewVerdict::Revise { findings } = result {
            assert_eq!(findings, vec!["Missing tests"]);
        } else {
            panic!("expected Revise, got {result:?}");
        }
    }

    // ── Legacy text format tests ───────────────────────────────────────

    #[test]
    fn legacy_approved() {
        assert_eq!(parse_review("LGTM"), ReviewVerdict::Approved);
    }

    #[test]
    fn legacy_approved_looks_good() {
        assert_eq!(
            parse_review("Looks good to me, ship it!"),
            ReviewVerdict::Approved
        );
    }

    #[test]
    fn legacy_rejected() {
        let result = parse_review("Rejected: security vulnerability in auth module");
        assert!(result.is_rejected());
        if let ReviewVerdict::Rejected { reason } = result {
            assert!(reason.contains("security vulnerability"));
        }
    }

    #[test]
    fn legacy_revise_with_list() {
        let input = "Changes requested:\n- Fix error handling\n- Add unit tests\n- Update docs";
        let result = parse_review(input);
        assert!(result.is_revise());
        if let ReviewVerdict::Revise { findings } = result {
            assert_eq!(findings.len(), 3);
            assert!(findings[0].contains("Fix error handling"));
        }
    }

    #[test]
    fn legacy_approved_but_qualified() {
        // "approved" + "but" -> unclear, not implicit approval.
        let result = parse_review("Approved, but there are some concerns about performance");
        assert!(result.is_unclear());
    }

    #[test]
    fn empty_input() {
        assert!(parse_review("").is_unclear());
        assert!(parse_review("   ").is_unclear());
    }

    #[test]
    fn ambiguous_text() {
        let result = parse_review("The code works but I'm not sure about the approach");
        assert!(result.is_unclear());
    }

    #[test]
    fn malformed_json_falls_through() {
        // Starts with { but is not valid JSON -- falls through to text.
        let input = "{not valid json at all}";
        let result = parse_review(input);
        assert!(result.is_unclear());
    }

    // ── Serde round-trip ───────────────────────────────────────────────

    #[test]
    fn verdict_serde_roundtrip() {
        let verdicts = [
            ReviewVerdict::Approved,
            ReviewVerdict::Revise {
                findings: vec!["fix".into()],
            },
            ReviewVerdict::Rejected {
                reason: "bad".into(),
            },
            ReviewVerdict::Unclear {
                summary: "hmm".into(),
            },
        ];
        for v in &verdicts {
            let json = serde_json::to_string(v).unwrap();
            let back: ReviewVerdict = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, v);
        }
    }
}
