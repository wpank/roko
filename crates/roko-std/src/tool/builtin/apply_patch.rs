//! `apply_patch` — apply a patch to one existing file in the worktree.
//!
//! Category: [`ToolCategory::Write`]. Permission: read + write.
//! Concurrency: [`ToolConcurrency::Serial`]. Idempotent: no.
//!
//! # Formats
//!
//! - A **unified diff** of one file, with one or more hunks. Each hunk's
//!   context lines must match the current file exactly at the line its
//!   header names (no fuzz). A diff without `---`/`+++` headers patches the
//!   call's `path`. Multi-file patches are rejected with a clear error
//!   pointing at the first unexpected file header.
//! - The **`*** Begin Patch` envelope** that OpenAI-family models send to a
//!   tool of this name (bug-f5470e): one `*** Update File: <path>` section
//!   whose hunks start with `@@`, optionally followed by a line of the file
//!   the hunk comes after, and hold ` `, `-` and `+` lines. A hunk is found
//!   by its context and removed lines, after the previous hunk: exactly, else
//!   ignoring trailing whitespace. `*** Add File`, `*** Delete File`,
//!   `*** Move to` and more than one file are refused with an error that says
//!   what the tool takes.
//!
//! A patch it cannot read is refused with both formats and an example of
//! each, so a model that sent something else can fix its next call.

use async_trait::async_trait;
use roko_core::tool::{
    ToolCall, ToolCategory, ToolConcurrency, ToolContext, ToolDef, ToolError, ToolHandler,
    ToolPermission, ToolResult, ToolSchema,
};

use super::sandbox::{require_string, require_within_worktree};

/// Canonical `snake_case` name.
pub const NAME: &str = "apply_patch";

/// Human-readable description sent to the LLM.
pub const DESCRIPTION: &str = "Apply a patch to one existing file in the worktree: a unified diff \
     (`--- a/f`, `+++ b/f`, `@@ -1,3 +1,3 @@` hunks), or a `*** Begin Patch` envelope with one \
     `*** Update File: <path>` section of `@@` hunks, ending `*** End Patch`. Create files with \
     write_file.";

/// The envelope's first line.
const BEGIN_PATCH: &str = "*** Begin Patch";

/// What the tool takes, with an example of each format, for a patch it
/// cannot read.
const FORMATS: &str = "apply_patch takes a patch of one existing file, in either format.\n\
A unified diff:\n\
--- a/src/app.py\n\
+++ b/src/app.py\n\
@@ -1,2 +1,2 @@\n \
def main():\n\
-    old()\n\
+    new()\n\
Or an envelope:\n\
*** Begin Patch\n\
*** Update File: src/app.py\n\
@@ def main():\n\
-    old()\n\
+    new()\n\
*** End Patch\n\
Context lines start with a space, removed lines with `-`, added lines with `+`. To create a file, \
use write_file.";

/// Build the [`ToolDef`] for `apply_patch`.
#[must_use]
pub fn tool_def() -> ToolDef {
    ToolDef::new(
        NAME,
        DESCRIPTION,
        ToolCategory::Write,
        ToolPermission::writes(),
    )
    .with_parameters(ToolSchema::from_value(serde_json::json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "Relative path to the file to patch; a patch that names its file \
                                takes that file."
            },
            "patch": {
                "type": "string",
                "description": "The patch of that one file: a unified diff, or a `*** Begin Patch` \
                                envelope with one `*** Update File: <path>` section. Context and \
                                removed lines must match the file."
            }
        },
        "required": ["path", "patch"],
        "additionalProperties": false
    })))
    .with_concurrency(ToolConcurrency::Serial)
    .with_idempotent(false)
    .with_timeout_ms(60_000)
}

/// Handler for `apply_patch` (§36.28).
#[derive(Debug, Clone, Copy, Default)]
pub struct Handler;

#[async_trait]
impl ToolHandler for Handler {
    fn name(&self) -> &str {
        NAME
    }

