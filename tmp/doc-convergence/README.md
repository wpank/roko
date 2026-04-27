# Doc Convergence — Unified Spec v3

## The Problem

Roko's specs and plans are scattered across 4 disconnected layers:

| Layer | Location | Files | Date | Vocabulary |
|---|---|---|---|---|
| **v1** | `docs/v1/` | 417 | Apr 12 | Old (Engram, Gate, Substrate, Scorer) |
| **v2** | `docs/v2/` | 34 | Apr 26 | New (Signal, Cell, Graph, Protocol) |
| **v2-depth** | `docs/v2-depth/` | 180 | Apr 26 | Mixed (40% absorbed from v1, 60% pending) |
| **tmp/prds** | `tmp/prds/` | 22 | Apr 21 | Old vocabulary, implementation-oriented |

Additionally:
- `bardo-backup/prd/` — 359 files of original bardo PRDs (historical)
- `bardo-backup/tmp/roko-progress/` — 140+ files of parity checklists
- `~/Downloads/isfr-index-spec-v4.md` and `~/Downloads/02-isfr-index.md` — standalone ISFR specs
- The actual Rust code uses v1 vocabulary (Engram, Gate, etc.)

None of these reference each other. The code doesn't match any spec fully.

## The Goal

Produce `docs/v3/` — a single canonical spec that:
1. Uses v2's vocabulary and architecture model (Signal/Cell/Graph/Protocol)
2. Includes v1's depth (the 60% that v2-depth never absorbed)
3. Includes tmp/prds' implementation detail and task checklists
4. Annotates every section with actual code status (DONE / PARTIAL / NOT STARTED)
5. Identifies new synergies visible only when all sources are combined
6. Produces actionable implementation plans that can feed into `roko prd`

## The Process

### Phase 1: Build the Topic Matrix
**Script**: `scripts/01-build-matrix.sh`
**Prompt**: `prompts/01-build-matrix.md`
**Output**: `status/MATRIX.md`

Scans all doc sets and code to produce a single matrix: one row per topic, showing where it's covered and what state it's in.

### Phase 2: Per-Topic Convergence (parallelizable)
**Script**: `scripts/02-converge-topics.sh`
**Prompt**: `prompts/02-converge-topic.md` (parameterized by topic)
**Output**: `output/{NN}-{TOPIC}.md` (one per topic)

For each topic in the matrix, an agent:
1. Reads all source docs for that topic (v1 + v2 + v2-depth + tmp/prds)
2. Reads the actual Rust code implementing that topic
3. Produces a converged doc with: Spec, Status, Plan, Discoveries

**This phase runs N agents in parallel** — one per topic or batched.

### Phase 3: Cross-Topic Synthesis
**Script**: `scripts/03-synthesize.sh`
**Prompt**: `prompts/03-synthesize.md`
**Output**: `output/00-SYNTHESIS.md`

Reads all converged topic docs and identifies:
- Redundant subsystems
- Missing connections
- New synergies
- Priority reordering
- Revised roadmap

### Phase 4: Dogfood into Roko
**Script**: `scripts/04-dogfood.sh`
**Prompt**: `prompts/04-dogfood.md`
**Output**: `.roko/prd/` entries + `plans/` task files

Converts the converged spec + plans into roko's own PRD and task format so roko tracks its own implementation.

### Phase 5: Redesign Pass
**Script**: `scripts/05-redesign.sh`
**Prompt**: `prompts/05-redesign.md`
**Output**: `output/00-REDESIGN.md`

With full convergence complete, do a fresh architecture review:
- What should change given everything we now see?
- What new features emerge from combining subsystems?
- What should be cut?
- Updated architecture diagrams

## Directory Structure

```
tmp/doc-convergence/
  README.md              # This file
  prompts/
    01-build-matrix.md   # Phase 1: inventory all docs
    02-converge-topic.md # Phase 2: per-topic convergence template
    03-synthesize.md     # Phase 3: cross-topic synthesis
    04-dogfood.md        # Phase 4: feed into roko
    05-redesign.md       # Phase 5: architecture redesign
  scripts/
    01-build-matrix.sh   # Run Phase 1
    02-converge-topics.sh # Run Phase 2 (parallel agents)
    03-synthesize.sh     # Run Phase 3
    04-dogfood.sh        # Run Phase 4
    05-redesign.sh       # Run Phase 5
    run-all.sh           # Run everything in sequence
    _common.sh           # Shared variables and functions
  output/                # Converged docs land here
  status/
    MATRIX.md            # Topic matrix (Phase 1 output)
    PROGRESS.md          # Overall convergence progress tracker
```

## How to Run

```bash
# Phase 1: Build the matrix (single agent, ~10 min)
./scripts/01-build-matrix.sh

# Phase 2: Converge all topics (parallel agents, ~30-60 min)
./scripts/02-converge-topics.sh

# Or converge a single topic:
./scripts/02-converge-topics.sh 05-AGENT

# Phase 3: Synthesize (single agent, ~15 min)
./scripts/03-synthesize.sh

# Phase 4: Dogfood (single agent, ~10 min)
./scripts/04-dogfood.sh

# Phase 5: Redesign (single agent, ~20 min)
./scripts/05-redesign.sh

# Or run everything:
./scripts/run-all.sh
```

## Key Paths

| What | Absolute Path |
|---|---|
| Workspace root | `/Users/will/dev/nunchi/roko/roko/` |
| v1 docs | `/Users/will/dev/nunchi/roko/roko/docs/v1/` |
| v2 docs (canonical) | `/Users/will/dev/nunchi/roko/roko/docs/v2/` |
| v2-depth docs | `/Users/will/dev/nunchi/roko/roko/docs/v2-depth/` |
| tmp/prds | `/Users/will/dev/nunchi/roko/roko/tmp/prds/` |
| impl status | `/Users/will/dev/nunchi/roko/roko/tmp/prds/impl/STATUS.md` |
| Rust crates | `/Users/will/dev/nunchi/roko/roko/crates/` |
| Bardo backup PRDs | `/Users/will/dev/nunchi/roko/bardo-backup/prd/` |
| Mori parity checklist | `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/MORI-PARITY-CHECKLIST.md` |
| ISFR spec v4 | `/Users/will/Downloads/isfr-index-spec-v4.md` |
| Convergence output | `/Users/will/dev/nunchi/roko/roko/tmp/doc-convergence/output/` |
