# Doc Quality Audit: Refinements Runner Output

**Auditor:** Claude Opus 4.6
**Date:** 2026-04-17
**Branch:** `agent-refinements` (d29d34cf)
**Scope:** 35 refinement proposals (REF01-REF35) propagated into `docs/`

---

## Executive Summary

The refinements runner produced **high-quality structural documentation** with consistent
terminology, coherent voice, and well-linked cross-references. The strongest chapters --
synergy map, safety spine, observability, StateHub, and the consolidated roadmap -- read as
unified design documents, not pasted-in fragments. However, the audit found **three systemic
issues** and a handful of localized problems that should be addressed before these docs become
the source of truth for implementation.

**Overall score: 3.8 / 5** -- good enough to use as planning material, not yet clean enough
to ship as a developer guide or external spec.

---

## 1. docs/INDEX.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/INDEX.md`

### Assessment

The top-level INDEX is **coherent but excessively front-loaded**. Lines 13-161 form a single
growing paragraph that was appended to by successive refinements. Each REF adds another
sentence or clause pointing to a new doc. The result is a 150-line block quote that no
developer will read.

**Specific issues:**

| Issue | Lines | Severity |
|---|---|---|
| Wall-of-text "Current Framing" block with 15+ REF citations | 171-214 | Medium |
| Absolute paths to the user's local machine in generation notes | 251-253 | Low (cosmetic) |
| Topics list descriptions are inconsistent: some include REF numbers, others do not | 224-245 | Low |

### Scores

| Dimension | Score | Notes |
|---|---|---|
| Consistency | 4/5 | Terms match glossary throughout |
| Coherence | 2/5 | The "Current Framing" block is an accretive wall |
| Accuracy | 4/5 | Cross-references are valid |
| Completeness | 5/5 | Every refinement is cited |
| Terminology hygiene | 4/5 | No retired terms in active use |
| Internal links | 5/5 | All sampled links resolve |

---

## 2. docs/00-architecture/ (Architecture)

### 2.1 INDEX.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/INDEX.md`

The architecture INDEX is the best-structured chapter file in the tree. The reading order in
the Prerequisites section (lines 133-142) is genuinely useful. The contents table (lines
83-122) is complete and each entry's description was updated to reflect refinement content.

**Issues found:**

| Issue | File | Lines | Severity |
|---|---|---|---|
| **STALE STATUS: "roko-serve: HTTP API not wired"** | INDEX.md | 206 | **High** |
| **STALE STATUS: "TUI: Text-mode dashboard only, no interactive terminal UI"** | INDEX.md | 208 | **High** |
| Generated date says "2026-04-11" but Sub-docs says 29 when there are now 36 files | INDEX.md | 224-226 | Medium |
| "roko-agent (346 tests)" -- test count is stale and unverified | INDEX.md | 193 | Low |

The high-severity items directly contradict CLAUDE.md, which marks both `roko-serve` (~85
routes, wired) and the interactive TUI (ratatui, F1-F7 tabs) as **Wired**. The refinements
runner did not update the "Current Status and Implementation Gaps" section at the bottom of
this INDEX to reflect reality. This section was generated on 2026-04-11 and has not been
touched since, even though the code has moved significantly.

### 2.2 01-naming-and-glossary.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/01-naming-and-glossary.md`

This is the strongest doc in the set. The A-Z glossary format works well. Every new term from
REF10-REF35 has a corresponding entry. The "Retired / Deprecated Terms" table (lines 613-633)
is explicit and thorough.

**Minor issues:**

- The "Terms Deliberately Not Defined Here" section (lines 636-645) is a nice touch but could
  also mention `trace`, `span`, and `principal` which are used architecturally but not given
  Roko-specific definitions.
- The glossary links to `tmp/refinements/` files extensively. Those are source proposals, not
  canonical docs. Over time these links will rot if the refinements directory moves.

**Score: 4.5/5** -- excellent vocabulary discipline.

### 2.3 34-synergy-integration-map.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/34-synergy-integration-map.md`

This is one of the best-written docs in the set. The 10-primitive table, the synergy matrix,
and the 10 named synergies section all read as a single coherent voice. The "Non-Synergies
Worth Naming" section (section 8) is unusually honest for generated documentation.

**No issues found.** This doc is ready to use as-is.

