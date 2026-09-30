+++
id = "gap-6d172d"
kind = "gap"
title = "The Graph pre-verify screen doesn't run SafetyLayer::post_dispatch_check"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-agent/safety"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-tamper's report, branch work/gap-b72761 at 7531304ca)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/red_flags.rs", "crates/roko-agent/src/safety/mod.rs"]
lane = "rust-cold"
parent = "spec-9230a9"
links = { depends_on = ["gap-b72761"], blocks = [], related = ["gap-b72761", "bug-809e22"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn the_pre_verify_screen_runs_post_dispatch_check' crates/roko-cli/src/ && cargo test -p roko-cli --lib the_pre_verify_screen_runs_post_dispatch_check"
+++

## Problem

gap-b72761's branch adds a screen that runs before the gates, `screen_attempt` (`graph_task_dispatch/red_flags.rs:67` on the branch). It rejects empty diffs and malformed or overlong output. `SafetyLayer::post_dispatch_check` (`roko-agent/src/safety/mod.rs`), which checks an attempt's result against the role's safety contract, is called only from roko-acp (`bridge_events/mod.rs:914`, `runner.rs:1643`) and the safety tests. The Graph path never runs it.

## Why it matters

Check each attempt's diff for tampering and scope (epic spec-9230a9): the Graph path is where plans run, and it skips the post-dispatch safety check that ACP runs.

## Where

`screen_attempt` in `red_flags.rs`, and `post_dispatch_check` in the safety layer.

## Plan

1. Call `post_dispatch_check` from `screen_attempt`, with the attempt's plan, task, role and output. Turn its violations into screen failures with a reason.
2. Add `the_pre_verify_screen_runs_post_dispatch_check`.

## Done when

- [ ] A Graph attempt that violates its role's post-dispatch contract is stopped before the gates, with the violation recorded.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on gap-b72761's branch.
- **wk-tamper (2026-09-30):** Implemented on `work/gap-6d172d` at `523d1c393`, narrowed at `d9b524daa`; cargo
  verification deferred to the batch check. `screen_attempt` runs `post_dispatch_check` under the contract the
  attempt ran with (`effective_agent_contract`). It sees the output and the task's changed paths from the pre-verify
  diff, after the output checks and before the tamper, scope and no-changes checks. It rejects an attempt as
  `pre_verify:safety` for a changed path outside the tree, or a changed file under a role that forbids writes. A
  secret rejects it only in an exact format: the new `scrub::high_confidence_secret`, default patterns 1-8 (vendor
  key prefixes, JWTs, private-key blocks). A match of the `NAME = value` rule alone is logged and added to the
  feedback of any later rejection, and does not block; code such as `let token = next()` or
  `api_key: Option<String>` produces such matches. ACP's `post_dispatch_check` and `scrub_secrets` are unchanged.
  Tests: `the_pre_verify_screen_runs_post_dispatch_check` (a reviewer that writes a file; a `ghp_` token),
  `a_name_value_secret_match_is_recorded_without_blocking`, and `high_confidence_secrets_leave_out_name_value_code`
  in scrub.rs. Caveat: the check flags any changed file, new ones included, for roles that forbid only `edit_file`
  (researcher, strategist); no plan in `plans/` uses them.
