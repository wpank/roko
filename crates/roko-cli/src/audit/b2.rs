//! B2, extreme mutation (S05 §4.3, 7125).
//!
//! Could the checks that passed have failed? Each Rust function the attempt
//! changed, found with roko-lang-rust's symbol extractor over the diff's
//! hunks, has its body replaced in the audit worktree, one at a time, by
//! `Default::default()` and then by `unimplemented!()`. The task's checks
//! run once against each mutant through the A2 executor, at most
//! [`PER_MUTANT`] each and [`MAX_MUTANTS`] in all, and then the body is
//! restored. A surviving mutant marks a weak oracle, W = 1; W = 0 when
//! every mutant is killed; W is null when a run times out, the cap ends the
//! pass, or no changed Rust function has a body. Python waits for a mutator.

use std::path::Path;
use std::time::{Duration, Instant};

use roko_core::audit_types::AuditLabels;
use roko_core::{LanguageProvider, SymbolKind};
use roko_gate::attempt_diff::is_test_path;
use roko_lang_rust::RustLanguageProvider;
use serde_json::json;

use super::git;
use super::rerun::{checks_for, run_checks, target_dir};
use super::worker::{CheckOutcome, PhaseBCheck, UnitAudit};

/// The longest one mutant's checks may run.
pub const PER_MUTANT: Duration = Duration::from_secs(60);

/// The most mutants one audit runs.
pub const MAX_MUTANTS: usize = 40;

/// The extreme bodies, in the order they are tried.
const BODIES: [&str; 2] = ["Default::default()", "unimplemented!()"];

/// B2: extreme mutants of the Rust functions an attempt changed.
#[derive(Debug, Clone, Copy, Default)]
pub struct B2;

#[async_trait::async_trait]
impl PhaseBCheck for B2 {
    async fn check(&self, audit: &UnitAudit<'_>) -> CheckOutcome {
        let unit = audit.unit;
        let worktree = audit.worktree;
        let trees = unit.base_tree.as_deref().zip(unit.result_tree.as_deref());
        let Some((base, result)) = trees else {
            return null("the selection names no trees");
        };
        let functions = match changed_functions(audit.repo, worktree, base, result) {
            Ok(functions) => functions,
            Err(error) => return null(&error.to_string()),
        };
        if functions.is_empty() {
            return null("no changed Rust function has a body");
        }
        let mut changed: Vec<String> = functions
            .iter()
            .map(|function| function.path.clone())
            .collect();
        changed.dedup();
        let checks = checks_for(&unit.task.verify, &changed, PER_MUTANT);
        if checks.is_empty() {
            return null("the task has no checks to run against mutants");
        }
        let target = target_dir(audit.vault);
        let started = Instant::now();
        let mut mutants = Vec::new();
        let mut w = Some(false);
        'pass: for function in &functions {
            let file = worktree.join(&function.path);
            let Ok(original) = std::fs::read_to_string(&file) else {
                continue;
            };
            for body in BODIES {
                let left = audit.time_left.saturating_sub(started.elapsed());
                if mutants.len() >= MAX_MUTANTS || left.is_zero() {
                    w = None;
                    break 'pass;
                }
                let Some(mutant) = mutate(&original, function.line, body) else {
                    continue;
                };
                if std::fs::write(&file, mutant).is_err() {
                    continue;
                }
                let budget = left.min(PER_MUTANT);
                let survived = run_checks(worktree, &checks, budget, Some(&target), None).await;
                let _ = std::fs::write(&file, &original);
                mutants.push(json!({
                    "path": function.path,
                    "function": function.name,
                    "body": body,
                    "survived": survived,
                }));
                match survived {
                    Some(true) => {
                        w = Some(true);
                        break 'pass;
                    }
                    Some(false) => {}
                    None => {
                        w = None;
                        break 'pass;
                    }
                }
            }
        }
        if mutants.is_empty() {
            w = None;
        }
        CheckOutcome {
            labels: AuditLabels {
                w,
                ..AuditLabels::default()
            },
            cost_usd: 0.0,
            calls: Vec::new(),
            detail: json!({ "w": w, "mutants": mutants }),
        }
    }
}

/// B2's outcome when it cannot tell: no label, and why.
fn null(why: &str) -> CheckOutcome {
    CheckOutcome {
        detail: json!({ "null": why }),
        ..CheckOutcome::default()
    }
}

/// A changed Rust function: its file, its name and the 1-based line of its
/// `fn`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Function {
    path: String,
    name: String,
    line: usize,
}

