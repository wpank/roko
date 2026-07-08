# Gate Configuration

> The `[gate]` table configures Roko's verification pipeline: which gates run, in what
> order, how many retries agents get, and whether thresholds adapt over time.

**Status**: Shipping
**Crate**: `roko-gate`, `roko-orchestrator`
**Depends on**: [01-roko-toml-schema.md](01-roko-toml-schema.md)
**Used by**: [operations/error-handling/02-recovery-strategies.md](../error-handling/02-recovery-strategies.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

The gate pipeline is a sequence of verifiers that run after each agent task. The default
pipeline for a coding project is:

```toml
[gate]
pipeline    = ["compile", "test", "clippy", "diff"]
max_retries = 3
```

---

## What Gates Are

A gate is a deterministic pass/fail check applied to the agent's output. Gates do not
use LLMs. They run compilation, test suites, static analysis, diff analysis, and semantic
checks using local tooling. The agent does not see gate implementations; it only receives
the failure message if a gate rejects its output.

Roko ships 11 gates across 6 rungs. Gates in a rung are semantically related:

| Rung | Gates | What they check |
|------|-------|----------------|
| 0 — Structural | `compile` | Does the code compile? (`cargo check`) |
| 1 — Quality | `clippy` | Clippy lints with project settings |
| 1 — Quality | `format` | `cargo fmt --check` |
| 2 — Functional | `test` | `cargo nextest run` (or `cargo test`) |
| 2 — Functional | `coverage` | Test coverage floor (if configured) |
| 3 — Safety | `security` | `cargo-audit` + deny policy checks |
| 4 — Diff | `diff` | Diff size and churn within bounds |
| 5 — Semantic | `semantic` | LLM-judge rubric scoring against task spec |

Gates within a rung run in parallel; rungs execute sequentially. A failure in rung 0
stops execution without running rung 1, 2, 3, 4, or 5.

---

## The `pipeline` Key

```toml
[gate]
pipeline = ["compile", "test", "clippy", "diff"]
```

List the gates you want, in the order you want them. Roko respects the order for
reporting but runs rung-grouped parallelism internally.

**Minimal pipeline (fast feedback loop):**
```toml
pipeline = ["compile"]
```

**Default coding pipeline:**
```toml
pipeline = ["compile", "test", "clippy", "diff"]
```

**Full production pipeline:**
```toml
pipeline = ["compile", "test", "clippy", "format", "security", "diff", "semantic"]
```

**Research / writing pipeline (no compilation):**
```toml
pipeline = ["semantic"]
```

An **empty pipeline** (`pipeline = []`) disables all verification. Use this only for
exploratory or throwaway runs; never in production.

---

## Adaptive Thresholds

When `adaptive_thresholds = true` (the default), each gate tracks its pass rate via an
exponential moving average (EMA) with α = 0.1. The threshold for pass/fail adjusts
automatically:

- A gate with a consistent 98% pass rate will tighten its threshold (e.g. raise the
  minimum score for the `semantic` gate, or tighten the diff size limit for `diff`).
- A gate that is failing frequently will loosen its threshold to avoid blocking all
  progress while the root cause is being fixed.

**Why EMA with α = 0.1?** At α = 0.1, a gate's threshold responds to the last ~10
observations, giving a half-life of ~7 observations. This is slow enough to avoid
thrashing on individual noisy failures but fast enough to respond to a systematic
regression within a handful of tasks.

Disable adaptive thresholds to use fixed gate criteria:

```toml
[gate]
adaptive_thresholds = false
```

Disable this in reproducibility-required environments (CI, audited pipelines) where you
need consistent pass/fail behaviour across runs.

---

## Retries and Iteration Memory

When a gate fails, Roko does not immediately mark the task as failed. Instead:

1. The gate failure is encoded as a structured message and appended to the agent's
   context as an "iteration memory" entry.
2. The agent is re-invoked with the iteration memory visible. It can see:
   - What gate failed and why.
   - Which errors appeared in prior iterations (the "DO NOT RETRY" list).
   - The current iteration count.
3. After `max_retries` exhausted, the task is marked `Failed(GateFailed)`.

The iteration memory prevents the agent from repeating the same mistake on every retry.
If iteration 1 produced a type mismatch `E0308` and iteration 2 produced an import
error, iteration 3 sees both in its DO-NOT-RETRY list.

**Example with 5 retries for a complex task:**

```toml
[gate]
pipeline    = ["compile", "test", "clippy", "diff", "semantic"]
max_retries = 5
```

---

## Gate Timeouts

The `timeout_seconds` key limits each individual gate's execution time. This prevents
a slow test suite or hanging security scan from blocking the pipeline indefinitely:

```toml
[gate]
timeout_seconds = 180
```

If a gate times out, it is treated as a failure with the reason `"gate_timeout"`. The
agent receives this as a gate failure and retries (up to `max_retries`).

For large test suites, increase this value. The default of 120 seconds is appropriate
for small-to-medium projects with nextest.

---

## Custom Gates

Custom gates can be registered via the CLI:

```toml
[gate]
pipeline = ["compile", "test", "custom:integration"]
```

A custom gate named `integration` maps to a shell command configured outside the TOML
(in a future release, this will be inline in `roko.toml`). Custom gate configuration
is not yet stabilised.

---

## Two Full Examples

**Fast iteration pipeline (laptop, exploratory work):**

```toml
[gate]
pipeline              = ["compile"]
continue_on_failure   = false
max_retries           = 2
timeout_seconds       = 60
adaptive_thresholds   = true
```

**Production CI pipeline (server, team use):**

```toml
[gate]
pipeline              = ["compile", "test", "clippy", "format", "security", "diff", "semantic"]
continue_on_failure   = false
max_retries           = 4
timeout_seconds       = 300
adaptive_thresholds   = false  # deterministic in CI
```

---

## See Also

- [reference/05-operators/gate.md](../../reference/05-operators/gate.md) — gate internals, rung structure, verdict types
- [operations/error-handling/02-recovery-strategies.md](../error-handling/02-recovery-strategies.md) — what happens after max_retries
- [operations/error-handling/01-error-taxonomy.md](../error-handling/01-error-taxonomy.md) — gate-verdict error class

## Open Questions

- Per-gate `timeout_seconds` override (e.g. `gate.timeouts.test = 300`) is not yet implemented.
- Coverage gate floor configuration (`coverage.min_percent`) is not yet in the schema.
- The `semantic` gate rubric is currently the built-in rubric; configuring a custom rubric path is planned.
