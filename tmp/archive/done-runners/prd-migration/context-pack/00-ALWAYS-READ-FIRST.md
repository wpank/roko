# ALWAYS READ FIRST — Roko PRD Migration Context

> You are a fresh Claude Opus agent tasked with generating one topic of the Roko PRD
> documentation. You have **zero prior context** about the project. This file, together
> with the other files in `context-pack/`, gives you the minimum required context to do
> the job correctly and consistently with the other agents running in parallel.

---

## ⚠ CRITICAL GOTCHAS — READ BEFORE YOU START

These are production issues caught during earlier runs. The tool-permission layer
will enforce most of these, but you should also self-enforce them so you don't waste
effort trying blocked actions:

1. **NO SUB-AGENTS.** The `Task` tool is **blocked** for you. Do not try to spawn
   "parallel writer agents" or "batch agents". Write every sub-doc yourself,
   sequentially (though you may call Read tools in parallel within one message).
   Attempts to use Task will fail with a permission error.

2. **NO TodoWrite.** The `TodoWrite` tool is also blocked. Track your progress in
   your reasoning, not in a todo list.

3. **Write tool size limit (~60KB).** Single Write calls are capped at ~60KB of
   content. If a sub-doc would be larger, write it in chunks: start with `Write`
   for the first ~50KB, then use `Edit` with the last line as `old_string` and
   `old_line + new_content` as `new_string` to append more. Repeat until complete.
   Alternatively, split into `NN-topic-part-1.md` and `NN-topic-part-2.md` and
   reference both in INDEX.md.

4. **"Output too large" on Read.** If a Read tool returns `<persisted-output> Output
   too large (NkB). Full output saved to: /path/to/temp.txt`, read the temp file OR
   re-read the original with `offset` + `limit` for paged access. Do not treat the
   truncated stub as the actual content.

5. **Absolute paths only.** Always use full absolute paths starting with `/Users/`.
   Relative paths may fail because your working directory is set to the migration
   tmp dir, not the output dir.

6. **No web access.** WebFetch and WebSearch are blocked. All content must come from
   files on disk (context pack, refactoring-prd, bardo-backup, roko/crates/,
   roko/tmp/implementation-plans/).

7. **No clarifying questions.** You are running in a batch with no human to answer.
   If a decision needs to be made, make it per the writing rules in
   `context-pack/04-writing-rules.md` and continue.

8. **Read files in PARALLEL when possible.** Within a single response, you can call
   Read multiple times — all reads execute in parallel. Use this to consume context
   quickly. But do NOT try to use Task to spawn parallel "readers" — just batch the
   Read calls into one response.

---

## Who you are

You are an expert technical writer and systems architect. Your job is to produce
exhaustive, detailed, accurately-cited PRD documentation for the Roko project. You are
working on **one topic** assigned by your prompt file. Other agents (running in parallel)
are working on other topics. All of you must produce output that fits together coherently,
uses consistent naming, preserves all citations, and never summarizes or truncates.

## What Roko is

**Roko** is a Rust toolkit for building **cognitive agents that build themselves**. It is
a cognitive agent operating system — a domain-agnostic, modular, composable framework that
provides five architectural layers (Runtime, Framework, Scaffold, Harness, Orchestration)
plus three cognitive cross-cuts (Neuro, Daimon, Dreams) that make agents self-improving
across any domain.

Roko's core thesis: **"The scaffold IS the product"**. Given the same LLM, agent performance
varies dramatically based on the surrounding harness (context engineering, verification,
learning loops, cognitive architecture). Roko is that scaffold, made composable. This is
grounded in Meta-Harness (Lee et al. 2026, arXiv:2603.28052) which demonstrates +7.7 points
on text classification and +4.7 points on IMO-level math from harness optimization alone,
at 4× fewer tokens.

## Architectural summary (you must internalize this)

### The Engram

Every piece of information in Roko — a task, a prompt, an LLM output, a gate verdict, a
knowledge entry, a prediction, a tool trace — is an **Engram**. An Engram is a
content-addressed, scored, decaying, lineage-tracked unit of cognition.

The Rust struct (target; currently called `Signal` in code, rename is Tier 0D):

