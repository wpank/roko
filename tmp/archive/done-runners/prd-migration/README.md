# PRD Migration

This folder holds the migration plan for producing the next-generation Roko PRD docs from
the ~600 legacy source documents scattered across `bardo-backup/prd/`, `bardo-backup/tmp/`,
and the live implementation plans in `roko/tmp/implementation-plans/`.

**Goal**: generate a clean, consolidated set of Roko PRD docs at
`/Users/will/dev/nunchi/roko/roko/docs/` that describe the system in the **new architecture,
paradigm, naming, and design patterns**.

---

## Canonical source of truth

When any legacy doc contradicts the new architecture, **`/Users/will/dev/nunchi/roko/refactoring-prd/` wins**.
It is the latest iterated spec and defines naming, layer taxonomy, traits, cognitive subsystems,
and innovations. Every migration prompt must read the relevant refactoring-prd file(s) first,
then layer the legacy content through that lens.

The refactoring-prd set (12 files):

| # | File | Covers |
|---|---|---|
| 00 | `00-overview.md` | Synapse Architecture, naming map, crate map, C-Factor, recommended reading order |
| 01 | `01-synapse-architecture.md` | Engram struct + 7-axis Score, 6 Synapse traits, cognitive loop, provenance/attestation, active inference, cybernetic loops, composability |
| 02 | `02-five-layers.md` | L0 Runtime, L1 Framework, L2 Scaffold, L3 Harness, L4 Orchestration; trait×layer map; dependency rules; stigmergy |
| 03 | `03-cognitive-subsystems.md` | Neuro (6 knowledge types, tiers, HDC), Daimon (PAD, somatic markers, behavioral states), Dreams (3-phase), Oracles, cybernetic self-learning, VSM mapping |
| 04 | `04-knowledge-and-mesh.md` | Korai chain, HDC on-chain precompile, KORAI/DAEJI tokenomics (demurrage), agent mesh (WS/Iroh), ERC-8004, stigmergy, C-Factor |
| 05 | `05-agent-types.md` | Coding/Chain/Research/Ops/Cross-domain compositions, extensibility, niche construction, somatic coding |
| 06 | `06-interfaces.md` | CLI, HTTP API, TUI (29 screens), Web Portal, ROSEDUST design language, Spectre creature visualization |
| 07 | `07-implementation-priorities.md` | Tier 0–6 roadmap, crate renames, `roko-golem` dissolution plan, dropped items, kept/reframed items |
| 08 | `08-translation-guide.md` | **Key doc**. Old→new renames, conceptual reframes, flagged incompatibilities, citation preservation rules |
| 09 | `09-innovations.md` | Frontier features: 16 T0 probes, VCG auction, somatic landscape, hypnagogia, 31.6× collective calibration, forensic AI, EvoSkills, ADAS, cognitive kernel primitives, HDC cross-domain resonance, knowledge futures, resolved algorithms (EFE, foraging stopping, VCG bid, somatic dims, dream scheduling) |
| 10 | `10-developer-guide.md` | Quick start, trait cookbook, domain plugin walkthrough (medical example), WASM/Docker/daemon/edge deployment, plugin system, testing patterns |
| — | `MIGRATION-CHECKLIST.md` | Parallel checklist targeting `bardo-backup/prd-updated/`; this file augments it with the 22-doc tmp/prd-migration topology |

---

## Authoritative naming map

Applied to **every** generated doc. If a naming conflict exists between older notes and this
table, this table wins.

