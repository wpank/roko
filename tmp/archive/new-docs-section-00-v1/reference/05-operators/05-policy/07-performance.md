# Policy — Performance

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Implementation](./03-implementation.md)
**Last reviewed**: 2026-04-19

---

Policy is called once per tick, after the action has already been taken and its outcome
observed. It is on the **non-critical path** — the action latency has already been paid.
Policy overhead is rarely a bottleneck.

---

## `CircuitBreakerPolicy` complexity

| Operation | Time complexity | Notes |
|---|---|---|
| Rolling window update (`push_back` + `pop_front`) | O(1) | `VecDeque` with fixed capacity |
| Failure rate calculation | O(window_size) | Linear scan; window_size ≤ 10–50 in practice |
| Escalation streak check | O(1) | Single integer comparison |
| Circuit state transition | O(1) | Enum assignment |
| Total per tick | O(window_size) | window_size = 10 → ~10 comparisons |

<!-- ADDED: Complexity analysis inferred from data structure choices in implementation -->

Memory usage: `window_size * size_of::<bool>()` = 10 bytes for the default window. The
entire `CircuitBreakerPolicy` struct fits in a single CPU cache line.

---

## `SafetyPolicy` complexity

| Operation | Time complexity | Notes |
|---|---|---|
| Per-classifier check | O(response_length) | String scan or regex match |
| Total with N classifiers | O(N × response_length) | Short-circuits on first violation |

Safety classifiers are the dominant cost. A response of 1 000 tokens with 3 regex
classifiers typically takes 20–100 µs depending on pattern complexity.

<!-- ADDED: Classifier cost estimate inferred from typical safety filter benchmarks -->

---

## Where Policy sits in the tick budget

A typical 200 ms LLM-call tick:

```
SENSE        ~1 ms
RECALL       ~5 ms   (Substrate query)
SCORE        ~1 ms
GATE         ~1 ms
ROUTE/ACT   ~180 ms  (LLM call)
OBSERVE      ~2 ms
Policy       ~0.1 ms (CircuitBreakerPolicy) – ~1 ms (SafetyPolicy with 3 classifiers)
STORE        ~2 ms
─────────────────
Total       ~192 ms
```

Policy contributes <0.5% of total tick time for the circuit breaker path. Safety
classifiers add 0.5–5 ms, still well within normal tick budgets.

<!-- ADDED: Tick budget breakdown inferred from architecture timing model -->

---

## Gamma-speed loop considerations

At Gamma speed (sub-second, sub-100 ms target), every millisecond counts. Recommendations:

- Use `CircuitBreakerPolicy` only — avoid heavy safety classifiers on the hot path.
- Move safety classification to an async side-channel: publish the response to a
  `response.audit` Bus topic and check it off the main loop path.
- Keep `window_size` at 5–10 to reduce the failure-rate scan cost.

---

## Caching and warm paths

`CircuitBreakerPolicy` has no cold-start cost. The rolling window starts empty; failure
rate is 0% until `window_size` ticks have elapsed.

`SafetyPolicy` classifiers may have a warm-up cost if they load ONNX models or regex
engines on first call. Pre-warm by calling `evaluate` with a dummy outcome during agent
initialisation.

<!-- ADDED: Cold-start and warm-up considerations inferred from ML classifier lifecycle patterns -->

---

## Open Questions

- Should the safety classifier path be moved off-loop entirely? Doing so would reduce
  per-tick latency but introduce a window between action and safety check.
- Should `window_size` be dynamically adjusted based on tick latency?