**Score: 5/5**

### 2.4 35-consolidated-roadmap.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/35-consolidated-roadmap.md`

Well-structured Q1-Q4 breakdown with clear dependency ladder. The "Not-Doing List" (section 7)
and "One-Year Outcome" (section 8) are useful framing.

**Issues:**

| Issue | Lines | Severity |
|---|---|---|
| Q5-Q6 section header says "Phase 2 Optionality" -- unclear if these are real quarters or metaphorical | 166-178 | Low |
| Team shape section (lines 187-200) assumes 5-7 engineers; unclear if this matches project reality | 187-200 | Low |

**Score: 4/5** -- solid planning doc.

### 2.5 15-crate-map.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/15-crate-map.md`

**Accuracy issue (Medium severity):** The crate map describes `roko-bus`, `roko-hdc`, and
`roko-spi` as "target kernel crates" that do not yet exist. This is **correctly qualified**
throughout: "target boundaries proposed by REF20, not all fully shipped" (line 319), "New
target kernel crate" (lines 76-77). The same is true for `roko-defaults`, `roko-tools`,
`roko-compose-core`, and `roko-templates` -- none of these exist as separate crates yet.

Verified against actual workspace:

- `roko-bus` -- **does not exist**
- `roko-hdc` -- **does not exist**
- `roko-spi` -- **does not exist** (though `roko-plugin` does exist)
- `roko-defaults` -- **does not exist**
- `roko-tools` -- **does not exist**
- `roko-compose-core` -- **does not exist**
- `roko-templates` -- **does not exist**

The doc is honest about this gap. However, other docs reference these crates without the same
qualification (e.g., `docs/00-architecture/12-five-layer-taxonomy.md` line 221: "roko-core,
roko-bus, roko-hdc, and roko-spi are the only kernel-tier crates" -- present tense, not
qualified as target). This creates a consistency problem across the architecture chapter.

**Score: 4/5** -- good doc, but the current-vs-target boundary bleeds in adjacent files.

---

## 3. docs/05-learning/ (Learning)

### 3.1 INDEX.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/05-learning/INDEX.md`

The INDEX reads well as a unified story. The overview (lines 11-19) integrates REF10, REF12,
REF14, and REF16 coherently: prediction loops, demurrage, heuristics, and research-to-runtime
are woven into a single narrative rather than listed as separate additions.

**Issues:**

| Issue | Lines | Severity |
|---|---|---|
| "four durable learning surfaces" (line 19) then lists parenthetical "(episodes -> patterns -> heuristics/worldviews -> playbook projections)" which is actually 4 items, consistent | 19 | None |
| Rust code blocks in cross-cutting concerns section use types like `HdcVector`, `DifficultyModel` that exist only in the PRD, not the codebase | 251-360 | Medium |
| No explicit "future work" disclaimer on the Rust code blocks | 250-360 | Medium |

### 3.2 18-self-learning-cybernetic-loops.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/05-learning/18-self-learning-cybernetic-loops.md`

Reads as one coherent voice. The predict-publish-correct loop is explained clearly. The
per-operator calibration table (lines 36-43) is useful and concise.

**No significant issues.** The integration of REF10 concepts is natural, not pasted in.

**Score: 4.5/5**

### 3.3 19-heuristics-worldviews-and-falsifiers.md

**File:** `/Users/well/dev/nunchi/roko/roko/docs/05-learning/19-heuristics-worldviews-and-falsifiers.md`

Well-structured, reads as one document. The Rust struct definitions for `Heuristic` and
`Calibration` (lines 41-60) are clear specification material.

**Score: 4/5**

---

## 4. docs/12-interfaces/ (Interfaces)

### 4.1 INDEX.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/12-interfaces/INDEX.md`

The overview paragraph (lines 9) is a single **1,500-character sentence** that tries to cite
REF23, REF24, REF25, REF26, REF27, REF28, REF29, and REF30 all in one breath. This is the
worst accretive-citation problem in the tree. The Generation Notes section (lines 104-157)
then repeats most of the same citations in a separate list format.

**Issues:**