| Old | New | Notes |
|---|---|---|
| Bardo | **Roko** | Overall framework/project |
| Mori | **Roko Orchestrator** | Build/coding orchestration; often just "orchestrator" |
| Golem / Golems | **Agent / Agents** | The autonomous entity. Do NOT map to "roko" — the framework is roko, entities are agents |
| Grimoire | **Neuro** / `roko-neuro` / `NeuroStore` | Knowledge subsystem |
| Styx | **Agent Mesh** / **Mesh** | P2P relay + permissioned subnets |
| GNOS (token) | **KORAI** (mainnet token) / **DAEJI** (testnet token) | Token references |
| Korai (chain name) | **Korai** | Dedicated EVM chain. Mainnet. Replaces Styx. |
| Daeji | **Daeji** | Testnet for Korai. |
| Clade | **Collective** / **Mesh** | Groups of cooperating agents. Do NOT use "fleet". |
| golem.toml | **roko.toml** | Config file |
| `golem-*` / `bardo-*` crate names | `roko-*` | All crates |
| `bardo-primitives` | **`roko-primitives`** | HDC vectors, shared types |
| `bardo-runtime` | **`roko-runtime`** | Event bus, process supervision, adaptive clock |
| `roko-golem` | **Dissolved** | Subsystems redistributed: Daimon→`roko-daimon`, Dreams+Hypnagogia→`roko-dreams`, Grimoire→`roko-neuro`, ChainWitness→`roko-chain`, Mortality→removed |
| `Signal` (architecture noun) | **`Engram`** | Core data type. The existing Rust type is still named `Signal` in code — rename is Tier 0D |
| `SignalBuilder` | `EngramBuilder` | |
| `signal.rs` | `engram.rs` | |
| "1 noun, 6 verbs" | **Synapse Architecture** | Architecture branding |
| Bardo Sanctum | **Roko Portal** | Web dashboard |
| bardo-terminal | **Roko TUI** | Terminal dashboard |
| Mori parity | Roko parity | Checklist reference |

---

## Concepts removed

| Concept | Reason |
|---|---|
| Natural death / death protocol / Thanatopsis | Agents don't have natural death. Users create and delete agents. |
| Bloodstain | Death artifact — no longer applicable |
| Katabasis | Deep memory descent tied to death — removed |
| Necrocracy | Governance by dead agents — removed |
| Three mortality clocks (economic, epistemic, stochastic) | Reframed as budget limits + confidence tracking. Stochastic death removed entirely. |
| Vitality gauge (Thriving → Terminal phases) | Reframed as Daimon PAD behavioral states (Engaged ↔ Struggling ↔ Coasting ↔ Exploring ↔ Focused ↔ Resting). Cyclical, not terminal. |
| Succession / generational knowledge transfer / Thanatopsis phase | Replaced by user-controlled knowledge backup/restore + mesh sharing |
| Death-themed UX (terminal requiem, death animations, degraded ambient music) | Removed entirely. Spectre animations reflect cognitive state, not mortality. |
| `roko-golem` umbrella crate | Dissolved — subsystems must be independently composable (see 07-implementation-priorities.md §Tier 0C) |
| Mori vs. Golem separation | Unified under Roko. Chain is a domain plugin, not a separate agent type. |
| "Fleet" as group name | Use **Collective** or **Mesh** instead (memory was wrong on this) |

---

## Concepts kept (and what changed)

### Kept unchanged
- **Mirage / mirage-rs** — in-process EVM simulator (Korai proxy during dev)
- **CoALA** — 9-step cognitive cycle, mapped into universal loop
- **HDC / VSA** — 10,240-bit BSC vectors, XOR bind, majority bundle, Hamming similarity
- **Stigmergy** — generalized beyond termites to git commits, code patterns, HDC pheromones
- **Pheromone / Pheromone system** — typed Engrams with Threat/Opportunity/Wisdom/Alpha/Pattern/Anomaly/Consensus decay profiles
- **Sleepwalker** — reduced-capability sleep mode
- **Oneirography / Hypnagogia** — dream journals, hypnagogia engine for alpha convergence
- **ALMA** — three-layer temporal affect model (emotion/mood/personality)
- **Somatic markers** — Damasio 1994; now implemented as k-d tree over 8D strategy space
- **Bazaar / MPP** — commerce primitives
- **All academic citations** — ~200+ papers preserved

