# Cascade Router Design

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
