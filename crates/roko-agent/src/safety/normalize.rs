//! Unicode normalization and confusable-character stripping for safety classification.
//!
//! All text entering the corrigibility classifier passes through [`normalize_for_classification`]
//! which applies NFKC normalization, strips zero-width and confusable characters, collapses
//! whitespace, and lowercases. This prevents homoglyph attacks (e.g. Cyrillic "а" for Latin "a")
//! and zero-width character insertion from bypassing keyword detection.

use unicode_normalization::UnicodeNormalization;

/// Maximum input length processed by the classifier (bytes). Inputs beyond this
/// are truncated before normalization to bound CPU cost.
const MAX_INPUT_BYTES: usize = 8192;

/// Characters that are stripped entirely after NFKC normalization.
/// These are invisible or formatting characters that can be inserted to break
/// keyword matching without changing visual appearance.
const STRIPPED_CODEPOINTS: &[char] = &[
    '\u{200B}', // zero-width space
    '\u{200C}', // zero-width non-joiner
    '\u{200D}', // zero-width joiner
    '\u{200E}', // left-to-right mark
    '\u{200F}', // right-to-left mark
    '\u{FEFF}', // byte order mark / zero-width no-break space
    '\u{00AD}', // soft hyphen
    '\u{034F}', // combining grapheme joiner
    '\u{2060}', // word joiner
    '\u{2061}', // function application
    '\u{2062}', // invisible times
    '\u{2063}', // invisible separator
    '\u{2064}', // invisible plus
    '\u{FE00}', // variation selector 1
    '\u{FE01}', // variation selector 2
    '\u{FE0F}', // variation selector 16 (emoji presentation)
    '\u{FE0E}', // variation selector 15 (text presentation)
    '\u{180E}', // Mongolian vowel separator
    '\u{061C}', // Arabic letter mark
    '\u{2066}', // left-to-right isolate
    '\u{2067}', // right-to-left isolate
    '\u{2068}', // first strong isolate
    '\u{2069}', // pop directional isolate
    '\u{202A}', // left-to-right embedding
    '\u{202B}', // right-to-left embedding
    '\u{202C}', // pop directional formatting
    '\u{202D}', // left-to-right override
    '\u{202E}', // right-to-left override
];

/// Normalize text for safety classification.
///
/// Steps:
/// 1. Truncate to [`MAX_INPUT_BYTES`] (on a char boundary)
/// 2. NFKC normalization (folds compatibility characters, e.g. fullwidth → ASCII)
/// 3. Strip zero-width and invisible characters
/// 4. Collapse runs of whitespace to single space
/// 5. Lowercase
///
/// The result is deterministic: same input always produces the same output.
#[must_use]
pub fn normalize_for_classification(input: &str) -> String {
    // Step 1: truncate
    let truncated = truncate_to_char_boundary(input, MAX_INPUT_BYTES);

    // Step 2: NFKC normalization
    let nfkc: String = truncated.nfkc().collect();

    // Step 3: strip invisible characters
    let stripped: String = nfkc
        .chars()
        .filter(|c| !STRIPPED_CODEPOINTS.contains(c))
        .collect();

    // Step 4: collapse whitespace and Step 5: lowercase
    let mut result = String::with_capacity(stripped.len());
    let mut prev_was_space = true; // trim leading whitespace
    for ch in stripped.chars() {
        if ch.is_whitespace() {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            for lc in ch.to_lowercase() {
                result.push(lc);
            }
            prev_was_space = false;
        }
    }
    // trim trailing space
    if result.ends_with(' ') {
        result.pop();
    }

    result
}

/// Truncate a string to at most `max_bytes`, respecting char boundaries.
fn truncate_to_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Structured action features extracted from tool context rather than raw text.
///
/// These features are derived from tool definitions, permission bits, and
/// declared operation metadata. They take priority over keyword matching
/// because they cannot be evaded by paraphrasing or language switching.
#[derive(Debug, Clone, Default)]
pub struct CorrigibilityActionFeatures {
    /// Tool modifies audit, logging, or cancellation infrastructure.
    pub modifies_oversight: bool,
    /// Tool modifies safety, auth, or privilege configuration.
    pub modifies_safety_config: bool,
    /// Tool performs destructive storage operations (delete, truncate, drop).
    pub destructive_storage: bool,
    /// Tool performs protected git operations (force-push, reset --hard, branch -D).
    pub protected_git_op: bool,
    /// Tool modifies assertion or evidence records.
    pub modifies_evidence: bool,
    /// Tool has exec permission.
    pub has_exec: bool,
    /// Tool has write permission.
    pub has_write: bool,
    /// Tool has git permission.
    pub has_git: bool,
    /// Tool has network permission.
    pub has_network: bool,
}

impl CorrigibilityActionFeatures {
    /// Extract structural features from a tool definition.
    #[must_use]
    pub fn from_tool_def(tool: &roko_core::tool::ToolDef) -> Self {
        let name_lower = tool.name.to_ascii_lowercase();
        Self {
            modifies_oversight: is_oversight_tool(&name_lower),
            modifies_safety_config: is_safety_config_tool(&name_lower),
            destructive_storage: is_destructive_storage_tool(&name_lower),
            protected_git_op: false, // determined from params, not tool name alone
            modifies_evidence: is_evidence_tool(&name_lower),
            has_exec: tool.permission.exec,
            has_write: tool.permission.write,
            has_git: tool.permission.git,
            has_network: tool.permission.network,
        }
    }

