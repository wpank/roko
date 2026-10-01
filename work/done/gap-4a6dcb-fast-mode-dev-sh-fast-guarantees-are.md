+++
id = "gap-4a6dcb"
kind = "gap"
title = "FAST mode (dev.sh fast) guarantees are only partly ported to the Graph engine"
status = "done"
triage = "verified"
severity = "p2"
size = "M"
goal = "tooling"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "session:roko-b6 message 2026-09-28 (commit 725f21e05)"
discovered_from = "item:bug-f7943a"
anchors = ["crates/roko-cli/src/graph_execution/fast_lane.rs::arm_plan_deadline", "crates/roko-cli/src/graph_task_dispatch/turn_policy.rs::task_turn_limit", "crates/roko-cli/src/graph_task_dispatch/verification.rs::GraphTaskDispatcher::settle_task_verification", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch", "crates/roko-cli/src/runner/cargo_command.rs::cargo_profile_available", "dev.sh:225"]
links = { depends_on = [], blocks = [], related = ["bug-f7943a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn graph_fast_mode_bounds_dispatch' crates/roko-cli/src/ && grep -rqw 'fn graph_fast_mode_verify_uses_dev_fast_without_autofix' crates/roko-cli/src/ && cargo test -p roko-cli graph_fast_mode_"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:30Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "M"
claimed_at = "2026-10-01T16:12:57Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

`./dev.sh fast plans/<dir>` runs on the Graph engine (fixed by `bug-f7943a` in `725f21e05`), but only three
FAST guarantees reach the Graph path: bounded prompt context, the one-verify plan contract, and the run
deadline (`ROKO_FAST_PLAN_DEADLINE_SECS`). Five guarantees that FAST had under the deleted Runner-v2 loop do
nothing on `roko plan run` today:

1. the patch-only system prompt section (tell the provider to make the smallest patch, run no cargo/tests,
   and hand off);
2. the 90 s clamp on each task attempt and on agent silence;
3. the 6-turn agent cap (`ROKO_FAST_MAX_AGENT_TURNS`, default 6);
4. the `dev-fast` Cargo profile for gate/verify commands;
5. no compile auto-fix (`cargo fix`) after a failed verify.

So a FAST task can run the full tier turn cap (40-120 turns), the full `timeouts.agent_dispatch_secs`, let the
provider run Cargo itself, and trigger `cargo fix`, while CLAUDE.md (section "Opt-in FAST self-development")
still says "FAST tells the provider to hand off after patching, keeps Cargo out of the provider session".

## Why it matters

Goal `tooling`: FAST is the cheap, bounded loop for developing roko with roko. Without the turn, time and
prompt bounds, a "five-minute" FAST run is mostly bounded only by the plan deadline, which then kills the
run mid-task instead of each task finishing small. The docs overstate the guarantees, so operators trust
it for more than it does. Related: `bug-f7943a` (done: dev.sh fast now uses Graph).

## Where

- `crates/roko-cli/src/graph_execution/fast_lane.rs::arm_plan_deadline`: the only FAST code on the Graph
  path; armed at `graph_execution/plan_runner.rs:802`. Has a private `env_flag_enabled` helper.
- `crates/roko-cli/src/plan_policy.rs::PlanExecutionPolicy::for_environment`: FAST plan policy (max 4 tasks,
  bounded context), used by `dispatch/prompt_builder.rs::from_task` (`bounded_context_only`). Already works.
- `crates/roko-cli/src/graph_task_dispatch.rs`, `impl TaskDispatcher for GraphTaskDispatcher::dispatch`
  (starts about line 3175), the production path for every Graph task:
  - `task_turn_limit` (line 708) and its use at about line 3414: the turn cap (tier cap, express clamp);
  - `effective_timeout_secs` at about line 3626: `spec.timeout_secs` or `timeouts.agent_dispatch_secs`;
  - `AgentDispatchRequest { system_prompt: dispatch_plan.prompt.system_prompt.clone(), .. }` at about
    line 3642: where a FAST prompt section would be appended;
  - `settle_task_verification` (about line 1807): runs authored `[[task.verify]]` steps via
    `GatePayload::in_dir(..)`; the auto-fix block at about line 2061 is gated only by
    `self.config.gates.cargo_fix_enabled`.
- `dispatch_streaming` (about line 4127) duplicates the same logic but has no production caller.
- Runner-v2 leftovers that implement the missing pieces but are unreachable from Graph:
  `runner/agent_stream.rs::effective_agent_turn_limit` (line 703, reached only via
  `dispatch::spawn_streaming_cli_agent`, which has no callers); `runner/gate_dispatch.rs::run_gate_once`
  (lines 907-913: `dev-fast` profile, FAST verify contract; line 1315: no auto-fix in FAST);
  `runner/cargo_command.rs::{cargo_profile_available, cargo_command_with_profile}` (both `pub(super)`).
- `dev.sh` lines 225-227 (help text) and `CLAUDE.md` FAST paragraph: user-facing claims.
- `Cargo.toml` `[profile.dev-fast]` (line 297): exists.

## Current state

- Ported in `725f21e05`: the run deadline (`fast_lane.rs`), FAST plan policy and bounded prompt context.
- Missing: items 1-5 above. The Runner-v2 versions, for reference (from `git show
  6b5da8616^:crates/roko-cli/src/runner/event_loop.rs`):
  - prompt section appended to the system prompt: `## FAST implementation mode` / "Produce the smallest
    correct patch and hand off quickly. Do not run cargo, tests, clippy, npm, builds, or servers; the
    runner owns verification. Avoid broad refactors and unrelated edits. End with a structured summary of
    files changed, behavior implemented, and verification the runner should perform."
  - `agent_dispatch_timeout`: `configured.min(90 s)`; `fast_mode_deadline_policy`: `task_attempt` and
    `agent_silence` each `min(90 s)`. Gate deadlines were deliberately not shortened.
- The Graph path has no agent-silence watchdog at HEAD (no `silence` in `graph_task_dispatch.rs`); the
  attempt timeout is the only per-attempt time bound.
- `CLAUDE.md` now warns that FAST is partly ported (`4bc92ff7a`) but still lists the full feature set.
  dev.sh help says the other variables are "recorded in the evidence metadata" only.

## Plan

1. Add one FAST helper for the Graph path, e.g. in `graph_execution/fast_lane.rs`:
   `pub fn fast_mode_enabled() -> bool`, `fast_turn_cap(configured: u32) -> u32` (min with
   `ROKO_FAST_MAX_AGENT_TURNS`, default 6, ignore 0/invalid), `fast_attempt_timeout_secs(secs) -> u64`
   (min 90), and `FAST_PROMPT_SECTION: &str` (the text above).
2. In `GraphTaskDispatcher::dispatch`: when FAST, lower `max_turns` with `fast_turn_cap` after
   `task_turn_limit` (keep the raised-cap-after-turn-limit logic, but never above the FAST cap), clamp
   `effective_timeout_secs`, and append `FAST_PROMPT_SECTION` to the system prompt.
3. In `settle_task_verification`: when FAST, skip the auto-fix block, and run authored cargo verify
   commands with `--profile dev-fast` when `cargo_profile_available(workdir, "dev-fast")` (make the two
   `runner/cargo_command.rs` helpers `pub(crate)`; do not rewrite shell-composed commands, as
   `cargo_command_with_profile` already refuses them).
4. Silence clamp: the Graph path has no silence watchdog to clamp. Recommended: drop "silence" from the
   FAST claims and rely on the 90 s attempt timeout. Adding a watchdog is a separate feature.
5. Tests (in `graph_task_dispatch.rs` tests, reusing the fake-provider harness there, e.g.
   `make_test_dispatcher` and the `no_auto_fix` helper near line 5432): `graph_fast_mode_bounds_dispatch`
   (FAST on: request has `max_turns <= 6`, `timeout_ms <= 90_000`, system prompt contains
   "FAST implementation mode"; FAST off: unchanged) and
   `graph_fast_mode_verify_uses_dev_fast_without_autofix`. Env vars are process-global and roko-cli has
   no shared env lock for tests, so read `ROKO_FAST_MODE` once when the dispatcher is built (a
   `fast_mode: bool` field set by a builder such as `with_fast_mode(..)`, called from `run_graph_plan`)
   and let tests set the field instead of the environment.
6. Update `dev.sh` help (lines 225-227) and the CLAUDE.md FAST paragraph to list exactly what FAST does,
   and remove the "partly ported" warning if everything is ported.

Alternative to steps 2-3 for any feature judged not worth porting: delete its claim from dev.sh help and
CLAUDE.md, and drop the dead Runner-v2 code for it. Porting is recommended; each piece is a few lines.

## Done when

- A `ROKO_FAST_MODE=1` Graph dispatch sends at most 6 turns (or `ROKO_FAST_MAX_AGENT_TURNS`), at most a
  90 s timeout, and the FAST prompt section; with FAST off nothing changes.
- A FAST verify failure does not run `cargo fix`, and cargo verify commands use `--profile dev-fast`.
- dev.sh help and CLAUDE.md describe only what the Graph path does.
- Verify: `grep -rqw 'fn graph_fast_mode_bounds_dispatch' crates/roko-cli/src/ && grep -rqw 'fn graph_fast_mode_verify_uses_dev_fast_without_autofix' crates/roko-cli/src/ && cargo test -p roko-cli graph_fast_mode_`

## Notes

- FAST must never weaken authored verification: do not shorten gate/verify timeouts (Runner-v2 kept them)
  and keep the one-verify contract fail-closed.
- `dev-fast` builds into `target/dev-fast/`, a separate cache from `target/debug/`; the first FAST verify
  after a clean tree is a cold build. Measure before making it mandatory.
- `graph_task_dispatch.rs` is large and edited by many items; keep the change local to `dispatch` and
  `settle_task_verification`. Safe in parallel with items that do not touch those functions.
- Do not remove the Runner-v2 helpers in `runner/` as part of this item unless nothing else calls them.

## Original notes


`./dev.sh fast` runs on the Graph engine again (bug-f7943a), but only part of FAST mode was ported.
According to dev.sh's own help (lines 225-227), on the Graph engine `ROKO_FAST_MODE` bounds prompt context,
enforces the one-verify plan contract and stops the run at `ROKO_FAST_PLAN_DEADLINE_SECS`. The other
variables are only recorded in the evidence metadata. The session that ported the rest lists these as
still missing on the Graph path:

- the patch-only prompt section (tell the provider to hand off after patching);
- the 90 s attempt/silence clamp;
- the 6-turn agent cap (`ROKO_FAST_MAX_AGENT_TURNS` is read only in `runner/agent_stream.rs`);
- the `dev-fast` cargo profile for gates;
- no-autofix.

CLAUDE.md's FAST paragraph still describes the full set, so it overstates what FAST does today.
Done when each feature works on the Graph path and has a test, or is dropped from dev.sh's help and CLAUDE.md.

Re-verified 2026-09-29 at d9e79e9d8: unchanged. None of the five missing FAST features (patch-only prompt section, 90 s attempt/silence clamp, 6-turn cap, dev-fast gate profile, no-autofix) has a reader on the Graph path. CLAUDE.md's FAST paragraph now points at this item (4bc92ff7a), but it still lists the full feature set.

2026-10-01 (wk-planrun): implemented on work/gap-dd4826; cargo verification deferred to the batch check.
`graph_execution/fast_lane.rs` gains `FastAttemptBounds`, which `run_graph_plan_body` reads once from the environment (`FastAttemptBounds::from_env`) and hands the dispatcher (`with_fast_bounds`). A new `graph_task_dispatch/fast.rs` applies it, keeping the hot dispatch files to one-line hooks. Both the batch path (`dispatch`) and the streaming path (`dispatch_streaming`) bound the request before the prompt-treatment bind: `--max-turns` is at most `ROKO_FAST_MAX_AGENT_TURNS` (default 6), the attempt timeout is at most 90 s, and the `## FAST implementation mode` section is appended to the system prompt. Both caps apply after the turn-cap and timeout retry escalation, so a retry never exceeds them. In verification, the step gate runs `verify_command`: a simple `cargo check|clippy|test` gets `--profile dev-fast` when `Cargo.toml` declares that profile, and a composed command runs as authored. The auto-fix branch checks `auto_fix_enabled()`, which is false in FAST. `runner/cargo_command.rs`'s `cargo_command_with_profile` and `cargo_profile_available` are now `pub(crate)`.
Silence: the 90 s attempt cap also bounds silence. The stall watchdog's own thresholds are left as configured. Verify step timeouts stay as authored.
Docs: dev.sh's FAST help and the FAST paragraphs in CLAUDE.md and README.md now list what the Graph path does. The "partly ported" warnings and the stale "skips critical-path warmup/cleanup" claim are gone; `ROKO_SKIP_PREFLIGHT` has no reader.
Tests: `graph_fast_mode_bounds_dispatch` and `graph_fast_mode_verify_uses_dev_fast_without_autofix` (graph_task_dispatch/fast.rs), plus `fast_attempts_are_capped_never_raised` and `fast_verify_commands_build_in_the_fast_profile` (fast_lane.rs). A FAST turn-cap or timeout resume note can still quote the uncapped escalated limit; `./dev.sh fast` runs with `--max-retries 0`, so it does not arise there.
