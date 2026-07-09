# prompts/ — Per-topic agent briefings

> Each `.prompt.md` file in this directory is a complete, self-contained briefing for a
> fresh Claude Opus agent generating one topic of the Roko PRD documentation. The agent
> starts with zero prior context and uses the prompt to find its way through the context
> pack, the refactoring-prd spec, the legacy sources, and the implementation plans.

## Files

| Prompt | Topic | Output folder |
|---|---|---|
| `00-architecture.prompt.md` | Synapse Architecture foundation | `docs/00-architecture/` |
| `01-orchestration.prompt.md` | Plan DAG, parallel executor, stigmergy via git | `docs/01-orchestration/` |
| `02-agents.prompt.md` | Backends, MCP, tool loop, harness engineering | `docs/02-agents/` |
| `03-composition.prompt.md` | Context engineering, VCG auction, MVT foraging | `docs/03-composition/` |
| `04-verification.prompt.md` | Gates, ratcheting, EvoSkills, Forensic AI | `docs/04-verification/` |
| `05-learning.prompt.md` | Episodes, bandits, 8 feedback loops, 31.6× calibration | `docs/05-learning/` |
| `06-neuro.prompt.md` | Knowledge store (formerly Grimoire), 6 types, 4 tiers, HDC | `docs/06-neuro/` |
| `07-conductor.prompt.md` | Reactive intelligence, watchers, cognitive signals | `docs/07-conductor/` |
| `08-chain.prompt.md` | Korai chain, KORAI/DAEJI, mirage-rs, marketplace (**LARGEST**) | `docs/08-chain/` |
| `09-daimon.prompt.md` | PAD affect engine, somatic landscape, 6 behavioral states | `docs/09-daimon/` |
| `10-dreams.prompt.md` | 3-phase dream cycle, hypnagogia, Alpha Convergence | `docs/10-dreams/` |
| `11-safety.prompt.md` | Capabilities, audit, cognitive kernel primitives | `docs/11-safety/` |
| `12-interfaces.prompt.md` | CLI, TUI (ROSEDUST + Spectre), Web Portal | `docs/12-interfaces/` |
| `13-coordination.prompt.md` | Stigmergy, pheromones, mesh sync | `docs/13-coordination/` |
| `14-identity-economy.prompt.md` | ERC-8004, reputation, KORAI economics, x402 | `docs/14-identity-economy/` |
| `15-code-intelligence.prompt.md` | roko-index, symbol graphs, PageRank, HDC fingerprints | `docs/15-code-intelligence/` |
| `16-heartbeat.prompt.md` | CoALA pipeline, 3 cognitive speeds, active inference | `docs/16-heartbeat/` |
| `17-lifecycle.prompt.md` | Agent creation/deletion, backup/restore (**NO mortality**) | `docs/17-lifecycle/` |
| `18-tools.prompt.md` | Tool registry, MCP servers, 16 agent templates | `docs/18-tools/` |
| `19-deployment.prompt.md` | Native/WASM/Docker/daemon/edge/cloud | `docs/19-deployment/` |
| `20-technical-analysis.prompt.md` | Generalized oracles, coding TA equivalents | `docs/20-technical-analysis/` |
| `21-references.prompt.md` | Master citation index (24 domains) | `docs/21-references/` |

## Structure of a prompt file

Every prompt follows the same 9-step structure:

```
# Prompt: NN-topic

## Your mission
<what the topic covers and where the output goes>

## Step 1 — Read the context pack (MANDATORY, in order)
<7 files from context-pack/ to read first>

## Step 2 — Read canonical refactoring-prd sources
<specific refactoring-prd files relevant to this topic>

## Step 3 — Read SOURCE-INDEX entry
<pointer to the section in SOURCE-INDEX.md>

## Step 4 — Read legacy sources / implementation plans
<additional files specific to this topic>

## Step 5 — Read active code
<glob patterns for active Rust source files>

## Step 6 — Create output directory and plan sub-docs
<mkdir command + table listing every sub-doc with its content>

## Step 7 — Writing rules
<pointer to context-pack/04-writing-rules.md + topic-specific emphases>

## Step 8 — Write INDEX.md
<pointer to the INDEX.md schema>

## Step 9 — Self-check
<verification checklist — must pass before finishing>

## CRITICAL REMINDERS
<topic-specific gotchas and the universal rules>
```

## How prompts are used

Each prompt is run by `run-migration.sh` as a separate fresh Claude Opus invocation:

```bash
claude --print --model claude-opus-4-6 \
    --permission-mode bypassPermissions \
    --add-dir /Users/will/dev/nunchi/roko/roko \
    --add-dir /Users/will/dev/nunchi/roko/refactoring-prd \
    --add-dir /Users/will/dev/nunchi/roko/bardo-backup \
    < prompts/00-architecture.prompt.md \
    > logs/run-<id>/00-architecture.log 2>&1
```

The agent receives the full prompt as input, reads all the files it references using its
own Read/Glob/Grep tools, and then writes output files using the Write tool.

## Parallelism

Multiple prompts can run in parallel — each is independent. The `run-migration.sh` script
defaults to 3 parallel agents. Increase with `--parallel N` if you have the budget.

## Running a single topic

```bash
./run-migration.sh --only 00-architecture
```

Or multiple:

```bash
./run-migration.sh --only "00,01,02"
```

## Re-running after a failure

```bash
# Re-verify without re-running the agent
./run-migration.sh --only 00-architecture --verify-only

# Force re-run even if output exists
./run-migration.sh --only 00-architecture --force
```

## Editing prompts

If the output quality is unsatisfactory for a topic, edit its prompt file to:
- Add missing source files to the source list
- Clarify the sub-doc breakdown
- Add topic-specific rules to the CRITICAL REMINDERS section
- Increase the minimum sub-doc count or line count

Then re-run with `--force`.

## Adding a new topic

1. Create a new prompt file named `NN-topic.prompt.md` in this directory.
2. Add the topic to `lib/common.sh` `ALL_TOPICS` array.
3. Add a section for the topic in `SOURCE-INDEX.md`.
4. Run: `./run-migration.sh --only NN-topic`.
