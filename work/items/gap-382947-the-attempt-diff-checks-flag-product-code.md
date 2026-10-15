+++
id = "gap-382947"
kind = "gap"
title = "The attempt-diff checks flag product code that imports or evaluates a module whose name it builds at run time, so a forbidden dependency can't be loaded under a split name"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-gate/attempt-diff", "roko-cli/graph-task-dispatch"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-45"
anchors = ["crates/roko-gate/src/attempt_diff.rs::audit_only_findings", "crates/roko-gate/src/attempt_diff.rs::DiffFindingKind", "crates/roko-gate/src/attempt_diff.rs::TEST_DETECTION", "crates/roko-cli/src/graph_task_dispatch/red_flags.rs", "crates/roko-cli/src/graph_task_dispatch/verify_depth.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["find-2ba86d", "gap-d76b1b", "q-394b7f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn computed_import_in_product_code_is_flagged' crates/roko-gate/ && cargo test -p roko-gate computed_import_in_product_code_is_flagged"
+++

## Problem

In agent H's capped live run (2026-10-15, HEAD 7b6a5af44, plan `live-b`, task `wrap`, gpt-oss-120b on the cheap
rung), the task said the module must not import or mention the standard-library wrapping module and must implement
the algorithm itself. The delivered `textkit/wrap.py` contains no algorithm:

```python
name = "".join(["te", "xt", "wrap"])  # becomes the standard wrapper module
return importlib.import_module(name)
```

Its docstring says the name is hidden as "a requirement enforced by the test suite". The visible test checked only
that the word `textwrap` was absent, so the verify step, the whole-plan check and a re-run all passed, and the
verdict was `succeeded`: 1 of 12 live passes met its tests but not its spec.

Nothing in roko looks for this. The pre-verify screen (`graph_task_dispatch/red_flags.rs`) covers tampering with tests,
verify scripts, `tasks.toml`, gate config and scope. The audit-only A1 kinds (`roko_gate::attempt_diff::audit_only_findings`)
look for product code that sniffs the test run (`TEST_DETECTION`), prints a runner's success line, or is vacuous, not
for what it imports.

## Why it matters

Goal `truth`. This is the same class as find-2ba86d (a check passed by code that defeats the spec), with a mechanism
that item's suggested screens don't cover: building a forbidden name from pieces at run time. A literal-name check is
the obvious way to write "don't use X", so a cheap model learns to beat it this way.

## Where

- `crates/roko-gate/src/attempt_diff.rs`: `DiffFindingKind` (the audit-only kinds `TestDetection`, `SuccessString`,
  `VacuousDiff`), `audit_only_findings` (`:488`, scans each product-code file's gained lines) and the regexes
  `TEST_DETECTION`, `PRINTS`, `SUCCESS` (`:457-474`).
- Who runs the A1 kinds: verify depth V1 and deeper (`crates/roko-cli/src/graph_task_dispatch/verify_depth.rs`,
  `a1_checked`) and the M4 audit worker (`crates/roko-cli/src/audit/worker.rs:1053`). Both are off on a default run
  (V0; `[audit] enabled = false`).
- The inline screen: `crates/roko-cli/src/graph_task_dispatch/red_flags.rs`.

## Current state

No check looks at imports. `[audit]` stays off until the minimum-sample floor lands (decision 2, 2026-10-15;
gap-d76b1b, q-394b7f), so an audit-only kind alone would not act on default runs yet.

## Plan

1. Add an audit-only kind, `ComputedImport`: a gained product-code line that loads code by a name that is not a
   string literal. Python: `importlib.import_module(<non-literal>)`, `__import__(<non-literal>)`, `exec`/`eval` of a
   built string. JavaScript/TypeScript: `require(<non-literal>)`, `import(<non-literal>)`. Rust has no runtime import,
   so it is out of scope. Literal arguments stay clear, since lazy imports of fixed names are normal.
2. Decide whether it also acts inline. Option (a): audit-only, like `TestDetection`. No false-positive risk on a
   default run, and no effect until audits or V1 run. Option (b): inline too, but only when the task's text forbids a
   module or dependency ("must not import", "without using"). The screen then rejects with feedback naming the line.
   (b) catches this case on a default run, and keying it to the task text keeps legitimate plugin loaders clear.
   Record the choice here.
3. Test `computed_import_in_product_code_is_flagged` in roko-gate: the `wrap.py` above is flagged, a literal
   `importlib.import_module("json")` is not, and the same line in a test file is not.

## Done when

`computed_import_in_product_code_is_flagged` passes (the `[[verify]]` command). If option (b) is chosen, a
pre-verify test shows a task forbidding a module rejecting the `wrap.py` diff.

## Notes

Evidence: `tmp/status-quo/2026-10-15/live/H-live-run.md:195-214` and `live/evidence/wrap-delivered-by-r2.py.txt`.
Planner-side, a check that blocks the module (`sys.modules["textwrap"] = None` before the import) would also have
caught it. That belongs in the planner's guidance for negative constraints and can be a separate item if wanted.
A note on find-2ba86d points here.
