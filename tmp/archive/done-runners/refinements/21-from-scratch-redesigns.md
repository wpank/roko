# From-Scratch Redesigns

> **TL;DR**: Some of the refinements in this folder can be
> retrofitted. Others benefit from *removing* existing code and
> designing the replacement clean. This doc lists five candidate
> from-scratch rewrites, for each: what to remove, what to replace
> it with, what it unlocks, what it costs, and whether it's worth
> it. This is the "what would you do with an afternoon to break
> things" list — used sparingly, but occasionally a clean rewrite
> saves six months of incremental patching.

> **For first-time readers**: The refactoring plan in 06 is
> incremental — no subsystem breaks until Phase C, and even then via
> compiler-assisted signature updates. This doc is the contrasting
> "what would we rewrite clean?" list. Five candidates, each scored
> against a heuristic for when from-scratch beats incremental.
> Read 06 first; this is the alternative sequencing where worth it.

## 1. When a from-scratch is justified

Heuristic: rewrite from scratch when at least three of these are
true:

1. The current design embeds an *assumption* you want to invert
   (e.g., "one noun" when you want "two mediums").
2. The code is <10K lines — too small to warrant preservation
   costs.
3. The interface surface is small or can be held stable.
4. The new design unlocks a capability that incremental refactoring
   cannot reach cleanly.
5. The old design has few direct users and no plugin contracts.

Otherwise: incremental refactor wins. Rewriting is not a virtue;
it is a tool.

## 2. Candidates

### 2.1 `roko-core` kernel

**Current**: 1 noun (Signal/Engram), 6 traits, stable shape.
**Replace with**: 2 mediums (Engram + Pulse), 7 operators
(Substrate, Bus, Scorer, Gate, Router, Composer, Policy), per `02`,
`03`, `04`.
**Unlocks**: Bus as first-class, unified operator trait over either
medium, coherent kernel.
**Costs**: 2–3 weeks of careful work. ~15 consumer crates need
import updates. Public API break requires semver-major bump.
**Worth it?**: Yes — the current framing actively misrepresents
the system. Every week the current kernel persists, more code
takes an implicit dependency on "Engram only" that later has to
be undone.

### 2.2 `roko-learn` reorganization

**Current**: episodes, playbooks, bandits, experiments, efficiency,
cascade router, all in one crate with mixed concerns.
**Replace with**: five focused crates:
- `roko-episode` (record, store, query)
- `roko-playbook` (distillation, retrieval)
- `roko-bandit` (arm management, Thompson/UCB)
- `roko-experiment` (A/B, experiment store)
- `roko-heuristic` (new — see `14`)

Plus *learning-as-subscription*: each crate subscribes to Bus
topics and reacts. No crate is called explicitly; they self-wire.

**Unlocks**: cleaner dep graph (see `20`), ability to replace any
one learning strategy without touching others, easier plugin
contributions.
**Costs**: significant churn — ~3K lines rearranged across
five crates. Two weeks of work. No public API break if the CLI
retains its current shape.
**Worth it?**: Yes, but after the kernel rewrite. Doing it before
is wasted work if kernel types change.

### 2.3 Substrate trait rewrite

**Current**: `Substrate::get/put/delete/list`.
**Replace with**: `Substrate::put/get/query/scan/freeze/thaw`, with:
- `query(predicate)` for HDC-similarity and filter-based queries
- `scan(range)` for lineage walks
- `freeze(hash)` + `thaw(hash)` for demurrage-driven cold tier
- subscribe-style notifications via Bus

**Unlocks**: uniform content-addressed + similarity query, cold
tier (see `12`), replicable substrate semantics across storage
backends.
**Costs**: 1 week. Affects every call-site. Public API break for
the Substrate trait.
**Worth it?**: Yes — the current surface is too minimal for the
memory primitives we want. Forcing consumers to build their own
query layers produces divergent implementations.

### 2.4 Gate pipeline

**Current**: 11 gates in a 7-rung pipeline with adaptive
thresholds. Implementation is a roll-your-own state machine.
**Replace with**: gates as pure functions + composition combinators
(sequence, parallel, any, all, retry, budget). Thresholds are a
property of the *composition*, not the gate.

**Unlocks**: third-party gates as simple Rust functions or WASM
modules, testability (gates are pure), visualizability (the
composition is inspectable data).
**Costs**: 1–2 weeks. Existing gates need lightly adapting. Gate
results and reason strings need normalization.
**Worth it?**: Maybe. Gates are already working; this is cleaner
but not unlocking a specific user-facing capability. Schedule it
for a slow month.