/// The functions with a body that `base..result` changed in product Rust
/// files, read from `worktree`, which holds the result. What follows a
/// file's `#[cfg(test)]` is test code.
fn changed_functions(
    repo: &Path,
    worktree: &Path,
    base: &str,
    result: &str,
) -> anyhow::Result<Vec<Function>> {
    let args = ["diff", "--name-only", "-z", base, result, "--", "*.rs"];
    let names = git(repo, &args)?;
    let mut functions = Vec::new();
    for path in names.split('\0').filter(|path| !path.is_empty()) {
        if is_test_path(path) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(worktree.join(path)) else {
            continue;
        };
        let hunks = git(repo, &["diff", "-U0", base, result, "--", path])?;
        let lines: Vec<usize> = hunks.lines().filter_map(hunk_lines).flatten().collect();
        let tests_from = text
            .lines()
            .position(|line| line.trim_start().starts_with("#[cfg(test)]"))
            .map_or(usize::MAX, |index| index + 1);
        for symbol in RustLanguageProvider.extract_symbols(&text) {
            if symbol.kind != SymbolKind::Function || symbol.line >= tests_from {
                continue;
            }
            let Some((_, close)) = body_span(&text, symbol.line) else {
                continue;
            };
            let span = symbol.line..=line_of(&text, close);
            if lines.iter().any(|line| span.contains(line)) {
                functions.push(Function {
                    path: path.to_string(),
                    name: symbol.name,
                    line: symbol.line,
                });
            }
        }
    }
    Ok(functions)
}

/// The result-side lines of a `-U0` hunk header, `@@ -a,b +c,d @@`; a hunk
/// that only deletes touches the line it follows.
fn hunk_lines(line: &str) -> Option<std::ops::RangeInclusive<usize>> {
    let (_, added) = line.strip_prefix("@@ ")?.split_once(" +")?;
    let added = added.split(' ').next()?;
    let (start, count) = added.split_once(',').unwrap_or((added, "1"));
    let start = start.parse::<usize>().ok()?.max(1);
    let count = count.parse::<usize>().ok()?.max(1);
    Some(start..=start + count - 1)
}

/// `text` with the body of the function declared on line `line` replaced
/// by `body`.
fn mutate(text: &str, line: usize, body: &str) -> Option<String> {
    let (open, close) = body_span(text, line)?;
    Some(format!(
        "{}{{ {body} }}{}",
        &text[..open],
        &text[close + 1..]
    ))
}

/// The byte offsets of the braces around the body of the function declared
/// on 1-based line `line` of `text`, or `None` when it has no body.
fn body_span(text: &str, line: usize) -> Option<(usize, usize)> {
    let mut index: usize = text
        .split_inclusive('\n')
        .take(line.saturating_sub(1))
        .map(str::len)
        .sum();
    let bytes = text.as_bytes();
    let mut nested = 0_usize;
    let open = loop {
        match *bytes.get(index)? {
            b'(' | b'[' => nested += 1,
            b')' | b']' => nested = nested.saturating_sub(1),
            b';' if nested == 0 => return None,
            b'{' if nested == 0 => break index,
            _ => {}
        }
        index += 1;
    };
    Some((open, matching_brace(bytes, open)?))
}

/// The offset of the `}` that closes the `{` at `open`, past string and
/// character literals and comments.
fn matching_brace(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0_usize;
    let mut index = open;
    while let Some(&byte) = bytes.get(index) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            b'"' => index = string_end(bytes, index)?,
            b'\'' => index = char_end(bytes, index),
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index = find(bytes, index, b"\n").unwrap_or(bytes.len());
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index = find(bytes, index + 2, b"*/").map_or(bytes.len(), |at| at + 1);
            }
            _ => {}
        }
        index += 1;
    }
    None
}

/// The offset of the last byte of the string literal whose `"` is at
/// `quote`: a raw string ends at a `"` with as many `#` as it opened with.
fn string_end(bytes: &[u8], quote: usize) -> Option<usize> {
    let hashes = bytes[..quote]
        .iter()
        .rev()
        .take_while(|&&byte| byte == b'#')
        .count();
    let raw = quote
        .checked_sub(hashes + 1)
        .is_some_and(|at| bytes[at] == b'r');
    if raw {
        let mut end = vec![b'"'];
        end.extend(std::iter::repeat_n(b'#', hashes));
        return find(bytes, quote + 1, &end).map(|at| at + hashes);
    }
    let mut index = quote + 1;
    loop {
        match *bytes.get(index)? {
            b'\\' => index += 2,
            b'"' => return Some(index),
            _ => index += 1,
        }
    }
}

