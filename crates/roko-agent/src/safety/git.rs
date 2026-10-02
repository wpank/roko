//! Git branch-protection policy (§36.49).
//!
//! Gates git operations before they reach the `bash` handler. Every shell
//! command string the dispatcher is about to execute is passed through
//! [`check_git_command_with_policy`] first. Non-git commands are passed
//! through unconditionally; git commands are inspected against the caller's
//! [`GitPolicy`].
//!
//! # React dimensions
//!
//! - **Force push** — `git push --force`, `--force-with-lease`, or `-f` to a
//!   protected branch is rejected when [`GitPolicy::block_force_push`] is
//!   `true`. Branches listed in [`GitPolicy::allow_force_push_on`] are exempt.
//! - **Hard reset** — `git reset --hard` is rejected when
//!   [`GitPolicy::block_hard_reset_on_protected`] is `true` and any protected
//!   branch name appears in the segment (conservative because the running HEAD
//!   cannot be inspected without executing git).
//! - **Branch delete** — `git branch -D/-d <protected>`,
//!   `git push <remote> :<protected>`, and
//!   `git push <remote> --delete <protected>` are rejected when
//!   [`GitPolicy::block_branch_delete_protected`] is `true`.
//! - **Work discarded or branches moved** — `git stash` (except `stash list`
//!   and `stash show`), `git clean` (except dry runs), `git checkout`,
//!   `git switch`, `git restore` and any `git push` are rejected when
//!   [`GitPolicy::block_discarding_commands`] is `true`, as the Claude CLI
//!   hook (`claude_cli_guard.py`) rejects them.
//!
//! # Shell variations handled
//!
//! - Chained commands (`&&`, `||`, `;`, `|`, `&`, newlines) — each segment is
//!   checked independently.
//! - Leading prefixes on a segment (`sudo git …`, `cd /repo && git …`,
//!   `env VAR=x git …`) are stripped before the git subcommand is extracted.
//! - git's global options (`git -C <dir> …`, `git -c <key=value> …`,
//!   `--git-dir=…`, `--no-pager`) are skipped, so the subcommand after them
//!   is the one checked.
//!
//! # Example
//!
//! ```ignore
//! use roko_agent::safety::git::{check_git_command, check_git_command_with_policy, GitPolicy};
//!
//! // Default policy
//! assert!(check_git_command("git status").is_ok());
//! assert!(check_git_command("git push --force origin main").is_err());
//! assert!(check_git_command("git -C repo stash").is_err());
//! assert!(check_git_command("ls -la").is_ok());
//!
//! // Custom: lift the blanket denials and allow force pushes to a release branch
//! let policy = GitPolicy {
//!     allow_force_push_on: vec!["release/2.0".to_string()],
//!     block_discarding_commands: false,
//!     ..GitPolicy::default()
//! };
//! assert!(check_git_command_with_policy(
//!     "git push --force origin release/2.0",
//!     &policy
//! ).is_ok());
//! ```

use roko_core::tool::ToolError;

// ─── React ────────────────────────────────────────────────────────────────

/// Rules governing which git operations are permitted on which branches.
///
/// Construct via [`GitPolicy::default`] for the safe "block all destructive
/// writes to main/master" posture, then override fields as needed.
#[derive(Debug, Clone)]
pub struct GitPolicy {
    /// Branches that are considered protected (default: `["main", "master"]`).
    pub protected_branches: Vec<String>,

    /// Branches that ARE allowed to receive `push --force*` even though
    /// [`block_force_push`](GitPolicy::block_force_push) is set. Empty by
    /// default.
    pub allow_force_push_on: Vec<String>,

    /// If `true` (the default), `git push --force` / `--force-with-lease` /
    /// `-f` to any branch in [`protected_branches`](GitPolicy::protected_branches)
    /// is rejected, unless that branch also appears in
    /// [`allow_force_push_on`](GitPolicy::allow_force_push_on).
    pub block_force_push: bool,

    /// If `true` (the default), `git reset --hard` is rejected whenever any
    /// protected branch name appears anywhere in the same command segment.
    /// Because the current HEAD cannot be determined by pure string analysis,
    /// the check is conservative: even a bare `git reset --hard` (no explicit
    /// ref) is blocked.
    pub block_hard_reset_on_protected: bool,