### Kept, reframed through Synapse lens
- **Daimon** — PAD affect engine. Reframed: tracks cognitive performance (task success, urgency, confidence) instead of mortality anxiety. Drives tier routing, VCG bidding, somatic retrieval, behavioral state display.
- **Dreams** — Offline learning. Reframed: triggered by idle time / schedule, NOT by death proximity. Three-phase: NREM replay (Mattar-Daw utility), REM imagination (Boden's 3 creativity modes + Pearl SCM), integration staging (0.20→0.70 confidence promotion).
- **Neuro (formerly Grimoire)** — Knowledge store. Semantic wrapper around `Substrate`. Six knowledge types (Insight/Heuristic/Warning/CausalLink/StrategyFragment/AntiKnowledge) × four tiers (Transient 0.1×, Working 0.5×, Consolidated 1.0×, Persistent 5.0×). Ebbinghaus decay with tier multiplier.
- **Sonification** — Musical layers kept. Eno mandate preserved. Preset catalog remapped to behavioral states, NOT mortality phases. No terminal requiem.
- **Spectre creature visualization** — Kept. Reframed: visualizes Daimon PAD cognitive state, not vitality/mortality. Never "dies"; adapts.
- **ROSEDUST design language** — Kept unchanged. Rose on violet-black, glass morphism, luxury easing, applies to all visual interfaces.
- **Lifecycle** — Replaces "mortality". Covers creation, provisioning, deletion, knowledge transfer via backup/restore. User-directed, not biological.
- **TUI 29 screens** — Kept. Adds Spectre viewport, C-Factor dashboard, Neuro tier visualization.

---

## Concepts newly introduced

These are in the new architecture and did not exist (or were unnamed) in the legacy docs.

### Core
- **Engram** (replaces Signal as architectural noun) — content-addressed, scored, decaying, lineage-tracked unit of cognition. BLAKE3(kind+body+author+tags).
- **Synapse Architecture** — the 6-trait composition (Substrate/Scorer/Gate/Router/Composer/Policy) crystallized across 5 layers.
- **7-axis Score** — confidence, novelty, utility, reputation (existing) + **precision, salience, coherence** (new).
- **Attestation** — optional cryptographic proof on Engrams (Ed25519 signature + optional ChainAttestation).

### Layers
- **Five Layers**: L0 Runtime, L1 Framework, L2 Scaffold, L3 Harness, L4 Orchestration. Dependencies flow strictly downward. Cross-cuts injected via trait objects.
- **Cognitive Cross-Cuts**: Neuro, Daimon, Dreams (+ inference optimization, safety/provenance, observability).

### Cognitive primitives
- **Three cognitive speeds**: Gamma (~5-15s reactive), Theta (~75s reflective), Delta (~hours consolidation). From `roko-runtime` adaptive clock.
- **Dual-Process (System 1 / System 2)**: T0 (no LLM) / T1 (fast) / T2 (deep) cascade driven by prediction error.
- **Active inference / EFE**: Expected Free Energy for context selection and action selection. Pragmatic + Epistemic value, balanced by uncertainty. Zero hyperparameters for explore/exploit.
- **Good Regulator Theorem / Ashby's Law** — agents must model themselves; internal variety must match environmental variety.
- **VSM (Beer)** — Roko's 5 layers map to Beer's 5 recursive systems.

### Frontier innovations (from `09-innovations.md`)
- **16 T0 Probes** — zero-LLM probes that drive ~80% tier suppression.
- **VCG Attention Auction** — Vickrey-Clarke-Groves mechanism for truthful subsystem bidding on limited context budget.
- **Somatic Landscape** — k-d tree over 8D strategy space. Damasio somatic markers with mandatory 15% contrarian retrieval (Bower).
- **Hypnagogia engine** — ThalalamicGate + ExecutiveLoosener + DaliInterrupt + HomuncularObserver. Solves the Alpha Convergence Problem by forcing experiential divergence.
- **Three-phase Dream Engine** — NREM (Mattar-Daw replay) + REM (Boden + Pearl SCM + emotional depotentiation) + Integration (SQLite staging buffer).
- **31.6× Collective Calibration** — heuristic `1/sqrt(N×t)` scaling (CLT-inspired) with explicit caveats.
- **Predictive Foraging** — falsifiable predictions as learning signal; CalibrationTracker.
- **x402 Micropayments** — self-funding agent loop via Coinbase x402 (Linux Foundation).
- **Forensic AI / Causal Replay Engine** — content-addressed lineage enables cryptographically verifiable decision replay; regulatory pre-compliance moat (EU AI Act, SEC/CFTC, HIPAA, SOX).
- **EvoSkills** — self-evolving skill libraries via adversarial surrogate verification.
- **ADAS** — Automated Design of Agentic Systems; meta-agent architecture search.
- **Cognitive Kernel Primitives**: Cognitive Namespaces (isolated knowledge), Cognitive Signals (Pause/Resume/Reprioritize/InjectContext/Escalate/Cooldown/Explore/Shutdown), Cognitive Scheduling, Engram Syscalls.
- **Cross-Domain Insight Resonance** — HDC structural analogy detection across domains (threshold 0.526 for 10,240-bit with Bonferroni correction).
- **Generative Interfaces (A2UI)** — agents create their own UI using ROSEDUST primitives.
- **Distributed Context Engineering** — Write/Select/Compress/Isolate strategies at network scale.
- **Knowledge Futures Market** (deferred P3) — on-chain escrow for committed knowledge production.

### Collective intelligence
- **C-Factor (ratio)**: `Collective / Sum(Individual)`. Reporting metric. >1.0 = superlinear.
- **C-Score (composite)**: `gate_pass×0.3 + cost_eff×0.2 + speed×0.15 + first_try×0.25 + knowledge_growth×0.1`. Optimization metric.
- **Turn-taking equality, knowledge flow rate, cross-domain transfer, emergent coordination** — the 4 diagnostic signals (Woolley et al. 2010).

### Identity, chain, mesh
- **Korai chain** — dedicated EVM for agent coordination. 400ms blocks. HDC precompile.
- **KORAI token** (mainnet) with 1% annual demurrage. DAEJI on testnet.
- **ERC-8004** — agent identity (ERC-721 soulbound), reputation registry, validation registry.
- **Agent Mesh** — WebSocket (co-located) + Iroh (NAT-traversing P2P) + ERC-8004 (discovery).
- **Permissioned subnets** — company collectives with private knowledge meshes.

### Visualization
- **Spectre** — procedurally generated creature per agent; encodes behavioral state, knowledge complexity, activity, health, mesh connections, pheromone emission. Simplified ASCII/Unicode in TUI, WebGL in portal.
- **ROSEDUST** — dark-only design system. Void-black + rose accents + jade/amber/crimson/violet/sapphire signals.

---

## Migration principles

1. **refactoring-prd first, legacy second.** Every migration prompt must read the relevant
   refactoring-prd file(s) before touching legacy material. Legacy content is the body of
   research/citations/examples; refactoring-prd is the frame.
2. **Keep ALL academic citations.** Every paper, every reference, every quote — even if the
   surrounding narrative changes. Citations are the intellectual foundation.
3. **Keep research context and design rationale.** The "why" stays. Only the framing changes.
4. **Rewrite implementation details** to reference roko's crate structure (listed in 00-overview
   "Crate Map"), not the old monolith.
