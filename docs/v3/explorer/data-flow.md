---
title: Data Flow Animator
description: Step-by-step animated walkthroughs of five core Roko workflows showing how data moves through the system.
outline: [2, 3]
---

<script setup>
import DataFlowAnimator from '../.vitepress/components/DataFlowAnimator.vue'
</script>

# Data Flow Animator

## Interactive Pipeline

The interactive visualization below shows the high-level 8-stage signal
pipeline: Query, Score, Route, Compose, Act, Verify, Write, React. Use the
Play/Pause controls to animate the flow, or step through one stage at a time.
Click any stage for its description.

<ClientOnly>
  <DataFlowAnimator />
</ClientOnly>

The interactive pipeline above shows the high-level flow. See the detailed
sequence diagrams below for specifics on each workflow scenario.

---

Five animated walkthroughs of the core data flows in Roko. Each scenario
traces data through the system step by step, annotated with the crate
responsible for each stage.

::: tip How to read these diagrams
Each sequence diagram shows participants (components) across the top and
messages flowing between them in time order (top to bottom). Notes explain
what happens at each step. The `alt` blocks show conditional branches
(e.g., gate pass vs. gate fail).
:::

## Scenarios

[[toc]]

---

## 1. Plan Execution {#plan-execution}

The complete lifecycle from an idea to validated, committed code. This is
the self-hosting loop that Roko uses to develop itself.

### Phase 1: Idea to Plan

```mermaid
sequenceDiagram
    actor User
    participant CLI as roko-cli
    participant FS as roko-fs
    participant Agent as roko-agent
    participant Neuro as roko-neuro

    User->>CLI: roko prd idea "Add rate limiting"
    CLI->>FS: Write idea signal to .roko/prd/ideas/

    User->>CLI: roko prd draft new "rate-limiting"
    CLI->>Agent: Dispatch agent to write PRD
    Agent->>FS: Read codebase context
    Agent-->>CLI: Generated PRD document
    CLI->>FS: Write PRD to .roko/prd/drafts/

    User->>CLI: roko research topic "rate limiting"
    CLI->>Agent: Dispatch Perplexity research agent
    Agent-->>CLI: Research with citations
    CLI->>FS: Write research to .roko/research/

    User->>CLI: roko prd plan rate-limiting
    CLI->>Neuro: Query relevant knowledge
    Neuro-->>CLI: Past insights, anti-patterns
    CLI->>Agent: Generate tasks.toml from PRD + knowledge
    Agent-->>CLI: Implementation plan (task DAG)
    CLI->>FS: Write plan to plans/rate-limiting/
```

### Phase 2: Execution and Verification

```mermaid
sequenceDiagram
    participant CLI as PlanRunner<br/>(roko-cli)
    participant Graph as GraphEngine<br/>(roko-graph)
    participant Router as CascadeRouter<br/>(roko-learn)
    participant Compose as SystemPromptBuilder<br/>(roko-compose)
    participant Agent as AgentDispatcher<br/>(roko-agent)
    participant Gate as GatePipeline<br/>(roko-gate)
    participant Store as FileSubstrate<br/>(roko-fs)
    participant Learn as Feedback<br/>(roko-learn)

    CLI->>Graph: Load plan DAG, build execution graph
    Note right of Graph: Topological sort into<br/>parallel waves

    loop For each task in wave order
        Graph->>Router: Select model for task
        Router-->>Graph: model + provider

        Graph->>Compose: Build 9-layer system prompt
        Compose-->>Graph: Assembled prompt

        Graph->>Agent: Dispatch to LLM provider
        Note right of Agent: Tool loop: read, write,<br/>run commands

        Agent-->>Graph: Code changes + output

        Graph->>Gate: Run 7-rung pipeline
        Note right of Gate: R0 Compile / R1 Lint /<br/>R2 Test / R3 Symbol /<br/>R4-R6 Advanced

        alt Gates passed
            Gate-->>Graph: PASS + diagnostics
            Graph->>Store: Checkpoint state
            Graph->>Learn: Record episode, update router
        else Gates failed
            Gate-->>Graph: FAIL + error diagnostics
            Graph->>CLI: Build revision plan
            Note right of CLI: Inject error context,<br/>possibly escalate model
            CLI->>Graph: Retry with revised prompt
        end
    end

    Graph-->>CLI: Plan complete
    CLI->>Store: Final state checkpoint
```

