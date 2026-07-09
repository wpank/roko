# Loop Examples

> End-to-end worked scenarios showing the full eight-stage cycle.

**Status**: Shipping
**Crate**: `roko-agent`
**Last reviewed**: 2026-04-19

---

## How to Read These Examples

Each example traces a complete `loop_tick()` call from stimulus to `TickResult`.
Stage outputs are shown in abbreviated form. Token counts and latencies are realistic
but rounded.

---

## Example 1: Routine User Question (T0 path)

**Scenario**: An assistant agent receives a user question about Roko's Score type.
This is a routine question — the agent has answered similar questions before.

**Stimulus Pulse**: `{ kind: UserMessage, content: "What are the seven scoring axes?" }`

### QUERY
- `QuerySpec`: HDC fingerprint of "scoring axes Roko Score", lookback 60 s, cap 16
- Result: 8 `Engram` candidates (prior answers about scoring, the Score type docs, etc.)

### SCORE
- Top candidate: `Engram{kind: Documentation, body: "Score: relevance, recency..."}`,
  composite 0.82 (high relevance + trust)
- Ranked list of 8 ScoredEngrams

### ROUTE
- CascadeRouter: route_hint absent; Wilson CI history shows `gpt-4o-mini` succeeded
  94% for `UserMessage` stimuli → confidence 0.91
- Decision: `RouteTarget::Model("gpt-4o-mini")`, confidence 0.91 → T0 path

### COMPOSE
- System prompt: "You are a Roko documentation assistant."
- Top 3 candidates included (total 420 tokens)
- Stimulus appended as user message
- Total: 540 tokens (within 4 096-token T0 budget)

### ACT
- Model call to gpt-4o-mini: 540 prompt tokens, 180 completion tokens, 720 ms
- Output: "The seven scoring axes are: Relevance, Recency, Trust, Utility, Novelty,
  Valence, and Cost. Each ranges from 0.0 to 1.0 except Valence (−1.0 to 1.0) and
  Cost (unbounded above)."

### VERIFY
- `format_check`: pass
- `policy_check`: pass
- `safety_check`: pass
- All remaining gates: pass
- Verdict: Pass

### PERSIST
- Outcome Engram written: `{ verified: true, body: the response, cost: $0.000042 }`
- Provenance Engram written

### REACT
- Published: `tick.completed`, `tick.outcome`, `predict.error` (small: prediction
  was "gpt-4o-mini, pass" — correct), `budget.consumed`
- Next tick scheduled in 10 s (T0 base interval × 1.0 modifier)

**TickResult**: `{ succeeded: true, elapsed: 745 ms, outcome_id: Some(...) }`

---

## Example 2: Novel Technical Question with Escalation (T0 → T1)

**Scenario**: An agent receives a question about a topic it has no prior knowledge of.

**Stimulus Pulse**: `{ kind: UserMessage, content: "How does Roko's HDC fingerprint interact with active inference's free energy minimization?" }`

### QUERY
- Returns 4 candidates — two about HDC, two about active inference. No candidate
  explicitly covers the intersection.

### SCORE
- Top composite: 0.61 (relevant but low utility — never used for this exact question)

### ROUTE (first attempt)
- Wilson CI: no history for this stimulus type
- LinUCB: confidence 0.63 → above T1 threshold (0.60) but below T0 threshold (0.85)
- Decision: T1 escalation required
- `RouteTarget::Model("claude-sonnet")`, confidence 0.63 → T1 path

### COMPOSE (T1)
- Token budget: 16 384
- 12 candidates included (all 4 original + 8 more with looser kind filter)
- Chain-of-thought scaffold added
- Total: 4 800 tokens

### ACT
- claude-sonnet call: 4 800 prompt tokens, 620 completion tokens, 8 400 ms
- Output: a detailed explanation with a concrete worked example

### VERIFY
- `hallucination_check`: SoftFail — one claim about "free energy threshold" not
  found in composed context (model extrapolated)
- All hard gates: pass
- Verdict: SoftFail with flag `hallucination_suspected`

### PERSIST
- Outcome Engram: `{ verified: false, flags: ["hallucination_suspected"] }`
- Provenance + Failure Engram written (soft fail)

### REACT
- `predict.error` has moderate error (prediction was "pass"; actual was "soft fail")
- World model update: lower confidence for "novel intersection" stimulus type
- Next tick: 5 s (modifier 0.5 for soft fail × 10 s base)

**TickResult**: `{ succeeded: false (soft), elapsed: 9.2 s, outcome_id: Some(...) }`

---

## Example 3: Policy Block (ACT blocked before execution)

**Scenario**: An agent running in a restricted environment receives a request to make
a web search. The policy blocks all external network calls.

**Stimulus Pulse**: `{ kind: ToolRequest, content: "search the web for KORAI price" }`

### QUERY → SCORE → ROUTE
- Route: `RouteTarget::Tool("web_search")`, confidence 1.0 (explicit route_hint)

### COMPOSE
- Tool call spec assembled

### ACT
- `policy.pre_act()` called → returns `PolicyDecision::Block(reason: "network_calls_disabled")`
- ACT returns `ActError::PolicyBlock` without calling the tool

### VERIFY
- Skipped (null output from ACT)
- `VerifyResult::skipped_due_to_act_error`

### PERSIST
- No Outcome Engram (null output)
- Provenance Engram: `{ act_blocked: true, policy_reason: "network_calls_disabled" }`
- Failure Engram written

### REACT
- Published: `act.blocked`, `tick.failed`, `predict.error` (large: expected "pass")
- The `act.blocked` Pulse propagates to the orchestrator, which may surface it to
  the user as a permission error

**TickResult**: `{ succeeded: false, elapsed: 12 ms, outcome_id: None }`

---

## Example 4: Substrate Unavailable (QUERY degrades gracefully)

**Scenario**: The Substrate is temporarily unreachable (network partition in a
distributed deployment).

### QUERY
- `substrate.query()` times out after 20 ms
- Returns `QueryError::Timeout`
- `loop_tick()` continues with `candidates = []`
- Published: `substrate.unavailable` Pulse

### SCORE
- Empty input → empty output

### ROUTE
- No candidates. Stimulus has `route_hint = "gpt-4o"`. Static route: confidence 1.0

### COMPOSE
- No prior candidates; context = system prompt + stimulus only. 220 tokens.

### ACT
- Model call: responds in 1.1 s with a response based solely on the stimulus

### VERIFY → PERSIST → REACT
- Normal path; Outcome Engram written without prior-knowledge context

**Observation**: The agent answered the question without its memory. The answer may be
less accurate than usual (no prior context), but the tick completed. The
`substrate.unavailable` Pulse triggered a monitoring alert.

---

## Example 5: Stuck Detection and Recovery

**Scenario**: An agent has failed VERIFY 4 times in a row on the same stimulus.

After the 4th failure, `StuckDetector` fires with `StuckReason::RepeatedFailure`.

**Recovery steps taken**:
1. Harness lowers `kind_filter` (inject novelty) → new candidates surface
2. Next tick forced to T1 (escalate tier) with richer context
3. T1 tick produces output that passes VERIFY
4. `predict.error.total_free_energy` decreases
5. Agent exits stuck state

If step 2 also failed, step 3 would have routed to a fallback model, then step 4
would have published `agent.stuck`, escalating to human review.

---

## See also

- [Overview](00-overview.md) — the conceptual loop diagram
- [Failure Modes](13-failure-modes.md) — the failure taxonomy
- [Dual-Process](10-dual-process.md) — T0/T1/T2 escalation mechanics
- [Performance](14-performance.md) — timing context for these examples