5. **Apply the 5-layer taxonomy** (Runtime / Framework / Scaffold / Harness / Orchestration).
   Every subsystem lives at a specific layer.
6. **Integrate Synapse Architecture language** (Engrams flowing through 6 traits).
7. **Domain-agnostic core** — chain is a domain plugin (`roko-chain`), not the default framing.
   Coding is another domain plugin. Research, ops, medical, etc. are domain plugins.
8. **Remove all death/mortality framing** — reframe as resource constraints, cognitive pressure,
   lifecycle management (per `08-translation-guide.md` §2–§4).
9. **Cognitive subsystems are cross-cuts**, not layers. Neuro/Daimon/Dreams are injected into
   multiple layers via trait objects.
10. **Composability principle** — every subsystem must stand alone and be independently usable.
    No umbrella crates. Users compose what they need.
11. **Implementation-plans content** is supplementary — provides concrete task breakdowns,
    verification criteria, and gap analyses. Weave into the "Status / Gaps / Priority" sections
    of target docs, not the conceptual sections.

---

## Reframe rules (summary of `08-translation-guide.md`)

When you encounter legacy language, apply these rewrites:

| Legacy pattern | Rewrite as |
|---|---|
| "Because the golem is dying..." | "Because the agent's budget/confidence/time is constrained..." |
| "Thriving → Stable → Conservation → Declining → Terminal" | "Engaged → Focused → Exploring → Struggling → Resting" (Daimon PAD states) |
| "Economic death / burn-rate clock" | "Budget exhaustion / resource constraint function" |
| "Epistemic death / knowledge freshness clock" | "Prediction accuracy decline / knowledge plateau" |
| "Stochastic death / Weibull clock" | REMOVED (no random death) |
| "Succession / thanatopsis / generational inheritance" | "Knowledge backup/restore + mesh sharing" |
| "Dying golem selects heirs" | "User exports NeuroStore and selectively imports into new agent" |
| "Death-approach dreams / terminal dreams" | "Idle-triggered / scheduled dream consolidation" |
| "Lineage (golem families across generations)" | "Provenance (Engram lineage DAG across time)" |
| "Mortality anxiety in Daimon" | "Task performance + gate outcomes + prediction accuracy inputs" |
| "Terminal requiem / death-mapped sonification" | "Behavioral-state-mapped sonification (Engaged/Struggling/Coasting/Exploring/Focused/Resting)" |
| "Golem-X" (e.g., golem runtime, golem inference, golem tools, golem safety, golem eval) | Check if domain-specific (keep as chain domain plugin) or general (promote to core framework: roko-runtime, CascadeRouter, domain plugin tools, roko-core capabilities, roko-gate) |
| "Mori + Golem as separate systems" | "Roko Framework with coding and chain domain plugins" |
| "Signal" (architecture noun) | "Engram" — but keep `Signal` as the Rust type name where code currently uses it |

