# Refinement Batches

35 batches. One batch per refinement in `tmp/refinements/`. Each batch
propagates one refinement proposal into the canonical `docs/` tree.
Docs-only — no code touched.

## Dependency graph

```
foundation:
  REF01 (critique) ─┐
  REF02 (Pulse) ────┼─► REF03 (Bus) ─┐
                    │                 ├─► REF04 (operators) ──► REF05 (loop)
                    │                 │                         │
                    │                 │                         ├─► REF08 (code sketches)
                    │                 │                         │
                    │                 ├──────────────────► REF07 (naming)
                    │                 │
                    │                 └─► REF09 (phase-2)
                    │
                    └─► REF06 (refactor plan)  — independent

learning:
  REF10 (self-learning) ◀─ REF03
  REF11 (HDC) ◀─ REF02
  REF12 (demurrage) ◀─ REF02, REF11
  REF13 (c-factor) ◀─ REF10, REF11
  REF14 (heuristics) ◀─ REF12
  REF15 (scaling) ◀─ REF12, REF14
  REF16 (research) ◀─ REF14

moat:
  REF17 (plugin SPI) ◀─ REF03
  REF18 (moat) ◀─ REF15, REF17
  REF19 (innovations) ◀─ REF11, REF12, REF13, REF14
  REF20 (modularity) ◀─ REF03, REF11
  REF21 (rewrites) ◀─ REF06

ux-core:
  REF22 (dev UX) ◀─ REF04, REF20
  REF23 (user UX) ◀─ REF04
  REF24 (deployment) ◀─ REF03
  REF25 (domains) ◀─ REF17, REF23

ux-surface:
  REF26 (StateHub) ◀─ REF03, REF23
  REF27 (realtime) ◀─ REF26
  REF28 (CLI parity) ◀─ REF23
  REF29 (web UI) ◀─ REF26, REF27
  REF30 (primitives) ◀─ REF26, REF23

integrator:
  REF31 (synergy) ◀─ REF11, REF12, REF13, REF14, REF17
  REF32 (safety) ◀─ REF17, REF25
  REF33 (observability) ◀─ REF26, REF27, REF24
  REF34 (glossary) ◀─ REF07
  REF35 (roadmap) ◀─ REF31
```

## Serial execution order (ALL_BATCHES in lib/common.sh)

```
REF01 REF02 REF03 REF04 REF05 REF06 REF07 REF08 REF09       # foundation
REF10 REF11 REF12 REF13 REF14 REF15 REF16                    # learning
REF17 REF18 REF19 REF20 REF21                                # moat
REF22 REF23 REF24 REF25                                      # ux-core
REF26 REF27 REF28 REF29 REF30                                # ux-surface
REF31 REF32 REF33 REF34 REF35                                # integrator
```

## Batch manifest

