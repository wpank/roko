# Novel ACP Workflows — Making Roko Best-in-Class

## Competitive Context

**33+ ACP agents exist** (Claude Code, Codex CLI, Gemini CLI, Copilot CLI, Goose, Kiro, etc.).
Every one of them is a thin wrapper around a single LLM + tool loop. None have:
- Self-learning memory
- Affect/emotional intelligence
- Dream consolidation
- Multi-agent orchestration with DAG execution
- 7-rung verification pipeline
- Knowledge provenance chains
- Hyperdimensional code fingerprints

**Roko has all 15.** The gap isn't building — it's surfacing.

## Design Principle

> Every ACP session/update notification should make the user think:
> "I've never seen an agent do this."

The protocol primitives are: `AgentMessageChunk`, `ToolCall`, `Plan`, `UsageUpdate`,
`ConfigOption`, `session/request_permission`. Everything below maps to these.

---

## WORKFLOW 1: "Déjà Vu" — Knowledge-First Dispatch

**What happens**: Before the agent writes a single line of code, roko searches its
memory for similar problems. If it finds matches, it shows them as a knowledge card
and injects them into the agent's context.

**UX flow**:
```
User: "Fix the checkout 5xx spike"

┌─────────────────────────────────────────────────────────────┐
│ 🧠 Knowledge from neuro store — 3 hits                      │
│                                                             │
│  0.94  Playbook: P1 5xx → confirm scope → bisect deploy    │
│        window → minimal hotfix                              │
│  0.87  Episode #2812 — Last 5xx spike: nil deref after     │
│        refactor. Same shape.                                │
│  0.71  Research: checkout.go error handling patterns        │
│                                                             │
│  💡 Injected 3 items into agent context (2,847 tokens)      │
└─────────────────────────────────────────────────────────────┘

Agent: "I've seen this pattern before. Episode #2812 shows the
same shape — nil deref after a refactor..."
```

**Why unique**: No other agent has persistent cross-session memory with relevance
scoring. Claude Code has memory files but they're flat text — no similarity search,
no episode matching, no playbook retrieval.

**ACP mapping**:
- `ToolCall` (kind: other) for the knowledge card
- `AgentMessageChunk` for the agent's narrative referencing matches
- Knowledge injected into system prompt via `PromptAssemblyService`

**Batch**: R5_F05 (already defined)

---

## WORKFLOW 2: "Mood Ring" — Affect-Aware Dispatch

**What happens**: Roko's daimon engine tracks agent emotional state (PAD model:
pleasure/arousal/dominance). When the agent is frustrated (repeated failures),
roko automatically adjusts: switches to a more capable model, reduces task scope,
or suggests the user take over.

**UX flow**:
```
⊕ Implementing

[Agent writes code, tests fail, auto-fixes, tests fail again]

┌─────────────────────────────────────────────────────────────┐
│ 💛 Agent state: frustrated (arousal: high, pleasure: low)   │
│                                                             │
│  Somatic memory: "Last time this pattern occurred           │
│  (episode #1847), switching to Opus resolved it."           │
│                                                             │
│  ⚡ Auto-escalating: sonnet → opus                          │
│  📉 Reducing scope: full checkout.go → lines 13-45 only    │
└─────────────────────────────────────────────────────────────┘
```

**Why unique**: Zero agents expose internal state. Zero adjust behavior based on
emotional trajectory. This is roko's daimon engine — built, just not surfaced.

**ACP mapping**:
- `AgentMessageChunk` for the affect state card (markdown)
- `ConfigOptionUpdate` to reflect model escalation
- `Plan` entries updated to reflect scope reduction

**New batch needed**: R7_F07

---

## WORKFLOW 3: "Dream Journal" — Offline Consolidation Report

**What happens**: Between sessions (overnight, lunch break), roko runs a dream
consolidation cycle: replays important episodes, imagines counterfactuals, rehearses
threat scenarios. When the user returns, roko presents what it learned.

