# Migration Log — Cluster D: Vision & Vocab

> Audit trail for the Cluster D refactor. Records what moved where, what was split,
> what was combined, and what was noticed as missing.

**Cluster**: D — Vision, Glossary, Design Principles, Crate Map
**Date**: 2026-04-19
**Subagent session**: Cluster D
**Source files**: 4 (listed below)
**Target files written**: 10

---

## Source → Target Mapping

### Source 1: `docs/00-architecture/00-vision-and-thesis.md` (500 lines, 26.6 KB)

| Content | Destination | Notes |
|---|---|---|
| Abstract + scaffold thesis + empirical evidence (SWE-bench, AlphaCode, Meta-Harness, FrugalGPT, DSPy, Compound AI) | [`status/vision.md`](../status/vision.md) §1 + §2 | Kept verbatim structure; expanded rationale for each evidence point |
| Synapse Architecture introduction (two mediums, two fabrics, six operators) | [`status/vision.md`](../status/vision.md) §3 | Expanded with tables; split into §3.1–3.7 |
| Universal cognitive loop | [`status/vision.md`](../status/vision.md) §3.4 | Kept; forward link to [`reference/06-loop/`](../reference/06-loop/) |
| Three cognitive speeds | [`status/vision.md`](../status/vision.md) §3.5 | Summary table; full doc at [`reference/07-speeds/`](../reference/07-speeds/) |
| Five dependency layers | [`status/vision.md`](../status/vision.md) §3.6 | Summary table; full map at [`reference/11-crate-map.md`](../reference/11-crate-map.md) |
| Three cross-cuts (Neuro, Daimon, Dreams) | [`status/vision.md`](../status/vision.md) §3.7 | Summary; full docs at [`reference/09-cross-cuts/`](../reference/09-cross-cuts/) |
| Design principles overview | [`status/vision.md`](../status/vision.md) §4 | Overview only; full principles at [`reference/12-design-principles.md`](../reference/12-design-principles.md) |
| Self-improvement loops | [`status/vision.md`](../status/vision.md) §5 | Expanded; four loops described (heuristic, consolidation, self-hosting, collective) |
| Implementation status (as of 2026-04-17) | [`status/vision.md`](../status/vision.md) §6 | Pulled from STATUS.md; tagged (as of 2026-04-17 per STATUS.md) |
| Frontier research references (SWE-bench, AlphaCode, etc.) | [`research/frontier-summary.md`](../research/frontier-summary.md) | Research narrative expanded significantly |
| Design principles (detailed) | [`reference/12-design-principles.md`](../reference/12-design-principles.md) | Extracted to standalone doc |

**Coverage**: The source file was 26.6 KB; I had access to the first 5.3 KB (20%) from the device
tool cache. The remaining 80% was reconstructed from:
- The architecture INDEX.md (which summarizes section 00)
- The EXECUTIVE-SUMMARY.md (which cites specific design decisions)
- The STATUS.md (which provides implementation status)
- The cluster plan (which describes the refactor target)
Content marked as "extrapolated from context" is indicated in the Open Questions below.

---

### Source 2: `docs/00-architecture/01-naming-and-glossary.md` (706 lines, 38.7 KB)

| Content | Destination | Notes |
|---|---|---|
| Current naming map table | [`GLOSSARY.md`](../GLOSSARY.md) + [`ALIASES.md`](../ALIASES.md) | Naming map → aliases; term-by-term detail → GLOSSARY |
| Conventions paragraph | Dropped (per spec) | Conventions already in CONVENTIONS.md; not duplicated |
| History/narrative of naming changes | [`strategy/refinements/naming-history.md`](../strategy/refinements/naming-history.md) | Extracted and expanded into full narrative |
| A-Z term definitions | [`GLOSSARY.md`](../GLOSSARY.md) | Mechanical flat table; one line per term |
| "Public alias" entries | [`ALIASES.md`](../ALIASES.md) | Extracted to dedicated flat table |
| Refinements references (`tmp/refinements/...`) | Updated to relative links pointing to [`strategy/refinements/`](../strategy/refinements/) | Absolute paths removed per CONVENTIONS §4 |

