# Concern Plan: architecture-contract

**Tasks:** 12
**Primary gate:** `roko parity check --strict`

## Execution Contract

- Use this file to batch related work, but update the checkbox in the source-specific plan file when implementation completes.
- Do not implement a cross-cutting concern in isolation without checking every linked source task for adjacent storage, auth, realtime, dashboard, and verification requirements.
- A task is done only when its source-specific acceptance criteria and this concern gate pass.
- If implementing one task reveals a shared abstraction, update all affected task rows and avoid parallel duplicate abstractions.

## Shared Checklist

- [ ] Every source requirement has a ledger row.
- [ ] Contradictions are resolved with newer architecture winning runtime shape and old docs mapped or deferred.
- [ ] Strict parity check passes for the source area.

## Task Routing Table

| Task | Source | Plan | Heading | Score | Status |
|------|--------|------|---------|-------|--------|
| ARCH-03-S005 | `tmp/architecture/03-extensions.md:164` | [arch-03-extensions.md](arch-03-extensions.md) | Extension dependency resolution | 9.6 | [ ] |
| ARCH-03-S013 | `tmp/architecture/03-extensions.md:265` | [arch-03-extensions.md](arch-03-extensions.md) | Spec clarifications (added 2026-04-25) | 9.6 | [ ] |
| ARCH-06-S005 | `tmp/architecture/06-paid-feeds.md:135` | [arch-06-paid-feeds.md](arch-06-paid-feeds.md) | For pricier feeds: | 9.6 | [ ] |
| ARCH-10-S007 | `tmp/architecture/10-groups.md:141` | [arch-10-groups.md](arch-10-groups.md) | Membership protocol | 9.6 | [ ] |
| ARCH-10-S012 | `tmp/architecture/10-groups.md:269` | [arch-10-groups.md](arch-10-groups.md) | Coordination modes | 9.6 | [ ] |
| ARCH-12-S033 | `tmp/architecture/12-defi.md:824` | [arch-12-defi.md](arch-12-defi.md) | Integration with existing systems | 9.6 | [ ] |
| ARCH-13-S010 | `tmp/architecture/13-meta.md:227` | [arch-13-meta.md](arch-13-meta.md) | Lineage tracking | 9.6 | [ ] |
| ARCH-14-S026 | `tmp/architecture/14-registries.md:816` | [arch-14-registries.md](arch-14-registries.md) | Integration with existing systems | 9.6 | [ ] |
| ARCH-18-S002 | `tmp/architecture/18-roadmap.md:8` | [arch-18-roadmap.md](arch-18-roadmap.md) | Implementation path | 9.6 | [ ] |
| ARCH-18-S011 | `tmp/architecture/18-roadmap.md:138` | [arch-18-roadmap.md](arch-18-roadmap.md) | Bardo source references | 9.6 | [ ] |
| ARCH-20-S002 | `tmp/architecture/20-orchestrator-gaps.md:8` | [arch-20-orchestrator-gaps.md](arch-20-orchestrator-gaps.md) | Orchestrator gaps (from mori) | 9.6 | [ ] |
| ARCH-20-S023 | `tmp/architecture/20-orchestrator-gaps.md:451` | [arch-20-orchestrator-gaps.md](arch-20-orchestrator-gaps.md) | Gap 3: Error deduplication algorithm | 9.6 | [ ] |

## Self-Assessment

- Detail score: **9.6/10**
- Rationale: this concern file gives cross-source routing, a shared concern-specific checklist, source task links, and the concern gate. Source-specific files provide the detailed implementation packets.
