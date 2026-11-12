+++
id = "gap-9d7381"
kind = "gap"
title = "An attempt's prompt doesn't say its build trees are prebuilt, so an agent may delete or reconfigure them and throw the warm build away (phase 0 run 2: rm -rf build/plugin, 101.7 s; warm first build 12.5 s against 217 s cold)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "5495afea9"
source = "round-v7-roko-ladder#warm-hint"
discovered_from = "agent:roko-ladder (round v7 warm-build check, roko c25301eef)"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/dispatch/mod.rs::DispatchContext"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_checkout_with_warm_build_trees_is_told_to_keep_them' crates/ && cargo test -p roko-cli --lib -- a_checkout_with_warm_build_trees_is_told_to_keep_them"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T17:31:41Z"
commit = "5495afea9"
forced = false
evidence = "Test a_checkout_with_warm_build_trees_is_told_to_keep_them (two trees, one, none, and the minimal-context roles): red before the section existed, green after. roko-cli lib 3924 passed, 0 failed, 7 ignored (share test skipped); dispatch_feedback_projection_e2e 3 passed; loop_audit_faults checked with --features fault-injection; clippy -D warnings and nightly fmt clean. $0 stand-in run on a clone of Undertow a8031ad with build/plugin built (Undertow's roko.toml, stand-in CLI models, [runner] log_prompts on, two worktree tasks): each checkout got build/plugin warm, and both attempts' prompts carried, right after # Verify: '# Prebuilt build trees / `build/plugin` is already built in this checkout: build in it incrementally, and don't delete it or reconfigure it from scratch (`rm -rf`, `cmake --fresh`), which rebuilds everything.' With [gates] warm_build_dirs = [] no tree was given and neither prompt had the line. Diffed per task, the logged prompts differ only by those two lines."
+++

## Problem

gap-6754ab (22667b513) gives each attempt checkout a warm, relocated copy of the repository checkout's CMake build
trees, but nothing in the agent's prompt says so. The only mention is the log line in
`crates/roko-cli/src/orchestrator/worktree/build_trees.rs:250`. An agent that doesn't know its tree is prebuilt
may delete it or configure it afresh, and throw the warm build away:
- In round v7's phase 0 run 2, an agent ran `rm -rf build/plugin && tools/spec.sh 0 plugin`: 101.7 s, its longest
  call. In phase 0 run 1, T3's mid try 2 spent 802 of 1,246 s (64%) on cold builds.
- Measured on Undertow at a8031ad with roko c25301eef ($0, stand-in agents, the cpp-proof lane):
  - an attempt's first `cmake --build --preset plugin` on its warm tree: 12.4 to 12.7 s, 1 object compiled, no JUCE
    module;
  - `tools/spec.sh 0 plugin` (configure, then build) on it: 17.7 s, 1 object;
  - the cold build of the same tree in a fresh clone: 217.4 s, 70 objects, 51 of them JUCE modules.

Phase 4 is seven tasks on the JUCE editor. A deleted tree costs about 205 s per try.

## Why it matters

The warm tree saves about 94% of a try's first build, and only the agent can throw that away. One short line in
the prompt, shown only when the checkout got trees, protects it.

## Where

- `crates/roko-cli/src/dispatch/prompt_builder.rs`: the task prompt's sections; this adds one beside "Plans Running
  Beside This One".
- `crates/roko-cli/src/dispatch/mod.rs::DispatchContext`: carries the checkout's warm trees to prompt assembly.
- `crates/roko-cli/src/graph_task_dispatch.rs`: dispatch fills it from the attempt's lease, with the same
  `warm_build_dirs` the provider gave the trees by (`[gates] warm_build_dirs`, or every CMake build tree).

## Plan

1. `DispatchContext` (and the prompt context) carry `warm_build_trees`: the build trees the attempt's own checkout
   holds, listed by `crate::orchestrator::worktree::warm_build_dirs(checkout, [gates] warm_build_dirs)`. They are
   empty without a checkout of its own.
2. Prompt assembly adds one short line when it is non-empty. It names the trees (e.g. `build/core`,
   `build/plugin`), says they are prebuilt for this checkout and to build incrementally in them, and not to delete
   them or configure them afresh, which rebuilds everything. No line otherwise.
3. Tests both ways, plus a $0 stand-in run that shows the line in a real attempt's prompt.

## Done when

A task prompt names the checkout's warm build trees in one line when it has some, and has no such line when it has
none. Checked by the test in `[[verify]]`, and shown in a stand-in run's prompt.

## Notes

- Measured in `tmp/hermes-roko-v7/findings/roko-ladder.md` ("Is an attempt's first plugin build warm?").
- Asked by the lead on 2026-10-20 before phase 4.
