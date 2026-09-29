# Backend Plan Service — End-to-End Review

**Date:** 2026-09-26  
**Task:** T10 — Confirm the whole plan is real, not just compiling  
**Scope:** `GET /api/plans`, `GET /api/plans/{id}`, `GET /api/plans/{id}/tasks`, plus the three A2 root causes from `tmp/portal-audit/01-FINDINGS.md`

---

## Verification method

This task runs inside `roko plan run`, which holds an exclusive workspace lock. Starting
`roko serve` is not possible — it acquires the same lock and would fail immediately. All
verification below is done via:

1. Code reading (citing exact file and line numbers)
2. `cargo test -p roko-serve --test plan_discovery` — 6 integration tests that drive the
   axum router in-process without binding a port
3. On-disk inspection of `plans/`

Live server checks that cannot be performed here are listed under
**Deferred: live-server checks** at the end.

---

## Root cause A2 — three sub-bugs, all fixed

### Sub-bug 1: Wrong root (`plans_dir` hardcoded to `.roko/plans`)

**Status: FIXED**

`routes/plans.rs:1601–1607`:

```rust
fn plans_dir(workdir: &std::path::Path) -> std::path::PathBuf {
    let top = workdir.join("plans");
    if top.is_dir() {
        return top;
    }
    workdir.join(".roko").join("plans")
}
```

This is the same dual-root probe the CLI uses: prefers `<workdir>/plans/` when it
exists as a directory, falls back to `<workdir>/.roko/plans`. Two inline unit tests at
lines 2047–2071 verify both branches.

**Evidence for real plans on disk:**

`plans/` contains 33+ subdirectories. Both `example-hello-world` and `demo-hello` exist
as real directory plans. `ls plans/example-hello-world/` shows `plan.md` and
`tasks.toml`. `ls plans/demo-hello/` shows `tasks.toml`.

### Sub-bug 2: Wrong shape (extension filter skipped every directory)

**Status: FIXED**

The old `list_plans` walked the filesystem and filtered by `.toml`/`.json` extension,
skipping every plan stored as a directory (the normal layout). The new handler delegates
entirely to the `CliRuntime` trait, bypassing that filter:

`routes/plans.rs:48–76` — `list_plans`:
```rust
let plans = state.runtime.list_plans(&state.workdir).await ...
```

`routes/plans.rs:84–110` — `get_plan`:
```rust
let dto = state.runtime.load_plan_summary(&state.workdir, &id).await ...
```

`routes/plans.rs:122–159` — `plan_tasks`:
```rust
let dto = state.runtime.load_plan_tasks(&state.workdir, &id).await ...
```

The `CliRuntime` trait in `runtime.rs:385–417` declares `list_plans`, `load_plan_summary`,
and `load_plan_tasks`. Directory-layout plans are no longer skipped.

### Sub-bug 3: Wrong schema (`RawPlan` flat parser, not `[meta]` + `[[task]]`)

**Status: FIXED**

The new implementation in `serve_runtime.rs` calls through to the CLI's native parsers:

- `load_plan_summary` (lines 369–391): calls `crate::plan::discover_plan_by_id(workdir, plan_id)` 
- `load_plan_tasks` (lines 393–444): calls
  1. `crate::plan::discover_plan_by_id(workdir, plan_id)` — finds the plan directory
  2. `crate::plan::tasks_path(&plan_info)` — resolves to `{plan_dir}/tasks.toml`
  3. `crate::task_parser::TasksFile::parse(&tasks_path)` — parses the canonical
     `[meta]` + `[[task]]` TOML format

`plans/demo-hello/tasks.toml` uses exactly this format (`[meta]` header, `[[task]]`
sections). The old `RawPlan { id, title, description, tasks }` flat parser is gone from
the hot path.

---

## Root cause A1 — plan execution still broken (residual)

**Status: NOT FIXED in this plan**

`routes/plans.rs:249–316` — `execute_plan` calls `runtime.run_plan()`.

`serve_runtime.rs:531` — `run_plan_on_local_runtime()` calls `crate::runner::run()`.

`serve_runtime.rs:566`:
```rust
#[allow(deprecated)] // Runner-v2 removed; this call now returns an error
let report = crate::runner::run(plans, &run_config, &state_hub, cancel).await?;
```

This is the same broken call site documented in `01-FINDINGS.md §A1`. The
`execute_plan` route returns `202 Accepted` then emits `PlanCompleted { success: false }`
when the deprecated runner bails.

Additionally, `execute_plan` uses a local `find_plan` helper (lines 254, 1456–1465) that
only probes flat `.json`/`.toml` files in `plans_dir`, not `{id}/tasks.toml`. So even if
the runner were fixed, directory plans cannot be found by the execute route.

**This plan's scope was A2 (plan listing/reading).** A1 (plan execution) is a separate
defect tracked in the gaps file (see below).

---

## Integration test results

```
Running tests/plan_discovery.rs (target/debug/deps/plan_discovery-bcf47c18e847e82c)

running 6 tests
test plan_tasks_returns_full_envelope_with_ids_roles_and_depends_on ... ok
test missing_plan_tasks_returns_404 ... ok
test missing_plan_returns_404_not_500 ... ok
test path_traversal_plan_id_returns_400 ... ok
test path_traversal_dotdot_segment_returns_400 ... ok
test list_plans_returns_directory_plan_with_correct_task_count ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
```