---

## Files and directories

```
tmp/prd-migration/
├── README.md               ← this file: naming, reframe rules, principles
├── SOURCE-INDEX.md         ← full source file listing per target doc
├── CHECKLIST.md            ← 22 target docs with human-readable prompts
├── run-migration.sh        ← the OVERNIGHT RUNNER script ★
├── lib/
│   ├── common.sh           ← shared utilities (logging, paths, topic list)
│   ├── spawn.sh            ← spawns Claude Opus agents
│   └── verify.sh           ← per-topic quality checks
├── context-pack/           ← 7-file context bundle injected into every agent run
│   ├── README.md
│   ├── 00-ALWAYS-READ-FIRST.md
│   ├── 01-naming-map.md
│   ├── 02-reframe-rules.md
│   ├── 03-concepts-lifecycle.md
│   ├── 04-writing-rules.md
│   ├── 05-source-files.md
│   └── 06-output-structure.md
├── prompts/                ← 22 per-topic agent prompts ★
│   ├── README.md
│   ├── 00-architecture.prompt.md
│   ├── 01-orchestration.prompt.md
│   ├── ... (22 total)
│   └── 21-references.prompt.md
├── verify/
│   └── verify-topic.sh     ← standalone per-topic verification wrapper
└── logs/                   ← created at runtime: logs/<run-id>/<topic>.log + .result
```

## Running the migration overnight

The migration is fully automated via `run-migration.sh`. It spawns parallel Claude Opus
agents, each with a fresh context, generates all 22 topic folders under
`/Users/will/dev/nunchi/roko/roko/docs/`, and verifies each topic after generation.

### Prerequisites

- Claude Code CLI installed: `npm install -g @anthropic-ai/claude-code`
- `claude` must be in PATH. Check with `which claude`.
- Access to the Roko repo, refactoring-prd, and bardo-backup directories (the script
  auto-allows these via `--add-dir`).

