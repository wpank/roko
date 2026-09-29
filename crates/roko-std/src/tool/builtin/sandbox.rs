//! Path-sandboxing helpers shared by filesystem tool handlers.
//!
//! All filesystem tools must resolve user-supplied paths **inside** the
//! worktree supplied by [`roko_core::tool::ToolContext::worktree`]. Any
//! path that escapes via `..` or an absolute prefix is rejected with
//! [`ToolError::PathOutsideWorktree`].
//!
//! # Algorithm
//!
//! 1. If the caller supplies an absolute path, canonicalize it and check
//!    that it sits under the worktree root. If canonicalization fails
//!    (path doesn't exist yet, as for `write_file`), fall back to a
//!    purely lexical `starts_with` check.
//! 2. If the caller supplies a relative path, join it to the worktree
//!    root, then normalize `..` components lexically and verify the
//!    result is still under the root.
//!
//! The check is deliberately conservative — symlinks that point outside
//! the worktree are treated as escapes. Downstream handlers can loosen
//! this via the §36.46 capability system when necessary.
//!
//! Provider key files ([`roko_core::child_env::is_key_file`], such as
//! `.roko/.env`) are refused with [`ToolError::KeyFileBlocked`], inside the
//! worktree too, by the path as given and with symlinks resolved, and the
//! `bash` tool refuses a command that names one
//! ([`refuse_key_file_in_command`]). The tools check this themselves, so
//! the block holds whichever dispatcher runs them, with or without
//! roko-agent's `SafetyLayer`.

use std::path::{Component, Path, PathBuf};

use roko_core::child_env::{KEY_FILE_NAMES, is_key_file};
use roko_core::tool::ToolError;

/// Resolve `rel` against `worktree` and ensure the result stays inside
/// the worktree and is not a provider key file. Returns the absolute,
/// normalized path on success.
///
/// # Errors
///
/// Returns [`ToolError::KeyFileBlocked`] if the path is a provider key file
/// (see [`refuse_key_file`]), and [`ToolError::PathOutsideWorktree`] if the
/// resolved path escapes the supplied worktree.
pub fn require_within_worktree(worktree: &Path, rel: &str) -> Result<PathBuf, ToolError> {
    let candidate = Path::new(rel);
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        worktree.join(candidate)
    };
    let normalized = normalize(&joined);
    refuse_key_file(&normalized)?;
    let normalized_root = normalize(worktree);
    if !normalized.starts_with(&normalized_root) {
        return Err(ToolError::PathOutsideWorktree(normalized));
    }
    Ok(normalized)
}

/// Refuse `path` if it is a provider key file, as given or with symlinks
/// resolved: a symlink in the worktree can point at one.
///
/// # Errors
///
/// Returns [`ToolError::KeyFileBlocked`] naming the form that is a key file.
pub fn refuse_key_file(path: &Path) -> Result<(), ToolError> {
    if is_key_file(path) {
        return Err(ToolError::KeyFileBlocked(path.to_path_buf()));
    }
    let resolved = resolve_symlinks(path);
    if is_key_file(&resolved) {
        return Err(ToolError::KeyFileBlocked(resolved));
    }
    Ok(())
}

/// Refuse `command`, a shell command line that runs in `cwd`, if it names a
/// provider key file.
///
/// The rules are the Claude CLI command guard's (`claude_cli_guard.py` in
/// roko-agent), applied to the command's text and to its words with quotes
/// and escapes removed, those of `sh -c '…'` strings included:
///
/// - a path to a key file appears (`.roko/.env`, `~/.roko/config.toml`), or
///   a glob directly in a `.roko` directory (`.roko/*`);
/// - a word names a `.roko` directory itself (`cd ~/.roko`) and another
///   ends in a key file's name (`.env`) or is a bare glob (`*`);
/// - a word, or an option's value (`--env-file=…`), resolved against `cwd`
///   is a key file, as [`refuse_key_file`] decides (a symlink to one too).
///
/// # Errors
///
/// Returns [`ToolError::KeyFileBlocked`] naming the word.
pub fn refuse_key_file_in_command(command: &str, cwd: &Path) -> Result<(), ToolError> {
    let mut words = Vec::new();
    command_words(command, 0, &mut words);
    if let Some(word) = words.iter().find(|word| key_path_in_text(word)) {
        return Err(ToolError::KeyFileBlocked(word.into()));
    }
    if key_path_in_text(command) {
        return Err(ToolError::KeyFileBlocked(command.into()));
    }
    if words.iter().any(|word| names_roko_dir(word))
        && let Some(word) = words
            .iter()
            .find(|word| ends_in_key_name(word) || is_bare_glob(word))
    {
        return Err(ToolError::KeyFileBlocked(word.into()));
    }
    for word in &words {
        let value = word.split_once('=').map(|(_, value)| value);
        for candidate in [Some(word.as_str()), value].into_iter().flatten() {
            if let Some(path) = word_path(candidate, cwd) {
                refuse_key_file(&path)?;
            }
        }
    }
    Ok(())
}

