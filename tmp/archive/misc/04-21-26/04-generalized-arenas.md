# Generalized Arenas: Domain-Agnostic Self-Learning

## The Abstraction

SWE-bench is just one **arena** — a domain where agents get tasks, produce outputs, and
receive verifiable pass/fail signals. The learning infrastructure (CascadeRouter, playbooks,
experiments, adaptive thresholds, episode logging) is domain-agnostic. It works on anything
where you can define:

1. **Task source** — where instances come from (dataset, chain events, API, queue)
2. **Verification** — how to know if the output is correct (gates)
3. **Outcome signal** — pass/fail that feeds back into learning

The arena abstraction unifies SWE-bench, blockchain monitoring, security auditing, research
synthesis, operations, and any future domain under one concept:

```rust
trait Arena {
    /// Human-readable name
    fn name(&self) -> &str;

    /// Produce the next batch of tasks
    async fn sample(&self, batch_size: usize) -> Vec<TaskDef>;

    /// Domain-specific gate configuration for each task
    fn gates_for(&self, task: &TaskDef) -> Vec<GateConfig>;

    /// Aggregate outcomes into a domain-specific score
    fn score(&self, episodes: &[Episode]) -> ArenaScore;

    /// Optional: domain-specific prompt enrichment
    fn enrich_prompt(&self, task: &TaskDef, context: &ArenaContext) -> Option<String> {
        None
    }
}
```

Everything else — dispatch, learning, persistence, replan — is handled by the existing
orchestrator. An arena is just a task factory + gate config + score aggregator.

## The Universal Loop

```
┌─────────────────────────────────────────────────────────┐
│                    Arena (domain)                         │
│                                                           │
│  sample() → Vec<TaskDef>                                 │
│  gates_for() → Vec<GateConfig>                           │
│  enrich_prompt() → domain context                        │
│                                                           │
├───────────────────────┬───────────────────────────────────┤
│                       │                                   │
│              ┌────────▼────────┐                          │
│              │   Orchestrator   │                          │
│              │  (plan run)      │                          │
│              └────────┬────────┘                          │
│                       │                                   │
│   ┌───────────────────┼───────────────────┐              │
│   │                   │                   │              │
│   ▼                   ▼                   ▼              │
│ Compose            Dispatch            Gate              │
│ (SystemPrompt      (CascadeRouter     (domain gates     │
│  Builder +          picks model)       verify output)    │
│  arena enrichment)                                       │
│                                                           │
│              ┌────────────────────┐                       │
│              │  record_completed  │                       │
│              │  _run()            │                       │
│              └────────┬───────────┘                       │
│                       │                                   │
│   ┌───────────┬───────┼───────┬───────────┐              │
│   ▼           ▼       ▼       ▼           ▼              │
│ Cascade    Playbook  Episode  Prompt    Adaptive          │
│ Router     Store     Logger   Expts     Thresholds       │
│                                                           │
│              ┌────────▼────────┐                          │
│              │  score()        │ → .roko/bench/scores     │
│              └────────┬────────┘                          │
│                       │                                   │
│                  next batch                               │
│              (learning carries over)                      │
└─────────────────────────────────────────────────────────┘
```

## What Makes This Powerful

### 1. Cross-Arena Transfer

Knowledge learned in one arena can transfer to another:

- **Playbooks** from SWE-bench (successful patch patterns) inform code-editing tasks
  in the self-hosting arena
- **Model routing** learned from blockchain monitoring (which models handle time-pressure
  well) informs security auditing under deadlines
- **Prompt experiments** that win in research synthesis may also win in PRD drafting

The neuro store (HDC-indexed knowledge) enables this: insights are encoded as 10,240-bit
vectors with semantic similarity search. An insight learned in arena A can be retrieved
when a similar situation arises in arena B.

### 2. Concurrent Arena Execution

Multiple arenas can run simultaneously, sharing the same learning state:

```bash
# Terminal 1: SWE-bench grinder
roko arena run swe-bench --repeat 0

# Terminal 2: blockchain monitoring
roko arena run chain-monitor --repeat 0

# Terminal 3: self-hosting (roko developing itself)
roko plan run plans/

# All three write to the same:
#   .roko/learn/cascade-router.json
#   .roko/learn/playbooks/
#   .roko/episodes.jsonl
#   .roko/learn/experiments.json
```