### Before running overnight (recommended sanity checks)

1. **Preflight + dry-run** (no cost — just validates environment):
   ```bash
   ./run-migration.sh --dry-run
   ```
   Confirms the claude CLI, CLAUDECODE handling, directories, context-pack, prompts, key sources, and required binaries. Shows the exact command shape that would run for each topic.

2. **Verify function self-test** (no cost — tests verify_topic with mock input):
   ```bash
   ./verify/test-verify.sh
   ```
   Creates mock good + bad topic output and confirms the verification function catches failures and passes good output.

3. **Live spawn integration test** (~$0.50 — actually invokes Claude Opus via the runner's spawn function):
   ```bash
   ./verify/test-spawn-integration.sh
   ```
   Runs a tiny smoke-test prompt through `spawn_topic()`, verifies the agent reads 4 real source files (context pack + refactoring-prd + active code), writes an output file, and that the output contains verbatim content from each source. Proves end-to-end that context injection works before you commit to a full overnight run. The expected result: 13/13 checks pass in ~60 seconds.

4. **Single topic smoke test** (~$3-10 — spawns one real topic):
   ```bash
   ./run-migration.sh --only 00-architecture
   ```
   Inspect the output in `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/` and the log in `logs/run-*/00-architecture.log`. If quality is acceptable, proceed to the full overnight run.

### Quick start

```bash
cd /Users/will/dev/nunchi/roko/roko/tmp/prd-migration

# List topics and their completion state
./run-migration.sh --list

# Dry run + preflight (no cost — catches environment issues early)
./run-migration.sh --dry-run

# Run a single topic to test
./run-migration.sh --only 00-architecture

# Full overnight run with 3 parallel agents (default)
./run-migration.sh

# Full run with 5 parallel agents (more expensive, faster)
./run-migration.sh --parallel 5

# Re-run only failed topics (resume)
./run-migration.sh --only "04,08,17"

# Force re-run even if output exists
./run-migration.sh --force --only 00-architecture

# Verify existing output without re-running agents
./run-migration.sh --verify-only
```

### Nested-session gotcha (CRITICAL)

Claude Code refuses to spawn a nested session by default. If you launch the runner from
inside a Claude Code interactive session, the spawn will fail with:

> Error: Claude Code cannot be launched inside another Claude Code session.

**The runner handles this automatically** by prepending `env -u CLAUDECODE -u CLAUDE_CODE_ENTRYPOINT`
to every `claude` invocation (see `lib/spawn.sh`). The preflight check warns if
`CLAUDECODE` is set in your environment. This was discovered and fixed during end-to-end
testing. You do not need to do anything special — the runner does it for you.

### What happens during a run

1. **Bootstrap**: Checks that the `claude` CLI is available, verifies `SOURCE-INDEX.md`
   and `README.md` exist, creates the log directory.
2. **Topic selection**: Defaults to all 22 topics. `--only` accepts a comma-separated
   list (numeric prefixes also work, e.g., `--only "00,04,08"`).
3. **Parallel spawn**: Up to `--parallel N` agents run concurrently. Each agent:
   - Is a fresh `claude --print --model claude-opus-4-6` invocation with
     `--permission-mode bypassPermissions` and `--add-dir` for the three root directories.
   - Reads its prompt file from stdin.
   - Follows the prompt: reads the context-pack, reads refactoring-prd, reads the
     SOURCE-INDEX entry, reads legacy sources, reads implementation plans, reads active
     code, then writes 10-25 sub-docs + `INDEX.md` to its topic folder.
   - Runs under a configurable timeout (default 45 minutes per topic).
4. **Verification**: After each agent finishes, `verify_topic()` runs:
   - Checks `INDEX.md` exists and is ≥50 lines.
   - Checks at least 5 sub-docs exist (topics typically have 10-18).
   - Checks each sub-doc is ≥200 lines.
   - Checks total topic line count is ≥2500 lines.
   - Scans for forbidden terms (`Thanatopsis`, `Necrocracy`, `clade → fleet`, `GNOS token`,
     `terminal requiem`, `Thriving → Terminal`, etc.).
   - Scans for required terms (Roko, Engram, Synapse).
   - Counts citation-like patterns (warn if below floor).
5. **Summary**: Per-topic pass/warn/fail/skipped/dry counts. Logs point to
   `logs/<run-id>/<topic>.log` for each agent's full transcript.
6. **Master index**: After all agents finish, `generate_master_index()` writes
   `/Users/will/dev/nunchi/roko/roko/docs/INDEX.md` linking every topic folder.

### Tuning

Environment variables (set before calling the script):

| Variable | Default | Purpose |
|---|---|---|
| `ROKO_MIGRATION_MODEL` | `claude-opus-4-6` | Model ID |
| `ROKO_MIGRATION_TIMEOUT` | `2700` (45 min) | Per-topic timeout in seconds |
| `ROKO_MIGRATION_BUDGET_USD` | `15` | Per-topic dollar cap passed as `--max-budget-usd` |
| `ROKO_MIGRATION_PARALLEL` | `3` | Default parallelism |
| `ROKO_MIGRATION_CLAUDE_FLAGS` | `""` | Extra flags to pass to `claude` |
| `MIN_INDEX_LINES` | `50` | INDEX.md minimum lines |
| `MIN_SUBDOC_LINES` | `200` | Per-sub-doc minimum lines |
| `MIN_SUBDOCS` | `5` | Minimum sub-docs per topic (excluding INDEX) |
| `MIN_TOPIC_TOTAL_LINES` | `2500` | Total line count per topic |

### Cost estimate

Each topic spawns one Claude Opus agent that reads ~50-200KB of source material and
writes ~30-150KB of output. Per-topic token usage is very rough estimate 200-500K tokens
(~$1-5 per topic at Opus pricing). **22 topics × ~$3 average ≈ $60-100 per full run.**
Budget accordingly. Use `--parallel 5` if you want faster completion.

### Resumability

The runner is resumable. If a topic's `INDEX.md` exists and is non-empty, it's skipped by
default (unless you pass `--force`). If an agent fails or times out, you can re-run
just that topic:

```bash
./run-migration.sh --only 08-chain --force
```

### Troubleshooting

- **Agent hangs** — adjust `ROKO_MIGRATION_TIMEOUT` upward or check the log file for the
  last activity. Chain topic (08) is the largest and may need more time.
- **Forbidden terms found** — the agent used a term from the rename/reframe list.
  Re-read the prompt's CRITICAL REMINDERS section. Edit the prompt to emphasize the
  specific rule, then re-run with `--force`.
- **Too few sub-docs** — the agent summarized. Edit the prompt's sub-doc table to list
  explicit filenames, then re-run with `--force`.
- **Missing citations** — the agent dropped them. Edit the prompt to enumerate specific
  citations that must appear, then re-run.

## Target output directory

`/Users/will/dev/nunchi/roko/roko/docs/` (does not exist yet — the runner creates it).

Structure:
```
docs/
├── INDEX.md                      ← master index (auto-generated)
├── 00-architecture/
│   ├── INDEX.md
│   ├── 00-vision-and-thesis.md
│   ├── 01-naming-and-glossary.md
│   └── ... (17 sub-docs)
├── 01-orchestration/
│   └── ... (14 sub-docs)
├── ... (22 topic folders)
└── 21-references/
    └── ... (24 sub-docs, one per citation domain)
```

Each topic folder contains 10-25 focused sub-docs + one `INDEX.md`. Total output size is
expected to be ~200+ markdown files totaling 100,000+ lines across all topics.

Note: `refactoring-prd/MIGRATION-CHECKLIST.md` targets `bardo-backup/prd-updated/` with a
slightly different 24-doc topology. This tmp/prd-migration set uses the 22-doc topology
(00–20 + references) and writes to `roko/docs/`. The two are complementary — use whichever
output path fits your workflow.
