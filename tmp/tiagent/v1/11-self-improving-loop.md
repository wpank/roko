# Cybernetic Self-Improvement Loop Design

**Date**: August 2026

Every time you use Claude Code or Cursor, your 1000th session is exactly as capable
as your 1st. The tool learns nothing from your usage. tiagent is different --- it
gets measurably better the more you use it. After 100 coding sessions, your tiagent
routes tasks to the right model automatically, knows which prompt patterns work for
your codebase, and has extracted reusable strategies from your successful runs.

This document describes how that works: a system of nested feedback loops that
enables tiagent to improve its own behavior through experience, without human
intervention. Two of the three loops work entirely standalone --- no blockchain, no
external services, just local learning from your own usage. The third loop
(cross-agent learning via Celestia DA) is optional but powerful.

For background:

- **01-vision-and-overview.md** explains what tiagent is and why it exists.
- **02-architecture.md** explains the core abstractions: one noun (Signal), six verb
  traits (Substrate, Scorer, Gate, Router, Composer, Policy), and a universal loop.
- **07-tracecommons-integration.md** explains how tiagent shares traces with other
  agents through TraceCommons (optional, requires Celestia).

This document assumes no prior knowledge of cybernetics, control theory, or agent
self-improvement research. Every concept is built from first principles.

---

## Table of Contents

