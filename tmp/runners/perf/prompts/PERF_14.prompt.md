# PERF_14: Parallel gate rungs (B08-c)

## Task

Coalesce parallel-safe gate pairs (`compile`+`fmt`,
`compile`+`format-check`, `clippy`+`fmt`, `clippy`+`format-check`) into
groups that run via `futures::future::join_all`. Preserve compile-failure
short-circuit, preserve verdict order in the output report, and emit a
`group_id` field on `RuntimeEvent::Gate*` so consumers can re-correlate
interleaved log lines.

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_14](../ISSUE-TRACKER.md#perf_14)
- Plan: `tmp/solutions/perf/implementation/12-parallel-gate-rungs.md`
- Bottleneck: B08 third leg (BOTTLENECK-ANALYSIS.md §B08)
- Performance contract: **C-13** (compile + fmt run in parallel)
- Priority: P2
- Effort: ≈3 h
- Depends on: none (composes orthogonally with PERF_12 + PERF_13)
- Wave: 1

## Problem

`GateService::run_gates` iterates over rungs sequentially even when
two rungs operate on disjoint state. The whitelisted safe pairs
(verified in `BENCHMARK-RESULTS.md` §6 and AP-GATE-3):

- **`compile` + `fmt`** — disjoint workspaces.
- **`compile` + `format-check`** — same.
- **`clippy` + `fmt`** — same.
- **`clippy` + `format-check`** — same.

Unsafe pairs that the bottleneck doc warns against:

- **`compile` + `clippy`** — same `target/` lock.
- **`compile` + `test`** — test rebuilds on top of half-finished
  check artefacts.

## Exact Changes

### Step 1 — Add `parallel_safe_pair` and `build_parallel_groups`

`crates/roko-gate/src/gate_service.rs`. Add as private helpers near
the top:

```rust
/// Whitelist of gate-name pairs that are safe to run concurrently.
/// Gate names that contend for the cargo workspace lock (compile,
/// clippy, test) must NEVER be paired (AP-GATE-3).
fn parallel_safe_pair(a: &str, b: &str) -> bool {
    let pair = if a < b { (a, b) } else { (b, a) };
    matches!(
        pair,
        ("compile", "fmt")
        | ("compile", "format-check")
        | ("clippy", "fmt")
        | ("clippy", "format-check")
    )
}

/// Greedy grouping: walk gate names in order; for each name, attempt
/// to add to an existing group whose every member is parallel-safe
/// with it. Otherwise start a new group.
///
/// Output preserves the input order in the sense that gates appearing
/// earlier in `names` end up in earlier groups, but the in-group order
/// is determined by the sweep.
fn build_parallel_groups(names: &[String]) -> Vec<Vec<String>> {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut taken: std::collections::HashSet<String> = Default::default();
    for (i, name) in names.iter().enumerate() {
        if taken.contains(name) { continue; }
        let mut group = vec![name.clone()];
        for other in names.iter().skip(i + 1) {
            if taken.contains(other) { continue; }
            if group.iter().all(|g| parallel_safe_pair(g, other)) {
                group.push(other.clone());
                taken.insert(other.clone());
            }
        }
        taken.insert(name.clone());
        groups.push(group);
    }
    groups
}
```

### Step 2 — Extract `run_one_gate(name, config)` helper

The existing per-gate body inside `run_gates` (the rung dispatch + the
`judge` intercept + the `custom`/shell-gate handling) becomes a
private async method:

```rust
async fn run_one_gate(
    &self,
    gate_name: &str,
    config: &GateConfig,
    shell_gates: &mut std::slice::Iter<'_, ShellGateCommand>,
) -> Result<GateVerdict> {
    // ← move the body of the existing for-loop here, parameterised
    //   by `gate_name` and `config`. Return Result<GateVerdict>.
}
```

> The `shell_gates` iterator threading is awkward because `custom`
> gates pop from a shared iterator. Two options:
>
> 1. **Sort all `custom` gates into a single group at the end** so
>    they run sequentially (preserves the iterator semantics).
> 2. **Pre-bind shell-gate config to gate names** before the loop.
>
> Pick option 1 — it preserves existing semantics with minimal change.
> Filter `custom` gates out of the parallel-grouping pass; run them
> sequentially after the parallel groups complete.

### Step 3 — Restructure `run_gates`

```rust
async fn run_gates(&self, config: GateConfig) -> Result<GateReport> {
    use roko_runtime::pipeline_state::GateMode;

    // ← keep PERF_12's mode-resolution prelude unchanged ←

    let candidate_names = Self::ordered_gate_names(&config);
    let chosen_names: Vec<String> = match mode {
        GateMode::Full => candidate_names.clone(),
        GateMode::None => Vec::new(),
        GateMode::Express => candidate_names.iter()
            .filter(|n| EXPRESS_GATE_NAMES.contains(&n.as_str()))
            .cloned().collect(),
        GateMode::Auto => unreachable!("auto resolved above"),
    };

    // Split: custom gates run sequentially after the parallel groups.
    let (custom_names, parallel_candidates): (Vec<_>, Vec<_>) = chosen_names
        .into_iter()
        .partition(|n| n == "custom");

    let groups = build_parallel_groups(&parallel_candidates);

    let mut all_verdicts: Vec<GateVerdict> = Vec::new();

    // PERF_12 skipped-by-mode verdicts come first, unchanged.
    // ← keep that loop here ←

    let mut compile_failed = false;

    for (group_id, group) in groups.iter().enumerate() {
        // Compile-failure short-circuit: skip dependents in *this and
        // later* groups that depend on compile (test, clippy).
        if compile_failed {
            for n in group {
                if n == "test" || n == "clippy" {
                    all_verdicts.push(skipped_gate_verdict(
                        n.clone(),
                        "Skipped: compile failed in earlier rung",
                        "compile-failed-dependency",
                    ));
                }
            }
            continue;
        }

        if group.len() == 1 {
            let v = self.run_one_gate(&group[0], &config,
                &mut config.shell_gates.iter()).await?;
            if group[0] == "compile" && !v.passed { compile_failed = true; }
            self.emit_gate_event_with_group(&v, group_id as u32);
            all_verdicts.push(v);
        } else {
            // Concurrent group.
            let futures: Vec<_> = group.iter().map(|n| {
                let cfg = config.clone();
                let name = n.clone();
                async move {
                    // shell_gates iterator is unused for non-custom gates;
                    // pass an empty iter via slice::Iter on a static empty.
                    let empty: &[ShellGateCommand] = &[];
                    let mut it = empty.iter();
                    let v = self.run_one_gate(&name, &cfg, &mut it).await;
                    (name, v)
                }
            }).collect();
            let mut results = futures::future::join_all(futures).await;

            // Re-sort by rung order to keep the report deterministic.
            results.sort_by_key(|(n, _)| Self::rung_for_name(n).unwrap_or(u8::MAX));

            for (n, r) in results {
                let v = r?;
                if n == "compile" && !v.passed { compile_failed = true; }
                self.emit_gate_event_with_group(&v, group_id as u32);
                all_verdicts.push(v);
            }
        }
    }

    // Custom gates run last, sequentially.
    let custom_group_id = groups.len() as u32;
    let mut shell_gates = config.shell_gates.iter();
    for name in &custom_names {
        if compile_failed {
            all_verdicts.push(skipped_gate_verdict(
                name.clone(),
                "Skipped: compile failed in earlier rung",
                "compile-failed-dependency",
            ));
            continue;
        }
        let v = self.run_one_gate(name, &config, &mut shell_gates).await?;
        self.emit_gate_event_with_group(&v, custom_group_id);
        all_verdicts.push(v);
    }

    Ok(GateReport { verdicts: all_verdicts, /* unchanged fields */ })
}

fn emit_gate_event_with_group(&self, verdict: &GateVerdict, group_id: u32) {
    use crate::event_bus::emit_runtime_event;
    if verdict.passed {
        emit_runtime_event(roko_core::RuntimeEvent::GatePassed {
            run_id: self.run_id.clone(),
            gate_name: verdict.gate_name.clone(),
            duration_ms: verdict.duration_ms,
            #[cfg(feature = "perf-runner")] group_id,    // see Step 4
        });
    } else {
        emit_runtime_event(roko_core::RuntimeEvent::GateFailed {
            run_id: self.run_id.clone(),
            gate_name: verdict.gate_name.clone(),
            reason: verdict.output.clone(),
            #[cfg(feature = "perf-runner")] group_id,
        });
    }
}
```

> `futures::future::join_all` is the right primitive here because
> group arity is dynamic (1, 2, or 3). `tokio::join!` is fixed-arity
> macro. The Vec allocation cost is negligible compared to
> per-gate runtime.

### Step 4 — Add `group_id: u32` to `RuntimeEvent::Gate*`

`crates/roko-core/src/runtime_event.rs` (or wherever `RuntimeEvent` is
defined — search `rg -n 'enum RuntimeEvent' crates/`). Add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeEvent {
    // ... existing variants ...

    GatePassed {
        run_id: String,
        gate_name: String,
        duration_ms: u64,
        /// Parallel group id within a single gate phase. Gates with
        /// the same `group_id` ran concurrently. Defaults to 0 for
        /// backward compatibility with old logs.
        #[serde(default)]
        group_id: u32,
    },
    GateFailed {
        run_id: String,
        gate_name: String,
        reason: String,
        #[serde(default)]
        group_id: u32,
    },
    GateStarted {
        run_id: String,
        gate_name: String,
        #[serde(default)]
        group_id: u32,
    },
    // ...
}
```

> The `#[serde(default)]` is mandatory: existing `runtime-events.jsonl`
> files lack the field; readers must keep working (R-4 in
> `00-RULES.md`).

Then **drop the `#[cfg(feature = "perf-runner")]` guard from Step 3** —
it was scaffolding to make the field optional during development. Once
the field is in the enum unconditionally, the guard is unnecessary.

### Step 5 — Tests

Append to `crates/roko-gate/src/gate_service.rs`:

```rust
#[cfg(test)]
mod parallel_groups_tests {
    use super::*;

    #[test]
    fn build_parallel_groups_coalesces_safe_pairs() {
        let names = vec!["compile".into(), "fmt".into(),
                          "test".into(), "format-check".into()];
        let groups = build_parallel_groups(&names);
        // Group 0 contains compile + fmt + format-check (all
        // pair-safe). Group 1 contains test (depends on compile, no
        // safe pair).
        let g0_set: std::collections::HashSet<_> = groups[0].iter().cloned().collect();
        assert!(g0_set.contains("compile"));
        assert!(g0_set.contains("fmt"));
        assert!(g0_set.contains("format-check"));
        assert!(groups[1].contains(&"test".to_string()));
    }

    #[test]
    fn parallel_safe_pair_rejects_compile_clippy() {
        assert!(!parallel_safe_pair("compile", "clippy"));
        assert!(!parallel_safe_pair("compile", "test"));
        assert!(!parallel_safe_pair("clippy", "test"));
    }

    #[tokio::test]
    async fn compile_and_fmt_run_concurrently() {
        // Mock gates that sleep; sum of sleeps would dominate if serial.
        // (Actual implementation requires injecting mocks into
        // GateService — adapt to your existing test patterns.)
        // ...
    }

    #[tokio::test]
    async fn compile_failure_skips_dependents_in_next_group() {
        // (Adapt similarly: arrange a gate config where compile fails
        // and test/clippy are configured; assert the latter emit
        // skipped: true with reason "compile-failed-dependency".)
        // ...
    }
}
```

The wall-clock parallelism test typically requires a `MockVerify` that
sleeps. If your existing `GateService` tests have a mock harness,
follow the same pattern; if not, the structural tests
(`build_parallel_groups_coalesces_safe_pairs`,
`parallel_safe_pair_rejects_compile_clippy`) are sufficient for this
batch — file a follow-up to add the wall-clock test once a mock
harness exists.

## Write Scope

- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-core/src/runtime_event.rs` (for `group_id` field)

## Read-Only Context

- `crates/roko-gate/src/compile.rs`
- `crates/roko-gate/src/format_check_gate.rs`
- `crates/roko-gate/src/clippy_gate.rs`
- `crates/roko-runtime/src/event_bus.rs`
- `tmp/solutions/perf/implementation/12-parallel-gate-rungs.md`

## Acceptance Criteria

- [ ] `parallel_safe_pair` whitelist covers ONLY `(compile, fmt)`, `(compile, format-check)`, `(clippy, fmt)`, `(clippy, format-check)`.
- [ ] `build_parallel_groups(names) -> Vec<Vec<String>>` greedy grouping implemented.
- [ ] `run_one_gate(name, config, shell_gates_iter)` extracted from the old loop body.
- [ ] `run_gates` partitions `custom` gates and runs them sequentially after the parallel groups.
- [ ] `run_gates` uses `futures::future::join_all` for groups of size > 1.
- [ ] Compile-failure short-circuit preserved; downstream gates emit `skip_reason = "compile-failed-dependency"`.
- [ ] Verdict order in output matches rung order (`results.sort_by_key(rung_for_name)`).
- [ ] `RuntimeEvent::Gate{Started,Passed,Failed}` carry `group_id: u32` (additive serde field with `#[serde(default)]`).
- [ ] Structural tests `build_parallel_groups_coalesces_safe_pairs` and `parallel_safe_pair_rejects_compile_clippy` pass.

## Verify

```bash
# parallel_safe_pair must NOT include compile+clippy or compile+test:
rg -n 'parallel_safe_pair' crates/roko-gate/src/gate_service.rs

# group_id field present and serde-default:
rg -nU --multiline '"group_id"|group_id: u32' crates/roko-core/src/runtime_event.rs

# Old (sequential) loop body should be inside run_one_gate:
rg -n 'fn run_one_gate' crates/roko-gate/src/gate_service.rs
```

## Do NOT

- Do NOT parallelise compile+clippy or compile+test (AP-GATE-3). They
  contend for the same workspace lock; net win = 0.
- Do NOT parallelise more than 3 gates concurrently. OS scheduler
  thrashing dominates beyond a small group.
- Do NOT silently re-order verdicts in the output report. Re-sort by
  rung after the parallel join (the snippet does).
- Do NOT fan out to a `tokio::task::JoinSet`. JoinSet consumes futures
  eagerly; we want `join_all`'s simpler semantics.
- Do NOT remove the compile-failure short-circuit (AP-GATE-5). Failures
  invalidate test/clippy results; running them anyway wastes 10+ s
  per failed run.
- Do NOT touch the adaptive-threshold logic (already orthogonal —
  AP-GATE-2).
- Do NOT special-case Cargo workspace versus single-crate. The pair
  whitelist is safe in both because `fmt` truly doesn't touch `target/`.
- Do NOT skip the `#[serde(default)]` on `group_id`. Old log readers
  must keep parsing.
- Do NOT compile or run tests during the batch (see `00-RULES.md`).

## Tracker update

```
tracker: PERF_14 done <commit-sha>
```