---

## 2. Agent Dispatch {#agent-dispatch}

A single agent task dispatch, showing the internal pipeline from model
selection through the tool loop to output.

```mermaid
sequenceDiagram
    participant Runner as PlanRunner
    participant Router as CascadeRouter<br/>(roko-learn)
    participant Health as HealthRegistry<br/>(roko-learn)
    participant Compose as SystemPromptBuilder<br/>(roko-compose)
    participant Neuro as KnowledgeStore<br/>(roko-neuro)
    participant Daimon as DaimonState<br/>(roko-daimon)
    participant Playbook as PlaybookStore<br/>(roko-learn)
    participant Dispatch as AgentDispatcher<br/>(roko-agent)
    participant Safety as SafetyLayer<br/>(roko-agent)

    Runner->>Health: Filter unhealthy providers
    Health-->>Runner: Healthy provider set

    Runner->>Router: Select model (Static/Confidence/UCB)
    Note right of Router: Maturity stages:<br/>0-49 obs: Static<br/>50-199: Confidence<br/>200+: UCB bandit
    Router-->>Runner: Selected model + provider

    Runner->>Daimon: Read affect state
    Note right of Daimon: 3 timescales:<br/>fast emotion, medium mood,<br/>slow temperament
    Daimon-->>Runner: DispatchModulation<br/>(temperature, turn budget, exploration)

    Runner->>Neuro: Query relevant knowledge
    Neuro-->>Runner: Matching entries (fact, insight, heuristic)

    Runner->>Playbook: Query matching playbooks
    Playbook-->>Runner: Top when/then patterns

    Runner->>Compose: Build 9-layer prompt
    Note right of Compose: L1 Role / L2 Conventions<br/>L3 Domain / L4 Task<br/>L5 Feedback / L6 Anti-patterns<br/>L7 Playbooks / L8 Knowledge<br/>L9 Affect context
    Compose-->>Runner: Assembled system prompt

    Runner->>Safety: Check tool policy for role
    Safety-->>Runner: Allowed tool set

    Runner->>Dispatch: Send prompt + tools to provider
    Note right of Dispatch: Tool loop with safety<br/>enforcement on each call

    loop Tool calls
        Dispatch->>Safety: Validate tool request
        Safety-->>Dispatch: Allow or deny
        Dispatch->>Dispatch: Execute tool
    end

    Dispatch-->>Runner: Agent output + code changes
```

---

## 3. Gate Validation {#gate-validation}

The 7-rung gate pipeline that verifies every agent output, with adaptive
threshold mechanics and the failure-to-replan feedback loop.

```mermaid
sequenceDiagram
    participant Runner as PlanRunner
    participant Pipeline as GatePipeline<br/>(roko-gate)
    participant R0 as Rung 0: Compile
    participant R1 as Rung 1: Lint
    participant R2 as Rung 2: Test
    participant R3 as Rung 3: Symbol
    participant R456 as Rungs 4-6:<br/>Advanced
    participant Thresh as AdaptiveThresholds<br/>(roko-learn)
    participant Replan as GateFailureReplan<br/>(roko-cli)

    Runner->>Pipeline: Verify agent output
    Pipeline->>Thresh: Load current thresholds
    Thresh-->>Pipeline: Per-rung EMA thresholds

    Pipeline->>R0: cargo check / tsc / go build
    R0-->>Pipeline: Compile result

    alt Compile failed
        Pipeline-->>Runner: FAIL at R0
        Runner->>Replan: Build revision with compile errors
        Note right of Replan: Inject error diagnostics<br/>into next attempt prompt
        Replan-->>Runner: Revised plan
    else Compile passed
        Pipeline->>R1: cargo clippy / eslint
        R1-->>Pipeline: Lint result

        Pipeline->>R2: cargo test
        R2-->>Pipeline: Test result

        Pipeline->>R3: Symbol diff (public API check)
        R3-->>Pipeline: Symbol result

        Pipeline->>R456: Generated tests, property tests, integration
        R456-->>Pipeline: Advanced results
    end

    Pipeline->>Thresh: Update EMA thresholds
    Note right of Thresh: Tighten when passing,<br/>relax when failing.<br/>Persist to gate-thresholds.json

    Pipeline-->>Runner: Final verdict + diagnostics

    alt All rungs passed
        Runner->>Runner: Proceed to persist + learn
    else Any rung failed
        Runner->>Replan: Inject failure diagnostics
        Note right of Replan: Include which rung failed,<br/>exact error output,<br/>possibly escalate model
        Replan-->>Runner: Retry from dispatch
    end
```

