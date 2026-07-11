# CLI Command Routing Issues

## High

### Help text contradicts actual default
- `main.rs:1351`: Says "Graph Engine, default" but `--engine` default is `runner-v2`. Actively misleads users.

### `--no-replan` flag parsed but never wired to runner-v2
- `main.rs:278`: Global flag, used only in legacy `orchestrate.rs`. `RunConfig` has no corresponding field. Gate failures trigger replanning regardless.

### `--skip-validate` flag parsed but never wired to runner-v2
- `main.rs:282`: Same pattern. Flag is silently ignored.

### `max_gate_rung` formula produces 0 when `skip_tests=true` and `clippy_enabled=false`
- `commands/plan.rs:525-529`, `runner/types.rs:1427-1431`: Result is `max_gate_rung = 0`. Gate loop exits immediately → NO gate validation runs at all, not even compile.

### `dangerously_skip_permissions: true` hardcoded in `roko plan run`
- `commands/plan.rs:503`: Unconditional. No CLI flag to opt out. Runner always skips permissions.
- `runner/types.rs:1423`: Config-aware path exists but `plan.rs` overrides it.

## Medium

### `roko resume` engine change is unconditional
- `main.rs:2699`: Always uses `PlanEngine::RunnerV2`. If original run used `--engine graph`, resume switches engines silently. No engine choice persisted in snapshot.

### `roko run` silently re-routes to `cmd_do`
- `main.rs:2340-2356`: Without `--serve`/`--share`/`--max-retries`, dispatches to `cmd_do` (scope-resolver path) not `cmd_run` (workflow engine). Help text describes wrong pipeline.

### `--engine graph` discards all runner-v2 flags silently
- `commands/plan.rs:258-267`: `--approval`, `--max-retries`, `--max-tasks`, `--fresh`, `--force-resume` all accepted by parser but dropped without notice. Only `--resume-plan` gets a warning.

### `resolve_config_for_workdir` calls `std::process::exit(1)` inside Result-returning function
- `main.rs:2986-2993`: Bypasses error propagation, skips cleanup, makes function untestable.

### `legacy-runner-v2` feature flag gates nothing in source
- `Cargo.toml:15,20`: Zero `#[cfg(feature = "legacy-runner-v2")]` in `src/`. Only gates test files.