**UX flow**:
```
User opens new session in the morning.

┌─────────────────────────────────────────────────────────────┐
│ 🌙 Dream report — consolidated overnight                    │
│                                                             │
│  Replayed: 3 episodes (error handling in checkout module)   │
│  Imagined: "What if we had used Option<> instead of         │
│            unwrap()?" → 4 fewer panics in simulation       │
│  Rehearsed: 2 threat scenarios                              │
│    ⚠ Race condition in cart update (confidence: 0.73)       │
│    ⚠ OOM on large order batch (confidence: 0.61)           │
│                                                             │
│  Promoted: 2 insights to durable knowledge                  │
│  Forgot: 14 ephemeral task notes (below decay threshold)    │
│                                                             │
│  💡 Routing updated: Opus preferred for error handling tasks │
└─────────────────────────────────────────────────────────────┘
```

**Why unique**: Nobody else has offline learning. Claude Code's `/dream` command
exists but it's just memory file cleanup. Roko's dream engine does counterfactual
imagination, threat rehearsal, and routing advice — actual computation, not just
tidying.

**ACP mapping**:
- At `session/new`, check for pending dream reports
- Emit as `ToolCall` card (kind: other, title: "Dream report")
- Routing changes reflected in `ConfigOptionUpdate`

**New batch needed**: R7_F08

---

## WORKFLOW 4: "Provenance Chain" — Traceable Decisions

**What happens**: When roko makes a decision (model choice, approach selection,
pattern application), it shows the full provenance: which memory fragment, which
episode, which playbook, which dream insight influenced the decision.

**UX flow**:
```
Agent: "I'm using the retry-with-backoff pattern here."

┌─────────────────────────────────────────────────────────────┐
│ 📋 Decision provenance                                      │
│                                                             │
│  Pattern: retry-with-backoff                                │
│  ├─ Playbook #47 (mined from 12 episodes)                  │
│  │   └─ Success rate: 94% across 34 uses                   │
│  ├─ Episode #2134 — first successful application           │
│  │   └─ Gate pass: compile ✓ test ✓ clippy ✓               │
│  ├─ Dream insight (2026-04-27)                              │
│  │   └─ "Backoff with jitter outperforms fixed delay"       │
│  └─ Research: RFC 6585 (retry-after semantics)              │
│                                                             │
│  Confidence: 0.91 (HDC similarity: 0.94)                   │
└─────────────────────────────────────────────────────────────┘
```

**Why unique**: Every other agent treats context as a flat bag of tokens. Nobody
shows WHY a decision was made, traced back through memory → episodes → playbooks →
research. This is roko's custody audit chain + HDC similarity + knowledge store
working together.

**ACP mapping**:
- `ToolCall` card (kind: other, title: "Decision provenance")
- Content blocks with hierarchical text showing the chain
- HDC similarity score in metadata

**New batch needed**: R7_F09

---

## WORKFLOW 5: "Tournament" — Adversarial Multi-Agent

**What happens**: For high-stakes tasks, roko spawns 2-3 agents with different
strategies (e.g., conservative refactor vs. aggressive rewrite vs. incremental fix)
and lets them compete. The user sees a side-by-side comparison.

**UX flow**:
```
User: "Refactor the payment module for extensibility"
Config: workflow=tournament

┌─────────────────────────────────────────────────────────────┐
│ 🏆 Tournament — 3 approaches competing                      │
│                                                             │
│  Agent A (conservative): Extract interface, keep impl       │
│    ├─ Files: 3 modified, 0 new                              │
│    ├─ Gates: compile ✓ test ✓ clippy ✓                      │
│    └─ Cost: $0.12, 4,200 tokens                             │
│                                                             │
│  Agent B (aggressive): Full rewrite with trait objects       │
│    ├─ Files: 2 deleted, 5 new                               │
│    ├─ Gates: compile ✓ test ✗ (3 failures) clippy ✓         │
│    └─ Cost: $0.34, 12,100 tokens                            │
│                                                             │
│  Agent C (incremental): Add builder pattern, keep existing  │
│    ├─ Files: 4 modified, 1 new                              │
│    ├─ Gates: compile ✓ test ✓ clippy ✓                      │
│    └─ Cost: $0.08, 2,900 tokens                             │
│                                                             │
│  🏅 Recommendation: Agent C (best cost/quality ratio)        │
│                                                             │
│  [Apply A] [Apply B] [Apply C] [Compare diffs]              │
└─────────────────────────────────────────────────────────────┘
```