    async fn execute(&self, call: ToolCall, ctx: &ToolContext) -> ToolResult {
        let patch = match require_string(&call.arguments, "patch") {
            Ok(s) => s,
            Err(e) => return ToolResult::Err(e),
        };
        let path = call
            .arguments
            .get("path")
            .and_then(serde_json::Value::as_str);
        let parsed = match parse_patch(&patch, path) {
            Ok(p) => p,
            Err(e) => return ToolResult::Err(ToolError::Other(format!("apply_patch: {e}"))),
        };
        let absolute = match require_within_worktree(ctx.worktree(), &parsed.path) {
            Ok(p) => p,
            Err(e) => return ToolResult::Err(e),
        };
        let original = match tokio::fs::read_to_string(&absolute).await {
            Ok(s) => s,
            Err(e) => {
                return ToolResult::Err(ToolError::Other(format!(
                    "apply_patch: failed to read {}: {e}",
                    absolute.display()
                )));
            }
        };
        let patched = match &parsed.hunks {
            Hunks::Unified(hunks) => apply_hunks(&original, hunks),
            Hunks::Envelope(hunks) => apply_envelope_hunks(&original, hunks),
        };
        let patched = match patched {
            Ok(s) => s,
            Err(e) => return ToolResult::Err(ToolError::Other(format!("apply_patch: {e}"))),
        };
        match tokio::fs::write(&absolute, patched.as_bytes()).await {
            Ok(()) => ToolResult::text(format!(
                "patched {} ({} hunk(s))",
                absolute.display(),
                parsed.hunks.len()
            )),
            Err(e) => ToolResult::Err(ToolError::Other(format!(
                "apply_patch: write failed ({}): {e}",
                absolute.display()
            ))),
        }
    }
}

#[derive(Debug)]
struct ParsedPatch {
    path: String,
    hunks: Hunks,
}

/// A patch's hunks, as its format places them.
#[derive(Debug)]
enum Hunks {
    /// Unified-diff hunks, placed by the line their header names.
    Unified(Vec<Hunk>),
    /// Envelope hunks, placed by their lines.
    Envelope(Vec<EnvelopeHunk>),
}

impl Hunks {
    fn len(&self) -> usize {
        match self {
            Self::Unified(hunks) => hunks.len(),
            Self::Envelope(hunks) => hunks.len(),
        }
    }
}

#[derive(Debug)]
struct Hunk {
    old_start: usize,
    lines: Vec<HunkLine>,
}

/// A hunk of a `*** Begin Patch` envelope.
#[derive(Debug, Default)]
struct EnvelopeHunk {
    /// The file line its `@@` names, which the hunk comes after.
    anchor: Option<String>,
    lines: Vec<HunkLine>,
    /// It ends the file (`*** End of File`).
    at_end: bool,
}

#[derive(Debug)]
enum HunkLine {
    Context(String),
    Removed(String),
    Added(String),
}

impl HunkLine {
    /// A hunk line without its marker; an empty line is empty context.
    fn parse(line: &str) -> Option<Self> {
        if let Some(rest) = line.strip_prefix('+') {
            Some(Self::Added(rest.to_string()))
        } else if let Some(rest) = line.strip_prefix('-') {
            Some(Self::Removed(rest.to_string()))
        } else if let Some(rest) = line.strip_prefix(' ') {
            Some(Self::Context(rest.to_string()))
        } else if line.is_empty() {
            Some(Self::Context(String::new()))
        } else {
            None
        }
    }
}

/// `patch` in whichever format it is; a unified diff without file headers
/// patches `path`.
fn parse_patch(patch: &str, path: Option<&str>) -> Result<ParsedPatch, String> {
    if patch.trim_start().starts_with(BEGIN_PATCH) {
        parse_envelope(patch)
    } else {
        parse_single_file_patch(patch, path)
    }
}

fn parse_single_file_patch(diff: &str, default_path: Option<&str>) -> Result<ParsedPatch, String> {
    let mut path: Option<String> = None;
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut lines = diff.lines().peekable();
    while let Some(line) = lines.next() {
        if let Some(rest) = line.strip_prefix("+++ ") {
            let p = rest.trim_start_matches("b/").trim();
            if let Some(ref existing) = path {
                if existing != p {
                    return Err(format!(
                        "multi-file patch not supported (saw {existing} and {p})"
                    ));
                }
            } else {
                path = Some(p.to_string());
            }
            continue;
        }
        if line.starts_with("--- ") || line.starts_with("diff ") || line.starts_with("index ") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("@@ ") {
            let header = rest.trim_start_matches('-');
            let old_start = header
                .split([',', ' ', '@'])
                .next()
                .and_then(|n| n.parse::<usize>().ok())
                .ok_or_else(|| format!("malformed hunk header: {line}\n{FORMATS}"))?;
            let mut hunk = Hunk {
                old_start,
                lines: Vec::new(),
            };
            while let Some(peek) = lines.peek() {
                if peek.starts_with("@@ ") || peek.starts_with("--- ") || peek.starts_with("+++ ") {
                    break;
                }
                let Some(next) = lines.next() else { break };
                // Unrecognized line inside a hunk — bail out.
                let hline = HunkLine::parse(next)
                    .ok_or_else(|| format!("unexpected line in hunk: {next:?}\n{FORMATS}"))?;
                hunk.lines.push(hline);
            }
            hunks.push(hunk);
        }
    }
    if hunks.is_empty() {
        return Err(format!("no hunks found\n{FORMATS}"));
    }
    let path = path
        .or_else(|| default_path.map(str::to_string))
        .ok_or_else(|| format!("no `+++ ` header found and no `path`\n{FORMATS}"))?;
    Ok(ParsedPatch {
        path,
        hunks: Hunks::Unified(hunks),
    })
}