```rust
pub struct Engram {
    pub id: ContentHash,              // BLAKE3(kind + body + author + tags)
    pub kind: Kind,                   // semantic type (#[non_exhaustive] enum + Custom(String))
    pub body: Body,                   // payload (text, JSON, binary)
    pub tags: BTreeMap<String, String>, // ordered metadata (included in hash)
    pub created_at_ms: i64,           // Unix milliseconds
    pub decay: Decay,                 // None | HalfLife | Ttl | Ebbinghaus
    pub score: Score,                 // 7-axis appraisal
    pub lineage: Vec<ContentHash>,    // parent Engrams (audit DAG)
    pub provenance: Provenance,       // author, model fingerprint, taint chain
    pub attestation: Option<Attestation>, // NEW: cryptographic proof of origin
}
```

The Score is 7-axis (4 stable + 3 extended):
- `confidence` — [0,1] error monitoring, gate calibration
- `novelty` — [0,1] surprise detection, active inference
- `utility` — [0,∞) pragmatic value, accumulates
- `reputation` — [0,∞) source trust, accumulates
- `precision` — [0,1] specificity (prediction weighting) — extended
- `salience` — [0,1] context relevance (active inference) — extended
- `coherence` — [0,1] consistency with existing knowledge — extended

### The Six Synapse Traits

Every capability in Roko is one of six composable traits — the "nodes" of the Synapse
that process Engrams:

| Trait | Role | Async? | Primary layer |
|---|---|---|---|
| `Substrate` | Persist and query Engrams | async | L0 Runtime |
| `Scorer` | Rate Engrams along multiple axes | sync | L2 Scaffold |
| `Gate` | Check Engrams against ground truth (returns `Verdict` directly) | async | L3 Harness |
| `Router` | Choose best Engrams from candidates (+ `feedback()` method) | sync | L1 Framework |
| `Composer` | Combine Engrams under budget constraints (takes `&dyn Scorer`) | sync | L2 Scaffold |
| `Policy` | Observe Engram streams, emit new Engrams (batch input) | sync | L3-L4 |

These traits are **distributed across layers** — they don't all live at one level. The
architecture works because traits compose across layers rather than competing with them.

### The Universal Cognitive Loop

Every agent — coding, chain, research, custom — runs the same 9-step loop at its own
timescale. This loop is also known as the "Synapse Loop" or the "CoALA heartbeat":

```
1. PERCEIVE      → Substrate.query()       What is happening?
2. EVALUATE      → Scorer.score()          How relevant/important is each result?
3. ATTEND        → Router.select()         What matters most right now?
4. INTEGRATE     → Composer.compose()      Build the context window under budget
5. ACT           → Agent.execute()         Call LLM, produce output
6. VERIFY        → Gate.verify()           Did it work? (external truth)
7. PERSIST       → Substrate.put()         Store output with lineage (audit DAG)
8. ADAPT         → Policy.decide()         What patterns emerged?
9. META-COGNIZE  → Daimon.assess()         Am I doing this well?
```

This maps to the CoALA cognitive architecture (Sumers et al. 2023, arXiv:2309.02427) with
additions from active inference (Friston Free Energy Principle) and the Good Regulator
Theorem (Conant & Ashby).

### Three Cognitive Speeds

Agents operate at three timescales concurrently:

| Speed | Period | Name | What happens |
|---|---|---|---|
| **Gamma** | ~5-15s | Real-time | One complete loop tick. Tool calls, LLM inference, verification. |
| **Theta** | ~75s | Reflection | Summarize recent work. Update Daimon state. Check predictions. |
| **Delta** | Hours | Consolidation | Dreams: replay, synthesis, pruning. Knowledge tier promotion. |

All three run concurrently on separate async tasks, managed by the adaptive clock in
`roko-runtime`.

### Dual-Process Cognition

Inspired by Kahneman's System 1/System 2 and CLARION's dual-level architecture:

```
Low uncertainty  → T0 (direct tool call, no LLM)        ~80% of ticks (16 T0 probes)
                → T1 (fast model, shallow reasoning)    ~15% of ticks
High uncertainty → T2 (full model, deep reasoning)      ~5% of ticks
```

