+++
id = "find-8416f2"
kind = "finding"
title = "Researcher and strategist contracts forbid edit_file but still allow write_file and bash"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-agent/safety"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e3-discovery"
anchors = ["crates/roko-agent/src/safety/contracts/researcher.yaml", "crates/roko-agent/src/safety/contracts/strategist.yaml"]
links = { depends_on = [], blocks = [], related = ["bug-f43cf8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -q "write_file" crates/roko-agent/src/safety/contracts/researcher.yaml && grep -q "write_file" crates/roko-agent/src/safety/contracts/strategist.yaml'
+++

`researcher.yaml` forbids only `edit_file` and `apply_patch`; `strategist.yaml` forbids `edit_file`, `multi_edit` and `apply_patch`. Both roles can still overwrite any file with `write_file` or change the tree through `bash`, so the list does not make them read-only.

Decide the intended policy. If these roles must not modify the workspace, forbid `write_file` and restrict `bash` (or scope writes to an output directory). If they may write, drop the partial list and close this item as wontfix with that reason.