| Batch | Title | Group | Deps | Target subdirs | Required vocab |
|-------|-------|-------|------|----------------|----------------|
| REF01 | Critique one-noun | foundation | — | `docs/00-architecture` | two mediums / two fabrics / six operators |
| REF02 | Introduce Pulse | foundation | REF01 | `docs/00-architecture` | Pulse |
| REF03 | Promote Bus | foundation | REF02 | `docs/00-architecture` | Bus trait |
| REF04 | Generalize operators | foundation | REF02 REF03 | `docs/00-architecture` | Datum, two mediums |
| REF05 | Loop retold | foundation | REF04 | `docs/00-architecture`, `docs/16-heartbeat` | seven-step, SENSE, BROADCAST |
| REF06 | Refactor plan | foundation | — | `docs/00-architecture` | Phase A/B/C |
| REF07 | Naming | foundation | REF02 REF03 | `docs/00-architecture` | Pulse, Topic, Datum, Bus |
| REF08 | Code sketches | foundation | REF04 | `docs/00-architecture` | ` ```rust ` snippets |
| REF09 | Phase-2 implications | foundation | REF03 | `docs/08-chain`, `docs/10-dreams`, `docs/13-coordination`, `docs/16-heartbeat` | ChainBus, two-fabric |
| REF10 | Self-learning loops | learning | REF03 | `docs/05-learning`, `docs/00-architecture`, `docs/16-heartbeat` | prediction error, active inference |
| REF11 | HDC substrate | learning | REF02 | `docs/06-neuro`, `docs/00-architecture` | HDC, fingerprint |
| REF12 | Demurrage | learning | REF02 REF11 | `docs/00-architecture`, `docs/06-neuro`, `docs/05-learning` | demurrage, balance |
| REF13 | c-factor | learning | REF10 REF11 | `docs/00-architecture`, `docs/13-coordination` | c-factor |
| REF14 | Worldview validation | learning | REF12 | `docs/05-learning`, `docs/06-neuro` | heuristic, falsifier, worldview |
| REF15 | Exponential scaling | learning | REF12 REF14 | `docs/00-architecture`, `docs/20-technical-analysis`, `docs/13-coordination` | compounding, superlinear |
| REF16 | Research-to-runtime | learning | REF14 | `docs/21-references`, `docs/05-learning` | replication ledger, claim |
| REF17 | Plugin SPI | moat | REF03 | `docs/18-tools`, `docs/12-interfaces` | plugin, five-tier, SPI |
| REF18 | Moat | moat | REF15 REF17 | `docs/20-technical-analysis`, `docs/00-architecture` | moat |
| REF19 | Innovations catalog | moat | REF11 REF12 REF13 REF14 | `docs/00-architecture`, `docs/20-technical-analysis` | net-new, primitive |
| REF20 | Modularity | moat | REF03 REF11 | `docs/00-architecture` | roko-bus, roko-hdc, roko-spi |
| REF21 | Rewrites | moat | REF06 | `docs/00-architecture` | rewrite, from-scratch |
| REF22 | Dev UX | ux-core | REF04 REF20 | `docs/12-interfaces`, `docs/02-agents` | one-liner, builder, trait, runtime |
| REF23 | User UX | ux-core | REF04 | `docs/12-interfaces` | verb set, four surfaces |
| REF24 | Deployment | ux-core | REF03 | `docs/19-deployment` | laptop, single-server, container, clustered, edge |
| REF25 | Domains | ux-core | REF17 REF23 | `docs/02-agents`, `docs/12-interfaces`, `docs/18-tools` | domain profile, TypedContext, Custody |
| REF26 | StateHub | ux-surface | REF03 REF23 | `docs/12-interfaces` | StateHub, projection |
| REF27 | Realtime | ux-surface | REF26 | `docs/12-interfaces`, `docs/19-deployment` | WebSocket, SSE, channel |
| REF28 | CLI parity | ux-surface | REF23 | `docs/12-interfaces` | slash command, diff-first |
| REF29 | Web UI | ux-surface | REF26 REF27 | `docs/12-interfaces` | Home, Chat, Plans, Beliefs |
| REF30 | Rich UX primitives | ux-surface | REF26 REF23 | `docs/12-interfaces` | footnote, reasoning stream, uncertainty |
| REF31 | Synergy map | integrator | REF11 REF12 REF13 REF14 REF17 | `docs/00-architecture` | synergy, matrix |
| REF32 | Safety spine | integrator | REF17 REF25 | `docs/11-safety`, `docs/00-architecture` | custody, sandbox, attestation |
| REF33 | Observability | integrator | REF26 REF27 REF24 | `docs/19-deployment`, `docs/00-architecture` | telemetry, observability |
| REF34 | Glossary | integrator | REF07 | `docs/00-architecture` | glossary, retired |
| REF35 | Roadmap | integrator | REF31 | `docs/00-architecture` | roadmap, sequencing |

## Verification gates (summary)

Every batch runs the same four-gate sequence; no cargo involved:

1. **Scope gate** — `git status -- docs/` must be the only changes. Any
   file modified outside `docs/` fails the batch.
2. **Diff gate** — at least one file under `docs/` must have changed.
3. **Terminology gate** — retired terms absent from changed lines
   (outside explicitly retired contexts). Patterns in `RETIRED_TERMS`
   in `lib/common.sh`.
4. **Required-term gate** — the refinement's required vocabulary (above)
   appears in at least one changed file.
5. **Internal link gate** — soft warning; set
   `REF_LINK_CHECK_STRICT=1` to make it blocking.

## Conflict groups

Batches in the same write-scope group should not run in parallel
against the same worktree; the runner enforces serial execution anyway,
but if you hand-edit `--only`, respect these clusters:

- **foundation**: REF01–REF08 all touch `docs/00-architecture/`; REF09
  splits into the Phase-2 subsystem folders.
- **learning**: REF10–REF16 cluster in `docs/05-learning/`,
  `docs/06-neuro/`, `docs/13-coordination/`, `docs/21-references/`.
- **moat**: REF17–REF21 cluster in `docs/00-architecture/`,
  `docs/20-technical-analysis/`, `docs/18-tools/`.
- **ux-core**: REF22–REF25 cluster in `docs/12-interfaces/` and
  `docs/02-agents/`.
- **ux-surface**: REF26–REF30 all heavy in `docs/12-interfaces/`.
- **integrator**: REF31–REF35 re-touch `docs/00-architecture/INDEX.md`
  and `docs/INDEX.md` repeatedly.

## Tuning knobs

Per-batch overrides live in `lib/common.sh`:

- `batch_title` — human-readable label for logs and commits.
- `batch_refinement_file` — path to the canonical refinement source.
- `batch_target_docs` — candidate doc paths (advisory).
- `batch_deps` — DAG edges the runner enforces.
- `batch_group` — group membership for `--group` filtering.
- `batch_required_terms` — regex that at least one changed file must
  match after a batch.

Global retired-terms list lives in `RETIRED_TERMS` in `lib/common.sh`.
Add entries there to strengthen the terminology check globally.
