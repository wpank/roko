# 10 — Learning System: Dead Code & Broken Feedback Loops

## HIGH: 4 unexported learning modules (~800 LOC dead)

**Files in `crates/roko-learn/src/`:**

| Module | LOC | Purpose | References |
|--------|-----|---------|------------|
| `kalman.rs` | ~200 | Kalman filter for signal smoothing | Self only |
| `resonant_patterns.rs` | ~200 | Evolutionary pattern organisms (Lotka-Volterra) | Zero |
| `shapley.rs` | ~200 | Shapley value computation | Self only |
| `signal_metabolism.rs` | ~200 | Adaptive signal population dynamics | Zero |

These modules exist in the crate but are **not exported from `lib.rs`**. No consumer can access them. They were likely built by a mega-parity batch that implemented the algorithms without wiring them into the module tree.

---

## HIGH: Efficiency events record but don't close the loop

**File:** `crates/roko-cli/src/orchestrate.rs:5684-5763`

Efficiency events are written to `.roko/learn/efficiency.jsonl`. But the GAPS.md explicitly states:

```
runner_event_to_feedback translation produces empty model/provider/tokens
because RunnerEvent::TaskAttemptCompleted lacks usage.
```

This means the learning system records events with **no model attribution**. The cascade router can't learn which model performed well because the feedback has empty `model` and `provider` fields.

---

## HIGH: Cascade router needs context features never computed

**GAPS.md line 47:**
```
dispatch::ModelRouter::route returns the default slug when a CascadeRouter
is supplied — the cascade router needs a RoutingContext
```

The `CascadeRouter` was built to make intelligent model selection based on task tier, frequency, budget, and historical performance. But `ModelRouter::route()` falls back to the default slug because the `RoutingContext` struct is never populated with the required feature values.

**Result:** The cascade router is instantiated, persisted, and loaded from disk — but always returns the default model. All the bandit math runs but produces identical output to a hardcoded default.

---

## MEDIUM: LLM judge gate always skipped

**File:** `crates/roko-gate/src/gate_service.rs:197-256`

```rust
struct StubJudgeGate;

impl Verify for StubJudgeGate {
    async fn verify(&self, _signal: &Engram, _ctx: &Context) -> Verdict {
        Verdict::fail("stub-llm-judge", "LLM judge gate not yet implemented")
    }
}
```

And at runtime (line 249):
```rust
if matches!(gate_name.as_str(), "judge" | "llm-judge") {
    verdicts.push(skipped_gate_verdict(gate_name, "not implemented"));
    continue;
}
```

Rung 6 (highest complexity validation) is advertised but silently skipped. Projects that enable it get a "skipped" verdict, not a failure.

---

## MEDIUM: Dream triggers written but never consumed

**Files:**
- `crates/roko-cli/src/runtime_feedback/dreams.rs:20-22`
- `.roko/learn/dream_triggers.jsonl` (written to)

Dream triggers are written to JSONL but the GAPS.md confirms:
```
DreamTriggerSink writes triggers and optionally calls DreamRunner;
a production runner backed by roko-dreams::cycle::DreamCycle::run is not wired.
```

No cron job, no event loop subscriber, no background task reads the trigger file and runs the dream cycle.

---

## MEDIUM: Episode JSONL grows without bound

Episode logging writes to `.roko/episodes.jsonl` and `.roko/learn/efficiency.jsonl` with:
- No maximum file size enforcement
- No rotation policy
- No TTL-based trimming
- No archive/compress mechanism

After extended use, these files grow to tens or hundreds of megabytes, slowing reads.

---

## MEDIUM: Playbook store queried but not fed into prompts

**Files:**
- `crates/roko-cli/src/orchestrate.rs:15076` — `self.playbook.query()`
- `crates/roko-compose/src/prompt_assembly_service.rs:370` — `store.query()`

Playbooks ARE queried. But the query results are used for informational display, not injected into the system prompt builder's context. The agent never sees past playbook patterns in its prompt.

---

## LOW: VCG auction documentation stale

CLAUDE.md says: "VCG allocation in composition: Partial — greedy path dominates"

Reality: `vcg_allocate` IS called from `prompt.rs:1213`. But the `modulation` parameter is pre-applied then ignored:
```rust
let _ = modulation; // Already applied to adjusted_bid
```

The VCG mechanism degrades to greedy-by-value-density. The documentation should say "wired but degrades to greedy" not "partial."

---

## ROOT CAUSE: Write-only pattern

Multiple runner batches implemented the "record data" half of feedback loops without implementing the "use data for decisions" half:

| Batch | What it recorded | What it should inform |
|-------|-----------------|----------------------|
| arch S-track | Efficiency events → JSONL | Cascade router model selection |
| converge S-track | Dream triggers → JSONL | Dream consolidation cycle |
| mega-parity R3 | Knowledge queries at dispatch | System prompt context |
| mega-parity R4 | Routing observations | CascadeRouter feature weights |
| post-parity LF | Playbook patterns | Prompt assembly sections |

Each batch's prompt said "record X" and the agent dutifully wrote the recording code. No batch's prompt said "read X back and use it to make decision Y." This is the canonical "built but never connected" pattern called out in CLAUDE.md rule #2.