### 2.5 `roko-compose` engine

**Current**: 6-layer prompt builder with roles loaded from
template files, enrichment hooks.
**Replace with**: a *query-driven* compose pipeline — the system
prompt is not built from a fixed template, it's assembled from
whichever Engrams match a query (role, situation fingerprint,
relevant heuristics, recent episodes). Template is a query, not a
string.

**Unlocks**: dramatically more dynamic system prompts, plug-in
templates become data queries, per-situation prompt specialization
via HDC retrieval.
**Costs**: 2 weeks. Old template files become fallback defaults.
Consumers that depend on deterministic prompts need a stable-mode
flag.
**Worth it?**: Yes, long-term — this is where the HDC substrate
pays off dramatically. Short-term the existing engine is fine.
Schedule after the substrate rewrite (2.3).

## 3. The "no rewrite" list

Things that are tempting to rewrite but shouldn't be:

- **roko-fs**: stable, JSONL-on-disk, well-scoped. A Postgres
  backend is a new crate, not a rewrite.
- **roko-cli**: churn all we like; no rewrite. The subcommand
  structure is earning its keep.
- **roko-runtime**: except for the bus extraction in 2.1, leave
  alone. Supervisor + cancellation semantics are hard-won.
- **roko-agent dispatcher**: the backend fan-out is working and
  extensible; no need.
- **roko-orchestrator**: DAG + parallelism + merge queue is
  complex and right; extending, not rewriting.

## 4. The meta-rewrite: the docs

Docs aren't code but they benefit from the same logic. Two rewrites
already queued:

### 4.1 `docs/00-architecture/` chapter 1

Rewrite the foundational story to reflect "two mediums, six
operators." This is the payoff of this whole folder. Three to five
refreshed pages replacing the current one.

### 4.2 CLAUDE.md and README

Update the "1 noun + 6 verbs" line. It's on the literal front page
of the project. Every day it stays there, the wrong mental model
propagates. Rewrite after the kernel rewrite lands.

## 5. Sequencing

Optimal order:

```
1. roko-bus extraction              (week 1)  [from 20]
2. Kernel rewrite (2.1)             (weeks 2–4)
3. Substrate rewrite (2.3)          (weeks 4–5)
4. Docs rewrite (4.1, 4.2)          (week 5)
5. roko-learn reorganization (2.2)  (weeks 6–7)
6. Compose rewrite (2.5)            (weeks 8–9)
7. Gate rewrite (2.4) — if desired  (week 10)
```

Two months of focused rewrite work. At the end, Roko has a
substantially cleaner kernel and the rest of the refinements in
this folder can land on top of it cleanly.

## 6. Risk management

Rewriting anything in a production codebase is a risk. Mitigations:

### 6.1 Feature-flag the new kernel

Build both old and new types for one release cycle. `--kernel=v2`
flag exercises the new path. Gives time to shake out bugs without
a hard cutover.

### 6.2 Compatibility shim for Engram format

Old Engrams on disk should load into the new kernel. A one-shot
migration reader, not a dual-world permanent compat layer.

### 6.3 Extensive test parity

Before rewriting, record current test outputs. After rewriting,
outputs must match or the difference must be explicitly justified.
Test-driven refactoring at the crate level.

### 6.4 Sequenced landing

Never merge two rewrites in the same week. Land one, bake for days,
then the next. Rushing this is how multi-month regressions happen.

## 7. What we gain by committing

A version of Roko with the refinements in this folder — two
fabrics, HDC-everywhere, demurrage, heuristics, c-factor,
replication ledger, plugin tiers — is substantially different from
the current Roko. It is worth thinking about this as a *second
version of Roko*, not a refactoring. The kernel rewrite is the
moment that distinction is acknowledged.

That framing also helps planning: the current codebase is a
successful v1. The refinements are the design of v2. We can
deliberately choose which v1 behaviors carry forward and which
are replaced.

## 8. What we risk by not committing

If we don't do the rewrites, each refinement lands as a patch on
top of assumptions it contradicts. Demurrage clashes with the
current decay field. HDC clashes with the current content-address
query. Heuristics clash with the current playbook store. Each
patch works; together they form a Frankenstein. A Frankenstein
system is shippable but loses the architectural-coherence moat
described in `18`.

