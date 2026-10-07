+++
id = "gap-b0635e"
kind = "gap"
title = "An attempt worktree that doesn't ignore .roko/ could commit roko's own learning log alongside the agent's work"
status = "superseded"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "f74890b4b"
source = "wave-6 follow-up reports 2026-10-03 (gate 6b)"
discovered_from = "gate 6b (PK03's conflict_retry_prompt_names_the_conflict as a reference test; graph_task_dispatch.rs::recording_feedback as the risky pattern)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::recording_feedback", "crates/roko-cli/src/graph_execution/plan_runner.rs::build_graph_feedback_context", "crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs::accept_attempt"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "bug-412a5e" }

[[verify]]
command = "grep -rqw 'fn worktree_attempt_never_commits_roko_learn_files' crates/roko-cli/ && cargo test -p roko-cli worktree_attempt_never_commits_roko_learn_files"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T04:29:28Z"
forced = false
evidence = "Superseded by bug-412a5e, filed independently minutes later by a concurrent agent on the same gate-6b finding. This item correctly ruled out the attempt-level GraphFeedbackContext/build_graph_feedback_context path (rooted correctly at the workspace) but left the finding 'unconfirmed as an active production bug' because it didn't trace the actual culprit: the separate model-call-level write in dispatch_v2.rs::record_agent_dispatch_feedback (ModelCallFeedbackRecorder, not GraphFeedbackContext). bug-412a5e found and confirmed that mechanism, and I (second concurrent agent) independently confirmed the missing link this item's Plan step 2 asked for: graph_task_dispatch.rs:1008-1010's effective_workdir is explicitly 'worktree path if isolated, else shared workdir' (lease.path.clone() when isolated), and that effective_workdir is what becomes AgentDispatchRequest.workdir (lines 1164, 1288) that record_agent_dispatch_feedback then joins '.roko/learn' onto. So the bug is real and active for isolated attempts, resolving this item's open question. Its tracing of accept_attempt's commit/stage behavior (Plan step 1, not finished) and the recording_feedback doc-comment tightening (Notes) are still worth doing but belong under bug-412a5e now."
+++

## Problem

Gate 6b's finding names `conflict_retry_prompt_names_the_conflict`
(`crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs:1123`) as reproducing two sibling
attempts, in per-task worktrees of a repo that does not ignore `.roko/`, conflicting on
`.roko/learn/efficiency.jsonl` as well as on `same.txt`. As the test is **currently written**, it
builds its dispatcher with `GraphFeedbackContext::default()` (line 1133), whose `efficiency_path`
is `None` — and `feedback.rs:306`'s `if let Some(eff_path) = &self.feedback.efficiency_path` means
no efficiency row is written at all when it is `None`. So this exact test, as it stands today,
cannot be reproducing an efficiency.jsonl conflict; either gate 6b ran a variant of it with a
realistic feedback context wired in, or the finding is about the general *pattern*, not this literal
test run. That pattern is real and already encoded as a widely-used test helper:
`recording_feedback(workdir: &Path)` (`graph_task_dispatch.rs:2744-2756`) builds
`efficiency_path: Some(workdir.join(".roko/learn/efficiency.jsonl"))`, i.e. **keyed on whatever
`workdir` the caller passes** — and every call site of `recording_feedback` I found
(`failover.rs:1289,1998,2103,2216`, `served_model.rs:299,351,403,461`, `reflex_credit.rs:117`) is
in test code passing a per-attempt worktree path as `workdir`.

The real production constructor, `build_graph_feedback_context` (`graph_execution/plan_runner.rs:2335-2391`),
is the one actual plan runs use (wired at `plan_runner.rs:1427`, `.with_feedback(graph_feedback)`,
alongside `GraphTaskDispatcher::new(..., workdir.to_path_buf())` at line 1414): it takes a single
`workdir` set once per plan run — the plan's root workspace, not a per-attempt worktree — and
resolves `efficiency_path` via `RokoLayout::for_project(workdir).learn_dir()`. On the read of this
one function, production is NOT rooting the efficiency path at a worktree.

## Why it matters

