+++
id = "gap-2d0a4d"
kind = "gap"
title = "The prompt's learned patterns from prior episodes are other plans' failures cut at 420 characters, mostly build progress lines, with one entry listed twice: 325 of 2,404 bytes were assertion text in phase 4.3's T2 prompts"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md, section 9, run 3752b8ff), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs::collect_episode_knowledge_cached", "crates/roko-cli/src/dispatch/prompt_builder.rs::episode_statements"]
links = { depends_on = [], blocks = [], related = ["gap-ad9e03", "bug-3e3d08"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn learned_patterns_list_each_failure_once_without_build_lines' crates/roko-cli/src && cargo test -p roko-cli --lib learned_patterns_list_each_failure_once_without_build_lines"
+++

## Problem

The prompt's "Learned patterns from prior episodes" section lists up to 5 past episodes whose text shares the
task's topic words. Each entry is the episode's first statement, cut to 420 characters. For a failed try that
statement is the gate's failure text, so the entries are mostly build progress lines from other plans.

**Phase 4.3's T2** (Undertow, run 3752b8ff; all three T2 prompts in `.roko/prompt-logs/`):
- **The section was 2,404 bytes, five entries:** phase 4's T6, 4.2's T1, 2.2's T3, and phase 2's T2 twice. The
  last entry repeats the one before it.
- **All five were failures of other plans' tasks.** Each ends in `[truncated]` mid-line, e.g. `[ 98%] Built ta
  [truncated]`.
- **Only 325 bytes (14%) were assertion or logged text.** The rest:

  | Part | Bytes |
  |---|---|
  | Entry headers | 714 |
  | Build progress lines (`[ 21%] Built target …`) | 470 |
  | Step headers | 416 |
  | Banners | 339 |
  | Worktree paths | 140 |

- T1's prompt had 4 entries, also with a duplicate.

## Why it matters

- **Memory should say something a try can use.** Here it was other tasks' build logs.
- **It costs.** At 2.4 KB, about 0.8k tokens, it's re-sent on every call of every try: about 130 calls in this
  run.
- **Its quality affects the learning loops' measured value.** The section bandit decides from these sections
  (gap-ad9e03).
- Goal: learning.

## Where

- `crates/roko-cli/src/dispatch/prompt_builder.rs::collect_episode_knowledge_cached`:
  - scores episodes by topic overlap, then recency;
  - truncates to 5;
  - renders `- {task_id} ({outcome}, model: …): {first statement, 420 chars}`.
- `episode_statements` (same file) gives the statements.
- **Entry point:** the domain-context sections of `assemble`, in the same file.

## Current state

- Matching is by topic words only: `MIN_TOPIC_OVERLAP`, backlog 4213.
- Nothing removes duplicates. Two tries of the same task with the same failure text both rank.

## Plan

1. List each distinct statement once. Key the dedupe on task and text.
2. Before the 420-character cut, strip build progress lines, banners, and worktree path prefixes, so the cut keeps
   the failing test and assertion lines. bug-3e3d08's reader may already offer a parser for doctest lines.
3. Rank this plan's and this task's own episodes ahead of other plans', at equal overlap.
4. Add the test `learned_patterns_list_each_failure_once_without_build_lines`. Two episodes with the same failure
   statement render once, and an entry keeps its assertion line, not `Built target` lines.

## Done when

- The `[[verify]]` passes.
- The next plan run's prompt logs show no repeated entry in the section, and no `Built target` lines in it.

## Notes

- Evidence: `tmp/hermes-roko-v7/findings/run-analyst.md`, section 9, "What the retry prompts held". The bytes come
  from `tmp/hermes-roko-v7/run-analyst/prompt_logs.py`.
- Held, not part of this item: stripping the same noise from the retry feedback's raw gate output. That waits for
  phase 5's prompt logs, to see what bug-3e3d08's reader leaves.
- **Phase 5 repeats it (run 1f677933, roko 74c3594bc, 13 prompt logs).**
  - Every prompt has a learned-patterns section of 2,404–2,415 bytes, with 4–5 entries.
  - The repeats: T6#1 has 3 duplicate entries of 5, T4#1–T4#3 have 2 of 4, and T1#1–T1#3 have 1 of 5.
  - Only 42–468 bytes (2–19%) are assertion text. Build progress lines are 342–1,128 bytes.
  - The data is in `tmp/hermes-roko-v7/run-analyst/prompt_logs.py 1f677933 p5-ship`, and in
    `tmp/hermes-roko-v7/findings/run-analyst.md`, section 10.
- **The raw-gate-output part was settled by phase 5's logs, so it stays out of this item.** bug-3e3d08's reader
  leaves no build lines in the raw output, and only the worktree path prefixes remain. Those are a note in the
  run-analyst findings, not an item.