| Issue | Lines | Severity |
|---|---|---|
| Overview paragraph is an accretive mega-sentence citing 8 REFs | 9 | Medium |
| Generation Notes duplicates the same REF citations as separate blocks | 115-157 | Medium |
| Sub-Documents table skips #20 (exists: `20-ide-integration-strategy.md`) | 27-51 | Low |
| Prerequisites table (lines 57-70) uses inconsistent topic numbering: "01-core" for architecture, "05-orchestration" for orchestration (should be 01) | 61-64 | Low |

### 4.2 22-statehub-projection-layer.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/12-interfaces/22-statehub-projection-layer.md`

Excellent doc. The Projection trait (lines 28-41) is a clean spec. The canonical projections
table (lines 58-82) is comprehensive and well-scoped. The query+subscribe API examples (lines
89-95) are useful.

REF30 rich-UX primitives are integrated naturally -- the doc explains how projections supply
the typed data that reasoning streams, uncertainty bars, and replay scrubbers need, without
making those UI concepts feel bolted on.

**Score: 4.5/5**

---

## 5. docs/11-safety/ (Safety)

### 5.1 INDEX.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/11-safety/INDEX.md`

Clean, well-organized spine. REF32 is integrated as the through-line rather than an appendix.
The "Chapter Through-Line" section (lines 73-83) is a useful reading guide.

**Score: 4.5/5**

### 5.2 00-defense-in-depth.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/11-safety/00-defense-in-depth.md`

This doc is genuinely good. The shared permission vocabulary (section 1), seven-step loop
safety mapping (section 4), and defense layers stack (section 6) all read as unified design.
The `AuthzDecision` enum (lines 46-53) is a clean, small spec.

**No issues found.** The integration of REF32 concepts (TypedContext, Custody, taint) is
seamless.

**Score: 5/5**

---

## 6. docs/19-deployment/ (Deployment)

### 6.1 INDEX.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/19-deployment/INDEX.md`

Good structure. The five deployment shapes (laptop, single-server, container, clustered, edge)
are consistent throughout. The Key Concepts section (lines 34-42) integrates Engram, Pulse,
Bus, StateHub, and profiles cleanly.

**Issues:**

| Issue | Lines | Severity |
|---|---|---|
| Cross-references at bottom use vague labels like "Agent Types documentation, section 8" without links | 55-60 | Medium |
| "Synapse Architecture: 6-trait composition system" (line 36) lists the original six traits but no longer reflects Bus as a distinct fabric -- the framing is pre-REF03 | 36 | Low |

**Score: 4/5**

### 6.2 14-observability-and-telemetry.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/19-deployment/14-observability-and-telemetry.md`

One of the best-integrated refinement docs. REF33 content is woven into a coherent operator
story. The Roko-specific metrics table (lines 109-124) is concretely useful. The
"Replay and Time-Travel" section (lines 181-196) connects deployment concerns to the
Bus/Engram model naturally.

**Score: 5/5**

---

## 7. Systemic Issues

### Issue A: "Signal" still appears in active code and docs (HIGH)

The glossary correctly marks `Signal` as retired in favor of `Engram`. However, **`Signal`
still appears as a live Rust type name** in code snippets across at least 8 docs:

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

These docs were NOT updated by the refinements runner because they predate the glossary
changes. The scaffolder doc (02-roko-new-scaffolders.md) explicitly notes "will be renamed to
Engram in Tier 0D" (line 128), which is at least honest. But the conductor, orchestration,
forensic, and CLI-REFERENCE docs use `Signal` without any qualification.

**This is the biggest terminology hygiene gap in the tree.** The refinements correctly updated
all newly-written or REF-touched docs, but the pre-existing docs were not swept.

### Issue B: Target crates described as if they exist (MEDIUM)

The docs reference `roko-bus`, `roko-hdc`, `roko-spi`, `roko-defaults`, `roko-tools`,
`roko-compose-core`, and `roko-templates` across at least 7 files. None of these crates exist
in the workspace. The crate-map doc (`15-crate-map.md`) is honest about this gap, but other
docs are not:

- `docs/00-architecture/12-five-layer-taxonomy.md` line 221 says "roko-core, roko-bus,
  roko-hdc, and roko-spi **are** the only kernel-tier crates" (present tense).
- `docs/00-architecture/INDEX.md` line 99 describes the five-layer taxonomy as including
  "target dep graph boundaries for roko-bus, roko-hdc, roko-spi" -- this is qualified, but the
  table entry for the five-layer doc is not.

