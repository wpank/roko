# Prompt: 17-lifecycle

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/17-lifecycle/`. **This topic REPLACES the entire legacy "mortality" system.** Covers agent creation and provisioning, configuration and operator model, funding, knowledge transfer via backup/restore (replaces "succession"), agent deletion (user-initiated only), knowledge staleness via Ebbinghaus (applies to knowledge freshness, NOT agent lifespan), knowledge demurrage, replication.

**CRITICAL**: NO death. No mortality. No Thanatopsis. No stochastic death. No necrocracy. Keep ALL mortality research citations (Ray, Lenski, Ebbinghaus, Hayflick) — they ground knowledge transfer and decay, not agent lifespan.

## Step 1 — Context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order. **Pay special attention to `03-concepts-lifecycle.md`'s REMOVED section.**

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/04-knowledge-and-mesh.md` §5 Knowledge Backup & Restore (4-step BACKUP → DELETE → CREATE → RESTORE)
2. `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md` — **ALL incompatibility sections**
3. `/Users/will/dev/nunchi/roko/refactoring-prd/07-implementation-priorities.md` §Dropped Items (full removal list)
4. `/Users/will/dev/nunchi/roko/refactoring-prd/03-cognitive-subsystems.md` §1 Tier Progression (Ebbinghaus × tier — memory management, not mortality)
5. `/Users/will/dev/nunchi/roko/refactoring-prd/01-synapse-architecture.md` §Decay enum (Ebbinghaus variant — memory management)

## Step 3 — SOURCE-INDEX entry `## 17-lifecycle.md`

Legacy PRD files to EXTRACT content from (reframe mortality → lifecycle):
- `bardo-backup/prd/01-golem/06-creation.md`
- `bardo-backup/prd/01-golem/07-provisioning.md`
- `bardo-backup/prd/01-golem/08-funding.md`
- `bardo-backup/prd/01-golem/09-inheritance.md`
- `bardo-backup/prd/01-golem/10-replication.md`
- `bardo-backup/prd/01-golem/11-lifecycle.md`
- `bardo-backup/prd/01-golem/12-teardown.md`
- `bardo-backup/prd/01-golem/19-config-and-operator-model.md`
- `bardo-backup/prd/02-mortality/02-epistemic-decay.md` — knowledge staleness concept is still relevant
- `bardo-backup/prd/02-mortality/05-knowledge-demurrage.md` — knowledge decay over time
- `bardo-backup/prd/02-mortality/07-succession.md` — knowledge transfer mechanism (reframe as backup/restore)
- `bardo-backup/prd/02-mortality/14-research-foundations.md` — **KEEP ALL 130+ citations**
- `bardo-backup/prd/02-mortality/15-references.md` — **KEEP ALL 162 citations**
- `bardo-backup/prd/04-memory/03-mortal-memory.md` — reframe as lifecycle memory
- `bardo-backup/prd/11-compute/00-overview.md`, `01-architecture.md`, `02-provisioning.md`

**SKIP ENTIRELY** (do NOT incorporate content):
- `bardo-backup/prd/02-mortality/00-thesis.md`
- `bardo-backup/prd/02-mortality/01-architecture.md`
- `bardo-backup/prd/02-mortality/03-stochastic-mortality.md`
- `bardo-backup/prd/02-mortality/06-thanatopsis.md`
- `bardo-backup/prd/02-mortality/08-mortality-affect.md` (extract somatic marker citations only)
- `bardo-backup/prd/02-mortality/09-fractal-mortality.md`
- `bardo-backup/prd/02-mortality/11-immortal-control.md`
- `bardo-backup/prd/02-mortality/16-necrocracy.md`
- `bardo-backup/prd/02-mortality/18-antifragile-mortality.md`
- `bardo-backup/prd/01-golem/04-mortality.md`
- `bardo-backup/prd/01-golem/05-death.md`

## Step 4 — active code

- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/init.rs` (if exists) or equivalent
- Read the `roko neuro backup` and `roko neuro restore` command handlers

## Step 5 — Output and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/17-lifecycle
```