    /// If `true` (the default), deleting a protected branch via any of:
    /// - `git branch -D <protected>` / `-d <protected>`
    /// - `git push <remote> :<protected>` (refspec colon-delete syntax)
    /// - `git push <remote> --delete <protected>`
    ///
    /// …is rejected.
    pub block_branch_delete_protected: bool,

    /// If `true` (the default), the commands that discard uncommitted work or
    /// move branches are rejected on every branch, with the Claude CLI hook's
    /// exceptions:
    /// - `git stash`, except `stash list` and `stash show`;
    /// - `git clean`, except dry runs (`-n`, `--dry-run`);
    /// - `git checkout`, `git switch` and `git restore`, in every form;
    /// - any `git push`: roko delivers by fast-forward, agents never push.
    ///
    /// A force push or a protected-branch delete is still reported under its
    /// own rule above; with this switch off, those rules alone decide a push.
    pub block_discarding_commands: bool,
}

impl Default for GitPolicy {
    fn default() -> Self {
        Self {
            protected_branches: vec!["main".into(), "master".into()],
            allow_force_push_on: Vec::new(),
            block_force_push: true,
            block_hard_reset_on_protected: true,
            block_branch_delete_protected: true,
            block_discarding_commands: true,
        }
    }
}

/// git's global options that take the next word as their value
/// (`git -C <dir> stash`). The subcommand comes after them.
const GIT_VALUE_OPTIONS: &[&str] = &[
    "-C",
    "-c",
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--super-prefix",
    "--config-env",
    "--attr-source",
];

// ─── Public API ────────────────────────────────────────────────────────────

/// Check whether `command` is permitted under `policy`.
///
/// Returns `Ok(())` when:
/// - The command contains no git segments, **or**
/// - Every git segment satisfies the policy.
///
/// Returns [`ToolError::CommandNotAllowed`] (with a message naming the
/// violated rule and the offending segment) on the first violation.
///
/// # Algorithm
///
/// 1. Split on shell separators (`&&`, `;`, `|`, `&`) to get segments.
/// 2. For each segment, strip leading `sudo `/ `env …` / `cd … && ` prefixes.
/// 3. Skip segments whose first token is not `git`.
/// 4. Extract the git subcommand and apply policy checks.
///
/// # Errors
///
/// Returns [`ToolError::CommandNotAllowed`] if any segment violates the
/// policy.
pub fn check_git_command_with_policy(command: &str, policy: &GitPolicy) -> Result<(), ToolError> {
    for segment in split_shell_segments(command) {
        let trimmed = strip_leading_prefixes(segment.trim());
        check_segment(trimmed, policy)?;
    }
    Ok(())
}

/// Convenience wrapper using [`GitPolicy::default`].
///
/// # Errors
///
/// Returns [`ToolError::CommandNotAllowed`] if the command violates the
/// default policy.
pub fn check_git_command(command: &str) -> Result<(), ToolError> {
    check_git_command_with_policy(command, &GitPolicy::default())
}

// ─── Internal helpers ──────────────────────────────────────────────────────

/// Split `command` on shell separators: `&&`, `||`, `;`, `|`, `&` and
/// newlines.
///
/// Order matters: `&&` must be matched before `&`, and `||` before `|`,
/// so that the two-char operators don't get split as two single-char ones.
/// We do a simple character-by-character scan rather than calling
/// `split_terminator` on a single delimiter.
fn split_shell_segments(command: &str) -> Vec<&str> {
    let bytes = command.as_bytes();
    let len = bytes.len();
    let mut segments: Vec<&str> = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;

    while i < len {
        // Two-character separators first.
        if i + 1 < len {
            let two = &bytes[i..i + 2];
            if two == b"&&" || two == b"||" {
                segments.push(&command[start..i]);
                start = i + 2;
                i += 2;
                continue;
            }
        }
        // Single-character separators.
        if matches!(bytes[i], b';' | b'|' | b'&' | b'\n') {
            segments.push(&command[start..i]);
            start = i + 1;
        }
        i += 1;
    }
    // Trailing segment (may be empty if command ends with a separator).
    segments.push(&command[start..]);
    segments
}