**Coverage**: I had access to the first 5.3 KB (14%) from the device tool cache — through the
start of the A section (through "ACT"). The remaining 86% of the A-Z glossary was
reconstructed from:
- The current naming map table (captured in full in the first 5.3 KB)
- The EXECUTIVE-SUMMARY.md section summaries (which name all major types)
- The STATUS.md crate table (which names all crates and their types)
- The architecture INDEX.md (which names all concepts per section)

**Missing terms**: A number of terms from later in the A-Z that I could not reconstruct
with confidence are listed in the `## Terms Not Yet in the Glossary` section at the bottom
of GLOSSARY.md. A subsequent pass with full source access should fill these in.

---

### Source 3: `docs/00-architecture/15-crate-map.md` (360 lines, 18.6 KB)

| Content | Destination | Notes |
|---|---|---|
| Layer-by-layer crate table | [`reference/11-crate-map.md`](../reference/11-crate-map.md) §L0–L4 | Reconstructed from STATUS.md and EXECUTIVE-SUMMARY.md |
| Per-crate detail (status, responsibility, test count) | [`reference/11-crate-map.md`](../reference/11-crate-map.md) | All test counts tagged (as of 2026-04-17 per STATUS.md) |
| Dependency graph | [`reference/11-crate-map.md`](../reference/11-crate-map.md) §Dependency Graph | Simplified ASCII; full graph tool-verifiable from cargo |
| Workspace layout / operational view | Deferred to `operations/` (linked from crate map) | Operational layout is a separate concern from conceptual map |

**Coverage**: Source file was not in device cache. Reconstructed entirely from STATUS.md
(which provides test counts and status tiers) and EXECUTIVE-SUMMARY.md (which describes
each crate's responsibility). The source file likely contains LOC per crate, additional
dependency annotations, and more detailed cross-crate interaction notes that are not
represented in the migration output.

**Gaps flagged in output**: The crate map notes that per-crate LOC (other than roko-learn
at 35,847) is not available and requests a per-crate LOC audit.

---

### Source 4: `docs/00-architecture/17-design-principles-and-frontier-summary.md` (532 lines, 34.2 KB)

| Content | Destination | Notes |
|---|---|---|
| Design principles (§1–7) | [`reference/12-design-principles.md`](../reference/12-design-principles.md) | One principle per H2; rationale + examples for each |
| Frontier summary narrative | [`research/frontier-summary.md`](../research/frontier-summary.md) | Expanded with 7 research areas; each with Roko mapping and novel contributions |

**Coverage**: Source file was not in device cache. Reconstructed from:
- EXECUTIVE-SUMMARY.md (which cites specific design principles inline)
- STATUS.md (which enumerates shipping capabilities per-section)
- The architecture INDEX.md (which summarizes each section)
- General Roko architecture knowledge from other partially-read docs
- The vision doc's design principles overview paragraph

The source file's exact wording, ordering, and additional principles (if any beyond the
seven listed) are unknown. The reconstructed output may differ in emphasis or completeness
from the original.

---

## Additional Files Created

| File | Type | Notes |
|---|---|---|
| [`status/executive-summary.md`](../status/executive-summary.md) | Stub (per spec) | Points forward to `docs/EXECUTIVE-SUMMARY.md`; no content duplication |
| [`strategy/refinements/README.md`](../strategy/refinements/README.md) | Folder index | Index for promoted refinements; lists pending `tmp/refinements/` files |
| [`research/README.md`](../research/README.md) | Folder index | Index for research folder; cluster G creates most content |

---

## Terms Noticed as Missing from the Glossary

The following terms appear in documents I read but are not yet in GLOSSARY.md. They are
tracked in the "Terms Not Yet in the Glossary" section at the bottom of GLOSSARY.md.
A subsequent pass should add these with proper home-doc links.

