+++
id = "gap-0714c4"
kind = "gap"
title = "Codex's operation broker enforces no path confinement on reads, so an agent can read the audit vault outside its sandboxed workspace"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-agent/exec", "roko-agent/provider"]
created = 2026-10-03
updated = 2026-10-05
last_verified = 2026-10-05
last_verified_rev = "19f76451c"
source = "wave-5 follow-up reports 2026-10-02 (PK58 gap-f0a7ee)"
discovered_from = "gap-f0a7ee (backlog task 7119, step 4, deliberately deferred)"
anchors = ["crates/roko-agent/src/provider/claude_cli.rs::codex_sandbox_args", "crates/roko-agent/src/exec.rs::CodexOperationPolicy"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-baab0a", "gap-843aef"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'workspace-write does not confine reads' crates/roko-agent/src/ && grep -rq 'audit vault' crates/roko-agent/src/exec.rs crates/roko-agent/src/provider/claude_cli.rs"

[closed]
at = 2026-10-05
at_ts = "2026-10-05T15:52:41Z"
commit = "19f76451c"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-05T09:03:15Z"
forced = false
evidence = "Gate 19 (merged 19f76451c): its verify passes. The docs state that Codex's workspace-write sandbox does not confine reads; the broker's refuse_key_file_in_command check now has a test (codex_agent_is_stopped_at_a_command_that_reads_the_audit_vault). Deny-read support is q-c423b6 (Will)."
+++

## Problem

Backlog task 7119 ("Shell reads and Claude CLI tools refuse the audit vault", S05 task 6 second half, delivered
by `gap-f0a7ee`) added vault-read denial for roko's own bash tool (`refuse_secret_reads`) and for Claude Code
(`--settings` `Read`/`Edit` deny rules). Its Plan step 4 was explicitly deferred: "Codex: write down what its
broker lets the agent read outside the workspace; change nothing here." Nobody has written that down, and having
now read the actual code, the honest answer is: **nothing roko builds for Codex restricts reads by path at all,
in either of Codex's two run modes.**

- Normal mode: `codex_sandbox_args` (`crates/roko-agent/src/provider/claude_cli.rs:331-366`) passes
  `--sandbox workspace-write` plus `--add-dir <root>` for each of `codex_writable_roots`'s entries (Cargo's target
  dir and `CARGO_HOME`). Per its own doc comment and the `-c sandbox_workspace_write.network_access=...` override
  it also sets, this sandbox mode's name and its only configurable knobs (`writable_roots`, `network_access`)
  govern **writes** and **network**, not reads. codex-cli's `workspace-write` sandbox does not deny file reads
  outside the workspace at all (checked against codex-cli 0.152.0, the version `codex_writable_roots`'s own doc
  comment is checked against).
