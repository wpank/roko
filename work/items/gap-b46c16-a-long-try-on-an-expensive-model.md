+++
id = "gap-b46c16"
kind = "gap"
title = "A long try on an expensive model re-sends its whole growing history every turn, and roko's tool-result compaction waits for 80% of the context window: phase 4.2's T1 spent $1.21 of its $2.06 on cached input as prompts grew from 17k to 73k tokens, while gpt-5.4's trigger is 218k"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-agent/tool_loop"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md, section 8, run 1447385e), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs::prune_context_if_needed", "crates/roko-agent/src/tool_loop/mod.rs::compaction_trigger", "crates/roko-agent/src/tool_loop/compaction.rs::compact_tool_results", "crates/roko-core/src/config/provider.rs::ModelProfile", "crates/roko-cli/src/config.rs::ModelProfileLayer", "crates/roko-std/src/tool/builtin/edit_file.rs::DESCRIPTION", "crates/roko-core/src/defaults.rs::DEFAULT_TOOL_RESULT_COMPACTION_THRESHOLD_CHARS"]
links = { depends_on = [], blocks = [], related = ["bug-a8a787", "gap-1ba0ac", "bug-5b04d3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'compact_at_tokens' crates/roko-core/src/config && grep -rqw 'fn compaction_starts_at_the_configured_tokens_not_the_window' crates/roko-agent/src && cargo test -p roko-agent --lib compaction_starts_at_the_configured_tokens_not_the_window"

[[verify]]
command = "grep -rqw 'fn a_models_compact_at_tokens_survives_a_load' crates/roko-core/src && cargo test -p roko-core --lib a_models_compact_at_tokens_survives_a_load"
+++

## Problem

roko's tool loop sends the whole conversation on every turn. On a long try the prompt grows by every tool result,
and the provider bills each turn's re-sent prefix as cached input.

In phase 4.2 (run 1447385e), T1's two gpt-5.4 tries (46 and 59 turns) cost $2.06:
- **By token kind:** cached input $1.21 (59%, 4.84M tokens at $0.25/M), fresh input $0.46 (22%), output $0.39 (19%).
- **Prompt growth:** T1#1's prompt grew from 17.1k to 72.6k tokens (+1.2k a turn) and T1#2's from 17.5k to 64.5k
  (+0.8k a turn). The mean was 47–49k tokens, 96–97% of it cached.
- **What the growth is:** tool results, about 39k tokens of them per try.
  - `run_tests` output is 13–17k of that, from 13 and 17 runs.
  - The rest is file reads, greps and edit confirmations.
  - T1#2's 59 turns included 29 single `edit_file` calls.

roko already has compaction, but it never fired:
- **The trigger is the context window.** `prune_context_if_needed` runs `compaction::compact_tool_results` and
  `prune::prune_if_needed` only once the estimated prompt passes 80% of the model's `context_window`.
  - The first turns older tool results over 500 characters into a 200-character preview and keeps the newest two
    groups.
  - Undertow's gpt-5.4 has `context_window = 272000`, so the trigger is 217,600 tokens.
- **No config key sets it.** `openai_compat`'s `create_agent` passes `model.context_window` as the loop's limit.
  The other thresholds are constants in `roko-core/src/defaults.rs`: 500 characters, a 200-character preview, 2
  groups kept, and a 102,400-token fallback.
- **Lowering `context_window` is not a safe substitute.** The prompt builder's budgets come from it too
  (`roko-compose/src/role_prompts.rs`: soft limit 30% of the window, hard limit 50%). A 50k window would put 4.2's
  17.5k-token first prompt, with the whole spec bug-a8a787 now sends, over its soft budget.

## Why it matters

Golden path, cost.
- Cached input is now the largest cost line of a long try on an expensive model.
- gpt-5.4 ran 105 turns on one 6-file layout task, which was 92% of the run's spend. A task like that alone can reach
  a $3 cap.
- Phase 5's tasks are plugin tasks of the same shape.

## Where

- `roko-agent/src/tool_loop/mod.rs::prune_context_if_needed`, with `check_context_overflow` and `compaction_target`
  (80% of the window).
- `roko-agent/src/tool_loop/compaction.rs::compact_tool_results` and `tool_loop/prune.rs::prune_if_needed`.
- `roko-agent/src/provider/openai_compat.rs::create_agent`: `.with_context_token_limit(model.context_window)` and
  `.with_model_profile(...)`.
- `roko-core/src/defaults.rs`: the compaction constants.

## Current state

**Estimates.** A token-level simulation of T1's two tries (`tmp/hermes-roko-v7/run-analyst/context_sim.py`, output
in `data/context_sim.csv`):
- each turn's tool results are measured from consecutive prompts;
- the prefix cache breaks once wherever older history changes;
- turns and outputs are unchanged.

It reproduces the billed cost within 6% ($1.94 against $2.06). Savings are against its own baseline:

| Policy | T1, both tries | Saving |
|---|---|---|
| As run | $1.94 | |
| Drop each superseded `run_tests` result as it arrives | $1.92 | 1%: each prune breaks the cache, 12–16 times a try |
| roko's compaction, triggered at 60k tokens | $1.83 | 6% |
| roko's compaction, triggered at 40k tokens | $1.62 | 17% |
| Batch up to 3 consecutive edits into one turn | $1.70 | 13% |
| Batching and compaction at 40k | $1.44 | 26% |

On the billed $2.06, compaction at 40k is worth about $0.35, and with batching about $0.53.

## Plan

1. **Cheapest code change: a compaction trigger separate from the window.**
   - Add `[models.<key>] compact_at_tokens`. Its default is today's behaviour, 80% of the window.
   - `prune_context_if_needed` compacts once the prompt passes it, reusing `compact_tool_results` as it is, with one
     cache break per compaction.
   - Undertow then sets about 40,000 for gpt-5.4.
2. **Edit batching, by prompt guidance.** Tell the implementer to make every edit it can in one turn: `multi_edit`, or
   several edit calls in one turn. `MaxToolCallsPerTurn` allows 8–12.
3. **Not worth doing:** pruning superseded results as they arrive. The cache breaks eat the saving.

## What was built (roko-speed, 2026-10-26)

- `[models.<key>] compact_at_tokens` (`ModelProfile`, and the CLI's layered `ModelProfileLayer`): the estimated
  prompt size past which the tool loop compacts older tool results. `None` keeps 80% of `context_window`; a value
  above that is held to it, so compaction never starts later than before. The loader's schema sentinel knows the
  key, so a load keeps it.
- `ToolLoop::compaction_trigger` sets where `check_context_overflow` says `CompactRecommended`. Compaction itself is
  `compact_tool_results` as it was; messages are still dropped only past 80% of the window. Each compaction logs
  `tool_loop: compacted the conversation's older tool results` with the model, the estimate before and after, and
  the trigger, for tuning.
- Edit batching, by prompt guidance: `edit_file`'s and `multi_edit`'s descriptions ask for the edits a model already
  knows in one turn (several in one file with `multi_edit`, several files as several calls in one reply), since
  every turn re-sends the conversation.
- Undertow sets about `compact_at_tokens = 40000` on its gpt-5.4 entry, after a $0 rehearsal if one is wanted.
- Docs: `docs/v3/depth/05-agent/tool-loop-protocol.md` (tool-result compaction), `docs/v3/05-AGENT.md`.

## Done when

- `compact_at_tokens` exists and defaults to today's behaviour.
- A test shows compaction starting at the configured tokens rather than at 80% of the window:
  `compaction_starts_at_the_configured_tokens_not_the_window`.
- The `[[verify]]` passes.

## Notes

- **Caveats:**
  - Compaction drops old file contents and test output from the history, so the agent may re-read files. That adds
    turns and fresh input, which the estimate leaves out.
  - Tune the threshold on a $0 rehearsal before a paid run.
- **Evidence:** `tmp/hermes-roko-v7/findings/run-analyst.md`, section 8.
- **Related:** bug-a8a787 (the context fix that put the whole spec in the prompt), gap-1ba0ac (reasoning, which
  adds tokens of its own), bug-5b04d3 (catalog tokens in helper calls).
