# 12 — Parallel Gate Rungs (B08, third leg)

> Bottleneck: `GateService::run_gates` iterates over rungs sequentially
> even when two rungs operate on independent data and could run in
> parallel — most notably **compile + fmt-check** (rungs 0 and 4) and
> **clippy + format-check**.
>
> Target savings: 200–500 ms / standard run.
> Effort: ≈3 h. Risk: low–medium (subprocess scheduling, log
> interleaving).

---

## Goal & success criteria

After this change:

1. `GateService::run_gates` recognizes a small, hard-coded set of
   "parallel-safe" rung pairs and runs them via `tokio::join!`.
2. Failed compile still short-circuits subsequent gates that depend on
   compile output (test, clippy on warnings-as-errors).
3. The pipeline preserves verdict ordering by rung in the output
   `GateReport`.

Done when:

- A new test runs the full gate set and asserts compile/fmt verdicts
  arrive faster than the sum of their individual durations.
- `tracing::info!` confirms the pair ran concurrently (each gate logs
  its start/end timestamp).
- Macro-benchmark on standard workflow shows ≥150 ms improvement vs
  plan-11 baseline on a clean compile.

---

## Background

- Bottleneck source: `BOTTLENECK-ANALYSIS.md` §B08 #3,
  `OPTIMIZATION-PLAYBOOK.md` §9 #3.
- Gate ordering today is strictly sequential
  (`crates/roko-gate/src/gate_service.rs::run_gates`). Each gate is a
  subprocess (cargo) — running two cargo subprocesses concurrently is
  safe (cargo locks `target/.cargo-lock` per workspace).
- Important nuance: `cargo check`, `cargo clippy`, and `cargo test`
  share a build cache. Running `cargo check` and `cargo test`
  concurrently can cause the check process to invalidate the artifacts
  test depends on, then test rebuilds. Net effect: parallel saves
  *less* than the naive sum suggests.
- The safe pair is **`cargo check` + `cargo fmt --check`**: they touch
  completely disjoint state.

---

## Files to read first

| File | Why |
|---|---|
| `crates/roko-gate/src/gate_service.rs` | Edit site. |
| `crates/roko-gate/src/compile.rs`, `clippy_gate.rs`, `test_gate.rs` | Understand what each gate runs. |
| `crates/roko-gate/src/format_check_gate.rs` | The fmt gate adapter. |

---

## Code-level plan

### Step 1 — Define parallel-safe pairs

```rust
/// Gate names that are safe to run concurrently with their primary
/// neighbour. The boolean value is **true** when both gates are
/// independent (no shared file lock, no compilation cache contention).
fn parallel_safe_pair(a: &str, b: &str) -> bool {
    let pair = if a < b { (a, b) } else { (b, a) };
    matches!(pair,
        ("compile", "fmt") | ("compile", "format-check")
        | ("clippy", "fmt") | ("clippy", "format-check")
    )
}
```

### Step 2 — Restructure `run_gates`

Replace the linear loop with a "scheduled groups" pass:

```rust
async fn run_gates(&self, config: GateConfig) -> Result<GateReport> {
    let names = Self::ordered_gate_names(&config)
        .into_iter()
        .filter(|n| !self.should_skip_by_mode(n, config.gate_mode))   // plan 10
        .collect::<Vec<_>>();

    // Build groups: a Vec<Vec<String>> where each inner vec runs concurrently.
    let groups = build_parallel_groups(&names);

    let mut all_verdicts: Vec<GateVerdict> = Vec::new();
    let mut compile_failed = false;

    for group in groups {
        if compile_failed && group.iter().any(|n| n == "test" || n == "clippy") {
            // Compile failed — skip dependents for this group.
            for n in &group {
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
            let v = self.run_one_gate(&group[0], &config).await?;
            if group[0] == "compile" && !v.passed { compile_failed = true; }
            all_verdicts.push(v);
        } else {
            // Concurrent group.
            let futures = group.iter().map(|n| {
                let cfg = config.clone();
                let n = n.clone();
                async move { (n.clone(), self.run_one_gate(&n, &cfg).await) }
            });
            let mut results: Vec<(String, Result<GateVerdict>)> =
                futures::future::join_all(futures).await;
            // Re-sort by rung order to keep the report deterministic.
            results.sort_by_key(|(n, _)| Self::rung_for_name(n).unwrap_or(u8::MAX));
            for (n, r) in results {
                let v = r?;
                if n == "compile" && !v.passed { compile_failed = true; }
                all_verdicts.push(v);
            }
        }
    }

    Ok(GateReport { verdicts: all_verdicts, /* ... */ })
}

fn build_parallel_groups(names: &[String]) -> Vec<Vec<String>> {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut taken = std::collections::HashSet::new();
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

> **Why `futures::future::join_all` and not `tokio::join!`?** Group
> arity is dynamic (1, 2, or 3). `tokio::join!` is a fixed-arity
> macro; `join_all` handles variable lengths. The Vec allocation cost
> is negligible compared to the gate runtime.

### Step 3 — Refactor `run_one_gate`

Pull the per-gate body of the existing loop into a helper:

```rust
async fn run_one_gate(&self, gate_name: &str, config: &GateConfig) -> Result<GateVerdict> {
    // Existing logic from inside the for-loop in run_gates:
    // - skipped-gate verdicts for "judge"
    // - shell_gates iteration for "custom"
    // - rung-based dispatch
    // - call gate.verify(&signal, &ctx).await
    // ...
}
```

### Step 4 — Logging tweaks

Each gate logs its `gate_started` / `gate_finished` events. With
parallel runs, these will interleave in the JSONL log and CLI stderr.
Add a `group_id: u32` field to the events so consumers can re-group:

```rust
emit_runtime_event(RuntimeEvent::GateStarted {
    run_id: self.run_id.clone(),
    gate_name: name.into(),
    group_id,           // NEW: which parallel group this gate belongs to
});
```

The TUI dashboard (`crates/roko-cli/src/tui/views/dashboard_view.rs`)
should render parallel gates side-by-side. That's a follow-up ticket;
the JSON field is the contract.

---

## Step-by-step execution

1. `git checkout -b perf/12-parallel-gate-rungs`.
2. Add `parallel_safe_pair` and `build_parallel_groups` (Steps 1–2).
3. Refactor `run_gates` to use `run_one_gate` (Step 3).
4. Update events (Step 4) — schema bump; non-breaking (additive field).
5. Tests below.
6. Macro-benchmark.
7. PR `perf(gate): run independent gates concurrently (B08)`.

---

## Anti-patterns / things NOT to do

- **Do NOT parallelise compile + clippy.** They both invoke `cargo`
  and contend for the same workspace lock. The first one to acquire
  wins; the second waits. Net win: zero. Net cost: confusing logs.
- **Do NOT parallelise compile + test.** Same shared `target/` cache;
  test will rebuild on top of half-finished check artefacts.
- **Do NOT parallelise more than 3 gates concurrently.** Each gate is
  a subprocess; OS scheduler thrashing dominates beyond a small
  group.
- **Do NOT silently re-order verdicts in the output report.** The CLI
  / serve consumers may rely on rung-order. Re-sort by rung after the
  parallel join (the snippet above does).
- **Do NOT fan out to a `tokio::task::JoinSet`** for this. JoinSet
  consumes futures eagerly; we want the simpler `join_all` semantics.
- **Do NOT remove short-circuit on compile failure.** Compile failures
  invalidate test/clippy results; running them anyway wastes 10+
  seconds per failed run.
- **Do NOT touch the adaptive-threshold logic** in this plan. Adaptive
  skip already short-circuits per rung; it composes with parallelism
  trivially because a skipped rung is a no-op future.
- **Do NOT special-case Cargo workspace versus single-crate**
  detection inside the gate service. The pair list above is safe in
  both because `fmt` and `format-check` truly don't touch `target/`.

---

## Test plan

```rust
#[tokio::test]
async fn compile_and_fmt_run_in_parallel() {
    let svc = GateService::new();
    let cfg = GateConfig {
        workdir: small_rust_project(),
        enabled_gates: vec!["compile".into(), "fmt".into()],
        // ...
    };
    let start = Instant::now();
    let report = svc.run_gates(cfg).await.unwrap();
    let elapsed = start.elapsed();

    let compile_ms = report.verdicts.iter().find(|v| v.gate_name == "compile").unwrap().duration_ms;
    let fmt_ms = report.verdicts.iter().find(|v| v.gate_name == "fmt").unwrap().duration_ms;
    let sum = compile_ms + fmt_ms;
    assert!(elapsed.as_millis() < (sum as u128 * 9 / 10),
        "expected wall-time < 0.9 * (compile + fmt). got {}, sum {}",
        elapsed.as_millis(), sum);
}

#[tokio::test]
async fn compile_failure_skips_dependents_in_next_group() {
    let dir = small_rust_project_with_compile_error();
    let svc = GateService::new();
    let cfg = GateConfig {
        workdir: dir,
        enabled_gates: vec!["compile".into(), "fmt".into(), "test".into()],
        // ...
    };
    let report = svc.run_gates(cfg).await.unwrap();
    let compile = report.verdicts.iter().find(|v| v.gate_name == "compile").unwrap();
    assert!(!compile.passed);
    let test = report.verdicts.iter().find(|v| v.gate_name == "test").unwrap();
    assert!(test.skipped);
    assert_eq!(test.skip_reason.as_deref(), Some("compile-failed-dependency"));
}

#[test]
fn build_parallel_groups_coalesces_safe_pairs() {
    let names = vec!["compile".into(), "fmt".into(), "test".into(), "format-check".into()];
    let groups = build_parallel_groups(&names);
    // Group 0: compile + fmt + format-check (all pair-safe with each other).
    // Group 1: test (depends on compile).
    assert_eq!(groups.len(), 2);
    assert!(groups[0].contains(&"compile".to_string()));
    assert!(groups[0].contains(&"fmt".to_string()));
    assert!(groups[1].contains(&"test".to_string()));
}
```

Macro-benchmark: standard workflow with `--gates compile,fmt,test`.
Expect ≥150 ms improvement.

---

## Rollback plan

- A feature flag `[conductor.gate.parallel_pairs] = false` disables
  the grouping (falls back to sequential).
- `git revert` the change; the helper functions become dead code that
  still compiles.

---

## Status check (acceptance)

- [ ] `parallel_safe_pair` and `build_parallel_groups` implemented and
      tested.
- [ ] `run_gates` runs the configured groups; preserves rung order in
      output.
- [ ] Compile-failure short-circuit preserved.
- [ ] `RuntimeEvent::Gate*` carries `group_id` for log correlation.
- [ ] Macro-benchmark improvement ≥150 ms recorded.