/// How deep [`command_words`] reads quoted strings as command lines.
const MAX_COMMAND_NESTING: usize = 8;

/// The words of `text`, and those of each word that is itself a command
/// line (`sh -c '…'`), with quotes and escapes removed.
fn command_words(text: &str, depth: usize, words: &mut Vec<String>) {
    for word in shell_words(text) {
        if depth < MAX_COMMAND_NESTING
            && word.contains(|c: char| c.is_whitespace() || matches!(c, '\'' | '"' | '\\'))
        {
            command_words(&word, depth + 1, words);
        }
        words.push(word);
    }
}

/// Whether `c` ends an unquoted word: whitespace, a shell operator, or a
/// backquote, which starts a command substitution.
const fn ends_word(c: char) -> bool {
    c.is_ascii_whitespace() || matches!(c, ';' | '&' | '|' | '(' | ')' | '<' | '>' | '`')
}

/// The words of `text`, a shell command line, with quotes and escapes
/// removed. A quote left open, as in a heredoc, splits the text at
/// whitespace and operators instead, with the quotes dropped.
fn shell_words(text: &str) -> Vec<String> {
    parse_shell_words(text).unwrap_or_else(|| {
        text.split(ends_word)
            .filter(|word| !word.is_empty())
            .map(|word| word.replace(['\'', '"', '\\'], ""))
            .collect()
    })
}

/// The words of `text`, or `None` when a quote is left open.
fn parse_shell_words(text: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                in_word = true;
                loop {
                    match chars.next()? {
                        '\'' => break,
                        c => word.push(c),
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' => match chars.next()? {
                            '\n' => {}
                            c @ ('$' | '`' | '"' | '\\') => word.push(c),
                            c => {
                                word.push('\\');
                                word.push(c);
                            }
                        },
                        c => word.push(c),
                    }
                }
            }
            '\\' => match chars.next() {
                Some('\n') | None => {}
                Some(c) => {
                    in_word = true;
                    word.push(c);
                }
            },
            c if ends_word(c) => {
                if in_word {
                    words.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            c => {
                in_word = true;
                word.push(c);
            }
        }
    }
    if in_word {
        words.push(word);
    }
    Some(words)
}

/// Whether `c` can be part of a file name next to `.roko` or a key file's
/// name (the guard's `[\w.-]`).
const fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-')
}

/// Whether `text` holds a path to a key file (`.roko/.env`), or a glob
/// directly in a `.roko` directory (`.roko/*`).
fn key_path_in_text(text: &str) -> bool {
    text.match_indices(".roko/").any(|(start, dir)| {
        if text[..start].chars().next_back().is_some_and(is_name_char) {
            return false;
        }
        let rest = &text[start + dir.len()..];
        let names_key_file = KEY_FILE_NAMES.iter().any(|name| {
            rest.strip_prefix(*name).is_some_and(|after| {
                // `.env.local` still starts with `.env`; `.envrc` does not.
                !after
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
            })
        });
        names_key_file
            || rest
                .chars()
                .take_while(|&c| c != '/' && !ends_word(c))
                .any(|c| matches!(c, '*' | '?' | '[' | '{'))
    })
}

/// Whether `word` names a `.roko` directory itself (`~/.roko`, `D=.roko`).
fn names_roko_dir(word: &str) -> bool {
    let normalized = normalize(Path::new(word));
    let text = normalized.to_string_lossy();
    text.strip_suffix(".roko")
        .is_some_and(|before| !before.chars().next_back().is_some_and(is_name_char))
}

/// Whether `word` ends in a key file's name (`.env`, `$D/secrets.toml`).
fn ends_in_key_name(word: &str) -> bool {
    KEY_FILE_NAMES.iter().any(|name| {
        word.strip_suffix(*name)
            .is_some_and(|before| !before.chars().next_back().is_some_and(is_name_char))
    })
}

/// Whether `word` is a glob with no directory in it (`*`, `*.toml`).
fn is_bare_glob(word: &str) -> bool {
    !word.contains('/') && word.contains(['*', '?', '[', '{'])
}