/// A `*** Begin Patch` envelope with one `*** Update File` section.
fn parse_envelope(patch: &str) -> Result<ParsedPatch, String> {
    let refuse = |what: &str| {
        Err(format!(
            "{what}: apply_patch changes one existing file per call, from a `*** Update File: \
             <path>` section\n{FORMATS}"
        ))
    };
    let mut path: Option<String> = None;
    let mut hunks: Vec<EnvelopeHunk> = Vec::new();
    let mut lines = patch.lines().skip_while(|line| line.trim().is_empty());
    lines.next(); // `*** Begin Patch`
    for line in lines {
        if line.trim_end() == "*** End Patch" {
            break;
        }
        if let Some(file) = line.strip_prefix("*** Update File:") {
            if let Some(first) = &path {
                return refuse(&format!("a second file, {}, after {first}", file.trim()));
            }
            path = Some(file.trim().to_string());
            continue;
        }
        if line.starts_with("*** Add File:") {
            return refuse("`*** Add File` creates a file; use write_file for that");
        }
        if line.starts_with("*** Delete File:") {
            return refuse("`*** Delete File` is not supported");
        }
        if line.starts_with("*** Move to:") {
            return refuse("`*** Move to` is not supported");
        }
        if line.trim_end() == "*** End of File" {
            if let Some(hunk) = hunks.last_mut() {
                hunk.at_end = true;
            }
            continue;
        }
        if path.is_none() {
            return Err(format!(
                "expected `*** Update File: <path>` after `*** Begin Patch`, found {line:?}\n\
                 {FORMATS}"
            ));
        }
        if let Some(anchor) = line.strip_prefix("@@") {
            let anchor = anchor.trim();
            hunks.push(EnvelopeHunk {
                anchor: (!anchor.is_empty()).then(|| anchor.to_string()),
                ..EnvelopeHunk::default()
            });
            continue;
        }
        let hline = HunkLine::parse(line)
            .ok_or_else(|| format!("unexpected line in hunk: {line:?}\n{FORMATS}"))?;
        if hunks.is_empty() {
            hunks.push(EnvelopeHunk::default());
        }
        if let Some(hunk) = hunks.last_mut() {
            hunk.lines.push(hline);
        }
    }
    let path = path.ok_or_else(|| format!("no `*** Update File: <path>` line\n{FORMATS}"))?;
    hunks.retain(|hunk| !hunk.lines.is_empty());
    if hunks.is_empty() {
        return Err(format!("no hunks found\n{FORMATS}"));
    }
    Ok(ParsedPatch {
        path,
        hunks: Hunks::Envelope(hunks),
    })
}

fn apply_hunks(original: &str, hunks: &[Hunk]) -> Result<String, String> {
    let src_lines: Vec<&str> = original.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut src_idx: usize = 0; // 0-based
    for (h_idx, hunk) in hunks.iter().enumerate() {
        // old_start is 1-based; move src_idx forward to that line,
        // copying verbatim.
        let target_idx = hunk.old_start.saturating_sub(1);
        if target_idx < src_idx {
            return Err(format!("hunk {h_idx} overlaps with a prior hunk"));
        }
        while src_idx < target_idx {
            if src_idx >= src_lines.len() {
                return Err(format!("hunk {h_idx} starts past end of file"));
            }
            out.push(src_lines[src_idx].to_string());
            src_idx += 1;
        }
        for line in &hunk.lines {
            match line {
                HunkLine::Context(text) | HunkLine::Removed(text) => {
                    let actual = src_lines.get(src_idx).copied().unwrap_or("");
                    if actual != text {
                        return Err(format!(
                            "hunk {h_idx}: context mismatch at line {} (expected {:?}, got {:?})",
                            src_idx + 1,
                            text,
                            actual
                        ));
                    }
                    if matches!(line, HunkLine::Context(_)) {
                        out.push(text.clone());
                    }
                    src_idx += 1;
                }
                HunkLine::Added(text) => {
                    out.push(text.clone());
                }
            }
        }
    }
    // Tail of unchanged source.
    while src_idx < src_lines.len() {
        out.push(src_lines[src_idx].to_string());
        src_idx += 1;
    }
    Ok(joined(&out, original))
}