/// The offset of the `'` that ends the character literal opening at
/// `quote`, or `quote` itself when it opens a lifetime.
fn char_end(bytes: &[u8], quote: usize) -> usize {
    match (bytes.get(quote + 1), bytes.get(quote + 2)) {
        (Some(b'\\'), _) => find(bytes, quote + 3, b"'").unwrap_or(quote),
        (Some(_), Some(b'\'')) => quote + 2,
        _ => quote,
    }
}

/// Where `needle` first occurs in `bytes` at or after `from`.
fn find(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|at| from + at)
}

/// The 1-based line of the byte at `offset` in `text`.
fn line_of(text: &str, offset: usize) -> usize {
    text.as_bytes()[..offset]
        .split(|&byte| byte == b'\n')
        .count()
}

#[cfg(test)]
mod tests {
    use roko_core::audit_home::AuditVault;

    use super::*;
    use crate::audit::worker::{AuditTask, AuditUnit};
    use crate::audit::worktree::AuditWorktree;
    use crate::audit::worktree::tests::{repo_with, tree_of, write};

    const BASE: &str = "pub fn double(n: u32) -> u32 {\n    n + n\n}\n\n\
                        pub fn label() -> &'static str {\n    \"doubler\"\n}\n";

    /// The attempt rewrote `double`, a brace in a string included, and
    /// added a test.
    const RESULT: &str = "pub fn double(n: u32) -> u32 {\n    let _ = \"}{\";\n    n * 2\n}\n\n\
                          pub fn label() -> &'static str {\n    \"doubler\"\n}\n\n\
                          #[cfg(test)]\nmod tests {\n    #[test]\n    fn doubles() {\n        \
                          assert_eq!(super::double(2), 4);\n    }\n}\n";

    fn unit(base: &str, result: &str, verify: &str) -> AuditUnit {
        AuditUnit {
            sel_id: "sel-1".to_string(),
            attempt_key: "run-1:plan:T1:1".to_string(),
            run_id: "run-1".to_string(),
            plan_id: "plan".to_string(),
            task_id: "T1".to_string(),
            pi: 0.5,
            base_tree: Some(base.to_string()),
            result_tree: Some(result.to_string()),
            model: "claude-sonnet-4-6".to_string(),
            prediction_id: None,
            task: AuditTask {
                files: vec!["src/lib.rs".to_string()],
                verify: vec![("test".to_string(), verify.to_string())],
                kind: "code".to_string(),
                ..AuditTask::default()
            },
        }
    }

    #[tokio::test]
    async fn a_surviving_extreme_mutant_marks_a_weak_oracle() {
        let (open, close) = body_span(BASE, 1).expect("a body");
        assert_eq!(&BASE[open..=close], "{\n    n + n\n}");
        assert_eq!(body_span("trait T {\n    fn f(&self);\n}\n", 2), None);
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        let base = repo_with(&repo, &[("src/lib.rs", BASE)]);
        write(&repo, "src/lib.rs", RESULT);
        let result = tree_of(&repo);
        let home = temp.path().join("vault");
        let vault = AuditVault::resolve_with(&repo, Some(&home), None).expect("a vault");
        let path = vault.worktrees_dir().join("sel-1");
        let worktree = AuditWorktree::create(&repo, &path, &result, "audit").expect("a worktree");
        let found = changed_functions(&repo, worktree.path(), &base, &result).expect("the diff");
        let names: Vec<&str> = found
            .iter()
            .map(|function| function.name.as_str())
            .collect();
        assert_eq!(
            names,
            ["double"],
            "`label` is unchanged and `doubles` is a test"
        );

        // Tests that ignore `double`'s result let its first mutant live.
        let weak = unit(&base, &result, "true");
        let audit = UnitAudit {
            unit: &weak,
            repo: &repo,
            worktree: worktree.path(),
            vault: &vault,
            findings: &[],
            phase_a: AuditLabels::default(),
            usd_left: 0.0,
            time_left: Duration::from_secs(120),
        };
        let outcome = B2.check(&audit).await;
        assert_eq!(outcome.labels.w, Some(true), "{}", outcome.detail);

        // Tests that pin it kill both mutants, and the body comes back.
        let pinned = unit(&base, &result, "grep -q 'n \\* 2' src/lib.rs");
        let audit = UnitAudit {
            unit: &pinned,
            ..audit
        };
        let outcome = B2.check(&audit).await;
        assert_eq!(outcome.labels.w, Some(false), "{}", outcome.detail);
        assert_eq!(outcome.detail["mutants"].as_array().map(Vec::len), Some(2));
        let restored = std::fs::read_to_string(path.join("src/lib.rs")).expect("the file");
        assert_eq!(restored, RESULT);
        worktree.remove().expect("remove");
    }
}
