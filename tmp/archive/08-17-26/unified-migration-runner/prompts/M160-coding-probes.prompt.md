# M160 — Implement Coding T0 Probes

## Objective
Implement 6 coding-domain T0 probes in `roko-runtime` that evaluate code health without LLM calls. These probes (BuildHealth, TestRegression, ComplexityDrift, DependencyRisk, CoverageDelta, ErrorRate) read cached build/test output and return normalized f32 scores. Register all 6 in the ProbeRegistry (from M143). Each probe returns a value in [0.0, 1.0] where 0.0 = healthy and 1.0 = critical.

## Scope
- Crates: `roko-runtime`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat_probes.rs` (add coding probes)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/lib.rs` (re-export)
- Depth doc: `tmp/unified-depth/09-technical-analysis/` (coding probes)

## Steps
1. Read existing probe implementations for style reference:
   ```bash
   grep -n 'pub struct.*Probe\|impl.*Probe\|fn evaluate' /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat_probes.rs | head -20
   ```

2. Read the EngineState to understand available inputs:
   ```bash
   grep -A 30 'pub struct EngineState' /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat_probes.rs
   ```

3. Read what coding probes already exist:
   ```bash
   grep -rn 'coding\|BuildHealth\|TestRegression\|ComplexityDrift\|DependencyRisk\|CoverageDelta\|ErrorRate' /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/ --include='*.rs' | head -10
   grep -rn 'coding\|BuildHealth' /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/oracles/coding.rs | head -10
   ```

4. Implement `BuildHealth` probe:
   ```rust
   /// Detects build failures by parsing cached cargo stderr.
   ///
   /// Score: 0.0 = clean build, 0.5 = warnings only, 1.0 = errors present.
   /// Reads from `.roko/state/last_build.json` or equivalent cached state.
   pub struct BuildHealthProbe;

   impl BuildHealthProbe {
       pub fn evaluate_from_state(state: &EngineState) -> f32 {
           // Check last_build_errors and last_build_warnings from EngineState
           if state.build_errors > 0 { 1.0 }
           else if state.build_warnings > 0 { 0.5 * (state.build_warnings as f32 / 10.0).min(1.0) }
           else { 0.0 }
       }
   }
   ```

5. Implement `TestRegression` probe:
   ```rust
   /// Compares current test pass count against historical baseline.
   ///
   /// Score: 0.0 = no regression, 1.0 = significant regression.
   /// Formula: max(0, (baseline_passes - current_passes) / baseline_passes)
   pub struct TestRegressionProbe;
   ```

6. Implement `ComplexityDrift` probe:
   ```rust
   /// Tracks cyclomatic complexity delta from baseline.
   ///
   /// Score: 0.0 = no drift, 1.0 = complexity doubled.
   /// Formula: min(1.0, abs(current_complexity - baseline) / baseline)
   pub struct ComplexityDriftProbe;
   ```

7. Implement `DependencyRisk` probe:
   ```rust
   /// Counts known vulnerabilities from cargo audit cache.
   ///
   /// Score: 0.0 = no vulnerabilities, 1.0 = 5+ vulnerabilities.
   /// Formula: min(1.0, vulnerability_count / 5.0)
   pub struct DependencyRiskProbe;
   ```

8. Implement `CoverageDelta` probe:
   ```rust
   /// Tracks test coverage percentage delta.
   ///
   /// Score: 0.0 = coverage unchanged or improved, 1.0 = coverage dropped 20%+.
   /// Formula: max(0, (baseline_coverage - current_coverage) / 0.20)
   pub struct CoverageDeltaProbe;
   ```

9. Implement `ErrorRate` probe:
   ```rust
   /// Ratio of recent task failures to total tasks.
   ///
   /// Score: 0.0 = no failures, 1.0 = 50%+ failure rate.
   /// Formula: min(1.0, recent_failures / max(1, recent_total) / 0.50)
   pub struct ErrorRateProbe;
   ```

10. Register all 6 in the ProbeRegistry (or extend EngineState probes):
    ```rust
    pub fn coding_probes() -> Vec<Box<dyn Probe>> {
        vec![
            Box::new(BuildHealthProbe),
            Box::new(TestRegressionProbe),
            Box::new(ComplexityDriftProbe),
            Box::new(DependencyRiskProbe),
            Box::new(CoverageDeltaProbe),
            Box::new(ErrorRateProbe),
        ]
    }
    ```

11. Add necessary fields to `EngineState` if not present (build_errors, test_passes, etc.).

12. Write tests:
    - BuildHealth: 0 errors → 0.0, 1 error → 1.0, 5 warnings → 0.25
    - TestRegression: no regression → 0.0, half tests failing → 0.5
    - ErrorRate: 0 failures → 0.0, all failures → 1.0
    - All probes return values clamped to [0.0, 1.0]

## Verification
```bash
cargo check -p roko-runtime
cargo clippy -p roko-runtime --no-deps -- -D warnings
cargo test -p roko-runtime -- coding
cargo test -p roko-runtime -- probe
```

## What NOT to do
- Do NOT run cargo build/test/audit in the probes — read cached results only
- Do NOT add external dependencies for complexity analysis — use simple heuristics
- Do NOT make probes async — they are pure computation over cached state
- Do NOT add probes for other domains here — only coding domain
- Do NOT modify the Probe trait from M143 — implement it as defined