/// Strip common shell prefixes that can precede the actual `git` invocation
/// on a segment that has already been shell-split.
///
/// Patterns stripped (in order, repeatedly until stable):
/// - `sudo ` (single token)
/// - `env KEY=VAL … ` (one or more `KEY=VALUE` tokens)
/// - `cd <path> && ` / `cd <path>; `  (the `cd` command + its separator)
///   — note: this is a best-effort strip for the common `cd /repo && git`
///   pattern; it does not cover arbitrarily nested shells.
fn strip_leading_prefixes(mut s: &str) -> &str {
    loop {
        let before = s;

        // Strip leading `sudo `.
        if let Some(rest) = s.strip_prefix("sudo ") {
            s = rest.trim_start();
            continue;
        }

        // Strip leading `env ` so subsequent assignment tokens can be removed.
        if let Some(rest) = s.strip_prefix("env ") {
            s = rest.trim_start();
            continue;
        }

        // Strip `env KEY=VALUE` tokens (one at a time).
        // An env assignment looks like a token that contains `=` and whose
        // left side has no `/` (to avoid confusing paths like `/usr/bin/env`).
        let first_token_end = s
            .char_indices()
            .take_while(|(_, c)| !c.is_whitespace())
            .last()
            .map_or(0, |(i, c)| i + c.len_utf8());
        let first_token = &s[..first_token_end];
        if first_token.contains('=') && !first_token.contains('/') {
            s = s[first_token_end..].trim_start();
            continue;
        }

        // Strip `cd <path> &&` or `cd <path>;`.
        if s.starts_with("cd ") {
            // Find the separator after `cd <path>`.
            // Look for `&&` or `;` following the path token.
            if let Some(amp_pos) = s.find("&&") {
                s = s[amp_pos + 2..].trim_start();
                continue;
            }
            if let Some(semi_pos) = s.find(';') {
                s = s[semi_pos + 1..].trim_start();
                continue;
            }
        }

        // Nothing more to strip.
        if s == before {
            break;
        }
    }
    s
}

/// Tokenize a (prefix-stripped) segment and apply policy rules.
///
/// Skips immediately if the first token is not `git`.
fn check_segment(segment: &str, policy: &GitPolicy) -> Result<(), ToolError> {
    let tokens: Vec<&str> = segment.split_whitespace().collect();
    if tokens.is_empty() {
        return Ok(());
    }
    if tokens[0] != "git" {
        return Ok(());
    }

    // The subcommand is the first word after git's global options
    // (`git -C <dir> -c <key=value> --no-pager stash`).
    let mut index = 1;
    while index < tokens.len() && tokens[index].starts_with('-') {
        index += if GIT_VALUE_OPTIONS.contains(&tokens[index]) {
            2
        } else {
            1
        };
    }
    let Some(subcommand) = tokens.get(index).copied() else {
        return Ok(()); // bare `git` with no subcommand — pass through
    };

    let rest = &tokens[index + 1..]; // everything after the subcommand

    match subcommand {
        "push" => {
            check_push(segment, rest, policy)?;
            check_discarding(segment, subcommand, rest, policy)
        }
        "reset" => check_reset(segment, rest, policy),
        "branch" => check_branch(segment, rest, policy),
        "stash" | "clean" | "checkout" | "switch" | "restore" => {
            check_discarding(segment, subcommand, rest, policy)
        }
        _ => Ok(()),
    }
}

// ─── Subcommand checkers ───────────────────────────────────────────────────

