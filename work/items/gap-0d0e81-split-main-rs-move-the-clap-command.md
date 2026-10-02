+++
id = "gap-0d0e81"
kind = "gap"
title = "Split main.rs: move the clap command enums into their command modules"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/main", "roko-cli/commands"]
created = 2026-09-29
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "a788dfd8d"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e15"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W5-contention-parallelism.md (F2, F4, rec 2b)"
anchors = ["crates/roko-cli/src/main.rs", "crates/roko-cli/src/commands/"]
lane = "rust-hot"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["gap-c7c946"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qE 'enum (LearnCmd|PlanCmd|KnowledgeCmd|ConfigCmd|PrdCmd|JobCmd) [{]' crates/roko-cli/src/main.rs && test \"$(wc -l < crates/roko-cli/src/main.rs)\" -lt 3000 && cargo test -p roko-cli --bin roko"
+++

## Problem

`crates/roko-cli/src/main.rs` is 8,191 lines:
- 41 clap enums (lines 110–3238);
- `fn main` and its dispatch (3239–4538);
- about 3,650 lines of tests (4539–8191).

Adding or changing a subcommand therefore edits this hot file. Registrations such as S01.P0-13b (gap-c7c946) and
S04.T07b must wait for a freeze window, and W5 counts 23 open pieces of work queued on the file.

## Why it matters

Once the enums live next to their handlers, a new subcommand no longer touches a hot file, and BUILD-ORDER's rule that
registrations wait for the S01 hot-file pull request goes away (W5 rec 2b). It is part of epic spec-9a3131.

## Where

- **`crates/roko-cli/src/main.rs`:** the subcommand enums from `CacheCmd` to `ConfigMcpCmd`. `Cli` (line 335) and
  `Command` (line 419) stay.
- **`crates/roko-cli/src/commands/`:** the destination modules, such as `learn.rs`, `plan.rs`, `knowledge.rs`,
  `config_cmd.rs`, `prd.rs`, `research.rs`, `job.rs`, `bench.rs`, `backlog.rs`, `cache.rs`, `run_index.rs` and
  `tune.rs`. These modules belong to the `roko` binary, not the library.
- **New:** a sibling test file for main.rs's tests, such as `src/main_tests.rs`, included with `#[path]`.

## Current state

As above at `41c7ffbd6`. The env worktree `../roko-wt-env` (bug-7d7200) has uncommitted edits to
`load_startup_env_files` and to main.rs's tests.

## Plan

1. **Prepare.** Wait for bug-7d7200 to merge, then claim a short freeze on main.rs.
2. **Move the enums.** Move each subcommand enum into the module that handles it, as `pub(crate)`, with names, doc
   comments and clap attributes unchanged; `main.rs` imports them. An enum with no obvious module can stay.
3. **Move the tests.** Move main.rs's tests into the sibling file.
4. **Fix the docs.** Update the pointer in `CLAUDE.md` that places `PlanEngine` in main.rs.

## Done when

- [ ] `roko --help` and every subcommand's `--help` print the same text as before. Diff them, and put the result in
      the closing evidence.
- [ ] main.rs is under 3,000 lines and no longer defines the subcommand enums.
- [ ] The `[[verify]]` command passes.

## Notes

- **Hot file:** one writer, about half a day.
- **After the move:** gap-c7c946's `Telemetry` variant lands in `commands/learn.rs`, and that item touches no hot
  file.
- 2026-10-02 (wk-climain): implemented on work/bug-28c193 on top of the post-6d base (a788dfd8d); cargo
  verification deferred to the batch check. Done by a script, re-runnable if main.rs changes before the gate.
  - main.rs went from 8,852 to 2,787 lines. 34 subcommand enums moved, with doc comments and clap attributes
    unchanged and bare `enum` made `pub(crate)`:
    - backlog: BacklogCmd; bench: BenchCmd; cache: CacheCmd; job: JobCmd; mcp: ConfigMcpCmd; run_index: RunIndexCmd;
    - config_cmd: ConfigCmd, ConfigProviderCmd, ConfigModelCmd, ConfigSubscriptionCmd, PluginCmd;
    - knowledge: KnowledgeCmd, KnowledgeDreamCmd, KnowledgeCustodyCmd, KnowledgeSyncDirection, NeuroCmd,
      DreamCmdLegacy;
    - learn: LearnCmd, ExperimentsSubCmd, InspectSubsystem; plan: PlanEngine, PlanCmd, QueueCmd;
      prd: PrdCmd, PrdDraftCmd; research: ResearchBackend, SearchRecency, ResearchCmd;
    - server: DaemonCmd, DeployCmd; tune: TuneCmd, ConfigPresetCmd; util: IndexCmd, CompletionShell.
    `Cli`, `Command`, the global value enums and DemoCmd (dispatched in main.rs) stay. Their impl blocks stay in
    main.rs too, so their private methods keep their callers. main.rs imports the moved enums. The four modules
    that import from `crate` by name (cache, run_index, backlog, mcp) dropped their own enum from that list and
    import `clap::Subcommand`.
  - The test module moved to `src/main_tests.rs` (`#[cfg(test)] #[path = "main_tests.rs"] mod tests;`), one indent
    level out, leaving string-literal lines untouched. At the new indentation rustfmt would rejoin 11 statements and
    refill 3 `use` lists, and those edits are applied. A scan of the moved tests found the other multi-line
    constructs held by rustfmt's width heuristics, not by the 100-column limit.
  - Step 4's CLAUDE.md part (its pointer to `PlanEngine` in main.rs; it is now in `commands/plan.rs`) is deferred
    to bug-a804b1, the CLAUDE.md drift item that Will approves.
  - Still to do at the gate: diff `roko --help` and every subcommand's `--help` against the pre-split binary for the
    closing evidence (the attributes moved verbatim, so no change is expected), then run the cargo part of the
    verify.