Write **13 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-vision-and-mortality-replaced.md` | Lifecycle replaces mortality entirely. **Agents don't die naturally.** Users create agents. Users delete agents. Between those two events, agents persist. Explicit reframe explanation. Why the old mortality thesis was wrong (agents aren't biological, death clocks add complexity without benefit). |
| 01 | `01-agent-creation.md` | Agent creation. `roko init` workflow. Template selection. Domain plugin registration. Defaults. |
| 02 | `02-provisioning.md` | Compute provisioning (from 11-compute/01-architecture.md). Resource allocation. Environment setup. |
| 03 | `03-configuration-and-operator-model.md` | Operator model from 01-golem/19. Who controls the agent. How configuration flows. Override layers (CLI flags → env vars → roko.toml → defaults). |
| 04 | `04-funding-and-budgets.md` | Budget allocation. Cost tracking (cross-reference 05-learning.md §08). Per-task budget limits. Multi-level guardrails. **NOTE**: reframed from "economic mortality" — same math, different narrative. Budget exhaustion is a resource constraint, not a death trigger. |
| 05 | `05-knowledge-backup-export.md` | `roko neuro backup <agent>` — export NeuroStore. Format: JSONL + HDC vectors + tier metadata + provenance. What's included. What's excluded (Daimon state, private prompts). |
| 06 | `06-agent-deletion.md` | `roko delete <agent>` — user-initiated deletion. Clean shutdown. Resource cleanup. Backup prompt before deletion. Storage removed. |
| 07 | `07-new-agent-creation.md` | `roko init` after deletion creates a fresh agent with fresh NeuroStore. No automatic knowledge transfer. |
| 08 | `08-selective-restore.md` | `roko neuro restore <file>` — selective import from a backup. User picks which entries to restore. Entries start at Transient tier — must re-prove themselves. Provenance tracks origin ("restored from agent X on date Y"). |
| 09 | `09-knowledge-transfer-via-mesh.md` | Live agent-to-agent knowledge transfer via Collective/Mesh (cross-reference 13-coordination.md). Not inheritance, not succession — ongoing collective learning via shared Substrate. |
| 10 | `10-ebbinghaus-for-knowledge-not-agents.md` | Ebbinghaus 1885 forgetting curve. Applied to **knowledge freshness**, NOT agent lifespan. Successful use increases knowledge strength → decay slows. Failed use decreases → decay accelerates. Tier progression emerges naturally. Cross-reference 06-neuro.md §07. |
| 11 | `11-knowledge-demurrage.md` | Token-level analog of knowledge decay. KORAI demurrage (1% annual) mirrors Engram half-life. Cross-reference 14-identity-economy.md §10 for KORAI tokenomics details. |
| 12 | `12-academic-foundations.md` | **Keep ALL 130+ mortality research citations.** They're still relevant — they ground knowledge transfer and decay mechanisms, not agent biological lifespan. Ray 1991 Tierra (evolutionary CS — reframed). Lenski Long-Term Evolution Experiment. Ebbinghaus 1885 (forgetting curve). Hayflick 1961 (cellular senescence — only as historical reference). Tom Ray on evolutionary systems. All references. Group by topic. |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per context-pack rules. ≥200 lines per sub-doc, ≥3000 total. **Preserve ALL 130+ citations from 02-mortality/14-research-foundations.md and 02-mortality/15-references.md.** Reframe each in the commentary to be about knowledge lifecycle, not agent mortality.

Cross-reference 06-neuro (backup/restore), 13-coordination (mesh sharing), 14-identity-economy (knowledge demurrage on KORAI).

## CRITICAL REMINDERS

- **NO DEATH. NO MORTALITY. NO THANATOPSIS. NO NECROCRACY.** Agents are created and deleted by users only.
- Keep ALL mortality research citations. They ground knowledge lifecycle + decay, not agent lifespan.
- SKIP the 11 SKIP files listed above. Extract only citations from 08-mortality-affect.md.
- Apply naming map: mortality → lifecycle; succession → backup/restore; golem → agent; clade → collective/mesh.
- Ebbinghaus decay applies to **knowledge freshness**, NOT agent lifespan.
- Use Write tool. Don't ask questions.
