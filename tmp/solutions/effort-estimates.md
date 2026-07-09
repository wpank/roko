# Effort Estimates — Binary Issues (MASTER-INDEX.md)

**Date**: 2026-04-28
**Source**: `tmp/binary-issues/MASTER-INDEX.md`

## Per-Systemic-Problem

| # | Problem | Effort | Notes |
|---|---|---|---|
| S5 | Security-off-by-default | 2-3h | Config changes + middleware. Mechanical. |
| S2 | Throwaway HTTP clients | 2-3h | Shared client, thread through. Streaming path tricky. |
| S3 | Confirmation theater (easy) | 3-4h | Wire /system, /effort, /gate, /config set. Hard: inline /run. |
| S7 | Hardcoded values | 2-3h | Extract constants, read from config. ~20 call sites. |
| S11 | Mutex/unwrap risks | 1-2h | parking_lot swap, pattern fixes. |
| S4 | Silent error swallowing | 4-6h | 18+ .ok() in 21K-line orchestrate.rs. Needs judgment per-case. |
| S9 | Subprocess management | 3-4h | Timeouts, stderr redirect, handle storage. |
| S8 | Phantom features | 3-4h | Mostly one-liner wiring. Dream consumer is non-trivial. |
| S6 | Streaming | 8-12h | Hardest single item. SSE parser + UI threading. |
| S1 | Dispatch = agent session | 10-15h | Biggest arch change. Session-scoped agent for chat. |
| S10 | Duplicate code / two engines | 10-15h | 21K orchestrate.rs merge, chat loop dedup. High regression risk. |

## By Priority

| Priority | Items | Effort | What it gets you |
|---|---|---|---|
| P0 (security) | S5 | ~3h | Safe to deploy |
| P1 (usability) | S1, S2, S6, S3-partial | ~30h | Chat actually works like Claude Code |
| P2 (execution) | S7, S4-partial | ~6h | roko run / plan run more reliable |
| P3 (reliability) | S4, S8, S9, S11 | ~12h | Robustness, learning works |
| P4 (code health) | S10 | ~12h | Maintainability |

**Total: ~50-65 hours**

## Highest ROI

- S5 (security, 3h) + S2 (connection reuse, 3h) = 6h for safety + halved latency
- Critical path: S1 → S6 → S10 (intertwined, ~35h combined)
