# Batch AUD07: Fix codebase reality-check errors across ALL docs

**Audit refs**: 06-codebase-reality-check.md (full file), 07-doc-quality-audit.md
(Issue A: Signal references, Issue B: target crates, Issue C: stale status).
This is the broad sweep batch -- it touches many files to fix factual errors
that span the entire docs tree.

Read these files first:

- `tmp/refinement-audit-runner/context-pack/00-AUDIT-RULES.md`
- `tmp/refinements-audit/06-codebase-reality-check.md` (full file -- 10 reality checks)
- `tmp/refinements-audit/07-doc-quality-audit.md` (Issues A, B, C + section 10 Recommended Actions)
- `docs/00-architecture/01-naming-and-glossary.md` (retired terms table)
- `CLAUDE.md` (ground truth for what is wired)

For the Signal sweep, also read:

- `docs/12-interfaces/02-roko-new-scaffolders.md`
- `docs/07-conductor/01-watcher-ensemble.md`
- `docs/07-conductor/INDEX.md`
- `docs/11-safety/15-forensic-ai.md`
- `docs/01-orchestration/00-layer-overview.md`
- `docs/CLI-REFERENCE.md` (if it exists at `docs/CLI-REFERENCE.md`)

## Task

Fix factual errors found by the codebase reality check across the entire docs
tree. Three categories: (1) Replace stale `Signal` type references with
`Engram`, (2) Fix incorrect LOC/crate/route counts wherever they appear,
(3) Mark 0-code concepts as planned wherever they are presented in present
tense.

## Current state (evidence)

### Category 1: Signal -> Engram stragglers

The doc quality audit found `Signal` still used as a live Rust type in code
snippets across at least 8 pre-existing docs that the refinements-runner did
not touch:

| File | Context |
|---|---|
| `docs/12-interfaces/02-roko-new-scaffolders.md` line 65 | `use roko_core::{Context, Gate, Signal, Verdict};` |
| `docs/12-interfaces/02-roko-new-scaffolders.md` line 88 | `output: &Signal` |
| `docs/12-interfaces/02-roko-new-scaffolders.md` line 109 | `let signal = Signal::builder(Kind::AgentOutput)` |
| `docs/07-conductor/01-watcher-ensemble.md` line 17 | `fn decide(&self, stream: &[Signal], ctx: &Context) -> Vec<Signal>;` |
| `docs/07-conductor/INDEX.md` line 102 | `fn decide(&self, stream: &[Signal], ctx: &Context) -> Vec<Signal>;` |
| `docs/11-safety/15-forensic-ai.md` line 248 | `-> Vec<Signal>` |
| `docs/01-orchestration/00-layer-overview.md` line 65 | `roko_core::Signal` |
| `docs/CLI-REFERENCE.md` lines 116, 1043, 1109 | "Signal hash", "Signal trigger", "Signal kind" |

The glossary correctly marks `Signal` as retired. These docs were not swept.

### Category 2: Incorrect numbers

The reality check found these discrepancies:

| Claim in docs | Reality | Where to fix |
|---|---|---|
| ~177K LOC | 322,088 LOC | Any doc citing 177K |
| 18 crates | 36 workspace members | Any doc citing 18 |
| ~85 routes | 200+ routes | Any doc citing 85 |
| 1,568 tests | 3,761 test functions | STATUS.md (done in AUD01, verify others) |

Search ALL docs for these stale numbers and fix them.

### Category 3: 0-code concepts in present tense

The reality check confirmed these have zero code:

| Concept | Lines of code |
|---|---|
| Demurrage | 0 |
| Pulse (struct) | 0 |
| Bus (trait) | 0 (EventBus<E> struct exists) |
| Datum | 0 |
| Worldview | 0 |
| Custody | 0 |
| Claim / Paper | 0 |
| Replication ledger | 0 |
| Plugin SPI / roko-spi | 0 |
| Graduation (Pulse -> Engram) | 0 |

