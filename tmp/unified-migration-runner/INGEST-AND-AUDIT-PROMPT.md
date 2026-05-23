# Ingest-and-Audit Prompt

Copy everything below the `---` line. Replace `[DOCS_FOLDER]` with the folder name (e.g., `00-architecture`, `03-composition`, `11-safety`).

This prompt does three things in sequence:
1. **Writes depth docs** — redesigns source material using unified primitives
2. **Audits the codebase** — finds gaps between spec and code
3. **Appends runner batches** — creates migration tasks for the automated runner

After processing all 21 docs/ folders, the runner at `tmp/unified-migration-runner/` will have all batches populated and can execute overnight.

## Folder → Depth Directory → Spec File Mapping

| docs/ folder | Depth dir | Spec file(s) |
|---|---|---|
| `00-architecture` | `00-index/`, `01-signal/`, `02-block/` | `00-INDEX`, `01-SIGNAL`, `02-CELL` |
| `01-orchestration` | `03-graph/`, `05-execution-engine/` | `03-GRAPH`, `05-EXECUTION-ENGINE` |
| `02-agents` | `07-agent-runtime/` | `07-AGENT-RUNTIME` |
| `03-composition` | `02-block/` | `02-CELL`, `14-CONFIG-AND-AUTHORING` |
| `04-verification` | `02-block/` | `02-CELL` (Verify protocol) |
| `05-learning` | `10-learning-loops/` | `10-LEARNING-LOOPS` |
| `06-neuro` | `11-memory/` | `11-MEMORY-AND-KNOWLEDGE` |
| `07-conductor` | `05-execution-engine/`, `09-telemetry/` | `05-EXECUTION-ENGINE`, `09-TELEMETRY` |
| `08-chain` | `18-registries/` | `18-ON-CHAIN-REGISTRIES` |
| `09-daimon` | `07-agent-runtime/` | `07-AGENT-RUNTIME` |
| `10-dreams` | `11-memory/` | `11-MEMORY-AND-KNOWLEDGE` |
| `11-safety` | `17-security/` | `17-SECURITY-MODEL` |
| `12-interfaces` | `16-surfaces/` | `16-SURFACES` |
| `13-coordination` | `12-connectivity/` | `12-CONNECTIVITY` |
| `14-identity-economy` | `15-marketplace/` | `15-MARKETPLACE-AND-SHARING` |
| `15-code-intelligence` | `13-builtin-catalog/` | `13-BUILTIN-BLOCK-CATALOG` |
| `16-heartbeat` | `07-agent-runtime/` | `07-AGENT-RUNTIME` |
| `17-lifecycle` | `07-agent-runtime/` | `07-AGENT-RUNTIME` |
| `18-tools` | `13-builtin-catalog/` | `13-BUILTIN-BLOCK-CATALOG` |
| `19-deployment` | `20-deployment/` | `20-DEPLOYMENT` |
| `20-technical-analysis` | `21-roadmap/` | `21-ROADMAP` |
| `21-references` | `21-roadmap/` | `21-ROADMAP` |

---

## NAMING PRECEDENCE — READ THIS FIRST

```
tmp/unified/      → CANONICAL (22 spec files) — overrides EVERYTHING
tmp/architecture/ → CURRENT DESIGN (21 files) — supplements unified
docs/             → LEGACY (422 files) — algorithms and detail only, NEVER naming
```

When naming conflicts exist: unified wins. Always. The unified spec defines Signal, Cell, Graph, 9 protocols, 10 specializations. The architecture files define implementation patterns (9-step heartbeat, T0/T1/T2 gating, 12 primitives, Connector/Feed/Recipe). The docs/ folder is historical input — use it for algorithms and domain knowledge, never as a source of truth for names or structure.

## Instructions

You have three tasks. Execute them in order.

### Phase 1: Read Context

**Step 1a — Read the unified spec (CANONICAL, overrides everything):**

- `/Users/will/dev/nunchi/roko/roko/tmp/unified/00-INDEX.md` — vocabulary, principles, reading order
- `/Users/will/dev/nunchi/roko/roko/tmp/unified/01-SIGNAL.md` — Signal + Pulse + Bus + HDC + demurrage
- `/Users/will/dev/nunchi/roko/roko/tmp/unified/02-CELL.md` — Cell trait, 9 protocols, predict-publish-correct
- `/Users/will/dev/nunchi/roko/roko/tmp/unified/04-SPECIALIZATIONS.md` — 10 specializations (Flow through Connector)

