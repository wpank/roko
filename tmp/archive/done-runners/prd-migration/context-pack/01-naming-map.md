# Naming Map — Authoritative Old → New

> This is the canonical naming map for the Roko PRD migration. It supersedes any conflicting
> information in legacy sources. Apply these renames in every target doc without exception.
> If a legacy source uses an old name, rename it in your output.

## Project / framework names

| Old | New | Notes |
|---|---|---|
| Bardo | **Roko** | Overall framework/project. "Bardo" is the old umbrella name. |
| Mori | **Roko Orchestrator** | Build/coding orchestration. Often just "orchestrator". |
| Golem / Golems | **Agent / Agents** | The autonomous entity. Do NOT map to "roko" — the framework is roko, individual entities are agents. |
| Grimoire | **Neuro** / `roko-neuro` / `NeuroStore` | Knowledge subsystem. |
| Styx | **Agent Mesh** / **Mesh** | P2P relay + permissioned subnets. |
| GNOS (token) | **KORAI** (mainnet token) / **DAEJI** (testnet token) | Token names — applied wherever a token is discussed. |
| (new) Korai | **Korai** | Dedicated EVM chain for agent coordination. Mainnet. Replaces what was "Styx chain" conceptually. |
| (new) Daeji | **Daeji** | Testnet for Korai. |
| Clade | **Collective** / **Mesh** | Groups of cooperating agents. **DO NOT USE "fleet"** — this is a correction from an earlier incorrect rename. |

## Configuration file

| Old | New |
|---|---|
| `golem.toml` | `roko.toml` |

## Crate names

| Old | New |
|---|---|
| All `golem-*` crate names | `roko-*` |
| All `bardo-*` crate names | `roko-*` |
| `bardo-primitives` | `roko-primitives` |
| `bardo-runtime` | `roko-runtime` |
| `mori-index` | `roko-index` |
| `mori-context` | Distributed: context features in `roko-compose` + code intelligence in `roko-index` |
| `mori-mcp` | `roko-mcp-{stdio,github,slack,scripts}` |
| `bardo-terminal` | `roko-cli` (terminal UI is in `roko-cli`, with a separate TUI scaffold) |
| `roko-golem` | **DISSOLVED** — see Crate Dissolution below |

## Crate dissolution: `roko-golem`

The `roko-golem` crate is being removed. Its subsystems are redistributed:

| Subsystem | Current location | New location | Notes |
|---|---|---|---|
| Daimon (972 lines, fully implemented) | `roko-golem/daimon.rs` | `roko-daimon` | Move full implementation, delete scaffold duplicate in roko-daimon if present |
| Dreams (43 lines, placeholder) | `roko-golem/dreams.rs` | `roko-dreams` | Delete placeholder after roko-dreams is expanded |
| Grimoire (44 lines, placeholder) | `roko-golem/grimoire.rs` | `roko-neuro` | Delete placeholder; roko-neuro is the replacement |
| Chain Witness (43 lines, placeholder) | `roko-golem/chain_witness.rs` | `roko-chain` as `chain_witness` module | Move |
| Mortality (44 lines, placeholder) | `roko-golem/mortality.rs` | **DELETE ENTIRELY** | No mortality in the new architecture |
| Hypnagogia (42 lines, placeholder) | `roko-golem/hypnagogia.rs` | `roko-dreams` as `hypnagogia` module | Move |
| `ScaffoldEngine` trait | `roko-golem/lib.rs` | **DELETE** | Each subsystem defines its own trait — no umbrella needed |
| `GolemScaffold` aggregator | `roko-golem/lib.rs` | **DELETE** | Composition at application layer via config |

After dissolution: `roko-golem` is removed from workspace members in `Cargo.toml`.

**Composability principle**: Any subsystem can pipe to any other. Daimon emits Engrams →
Neuro stores them. Dreams reads from Neuro → produces new Engrams. Chain posts Engrams
on-chain. Everything flows through the 6 Synapse traits. No umbrella crate needed.

## Core types

