# Reframe Rules — How to Translate Legacy Concepts

> This file explains how to handle legacy content that does not fit the new Roko architecture.
> Some concepts are renamed, some are conceptually reframed, some are flagged as incompatible
> and must be rewritten, and some are removed entirely.
>
> This is a condensed version of `refactoring-prd/08-translation-guide.md`. When in doubt,
> refer to that document for the canonical translation rules.

## Rule categories

1. **Direct rename** — mechanical search and replace (see `01-naming-map.md`).
2. **Conceptual reframe** — keep the research, citations, and mechanism; change the framing.
3. **Incompatible** — reject the legacy framing entirely, rewrite from first principles.
4. **Remove entirely** — delete the content, do not carry forward.

## Reframe rules in detail

### Mortality → Resource Management & Cognitive Pressure

| Old concept | New concept | What to keep | What to change |
|---|---|---|---|
| Three death clocks (economic, epistemic, stochastic) | Budget limits + confidence tracking + scheduling pressure | The math and mechanisms. Economic death = budget exhaustion. Epistemic death = nothing new to try. | Remove "death" language. Frame as "budget exhaustion," "knowledge plateau," "scheduling timeout." Remove stochastic death entirely. |
| Vitality gauge (Thriving → Terminal phases) | Daimon PAD vector as cognitive state display | The visualization concept. | Map to PAD states instead. No Terminal destination. States are cyclical. |
| Behavioral phases (Thriving → Stable → Conservation → Declining → Terminal) | Behavioral states (Engaged → Focused → Exploring → Struggling → Resting ↔ Coasting) | The state machine concept. Phase transitions based on metrics. | Remove Terminal state. No death destination. States are cyclical. |
| Mortality-driven urgency | Deadline-driven / budget-driven urgency | The urgency mechanism — agents with limited resources must prioritize. | Source of urgency is external (deadlines, budgets), not intrinsic (finite lifespan). |
| Antifragility (strengthened by near-death) | Antifragility (strengthened by failures/challenges) | The concept that stress improves the system. | Stress comes from task difficulty, not approaching death. |

**Translation rule**: Every time a legacy source says "because the golem is dying,"
replace with "because the agent's budget/confidence/time is constrained." The mechanism
is identical — resource scarcity drives prioritization. The narrative changes from
existential to practical.

### Succession → Knowledge Backup/Restore + Mesh

| Old concept | New concept |
|---|---|
| Dying golem transfers knowledge to offspring | User exports NeuroStore, creates new agent, imports selected pieces |
| Generational learning (parent → child) | Collective learning (agent → mesh → agent) + selective restore |
| Knowledge inheritance as biological metaphor | Knowledge as portable, user-controlled data |
| Lineage (golem families across generations) | Provenance (Engram lineage DAG across time) |
| Thanatopsis phase (dying → knowledge selection) | User backup command (`roko neuro backup`) |

**Translation rule**: Replace biological inheritance language with data management
language. The knowledge types (Insight, Heuristic, Warning, etc.) and their structures
are identical. Only the transfer mechanism changes.

### Styx Relay → Agent Mesh

| Old Styx feature | New Mesh equivalent |
|---|---|
| Clade synchronization | Collective mesh sync via WebSocket/Iroh |
| Lethe knowledge exchange | P2P Engram sharing via Agent Mesh |
| Styx WebSocket server | Agent mesh coordinator (in `roko-serve`) |
| Clade membership | ERC-8004 registry + permissioned subnets |

### Golem-Specific → Domain-Agnostic + Domain Plugin

| Old (Golem-specific) | New (Generalized) |
|---|---|
| "Golem heartbeat" | "Agent cognitive loop" (universal, parameterized by domain) |
| "Golem runtime" | "Agent runtime" (`roko-runtime`) |
| "Golem tools" (423+ DeFi tools) | Domain plugin tools (chain tools are one set among many) |
| "Golem inference" | "Model routing" (`roko-agent` CascadeRouter) |
| "Golem safety" | "Safety capabilities" (`roko-core` + `roko-agent/safety`, domain-agnostic) |
| "Golem eval" | "Gate pipeline" (`roko-gate`, domain-agnostic) |

**Translation rule**: Any "golem-X" concept → check if it's domain-specific (keep as chain
domain plugin) or general (promote to core framework).

## Flagged incompatibilities

These legacy concepts **do not fit** the new framework and require significant rewrite.

### INCOMPATIBLE: Death Phases as UX

**Old**: Vitality gauge showed Thriving → Stable → Conservation → Declining → Terminal
with distinct UI treatments per phase. Terminal phase had "requiem" animation and
degraded ambient music.

**New**: No terminal state. The Spectre creature reflects Daimon PAD state, which is
cyclical (Engaged ↔ Struggling ↔ Coasting ↔ Exploring ↔ Focused ↔ Resting). Spectre
never "dies" — it adapts. ROSEDUST design language still applies but maps to cognitive
states, not mortality phases.

**Action**: Rewrite all UX specs to use behavioral states instead of mortality phases.
Keep ROSEDUST. Keep Spectre. **Remove any "death animation" or "terminal requiem" content entirely.**

### INCOMPATIBLE: Mortality Clock Math for Cognitive Pressure

**Old**: Three clocks with formulas: `economic_clock = f(balance, burn_rate)`,
`epistemic_clock = f(knowledge_freshness, learning_rate)`,
`stochastic_clock = random(weibull)`.

