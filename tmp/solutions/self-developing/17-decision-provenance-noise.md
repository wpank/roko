# 17: "Decision provenance" Card Clutters Zed UI

## Problem

Every non-slash-command prompt in Zed shows a visible "Decision provenance" tool-call card. Users don't know what it is, it takes up space, and it makes the agent look like it's doing something weird before answering.

The card shows knowledge hits, playbook matches, episode history, and dream patterns with a confidence percentage. Example:
```
🔍 Decision provenance
3 sources, 72% confidence

- Playbook `plan-gen-001` (5 runs, 80% success)
- Episode task-42: pass (compile:pass test:pass)
- Knowledge: "plan generation requires opus for complex PRDs..."
```

## Root Cause

Three separate call sites emit the provenance card as a `CognitiveEvent::ToolCallStart` + `ToolCallComplete`, which renders as a visible collapsible card in Zed:

1. `bridge_events.rs:1058-1062` — emitted on every non-slash-command, non-pipeline prompt
2. `runner.rs:593-594` — emitted during workflow engine `strategizing` phase
3. `runner.rs:1029-1046` — emitted during legacy pipeline `SpawnStrategist` action

The provenance data is useful for the MODEL (as context), but useless for the USER (as UI chrome).

## Fix Applied (2026-05-06)

Removed all three `emit_provenance_card` / `publish_provenance_card` call sites. The provenance data still flows into the model's context via `provenance_card` (rendered text passed through `knowledge_context`), but no visible tool-call card appears in Zed.

Also cleaned up dead code:
- Removed `provenance_card` field from `AcpWorkflowEventConsumer` struct
- Removed `publish_provenance_card` method
- Removed `emit_provenance_card` function

## What This Means for Users

- No more confusing "Decision provenance" cards in Zed
- The model still gets provenance context (knowledge, playbooks, episodes, dream patterns) in its system prompt — it just doesn't show the user
- If provenance visibility is desired in the future, it should be a config option (`show_provenance = true`) or a debug mode, not the default

## Files Modified

| File | Change |
|------|--------|
| `crates/roko-acp/src/bridge_events.rs:1058` | Removed `emit_provenance_card` call |
| `crates/roko-acp/src/bridge_events.rs:2580` | Removed `emit_provenance_card` function |
| `crates/roko-acp/src/runner.rs:515` | Removed `provenance_card` field from struct |
| `crates/roko-acp/src/runner.rs:541` | Removed `publish_provenance_card` method |
| `crates/roko-acp/src/runner.rs:593` | Removed strategizing-phase provenance publish |
| `crates/roko-acp/src/runner.rs:1029` | Removed legacy pipeline provenance publish |