/// Where `word` points as a path: relative to `cwd`, or to `HOME` after
/// `~`. `None` for an empty word, or one the shell would still expand.
fn word_path(word: &str, cwd: &Path) -> Option<PathBuf> {
    if word.is_empty() || word.contains(['$', '`', '*', '?', '[', '{']) {
        return None;
    }
    match word.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => {
            let home = std::env::var_os("HOME").filter(|home| !home.is_empty())?;
            Some(Path::new(&home).join(rest.trim_start_matches('/')))
        }
        _ => Some(cwd.join(word)),
    }
}

/// `path` with symlinks resolved: its deepest existing ancestor
/// canonicalized, and the components below it appended unchanged.
fn resolve_symlinks(path: &Path) -> PathBuf {
    let mut missing = Vec::new();
    let mut existing = path;
    loop {
        if let Ok(canonical) = existing.canonicalize() {
            return missing
                .iter()
                .rev()
                .fold(canonical, |resolved, name| resolved.join(name));
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                missing.push(name);
                existing = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// Extract a required string field from a JSON arguments object.
///
/// # Errors
///
/// Returns [`ToolError::SchemaInvalid`] if `key` is missing or is not a JSON
/// string.
pub fn require_string(args: &serde_json::Value, key: &str) -> Result<String, ToolError> {
    args.get(key)
        .and_then(serde_json::Value::as_str)
        .map_or_else(
            || {
                Err(ToolError::SchemaInvalid(format!(
                    "missing required string argument: {key}"
                )))
            },
            |s| Ok(s.to_string()),
        )
}

/// Lexically normalize a path, collapsing `.` and `..` components.
///
/// Does **not** touch the filesystem — purely syntactic. This matches
/// the behavior of Go's `path.Clean` / Rust's `path-clean` crate.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(p) => out.push(p.as_os_str()),
            Component::RootDir => out.push(std::path::MAIN_SEPARATOR_STR),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    // Can't go above root — keep as-is (caller's check
                    // against the worktree root will reject this).
                    out.push("..");
                }
            }
            Component::Normal(n) => out.push(n),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::tool::{ToolCall, ToolContext, ToolHandler, ToolResult};
    use serde_json::json;

    /// Run the builtin tool `tool` directly, with no `SafetyLayer` in front.
    async fn run_tool(ctx: &ToolContext, tool: &str, arguments: serde_json::Value) -> ToolResult {
        let handler = crate::tool::handler_for(tool).expect("builtin handler");
        ToolHandler::execute(&*handler, ToolCall::new("c", tool, arguments), ctx).await
    }

    #[test]
    fn require_string_extracts_field() {
        let args = serde_json::json!({"path": "foo.rs"});
        assert_eq!(require_string(&args, "path").expect("ok"), "foo.rs");
    }

    #[test]
    fn require_string_rejects_missing_field() {
        let args = serde_json::json!({"other": 1});
        let err = require_string(&args, "path").expect_err("missing");
        assert!(matches!(err, ToolError::SchemaInvalid(_)));
    }

    #[test]
    fn require_string_rejects_wrong_type() {
        let args = serde_json::json!({"path": 42});
        assert!(require_string(&args, "path").is_err());
    }

    #[test]
    fn relative_path_inside_worktree_passes() {
        let worktree = Path::new("/repo");
        let resolved = require_within_worktree(worktree, "src/main.rs").expect("ok");
        assert!(resolved.ends_with("src/main.rs"));
    }

    #[test]
    fn parent_dir_escape_is_rejected() {
        let worktree = Path::new("/repo");
        let err = require_within_worktree(worktree, "../etc/passwd").expect_err("escape");
        assert!(matches!(err, ToolError::PathOutsideWorktree(_)));
    }

    #[test]
    fn absolute_path_outside_worktree_is_rejected() {
        let worktree = Path::new("/repo");
        let err = require_within_worktree(worktree, "/etc/passwd").expect_err("escape");
        assert!(matches!(err, ToolError::PathOutsideWorktree(_)));
    }

    #[test]
    fn current_dir_components_are_normalized() {
        let worktree = Path::new("/repo");
        let resolved = require_within_worktree(worktree, "./src/./foo.rs").expect("ok");
        assert!(resolved.ends_with("src/foo.rs"));
    }

    #[test]
    fn parent_within_worktree_is_allowed() {
        let worktree = Path::new("/repo");
        let resolved = require_within_worktree(worktree, "src/../lib.rs").expect("ok");
        assert!(resolved.ends_with("lib.rs"));
    }

    #[test]
    fn normalize_preserves_unicode_names() {
        let worktree = Path::new("/repo");
        let resolved = require_within_worktree(worktree, "файл.rs").expect("ok");
        assert!(resolved.to_string_lossy().contains("файл.rs"));
    }

    #[test]
    fn commands_naming_key_files_are_recognised() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().canonicalize().expect("canonical tempdir");
        for command in [
            "cat .roko/.env",
            "cat .ro\"\"ko/.e''nv",
            "bash -c \"bash -c 'cat .roko/.e\\\"\\\"nv'\"",
            "D=.roko; cat $D/credentials.json",
            "cd .roko/state/.. && cat ./secrets.toml",
            "x=$(<.roko/.env)",
            "cat `echo .roko/.env`",
            "cat .roko/[cs]*",
            "echo 'a quote left open .roko/.env",
        ] {
            assert!(
                matches!(
                    refuse_key_file_in_command(command, &root),
                    Err(ToolError::KeyFileBlocked(_))
                ),
                "`{command}` should be refused"
            );
        }
        for command in [
            "cargo test --workspace",
            "cat .env .envrc prod.env",
            "cat .roko/state/x.json .env.local",
            "ls .roko && cat README.md",
            "git commit -m \"docs: explain the key files\"",
            "",
        ] {
            assert!(
                refuse_key_file_in_command(command, &root).is_ok(),
                "`{command}` should run"
            );
        }
        assert_eq!(
            shell_words("a 'b c' \"d\\\"e\" f\\ g h;i"),
            ["a", "b c", "d\"e", "f g", "h", "i"]
        );
    }

    #[tokio::test]
    async fn std_file_tools_refuse_key_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().canonicalize().expect("canonical tempdir");
        let secret = "sk-test-not-real";
        let key_file = format!("OPENAI_API_KEY={secret}\n");
        std::fs::create_dir_all(root.join(".roko")).expect("mkdir .roko");
        std::fs::write(root.join(".roko/.env"), &key_file).expect("write .env");
        std::fs::write(root.join(".roko/secrets.toml"), &key_file).expect("write secrets.toml");
        std::fs::write(root.join(".roko/state.json"), "{}").expect("write state.json");
        #[cfg_attr(not(unix), allow(unused_mut))]
        let mut key_paths = vec![
            ".roko/.env",
            "src/../.roko/secrets.toml",
            ".roko/credentials.json",
        ];
        // A symlink to a key file is refused by what it resolves to.
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join(".roko/.env"), root.join("notes.txt"))
                .expect("symlink");
            key_paths.push("notes.txt");
        }
        let ctx = ToolContext::testing(&root);

        for path in key_paths {
            let patch = format!("--- a/{path}\n+++ b/{path}\n@@ -1 +1 @@\n-a\n+b\n");
            for (tool, arguments) in [
                ("read_file", json!({ "path": path })),
                ("write_file", json!({ "path": path, "content": "x" })),
                (
                    "edit_file",
                    json!({ "path": path, "old_string": secret, "new_string": "x" }),
                ),
                (
                    "multi_edit",
                    json!({ "path": path, "edits": [{ "old_string": secret, "new_string": "x" }] }),
                ),
                (
                    "notebook_edit",
                    json!({ "path": path, "cell_index": 0, "source": "x" }),
                ),
                ("apply_patch", json!({ "patch": patch })),
                ("ls", json!({ "path": path })),
            ] {
                let result = run_tool(&ctx, tool, arguments).await;
                assert!(
                    matches!(result, ToolResult::Err(ToolError::KeyFileBlocked(_))),
                    "{tool} {path}: expected KeyFileBlocked, got {result:?}"
                );
            }
        }
        let env = std::fs::read_to_string(root.join(".roko/.env")).expect("read .env");
        assert_eq!(env, key_file, "the key file changed");

        // The rest of .roko stays readable, and grep skips key files unread,
        // in .roko and through the symlink.
        let state = run_tool(&ctx, "read_file", json!({ "path": ".roko/state.json" })).await;
        assert!(matches!(state, ToolResult::Ok { .. }), "{state:?}");
        for arguments in [
            json!({ "pattern": secret, "path": ".roko" }),
            json!({ "pattern": secret }),
        ] {
            let result = run_tool(&ctx, "grep", arguments).await;
            assert!(matches!(result, ToolResult::Ok { .. }), "{result:?}");
            assert!(!result.text_content().contains(secret), "{result:?}");
        }
    }
}
