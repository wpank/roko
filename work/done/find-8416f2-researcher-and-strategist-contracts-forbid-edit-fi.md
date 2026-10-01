+++
id = "find-8416f2"
kind = "finding"
title = "Researcher and strategist contracts forbid edit_file but still allow write_file and bash"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-agent/safety"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "aec267cac"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e3-discovery"
anchors = ["crates/roko-agent/src/safety/contracts/researcher.yaml", "crates/roko-agent/src/safety/contracts/strategist.yaml"]
links = { depends_on = [], blocks = [], related = ["bug-f43cf8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c 'import json,sys; fb=lambda r:{t for g in json.load(open(\"crates/roko-agent/src/safety/contracts/%s.yaml\" % r))[\"governance\"] for t in g.get(\"ForbiddenTools\",[])}; sys.exit(0 if all({\"write_file\",\"bash\"} <= fb(r) for r in (\"researcher\",\"strategist\")) else 1)'"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T13:44:23Z"
by = "coordinator (session 7622b882)"
claimed_at = "2026-10-01T09:04:53Z"
forced = false
evidence = "Batch 20e gate on 1a8aad603, re-checked with the compile fixes (cf722c1be, bed29287d), tiers' rustfmt (8a6c932ce) and the run-index scrub fix (d972959bd) on 32fe02384; MAIN aec267cac has the same crates: check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/fs/gateway/graph/learn/neuro/serve/std; lib tests roko-cli 3305, roko-agent 2294, roko-core 1962, roko-learn 1213, roko-serve 991, roko-graph 478, roko-fs 260, roko-neuro 239, roko-std 229, roko-acp 199, roko-dreams 100, roko-gateway 42 all pass; extras: golden_path_suite 2/2, all eight canaries pass (secret_canary 11/11 and C2 2/2 after the scrub fix), worktree_task_diff 2/2, plan_run_config_flag 1/1, default_engine 1, bin 429, routing crash loop 10/10, bench driver 18; the item's JSON verify passes: researcher, strategist and architect contracts forbid every write tool and bash. Merged 5202f230d (work/find-8416f2 5883d375a)."
+++

`researcher.yaml` forbids only `edit_file` and `apply_patch`; `strategist.yaml` forbids `edit_file`, `multi_edit` and `apply_patch`. Both roles can still overwrite any file with `write_file` or change the tree through `bash`, so the list does not make them read-only.

Decide the intended policy. If these roles must not modify the workspace, forbid `write_file` and restrict `bash` (or scope writes to an output directory). If they may write, drop the partial list and close this item as wontfix with that reason.

## Notes

- Premise held at ea5fbe4b2: `researcher.yaml` forbade only `edit_file` and `apply_patch`, `strategist.yaml` `edit_file`, `multi_edit` and `apply_patch`; both allowed `write_file` and `bash`.
- Decided: both roles are read-only. Their contracts, and architect's (which lacked `notebook_edit`), forbid `edit_file`, `write_file`, `multi_edit`, `apply_patch`, `notebook_edit` and `bash`. The contract is the policy that applies: the safety layer refuses those tools on roko's dispatcher, and Claude CLI agents get them as `--disallowedTools` (Edit, Write, MultiEdit, NotebookEdit, Bash). The role menus in `tool_selector.rs` are not attached to a dispatcher (gap-48faa7).
- No read-only `bash`: `ForbiddenTools` cannot tell a read-only command from another, so the roles read through `read_file`, `grep`, `glob` and `ls` (the researcher prompt's `grep -rn` maps to the `grep` tool). Nothing legitimate breaks: no plan under `plans/` runs a task as either role; plan authoring and revision run the strategist through Claude CLI with Read, Grep and Glob alone and write the plan from its text; the strategist template that asks it to write a brief and a tasks file (`StrategistInput`'s write paths) is not used in production, only its role identity is.
- The strategist's menu drops `bash` to match its contract (`menus_stay_within_contracts` would fail otherwise); the pre-planner, which has no bundled contract, keeps it.
- Test: `read_only_role_contracts_forbid_writes_and_bash` (architect, researcher, strategist). The item's verify (the JSON check) passes.
- Checked without cargo: static review and nightly fmt only; roko-agent was not compiled.
- Implemented on `work/find-8416f2` at `ad877bdec`; cargo verification deferred to the batch check.