/// Check a `git push …` invocation.
fn check_push(segment: &str, args: &[&str], policy: &GitPolicy) -> Result<(), ToolError> {
    let has_force = args
        .iter()
        .any(|a| *a == "--force" || *a == "--force-with-lease" || *a == "-f");

    // Detect colon-prefix delete: `git push <remote> :<branch>`.
    // Also detect `--delete <branch>`.
    if policy.block_branch_delete_protected {
        // Colon-delete refspec: any arg starting with `:` (not `::` — that's
        // a different fetch refspec form, but we're conservative).
        for arg in args {
            if let Some(branch) = arg.strip_prefix(':')
                && !branch.is_empty()
                && is_protected(branch, policy)
            {
                return Err(blocked(
                    "block_branch_delete_protected",
                    segment,
                    &format!("colon-delete of protected branch `{branch}`"),
                ));
            }
        }

        // `--delete <branch>` flag.
        if let Some(pos) = args.iter().position(|a| *a == "--delete" || *a == "-d")
            && let Some(branch) = args.get(pos + 1)
            && is_protected(branch, policy)
        {
            return Err(blocked(
                "block_branch_delete_protected",
                segment,
                &format!("--delete of protected branch `{branch}`"),
            ));
        }
    }

    if !policy.block_force_push || !has_force {
        return Ok(());
    }

    // Determine the target branch for the force-push check.
    // Argument layout after `push`: flags interleaved with `[<remote>] [<refspec>]`.
    // The branch is the last non-flag argument. We do a simple positional scan:
    // collect non-flag tokens; if there are ≥2, the last is the branch; if exactly
    // 1, it's ambiguous (could be just a remote — no branch specified). With no
    // branch specified we conservatively block.
    let non_flag_args: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .copied()
        .collect();

    let target_branch: Option<&str> = if non_flag_args.len() >= 2 {
        // `git push <remote> <branch>` — last token is the branch.
        non_flag_args.last().copied()
    } else if non_flag_args.len() == 1 {
        // `git push <remote>` with no branch — we treat it as targeting the
        // default upstream, which could be anything. Conservative: block.
        // We pass `None` so the branch check below treats it as "unknown".
        None
    } else {
        // `git push` with only flags — no remote, no branch. Conservative: block.
        None
    };

    match target_branch {
        Some(branch) => {
            // Strip any refspec src:dst form — we care about the destination.
            let dst = branch.split(':').next_back().unwrap_or(branch);
            if is_protected(dst, policy) && !is_force_allowed(dst, policy) {
                return Err(blocked(
                    "block_force_push",
                    segment,
                    &format!("force push to protected branch `{dst}`"),
                ));
            }
        }
        None => {
            // No explicit branch — conservative block.
            return Err(blocked(
                "block_force_push",
                segment,
                "force push with no explicit branch (potentially targets a protected branch)",
            ));
        }
    }

    Ok(())
}

/// Check a `git reset …` invocation.
fn check_reset(segment: &str, args: &[&str], policy: &GitPolicy) -> Result<(), ToolError> {
    if !policy.block_hard_reset_on_protected {
        return Ok(());
    }
    let has_hard = args.contains(&"--hard");
    if !has_hard {
        return Ok(());
    }
    // Check if any protected branch name appears anywhere in this segment.
    // This is conservative: `git reset --hard main` is blocked, and so is
    // `git reset --hard` (HEAD could be a protected branch).
    let segment_mentions_protected = policy
        .protected_branches
        .iter()
        .any(|b| segment_contains_branch(segment, b));

    // Also block bare `git reset --hard` (no ref given) — can't know the HEAD.
    let no_explicit_ref = args.iter().filter(|a| !a.starts_with('-')).count() == 0;

    if segment_mentions_protected || no_explicit_ref {
        let reason = if no_explicit_ref {
            "bare `--hard` reset (HEAD may be a protected branch)".to_string()
        } else {
            let name = policy
                .protected_branches
                .iter()
                .find(|b| segment_contains_branch(segment, b))
                .map_or("(protected)", String::as_str);
            format!("hard reset mentions protected branch `{name}`")
        };
        return Err(blocked("block_hard_reset_on_protected", segment, &reason));
    }

    Ok(())
}

/// Check a `git branch …` invocation.
fn check_branch(segment: &str, args: &[&str], policy: &GitPolicy) -> Result<(), ToolError> {
    if !policy.block_branch_delete_protected {
        return Ok(());
    }
    // Delete flags: `-D` (force delete) or `-d` (safe delete).
    let has_delete = args.iter().any(|a| *a == "-D" || *a == "-d");
    if !has_delete {
        return Ok(());
    }
    // Branch names are the non-flag tokens after the delete flag.
    let branch_names: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .copied()
        .collect();
    for branch in branch_names {
        if is_protected(branch, policy) {
            return Err(blocked(
                "block_branch_delete_protected",
                segment,
                &format!("branch delete of protected branch `{branch}`"),
            ));
        }
    }
    Ok(())
}