**Why unique**: Multi-agent exists in Claude Code teams and Cursor 3, but they're
cooperative (divide tasks). Nobody does adversarial/competitive exploration where
agents independently solve the same problem and the user picks the winner.

**ACP mapping**:
- `Plan` entries for each agent's progress
- `ToolCall` cards for each agent's results
- `session/request_permission` for final selection (apply A/B/C)
- `UsageUpdate` with per-agent cost breakdown

**New batch needed**: R3_F05 (tournament mode in pipeline)

---

## WORKFLOW 6: "Gate Autopsy" — Forensic Failure Analysis

**What happens**: When gates fail, instead of just showing the error output, roko
runs a forensic replay: reconstructs the causal chain from the original prompt
through each decision to the failure point.

**UX flow**:
```
⊕ Gating

✗ test gate failed (2 failures)

┌─────────────────────────────────────────────────────────────┐
│ 🔬 Gate autopsy — forensic analysis                         │
│                                                             │
│  Root cause: Implementer changed return type from           │
│  Result<Cart> to Option<Cart> (line 14) but test_checkout   │
│  still expects Result::Err variant (test:47)                │
│                                                             │
│  Causal chain:                                              │
│  1. Prompt: "fix nil deref" → agent chose Option<>          │
│  2. File edit: checkout.go:14 (Result→Option)               │
│  3. Missed: test_checkout.go:47 still uses .unwrap_err()    │
│  4. Gate: test_checkout::test_error_case FAILED             │
│                                                             │
│  Similar past failure: Episode #1203 (same pattern)         │
│  Resolution then: Update test to match new return type      │
│                                                             │
│  Confidence: 0.89 (this is a type mismatch, not logic bug)  │
└─────────────────────────────────────────────────────────────┘

🔧 Auto-fixing iter 2
```

**Why unique**: Every other agent just dumps the error output. Roko does causal
chain reconstruction, cross-references with past failures, and classifies the
error type. This is roko's forensic replay + episode matching + post-gate
reflection working together.

**ACP mapping**:
- `ToolCall` card (kind: other, title: "Gate autopsy")
- Structured content blocks showing causal chain
- Episode reference links
- Confidence score

**New batch needed**: R7_F10

---

## WORKFLOW 7: "Graduated Trust" — Somatic Permission Learning

**What happens**: Instead of binary Allow/Reject, roko learns your comfort level
per action type. High-trust actions auto-approve. Low-trust actions always ask.
Medium-trust actions ask the first few times, then learn.

**UX flow**:
```
First time editing production file:
┌─────────────────────────────────────────────────────────────┐
│ ⓘ Edit services/checkout.go?                                │
│   L13-17 · production-tracked file                          │
│                                                             │
│   [Allow] [Always Allow] [Reject]                           │
│                                                             │
│   Trust level: 🟡 Medium (first edit to this file)          │
│   Similar actions approved: 3/5 (60%)                       │
└─────────────────────────────────────────────────────────────┘

After 5 approvals to same file type:
  (auto-approved — trust level: 🟢 High)
  💬 "Auto-approved: editing checkout.go (trust: high, 5/5 prior approvals)"

After a rejection:
  (trust drops, future edits to this path always ask)
```

**Why unique**: Every permission system is binary. Nobody learns from your approval
patterns. Roko's somatic markers + behavioral state tracker can learn a personalized
autonomy profile that evolves with each session.

**ACP mapping**:
- `session/request_permission` with trust level metadata
- `AgentMessageChunk` for auto-approval notifications
- Trust profile stored in session state (persists across sessions)

**Enhancement to**: R3_F04 (permission bridge)

---

## WORKFLOW 8: "Model Routing Explainer" — Transparent Intelligence

**What happens**: When roko picks a model (via CascadeRouter), it explains WHY.
The user can override, and the override feeds back into learning.

**UX flow**:
```
Status bar: claude-sonnet-4.7

User clicks model selector:
┌─────────────────────────────────────────────────────────────┐
│ 🔀 Model routing — auto-selected                            │
│                                                             │
│  Task type: error handling (detected from prompt)           │
│  Cascade decision:                                          │
│    sonnet: 91% success, $0.08 avg, 3.2s latency            │
│    opus:   97% success, $0.34 avg, 8.1s latency            │
│    haiku:  72% success, $0.02 avg, 1.1s latency            │
│                                                             │
│  Selected: sonnet (best cost/quality at effort=medium)      │
│                                                             │
│  Override? Selecting a different model teaches the router.  │
│  [sonnet ✓] [opus] [haiku]                                  │
└─────────────────────────────────────────────────────────────┘
```

