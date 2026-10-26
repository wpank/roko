+++
id = "gap-dc24dd"
kind = "gap"
title = "The post-merge regression checkout gets no warm build trees from the person's checkout, so a workspace's first delivery check builds every CMake tree cold: roko-ladder's 5.1 rehearsal built core there from nothing while its attempt worktrees all got warm core and plugin trees"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 roko-speed, the lead's question on roko-ladder's 5.1 rehearsal (tmp/hermes-roko-v7/roko-ladder/rehearsals/p51)"
discovered_from = "agent:roko-speed (round v7)"
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs::reset_regression_checkout", "crates/roko-cli/src/orchestrator/worktree/build_trees.rs::give_build_trees"]
links = { depends_on = [], blocks = [], related = ["gap-6754ab", "gap-b6b7a1", "bug-8cf581"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn the_regression_checkout_starts_with_the_workdirs_build_trees' crates/roko-cli/src && cargo test -p roko-cli --lib the_regression_checkout_starts_with_the_workdirs_build_trees"
+++

## Problem

A plan's post-merge regression check runs in the workspace's regression checkout
(`.roko/state/regression-checkout`, bug-8cf581). roko creates that checkout with `git worktree add` and resets it
for each delivery with `checkout --force` and `clean -ffdq`. It never gives it the person's checkout's build trees,
as gap-6754ab does for every attempt worktree. So the first delivery check in a workspace builds every CMake tree
cold. Later deliveries reuse what earlier ones built there: the checkout stays, and `clean -ffdq` keeps ignored
`build/`.

Seen in roko-ladder's phase 5.1 rehearsal (workspace `tmp/hermes-roko-v7/roko-ladder/rehearsals/p51/ws`, binary
74c3594bc):
- The person's checkout had warm `build/core` and `build/plugin` from setup (setup built them in 173 s and 155 s
  through the cpp-proof lane).
- The regression checkout, created at the first delivery, had neither. check.sh built `build/core` there cold: 47
  objects, from 17:53:17 to 17:53:23. It rendered `build/demos`, then failed before it built the plugin, because of
  a check.sh order bug of undertow-spec's.
- Undertow's live regression checkout has both trees only because phases 0 to 4 built them there over time.
- The log shows no warm copy for the regression checkout. Every attempt worktree of the same run got both trees.

## Why it matters

Golden path, speed. On a C++ project, a workspace's first delivery check pays a full cold build: Undertow's core
and JUCE plugin, minutes through the cpp lane. That is the cost gap-6754ab removed from attempts. A fresh
workspace, or one whose regression checkout was removed, pays it again.

## Where

- `crates/roko-cli/src/graph_execution/delivery.rs::reset_regression_checkout`: the `git worktree add` branch, where
  the checkout is made.
- `crates/roko-cli/src/orchestrator/worktree/build_trees.rs::give_build_trees`: the clone and relocation attempt
  worktrees get. It already skips a tree the checkout has, and honours `[gates] warm_build_dirs`.

## The counter-case: a cold check finds what warm trees hide

This rehearsal is exactly where a cold regression checkout earned its keep (the lead, 2026-10-26).
- Built from nothing, check.sh failed on its own step order.
- Undertow's warm, persistent trees had hidden that from phase 0 through phase 5: the plugin tree was already there
  when its step ran.
- A delivery check run only on warm trees can pass on build state no clean checkout reproduces.

So the shape is **warm by default for speed, plus a way to force a cold delivery check**.

## Plan

1. **Warm by default.**
   - After `reset_regression_checkout` adds or resets the checkout, give it the person's checkout's build trees with
     `give_build_trees(workdir, checkout, warm_build_dirs)`, on a blocking thread as the workspace provider does.
   - A tree the checkout already has, from an earlier delivery, is left alone, as `give_build_trees` does.
   - So this costs only the first delivery's clone, about 1 s a tree on APFS.
   - Log the trees given, as for attempt worktrees.
2. **A cold check on demand.** One switch makes a delivery check start from no build trees at all: none given, and the
   checkout's own removed (`git clean -ffdxq`, or a fresh checkout). Either:
   - a config key such as `[delivery] cold_regression = true`, with a per-plan `[meta]` override, or
   - a `plan run` flag (`--cold-delivery-check`), for a ship phase.
   Pick whichever fits how plans are run. The run's records and the delivery's evidence log say which mode the
   check ran in.
3. **Which one a plan's final delivery uses** must be stated, in the docs and the delivery evidence:
   - By default: warm, like every other delivery.
   - A ship phase sets cold.
   - Recommended for Undertow: its last phase, or any plan whose `[meta]` marks it as shipping, runs cold. That is
     where a step-order or missing-artifact bug in the project's own check script must show up.

## Done when

- A new regression checkout holds the person's checkout's build trees, relocated to its own path, before its first
  command runs. Test: `the_regression_checkout_starts_with_the_workdirs_build_trees`.
- A delivery check can be forced cold, from no build trees, and its record says which mode it ran in.
- The docs say which mode a plan's final delivery uses.
- The `[[verify]]` passes.

## Notes

- Found by the lead in roko-ladder's 5.1 rehearsal; roko-speed traced it (round v7). The check.sh order bug it
  exposed is undertow-spec's.
- Related: gap-6754ab (attempt worktrees' warm trees), gap-b6b7a1 (dependents' warm trees), bug-8cf581 (the
  regression checkout).