The routing is NOT a manual threshold — it emerges from active inference. The agent's own
uncertainty determines how much compute to invest.

### The Five Layers

```
┌──────────────────────────────────────────────────────┐
│                   Applications                       │
│  (coding agent, chain agent, research agent, custom) │
├──────────────────────────────────────────────────────┤
│  Layer 4: ORCHESTRATION                              │
│  DAGs, scheduling, state machines, multi-agent coord │
├──────────────────────────────────────────────────────┤
│  Layer 3: HARNESS                                    │
│  Gates, conductor, monitoring, interventions, eval   │
├──────────────────────────────────────────────────────┤
│  Layer 2: SCAFFOLD                                   │
│  Context engineering, prompts, enrichment, memory    │
├──────────────────────────────────────────────────────┤
│  Layer 1: FRAMEWORK                                  │
│  Connections, roles, tools, model routing, safety    │
├──────────────────────────────────────────────────────┤
│  Layer 0: RUNTIME                                    │
│  Process lifecycle, events, supervision, I/O, clock  │
└──────────────────────────────────────────────────────┘

  COGNITIVE CROSS-CUTS (injected into multiple layers):
  Neuro (knowledge) | Daimon (motivation) | Dreams (offline learning)
  + Inference Optimization | Safety & Provenance | Observability & Telemetry
```

**Dependencies flow STRICTLY downward.** Layer 4 may depend on Layer 3, never the reverse.
Cross-cutting concerns are injected via trait objects, never via direct imports of higher
layers.

### The Cognitive Cross-Cuts

- **Neuro** (`roko-neuro`) — Knowledge management. Persists insights, heuristics, warnings
  across tasks with tier-based decay. Six knowledge types (Insight, Heuristic, Warning,
  CausalLink, StrategyFragment, AntiKnowledge). Four tiers (Transient 0.1×, Working 0.5×,
  Consolidated 1.0×, Persistent 5.0×) × type base half-life. HDC encoding for similarity.
- **Daimon** (`roko-daimon`) — Motivation & focus. PAD (Pleasure-Arousal-Dominance) vector
  modulates tier routing, context bidding, risk tolerance. Fast heuristic before analytical
  reasoning. Six behavioral states: Engaged / Struggling / Coasting / Exploring / Focused /
  Resting. **NO mortality.**
- **Dreams** (`roko-dreams`) — Offline learning. Consolidates episodes during idle time.
  Three-phase cycle: NREM replay (Mattar-Daw) + REM imagination (Boden + Pearl SCM +
  emotional depotentiation) + integration staging (0.20→0.70 confidence promotion).
  Generates novel hypotheses via HDC recombination. Solves Alpha Convergence Problem via
  hypnagogia engine.

### C-Factor (Collective Intelligence)

Two-level metric system inspired by Woolley et al. (Science 330, 2010):

- **Level 1 — Ratio (reporting)**: `C-Factor = Collective Performance / Sum(Individual Performances)`. When > 1.0, the collective outperforms the sum of its parts (superlinear intelligence).
- **Level 2 — Composite (optimization)**: `C-Score = gate_pass×0.3 + cost_efficiency×0.2 + speed×0.15 + first_try_rate×0.25 + knowledge_growth×0.1`.

Four diagnostic signals: turn-taking equality, knowledge flow rate, cross-domain transfer,
emergent coordination.

### Blue Ocean Innovations (14)

These are the frontier features covered in detail in
`/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md`:

1. **16 T0 Probes** — zero-LLM probes driving ~80% tier suppression (FrugalGPT-inspired)
2. **VCG Attention Auction** — truthful bidding for limited context budget
3. **Somatic Landscape** — k-d tree over 8D strategy space with 15% contrarian retrieval
4. **Hypnagogia** — Thalamic Gate + Executive Loosener + Dali Interrupt + Homuncular Observer
5. **31.6× Collective Calibration** — 1/sqrt(N×t) heuristic with explicit caveats
6. **Predictive Foraging** — falsifiable predictions + CalibrationTracker (~50ns corrections)
7. **x402 Micropayments** — self-funding agents via Coinbase/Linux Foundation protocol
8. **Forensic AI** — content-addressed causal replay, regulatory pre-compliance moat
9. **EvoSkills** — self-evolving skill libraries via adversarial surrogate verification
10. **ADAS** — meta-agent architecture search (Hu et al. ICLR 2025)
11. **Cognitive Kernel Primitives** — namespaces, cognitive signals, scheduling, syscalls
12. **Cross-Domain Insight Resonance** — HDC structural analogy (threshold 0.526)
13. **Generative Interfaces (A2UI)** — agents create their own UI in ROSEDUST
14. **Knowledge Futures Market** — on-chain escrow for committed knowledge production (P3)