The worst outcome is a Roko that *has* these features but *is* a
patch quilt. We'd lose the defensibility claim while still paying
the complexity cost. Better to commit to the kernel rewrite or
commit to not doing the refinements, rather than straddling.

## 9. Recommendation

Commit. Do the rewrites in the order above. Two months of focused
work, then the refinements in docs 2–19 land on the new foundation
cleanly and accrue the moat. The alternative — patching
indefinitely — produces a system that's harder to explain, harder
to extend, harder to defend.

The sunk cost of existing code should not overweight the
opportunity cost of the superior design. Especially when the
existing code is 18 crates and 177K lines — a scale where
rewriting the kernel *is* feasible, unlike at 10x scale where it
would not be.

## 10. The "not a rewrite" in each candidate

Even the most aggressive of the five candidates (2.1, kernel rewrite)
keeps a lot intact. For the record, the things that *don't* change in
each:

- **Kernel rewrite (2.1)** — Engram struct and its persistence format.
  BLAKE3 content hashing. The 7-axis Score. The Decay enum. Provenance
  and attestation shapes. The existing 131 trait impls mostly compile
  unchanged; signatures widen, bodies stay.
- **Learn reorganization (2.2)** — bandit algorithms, cascade-router
  logic, experiment store math. The crate-reshaping is mechanical.
- **Substrate rewrite (2.3)** — BLAKE3 addressing, JSONL format on
  disk, lineage field, scoring pipeline. The `query_similar` adds; the
  existing `query` stays.
- **Gate rewrite (2.4)** — existing gate implementations (compile,
  test, clippy, diff). The state-machine/composition change is about
  orchestration, not the gates themselves.
- **Compose rewrite (2.5)** — layer types, template content,
  enrichment hooks. The engine changes; the templates as data stay.

The rewrites are structural, not content-level. That's what makes
them tractable.

## 11. Risk ranking of the five

Explicitly scoring risk so sequencing reflects it:

| # | Candidate | Risk of breakage | Mitigating factor |
|---|---|---|---|
| 2.1 | Kernel rewrite | High | Feature-flag path per 6.1; 2-week bake |
| 2.2 | Learn reorganization | Low | Mostly crate-reshaping |
| 2.3 | Substrate rewrite | Medium | JSONL format preserved |
| 2.4 | Gate rewrite | Medium-high | Changes result shapes |
| 2.5 | Compose rewrite | High | Prompt determinism suffers |

The §5 sequencing respects this: kernel (2.1) rewrite first while
the team has the most energy, gate (2.4) scheduled last because it's
in the "maybe" column anyway. 2.3 lands right after the kernel
because substrate consumers are also the top-affected by the kernel
change, so the churn overlaps.

## 12. Red flags during a rewrite

Rewrites fail in characteristic ways. If any of these appear, stop
and reassess:

- **Week-three decision paralysis.** Two months in, the team is
  still arguing about naming. Means the design isn't fixed; go back
  to the doc before coding.
- **"We'll clean this up later" commits.** Rewrite with shortcut
  becomes incremental with scaffolding. The original code is better
  than two layers of half-built.
- **Silent test regressions.** A rewrite that doesn't pass the
  recorded test parity from 6.3 is not done. Don't ship it.
- **Users reporting behavior changes the team didn't intend.** The
  test parity missed something. Investigate, don't handwave.

Each is a signal the rewrite is off-track. Catching early costs
days; catching late costs months.

## 13. Rewrite governance

For each from-scratch rewrite, the team needs:

- A single named **owner** accountable for landing.
- A **written before/after contract** listing every trait or type
  that changes, with migration notes.
- A **rollback plan** — feature flag, fork branch, or reversible
  commit sequence.
- A **bake period** before removing the old path (minimum 1 week
  for 2.2–2.4; 2 weeks for 2.1 and 2.5).
- **Two sign-offs** before merging the kernel rewrite — one from
  the kernel owner, one from the consumer most affected.

This feels heavyweight until the first bad rewrite wrecks a week of
shipping. Then it feels cheap.

## 14. Cross-references

- Incremental alternative is `06-refactoring-plan.md`.
- Each rewrite candidate points at the home doc that justifies it:
  2.1 (→ 02, 03, 04), 2.2 (→ 10, 14), 2.3 (→ 11, 12), 2.4 (→ 17 §5),
  2.5 (→ 11, 14).
- The dep-graph changes the rewrites enable are in `20-modularity-composability.md`.
- The consolidated timeline that merges the rewrite plan with the
  incremental plan and the UX work is in `35-consolidated-roadmap.md`.