/// Apply envelope `hunks` to `original` in order, each found by its context
/// and removed lines after the one before it, and after its `@@` line when it
/// names one.
fn apply_envelope_hunks(original: &str, hunks: &[EnvelopeHunk]) -> Result<String, String> {
    let mut lines: Vec<String> = original.lines().map(str::to_string).collect();
    let mut cursor = 0;
    for (h_idx, hunk) in hunks.iter().enumerate() {
        let old: Vec<&str> = hunk
            .lines
            .iter()
            .filter_map(|line| match line {
                HunkLine::Context(text) | HunkLine::Removed(text) => Some(text.as_str()),
                HunkLine::Added(_) => None,
            })
            .collect();
        let from = match &hunk.anchor {
            Some(anchor) => {
                let found = lines
                    .iter()
                    .skip(cursor)
                    .position(|line| line.trim() == anchor.as_str())
                    .ok_or_else(|| {
                        format!(
                            "hunk {h_idx}: its `@@ {anchor}` line is not in the file after line \
                             {cursor}; read the file and copy the line exactly"
                        )
                    })?;
                cursor + found + 1
            }
            None => cursor,
        };
        let at = if old.is_empty() {
            // A hunk that only adds lines goes where its `@@` line or the
            // end of the file puts it.
            match (hunk.at_end, &hunk.anchor) {
                (true, _) => lines.len(),
                (false, Some(_)) => from,
                (false, None) => {
                    return Err(format!(
                        "hunk {h_idx} only adds lines: give it context lines, or an `@@` line \
                         it comes after"
                    ));
                }
            }
        } else {
            let end_block = lines
                .len()
                .checked_sub(old.len())
                .filter(|start| *start >= from && same_lines(&lines[*start..], &old));
            match end_block.filter(|_| hunk.at_end) {
                Some(start) => start,
                None => find_block(&lines, from, &old).ok_or_else(|| {
                    format!(
                        "hunk {h_idx}: its context and removed lines are not in the file after \
                         line {from}; the first is {:?}. Read the file and copy them exactly",
                        old[0]
                    )
                })?,
            }
        };
        // Context lines keep the file's own text, which matched them.
        let mut matched = lines[at..at + old.len()].iter();
        let mut new = Vec::new();
        for line in &hunk.lines {
            match line {
                HunkLine::Context(_) => new.extend(matched.next().cloned()),
                HunkLine::Removed(_) => {
                    matched.next();
                }
                HunkLine::Added(text) => new.push(text.clone()),
            }
        }
        cursor = at + new.len();
        lines.splice(at..at + old.len(), new);
    }
    Ok(joined(&lines, original))
}

/// The first index at or after `from` where `block` matches `lines`:
/// exactly, else ignoring trailing whitespace.
fn find_block(lines: &[String], from: usize, block: &[&str]) -> Option<usize> {
    let last = lines.len().checked_sub(block.len())?;
    let starts = || from..=last;
    starts()
        .find(|&start| {
            lines[start..start + block.len()]
                .iter()
                .zip(block)
                .all(|(line, want)| line == want)
        })
        .or_else(|| starts().find(|&start| same_lines(&lines[start..], block)))
}

/// Whether `lines` begins with `block`, ignoring trailing whitespace.
fn same_lines(lines: &[String], block: &[&str]) -> bool {
    lines.len() >= block.len()
        && lines
            .iter()
            .zip(block)
            .all(|(line, want)| line.trim_end() == want.trim_end())
}

