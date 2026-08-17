# Audit-2026-05-01 Runner Rules

## CRITICAL: Do NOT compile or run tests

**DO NOT run any of these commands:**
- `cargo check`, `cargo build`, `cargo test`, `cargo clippy`, `cargo run`
- `rustc`, `rustfmt`, `cargo fmt`
- Any compilation or test execution

**WHY:** Compilation runs in a separate validation pipeline AFTER the
batch is merged. Running cargo wastes significant time and resources.
Just write correct code and commit it. If you need to understand types
or signatures, READ the source files instead of compiling.

## Source plans

This runner implements the open items from
`tmp/subsystem-audits/implementation-plans/`. Every prompt cites a
section of one of those plan files. **Read the plan section before
implementing**; the prompt has the highlights but the plan has the
full context.

The 10 universal anti-patterns from
`tmp/subsystem-audits/implementation-plans/02-ANTI-PATTERNS.md` apply
to every batch. Carry forward verbatim.

## Universal Anti-Patterns (carry forward)

These are critical. **Violating any of them is an automatic reject**,
regardless of how clean the code looks.

1. **Skeletons ≠ migrations.** A new type / trait compiling does not
   mean the runtime uses it. Migration is only complete when:
   - The new type is constructed in the actual product path (not just tests).
   - The old type is removed (or feature-gated and routed only from tests).
   - A focused integration test asserts the old code path is unreachable.
2. **Unknown ≠ zero.** Missing usage / cost / context must remain
   `None` (`Option<u64>` / `Option<f64>` / `Option<RoutingContext>`).
   Never substitute `0`, `0.0`, or `Default::default()` for "I didn't
   get a value."
3. **No silent fallback.** Failed resolution / auth / capability /
   config load → typed error. Never:
   - Synthesize a default config and proceed.
   - Fall back to another provider/model "just to keep going."
   - Downgrade a `Rejected` to a `Skipped`.
   - Convert a `Failed` stream event to a `Completed` with empty content.
4. **Missing/invalid config → restricted.** When a contract YAML, safety
   profile, or auth config is missing or invalid, fewer permissions, not
   more. Production code paths use `AgentContract::restricted(role)`.
   Permissive fallback is test-only; mark `#[cfg(test)]`.
5. **No regex prompt scraping.** Use typed `CommandEvent` lifecycle
   events: `Started`, `Output`, `Exited`, `SpawnFailed`, `Cancelled`.
6. **No string-interpolated payloads.** `format!`-built TOML / JSON /
   SSE / wire payloads are wrong. Use `toml::to_string_pretty(&struct)`,
   `serde_json::to_string(&struct)`, `axum::Json(...)`, etc.
7. **No fifth dispatch path.** The codebase already has four LLM dispatch
   paths (`ModelCallService`, `DispatchResolver`, `dispatch_direct.rs`,
   route-local clients). Adding a fifth is forbidden. Extend
   `ModelCallService` or `DispatchResolver` instead.
8. **One item per commit.** Each commit/PR addresses one batch ID. If
   you discover a follow-up while working, **split it** — file a new
   batch, do not expand scope.
9. **No `unwrap()` / `panic!()` / `expect()` in changed code.** Existing
   `unwrap()`s in unrelated code stay. New or touched lines use typed
   errors (`?` propagation, `anyhow::Context`, custom `thiserror`).
10. **No unrelated edits.** Don't refactor neighbors, add comments,
    change formatting, or "improve" adjacent code. The diff for one
    batch touches only the files / lines required by that batch.

## Pre-deletion safety check (mandatory for any T2 deletion batch)

```bash
# 1. Confirm the symbol/module has no callers outside its own crate
rg '<crate_name>::<module_name>' crates/ -g '*.rs' | rg -v 'crates/<crate_name>/'

# 2. Confirm no test references
rg '<symbol>' crates/ -g '*.rs' tests/

# 3. Sanity: file presence as expected
ls crates/<path>/<file>.rs
```

If step 1 returns hits, the module is **not** dead — re-evaluate the
audit claim and report the discrepancy in the batch log before deleting.

## File-touch rules

- `scope = [...]` in `batches.toml` is the **write allowlist**. Do not
  modify any file outside this list.
- `also_read = [...]` is informational; you can read them, you cannot
  write them.
- If a batch needs to write a file not in `scope`, that's a sign the
  batch is misscoped — log it and stop. Do not silently expand scope.

## Test code is exempt from the new-code rules above

- `#[cfg(test)]`-gated code may use `unwrap()`, `expect()`, fixtures,
  helper construction patterns that production code can't.
- Bench / example code is **not** test code; it follows production rules.

## When in doubt

- Re-read the linked plan section in `tmp/subsystem-audits/implementation-plans/`.
- Re-read the relevant source file.
- If the prompt and plan disagree, the plan wins.
- If you cannot find a typed alternative to `unwrap()` or to a string-
  interpolated payload, **stop and report**, do not improvise.
