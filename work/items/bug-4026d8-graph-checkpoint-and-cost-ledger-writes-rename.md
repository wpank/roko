+++
id = "bug-4026d8"
kind = "bug"
title = "Graph checkpoint and cost-ledger writes rename an unsynced temp file: a power cut can leave checkpoint.json or costs.json empty and the plan unresumable"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_checkpoint"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 roko-recovery kill matrix (tmp/hermes-roko-v7/findings/roko-recovery.md, tmp/hermes-roko-v7/roko-recovery/runs/roko-main-4d22b94e8), the lead's ask"
discovered_from = "agent:roko-recovery (round v7, 4d22b94e8)"
anchors = ["crates/roko-cli/src/graph_checkpoint.rs::write_manifest_atomic", "crates/roko-cli/src/graph_checkpoint.rs::write_cost_ledger_atomic"]
links = { depends_on = [], blocks = [], related = ["gap-36f3fb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "awk '/^fn write_cost_ledger_atomic/,/^}/' crates/roko-cli/src/graph_checkpoint.rs | grep -qE 'sync_all|sync_data|synced' && awk '/^fn write_manifest_atomic/,/^}/' crates/roko-cli/src/graph_checkpoint.rs | grep -qE 'sync_all|sync_data|synced'"
+++

## Problem

A plan's Graph checkpoint manifest (`.roko/state/graph/<plan>/checkpoint.json`) and its cost ledger (`costs.json`)
are written by `fs::write` of a temp file followed by `rename`. Neither the file nor its directory is synced:
`graph_checkpoint.rs::write_manifest_atomic` and `::write_cost_ledger_atomic`.

A SIGKILL keeps the page cache, so roko's kill tests never see a problem. A real power loss is different: the
2026-10-26 reboot, or a laptop running out of battery. APFS can then persist the rename and lose the data, leaving an
empty or torn `checkpoint.json` or `costs.json`. The resume then fails to parse the checkpoint, and the only way on is
`--fresh`, which throws away every passed task.

Roko already syncs where it matters elsewhere:
- the workspace and runner lock files (`sync_data`);
- the worktree manager's journal (`orchestrator/worktree/git_ops.rs::write_new_synced_file`, `sync_directory`);
- serve's run-end index rows (`run_end::append`, `sync_data`).

The checkpoint's own atomic writes don't.

## Why it matters

Goal core. These two files are what a resume reads. The round-v7 kill matrix
(`tmp/hermes-roko-v7/findings/roko-recovery.md`) shows that resume works when the kill lands outside a task's flight.
That only holds if the files survive.

## Where

- `crates/roko-cli/src/graph_checkpoint.rs::write_manifest_atomic`
- `crates/roko-cli/src/graph_checkpoint.rs::write_cost_ledger_atomic`
- Check the run manifest's store as well (`roko-learn` `RunProvenanceManifest::store`) and the Activity log's appends
  (`roko-graph` `ActivityRecorder`), which the manifest's `committed` point already guards.

## Plan

`sync_all` the temp file before the `rename`, then sync the parent directory. Reuse `write_new_synced_file` and
`sync_directory`, or move them where both can use them. On macOS a plain `fsync` does not flush the drive's cache
(`F_FULLFSYNC` does): check what `sync_all` does on the toolchain in use.

## Done when

Both writers sync the file and the directory, and the `[[verify]]` passes.

## Notes

This can't be shown by SIGKILL; the fix is by inspection. Related: the roko-recovery p1 and gap-36f3fb.