### Issue C: Stale implementation status (MEDIUM)

The architecture INDEX `Current Status and Implementation Gaps` section (lines 189-218) was
generated on 2026-04-11 and never updated. Two items are factually wrong as of 2026-04-17:

1. `roko-serve: HTTP API not wired` -- **WRONG.** CLAUDE.md marks this as Wired with ~85
   routes.
2. `TUI: Text-mode dashboard only, no interactive terminal UI` -- **WRONG.** CLAUDE.md marks
   this as Wired with ratatui F1-F7 tabs.

The refinements runner focused on propagating new architecture concepts but did not reconcile
the status section against the actual codebase state.

---

## 8. Copy-Paste Artifacts Check

### Duplicate sections

No exact duplicate sections were found across the sampled docs. The refinements runner did a
good job of propagating concepts without literal copy-paste. Each doc adapts the refinement
content to its own context.

### Inconsistent formatting

The only formatting inconsistency is in the INDEX.md files. The top-level `docs/INDEX.md` has
the accretive "Current Framing" block, and `docs/12-interfaces/INDEX.md` has the accretive
overview paragraph. All other INDEXes are clean.

### Context-free paragraphs

None found. Every added paragraph connects to its surrounding doc.

### Over-long docs

The learning INDEX (`docs/05-learning/INDEX.md`) is 400 lines, which is long but justified by
the Rust code blocks in the cross-cutting concerns section. The architecture INDEX is 246
lines, also reasonable given its role.

---

## 9. Dimension Scores

| Dimension | Score | Key observations |
|---|---|---|
| **Consistency** | 4/5 | New docs use canonical terms; pre-existing docs still use `Signal` |
| **Coherence** | 4/5 | Refinement content reads as one voice in individual docs; INDEX accretion is the main weakness |
| **Accuracy** | 3/5 | Stale status section, target crates described in present tense, code snippets with pre-rename types |
| **Completeness** | 4/5 | All 35 refinements are represented; some docs lack future-work disclaimers on unbuilt features |
| **Terminology hygiene** | 3.5/5 | Glossary is excellent; but ~40 `Signal` references remain in non-retired contexts across 8+ files |
| **Internal links** | 5/5 | All 15+ sampled cross-references resolve to existing files |
| **Copy-paste artifacts** | 5/5 | No duplicates, no context-free fragments, no pasted-in blocks |

**Overall: 3.8 / 5**

---

## 10. Recommended Actions

### P0 (fix before using docs as implementation guide)

1. **Update architecture INDEX status section** (`docs/00-architecture/INDEX.md` lines
   204-218) to reflect that `roko-serve` and TUI are wired. Reconcile against CLAUDE.md.

2. **Sweep `Signal` from pre-existing docs** that were not touched by the refinements runner.
   At minimum: `docs/07-conductor/`, `docs/12-interfaces/02-roko-new-scaffolders.md`,
   `docs/01-orchestration/00-layer-overview.md`, `docs/11-safety/15-forensic-ai.md`, and
   `docs/CLI-REFERENCE.md`.

### P1 (fix before external review)

3. **Qualify target crates in `12-five-layer-taxonomy.md`**: change "are the only kernel-tier
   crates" to "are the target kernel-tier crates" on line 221.

4. **Collapse INDEX.md accretive citations**: rewrite the "Current Framing" block in
   `docs/INDEX.md` (lines 171-214) as a structured table or bulleted list instead of a
   growing paragraph.

5. **Fix `12-interfaces/INDEX.md` overview**: break the 1,500-character sentence into a
   paragraph with clear structure.

6. **Add future-work disclaimer to Rust code blocks** in `docs/05-learning/INDEX.md` cross-
   cutting concerns section (lines 250-360) for types like `DifficultyModel`,
   `CurriculumScheduler`, `ToolUsageProfile` that are not yet implemented.

### P2 (cleanup)

7. **Fix deployment INDEX cross-references** (`docs/19-deployment/INDEX.md` lines 55-60):
   replace vague "Agent Types documentation, section 8" with actual links.

8. **Add `20-ide-integration-strategy.md`** to the interfaces sub-documents table.

9. **Stabilize refinement links**: many docs link to `tmp/refinements/` for source proposals.
   These will rot. Consider adding a note that `tmp/refinements/` is frozen source material.