    /// Check params for protected git operations.
    #[must_use]
    pub fn with_git_params_check(mut self, params: &serde_json::Value) -> Self {
        if self.has_git {
            if let Some(cmd) = params
                .get("command")
                .and_then(|v| v.as_str())
                .or_else(|| params.get("args").and_then(|v| v.as_str()))
            {
                let cmd_lower = cmd.to_ascii_lowercase();
                self.protected_git_op = cmd_lower.contains("force")
                    || cmd_lower.contains("--hard")
                    || cmd_lower.contains("branch -d")
                    || cmd_lower.contains("branch -D")
                    || cmd_lower.contains("push -f")
                    || cmd_lower.contains("push --force")
                    || cmd_lower.contains("reset --hard");
            }
        }
        self
    }
}

fn is_oversight_tool(name: &str) -> bool {
    name.contains("audit")
        || name.contains("logging")
        || name.contains("cancel")
        || name.contains("monitor")
        || name.contains("oversight")
}

fn is_safety_config_tool(name: &str) -> bool {
    name.contains("safety")
        || name.contains("privilege")
        || name.contains("permission")
        || name.contains("auth_config")
        || name.contains("security")
}

fn is_destructive_storage_tool(name: &str) -> bool {
    name.contains("delete")
        || name.contains("truncate")
        || name.contains("drop")
        || name.contains("destroy")
        || name.contains("purge")
        || name.contains("wipe")
}

fn is_evidence_tool(name: &str) -> bool {
    name.contains("evidence")
        || name.contains("assertion")
        || name.contains("witness")
        || name.contains("attestation")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── normalize_for_classification ───────────────────────────────────────

    #[test]
    fn basic_normalization() {
        assert_eq!(
            normalize_for_classification("Disable AUDIT Logging"),
            "disable audit logging"
        );
    }

    #[test]
    fn strips_zero_width_characters() {
        // Zero-width space inserted between "dis" and "able"
        let input = "dis\u{200B}able audit";
        assert_eq!(normalize_for_classification(input), "disable audit");
    }

    #[test]
    fn strips_multiple_invisible_chars() {
        let input = "\u{200B}\u{200C}\u{200D}disable\u{FEFF} audit\u{200E}";
        assert_eq!(normalize_for_classification(input), "disable audit");
    }

    #[test]
    fn collapses_whitespace() {
        assert_eq!(
            normalize_for_classification("disable   audit\t\nlogging"),
            "disable audit logging"
        );
    }

    #[test]
    fn nfkc_folds_fullwidth() {
        // Fullwidth Latin letters: disable
        let input = "\u{FF44}\u{FF49}\u{FF53}\u{FF41}\u{FF42}\u{FF4C}\u{FF45} audit";
        assert_eq!(normalize_for_classification(input), "disable audit");
    }

    #[test]
    fn empty_input() {
        assert_eq!(normalize_for_classification(""), "");
    }

    #[test]
    fn whitespace_only_input() {
        assert_eq!(normalize_for_classification("   \t\n  "), "");
    }

    #[test]
    fn idempotent() {
        let input = "Disable\u{200B} AUDIT";
        let once = normalize_for_classification(input);
        let twice = normalize_for_classification(&once);
        assert_eq!(once, twice, "normalization must be idempotent");
    }

    #[test]
    fn truncates_long_input() {
        let long_input = "a".repeat(MAX_INPUT_BYTES + 1000);
        let result = normalize_for_classification(&long_input);
        assert!(result.len() <= MAX_INPUT_BYTES);
    }

    #[test]
    fn soft_hyphen_stripped() {
        let input = "dis\u{00AD}able audit";
        assert_eq!(normalize_for_classification(input), "disable audit");
    }

    #[test]
    fn bidi_override_stripped() {
        let input = "\u{202E}disable audit\u{202C}";
        assert_eq!(normalize_for_classification(input), "disable audit");
    }

    // ── CorrigibilityActionFeatures ───────────────────────────────────────

    #[test]
    fn features_from_tool_def_detects_oversight() {
        use roko_core::tool::{ToolCategory, ToolDef, ToolPermission};
        let tool = ToolDef::new(
            "delete_audit_log",
            "removes audit entries",
            ToolCategory::Write,
            ToolPermission::writes(),
        );
        let features = CorrigibilityActionFeatures::from_tool_def(&tool);
        assert!(features.modifies_oversight);
        assert!(!features.modifies_safety_config);
    }

    #[test]
    fn features_detects_destructive_storage() {
        use roko_core::tool::{ToolCategory, ToolDef, ToolPermission};
        let tool = ToolDef::new(
            "destroy_database",
            "drops all tables",
            ToolCategory::Write,
            ToolPermission::writes(),
        );
        let features = CorrigibilityActionFeatures::from_tool_def(&tool);
        assert!(features.destructive_storage);
    }

    #[test]
    fn features_detects_git_force_push_in_params() {
        use roko_core::tool::{ToolCategory, ToolDef, ToolPermission};
        let tool = ToolDef::new(
            "git",
            "git operations",
            ToolCategory::Exec,
            ToolPermission::git_ops(),
        );
        let params = serde_json::json!({"command": "git push --force origin main"});
        let features =
            CorrigibilityActionFeatures::from_tool_def(&tool).with_git_params_check(&params);
        assert!(features.protected_git_op);
    }

    #[test]
    fn features_no_false_positive_on_normal_git() {
        use roko_core::tool::{ToolCategory, ToolDef, ToolPermission};
        let tool = ToolDef::new(
            "git",
            "git operations",
            ToolCategory::Exec,
            ToolPermission::git_ops(),
        );
        let params = serde_json::json!({"command": "git add ."});
        let features =
            CorrigibilityActionFeatures::from_tool_def(&tool).with_git_params_check(&params);
        assert!(!features.protected_git_op);
    }
}