| Term | Where noticed | Notes |
|---|---|---|
| `Experiment` | EXECUTIVE-SUMMARY.md §05 | Bandit experiment framework in `roko-learn` |
| `Episode` | EXECUTIVE-SUMMARY.md §05 | Execution record unit; `roko-learn` episode logger |
| `Playbook` | EXECUTIVE-SUMMARY.md §05 | Collection of extracted heuristic rules |
| `SkillLibrary` | EXECUTIVE-SUMMARY.md §05 | Reusable execution patterns |
| `PatternMiner` | EXECUTIVE-SUMMARY.md §05 | Pattern extraction subsystem |
| `CostTracker` | EXECUTIVE-SUMMARY.md §05 | Efficiency accounting |
| `SporePool` / `SparrowJob` | EXECUTIVE-SUMMARY.md §08 | Korai chain job marketplace |
| `Kauri BFT` | EXECUTIVE-SUMMARY.md §08 | Korai chain consensus algorithm |
| `ERC-8004` | EXECUTIVE-SUMMARY.md §08 | Agent passport standard |
| `KORAI` / `DAEJI` tokens | EXECUTIVE-SUMMARY.md §08 | Korai chain tokens |
| `SpecPool` | EXECUTIVE-SUMMARY.md §08 | EVM parallel execution |
| `UCB1` / `LinUCB` / `Track-and-Stop` | EXECUTIVE-SUMMARY.md §05 | Three bandit algorithms in `roko-learn` |
| `GateVerdict` | EXECUTIVE-SUMMARY.md §04 | Gate pipeline verdict type |
| `ParallelExecutor` | EXECUTIVE-SUMMARY.md §01 | Orchestrator's core state machine |
| `SystemPromptBuilder` | EXECUTIVE-SUMMARY.md §03 | 7-layer prompt assembly type |
| `CascadeRouter` | EXECUTIVE-SUMMARY.md §02 | Three-stage model selection router |
| `somatic marker hypothesis` | EXECUTIVE-SUMMARY.md §09 | Damasio 1994; basis for Daimon's fast pattern-matching |
| `ProcessSupervisor` | EXECUTIVE-SUMMARY.md | `roko-runtime` supervisor type |

---

## Coverage Assessment

| Source file | KB | Had access to | Coverage | Risk |
|---|---|---|---|---|
| 00-vision-and-thesis.md | 26.6 | 5.3 KB | 20% | **Medium** — first 20% is highest-density content; remaining 80% likely elaboration |
| 01-naming-and-glossary.md | 38.7 | 5.3 KB | 14% | **High** — only saw A through partial ACT; most A-Z glossary missing |
| 15-crate-map.md | 18.6 | 0 KB | 0% | **High** — fully reconstructed from secondary sources |
| 17-design-principles-and-frontier-summary.md | 34.2 | 0 KB | 0% | **High** — fully reconstructed from secondary sources |

**Recommended action**: A verification pass with full source access should:
1. Compare GLOSSARY.md against the full A-Z of 01-naming-and-glossary.md, adding missing terms
2. Compare reference/11-crate-map.md against 15-crate-map.md, adding any crates or
   dependency notes that are in the source but not in the output
3. Compare reference/12-design-principles.md against 17-design-principles-and-frontier-summary.md,
   verifying principle count and adding any principles that were not in the reconstruction
4. Fill in the "Terms Not Yet in the Glossary" section

---

## Conventions Compliance

- No absolute paths — all links are relative ✓
- Status tags on all substantive pages ✓
- Frontmatter on all content pages ✓
- "Last reviewed" date set on all pages ✓
- No `tmp/` paths in target docs — all `tmp/refinements/` links updated to
  `strategy/refinements/` ✓
- Stale LOC/test counts tagged with "(as of 2026-04-17 per STATUS.md)" ✓
- Retired terms in naming-history only; not in glossary prose ✓
- Open Questions section on all substantive pages ✓
