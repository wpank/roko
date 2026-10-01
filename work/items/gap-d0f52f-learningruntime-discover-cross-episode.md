+++
id = "gap-d0f52f"
kind = "gap"
title = "LearningRuntime::discover_cross_episode_patterns has no caller, so EpisodeView::succeeded has no production reader"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-learn/runtime_feedback", "roko-learn/pattern_discovery"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-settle's report on gap-88c547, branch work/gap-88c547 at 24b23580d)"
anchors = ["crates/roko-learn/src/runtime_feedback/mod.rs", "crates/roko-learn/src/pattern_discovery.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = ["gap-88c547"], blocks = [], related = ["gap-88c547"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rq 'fn discover_cross_episode_patterns' crates/ || grep -rn --include='*.rs' 'discover_cross_episode_patterns(' crates/ | grep -v 'fn discover_cross_episode_patterns' | grep -q ."
+++

## Problem

On gap-88c547's branch, `LearningRuntime::discover_cross_episode_patterns` (`crates/roko-learn/src/runtime_feedback/mod.rs:1111`) has no caller. The `EpisodeView` trait's `succeeded` (`pattern_discovery.rs:57`, implemented for the runtime's episodes at `runtime_feedback/mod.rs:127`) is read only through that discovery, so it has no production reader either. The branch updated both to the settled label, but nothing runs them.

## Why it matters

Hygiene (epic spec-9a3131): dead code that looks wired gets maintained, and misleads readers about which learning loops run.

## Where

The two anchors.

## Plan

Pick one:

- **(a) Wire it:** call pattern discovery from the offline consolidation or dream cycle, where cross-episode patterns belong.
- **(b) Delete it:** remove `discover_cross_episode_patterns`, and the `EpisodeView` implementation if nothing else uses it.

## Done when

- [ ] The function is called from production code, or it is gone.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on gap-88c547's branch.