| Old | New | Notes |
|---|---|---|
| `Signal` (as architecture noun) | `Engram` | The canonical architectural term. Use in all new writing. |
| `Signal` (as existing Rust type name) | `Signal` (for now) | The Rust type is still named `Signal` in the current codebase. Rename to `Engram` is Tier 0D in the implementation plan. In PRD docs, use "Engram" but note the current code type name. |
| `SignalBuilder` | `EngramBuilder` | |
| `signal.rs` | `engram.rs` | |
| "1 noun, 6 verbs" | **Synapse Architecture** | Architecture branding. |

## Interfaces

| Old | New |
|---|---|
| Bardo Sanctum | **Roko Portal** (web dashboard) |
| bardo-terminal | **Roko TUI** (terminal dashboard) |
| Mori TUI | **Roko TUI** |

## Checklists and references

| Old | New |
|---|---|
| "Mori parity" / "Mori parity checklist" | "Roko parity" |
| Mori tests | Roko tests |

## Tokens — VERY IMPORTANT

| Old | New | Chain | Notes |
|---|---|---|---|
| GNOS | **KORAI** | Korai (mainnet) | 1% annual demurrage |
| (new) | **DAEJI** | Daeji (testnet) | Testnet equivalent |

When a legacy document mentions "GNOS token", rename to "KORAI token" (or "DAEJI token" if
explicitly about testnet). When it mentions generic "golem chain", rename to "Korai chain".

## Subsystems — kept names

These names are kept unchanged in the new architecture (no rename needed):

- Mirage / mirage-rs (in-process EVM simulator)
- Heartbeat (cognitive loop)
- CoALA (cognitive architecture framework)
- Pheromone system
- Sleepwalker (reduced-capability sleep mode)
- Oneirography / Hypnagogia
- ALMA (three-layer affect model)
- Somatic markers (Damasio)
- Library of Babel (cross-collective knowledge)
- Bazaar (commerce primitives)
- MPP (Machine Payment Protocol)
- Testament (repurposed: knowledge transfer between agents, not death inheritance)
- Portal (interface concept)
- Creature (visual concept — now called "Spectre" specifically)
- Spectre (new name for the creature visualization system)
- ROSEDUST (design language)

## New names introduced (not in legacy)

These terms are new and only appear in refactoring-prd:

- **Engram** — the core data type (replaces Signal as noun)
- **Synapse Architecture** — the 6-trait composition (replaces "1 noun, 6 verbs")
- **Five Layers** — Runtime/Framework/Scaffold/Harness/Orchestration
- **Cognitive Cross-Cuts** — Neuro/Daimon/Dreams/etc.
- **C-Factor** — collective intelligence metric (Woolley et al. 2010)
- **C-Score** — composite optimization metric
- **Spectre** — procedurally generated creature per agent
- **ROSEDUST** — dark-only design system (rose on void-black)
- **Three Cognitive Speeds** — Gamma/Theta/Delta
- **16 T0 Probes** — zero-LLM probes
- **VCG Attention Auction** — Vickrey-Clarke-Groves for context allocation
- **Somatic Landscape** — k-d tree over 8D strategy space
- **Hypnagogia Engine** — Thalamic Gate + Executive Loosener + Dali Interrupt + Homuncular Observer
- **Cognitive Kernel Primitives** — namespaces, signals, scheduling, syscalls
- **Korai Passport** — ERC-721 soulbound agent identity
- **Spore / Sparrow** — job market protocols
- **ISFR** — Intersubjective Fact Registry
- **Valhalla** — privacy layer (TEE, PSI, ZK proofs)

## How to apply the rename

1. When quoting a legacy source verbatim, keep the old name in the quote but add a
   parenthetical or footnote: "(formerly Grimoire, now Neuro)".
2. When paraphrasing or summarizing, use the new name directly.
3. Code samples: use new crate names (`roko-primitives`, `roko-runtime`, etc.) even if the
   current Rust code still uses the old names. Note the current name in a comment or
   side note.
4. Struct/type names in Rust code: `Engram` is the target, but the current code uses
   `Signal`. When writing prose, say "Engram". When writing a Rust code snippet, use
   `Signal` and add a comment like `// will be renamed to Engram in Tier 0D`.
5. File and path references: update all paths from `bardo-*` to `roko-*`, `mori-*` to
   `roko-*`.
6. Never say "Golem SDK" — say "Agent SDK" or "Roko SDK".
7. Never say "Mori + Golem" — say "Roko framework with coding and chain domain plugins".