/// Check a command that discards uncommitted work or moves branches
/// (`stash`, `clean`, `checkout`, `switch`, `restore`, `push`), with the
/// Claude CLI hook's exceptions: `stash list`, `stash show` and `clean` dry
/// runs.
fn check_discarding(
    segment: &str,
    subcommand: &str,
    args: &[&str],
    policy: &GitPolicy,
) -> Result<(), ToolError> {
    if !policy.block_discarding_commands {
        return Ok(());
    }
    let reason = match subcommand {
        "stash" if matches!(args.first().copied(), Some("list" | "show")) => return Ok(()),
        "stash" => "git stash can lose uncommitted work (stash list and stash show are allowed)",
        "clean" if is_dry_run(args) => return Ok(()),
        "clean" => "git clean deletes untracked files (dry runs with -n are allowed)",
        "checkout" => "git checkout switches branches or discards changes",
        "switch" => "git switch moves the checkout to another branch",
        "restore" => "git restore discards uncommitted changes",
        "push" => "agents must not push: roko delivers the work itself",
        _ => return Ok(()),
    };
    Err(blocked("block_discarding_commands", segment, reason))
}

// ─── Utilities ─────────────────────────────────────────────────────────────

/// True if `git clean` arguments ask for a dry run: `--dry-run`, or `n` in a
/// short-option cluster (`-n`, `-nd`) before any `--`.
fn is_dry_run(args: &[&str]) -> bool {
    args.iter()
        .take_while(|arg| **arg != "--")
        .any(|arg| *arg == "--dry-run" || is_short_cluster_with(arg, 'n'))
}

/// True if `arg` is a short-option cluster (`-fdx`) that holds `letter`.
fn is_short_cluster_with(arg: &str, letter: char) -> bool {
    arg.len() > 1 && arg.starts_with('-') && !arg.starts_with("--") && arg.contains(letter)
}

/// True if `branch` is in `policy.protected_branches`.
fn is_protected(branch: &str, policy: &GitPolicy) -> bool {
    policy.protected_branches.iter().any(|b| b == branch)
}

/// True if force-push is explicitly allowed on `branch`.
fn is_force_allowed(branch: &str, policy: &GitPolicy) -> bool {
    policy.allow_force_push_on.iter().any(|b| b == branch)
}

/// True if `segment` contains `branch` as a whitespace-delimited token.
///
/// Whole-word check prevents `main` from matching `maintenance`.
fn segment_contains_branch(segment: &str, branch: &str) -> bool {
    segment.split_whitespace().any(|tok| tok == branch)
}