**Step 1b — Read the architecture files (current design, supplements unified):**

Read ALL files in `/Users/will/dev/nunchi/roko/roko/tmp/architecture/` — especially:
- `00-INDEX.md` — 12 primitives overview, dependency graph
- `02-agent-runtime.md` — 9-step heartbeat, T0/T1/T2, cortical state
- `03-extensions.md` — 8 layers, 22 hooks, Connector primitive
- `09-knowledge.md` — InsightStore, 6 kinds, HDC, Ebbinghaus decay

**Step 1c — Read the depth layer guide and master index:**

- `/Users/will/dev/nunchi/roko/roko/tmp/unified-depth/GUIDE.md`
- `/Users/will/dev/nunchi/roko/roko/tmp/unified-depth/INDEX.md`

**Step 1d — Read ALL files in the source folder:**

- `/Users/will/dev/nunchi/roko/roko/docs/[DOCS_FOLDER]/`

Read the target depth directory's INDEX.md (use the mapping table above to find which depth directory this folder maps to). If the folder maps to multiple depth directories, read all their INDEX.md files.

**Step 1e — Read ALL migration phase files (these drive the batches):**

- `/Users/will/dev/nunchi/roko/roko/tmp/unified-migration/00-INDEX.md`
- `/Users/will/dev/nunchi/roko/roko/tmp/unified-migration/01-PHASE-0-PREP.md`
- `/Users/will/dev/nunchi/roko/roko/tmp/unified-migration/02-PHASE-1-KERNEL.md`
- `/Users/will/dev/nunchi/roko/roko/tmp/unified-migration/03-PHASE-2-ENGINE.md`
- `/Users/will/dev/nunchi/roko/roko/tmp/unified-migration/04-PHASE-3-ECONOMY.md`

Read the runner context pack for the vocabulary and rules your batches will operate under:

- `/Users/will/dev/nunchi/roko/roko/tmp/unified-migration-runner/context-pack/01-unified-vocabulary.md`
- `/Users/will/dev/nunchi/roko/roko/tmp/unified-migration-runner/context-pack/02-migration-rules.md`

### Phase 2: Write Depth Docs

**Do not copy, translate, or summarize the source material.** Redesign from scratch using unified primitives (Signal, Cell, Graph + 9 protocols + 10 specializations).

For each depth doc you write:

1. **Redesign, don't transcribe** — Take the core insight and re-derive using unified primitives. The new version should feel native to the Signal/Cell/Graph model.

2. **Improve and extend** — For every mechanism: What would 10x better look like? What adjacent capability does this unlock? Where does it break at scale?

3. **Innovate beyond the source** — Novel primitives, emergent capabilities from the unified model, cybernetic loops, exponential scaling patterns, unique differentiators.

4. **Find gaps and contradictions** — Missing feedback loops, missing failure modes, over/under-engineering.

5. **Combine across boundaries** — The biggest wins come from combining ideas that were in separate specs.

**Output format for depth docs:**

```markdown
# [Title]

> Depth for [spec-file]. [One sentence on what this adds.]

[Content: algorithms, pseudocode, config examples, novel ideas]

## What This Enables
[Capabilities that didn't exist before]

## Feedback Loops
[How the system observes, learns, and improves itself]

## Open Questions
[Genuine unknowns worth investigating]
```

Write depth docs to the appropriate directories under `/Users/will/dev/nunchi/roko/roko/tmp/unified-depth/`.

After writing, update each directory's INDEX.md:
- Move ingested source docs from "Pending" to "Absorbed"
- Add new depth docs to the "Depth docs" section

### Phase 3: Audit Codebase and Generate Runner Batches

Now audit the actual codebase to find gaps between the spec/depth docs and the code. For each docs/ folder, identify the relevant crates and:

