+++
id = "find-8416f2"
kind = "finding"
title = "Researcher and strategist contracts forbid edit_file but still allow write_file and bash"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-agent/safety"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ea5fbe4b2"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e3-discovery"
anchors = ["crates/roko-agent/src/safety/contracts/researcher.yaml", "crates/roko-agent/src/safety/contracts/strategist.yaml"]
links = { depends_on = [], blocks = [], related = ["bug-f43cf8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c 'import json,sys; fb=lambda r:{t for g in json.load(open(\"crates/roko-agent/src/safety/contracts/%s.yaml\" % r))[\"governance\"] for t in g.get(\"ForbiddenTools\",[])}; sys.exit(0 if all({\"write_file\",\"bash\"} <= fb(r) for r in (\"researcher\",\"strategist\")) else 1)'"
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