All 6 pass. The tests drive the real axum router via `tower::ServiceExt::oneshot` with a
stub `PlanDiscoveryRuntime` seeded with a fixture plan mirroring `plans/demo-hello/tasks.toml`.
They confirm:

| Test | What it proves |
|---|---|
| `list_plans_returns_directory_plan_with_correct_task_count` | Directory-layout plans are returned, `task_count`/`status` are correct |
| `plan_tasks_returns_full_envelope_with_ids_roles_and_depends_on` | `/tasks` returns `plan_id`, `task_count`, full `tasks[]` with `id`, `role`, `depends_on` |
| `missing_plan_returns_404_not_500` | Unknown plan id → 404, not 500 |
| `missing_plan_tasks_returns_404` | Same fix on the `/tasks` sub-route |
| `path_traversal_plan_id_returns_400` | URL-encoded `..%2F` → 400 |
| `path_traversal_dotdot_segment_returns_400` | Bare `..` id → 400 |

Note: the stub runtime bypasses real file I/O. The live-server checks below confirm the
full file-parse path.

---

## Deferred: live-server checks

These commands require a running server (`roko serve` on :6677). They cannot be run inside
a `roko plan run` session due to the exclusive workspace lock. Paste each after `roko serve`
starts in a separate terminal.

### Check 1 — `example-hello-world` and `demo-hello` appear in `GET /api/plans`

```bash
curl -s http://localhost:6677/api/plans | jq '[.[] | {id, title, task_count, status}]'
```

**Expected:** An array of 33+ objects. Both `example-hello-world` and `demo-hello` must
appear with non-zero `task_count`. This proves the dual-root probe and directory-traversal
fix work against the real filesystem.

### Check 2 — tasks of `demo-hello` match `plans/demo-hello/tasks.toml`

```bash
curl -s http://localhost:6677/api/plans/demo-hello/tasks | jq '{plan_id, task_count, tasks: [.tasks[] | {id, role, depends_on}]}'
```

**Expected:** `plan_id: "demo-hello"`, `task_count: 1`, a single task with `id: "DEMO-T01"`,
`role: "implementer"`, `depends_on: []`. Cross-check against the actual file:

```bash
cat plans/demo-hello/tasks.toml
```

This proves the `[meta]` + `[[task]]` parser is wired end-to-end, not just in the stub.

### Check 3 — nonexistent plan returns 404

```bash
curl -o /dev/null -w "%{http_code}" http://localhost:6677/api/plans/no-such-plan-xyzzy
```

**Expected:** `404`

### Check 4 — path traversal returns 400

```bash
curl -o /dev/null -w "%{http_code}" "http://localhost:6677/api/plans/..%2Fetc%2Fpasswd"
```

**Expected:** `400`

### Check 5 — execute still fails (A1 residual confirmation)

```bash
curl -s -X POST http://localhost:6677/api/plans/demo-hello/execute | jq .
```

**Expected:** `202` accepted, but the plan event stream should show
`PlanCompleted { success: false }` (or similar). This confirms A1 is still broken and
should be tracked as follow-up.

---

## Follow-up gaps

These items were discovered during this review and should be added to `.roko/GAPS.md`:

1. **A1 residual — `execute_plan` uses deprecated Runner-v2.** `serve_runtime.rs:566`
   calls `crate::runner::run()` which always bails. Fix requires wiring
   `run_plan_on_local_runtime` to the Graph engine (analogous to how `roko plan run`
   works via `crates/roko-cli/src/commands/plan.rs:2870`).

2. **A1 residual — `execute_plan`'s `find_plan` skips directory plans.**
   `routes/plans.rs:1456–1465` probes only `{id}.json` and `{id}.toml`. Even after
   fixing the runner, a directory-layout plan like `demo-hello` would return 404 from
   the execute route. The fix is to reuse `runtime.load_plan_summary()` or a
   `runtime.find_plan_path()` method.

3. **A3 not in scope.** The IPC bridge (`start_hub_ipc_server`) has zero production call
   sites; a CLI plan run is invisible to the portal's event stream. This is the
   remaining structural gap in `01-FINDINGS.md §A3`.

---

## Verdict

| Root cause | Status | Evidence |
|---|---|---|
| A2-1 Wrong root | **Fixed** | `routes/plans.rs:1601–1607` dual-root probe; unit tests at lines 2047–2071 |
| A2-2 Wrong shape | **Fixed** | `routes/plans.rs:48–159` delegates to `CliRuntime`; 6 integration tests pass |
| A2-3 Wrong schema | **Fixed** | `serve_runtime.rs:393–444` uses `crate::task_parser::TasksFile::parse` |
| A1 Execute broken | **Not fixed** | `serve_runtime.rs:566` still calls deprecated `runner::run()`; `find_plan` still skips directories |
| A3 CLI↔server bridge | **Not in scope** | Separate gap; `start_hub_ipc_server` remains unwired |

The listing and task-reading surfaces are correct. Plan execution remains broken.
Live-server confirmation of the listing fix is deferred to the operator commands above.
