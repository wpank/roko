# SOURCE-INDEX — Core Anchors For PU06

This source index is intentionally narrow. It tracks the code that matters for
the post-audit neuro parity story.

## Kernel And HDC

| File | Why it matters |
|------|----------------|
| `crates/roko-core/src/engram.rs:38-65` | `Engram` exists and currently has no HDC fingerprint field |
| `crates/roko-core/src/traits.rs:34-63` | `Substrate` exists with `put/get/query/prune`; no `query_similar()` |
| `crates/roko-primitives/src/hdc.rs:24-255` | shipping `HdcVector` implementation |

## Neuro Runtime

| File | Why it matters |
|------|----------------|
| `crates/roko-neuro/src/lib.rs:69-150` | knowledge kinds and tier multipliers |
| `crates/roko-neuro/src/lib.rs:214-260` | `KnowledgeEntry` surface |
| `crates/roko-neuro/src/knowledge_store.rs:23-117` | knowledge store constants, confirmation record, and store setup |
| `crates/roko-neuro/src/knowledge_store.rs:167-239` | ingest path |
| `crates/roko-neuro/src/distiller.rs:29-94` | `DistillationBackend` and `Distiller` |
| `crates/roko-neuro/src/tier_progression.rs:24-258` | thresholds, `TierProgression`, and `TierProgressionDecision` |
| `crates/roko-neuro/src/context.rs:221-287` | `ContextAssembler` core gather pipeline |

## CLI And Event Reality

| File | Why it matters |
|------|----------------|
| `crates/roko-cli/src/main.rs:569-591` | `NeuroCmd` is `Query`, `Stats`, `Gc` only |
| `crates/roko-runtime/src/event_bus.rs:101-129` | event bus carries exactly `PlanRevision` and `PrdPublished` |

## Absences That Must Stay Labeled As Absent

Run these before claiming a feature ships:

```bash
rg -n "query_similar" crates/roko-core crates/roko-fs crates/roko-neuro
rg -n "Resonance|TransferRisk|DomainProfile|AnalogyResult" crates/
rg -n "demurrage|Worldview|Library of Babel|KoraiChannel|LetheChannel" crates/
rg -n "Backup|Restore|Publish" crates/roko-cli/src/main.rs crates/roko-neuro crates/roko-fs
```

Current expected outcome:

- no `query_similar()` on `Substrate`
- no production cross-domain transfer API
- no demurrage or worldview implementation
- no neuro backup / restore / publish CLI
