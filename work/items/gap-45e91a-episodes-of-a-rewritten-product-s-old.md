+++
id = "gap-45e91a"
kind = "gap"
title = "Episodes of a rewritten product's old spec are still retrieved: phase 6's T4 retry prompt carried round v7's Undertow failures (\"Implement the editable unit array\") as learned patterns, and every phase 6 attempt's playbooks ranking had v7 episodes in its top three"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-cli/dispatch"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "c39fc94bc"
source = "round v8 hermes-pilot-v8 and run-analyst-v8: phase 6 run 23837ede (Undertow 8945b3c, roko e722f0e1f); filed by roko-planner-v8 at the lead's word"
discovered_from = "agent:run-analyst-v8 (round v8)"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs::collect_episode_knowledge_cached", "crates/roko-cli/src/dispatch/prompt_builder.rs::collect_playbooks_cached", "crates/roko-learn/src/episode_logger.rs::Episode"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_episode_of_a_retired_lineage_is_never_retrieved' crates/ && cargo test -p roko-cli --lib an_episode_of_a_retired_lineage_is_never_retrieved"
+++

## Problem

roko still retrieves episodes from a rewritten product's old spec into new tasks' prompts and playbook rankings.

- **The workspace.** Undertow 2 (round v8) replaced Undertow v1 (round v7) in the same workspace, `~/dev/undertow`.
  Its `.roko/episodes.jsonl` keeps v7's episodes.
- **The prompt.** Phase 6's T4 try 3 (run 23837ede, invocation 3) has the prompt log
  `.roko/prompt-logs/p6-the-window-T4-1794520853370.txt`. Under "# Learned patterns from prior episodes" it carries
  round v7's failures, each with v7's build output:
  - "T4 (failed, model: gpt-5.4-mini): verify: 1/1 verify step(s) failed for task `Implement the editable unit
    array`" (v7's phase 4);
  - "T3 (failed, model: glm-4.7): … pre_verify:no_changes";
  - "T2 (failed, model: gpt-5.4-mini): … `Repair the status head, clip bands, number, and pump field`" (v7's phase
    4.3).
- **The rankings.** In the run's `decisions.jsonl`, every phase 6 attempt's playbooks decision ranked v7 episodes
  in its top three. T4's try 3 ranked `ep-p4-undertow-phosphor-editor-T2-1792573411335` third. T2's try 1 ranked
  `ep-p4-undertow-phosphor-editor-T2-…` and `-T6-…` first and second.
- **Why.** `collect_episode_knowledge_cached` matches an episode on topic words alone (backlog 4213), never on its
  plan, spec or product. So a v7 "step cells" failure matches a v8 "step cells" task.

## Why it matters

- A retry prompt is where the context matters most. v7's failures describe a different design, with another unit
  array and other files, and an agent may follow them.
- roko has no way to retire episodes, as `roko knowledge retire` retires knowledge entries.
- Goal: learning loops.

## Where

- `crates/roko-cli/src/dispatch/prompt_builder.rs`: `collect_episode_knowledge_cached` and `collect_playbooks_cached`.
- `crates/roko-learn/src/episode_logger.rs`: `Episode`, which records no plan, spec or lineage beyond its id.

## Plan

1. Pick one of two approaches, or both:
   - **Scope.** Record each episode's plan and the hash of the spec it ran under, and retrieve only episodes of the
     same plan lineage or of a spec it descends from.
   - **Retire.** Give episodes a retire list like knowledge's (for example `roko learn episodes retire --before <date>`
     or `--plan <prefix>`), honoured by both sections.
2. Add the test `an_episode_of_a_retired_lineage_is_never_retrieved`. An episode from another lineage, or a retired
   one, that matches the task's topic words appears in neither the learned patterns nor the playbooks ranking.

## Done when

- The `[[verify]]` command passes.

## Notes

- Evidence: the prompt log and the run's `decisions.jsonl` in Undertow's `.roko` (read-only); run-analyst-v8's
  findings (23:2x, phase 6 landed).
- Related: backlog 4213 (topic matching), `roko knowledge retire`.
