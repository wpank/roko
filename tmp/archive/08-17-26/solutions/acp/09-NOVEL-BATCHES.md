# Novel Workflow Batches — Ready for Mega-Parity

## New Batches (P0 + P1 workflows from 08-NOVEL-WORKFLOWS.md)

### TOML entries (append to batches.toml)

```toml
# ============================================================
# ACP Novel Workflows — Surfacing roko's unique capabilities
# ============================================================

# --- Workflow 4: Provenance Chain (R5_F06) ---
[[batch]]
id = "R5_F06"
title = "Emit decision provenance cards showing knowledge → episode → playbook chain"
group = "5F"
deps = ["R5_F05"]
scope = ["crates/roko-acp/src/bridge_events.rs", "crates/roko-acp/src/runner.rs"]
also_read = ["crates/roko-neuro/src/lib.rs", "crates/roko-learn/src/playbooks.rs"]
verify = "quick"

# --- Workflow 6: Gate Autopsy (R7_F10) ---
[[batch]]
id = "R7_F10"
title = "Forensic gate failure analysis with causal chain + episode cross-reference"
group = "7F"
deps = ["R7_F04"]
scope = ["crates/roko-acp/src/runner.rs"]
also_read = ["crates/roko-gate/src/lib.rs", "crates/roko-learn/src/episodes.rs"]
verify = "quick"

# --- Workflow 8: Routing Explainer (enhance R5_F02) ---
# Already covered by R5_F02 — add routing explanation to cascade router card

# --- P1 workflows (deferred) ---
# R7_F07: Affect display (Workflow 2: Mood Ring) — deferred to P1
# R7_F08: Dream journal (Workflow 3) — deferred to P1
# R3_F05: Tournament mode (Workflow 5) — deferred to P2
```

---

## Prompt Files

### R5_F06: Decision Provenance Cards

When the agent applies a pattern (retry-with-backoff, error handling, etc.),
trace it through: playbook → episodes → dream insights → research.
Emit as ToolCall card with hierarchical content blocks.

Key implementation:
- Query playbook store for pattern matches at dispatch time
- For each match, include source episodes and confidence
- If dream routing advice exists, include it
- Show HDC similarity score

### R7_F10: Gate Autopsy

When gates fail, don't just dump error output. Analyze:
1. What the agent changed (git diff)
2. What the test expected (parse test assertion)
3. Why they don't match (type mismatch? logic error? missing update?)
4. Cross-reference with similar past failures (episode query)
5. Classify error type (compile, type, logic, test-drift, runtime)

Emit as ToolCall card with causal chain in content blocks.

### R7_F07: Affect State Display

Load DaimonState at pipeline start. After each phase:
1. Update PAD (pleasure/arousal/dominance) based on outcome
2. If arousal high + pleasure low (frustration): emit affect card
3. If frustration persists 2+ phases: suggest model escalation
4. Check somatic markers for similar past situations

Emit as AgentMessageChunk with affect summary.

### R7_F08: Dream Journal at Session Start

At `session/new`:
1. Check for `.roko/learn/dream-report.json` (last consolidation)
2. If newer than last session: emit dream report as ToolCall card
3. Include: replayed episodes, imagined scenarios, promoted insights,
   routing updates, threat rehearsals
4. Mark report as presented (don't show again)

---

## Updated Batch Count

| Category | Batches | IDs |
|----------|---------|-----|
| Agent session (R3) | 4 | R3_F01-F04 |
| Telemetry/learning (R5) | 6 | R5_F01-F06 |
| UX polish (R7) | 10 | R7_F01-F10 |
| **Total** | **20** | |

## Full Dependency DAG (Updated)

```
Wave 1 (independent, run immediately):
  R7_F01 (conversation history)
  R7_F02 (file change notifications)
  R7_F03 (slash commands + concurrency)
  R7_F04 (phase badges + iteration)
  R7_F06 (context providers)
  R7_F07 (affect display)
  R7_F08 (dream journal)

Wave 2 (after R7_F04):
  R7_F05 (narrative text)
  R7_F10 (gate autopsy)

Wave 3 (after mega-parity R3 core):
  R3_F01 (ACP dispatcher)

Wave 4 (after R3_F01):
  R3_F02 (system prompts)
  R3_F03 (safety contracts)
  R3_F04 (permission bridge + graduated trust)

Wave 5 (after mega-parity R5 core):
  R5_F01 (episode logging)

Wave 6 (after R5_F01):
  R5_F02 (cascade router + routing explainer)
  R5_F03 (cost tracking + UsageUpdate)
  R5_F05 (knowledge cards)

Wave 7 (after R5_F02 + R5_F05):
  R5_F04 (integration proof)
  R5_F06 (provenance chains)
```

## What This Gets You

With all 20 batches, roko ACP sessions will show:

1. ✅ Phase badges with iteration tracking (⊕ Strategizing, 🔧 Auto-fixing iter 2)
2. ✅ Knowledge cards with relevance scores from neuro store
3. ✅ Permission dialogs with graduated trust learning
4. ✅ Narrative text explaining what happened between phases
5. ✅ Context resolution (@file, @branch-diff, @logs)
6. ✅ Token/cost counter in status bar
7. ✅ Decision provenance chains (playbook → episode → dream)
8. ✅ Gate autopsy with causal analysis and episode cross-reference
9. ✅ Affect state display with auto-escalation
10. ✅ Dream journal reports at session start
11. ✅ Model routing explanations with override learning
12. ✅ Conversation history across turns
13. ✅ File change notifications after commits
14. ✅ Safety contract enforcement per mode
15. ✅ 9-layer system prompts (not static strings)
16. ✅ Episode logging for all ACP sessions
17. ✅ CascadeRouter learning from ACP usage

**No other ACP agent has items 2, 7, 8, 9, 10, or 11.**
Items 1, 3, 4, 5 exist partially in some agents but not with this depth.
Items 6, 12-17 are table stakes done right.