1. [What This Means in Practice](#1-what-this-means-in-practice)
2. [What is a Cybernetic System?](#2-what-is-a-cybernetic-system)
3. [Three Feedback Loops](#3-three-feedback-loops)
4. [Inner Loop: Execution Feedback](#4-inner-loop-execution-feedback)
5. [Middle Loop: Learning Across Tasks](#5-middle-loop-learning-across-tasks)
6. [Outer Loop: Cross-Agent Learning](#6-outer-loop-cross-agent-learning)
7. [Harness Self-Optimization (HarnessX)](#7-harness-self-optimization-harnessx)
8. [Sleep-Time Consolidation](#8-sleep-time-consolidation)
9. [Safety Guardrails](#9-safety-guardrails)
10. [Measuring Improvement](#10-measuring-improvement)
11. [Research Foundations](#11-research-foundations)

---

## 1. What This Means in Practice

Most developer tools are static. You configure them once, and they stay that way
forever. tiagent is different: it accumulates knowledge from every task you run and
uses that knowledge to get faster, cheaper, and more accurate over time.

Here is the concrete progression:

```
    WEEK 1 (baseline)
    ──────────────────────────────────────────────────────
    - All tasks routed to your default model (e.g., Sonnet)
    - Generic system prompts, no codebase-specific knowledge
    - No playbooks — every problem solved from scratch
    - Gate thresholds at factory defaults

    MONTH 1 (~50-100 tasks)
    ──────────────────────────────────────────────────────
    - CascadeRouter has learned model preferences per task type
      (e.g., Haiku for tests, Sonnet for refactors, Opus for design)
    - 3-5 playbooks extracted from successful runs
    - Gate thresholds adapted to your codebase's realistic baselines
    - Token budgets calibrated to actual usage patterns

    MONTH 3 (~200-500 tasks)
    ──────────────────────────────────────────────────────
    - 15+ playbooks covering common task patterns
    - Routing accuracy >90% (right model for the right job)
    - Cost per task reduced ~40% (fewer retries, cheaper models where safe)
    - Time per task reduced ~30% (better prompts, fewer gate failures)
    - Prompt templates evolved through A/B experiments

    WITH CELESTIA (optional)
    ──────────────────────────────────────────────────────
    - Your agent also benefits from improvements discovered
      by other developers' agents across the network
    - Playbooks from the community augment your local library
    - Routing weights reflect collective experience, not just yours
```

None of this requires you to do anything. You just use tiagent, and it gets better.
The inner and middle loops that drive this progression work entirely standalone ---
no Celestia, no external services. The rest of this document explains how.

---

## 2. What is a Cybernetic System?

### The thermostat analogy

Consider a thermostat. You set the desired temperature to 21C. The thermostat
measures the current temperature. If the room is too cold, it turns on the heater.
If the room is too warm, it turns the heater off. It does this continuously,
forever, without any human telling it what to do moment-to-moment.

This is a **feedback loop**. It has four parts:

1. **Sense** --- measure the current state (room temperature).
2. **Compare** --- evaluate the current state against the desired state (21C).
3. **Act** --- do something to close the gap (turn heater on or off).
4. **Repeat** --- go back to step 1.

The word "cybernetic" comes from the Greek *kybernetes*, meaning "steersman" or
"governor." Norbert Wiener coined the modern usage in 1948 to describe systems that
regulate themselves through feedback. A thermostat is cybernetic. A cruise control
system is cybernetic. Your body's temperature regulation is cybernetic. The key
property is **self-regulation**: the system adjusts its own behavior based on
observed outcomes, without external direction.

### Agent harnesses as cybernetic systems

An AI agent harness --- the runtime that sits between an LLM and the outside world
--- can be designed as a cybernetic system. The analogy maps directly:

```
Thermostat                         Agent Harness
─────────────────────────────────────────────────────
Sense    room temperature          task success rate, token cost, error frequency
Compare  desired temperature       baseline metrics, target performance
Act      toggle heater             adjust prompts, routing, tool configs
Repeat   continuous loop           continuous across task executions
```

Most agent harnesses today are **open-loop**: they execute tasks, but they do not
feed the outcomes back into their own configuration. If a particular prompt template
works poorly, or a model is consistently bad at a certain task type, the harness
does not notice and does not adapt. A human must manually adjust the configuration.

tiagent closes the loop. Every execution produces structured data about what
happened. That data is automatically analyzed, compared against baselines, and used
to adjust the harness configuration for future executions. The harness improves
itself by running.

### The goal

A fully cybernetic agent harness converges toward better performance over time:

- Tasks that used to fail begin to succeed.
- Tasks that used to consume many tokens learn to complete with fewer.
- Tasks that used to require expensive models learn to use cheaper ones.
- Strategies that worked once become reusable playbooks for similar tasks.
- Knowledge gained by one agent propagates to all agents in the network.

This is not artificial general intelligence. It is not recursive self-improvement
in the science-fiction sense. It is the methodical application of control theory to
software configuration: measure, compare, adjust, measure again.

---

## 3. Three Feedback Loops

tiagent's self-improvement system is organized into three nested feedback loops,
each operating at a different timescale and scope:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                         │
│   OUTER LOOP (cross-agent)                                              │
│   *** Requires Celestia DA — optional but powerful ***                   │
│   Timescale: hours to days                                              │
│   Scope: across all tiagent instances in the network                    │
│                                                                         │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │                                                                 │   │
│   │   MIDDLE LOOP (cross-execution)                                 │   │
│   │   *** Works standalone — no Celestia needed ***                  │   │
│   │   Timescale: minutes to hours                                   │   │
│   │   Scope: across all tasks executed by a single agent            │   │
│   │                                                                 │   │
│   │   ┌─────────────────────────────────────────────────────────┐   │   │
│   │   │                                                         │   │   │
│   │   │   INNER LOOP (per-execution)                            │   │   │
│   │   │   *** Works standalone — no Celestia needed ***          │   │   │
│   │   │   Timescale: seconds to minutes                         │   │   │
│   │   │   Scope: within a single task execution                 │   │   │
│   │   │                                                         │   │   │
│   │   │   gate fails → replan → retry                           │   │   │
│   │   │   tool errors → adapt → retry                           │   │   │
│   │   │   budget exceeded → escalate                            │   │   │
│   │   │                                                         │   │   │
│   │   └─────────────────────────────────────────────────────────┘   │   │
│   │                                                                 │   │
│   │   route weights updated, thresholds adjusted                    │   │
│   │   playbooks extracted, prompts refined                          │   │
│   │                                                                 │   │
│   └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│   traces published to DA, consumed by other agents                      │
│   TraceCommons trajectories retrieved and applied                       │
│   routing strategies shared and evolved                                 │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

Each loop feeds into the next. The inner loop produces episode records. The middle
loop aggregates those records into learning artifacts. The outer loop shares those
artifacts across agents. Improvements flow upward (from concrete observations to
abstract strategies) and downward (from network-wide patterns to local
configuration changes).

The rest of this document describes each loop in detail.

---

## 4. Inner Loop: Execution Feedback

The inner loop operates within a single task execution. It is the fastest loop ---
measured in seconds --- and handles the immediate, tactical responses to problems
that arise while an agent is working.

### 4.1 Gate failure triggers replanning

tiagent validates agent output through a multi-rung gate pipeline. Each rung checks
a different quality dimension: compilation, tests, linting, diff size, coverage,
and higher-level semantic checks. When a gate rung fails, the inner loop does not
simply report the failure. It feeds the failure details back to the agent and
asks for a revised plan.

The flow looks like this:

```
    Agent produces output
            │
            ▼
    ┌───────────────┐
    │  Gate Pipeline │
    │  (7 rungs)    │
    └───────┬───────┘
            │
       Pass │  Fail
       ┌────┴────┐
       │         │
       ▼         ▼
    Accept   ┌──────────────────┐
             │ Build replan     │
             │ context:         │
             │  - which rung    │
             │  - error output  │
             │  - original task │
             │  - attempt count │
             └────────┬─────────┘
                      │
                      ▼
              Agent retries with
              failure context
                      │
                      ▼
              Gate pipeline again
                      │
                 ...repeats...
                (max 3 attempts)
```

The key design choice: the replan context is structured, not just "try again." The
agent receives the specific rung that failed (e.g., "rung 2: cargo test"), the
exact error output (e.g., the failing test name and assertion), and the attempt
number. This gives the LLM enough context to make a targeted fix rather than
blindly regenerating.

### 4.2 Tool error adaptation

When a tool call fails --- a file is not found, a command returns an error, an API
call times out --- the inner loop feeds the error back to the LLM as part of the
conversation context. The agent can then choose a different approach:

- **Retry with modified arguments**: the file was at a different path.
- **Use a different tool**: read the directory listing first to find the right file.
- **Escalate**: report the error as a blocker and move on.

This is standard agent retry behavior, but tiagent records every tool error as a
structured Signal. This matters because the middle loop will later analyze these
errors to detect patterns (e.g., "this tool fails 40% of the time on this type of
task --- maybe route to a different tool or add a pre-check").

### 4.3 Token budget tracking

Every agent turn consumes tokens. The inner loop tracks cumulative token usage
against a per-task budget. When the budget is approached or exceeded, the system
can:

- **Warn the agent**: inject a system message saying "you have N tokens remaining."
- **Switch models**: route to a cheaper model for remaining work.
- **Terminate**: halt the task and record the budget overrun.

The budget numbers themselves are not fixed. They are adjusted by the middle loop
based on historical token usage for similar tasks.

### 4.4 Episode recording

Every inner-loop execution produces an **episode** --- a structured record of what
happened:

```
Episode {
    task_id:         "P23-T04",
    agent_backend:   "claude-sonnet-4",
    started_at:      "2026-08-13T14:22:00Z",
    finished_at:     "2026-08-13T14:23:47Z",
    turns:           12,
    input_tokens:    18_420,
    output_tokens:   4_310,
    tool_calls:      7,
    tool_errors:     1,
    gate_attempts:   2,
    gate_passed:     true,
    final_rung:      7,
    hdc_fingerprint: [0.12, -0.44, 0.87, ...],   // 256-dim behavioral vector
}
```

Episodes are the raw material for every higher-level loop. They are appended to
a local JSONL file (`.roko/episodes.jsonl`) and optionally published to Celestia
DA for cross-agent consumption.

---

## 5. Middle Loop: Learning Across Tasks

The middle loop operates across task executions within a single agent. Its timescale
is minutes to hours --- it updates after each completed task and periodically
consolidates learning artifacts. Where the inner loop reacts to immediate problems,
the middle loop identifies patterns and adjusts the harness configuration.

**This is the loop that matters most to individual developers.** It works entirely
standalone --- no Celestia, no network, no external services. Here is what it does
in concrete terms:

- **After 50 runs**, tiagent learns that Claude Haiku handles your test-writing
  tasks at 94% success rate (saving 5x on cost vs Opus). It routes test tasks to
  Haiku automatically.

- **After a gate catches a common lint error pattern 10 times**, tiagent adds it
  to the system prompt so the agent avoids it proactively. The error stops
  occurring.

- **Successful multi-step refactors become playbooks** --- next time a similar
  refactor is needed, tiagent has a template. Instead of solving the problem from
  scratch, the agent starts with a proven strategy.

The subsections below describe each mechanism in detail.

### 5.1 CascadeRouter weight updates

tiagent uses a cascade router to select which LLM model handles each task. The
router maintains a set of models ranked by preference, with weights that encode
historical success rates per task category:

```
CascadeRouter state (simplified):

    Task Category     Model             Weight    Success Rate
    ─────────────────────────────────────────────────────────
    code_generation   claude-sonnet-4   0.82      91%
    code_generation   gpt-4.1           0.71      84%
    code_generation   gemini-2.5-pro    0.68      79%
    code_review       claude-sonnet-4   0.79      88%
    code_review       gpt-4.1           0.83      92%
    debugging         claude-sonnet-4   0.90      95%
    debugging         gemini-2.5-pro    0.61      72%
```

After each task, the router updates the weight for the model that was used:

- **Task succeeded**: weight increases (EMA update toward 1.0).
- **Task failed**: weight decreases (EMA update toward 0.0).

Over time, the router learns which models are best at which types of tasks and
routes accordingly. A task categorized as "debugging" will preferentially be routed
to the model with the highest debugging weight.

The weights are persisted to disk (`.roko/learn/cascade-router.json`) so they
survive across restarts.

### 5.2 Efficiency metrics

The middle loop tracks four key efficiency metrics across all tasks:

| Metric          | What it measures                            |
|-----------------|---------------------------------------------|
| Tokens per task | Average input + output tokens to complete   |
| Time per task   | Wall-clock seconds from start to gate pass  |
| Cost per task   | Dollar cost based on model pricing          |
| Attempts per task | Average gate attempts before success      |

These metrics are computed as exponential moving averages (EMAs), giving more weight
to recent executions. They serve two purposes:

1. **Anomaly detection**: if a task consumes 5x the average tokens, something
   unusual happened --- perhaps the prompt was bad or the model got stuck in a loop.
2. **Budget calibration**: the inner loop's token budgets are derived from these
   averages, so they automatically adjust as the agent's efficiency changes.

Metrics are written to `.roko/learn/efficiency.jsonl`, one record per task.

### 5.3 Prompt experiment store

tiagent can run A/B experiments on prompt templates. The experiment store manages
the lifecycle:

```
Experiment lifecycle:

    ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
    │ CREATED  │────▶│ RUNNING  │────▶│ ANALYZED │────▶│ PROMOTED │
    │          │     │          │     │          │     │ or       │
    │ Define   │     │ Split    │     │ Compare  │     │ REJECTED │
    │ variants │     │ traffic  │     │ outcomes │     │          │
    └──────────┘     └──────────┘     └──────────┘     └──────────┘
```

Two or more prompt variants are defined. Incoming tasks are randomly assigned to
a variant. After enough samples accumulate, the experiment store compares success
rates and promotes the winning variant. This is how prompt templates evolve without
a human hand-editing them.

Experiments are persisted to `.roko/learn/experiments.json`.

### 5.4 Adaptive gate thresholds

Gate rungs have configurable thresholds. For example, rung 5 (coverage check) might
require 80% code coverage. But what if the codebase is in a state where 80% is
unrealistic? Or what if agents have improved enough that 80% is too lenient?

The middle loop adjusts thresholds using an exponential moving average of pass
rates:

- If a rung passes too easily (pass rate > 95%), the threshold tightens.
- If a rung fails too often (pass rate < 30%), the threshold loosens.

This prevents gates from becoming either rubber stamps or brick walls. The system
finds the threshold that meaningfully distinguishes good output from bad output,
given the current state of the agent and the codebase.

Thresholds are persisted to `.roko/learn/gate-thresholds.json`.

### 5.5 Playbook extraction

When an agent successfully completes a task that previously failed, the middle loop
can extract a **playbook** --- a reusable strategy document that describes what
worked:

```
Playbook {
    trigger:    "rust compilation error involving lifetime bounds",
    strategy:   "1. Read the full error message, focusing on the 'help' suggestion.
                 2. Check if the lifetime can be elided. If yes, remove explicit
                    annotations. If no, add the suggested bound.
                 3. Run cargo check to verify before proceeding.",
    source:     "episode P23-T04, attempt 2 succeeded after attempt 1 failed",
    success_rate: 0.87,
    uses:        14,
}
```

Playbooks are injected into the system prompt for future tasks that match the
trigger pattern. They function as a persistent, evolving memory of strategies ---
the agent remembers what worked and applies it to similar situations. This is the
same mechanism described in the Dynamic Cheatsheet research (see Section 11), but
implemented at the harness level rather than as a model feature.

---

## 6. Outer Loop: Cross-Agent Learning

> **This is the optional Celestia-powered layer.** If you are running tiagent
> standalone, you can skip this entire section. Everything in the inner loop
> (Section 4) and middle loop (Section 5) works without Celestia, without a
> network connection, and without any external services. The outer loop adds
> cross-agent learning for teams and communities that want it.

The outer loop operates across multiple tiagent instances. Its timescale is hours
to days, and its scope is the entire network of agents. Where the middle loop
improves a single agent, the outer loop ensures that improvements propagate to all
agents.

### 6.1 Publishing to Celestia DA

tiagent publishes learning artifacts to Celestia's data availability layer. Other
tiagent instances subscribe to the relevant namespaces and merge the artifacts into
their local state. The artifacts include:

| Artifact               | Namespace                    | Content                          |
|------------------------|------------------------------|----------------------------------|
| Routing weights        | `tiagent.learn.routing`      | CascadeRouter weight snapshots   |
| Efficiency metrics     | `tiagent.learn.efficiency`   | Per-task-category averages       |
| Gate thresholds        | `tiagent.learn.gates`        | Adaptive threshold values        |
| Playbooks              | `tiagent.learn.playbooks`    | Extracted strategy documents     |
| Episode summaries      | `tiagent.episodes`           | Compressed episode records       |

The publishing flow:

```
    Local agent completes task
            │
            ▼
    Middle loop updates local state
            │
            ▼
    ┌───────────────────────┐
    │ Publish to Celestia   │
    │ DA via light node     │
    │                       │
    │ Serialize artifact    │
    │ Submit as blob        │
    │ Get block height +    │
    │   commitment proof    │
    └───────────┬───────────┘
                │
                ▼
    Other agents subscribe to
    namespace, receive blob
                │
                ▼
    ┌───────────────────────┐
    │ Merge into local      │
    │ learning state        │
    │                       │
    │ Weighted merge:       │
    │   local_weight = 0.7  │
    │   remote_weight = 0.3 │
    └───────────────────────┘
```

The merge is weighted, not overwritten. An agent's local experience carries more
weight (0.7 by default) than remote artifacts (0.3). This prevents a single agent's
idiosyncratic behavior from dominating the network, while still allowing useful
patterns to propagate.

### 6.2 TraceCommons trajectory consumption

In addition to Celestia DA (which shares structured learning artifacts between
tiagent instances), tiagent integrates with TraceCommons (see **07-tracecommons-integration.md**)
for cross-harness trajectory retrieval. The outer loop uses TraceCommons in two ways:

1. **Before execution**: query TraceCommons for trajectories similar to the current
   task. If a high-quality trajectory exists, inject relevant fragments into the
   agent's context window as example behavior.

2. **After execution**: submit the completed episode to TraceCommons for scoring
   and inclusion in the shared corpus. High-quality traces earn NEAR-denominated
   credits.

The combination of Celestia DA and TraceCommons gives tiagent two complementary
sharing channels:

```
    Celestia DA                         TraceCommons
    ────────────────────────────────────────────────────────
    Structured artifacts                Raw trajectories
    Between tiagent instances only      Cross-harness (any agent)
    Routing weights, thresholds,        Full execution traces
      playbooks, metrics                  with tool calls
    Lightweight (KBs)                   Heavy (MBs per trace)
    Always on                           Opt-in per execution
    Real-time subscription              Query on demand
```

### 6.3 HDC fingerprint sharing

Every episode includes an HDC (hyperdimensional computing) fingerprint --- a 256-
dimensional vector that encodes the behavioral characteristics of the execution.
Tasks with similar fingerprints tend to respond well to similar strategies.

When the outer loop receives fingerprints from other agents, it clusters them with
local fingerprints to identify behavioral families. If a remote agent found a good
strategy for tasks in a particular cluster, the local agent can adopt that strategy
for its own tasks in the same cluster --- even if the specific task details are
different.

```
    HDC Fingerprint Space (2D projection)

    ▲
    │   ○ ○           ● ●
    │  ○   ○         ●   ●          ○ = local episodes
    │   ○ ○           ● ●          ● = remote episodes
    │                               ★ = cluster center
    │      ★              ★
    │
    │                   △ △
    │                  △   △        △ = unclustered (novel)
    │                   △ △
    │
    └──────────────────────────▶

    Clusters with mixed local/remote episodes indicate
    shared behavioral patterns. Strategies transfer well
    within a cluster.
```

### 6.4 The rising tide effect

Cross-agent learning creates a positive feedback cycle: each agent's improvement
benefits all agents in the network.

```
    Agent A improves at debugging
            │
            ▼
    Publishes updated routing weights + playbook to DA
            │
            ▼
    Agent B receives and merges artifacts
            │
            ▼
    Agent B improves at debugging (without its own trial and error)
            │
            ▼
    Agent B's freed capacity lets it improve at code review
            │
            ▼
    Agent B publishes code review improvements
            │
            ▼
    Agent A receives and merges
            │
            ▼
    Both agents are now better at debugging AND code review
```

This is the network effect that makes the outer loop valuable. A single agent
learning in isolation improves linearly. A network of agents sharing their learning
improves super-linearly, because each agent benefits from the combined experience
of all agents without paying the cost of individual discovery.

---

## 7. Harness Self-Optimization (HarnessX)

### 7.1 The harness as a parameter space

Most discussions of "improving an AI agent" focus on the model: fine-tuning weights,
updating training data, or adjusting generation parameters. But the harness --- the
runtime infrastructure that wraps the model --- has its own large parameter space
that dramatically affects agent performance:

| Parameter                    | Example values                          |
|------------------------------|-----------------------------------------|
| System prompt template       | Template A, B, C (different structures) |
| Tool ordering in prompt      | Alphabetical, by frequency, by category |
| Context window allocation    | 60% history / 40% tools vs. 40% / 60%  |
| Model routing weights        | Sonnet bias 0.8 vs. 0.5 vs. 0.3        |
| Gate thresholds              | Coverage 80% vs. 70% vs. 90%           |
| Retry budget                 | 3 attempts vs. 5 attempts               |
| Temperature                  | 0.0 vs. 0.3 vs. 0.7                    |
| Max tokens per turn          | 4096 vs. 8192 vs. 16384                |

Research on harness optimization (see Section 11) shows that tuning these parameters
can produce gains comparable to switching model generations. RHO (Retrieval-augmented
Harness Optimization) improved SWE-Bench Pro performance from 59% to 78% by
optimizing the harness alone, without changing the underlying model.

tiagent treats these parameters as a search space to be explored systematically.

### 7.2 A/B experiment framework

The prompt experiment store (Section 5.3) is a specific instance of a general A/B
experiment framework. The framework can test any harness parameter:

```
    ┌─────────────────────────────────────────────────────────────┐
    │                    Experiment Definition                     │
    │                                                             │
    │  Name:       "system-prompt-structure-v3"                   │
    │  Parameter:  system_prompt_template                         │
    │  Variants:                                                  │
    │    A (control):  current 9-layer template                   │
    │    B (treatment): compressed 5-layer template               │
    │  Traffic split:  50/50                                      │
    │  Min samples:    30 per variant                             │
    │  Metric:         task_success_rate                          │
    │  Duration:       72 hours or min samples reached            │
    │                                                             │
    └─────────────────────────────────────────────────────────────┘

    Incoming tasks are assigned to variants:

        Task 1 ──▶ Variant A ──▶ success  ┐
        Task 2 ──▶ Variant B ──▶ success  │
        Task 3 ──▶ Variant A ──▶ failure  │  Accumulate
        Task 4 ──▶ Variant B ──▶ success  │  results
        Task 5 ──▶ Variant A ──▶ success  │
        ...                               ┘

    After min samples reached:

        Variant A: 22/30 success = 73.3%
        Variant B: 26/30 success = 86.7%
        p-value:   0.018 (significant)

        ──▶ PROMOTE Variant B
```

When an experiment concludes with a statistically significant winner, the winning
configuration is promoted to become the new default. The previous default is
archived as a Signal in the DAG, preserving the full history of configuration
evolution.

### 7.3 Version tracking

Every configuration change is recorded as a Signal with parent pointers to the
experiment that produced it and the previous configuration it replaced. This creates
a linear history of harness evolution:

```
    [Config v1] ──▶ [Experiment: prompt-structure-v2] ──▶ [Config v2]
                                                              │
                    [Experiment: routing-weights-v3] ──────────┘
                                                              │
                                                         [Config v3]
                                                              │
                    [Experiment: gate-threshold-v4] ──────────┘
                                                              │
                                                         [Config v4]
```

If performance degrades after a configuration change, the system can walk backwards
through this DAG to identify which change caused the regression and revert it.

---

## 8. Sleep-Time Consolidation

### 8.1 The problem with active-only learning

The three feedback loops described above all operate during active task execution.
But some learning is better done offline --- when the agent is idle and has time
for reflection. Active-time learning is biased toward recency (the last few tasks
dominate) and is constrained by the time pressure of task execution.

Sleep-time consolidation addresses this by using idle periods to:

- Review and re-analyze episode history with more compute.
- Identify long-term patterns that per-task analysis misses.
- Compress large episode logs into compact summaries.
- Generate and refine playbooks from clusters of related episodes.

### 8.2 The dream cycle

tiagent's consolidation process is called a "dream cycle" --- a deliberate analogy
to biological sleep, where the brain consolidates memories and extracts patterns
from the day's experiences. The cycle runs during idle time, triggered either by a
cron schedule or by detecting that the agent has been inactive for a configurable
threshold (default: 15 minutes).

The cycle has four phases:

```
    DREAM CYCLE
    ═══════════════════════════════════════════════════

    Phase 1: REVIEW
    ┌─────────────────────────────────────────────────┐
    │ Read recent episodes from .roko/episodes.jsonl  │
    │ Filter to episodes since last dream cycle       │
    │ Group by task category and outcome              │
    └──────────────────────┬──────────────────────────┘
                           │
                           ▼
    Phase 2: EXTRACT
    ┌─────────────────────────────────────────────────┐
    │ Identify success patterns:                      │
    │   - Which strategies led to first-attempt       │
    │     gate passes?                                │
    │   - Which tool sequences were most efficient?   │
    │                                                 │
    │ Identify failure patterns:                      │
    │   - Which error types recurred?                 │
    │   - Which tasks consistently needed retries?    │
    │                                                 │
    │ Cluster episodes by HDC fingerprint             │
    └──────────────────────┬──────────────────────────┘
                           │
                           ▼
    Phase 3: CONSOLIDATE
    ┌─────────────────────────────────────────────────┐
    │ Update or create playbooks from patterns        │
    │ Compress old episodes into summary records      │
    │ Adjust confidence weights on existing playbooks │
    │ Promote high-confidence strategies              │
    │ Deprecate strategies that stopped working       │
    └──────────────────────┬──────────────────────────┘
                           │
                           ▼
    Phase 4: DISTILL
    ┌─────────────────────────────────────────────────┐
    │ Generate a compact knowledge summary:           │
    │   "Over the last 48 hours, this agent executed  │
    │    37 tasks. Key findings: ..."                 │
    │                                                 │
    │ Store in durable knowledge store                │
    │ Optionally publish summary to Celestia DA       │
    └─────────────────────────────────────────────────┘
```

### 8.3 Cost reduction through sleep-time pre-processing

Sleep-time compute is significantly cheaper than active-time compute because there
is no latency requirement. The system can:

- Use batch API pricing (typically 50% cheaper than real-time).
- Use larger, more capable models for analysis (since time is not a constraint).
- Process many episodes in a single large context window.

Research on sleep-time compute (see Section 11) has shown up to 5x reduction in
effective inference cost when knowledge is pre-organized during idle time rather
than retrieved and processed during active execution.

The practical impact: an agent that runs dream cycles overnight starts each
morning with a better-organized knowledge base, updated playbooks, and refined
routing weights --- all computed at batch rates during hours when the infrastructure
would otherwise sit idle.

---

## 9. Safety Guardrails

The phrase "self-improving agent" might sound alarming. Here is the key guarantee:
**tiagent will not silently change your development workflow.** All self-modifications
are logged, bounded, and reversible. The system can tune its own knobs (which model
to use, what thresholds to set, which prompt templates to prefer), but it cannot
change which knobs exist, and it cannot touch anything outside its defined parameter
space.

tiagent implements four categories of guardrails.

### 9.1 Bounded modification scope

The self-improvement loop can only modify a defined set of parameters:

```
    MODIFIABLE BY SELF-IMPROVEMENT          NOT MODIFIABLE
    ──────────────────────────────          ──────────────────────
    Prompt templates                        Safety policies
    Model routing weights                   Tool whitelist/blacklist
    Gate thresholds (within bounds)         Filesystem access scope
    Token budgets                           Network access scope
    Temperature / sampling params           Authentication credentials
    Playbook content                        Gate rung definitions
    Context window allocation               Audit logging behavior
    Retry limits (within bounds)            The self-improvement
                                              loop itself
```

The right column is critical: the self-improvement loop cannot modify the safety
policies that govern its own behavior, the set of tools the agent is allowed to
use, or the audit logging that records its actions. It also cannot modify its own
implementation --- the feedback loop code is not part of the parameter space.

### 9.2 Human-in-the-loop thresholds

Configuration changes are classified by magnitude. Small changes (e.g., a routing
weight moving from 0.82 to 0.84) are applied automatically. Large changes (e.g.,
a routing weight moving from 0.82 to 0.30, or a gate threshold dropping below a
minimum bound) require human approval:

```
    Change magnitude          Action
    ────────────────────────────────────────────────
    Small (< 10% delta)      Apply automatically
    Medium (10-30% delta)    Apply with notification
    Large (> 30% delta)      Block until human approval
    Safety-adjacent          Always block
```

This prevents the system from making dramatic configuration changes that might
degrade performance or violate safety constraints, even if the A/B experiment
data appears to support the change.

### 9.3 Automatic rollback

Every configuration change includes a rollback trigger: if the task success rate
drops below a threshold (default: 15 percentage points below baseline) within a
window after the change (default: 20 tasks), the change is automatically reverted
and the experiment is recorded as a false positive.

```
    Config v3 promoted at task #100
            │
            ▼
    Tasks #101-120: success rate = 62% (baseline was 85%)
            │
            ▼
    Delta = -23 points > -15 point threshold
            │
            ▼
    AUTOMATIC ROLLBACK to Config v2
            │
            ▼
    Experiment logged as false positive
    Config v3 marked "reverted" in DAG
```

### 9.4 Audit trail

Every self-modification is recorded as a Signal in the DAG. The audit trail
includes:

- **What changed**: the specific parameter and its old/new values.
- **Why it changed**: the experiment or learning event that triggered the change.
- **When it changed**: timestamp.
- **What happened after**: the performance metrics in the window following the
  change.

This trail is append-only and cannot be modified by the self-improvement loop. It
provides a complete forensic record for any human who wants to understand how the
agent's configuration evolved over time.

---

## 10. Measuring Improvement

### 10.1 Core metrics

tiagent tracks four primary metrics for measuring self-improvement:

| Metric             | Formula                                      | Good direction |
|--------------------|----------------------------------------------|----------------|
| Task success rate  | successful_tasks / total_tasks                | Higher         |
| Tokens per task    | total_tokens / successful_tasks               | Lower          |
| Cost per task      | total_cost_usd / successful_tasks             | Lower          |
| Time per task      | total_wall_seconds / successful_tasks         | Lower          |

Note: all per-task metrics are computed against successful tasks only. Failed tasks
are tracked separately as the success rate metric. This prevents a perverse
incentive where the system could "improve" efficiency by abandoning hard tasks.

### 10.2 Baseline comparison

Every metric is compared against a baseline --- the performance measured during the
agent's initial configuration period (the first N tasks, where N is configurable,
default 50). The baseline is frozen and never updated. This provides a stable
reference point for measuring long-term improvement.

```
    Improvement Dashboard (conceptual)

    Success Rate         Tokens/Task          Cost/Task
    ─────────────────    ─────────────────    ─────────────────
    100%│         ╭──    25K│                 $0.50│
        │     ╭───╯          │                     │
     80%│ ╭───╯           20K│╲                    │╲
        │─┤ baseline          │ ╲               $0.30│ ╲
     60%│ │               15K│  ╲──╮               │  ╲──╮
        │ │                   │     ╲──            │     ╲──
     40%│ │               10K│        ╲─       $0.10│       ╲─
        │ │                   │                     │
        └─┴───────────────    └───────────────     └───────────────
         task count            task count            task count
```

### 10.3 Statistical rigor

Self-improvement claims must be statistically grounded. tiagent applies two rules:

1. **Minimum sample size**: no configuration change is promoted based on fewer than
   30 observations per variant (configurable). This is a practical minimum for
   detecting meaningful effect sizes in A/B tests.

2. **Significance testing**: the experiment store computes a p-value for the
   difference between variants. Only changes with p < 0.05 are promoted. This
   guards against promoting changes that appear beneficial but are actually noise.

These thresholds are deliberately conservative. It is better to miss a small
improvement than to promote a change that was just noise and will regress over time.

### 10.4 Dashboard visibility

All metrics, experiments, and configuration history are available through tiagent's
dashboard (the ratatui TUI or HTTP API):

- **Real-time metrics**: current success rate, efficiency, cost.
- **Trend charts**: metrics over time, overlaid with configuration changes.
- **Active experiments**: which experiments are running, their current sample sizes
  and preliminary results.
- **Configuration history**: timeline of all promoted changes with their measured
  impact.
- **Playbook inventory**: all extracted playbooks with their trigger patterns and
  success rates.

The dashboard makes the self-improvement process visible and auditable. A human
operator can see exactly what the system is doing, why it made each change, and
whether the changes are actually helping.

---

## 11. Research Foundations

The design described in this document is grounded in five research threads. This
section summarizes each one and explains how tiagent applies the ideas.

### 11.1 RHO (Retrieval-augmented Harness Optimization)

**Source**: arXiv:2606.05922

RHO demonstrated that optimizing the agent harness --- system prompts, retrieval
strategies, tool configurations --- can produce gains as large as switching model
generations. On SWE-Bench Pro, RHO improved task completion from 59% to 78% by
optimizing the harness around Claude Sonnet, without changing the model itself.

The key mechanism: RHO builds a trajectory corpus from past executions, identifies
which harness configurations correlated with success, and automatically refines the
system prompt and retrieval strategy based on those correlations.

**tiagent application**: The HarnessX framework (Section 7) is a direct
implementation of this idea. The prompt experiment store, adaptive gate thresholds,
and CascadeRouter weight updates are all forms of harness optimization. The
trajectory corpus is the episode log.

### 11.2 Dynamic Cheatsheet

**Source**: arXiv:2504.07952 (ICLR 2026, Suzgun et al.)

Dynamic Cheatsheet showed that giving an agent a persistent, evolving text document
of strategies accumulated from prior attempts produces dramatic performance gains.
Claude 3.5 Sonnet's AIME accuracy more than doubled. GPT-4o on Game of 24 went
from 10% to 99%.

The mechanism is simple: a structured text blob is appended to the agent's context
window before each task. The blob contains strategies that worked on similar tasks
in the past. After each task, the blob is updated with new strategies.

**tiagent application**: The playbook system (Section 5.5) implements this pattern
at the harness level. Playbooks are extracted from successful episodes, stored
persistently, and injected into the system prompt for matching tasks.

### 11.3 Sleep-Time Compute

**Source**: DeepMind research program, 2025-2026

Sleep-time compute uses idle periods to pre-process and organize knowledge,
reducing the compute required during active inference. The reported result is
approximately 5x reduction in active inference cost when knowledge is pre-organized
during downtime.

The analogy to biological sleep is apt: the brain consolidates memories during
sleep, converting short-term episodic memories into long-term semantic knowledge.
The agent equivalent is converting raw episode logs into compressed summaries,
playbooks, and routing weight updates.

**tiagent application**: The dream cycle (Section 8.2) implements sleep-time
consolidation with four phases: review, extract, consolidate, distill. It runs
during idle periods using batch pricing.

### 11.4 HarnessX Foundry Pattern

**Source**: Industry practice, formalized by multiple agent platform teams

The HarnessX pattern treats the agent harness itself as a parameter to be optimized
through systematic experimentation. The "foundry" metaphor refers to the idea that
the harness is being forged --- heated, shaped, and tempered --- by the accumulated
experience of running tasks.

The core insight: most teams treat harness configuration as a one-time setup task.
They write a system prompt, pick a model, set some thresholds, and leave everything
static. The HarnessX pattern makes configuration dynamic, driven by measured
outcomes rather than intuition.

**tiagent application**: The A/B experiment framework (Section 7.2) and version
tracking (Section 7.3) implement this pattern. Every configuration change is an
experiment, every experiment produces measured outcomes, and the measured outcomes
drive the next configuration change.

### 11.5 EvoRoute (Evolutionary Model Routing)

**Source**: Research on multi-model routing optimization

EvoRoute applies evolutionary algorithms to the model routing problem. Instead of
manually specifying which model handles which task type, EvoRoute maintains a
population of routing configurations. Each configuration is a set of routing rules
(e.g., "use Sonnet for debugging, use GPT-4.1 for code review"). Configurations
compete by measuring their aggregate task success rate. High-performing
configurations reproduce (with mutations). Low-performing configurations are
eliminated.

Over generations, the population converges on routing configurations that are
well-adapted to the agent's actual workload.

**tiagent application**: The CascadeRouter (Section 5.1) uses EMA-based weight
updates as a simpler alternative to full evolutionary optimization. The population-
based approach is a potential future enhancement for workloads where the simpler
approach plateaus.

---

## Summary

tiagent's self-improvement system is not a single mechanism but a layered
architecture of feedback loops, each operating at a different timescale:

```
    Timescale           Loop              What it does
    ══════════════════════════════════════════════════════════════
    Seconds             Inner             React to immediate failures
                                          (gate replan, tool retry)

    Minutes-hours       Middle            Learn from task patterns
                                          (routing, thresholds, playbooks)

    Hours-days          Outer             Share learning across agents
                                          (DA publication, TraceCommons)

    Idle periods        Sleep-time        Consolidate and distill
                                          (dream cycle, compression)

    Continuous          HarnessX          Optimize harness configuration
                                          (A/B experiments, promotion)
```

Each loop is bounded by safety guardrails: limited modification scope, human-in-
the-loop thresholds for large changes, automatic rollback on performance regression,
and an append-only audit trail. The system is designed to improve steadily and
safely, not to make dramatic unsupervised changes.

The net effect is an agent harness that gets better at its job simply by doing its
job. Every task execution is both productive work and a learning opportunity. Every
agent's improvement benefits every other agent in the network. The system converges
toward better performance over time, with full visibility and human oversight at
every step.