/// `lines` as file text, ending with a newline when `original` did.
fn joined(lines: &[String], original: &str) -> String {
    let mut result = lines.join("\n");
    if original.ends_with('\n') {
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_file_with_one_hunk() {
        let patch = "--- a/x.txt\n+++ b/x.txt\n@@ -1,3 +1,3 @@\n a\n-b\n+B\n c\n";
        let p = parse_single_file_patch(patch, None).expect("parse");
        assert_eq!(p.path, "x.txt");
        assert_eq!(p.hunks.len(), 1);
        let Hunks::Unified(hunks) = &p.hunks else {
            panic!("a unified diff");
        };
        assert_eq!(hunks[0].old_start, 1);
    }

    #[test]
    fn parse_rejects_multi_file_patch() {
        let patch = "+++ b/x.txt\n@@ -1 +1 @@\n-a\n+b\n+++ b/y.txt\n";
        let err = parse_single_file_patch(patch, None).expect_err("multi-file");
        assert!(err.contains("multi-file"));
    }

    #[test]
    fn parse_rejects_missing_header() {
        let patch = "@@ -1 +1 @@\n-a\n+b\n";
        let err = parse_single_file_patch(patch, None).expect_err("no file named");
        assert!(err.contains("*** Begin Patch"), "{err}");
    }

    /// bug-f5470e: a unified diff without file headers patches the call's
    /// `path`.
    #[test]
    fn a_diff_without_headers_patches_the_calls_path() {
        let patch = "@@ -1,2 +1,2 @@\n a\n-b\n+B\n";
        let p = parse_patch(patch, Some("x.txt")).expect("parse");
        assert_eq!(p.path, "x.txt");
    }

    #[test]
    fn apply_single_hunk_replacement() {
        let original = "a\nb\nc\n";
        let patch = "--- a/x.txt\n+++ b/x.txt\n@@ -1,3 +1,3 @@\n a\n-b\n+B\n c\n";
        let p = parse_single_file_patch(patch, None).expect("parse");
        let Hunks::Unified(hunks) = &p.hunks else {
            panic!("a unified diff");
        };
        let applied = apply_hunks(original, hunks).expect("apply");
        assert_eq!(applied, "a\nB\nc\n");
    }

    #[test]
    fn apply_rejects_context_mismatch() {
        let original = "a\nb\nc\n";
        let patch = "--- a/x.txt\n+++ b/x.txt\n@@ -1,3 +1,3 @@\n a\n-WRONG\n+B\n c\n";
        let p = parse_single_file_patch(patch, None).expect("parse");
        let Hunks::Unified(hunks) = &p.hunks else {
            panic!("a unified diff");
        };
        let err = apply_hunks(original, hunks).expect_err("mismatch");
        assert!(err.contains("context mismatch"));
    }

    #[test]
    fn apply_addition_at_end() {
        let original = "a\nb\n";
        let patch = "--- a/x.txt\n+++ b/x.txt\n@@ -1,2 +1,3 @@\n a\n b\n+c\n";
        let p = parse_single_file_patch(patch, None).expect("parse");
        let Hunks::Unified(hunks) = &p.hunks else {
            panic!("a unified diff");
        };
        let applied = apply_hunks(original, hunks).expect("apply");
        assert_eq!(applied, "a\nb\nc\n");
    }

    #[test]
    fn apply_preserves_no_trailing_newline() {
        let original = "a\nb";
        let patch = "--- a/x.txt\n+++ b/x.txt\n@@ -1,2 +1,2 @@\n a\n-b\n+B";
        let p = parse_single_file_patch(patch, None).expect("parse");
        let Hunks::Unified(hunks) = &p.hunks else {
            panic!("a unified diff");
        };
        let applied = apply_hunks(original, hunks).expect("apply");
        assert_eq!(applied, "a\nB");
    }

    /// The head of `demo/multistage-plan/validate.sh`, which demo-multistage's
    /// STAGE-3 edits.
    const VALIDATE_SH: &str = "#!/bin/sh\nset -eu\n\n# Verify required artefacts exist.\n\
for f in demo/multistage-plan/discovery.md demo/multistage-plan/decision.md; do\n  test -f \"$f\"\n\
done\n\n# Verify required Markdown headings are present.\n\
rg -q '^## Repository signals' demo/multistage-plan/discovery.md\n\
rg -q '^## Decision' demo/multistage-plan/decision.md\n\n\
# Verify cross-file references inside the decision document.\n\
rg -q 'discovery\\.md' demo/multistage-plan/decision.md\n";

    /// Run `apply_patch` on `patch` for `path` in a worktree holding
    /// `demo/multistage-plan/validate.sh`; the result and the file after it.
    async fn run(path: &str, patch: &str) -> (ToolResult, String) {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("demo/multistage-plan/validate.sh");
        std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
        std::fs::write(&file, VALIDATE_SH).expect("seed");
        let call = ToolCall::new(
            "call-1",
            NAME,
            serde_json::json!({ "path": path, "patch": patch }),
        );
        let result = Handler
            .execute(call, &ToolContext::testing(dir.path()))
            .await;
        (result, std::fs::read_to_string(&file).expect("read back"))
    }

    /// bug-f5470e: the `*** Begin Patch` envelope gpt-oss-120b sent in
    /// demo-multistage's STAGE-3 (13 times, each refused) applies to the file
    /// it names: an `@@` with no line, then the hunk found by its lines.
    #[tokio::test]
    async fn apply_patch_accepts_the_begin_patch_envelope() {
        let patch = "*** Begin Patch\n\
*** Update File: demo/multistage-plan/validate.sh\n\
@@\n\
-# Verify required Markdown headings are present.\n\
-rg -q '^## Repository signals' demo/multistage-plan/discovery.md\n\
-rg -q '^## Decision' demo/multistage-plan/decision.md\n\
+# Verify required Markdown headings are present.\n\
+rg -q '^## Repository signals' demo/multistage-plan/discovery.md\n\
+rg -q '^## Decision' demo/multistage-plan/decision.md\n\
+rg -q '^## Evidence considered' demo/multistage-plan/decision.md\n\
*** End Patch\n";
        let (result, after) = run("demo/multistage-plan/validate.sh", patch).await;

        assert!(matches!(result, ToolResult::Ok { .. }), "{result:?}");
        let expected = VALIDATE_SH.replace(
            "rg -q '^## Decision' demo/multistage-plan/decision.md\n",
            "rg -q '^## Decision' demo/multistage-plan/decision.md\n\
             rg -q '^## Evidence considered' demo/multistage-plan/decision.md\n",
        );
        assert_eq!(after, expected);
    }

    /// An envelope hunk after an `@@` line goes after that line; one marked
    /// `*** End of File` that only adds lines goes at the end; context lines
    /// whose trailing whitespace differs still match.
    #[tokio::test]
    async fn envelope_hunks_follow_their_at_lines_and_the_end_of_file() {
        let patch = "*** Begin Patch\n\
*** Update File: demo/multistage-plan/validate.sh\n\
@@ # Verify required artefacts exist.\n \
for f in demo/multistage-plan/discovery.md demo/multistage-plan/decision.md; do   \n\
-  test -f \"$f\"\n\
+  test -s \"$f\"\n\
@@\n\
+echo done\n\
*** End of File\n\
*** End Patch";
        let (result, after) = run("ignored.sh", patch).await;

        assert!(matches!(result, ToolResult::Ok { .. }), "{result:?}");
        let expected = format!(
            "{}echo done\n",
            VALIDATE_SH.replace("test -f \"$f\"", "test -s \"$f\"")
        );
        assert_eq!(after, expected);
    }

    /// bug-f5470e: what the tool cannot take is refused with what it takes:
    /// a new file, a second file, a hunk whose lines are not in the file, and
    /// text in no format. The file is unchanged.
    #[tokio::test]
    async fn a_patch_it_cannot_take_says_what_it_takes() {
        let path = "demo/multistage-plan/validate.sh";
        let cases = [
            (
                "*** Begin Patch\n*** Add File: new.sh\n+echo hi\n*** End Patch\n",
                "use write_file",
            ),
            (
                "*** Begin Patch\n*** Update File: a.sh\n@@\n-x\n+y\n*** Update File: b.sh\n\
                 *** End Patch\n",
                "a second file, b.sh, after a.sh",
            ),
            (
                "*** Begin Patch\n*** Update File: demo/multistage-plan/validate.sh\n@@\n\
                 -rg -q '^## Missing' x.md\n+rg -q '^## Found' x.md\n*** End Patch\n",
                "Read the file and copy them exactly",
            ),
            ("please replace line 3 with set -e", "A unified diff:"),
        ];
        for (patch, says) in cases {
            let (result, after) = run(path, patch).await;
            let ToolResult::Err(error) = result else {
                panic!("{patch:?} must be refused");
            };
            assert!(error.to_string().contains(says), "{patch:?}: {error}");
            assert_eq!(after, VALIDATE_SH, "{patch:?} changed the file");
        }
    }
}
