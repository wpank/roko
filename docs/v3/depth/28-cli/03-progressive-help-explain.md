# 28.03 -- Progressive Help (`roko explain`)

> Depth file for [28-CLI.md](../../28-CLI.md) -- v1/12/03.

---

## Overview

`roko explain <topic>` provides a three-level progressive disclosure help system.
Each topic has increasing depth levels that reveal more implementation detail.

## Usage

```bash
# Level 1: Brief summary (default)
roko explain gates

# Level 2: How it works in roko
roko explain gates --depth 2

# Level 3: Internals and configuration
roko explain gates --depth 3

# List all available topics
roko explain --list
```

## Topic Structure

Each topic is a `TopicEntry` with three disclosure levels:

```rust
pub struct TopicEntry {
    pub name: &'static str,       // Machine name for CLI
    pub title: &'static str,      // Human-readable title
    pub summary: &'static str,    // Level 1: 2-3 sentences
    pub detail: &'static str,     // Level 2: how it works
    pub internals: &'static str,  // Level 3: implementation details
}
```

All content is compiled into the binary as `&'static str` -- no file I/O
required. The `TOPICS` static array is the single source of truth.

## Registered Topics

### `gates` -- Verify Pipeline

- **Level 1**: Gates validate agent output. Pass/fail checks (compile, test,
  clippy, diff-review) ensure code quality.
- **Level 2**: 7-rung pipeline after every agent turn. Adaptive thresholds tune
  via EMA. Results recorded in `.roko/episodes.jsonl`. Configure gates in
  `roko.toml` under `[[gates]]`.
- **Level 3**: Implementation in `crates/roko-gate/src/`. `GatePipeline` struct.
  Thresholds at `.roko/learn/gate-thresholds.json`. `HotellingDetector` uses
  Hotelling's T-squared statistic for multivariate anomaly detection. Verdicts
  emit `DashboardEvent::GateVerdict` for real-time TUI updates.

### `routing` -- Cascade Router

- **Level 1**: Selects which LLM model handles each task. Starts cheap,
  escalates on gate failure.
- **Level 2**: Bandit-style policy mapping complexity tiers to models. Thompson
  sampling. State at `.roko/learn/cascade-router.json`. Configure models in
  `roko.toml` under `[routing]`.
- **Level 3**: Implementation in `crates/roko-learn/src/cascade_router.rs`. HDC
  vectors from `roko-primitives` for tier assignment. Efficiency events at
  `.roko/learn/efficiency.jsonl` for offline analysis.

### `cognitive` -- Cognitive Architecture

- **Level 1**: One noun (Signal) and 12 kernel traits. Universal loop: query,
  score, route, compose, act, verify, write, react.
- **Level 2**: Trait contracts for each phase. `SystemPromptBuilder` assembles
  9-layer prompts from role templates, domain context, and runtime state.
- **Level 3**: Traits in `crates/roko-core/src/traits.rs`. Universal loop wired
  in `crates/roko-cli/src/run.rs` via `run_once()`. Prompt assembly uses
  `RoleSystemPromptSpec` in the runner module. Templates in
  `crates/roko-compose/src/templates/` (11 role templates).

### `neuro` -- Durable Knowledge Store

- **Level 1**: Long-term memory. Stores distilled knowledge, learned patterns,
  factual summaries across sessions.
- **Level 2**: Distillation pipeline extracts insights from episodes. Tier
  progression (novice through expert). Query via `roko knowledge query`.
- **Level 3**: Implementation in `crates/roko-neuro/`. Signal persistence as
  JSONL in `.roko/neuro/`. HDC vectors from `roko-primitives` for similarity
  search. Tiers tracked per-domain in `.roko/neuro/tiers.json`.

### `daimon` -- Behavioral Primitives

- **Level 1**: Daimons encode preferences, habits, and response patterns that
  emerge from experience. Each tracks an affect dimension.
- **Level 2**: Affect values (curiosity, caution, confidence) shift based on
  outcomes. Influence prompt composition, model selection, and gate thresholds.
- **Level 3**: Implementation in `crates/roko-daimon/`. Affect values are f64
  in [-1, 1] with drift and decay. `AffectMap` holds all dimensions. State
  persists at `.roko/daimon/`.

### `dreams` -- Offline Consolidation

- **Level 1**: Dreams process completed episodes to extract patterns, distill
  knowledge, and tune parameters. They run when the system is idle.
- **Level 2**: Three phases: hypnagogia (light review), imagination (creative
  recombination), deep sleep (parameter consolidation). Use
  `roko knowledge dream run` or configure scheduling in `roko.toml`.
- **Level 3**: Implementation in `crates/roko-dreams/`. `DreamRunner`
  orchestrates phases. Hypnagogia reads `.roko/episodes.jsonl`. Deep sleep
  updates cascade router weights and gate thresholds. Artifacts persist at
  `.roko/dreams/`.

### `signal` -- Signal Storage

- **Level 1**: Signals are the fundamental data unit. Every piece of information
  is stored as a content-addressed signal with a blake3 hash and DAG lineage.
- **Level 2**: Signals form an immutable DAG audit trail. Use
  `roko replay <hash>` to walk the lineage. Signals persist in
  `.roko/engrams.jsonl`.
- **Level 3**: Implementation in `crates/roko-core/src/engram.rs`. `Signal` is
  the primary type name; `Engram` is the underlying struct with
  `pub type Engram = Signal`.

### Additional Topics

Further topics cover:

- `safety` -- Trust-origin IFC, immune Graph, corrigibility
- `learning` -- Episodes, playbooks, experiments, efficiency
- `graphs` -- Cell-based DAG execution engine
- `feeds` -- Runtime data feeds and bus bridging
- `triggers` -- Declarative trigger bindings
- `groups` -- Agent group coordination
- `payments` -- x402 protocol and cost tracking

## Output Formatting

The explain command formats output for terminal display:

```
== Verify Pipeline ==

Gates validate agent output before it is accepted. Each gate is a
pass/fail check (compile, test, clippy, diff-review) that ensures
code quality. Tasks must pass all configured gates to advance.
```

At depth 2, the detail section is appended below the summary with a separator.
At depth 3, the internals section follows the detail section.

## Error Handling

Unknown topics print an error with the list of valid topic names and exit with a
non-zero code:

```bash
$ roko explain unknown-topic
error: unknown topic: 'unknown-topic'
available topics: gates, routing, cognitive, neuro, ...
```

An `AtomicBool` flag tracks whether an error has occurred, enabling correct exit
code propagation.

## Integration with `roko doctor`

The explain system complements `roko doctor` diagnostics. Where `doctor`
reports workspace health issues, `explain` teaches the user about the
subsystems that doctor checks:

```bash
# Doctor says gates are misconfigured
roko doctor
# > WARNING: gate thresholds file missing

# Explain how gates work
roko explain gates --depth 3
# > Adaptive thresholds persist at .roko/learn/gate-thresholds.json...
```

## Design Rationale

Compiling all topic content as static string slices eliminates file I/O and
dependency on external documentation files. This means `roko explain` works in
any deployment environment, including minimal containers, without access to a
docs directory. Each topic's three levels follow the same pedagogical pattern:
what it is, how roko uses it, and where to find the implementation.

## Source

- `crates/roko-cli/src/explain.rs` -- Topic registry and display logic
