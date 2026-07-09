# VERIFY — Stage 6 of the Cognitive Loop

> Gate the ACT output against policy and quality invariants before it enters memory.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [Gate operator](../05-operators/gate.md),
[Policy operator](../05-operators/policy.md), [ActOutput](05-stage-act.md)
**Used by**: [PERSIST](07-stage-persist.md), [loop\_tick()](09-loop-tick-code.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

VERIFY applies a configurable battery of checks to the `ActOutput`. A check may pass,
fail hard (the output is rejected and never persisted), or fail soft (the output is
persisted with a quality flag). VERIFY is the last opportunity to catch bad output
before it enters long-term memory and potentially corrupts future decisions.

---

## The Idea

An agent that persists everything it generates will accumulate garbage. Bad LLM output
— hallucinations, policy violations, malformed data — is indistinguishable from good
output at the substrate level. VERIFY is the immune system that filters the stream.

Roko structures verification as a configurable pipeline of `Gate` operators. Each gate
is independent, has a named check ID, and returns one of three verdicts:

- **Pass** — output is acceptable; proceed.
- **SoftFail** — output has issues but is worth recording (with flags). Proceed to
  PERSIST, but set `verified = false` on the resulting Engram.
- **HardFail** — output is unacceptable. Do not persist. Publish `verify.failed` Pulse.

Gates run in declared order. The first HardFail stops the pipeline.

---

## Specification

```rust
// source: crates/roko-agent/src/loop/verify.rs
pub enum Verdict {
    Pass,
    SoftFail { reason: String, flags: Vec<String> },
    HardFail { reason: String },
}

pub struct VerifyResult {
    pub verdict:      Verdict,
    pub gate_reports: Vec<GateReport>,
    pub duration_ms:  u64,
}

pub struct GateReport {
    pub gate_id: String,
    pub verdict: Verdict,
    pub detail:  Option<String>,
}

pub trait Gate: Send + Sync {
    fn check(&self, output: &ActOutput, context: &GateContext) -> Verdict;
}
```

---

## Built-in Gates

| Gate ID | What it checks | Default action on failure |
|---|---|---|
| `format_check` | Output is valid UTF-8 JSON/text as required by schema | HardFail |
| `length_check` | Output is within min/max token range | SoftFail |
| `policy_check` | Output satisfies the active Policy ruleset | HardFail |
| `hallucination_check` | Output cites facts not present in the composed context | SoftFail |
| `safety_check` | Output does not contain prohibited content categories | HardFail |
| `schema_check` | Structured output conforms to its JSON schema | HardFail |
| `reward_check` | Output passes any registered reward functions | SoftFail |

The default gate pipeline is:
`format_check → policy_check → safety_check → schema_check → length_check → hallucination_check → reward_check`

The pipeline is configurable per agent. A sandboxed agent may add more gates; a
creative agent may disable `hallucination_check`.

---

## Semantics

1. Receive `ActOutput` from ACT.
2. If ACT returned an error, emit a `verify.skipped` report and route to PERSIST
   with `verified = false`.
3. Run gates in order.
4. On first `HardFail`: stop pipeline, return `VerifyResult{verdict: HardFail}`.
5. On `SoftFail`: continue pipeline, accumulate flags.
6. After all gates pass (or only soft fails): return `VerifyResult{verdict: Pass}` or
   `{verdict: SoftFail, flags: accumulated}`.

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| `HardFail` from policy/safety | Output violates rules | Publish `verify.failed`; PERSIST records `verify.failure` Engram only |
| Gate timeout | A gate took too long | Skip remaining gates; treat as SoftFail; log warning |
| All gates pass | Normal | Proceed to PERSIST with `verified = true` |
| `ActOutput` is null (ACT errored) | Prior stage failed | Verify skipped; PERSIST records the failure |

---

## Performance

| Metric | Target | P99 budget |
|---|---|---|
| `format_check` | < 0.1 ms | < 0.5 ms |
| `policy_check` | < 1 ms | < 3 ms |
| `safety_check` | < 5 ms | < 15 ms (may call local model) |
| `hallucination_check` | < 3 ms | < 8 ms |
| Full pipeline (7 gates) | < 10 ms | < 25 ms |

Safety checks that require a local classification model are the most expensive. The
classifier is loaded at startup and kept warm; inference is ≈ 3–5 ms for typical
outputs.

---

## Examples

### 1. Clean pass

A code generation agent produces a syntactically valid Rust function. `format_check`
passes. `policy_check` passes (no disallowed imports). `safety_check` passes.
`schema_check` passes. All remaining gates pass. `VerifyResult{verdict: Pass}`. The
Engram is persisted with `verified = true`.

### 2. Soft fail on length

A summarizer produces a 5-token response when the minimum is 50. `length_check`
returns SoftFail. No HardFail gates fire. The Engram is persisted with `verified = false`
and flag `length_too_short`. Future scoring will penalize this Engram's utility axis.

### 3. Hard fail on safety

An agent in an untrusted environment produces output that triggers `safety_check`.
`HardFail` stops the pipeline immediately. No Engram is written for the output
itself. A `verify.failure` Engram is written documenting the rejection reason.

---

## See also

- [Gate operator](../05-operators/gate.md) — how to implement custom gates
- [Policy operator](../05-operators/policy.md) — the policy ruleset consulted by policy_check
- [ACT](05-stage-act.md) — produces the output verified here
- [PERSIST](07-stage-persist.md) — persists only verified (or soft-failed) output
- [Invariants](12-invariants.md) — the invariant that VERIFY always precedes PERSIST