If any production code path *does* end up constructing a `GraphFeedbackContext` (or otherwise
writing a `.roko/learn/*` file) keyed on a per-attempt worktree rather than the plan's root
workspace, two bad outcomes follow depending on whether the worktree's own repo ignores `.roko/`:
either the file is committed into the plan branch as part of the attempt's "accepted" changes
(polluting the plan's history with roko's own telemetry, and risking spurious conflicts between
sibling attempts exactly as the named test demonstrates for `same.txt`), or it is silently lost
when the worktree is torn down (a hole in the learning signal). Either way this is worth nailing
down and guarding against, even though I could not confirm an actively-firing production bug in
the one hour I spent tracing it.

## Where

- Test demonstrating the conflict mechanism in general: `attempt_workspace.rs::conflict_retry_prompt_names_the_conflict`
  (as written, uses `GraphFeedbackContext::default()` — does not itself write efficiency.jsonl; a
  variant using `recording_feedback` on a worktree path would).
- The risky pattern: `graph_task_dispatch.rs::recording_feedback` (test helper, 2744-2756), keyed on
  its caller's `workdir` argument.
- The real production constructor, which I confirmed roots correctly at the plan's workspace, not
  a worktree: `graph_execution/plan_runner.rs::build_graph_feedback_context` (2335-2391), wired at
  `plan_runner.rs:1427`.
- The actual commit/accept path for an attempt's worktree: `attempt_workspace.rs::accept_attempt`
  (55-130) delegates to `graph_execution/workspaces.rs`'s `accept` (228-276), which delegates to
  `self.manager.accept_attempt(...)` — I did not finish tracing into the worktree manager itself to
  see exactly what gets staged/committed (e.g. whether it's a targeted diff of known-changed paths
  or a broad `git add -A`-style sweep that would pick up any stray untracked file sitting in the
  worktree, `.roko/` included, regardless of which writer put it there).

## Current state

Unconfirmed as an active production bug from my own reading; confirmed as a real pattern in test
helper code that, if it leaked into (or already matches) any production code path I didn't find,
would reproduce exactly what gate 6b describes. The named test does not demonstrate it as currently
written.

## Plan

1. Finish the trace I started: read the worktree manager's `accept_attempt`/commit implementation
   (follow from `graph_execution/workspaces.rs:257`'s `self.manager.accept_attempt(...)`) to see
   whether it stages a specific, known changeset or sweeps the whole worktree — this determines
   whether a stray `.roko/learn/*` file would ever actually reach a commit even if nothing writes
   there on purpose.
2. Audit every production (non-test) construction of `GraphFeedbackContext` for whether any of them
   ever receives a per-attempt worktree path as `workdir`/`efficiency_path`'s base, rather than
   reusing the dispatcher's one root-level context. `build_graph_feedback_context` looks correct;
   confirm there is no second production constructor.
3. As defense in depth regardless of (1)/(2)'s answer: either have the attempt-worktree provider
   explicitly exclude `.roko/` from what it stages/commits (a `.git/info/exclude` entry or
   equivalent in each worktree, independent of the outer repo's own `.gitignore`), or confirm
   `recording_feedback`-style per-worktree paths are test-only and will never appear in production,
   and say so explicitly in a comment at `recording_feedback`'s definition so a future caller
   doesn't copy the pattern into real code.
4. Write a test that actually reproduces the finding as literally described (two sibling attempts,
   worktrees of a repo that does not ignore `.roko/`, a realistic feedback context such as
   `recording_feedback` wired to each attempt's own worktree) and confirm whether it conflicts on
   `.roko/learn/efficiency.jsonl` the way gate 6b reported — this is the fastest way to settle
   whether this is latent-pattern risk or an active bug.

## Done when

- The worktree commit/accept path is confirmed to either exclude `.roko/` or never receive a
  learning-log file inside the worktree in the first place.
- A test demonstrating (or ruling out) the exact scenario gate 6b described exists and passes.
- The `[[verify]]` command passes.

## Notes

- I did not run cargo or any test; everything above is from reading source at HEAD.
- `recording_feedback`'s doc comment ("Every record file a Graph attempt writes, under
  `workdir/.roko`") is itself ambiguous about which `workdir` it means — worth tightening once (2)
  is answered.
