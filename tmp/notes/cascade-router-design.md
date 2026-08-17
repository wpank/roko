# Cascade Router Design

> **STALE**: Design sketch from May 2026. CascadeRouter is fully implemented in
> `roko-learn` and persists to `.roko/learn/cascade-router.json`. See `crates/roko-learn/src/cascade_router.rs`.
> Last updated: 2026-08-13

## Goal
Route tasks to the cheapest model that can handle them.

## Tiers
1. Haiku (fast, cheap) — docs, formatting, simple edits
2. Sonnet (balanced) — implementation, refactoring
3. Opus (powerful) — architecture, complex debugging

## Routing signals
- Task complexity estimate (from plan metadata)
- Historical success rate per model per task type
- Current budget remaining
- Token estimate for prompt

## Learning
- Track success/fail per (model, task_type) pair
- EMA update on each outcome
- Persist to `.roko/learn/cascade-router.json`