---

## 4. Knowledge Query {#knowledge-query}

How knowledge is retrieved, scored, and used during prompt composition.
Shows the HDC similarity lookup, tier filtering, and the path from
knowledge entry to prompt layer.

```mermaid
sequenceDiagram
    participant Compose as SystemPromptBuilder<br/>(roko-compose)
    participant Neuro as KnowledgeStore<br/>(roko-neuro)
    participant HDC as HDC Engine<br/>(roko-primitives)
    participant Decay as Decay Engine<br/>(roko-core)
    participant Daimon as DaimonState<br/>(roko-daimon)

    Compose->>Neuro: query("rate limiting", context)
    Note right of Neuro: Search by text match<br/>and HDC similarity

    Neuro->>HDC: Encode query as HDC vector
    HDC-->>Neuro: 10,240-bit query fingerprint

    Neuro->>HDC: Cosine similarity against stored entries
    HDC-->>Neuro: Ranked candidates by similarity

    Neuro->>Decay: Filter by decay curve
    Note right of Decay: 4 variants:<br/>None, HalfLife, TTL, Ebbinghaus
    Decay-->>Neuro: Active (non-expired) entries

    Neuro->>Neuro: Filter by validation tier
    Note right of Neuro: Transient < Working < Reference<br/>Higher tiers get higher weight

    Neuro->>Daimon: Check mood-congruent retrieval
    Note right of Daimon: 85% standard retrieval<br/>15% contrarian retrieval<br/>(mood-incongruent)
    Daimon-->>Neuro: Retrieval bias adjustment

    Neuro-->>Compose: Ranked knowledge entries
    Note right of Compose: Entries become Layer 8<br/>of the 9-layer prompt

    Compose->>Compose: Token-budget the knowledge
    Note right of Compose: Truncate to fit within<br/>available token budget.<br/>High-tier entries prioritized.
```

### Knowledge Tier Progression

After a successful gate-backed execution, knowledge entries can be promoted:

```mermaid
stateDiagram-v2
    [*] --> Transient: New entry created
    Transient --> Working: Confirmed by gate success<br/>(context evidence)
    Working --> Reference: Consistent confirmation<br/>across multiple episodes
    Reference --> Working: Contradicted by new evidence
    Working --> Transient: Failed to reconfirm<br/>(decay timeout)
    Transient --> [*]: Expired (decay curve)
```

---

## 5. Dream Consolidation {#dream-consolidation}

The offline learning cycle that extracts patterns from completed episodes
and promotes them into durable knowledge and playbooks.