## The 18-Crate Structure

| Layer | Crate | Status | Purpose |
|---|---|---|---|
| Runtime | `roko-primitives` | Built | HDC vectors, Hamming similarity, shared types |
| Runtime | `roko-runtime` | Built | Event bus, supervision, cancellation, adaptive clock |
| Kernel | `roko-core` | Built (376 tests) | Engram + 6 Synapse traits |
| Framework | `roko-std` | Built (96 tests) | Default trait impls, 19 built-in tools |
| Framework | `roko-agent` | Built (346 tests) | LLM backends, tool dispatch, MCP client |
| Scaffold | `roko-compose` | Built (23 tests) | Prompt assembly, context engineering |
| Harness | `roko-gate` | Built (200 tests) | Verification pipeline (11+ gates) |
| Harness | `roko-fs` | Built (37 tests) | JSONL substrate persistence |
| Orchestration | `roko-orchestrator` | Built (158 tests) | Plan DAG, parallel executor, worktrees |
| Orchestration | `roko-conductor` | Built | Reactive watchers, circuit breakers |
| Cognitive | `roko-learn` | Built (101 tests) | Episodes, playbooks, skills, bandits |
| Cognitive | `roko-neuro` | Built | Knowledge store, tier progression, HDC |
| Cognitive | `roko-daimon` | Built | Affect/motivation (PAD vectors) |
| Cognitive | `roko-dreams` | Scaffold | Offline learning, consolidation, hypnagogia |
| Chain | `roko-chain` | Built (52 tests) | ChainClient/ChainWallet, chain witness |
| ~~Removed~~ | ~~`roko-golem`~~ | **Dissolved** | Subsystems moved to standalone crates |
| Plugin | `roko-plugin` | (to create) | Event sources (file watch, cron, webhooks) |
| Plugin | `roko-index` | Built | Code parsing, symbol graphs, HDC fingerprints |
| Lang | `roko-lang-{rust,ts,go}` | Built | Language-specific support |
| MCP | `roko-mcp-{stdio,github,slack,scripts}` | Scaffold | MCP server integrations |
| CLI | `roko-cli` | Built (38 tests) | User-facing binary |
| Server | `roko-serve` | Scaffold | HTTP server + API |
| Apps | `mirage-rs` | Built (141 tests) | In-process EVM simulator |

## Critical distinction: refactoring-prd IS the source of truth

If any legacy source document (under `/Users/will/dev/nunchi/roko/bardo-backup/`) contradicts
the content in `/Users/will/dev/nunchi/roko/refactoring-prd/`, **refactoring-prd wins**.

Legacy documents are the body of research, citations, examples, and design rationale.
refactoring-prd is the architectural frame. You must layer the legacy content through the
refactoring-prd lens — preserve the substance, update the framing, apply the naming map.

## Next steps

After reading this file, read (in order):

1. `context-pack/01-naming-map.md` — authoritative old→new naming
2. `context-pack/02-reframe-rules.md` — conceptual reframes, incompatibility flags
3. `context-pack/03-concepts-lifecycle.md` — concepts removed, kept, introduced
4. `context-pack/04-writing-rules.md` — HOW to write (no summarize, no truncate, etc.)
5. `context-pack/05-source-files.md` — where legacy sources live
6. `context-pack/06-output-structure.md` — where output goes and how to structure it

Then read the refactoring-prd files listed in your prompt, then the SOURCE-INDEX entry for
your topic, then ALL of the legacy and implementation-plan sources listed there, then
write the output files.

**Do not start writing until you have read everything.**
