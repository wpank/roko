# D — Feedback & Thresholds (Docs 06, 08)

Post-audit refresh for adaptive-threshold and gate-feedback status.

---

## Verdict

This section is **mostly shipped**, with one important scoping rule:

- the EMA/persistence/data-capture loop is real
- some read-side/advisory behavior is still narrower than the docs imply

That is a much smaller gap than the old parity pack described.

---

## Adaptive Thresholds

### Shipped

- `AdaptiveThresholds` tracks per-rung EMA pass rates.
- It loads from and saves to `.roko/learn/gate-thresholds.json`.
- Gate runs update the tracked rung after execution.
- CLI, TUI, and HTTP surfaces expose retry/skip advisories from the same data.

Key anchors:

- `crates/roko-gate/src/adaptive_threshold.rs:71-188`
- `crates/roko-cli/src/orchestrate.rs:3828-3832`
- `crates/roko-cli/src/orchestrate.rs:3962-3966`
- `crates/roko-cli/src/orchestrate.rs:4100-4104`
- `crates/roko-cli/src/orchestrate.rs:4563-4568`
- `crates/roko-cli/src/orchestrate.rs:6181-6245`
- `crates/roko-serve/src/routes/learning.rs:101-113`
- `crates/roko-serve/src/routes/learning.rs:256-265`

### Narrow

The refreshed docs should say:

- adaptive thresholds are **wired**
- the system **persists EMA per rung**
- retry/skip advice is **observable**

They should **not** overclaim that every advisory is already the dominant control path for orchestration policy.

---

## Gate Results Into Learning

This is live and should stay in present tense:

- gate runs create gate episodes
- gate runs feed completed-run enrichment
- gate verdicts become part of the learning/event surface

Key anchors:

- `crates/roko-cli/src/orchestrate.rs:6181-6245`
- `crates/roko-learn/src/runtime_feedback.rs:782-845`
- `crates/roko-learn/src/episode_logger.rs:90-119`
- `crates/roko-learn/src/episode_logger.rs:860-885`

---

## GateFeedback

### Shipped foundation

- `GateFeedback` and `feedback_for_agent(...)` exist as real library code.
- The classifier is implemented and serializable.

Key anchors:

- `crates/roko-gate/src/feedback.rs:53-95`
- `crates/roko-gate/src/feedback.rs:100-192`
- `crates/roko-gate/src/feedback.rs:202-237`

### Narrow the wording

Do not claim this is the canonical runtime retry path unless the orchestrator callsite proves it.

The accurate posture is:

- classifier exists
- it is a valid shipped foundation
- full retry-path ownership should be described cautiously

---

## Explicitly Defer

Mark these parts of doc `06` as future work:

- SPC detector stack
- extended health analytics
- control-chart / BOCPD / PELT style expansions

They are not required to explain the current verification runtime.

---

## Replacement Summary

This section should now read as:

- **shipped**: EMA thresholds, persistence, visibility, gate-result learning hooks
- **partial**: advisory/read-side orchestration use and structured retry shaping
- **deferred**: advanced statistical monitoring