The CascadeRouter observes outcomes from all arenas simultaneously. A model that excels
at SWE-bench but fails at chain monitoring gets routed to code tasks only. This is the
contextual bandit working as designed — the context vector encodes task type, domain,
complexity, and the LinUCB learns the full interaction.

### 3. Arena-Specific Specialization

Each arena can define domain-specific:

- **Gates**: SWE-bench uses `git apply --check` + pytest. Chain uses tx simulation + balance checks. Security uses formal verification.
- **Prompt enrichment**: Chain arena injects recent block data. Research arena injects citations. Code arena injects symbol context.
- **Scoring**: SWE-bench measures % resolved. Chain measures prediction accuracy + profit. Research measures citation quality.
- **Pacing**: SWE-bench can run in bulk batches. Chain must react in real-time. Research can run overnight.

### 4. The Flywheel

Each arena generates three things that compound:

1. **Training signal** — pass/fail outcomes that tune the CascadeRouter
2. **Playbooks** — successful strategies that get injected into future prompts
3. **Knowledge** — insights that persist in the neuro store across sessions

More arenas → more diverse training signal → better routing → better performance
across all arenas → more successful episodes → richer playbooks → ...

This is a genuine network effect across domains. The system gets better at everything
by doing anything.

## The Meta-Arena

The most interesting arena is **roko developing roko** — the self-hosting loop is itself
an arena where:

- Task source = PRDs and implementation plans
- Gates = compile + test + clippy + diff review
- Outcome = did the feature work?

Improvements to the arena system itself (better gates, better routing, better prompts)
are tasks in the self-hosting arena. The system improves its own improvement loop.

This is the fixed-point: the cybernetic system that tunes itself, running in the arena
framework that it also tunes.

## The HuggingFace Amplifier

HuggingFace turns the flywheel into a network:

```
Local learning (one roko instance)
  → publish to Hub (playbooks, models, episodes)
    → other instances discover via Hub API
      → pull and explore (CascadeRouter adds as new arms)
        → their learning → publish back → ...
```

Plus the fine-tuning loop (Layer 5 from the HF integration doc):

```
Arena outcomes (successful episodes)
  → upload as training dataset to Hub
    → AutoTrain fine-tunes base model
      → push fine-tuned model to Hub
        → CascadeRouter adds as new arm
          → if it wins → more traffic → more training data → ...
```

The combination of multi-arena learning + Hub publishing + fine-tuning creates
three nested feedback loops:

1. **Inner loop** (per-turn): CascadeRouter updates model weights
2. **Middle loop** (per-batch): Playbooks and experiments converge
3. **Outer loop** (periodic): Fine-tuned models get trained and deployed

Each loop runs at a different timescale, and they compound.

## What This Looks Like in Practice

### Scenario: Roko running 4 arenas simultaneously

```
Arena 1: SWE-bench-Lite (batch mode, 50 instances/batch)
  - Learning: which models solve which issue types
  - Playbooks: "Django ORM issues → modify queryset, not model"
  - Score trend: 12% → 18% → 23% over 10 batches

Arena 2: Chain monitor (real-time, Ethereum mainnet)
  - Learning: which models predict MEV opportunities
  - Playbooks: "sandwich attack detection → check mempool ordering"
  - Score trend: 34% prediction accuracy → 41% → 47%

Arena 3: Self-hosting (PRD-driven, continuous)
  - Learning: which models write better Rust
  - Playbooks: "borrow checker errors → add lifetime annotations first"
  - Score trend: 67% gate pass rate → 74% → 79%

Arena 4: Security audit (batch mode, CVE dataset)
  - Learning: which models find vulnerabilities
  - Playbooks: "SQL injection → check parameterized queries"
  - Score trend: 8% → 11% → 15%

Cross-arena transfer:
  - Arena 3's Rust playbooks help Arena 1 on Rust-based SWE-bench instances
  - Arena 2's time-pressure model routing helps Arena 4 on urgent CVEs
  - Arena 1's patch formatting playbooks help Arena 3 produce cleaner diffs
```

Every arena makes every other arena better. That's the network effect.