/// Construct a [`ToolError::CommandNotAllowed`] with a consistent format:
/// `[<rule>] <reason> — in segment: <segment>`.
fn blocked(rule: &str, segment: &str, reason: &str) -> ToolError {
    ToolError::CommandNotAllowed(format!("[{rule}] {reason} — in segment: `{segment}`"))
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ─────────────────────────────────────────────────────────────

    fn assert_blocked(cmd: &str) {
        let res = check_git_command(cmd);
        assert!(
            matches!(res, Err(ToolError::CommandNotAllowed(_))),
            "expected `{cmd}` to be blocked, got {res:?}"
        );
    }

    fn assert_allowed(cmd: &str) {
        let res = check_git_command(cmd);
        assert!(res.is_ok(), "expected `{cmd}` to be allowed, got {res:?}");
    }

    fn assert_blocked_with_policy(cmd: &str, policy: &GitPolicy) {
        let res = check_git_command_with_policy(cmd, policy);
        assert!(
            matches!(res, Err(ToolError::CommandNotAllowed(_))),
            "expected `{cmd}` to be blocked under custom policy, got {res:?}"
        );
    }

    fn assert_allowed_with_policy(cmd: &str, policy: &GitPolicy) {
        let res = check_git_command_with_policy(cmd, policy);
        assert!(res.is_ok(), "expected `{cmd}` to be allowed, got {res:?}");
    }

    /// The default policy without the blanket push denial, so the force-push
    /// and branch-delete rules decide a push.
    fn pushes_permitted() -> GitPolicy {
        GitPolicy {
            block_discarding_commands: false,
            ..GitPolicy::default()
        }
    }

    // ── Test 1: force push to main blocked ───────────────────────────────────

    #[test]
    fn force_push_to_main_blocked() {
        assert_blocked("git push --force origin main");
    }

    // ── Test 2: force push to master blocked ────────────────────────────────

    #[test]
    fn force_push_to_master_blocked() {
        assert_blocked("git push --force origin master");
    }

    // ── Test 3: force push to feature branch allowed ────────────────────────

    #[test]
    fn force_push_to_feature_branch_allowed() {
        assert_allowed_with_policy(
            "git push --force origin feature/my-change",
            &pushes_permitted(),
        );
    }

    // ── Test 4: --force-with-lease to main blocked ──────────────────────────

    #[test]
    fn force_with_lease_to_main_blocked() {
        assert_blocked("git push --force-with-lease origin main");
    }

    // ── Test 5: short flag -f to main blocked ───────────────────────────────

    #[test]
    fn short_flag_f_to_main_blocked() {
        assert_blocked("git push -f origin main");
    }

    // ── Test 6: force push to main allowed when branch is on allow list ─────

    #[test]
    fn force_push_to_main_allowed_when_in_allow_list() {
        let policy = GitPolicy {
            allow_force_push_on: vec!["main".to_string()],
            ..pushes_permitted()
        };
        assert_allowed_with_policy("git push --force origin main", &policy);
    }

    // ── Test 7: force push with no explicit branch is conservatively blocked ─

    #[test]
    fn force_push_flag_without_branch_arg_safe() {
        // `git push --force` with no remote/branch — conservative: block.
        assert_blocked("git push --force");
    }

    // ── Test 8: force push allowed when flag disabled ────────────────────────

    #[test]
    fn force_push_to_main_allowed_when_flag_disabled() {
        let policy = GitPolicy {
            block_force_push: false,
            ..pushes_permitted()
        };
        assert_allowed_with_policy("git push --force origin main", &policy);
    }

    // ── Test 9: hard reset with protected branch name blocked ────────────────

    #[test]
    fn hard_reset_with_protected_name_blocked() {
        assert_blocked("git reset --hard main");
    }

    // ── Test 10: soft reset is not blocked ───────────────────────────────────

    #[test]
    fn soft_reset_not_blocked() {
        assert_allowed("git reset --soft main");
    }

    // ── Test 11: git branch -D main blocked ──────────────────────────────────

    #[test]
    fn branch_capital_d_delete_main_blocked() {
        assert_blocked("git branch -D main");
    }

    // ── Test 12: git push origin :main blocked ───────────────────────────────

    #[test]
    fn push_colon_delete_main_blocked() {
        assert_blocked("git push origin :main");
    }

    // ── Test 13: git push origin --delete main blocked ───────────────────────

    #[test]
    fn push_delete_flag_main_blocked() {
        assert_blocked("git push origin --delete main");
    }

    // ── Test 14: git branch -d feature allowed ───────────────────────────────

    #[test]
    fn branch_delete_feature_allowed() {
        assert_allowed("git branch -d feature/my-change");
    }

    // ── Test 15: non-git command passes through ───────────────────────────────

    #[test]
    fn non_git_command_passes() {
        let res = check_git_command("ls -la").expect("ls -la should pass through git policy");
        let _ = res; // res is () — just confirming Ok
    }

    // ── Test 16: sudo prefix stripped ────────────────────────────────────────

    #[test]
    fn sudo_prefix_stripped() {
        assert_blocked("sudo git push --force origin main");
    }

    // ── Test 17: cd prefix stripped ──────────────────────────────────────────

    #[test]
    fn cd_prefix_stripped() {
        assert_blocked("cd /repo && git push --force origin main");
    }

    // ── Test 18: env prefix stripped ─────────────────────────────────────────

    #[test]
    fn env_prefix_stripped() {
        assert_blocked("env GIT_SSH_COMMAND=ssh git push --force origin main");
    }

    // ── Test 19: chained command — second segment checked ────────────────────

    #[test]
    fn chained_command_each_segment_checked() {
        // First segment is safe; the second violates the policy.
        assert_blocked("git status && git push --force origin main");
    }

    // ── Test 20: default policy values are sane ───────────────────────────────

    #[test]
    fn default_policy_values_sane() {
        let p = GitPolicy::default();
        assert!(
            p.protected_branches.contains(&"main".to_string()),
            "default policy must protect main"
        );
        assert!(
            p.protected_branches.contains(&"master".to_string()),
            "default policy must protect master"
        );
        assert!(
            p.allow_force_push_on.is_empty(),
            "allow list must start empty"
        );
        assert!(p.block_force_push, "block_force_push must default to true");
        assert!(
            p.block_hard_reset_on_protected,
            "block_hard_reset_on_protected must default to true"
        );
        assert!(
            p.block_branch_delete_protected,
            "block_branch_delete_protected must default to true"
        );
        assert!(
            p.block_discarding_commands,
            "block_discarding_commands must default to true"
        );
    }

    // ── Test 21: custom protected branches list ───────────────────────────────

    #[test]
    fn custom_protected_branches_list() {
        let policy = GitPolicy {
            protected_branches: vec!["develop".to_string(), "staging".to_string()],
            ..pushes_permitted()
        };
        // develop is now protected
        assert_blocked_with_policy("git push --force origin develop", &policy);
        // main is no longer protected under this custom policy
        assert_allowed_with_policy("git push --force origin main", &policy);
    }

    // ── Additional edge-case tests ────────────────────────────────────────────

    #[test]
    fn git_status_always_allowed() {
        assert_allowed("git status");
        assert_allowed("git log --oneline");
        assert_allowed("git diff HEAD");
    }

    #[test]
    fn push_to_feature_no_force_allowed() {
        assert_allowed_with_policy("git push origin feature/new-ui", &pushes_permitted());
    }

    #[test]
    fn hard_reset_bare_conservatively_blocked() {
        // No ref given — we can't know if HEAD is protected, so block.
        assert_blocked("git reset --hard");
    }

    #[test]
    fn hard_reset_on_feature_branch_allowed() {
        // Non-protected name — hard reset is allowed.
        assert_allowed("git reset --hard feature/some-work");
    }

    #[test]
    fn push_colon_delete_feature_allowed() {
        assert_allowed_with_policy("git push origin :feature/cleanup", &pushes_permitted());
    }

    #[test]
    fn error_message_contains_rule_and_segment() {
        let res = check_git_command("git push --force origin main");
        match res {
            Err(ToolError::CommandNotAllowed(msg)) => {
                assert!(
                    msg.contains("block_force_push"),
                    "error must name the violated rule; got: {msg}"
                );
                assert!(
                    msg.contains("git push --force origin main"),
                    "error must include the offending segment; got: {msg}"
                );
            }
            other => panic!("expected CommandNotAllowed, got {other:?}"),
        }
    }

    #[test]
    fn semicolon_chain_each_segment_checked() {
        assert_blocked("echo hi; git push --force origin main");
    }

    #[test]
    fn pipe_chain_each_segment_checked() {
        // git reset --hard after a pipe — still caught.
        assert_blocked("git log | git reset --hard main");
    }

    // ── Work discarded or branches moved (the Claude CLI hook's rules) ──────

    #[test]
    fn default_git_policy_denies_stash_clean_checkout_restore() {
        for cmd in [
            "git stash",
            "git stash push -m x",
            "git clean -fdx",
            "git checkout -- .",
            "git restore .",
            "git switch main",
            "git -C . stash",
            "cd sub && git clean -fd",
            "git -c core.pager=cat --no-pager clean -f",
            "git --git-dir=.git --work-tree . checkout -b topic",
            "git status\ngit stash",
        ] {
            assert_blocked(cmd);
        }
        for cmd in [
            "git status",
            "git diff",
            "git stash list",
            "git stash show -p",
            "git clean -n",
            "git clean --dry-run -d",
            "git commit -m x",
            "git log",
            "git -C sub status",
        ] {
            assert_allowed(cmd);
        }

        let Err(ToolError::CommandNotAllowed(message)) = check_git_command("git -C . stash") else {
            panic!("`git -C . stash` must be denied");
        };
        assert!(
            message.contains("block_discarding_commands"),
            "the denial must name its rule; got: {message}"
        );
    }

    #[test]
    fn default_git_policy_denies_any_push() {
        for cmd in [
            "git push",
            "git push origin feature/new-ui",
            "git -C sub push origin HEAD",
        ] {
            assert_blocked(cmd);
        }
    }

    #[test]
    fn discarding_commands_allowed_when_switch_off() {
        let policy = pushes_permitted();
        for cmd in [
            "git stash",
            "git clean -fdx",
            "git checkout -- .",
            "git push origin topic",
        ] {
            assert_allowed_with_policy(cmd, &policy);
        }
    }
}