Any doc that describes these in present tense ("Engrams carry demurrage
balance", "Pulse types flow through the Bus") needs qualifying. Previous
batches (AUD02-AUD06) handle specific sections; this batch catches any
remaining instances across the full tree.

## Implementation

### 1. Signal -> Engram sweep

For each file listed in the table above:

- Replace `Signal` with `Engram` in Rust code snippets
- Replace `signal` with `engram` in variable names within code snippets
- Replace `Signal::builder` with `Engram::builder`
- Replace `&[Signal]` with `&[Engram]`
- Replace `Vec<Signal>` with `Vec<Engram>`
- Replace `use roko_core::{..., Signal, ...}` with `use roko_core::{..., Engram, ...}`
- In prose, replace "Signal hash" with "Engram hash", "Signal kind" with
  "Engram kind", etc.
- If a doc has a note like "will be renamed to Engram in Tier 0D", remove that
  note since the rename is complete

Also search for any OTHER docs not in the audit's list that still use `Signal`
as a type name. Run a search across `docs/` for the pattern. Exclude:
- The glossary's "Retired Terms" table (which correctly lists Signal as retired)
- Unix signal references (SIGTERM, SIGKILL) which are unrelated
- Meta-references ("the Signal -> Engram rename is complete")

### 2. Fix stale numbers across all docs

Search `docs/` for:
- `177K` or `177,000` or `~177` -- replace with `322K` or `~320K`
- `18 crates` -- replace with `36 crates`
- `~85 routes` or `85 routes` -- replace with `200+ routes`
- `1,568 tests` or `1568` -- replace with `3,761 tests` or `~3,700 tests`

Be careful to only fix references to the overall codebase. If a doc says
"roko-core has 18 modules" that is a different number and should not be
changed.

### 3. Catch remaining 0-code present-tense claims

Search `docs/` for present-tense usage of the 0-code concepts listed above.
Focus on claims like:
- "Engrams carry demurrage balance"
- "Pulse messages flow through..."
- "The Bus trait provides..."
- "Datum abstracts over..."

If AUD02-AUD06 already handled the file, skip it. Only fix files NOT covered
by those batches. Add a brief qualifier like `(target-state)` or
`(planned; not yet implemented)` after the present-tense claim.

## Write scope

Primary (Signal sweep):
- `docs/12-interfaces/02-roko-new-scaffolders.md`
- `docs/07-conductor/01-watcher-ensemble.md`
- `docs/07-conductor/INDEX.md`
- `docs/11-safety/15-forensic-ai.md`
- `docs/01-orchestration/00-layer-overview.md`
- `docs/CLI-REFERENCE.md` (if it exists)
- Any other docs found with stale `Signal` type references

Secondary (number fixes):
- Any doc in `docs/` that cites 177K LOC, 18 crates, ~85 routes, or 1,568 tests

Tertiary (0-code qualifiers):
- Any doc in `docs/` NOT already covered by AUD02-AUD06 that uses 0-code
  concepts in present tense

## Rules

1. **Signal -> Engram is mechanical.** Do not rewrite surrounding prose. Just
   swap the type name and update variable names in code snippets.
2. **Number fixes are mechanical.** Replace old number with new number. Do not
   rewrite surrounding context.
3. **0-code qualifiers are minimal.** Add `(target-state)` or `(planned)` --
   do not add multi-line callouts. The callouts were handled by AUD02-AUD06.
4. **Do NOT touch files already fully handled by AUD02-AUD06.** If a file was
   in their write scope AND the specific issue was addressed there, skip it.
   If a file was in their scope but they did not address this specific issue,
   fix it.
5. **Preserve the glossary's retired-terms table.** Do not remove Signal from
   the retired list. That entry is correct and useful.
6. **Do not rewrite prose.** This is a factual correction batch, not a
   narrative rewrite.
7. **Be thorough.** Search the entire `docs/` tree. The audit found 8 files
   with Signal issues; there may be more.

## Done when

- Zero docs use `Signal` as a live Rust type name in code snippets (except
  the retired-terms table and Unix signal references)
- Zero docs cite 177K LOC, 18 crates, ~85 routes, or 1,568 tests
- Any 0-code concept used in present tense (outside AUD02-AUD06 scope) has
  a qualifier
- All edits are mechanical (type swap, number swap, brief qualifier) -- no
  prose rewrites
- Final message lists: (a) number of files with Signal->Engram fixes, (b) number
  of files with stale numbers fixed, (c) number of files with 0-code qualifiers
  added, (d) the full list of files edited
