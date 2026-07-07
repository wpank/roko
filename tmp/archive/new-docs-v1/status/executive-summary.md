# Executive Summary

> This is the `status/` folder scaffold for the Roko documentation tree.
> The executive summary lives at the repository root as a top-level document.

**Status**: Scaffold (stub — points forward)

---

## This File's Purpose

`status/executive-summary.md` is a navigation entry point, not a content page. The full
executive summary is maintained as a top-level document at:

```
docs/EXECUTIVE-SUMMARY.md
```

That document is the canonical one-page pitch for investors, technical executives, and
engineering leads. It is versioned alongside the codebase and updated with each major
milestone. Do not duplicate its content here.

---

## What `status/` Contains

The `status/` folder holds point-in-time snapshots of the project's implementation and
strategic state:

| File | Contents |
|---|---|
| [`vision.md`](vision.md) | The pitch document: scaffold thesis, empirical evidence, Synapse Architecture intro, design principles overview, self-improvement loops |
| `executive-summary.md` | ← you are here (stub pointing to `docs/EXECUTIVE-SUMMARY.md`) |
| `benchmarks.md` | Benchmark results and methodology (to be populated) |
| `status.md` | Master implementation status matrix — what is Shipping / Built / Scaffold / Specified / Deferred |

---

## Where to Find the Executive Summary

The canonical document is [`docs/EXECUTIVE-SUMMARY.md`](../../docs/EXECUTIVE-SUMMARY.md)
(relative to the repository root).

For investors and technical buyers, the recommended reading path is:

1. [`docs/EXECUTIVE-SUMMARY.md`](../../docs/EXECUTIVE-SUMMARY.md) — the pitch
2. [`status/vision.md`](vision.md) — the thesis with full empirical evidence
3. [`status/benchmarks.md`](benchmarks.md) — quantified results
4. [`reference/11-crate-map.md`](../reference/11-crate-map.md) — what is actually built

---

## Open Questions

- Should a condensed version of the executive summary live here so `status/` is
  self-contained for readers who enter via this path? (Deferred — avoid duplication
  until the top-level doc stabilizes.)
