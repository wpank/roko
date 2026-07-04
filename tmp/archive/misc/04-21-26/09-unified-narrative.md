# The Unified Narrative: HDC + Chain + Context + Arenas

## The One-Paragraph Version

Roko agents are long-lived processes with a heartbeat. On every tick, they OBSERVE
the environment, RETRIEVE knowledge by querying the local neuro store AND the Korai
chain's HDC layer, then GATE to decide whether the observation is novel enough to
warrant an LLM call (80% of the time it isn't — $0). When they do reason, the prompt
is assembled via a VCG auction where knowledge from the chain competes for context
window budget against local playbooks, code intelligence, research, and other sources.
After execution, outcomes are fingerprinted as HDC vectors, clustered, and the best
insights are published back to Korai — PP-HDC encoded for privacy. The arena framework
provides measurement: does chain knowledge actually improve scores? The section
effectiveness registry gives the answer per-section, per-role, per-arena. Everything
compounds.

## Performance: Why It's NOT Seconds

The user asked: "what is the performance impact? It sounds like it might be seconds."

**It's microseconds.** Here's the actual budget:

| Operation | Time | When |
|-----------|------|------|
| HDC fingerprint (serde + FNV + splitmix64) | ~5 μs | Per episode |
| HDC similarity (XOR + popcount, 160 words) | <1 μs | Per comparison |
| Somatic marker check (k-d tree, ~1000 entries) | <100 μs | Per dispatch |
| Local neuro query (brute-force, ~10K entries) | ~10 ms | Per dispatch |
| Episode clustering (k-medoids, 500 episodes) | ~50 ms | Every 50 episodes (background) |
| Cross-domain resonance (~100 patterns) | ~1 ms | Per dispatch |
| VCG auction (8 bidders, ~50 sections) | <1 ms | Per prompt assembly |
| Section effectiveness lookup (HashMap) | <1 μs | Per section |

**The chain query is the only potentially slow part.** But:
- Mirage (local EVM): ~1-5 ms for `hdc_topk`
- Korai (L3, 400ms blocks): ~50-200 ms for RPC round-trip

**Mitigation**: Cache. The chain query result is valid for the duration of a task
(knowledge doesn't change per-turn). Query once at task start, cache the top-k
results, use them for all turns within that task. Total added latency per task:
one chain RPC call (~100ms). Per turn: 0ms (cached).

For comparison: a single LLM call takes 5-60 seconds. The HDC + chain overhead
is <0.1% of total task time. Invisible.

**The T0 gating is where the real performance win lives.** When the Golem heartbeat
ticks and the cognitive gate says "I've seen this pattern before" (via somatic
marker check, <100μs), the agent skips the LLM entirely. That's not saving
milliseconds — it's saving $0.05 per tick × 80% of ticks = massive cost reduction.

## How It All Fits Together (The Full Flow)

### Phase 0: Agent Boots

```
Agent<Provisioning>
  → load extensions (Neuro, Daimon, Chain, Conductor, Code, Playbooks, ...)
  → extensions.on_boot()
    → NeuroExtension: load local knowledge store
    → ChainExtension: connect to Mirage/Korai RPC
    → DaimonExtension: initialize PAD state
    → ConductorExtension: initialize 10 watchers
  → activate() → Agent<Active>
```

### Phase 1: Heartbeat Tick (Gamma, every 5-15s)

```
Agent<Active>.tick()
  → OBSERVE: read environment
      ChainExtension: check new blocks (Binary Fuse filter, ~10ns reject)
      CodeExtension: check file changes
      EventFabric: dequeue events
  → RETRIEVE: query knowledge
      NeuroExtension: HDC query local store (~10ms)
      ChainExtension: hdc_topk on Korai (~100ms, CACHED per task)
        → returns top-20 knowledge entries
        → weighted by submitter reputation
        → PP-HDC encoded (privacy-safe)
  → GATE: decide cognitive tier
      SomaticMarkerCheck: query k-d tree (<100μs)
        → if similar to known pattern → T0 (no LLM, $0)
        → if novel but simple → T1 (Haiku, $0.001)
        → if novel and complex → T2 (Opus, $0.05)
```

### Phase 2: T1/T2 Reasoning (When Gated In)

```
  → ASSEMBLE CONTEXT (via VCG auction):
      8 bidders compete for context window:
        Neuro: chain knowledge entries (reserved 15% budget)
        Daimon: affect guidance
        PlaybookRules: successful strategies from prior episodes
        CodeIntelligence: workspace symbols, files
        Research: external domain context
        TaskContext: current task brief, plan
        Oracles: predictions, warnings
        IterationMemory: recent turns

      VCG auction:
        → sort by value density (relevance × reputation × learned lift) / tokens
        → greedily allocate until budget exhausted
        → second-price payments (fair)
        → affect modulation (arousal → urgency, pleasure → valence bias)

  → INFERENCE: CascadeRouter picks model
      HD-CB (HDC contextual bandit) uses cluster features:
        "this task fingerprint is in cluster 7"
        "model X has 85% pass rate for cluster 7"
        → select model X

  → EXECUTE: tool calls, code edits, etc.

  → VERIFY: run gates (compile, test, clippy, diff, etc.)
```

### Phase 3: After Execution

```
  → RECORD:
      fingerprint_episode(prompt, outcome) → 10,240-bit HDC vector
      record_completed_run():
        → CascadeRouter.observe(context, model, success)
        → playbook_store.record(task, success)
        → episode_logger.append(episode)
        → efficiency_event.emit(tokens, cost, latency)
        → experiment_store.record_variant(variant, success)
        → section_effectiveness.update(included, gate_passed)
        → somatic_markers.insert(task_fingerprint, outcome)

  → CLUSTER (every N episodes, background):
      k_medoids(recent_fingerprints, k=10) → clusters
        → persist to .roko/learn/episode-clusters.json
        → detect cross-domain resonance
        → update resonant pattern Lotka-Volterra dynamics

  → PUBLISH (if episode is publishable):
      distill episode → knowledge entry
      HDC encode → PP-HDC hash-encode (privacy)
      submit to Korai chain with reputation stake
        → other agents' future hdc_topk queries find this
```

### Phase 4: Dream Cycle (Delta, every ~50 ticks)

```
  → sleep pressure exceeds threshold
  → begin_dream() → Agent<Dreaming>

  Dream cycle:
    NREM Replay: Mattar-Daw utility-weighted episode selection
      → replay high-utility episodes
      → extract heuristics from successful patterns
    REM Imagination: Pearl SCM counterfactual reasoning
      → "what if I had used a different model?"
      → "what if the prompt had included chain knowledge?"
    Integration Staging: promote knowledge tiers
      → Transient → Working → Consolidated → Persistent
      → confidence increases with cross-validation

  → wake() → Agent<Active>
  → resume ticking
```

## Where Things Connect

### HDC Is The Connective Tissue

HDC vectors appear at every stage:

| Stage | HDC Role |
|-------|---------|
| Observe | Binary Fuse filter (probabilistic HDC) rejects 90% of chain events |
| Retrieve | hdc_topk queries chain for similar knowledge |
| Gate | Somatic marker check (fingerprint → k-d tree → gut feeling) |
| Assemble | Neuro bidder packages HDC-retrieved entries for auction |
| Execute | (no HDC involvement during LLM call) |
| Record | fingerprint_episode encodes outcome as vector |
| Cluster | k-medoids groups similar episodes |
| Publish | PP-HDC hash-encode before chain submission |
| Dream | Replay selection weighted by HDC novelty |

### Korai Is The Shared Memory

Without Korai: each agent's knowledge dies when the process ends. With Korai:

1. **Agent A** solves a Django migration issue in SWE-bench arena
2. Records episode → fingerprints → publishes to Korai (PP-HDC encoded)
3. **Agent B** gets a similar Django task → queries Korai → hdc_topk returns A's insight
4. VCG auction gives the insight a reserved budget slot (15%)
5. B succeeds faster because it starts with A's knowledge
6. B's success → publishes refined insight → next agent gets even better knowledge

Multiply by N agents across M arenas. Knowledge compounds.

### The Arena Measures Everything

SWE-bench (and other arenas) are the measurement instrument:

```
Batch 1: chain empty → baseline score (e.g., 15% resolved)
Batch 5: chain has 200 entries → score rises (e.g., 19%)
Batch 20: chain has 1000 entries → score plateaus higher (e.g., 24%)
```

The section effectiveness registry tells you EXACTLY how much the chain contributed:
```
neuro_included_pass_rate: 0.27
neuro_excluded_pass_rate: 0.18
lift: +0.09 (9 percentage points)
```

If the lift is positive, the chain is working. If not, something is wrong with the
knowledge quality, reputation gating, or retrieval threshold.

## Synergy with Generalizations Work

The `generalizations/` documents describe the same architectural shift:
agents need to be persistent processes (not spawn-die), with heartbeat loops,
event subscriptions, and extension-based composition.

**The chain integration IS the motivation for the Golem runtime.** Without
persistent agents, you can't:
- Subscribe to chain blocks (need continuous heartbeat)
- Cache chain queries across turns (need agent state)
- Accumulate somatic markers (need persistent k-d tree)
- Run dream cycles (need sleep pressure accumulation)
- Gate at T0 (need pattern memory across ticks)

The `AgentRuntime` trait from `04-agent-runtime-design.md` is exactly the
structure needed. The `assemble_context()` extension hook is where the
chain query + VCG auction + HDC retrieval happens. The `on_tick_start()`
hook is where the somatic marker check and cognitive gating happen.

**Bottom line**: The chain integration and the Golem runtime redesign are
the same work, approached from different angles. Doing them together is
natural — you can't have one without the other.