1. **Grep/read** the relevant crate source code
2. **Compare** what the spec says vs what the code does
3. **Identify gaps**:
   - Dead code that needs wiring (built but not connected)
   - Missing implementations (spec says X, code doesn't have it)
   - Renames needed (Engram→Signal, Substrate→Store, etc.)
   - Test gaps (changed code paths without tests)
   - Inconsistencies between crates

For each gap found, generate a migration batch:

#### A. Write a batch prompt file

Create `/Users/will/dev/nunchi/roko/roko/tmp/unified-migration-runner/prompts/M###-short-name.prompt.md`:

```markdown
# M### — [Short Title]

## Objective
[One paragraph: what this batch does and why]

## Scope
- Crates: `roko-<name>` [list affected crates]
- Files: [list key files to modify]
- Phase ref: [unified-migration phase file + section]

## Steps
1. [Concrete step with file paths]
2. [Concrete step with file paths]
3. [Concrete step with file paths]

## Verification
```bash
cargo check -p <crate>
cargo clippy -p <crate> --no-deps -- -D warnings
[additional verification commands]
```

## What NOT to do
- [Specific anti-patterns for this batch]
```

#### B. Append to lib/common.sh

For each batch, add entries to the following functions in
`/Users/will/dev/nunchi/roko/roko/tmp/unified-migration-runner/lib/common.sh`:

1. Add the batch ID to the `ALL_BATCHES=()` array
2. Add a case to `batch_title()`: `M###) echo "Short description" ;;`
3. Add a case to `batch_deps()`: `M###) echo "M001 M002" ;;` (if it has dependencies)
4. Add a case to `batch_group()`: `M###) echo "phase1" ;;`
5. Add a case to `batch_verify_commands()` with the appropriate cargo commands
6. Add a case to `batch_phase_ref()`: `M###) echo "02-PHASE-1-KERNEL.md §1.3" ;;`

#### C. Write context files (if needed)

If a batch needs extra context beyond the shared context pack, create
`/Users/will/dev/nunchi/roko/roko/tmp/unified-migration-runner/context/M###.md` or a
directory `context/M###/` with multiple files.

#### D. Update MASTER-CHECKLIST.md

Add each new batch to the appropriate phase section in
`/Users/will/dev/nunchi/roko/roko/tmp/unified-migration-runner/MASTER-CHECKLIST.md`:

```markdown
- [ ] **M###** — [title]. Phase ref: [phase-file §section].
```

Update the summary table counts.

### Numbering Convention

Batch IDs are assigned sequentially: M001, M002, M003, ...

When appending to an existing set, check the current highest batch number in `ALL_BATCHES` and continue from there. Don't leave gaps.

### Ordering and Dependencies

- Phase 0 batches come first (prep, cleanup)
- Phase 1 batches next (kernel renames)
- Phase 2 batches after (engine rewiring)
- Phase 3 batches last (economy)
- Within a phase, order by dependency: if M005 depends on M003, M003 comes first
- Minimize cross-crate dependencies within a batch

### Quality Bar for Batch Prompts

Each batch prompt must be **self-contained and agent-executable**:

- An agent reading only the context pack + the batch prompt should be able to complete it
- Include exact file paths (absolute preferred)
- Include exact grep patterns for finding code to modify
- Include the verification commands that must pass
- Include anti-patterns specific to this batch
- Target 15-60 minutes of agent work per batch

### Example Batch Prompt

```markdown
# M001 — Type alias: Engram → Signal in roko-core

## Objective
Add `pub type Signal = Engram;` to roko-core's public API, making Signal the canonical
name while preserving backward compatibility during migration.

## Scope
- Crates: `roko-core`
- Files: `crates/roko-core/src/lib.rs`, `crates/roko-core/src/types/mod.rs`
- Phase ref: 02-PHASE-1-KERNEL.md §1.1

## Steps
1. Find the Engram type definition:
   `grep -rn 'pub struct Engram' crates/roko-core/src/ --include='*.rs'`
2. Add a type alias in the same module: `pub type Signal = Engram;`
3. Re-export from `crates/roko-core/src/lib.rs`
4. Update any doc comments that reference "Engram" to mention "Signal"

## Verification
```bash
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
cargo test -p roko-core --lib --no-run
```

## What NOT to do
- Do NOT rename the Engram struct itself yet — that's a later batch
- Do NOT update other crates — they'll migrate in subsequent batches
- Do NOT add new functionality — this is purely a naming bridge
```