```mermaid
sequenceDiagram
    participant Daemon as Dream Daemon<br/>(roko-dreams)
    participant Episodes as EpisodeLogger<br/>(roko-learn)
    participant HDC as HDC Engine<br/>(roko-primitives)
    participant Neuro as KnowledgeStore<br/>(roko-neuro)
    participant Playbook as PlaybookStore<br/>(roko-learn)
    participant Bus as EventBus<br/>(roko-runtime)

    Note over Daemon: Triggered by:<br/>adaptive idle, cron,<br/>or episode count

    Daemon->>Episodes: Batch recent completed episodes
    Episodes-->>Daemon: Episode batch

    rect rgb(45, 40, 55)
        Note right of Daemon: Phase 1: Hypnagogia
        Daemon->>HDC: Compute HDC fingerprint per episode
        HDC-->>Daemon: Fingerprint vectors

        Daemon->>Daemon: Cluster episodes by HDC similarity
        Note right of Daemon: Group similar task shapes<br/>(e.g., "add API endpoint",<br/>"fix test failure")
    end

    rect rgb(40, 45, 55)
        Note right of Daemon: Phase 2: NREM Replay
        Daemon->>Daemon: Replay success patterns
        Note right of Daemon: Extract: what model worked,<br/>which tools were used,<br/>what prompt patterns succeeded

        Daemon->>Neuro: Distill knowledge entries
        Note right of Neuro: 6 types: fact, insight,<br/>heuristic, procedure,<br/>constraint, anti-knowledge
        Neuro-->>Daemon: Stored entries with tiers
    end

    rect rgb(45, 45, 50)
        Note right of Daemon: Phase 3: REM Imagination
        Daemon->>Daemon: Generate counterfactual scenarios
        Note right of Daemon: "What if we had used<br/>a different model?"<br/>"What if the test gate<br/>had been stricter?"

        Daemon->>Playbook: Promote reliable patterns
        Note right of Playbook: when/then rules:<br/>"When task matches X,<br/>then use strategy Y"
        Playbook-->>Daemon: Stored playbooks
    end

    Daemon->>Bus: Publish DreamConsolidated event
    Bus-->>Daemon: Event delivered to watchers

    Daemon->>Daemon: Write dream journal
    Note right of Daemon: .roko/dreams/journal/<br/>timestamped entries
```

### How Consolidated Knowledge Feeds Back

The playbooks and knowledge entries produced by dream consolidation are
injected into future agent dispatches, completing the learning loop:

```mermaid
graph LR
    EXEC["Agent Execution<br/><i>roko-agent</i>"]
    GATE["Gate Verification<br/><i>roko-gate</i>"]
    EPISODE["Episode Recording<br/><i>roko-learn</i>"]
    DREAM["Dream Consolidation<br/><i>roko-dreams</i>"]
    KNOWLEDGE["Knowledge Store<br/><i>roko-neuro</i>"]
    PLAYBOOK["Playbook Store<br/><i>roko-learn</i>"]
    COMPOSE["Prompt Composition<br/><i>roko-compose</i>"]

    EXEC --> GATE
    GATE --> EPISODE
    EPISODE --> DREAM
    DREAM --> KNOWLEDGE
    DREAM --> PLAYBOOK
    KNOWLEDGE --> COMPOSE
    PLAYBOOK --> COMPOSE
    COMPOSE --> EXEC

    style EXEC fill:#fff3e0,stroke:#e65100
    style GATE fill:#fce4ec,stroke:#b71c1c
    style EPISODE fill:#f3e5f5,stroke:#6a1b9a
    style DREAM fill:#f3e5f5,stroke:#6a1b9a
    style KNOWLEDGE fill:#f3e5f5,stroke:#6a1b9a
    style PLAYBOOK fill:#f3e5f5,stroke:#6a1b9a
    style COMPOSE fill:#fff3e0,stroke:#e65100
```

---

## CLI Commands for Each Flow

| Flow | Commands |
|------|----------|
| Plan Execution | `roko prd idea`, `roko prd draft`, `roko prd plan`, `roko plan run` |
| Agent Dispatch | `roko run "<prompt>"`, `roko do "<prompt>"` |
| Gate Validation | Automatic during `roko plan run`; inspect with `roko learn gates` |
| Knowledge Query | `roko knowledge query "<topic>"`, `roko knowledge stats` |
| Dream Consolidation | `roko knowledge dream run`, `roko knowledge dream report` |

---

## Related

- [Architecture Explorer](./architecture) -- interactive component diagram
- [Crate Map](./crate-map) -- dependency graph for all crates
- [Architecture Guide: Data Flow](/35-ARCHITECTURE#6-data-flow-from-idea-to-completed-code) -- narrative walkthrough