**New**: Budget limits and confidence tracking serve the same purpose without the death
metaphor. Economic pressure = budget remaining. Epistemic pressure = prediction accuracy
declining. **No stochastic death.**

**Action**: Keep the economic and epistemic formulas but reframe as resource constraint
functions. **Remove the stochastic (random death) clock entirely.** The mechanisms are
useful; the mortality framing is not.

### INCOMPATIBLE: Succession / Generational Knowledge Transfer

**Old**: When a golem dies, it runs a "thanatopsis" phase where it selects knowledge to
pass to a new golem. Knowledge transfer is automatic and golem-directed.

**New**: Knowledge transfer is user-directed. Users export, select, and import. Agents
don't die and don't choose successors.

**Action**: Remove thanatopsis, succession, and generational transfer. Replace with
backup/restore UX and mesh-based knowledge sharing. The knowledge types and structures
are identical — only the transfer trigger changes.

### INCOMPATIBLE: "Dream as Approaching Death" Framing

**Old**: Dreams were triggered by proximity to death. The closer to death, the more
intense the dream consolidation. "Terminal dreams" were the final consolidation before
death.

**New**: Dreams are triggered by idle time (Delta frequency) or scheduled (nightly
consolidation). Intensity is based on volume of unprocessed episodes, not proximity to
death.

**Action**: Keep all dream mechanics (replay, consolidate, prune, synthesize, validate).
Remove death-proximity triggers. Replace with idle-time and schedule-based triggers.

### INCOMPATIBLE: Emotion Mapped to Mortality

**Old**: Daimon PAD vector had mortality-specific mappings. Fear increased as death
approached. Joy was highest in "Thriving" phase. Mortality directly modulated emotional
state.

**New**: Daimon PAD vector tracks cognitive performance. Pleasure = task success. Arousal
= urgency/load. Dominance = confidence. **No mortality input.**

**Action**: Keep PAD vector math, Plutchik classification, somatic markers (Damasio).
Remove mortality as an input signal. Add task performance, gate results, and prediction
accuracy as input signals instead.

### NEEDS REDESIGN: Sonification

**Old**: Eight musical presets mapped to mortality phases. Eno mandate ("simultaneously
ignorable and interesting"). Five musical layers. Terminal requiem with deterministic fade.

**New**: Sonification is kept but reframed. Maps to Daimon behavioral states, NOT
mortality phases. **No terminal requiem.** The Eno mandate and musical layer architecture
are still valid.

**Action**: Redesign the preset catalog to map to behavioral states (Engaged, Struggling,
Coasting, Exploring, Focused, Resting) instead of mortality phases (Thriving → Terminal).
Keep all the music theory, emotion-scale mappings, and architectural decisions. The
musical language guide is mostly still valid — just change the phase names.

## Keep entirely (direct translation)

These sections of the old PRDs translate directly with only naming changes:

- **All academic citations** — every paper, every reference, every arXiv ID, every year, every author name. NO exceptions.
- **Research foundations** — Hypnagogia neuroscience (Lacaux 2021, Dormio, CLS theory), HDC math (Kanerva, Plate, Frady, Kleyko), Stigmergy (Grassé, Parunak, Dorigo), Knowledge layer (6 knowledge types, context assembly), Tokenomics (rename GNOS→KORAI/DAEJI otherwise keep), Chain architecture, Predictive foraging, Exponential flywheels, mirage-rs, Config reference (rename golem.toml→roko.toml).

## Citation preservation rules (CRITICAL)

1. **Keep ALL academic citations.** Every paper referenced in the old PRDs is still relevant. Preserve author names, years, journals, arXiv IDs, DOIs.
2. **Update surrounding context** to use Synapse Architecture terminology.
3. **Do NOT remove research context** — it's the intellectual foundation of Roko.
4. **Add new citations** from refactoring-prd research (collective intelligence, active inference, stigmergy in digital systems, provenance, ERC-8004, C2PA, Meta-Harness, FrugalGPT, ACE, CSO, ACON, DSPy, Reflexion, ExpeL, Voyager, EvoSkills, ADAS, Mattar-Daw, Boden, Pearl SCM, Walker & van der Helm, Lacaux, Dormio, Derrida, Damasio, Mehrabian, Scherer, Plutchik, Bower, Friston, Conant-Ashby, Beer VSM, Woolley, Ousterhout, etc.).
5. **Organize by topic** in the new docs rather than by the old document structure.

## Format for citations

Use inline citations in prose:

- `Lee et al. 2026 (arXiv:2603.28052)` — with arXiv ID when available
- `(Woolley et al., Science 330(6004), 2010)` — with journal and issue
- `Kanerva 2009, Cognitive Computation 1(2)` — with venue
- `[Grassé 1959]` — older papers that may not have arXiv/DOI

Every citation must have at minimum: author(s), year. Include more when available.

## What to do when you encounter something ambiguous

If a legacy source has content that could reasonably be kept, reframed, or removed:

1. **Default to keeping** — preserve the intellectual content.
2. **Reframe the motivation** — change "to delay death" to "to conserve budget", etc.
3. **Flag uncertainty** — add a footnote like "Note: the original source framed this as X; we have reframed as Y per the new architecture (see `refactoring-prd/08-translation-guide.md`)".
4. **Never silently drop content** — if you remove something, note it was removed.