**Why unique**: Every agent just uses whatever model is configured. Nobody shows
the decision process OR learns from manual overrides. This is roko's cascade router
+ efficiency events + UX34 (force_backend override learning).

**ACP mapping**:
- `ConfigOption` for model selector (already exists)
- `ToolCall` card for routing explanation (on click/expand)
- `ConfigOptionUpdate` when user overrides
- Override recorded as efficiency event for router learning

**Enhancement to**: R5_F02 (cascade router)

---

## WORKFLOW 9: "Time Warp" — Session Replay & Branch

**What happens**: After a session completes, the user can scrub through a timeline
of every phase, decision, and edit. They can branch from any point: "What if I'd
rejected that edit at step 4?"

**UX flow**:
```
Session complete. [⟳ replay] button in status bar.

Click → opens timeline:
┌─────────────────────────────────────────────────────────────┐
│ ⟳ Session #acp-2847 — 12 turns, 4m 32s                     │
│                                                             │
│  ─●────●────●────●────●────●────●────●────●────●──          │
│   1    2    3    4    5    6    7    8    9   10             │
│   ↑ prompt  ↑ strategy  ↑ implement  ↑ gate    ↑ commit    │
│              ↑ knowledge  ↑ permission  ↑ autofix           │
│                                                             │
│  Turn 4: Edit checkout.go:13-17                             │
│  [View diff] [Branch from here] [View agent reasoning]      │
└─────────────────────────────────────────────────────────────┘
```

**Why unique**: Claude Code's episode files exist but no tool renders them as an
interactive timeline. `ckpt` does filesystem snapshots but not agent decision
replay. Roko's episode log + HDC fingerprints enable both.

**ACP mapping**:
- `/replay` slash command opens episode viewer
- `ToolCall` cards for each turn in replay mode
- `session/request_permission` for "branch from here" (creates new worktree)

**New batch needed**: R7_F11

---

## WORKFLOW 10: "Conductor Override" — Live Intervention

**What happens**: During pipeline execution, the user can intervene at any point:
pause, change model, adjust scope, skip a phase, or inject feedback mid-turn.

**UX flow**:
```
♦ Implementing (agent writing code)

User types mid-execution:
> "Actually, use the builder pattern instead of constructor"

┌─────────────────────────────────────────────────────────────┐
│ ⚡ Live intervention received                                │
│                                                             │
│  Agent will incorporate your feedback into the current      │
│  implementation phase.                                      │
│                                                             │
│  [Restart phase with feedback]                              │
│  [Queue for next iteration]                                 │
│  [Ignore (continue current approach)]                       │
└─────────────────────────────────────────────────────────────┘
```

**Why unique**: Every agent either runs to completion or gets cancelled. Nobody
supports mid-execution steering without restarting. Roko's conductor + circuit
breaker + cancel token architecture makes this possible.

**ACP mapping**:
- `session/prompt` during active session → treated as intervention
- `session/request_permission` for intervention options
- Pipeline pauses, incorporates feedback, resumes

**Enhancement to**: existing pipeline in runner.rs

---

## Priority Matrix

| Workflow | Uniqueness | Implementation Effort | Visual Impact | Priority |
|----------|-----------|----------------------|---------------|----------|
| 1. Déjà Vu (knowledge) | Very high | Medium (R5_F05 exists) | High | **P0** |
| 4. Provenance Chain | Very high | Medium | Very high | **P0** |
| 6. Gate Autopsy | High | Medium | High | **P0** |
| 8. Routing Explainer | High | Low (enhance R5_F02) | High | **P1** |
| 2. Mood Ring (affect) | Very high | Medium | Medium | **P1** |
| 7. Graduated Trust | High | Low (enhance R3_F04) | High | **P1** |
| 3. Dream Journal | Very high | Medium | Medium | **P1** |
| 10. Conductor Override | High | High | Very high | **P2** |
| 5. Tournament | Very high | Very high | Very high | **P2** |
| 9. Time Warp (replay) | High | High | High | **P2** |
