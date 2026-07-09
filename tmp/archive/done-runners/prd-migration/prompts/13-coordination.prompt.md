# Prompt: 13-coordination

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/13-coordination/`. Covers stigmergy theory (Grassé, Parunak, Dorigo), digital pheromones (typed Engrams), PheromoneKind enum, PheromoneScope (Local/Mesh/Global), agent mesh sync (WS/Iroh/ERC-8004), morphogenetic specialization, exponential flywheels (Reed's Law), **generalized stigmergy beyond blockchain**.

## Step 1 — Context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order.

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/04-knowledge-and-mesh.md` §3 Agent Mesh P2P, §4 Stigmergy Generalized (beyond termite metaphor)
2. `/Users/will/dev/nunchi/roko/refactoring-prd/02-five-layers.md` §Stigmergy (git as stigmergy, knowledge as stigmergy, pheromone types, cross-domain)
3. `/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md` §VI Network Flywheel, §XIII Cross-Domain Insight Resonance
4. `/Users/will/dev/nunchi/roko/refactoring-prd/05-agent-types.md` §2 Pheromones (coding PATTERN traces)

## Step 3 — SOURCE-INDEX entry `## 13-coordination.md`

Key legacy:
- `bardo-backup/prd/02-mortality/10-clade-ecology.md` (rename clade → collective/mesh)
- `bardo-backup/prd/02-mortality/10b-morphogenetic-specialization.md`
- `bardo-backup/prd/09-economy/04-coordination.md`
- `bardo-backup/prd/13-runtime/06-collective-intelligence.md`
- `bardo-backup/prd/20-styx/00-architecture.md`, `03-clade-sync.md` (rename), `04-marketplace.md`, `07-p2p-transport.md`, `08-transport-config.md`
- `bardo-backup/tmp/agent-chain/03-stigmergy.md` — full stigmergy spec
- `bardo-backup/tmp/agent-chain/proving-collective-intelligence.md`
- `bardo-backup/tmp/agent-chain/09-exponential-flywheels.md`
- `bardo-backup/tmp/agent-chain-new/02-coordination-theory.md`
- `bardo-backup/tmp/mori-refactor/18-agent-ecology.md`
- `bardo-backup/tmp/death/tools/c05-multi-agent.md` (extract mechanism, drop mortality framing)

## Step 4 — implementation-plans

- `12b-chain-layer.md` §B Gossip (4-tier, 8 topics, GossipSub v1.1), §N ISFR (collective price discovery)

## Step 5 — Output and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/13-coordination
```

Write **13 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-stigmergy-theory.md` | Grassé 1959 (Insectes Sociaux 6(1)) — the original termite stigmergy paper. Theraulaz 1999 (Artificial Life 5(2)). Dorigo 1997 Ant Colony Optimization (IEEE Trans. Evol. Comp. 1(1)). Indirect coordination through environmental modification. No direct messaging. |
| 01 | `01-stigmergy-beyond-termites.md` | Generalized to any domain. Coding (git repo commits). Blockchain (Korai knowledge entries). Research (shared Substrate insights). Operations (infrastructure state). Cross-domain (HDC vector space structural patterns). Full table. |
| 02 | `02-git-as-stigmergy.md` | Git repository as shared environment for coding agents. Each commit is a pheromone. Agents read workspace state, modify it, commit, leave traces. No coordination protocol needed. |
| 03 | `03-digital-pheromones.md` | Parunak et al. 2002 digital pheromones for multi-agent coordination. Pheromones as typed Engrams with specific decay profiles and response patterns. Full `Pheromone` struct (kind, intensity, decay_rate, source, scope). |
| 04 | `04-pheromone-kinds.md` | Universal: Threat (fast decay, hours), Opportunity (medium decay, days), Wisdom (slow decay, weeks). Domain-specific: Alpha (very fast, minutes — blockchain), Pattern (medium, code), Anomaly (medium), Consensus (slow — multi-agent agreement). User-extensible: Custom(String). |
| 05 | `05-pheromone-scope.md` | Local(SubstrateId) — this agent's store only. Mesh(CollectiveId) — within a permissioned subnet. Global — public Korai chain. How scope affects decay and propagation. |
| 06 | `06-agent-mesh-sync.md` | WebSocket (co-located, low latency) + Iroh (NAT-traversing P2P, encrypted) + ERC-8004 (service discovery via Agent Cards). Cross-reference 08-chain.md for chain-side details. |
| 07 | `07-morphogenetic-specialization.md` | Agents differentiate roles through pheromone gradients. Emergent specialization, not imposed. Integration with skill library. |
| 08 | `08-permissioned-subnets.md` | Company collectives with private knowledge meshes. Boston Dynamics example. Internal reputation separate from public. Opt-in publishing to Korai. Organizational "agent intranets." |
| 09 | `09-stigmergy-scaling.md` | O(1) per agent — agents read/write to shared state, not to each other. Adding agents doesn't increase coordination overhead. Self-organizing (useful knowledge rises, bad knowledge decays). Cross-domain (HDC structural analogy). Asynchronous (no clock sync). Fault-tolerant (individual failure doesn't break coordination). |
| 10 | `10-exponential-flywheel.md` | More agents → more knowledge posted → better collective knowledge → each agent performs better → more agents attracted → even more knowledge → superlinear scaling. Reed's Law (2^N for groups). Metcalfe's Law (N² for networks). O(N) individual contributions → O(N²) network value. |
| 11 | `11-collective-intelligence-metrics.md` | Connection to C-Factor (cross-reference 00-architecture.md). Turn-taking equality. Knowledge flow rate. Cross-domain transfer. Emergent coordination. Measuring collective intelligence. Woolley et al. 2010. |
| 12 | `12-current-status-and-gaps.md` | Pheromone types designed but not implemented (Tier 5E P2). Code uses basic Engrams with Decay::THREAT/OPPORTUNITY/WISDOM constants. Pheromone-specific routing and scope enforcement are target features. Agent Mesh not yet wired (Tier 5). |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per context-pack rules. ≥200 lines per sub-doc, ≥3000 total. Citations: Grassé 1959, Theraulaz 1999, Parunak 2002, Dorigo 1997, Reed's Law, Metcalfe's Law, Beer VSM, Woolley 2010, Heylighen (stigmergy theory), Holland 1992 (complex adaptive systems), Odling-Smee 2003 (niche construction).

Cross-reference 00-architecture, 04-knowledge-and-mesh, 08-chain, 14-identity-economy, 06-neuro.

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE.
- Generalize stigmergy **beyond blockchain**. Git, code patterns, research, infra, HDC space — not just pheromone chains.
- Rename: clade → collective/mesh; styx → Agent Mesh; golem → agent; bardo → roko.
- Use Write tool. Don't ask questions.