- Bypassed mode (`skip_permissions && level.allows_sandbox_bypass()`, i.e. `SandboxLevel::None`/`Observe`):
  `--dangerously-bypass-approvals-and-sandbox` removes Codex's OS sandbox entirely — same conclusion, more
  directly (this half is also documented by `gap-248670`, filed in an earlier batch, for the network side of the
  same bypass; reads were out of that item's scope).
- Separately, `CodexOperationPolicy`/`CodexStreamBroker` (`crates/roko-agent/src/exec.rs:118-260`, the only thing
  anyone would reasonably call "Codex's broker") is **not** a path-based access-control layer at all: it is a
  coarse, reactive gate over four operation *categories* (`CommandExecution`, `FileChange`, `WebSearch`,
  `McpToolCall`), checked by scanning Codex's own JSONL progress stream *after* each operation starts — its own
  doc comment says plainly "bounds what the operation can do but cannot prevent it from starting." It has no
  concept of "this read targets the audit vault" to deny.

So Codex has strictly weaker vault confinement than the other two paths 7119 hardened: bash gets
`refuse_secret_reads`, Claude Code gets explicit `Read`/`Edit` deny rules, Codex gets nothing analogous to either,
in its default *or* bypassed sandbox mode.

## Why it matters

7119's own Why-it-matters: "Most Graph runs execute through Claude Code; without these rules a suite file is one
`Read` away" — Codex is a smaller but real fraction of runs, and for those runs the audit vault (which "must be
unreadable to agents" per the same item) is fully readable today, by design of codex-cli's own sandbox modes, not
by any oversight specific to roko's wiring of them. This is the exact class of risk 7119's own Notes already names
for the *general* same-uid case ("A same-uid process can still read the vault without an OS sandbox... canaries
(7117) are the detection layer") — but Codex gets *no* active deny attempt at all, unlike bash and Claude Code,
which at least try before that same-uid ceiling could theoretically be worked around.

## Where

- `crates/roko-agent/src/provider/claude_cli.rs::codex_sandbox_args` (lines 331-366) and `codex_writable_roots`
  (368-396): builds Codex's `--sandbox workspace-write` / `--add-dir` / bypass arguments. No read-path flag exists
  here or, as far as this item's research found, in codex-cli 0.152.0's own `--sandbox` option set.
- `crates/roko-agent/src/exec.rs::CodexOperationPolicy` / `CodexStreamBroker` (lines 118-260 and around 475-535):
  the actual "operation broker" — category-level, reactive, not path-aware.
- Compare: `crates/roko-agent/src/claude_cli_agent.rs::key_file_deny_rules`/`build_settings_json` (Claude's
  path-level deny rules, extended to the vault by 7119) and `crates/roko-std/src/tool/builtin/sandbox/reads.rs::refuse_secret_reads`
  (bash's path-level deny rules, likewise extended) — the two mechanisms Codex has no equivalent of.
- Related, not duplicate: `gap-baab0a` (done; Codex's *tool policy*, i.e. this same operation-category broker,
  for a different purpose) and `gap-843aef` (done; Codex's sandbox-bypass-under-skip-permissions, the network
  side of the same bypass mode).

## Current state

Undocumented and unmitigated. 7119 (done) explicitly left this step unwritten; `gap-f0a7ee`'s own Progress notes
say so.

## Plan

1. At minimum (what 7119 step 4 actually asked for): document this plainly, next to `codex_sandbox_args` and in
   7119/S05's own "Current state" lineage, so the limitation is discoverable instead of silently absent. Suggested
   wording: "`--sandbox workspace-write` confines Codex's writes and (optionally) its network, never its reads;
   a Codex-run agent can read the audit vault and any other same-uid-readable file regardless of sandbox level."
2. If Will decides this residual risk is not acceptable for Codex specifically (unlike the general same-uid case,
   Codex gets zero active deterrent, not just a same-uid-bypassable one): investigate whether codex-cli exposes
   any read-side sandbox option in a newer version, or whether roko can add its own pre-exec check (e.g. refuse to
   launch a Codex agent at all when `[audit] enabled` and the workspace's `ROKO_AUDIT_HOME` is readable without
   an OS-level read sandbox) as a stronger mitigation than documentation alone.
3. Either way, 7117's canary scanner remains the detection backstop, as it already is for bash and Claude Code's
   theoretical same-uid bypass.

## Done when

- The limitation is written down somewhere a reader of `codex_sandbox_args` or 7119's lineage will actually find
  it (a doc comment is sufficient to close this item; real prevention is a stretch goal pending Will's decision).
- The `[[verify]]` command passes.

## Notes

- This is a "write it down" task by design (7119's own step 4 says "change nothing here") — do not expand scope
  into hardening Codex's sandbox without checking with Will first, per the same reasoning `gap-3cfe4f` (an earlier
  batch, the ViabilityBench keychain finding) used for an analogous same-uid risk.
- Filed as a new item rather than a note because 7119 itself is closed (`status = "done"`) and task 7119's own
  file, while still open in `tmp/backlog/`, is not part of `work/`'s tracked-and-rendered graph the way `gap-f0a7ee`
  (its closed package) is.

## Progress

- 2026-10-05 (w4-length): implemented on `work/gap-d10a97` at 5ed17eb8a; cargo verification deferred to the
  batch gate.
  - `codex_sandbox_args`' doc now says that workspace-write does not confine reads, so a Codex run can read the
    audit vault at every sandbox level. `guarded_command_violation`'s doc names the audit vault and says what the
    broker's check misses.
  - Premise, corrected: Codex is not unguarded. `ExecAgent`'s broker already runs roko-std's
    `refuse_key_file_in_command`, which refuses a command that names the vault (`ROKO_AUDIT_HOME`,
    `~/.roko/audit`) since 7119, on every `command_execution` Codex starts, with or without a policy. The check
    acts after the command has started and goes by the command's text. Nothing pinned it, so this item adds
    `codex_agent_is_stopped_at_a_command_that_reads_the_audit_vault`.
  - A stronger mitigation, described here and not added: the codex-cli 0.152.0 binary carries deny-read support.
    Its strings include `permissions.filesystem.deny_read` (requirements-level, managed config), deny-read glob
    entries in permission profiles, and a Seatbelt `(deny file-read* (regex ...))`. The user-level syntax is
    unverified here. Wiring it means checking it with a local codex sandbox run (no model call needed) and Will's
    go-ahead, per this item's notes.
  - Not edited: 7119's task file under `tmp/backlog/`, which this package does not own.
