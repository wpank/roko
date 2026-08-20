# 133 — `dangerously_skip_permissions` Must Be Opt-In

**Priority**: P1 — Every plan run today bypasses CLI-level sandbox protections by default because `dangerously_skip_permissions: true` is set unconditionally in `commands/plan.rs`; this is a security posture regression that should require explicit operator consent.
**Size**: XS (1-2 hours)
**Crates**: `crates/roko-cli/src/commands/do_cmd.rs` or `crates/roko-cli/src/commands/plan.rs`
**Depends on**: None
**Sources**: `tmp/backlog/_mori-diffs-gaps.md` §B-3 (suggested 115)

---

## Background

The `RunConfig` struct has a `dangerously_skip_permissions: bool` field that, when true, bypasses the CLI-level permission sandbox for spawned agent processes. This flag is intended for advanced use cases where the operator explicitly accepts the risk of running agents without tool-use restrictions.

The problem is that `commands/plan.rs` (or `do_cmd.rs`) sets `dangerously_skip_permissions: true` unconditionally, meaning every `roko plan run` invocation bypasses sandbox protections regardless of whether the operator requested it. This is distinct from backlog #60 (Safety Dispatch Hardening), which addresses tool allowlists and role contracts; this item is specifically about the runner-level sandbox bypass being the default rather than an opt-in.

The fix is three lines: change the default to `false`, add a `--dangerously-skip-permissions` flag to the CLI, and emit a safety audit record when it is set.

## Current State

- `crates/roko-cli/src/commands/plan.rs` or `do_cmd.rs` — contains `dangerously_skip_permissions: true` in `RunConfig` construction.
- No CLI flag exists for the operator to opt in explicitly.
- No audit record is emitted when the flag is set.
- `RunConfig::default()` may also have `dangerously_skip_permissions: true` (needs inspection).

## Implementation Plan

1. **Change default to `false`**: In the `RunConfig` construction in `plan.rs`/`do_cmd.rs`, change `dangerously_skip_permissions: true` to `dangerously_skip_permissions: false` (or remove it if `RunConfig::default()` provides `false`).

2. **Change `RunConfig::default()`**: If `RunConfig` derives `Default` with `dangerously_skip_permissions: true`, change the field default to `false`.

3. **Add `--dangerously-skip-permissions` CLI flag**: Add a boolean flag to the `plan run` subcommand. When present, set `run_config.dangerously_skip_permissions = true`.

4. **Emit safety audit record**: When `dangerously_skip_permissions` is `true`, write a `RunnerEvent::SafetyAudit { kind: "dangerously_skip_permissions", set_by: "cli_flag", timestamp }` to `.roko/events.jsonl`. This is not a denial event (the flag is being honored) but an audit trail.

5. **Print a warning**: When the flag is set, print to stderr:
   ```
   WARNING: --dangerously-skip-permissions is set. Agent tool-use sandboxing is disabled for this run.
   ```

6. **Verify no regression**: Run the standard test suite with the flag unset. If any tests relied on `dangerously_skip_permissions: true` as the default, they need to be updated to pass the flag explicitly.

## Acceptance Criteria

1. `roko plan run plans/` without any flags runs with `dangerously_skip_permissions: false`.
2. `roko plan run plans/ --dangerously-skip-permissions` sets the flag to `true` with a warning printed.
3. An audit record is written to `.roko/events.jsonl` when the flag is active.
4. `cargo test --workspace` passes with the new default.
5. No existing functionality regresses (agents can still dispatch and run plans with the flag unset).

## Verification Checklist

- [ ] `grep -rn 'dangerously_skip_permissions.*true' crates/ --include='*.rs' | grep -v target/` returns only the explicit flag activation path (not a hardcoded default).
- [ ] Run `roko plan run <test-plan>` without the flag; verify execution completes successfully.
- [ ] Run with `--dangerously-skip-permissions`; verify warning is printed to stderr.
- [ ] Check `.roko/events.jsonl` after a flag-enabled run; verify audit record is present.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/commands/plan.rs` or `do_cmd.rs` | Change `dangerously_skip_permissions: true` to `false`; add CLI flag |
| `crates/roko-cli/src/runner/types.rs` | Change `RunConfig` default for the field to `false` |
| `crates/roko-cli/src/runner/event_loop.rs` | Emit safety audit event when flag is active |
