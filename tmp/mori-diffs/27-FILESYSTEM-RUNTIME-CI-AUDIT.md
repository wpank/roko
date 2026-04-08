# Filesystem, Runtime, CI, and Worktree Audit

Date: 2026-04-26

This is a continuation of the code-only audits in `25-CODE-ONLY-LEGACY-AUDIT.md`
and `26-REPOSITORY-WIDE-CODE-AUDIT.md`. It focuses on surfaces that are easy to
miss when only reading tracked Rust source: ignored proof scripts, CI, runtime
state, worktrees, deployment topology, and provider proof coverage.

## Proof of Search Scope

Commands run from `/Users/will/dev/nunchi/roko/roko`:

- [ ] Re-run `git status --short --untracked-files=all`.
  - Observed dirty tracked files: `CLAUDE.md`, `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/runner/tui_bridge.rs`, `crates/roko-learn/src/cascade_router.rs`.
- [ ] Re-run `git ls-files | wc -l`.
  - Observed tracked file count: `1798`.
- [ ] Re-run `git ls-files | rg -v '(^|/)(docs|tmp/mori-diffs|tmp/unified|tmp/unified-depth)(/|$)|\.(md|mdx|rst|txt)$' | wc -l`.
  - Observed tracked implementation/config/script/UI file count: `1312`.
- [ ] Re-run `find . \( -path './.git' -o -path './target' -o -path './node_modules' -o -path './contracts/cache' -o -path './contracts/out' -o -path './tmp/screenshots' -o -path './.claude/worktrees' \) -prune -o -type f -print | wc -l`.
  - Observed active checkout file count excluding build/cache/screenshots/worktrees: `8385`.
- [ ] Re-run the same `find` command without pruning `./.claude/worktrees`.
  - Observed `.claude/worktrees` file count alone: `186139`.
- [ ] Re-run active non-doc/non-image artifact count:
  - `find . \( -path './.git' -o -path './target' -o -path './node_modules' -o -path './contracts/cache' -o -path './contracts/out' -o -path './tmp/screenshots' -o -path './.claude/worktrees' \) -prune -o -type f -print | sed 's#^./##' | awk 'BEGIN{IGNORECASE=1} !/(^|\/)(docs|tmp\/mori-diffs|tmp\/unified|tmp\/unified-depth)(\/|$)/ && $0 !~ /\.(md|mdx|rst|txt|png|jpg|jpeg|gif|svg)$/ {print}' | wc -l`
  - Observed active non-doc/non-image implementation/config/script/state/artifact count: `3990`.
- [ ] Re-run `find .claude/worktrees -mindepth 1 -maxdepth 1 -type d -print | wc -l`.
  - Observed direct `.claude/worktrees` checkout count: `23`.
- [ ] Re-run `git worktree list --porcelain`.
  - Observed active root worktree on `wp-arch2`, several `roko-mr-stream-*` worktrees, 23 `.claude/worktrees/*` checkouts, and additional worktrees under `/Users/will/dev/uniswap/bardo/roko/.claude/worktrees/*`.
- [ ] Re-run `git branch --no-merged wp-arch2 --format='%(refname:short) %(objectname:short)'`.
  - Observed unmerged branch families: `backup/*`, `claude/migration-run-*`, `codex/*`, and `tui-fixes-*`.
- [ ] Re-run marker scans from `26-REPOSITORY-WIDE-CODE-AUDIT.md`.
  - Observed unresolved hard markers remain in non-doc files, including `todo!()` in `crates/roko-chain/src/phase2.rs`, mock/placeholder surfaces across agent/chain/serve/TUI/gate/demo code, and proof harness skip/status branches.

## Finding 1: The Main Proof Harness Is Ignored and Not Reproducible

Severity: P0

The tracked proof entrypoint is only a five-line wrapper:

`tests/proof/mori-diffs/prove-runtime-end-to-end.sh`

It execs:

`scripts/proof/mori-diffs/prove-runtime-end-to-end.sh`

That target is the real 1015-line proof harness, but it is ignored by `.gitignore`.
A clean checkout would keep the wrapper and drop the implementation.

Evidence:

- [ ] `wc -l scripts/proof/mori-diffs/prove-runtime-end-to-end.sh tests/proof/mori-diffs/prove-runtime-end-to-end.sh`
  - Observed: `1015` lines in `scripts/proof/...`; `5` lines in `tests/proof/...`.
- [ ] `git check-ignore -v scripts/proof/mori-diffs/prove-runtime-end-to-end.sh`
  - Observed: `.gitignore:65:scripts/`.
- [ ] `git ls-files tests/proof/mori-diffs/prove-runtime-end-to-end.sh scripts/proof/mori-diffs/prove-runtime-end-to-end.sh`
  - Observed: only `tests/proof/mori-diffs/prove-runtime-end-to-end.sh` is tracked.

Implementation checklist:

- [ ] Move the real proof harness into a tracked path, preferably `tests/proof/mori-diffs/prove-runtime-end-to-end.sh`.
- [ ] Delete the tracked wrapper or invert it so ignored convenience scripts call tracked proof code, never the reverse.
- [ ] Add a CI job that runs `bash -n tests/proof/mori-diffs/prove-runtime-end-to-end.sh`.
- [ ] Add a CI job that runs the proof harness in dry-run/list mode without provider credentials.
- [ ] Add a repository invariant test that fails if any tracked proof script execs an ignored `scripts/` or `tmp/` path.
- [ ] Add a repository invariant test that fails if a tracked CI workflow references an untracked file.

## Finding 2: CI References Missing Ignored Temporary Scripts

Severity: P0

The tracked workflow `.github/workflows/tui-parity-dry-run.yml` calls two scripts
under `tmp/`, but those scripts do not exist in the active checkout and `tmp/` is
ignored. This workflow cannot prove parity from a clean checkout.

Evidence:

- [ ] `sed -n '1,220p' .github/workflows/tui-parity-dry-run.yml`
  - Observed calls: `bash tmp/tui-parity/run-tui-parity.sh --dry-run` and `bash tmp/ux-followup-runner/run-ux-followup.sh --dry-run`.
- [ ] `test -e tmp/tui-parity/run-tui-parity.sh`
  - Observed exit code: `1`.
- [ ] `test -e tmp/ux-followup-runner/run-ux-followup.sh`
  - Observed exit code: `1`.
- [ ] `git check-ignore -v tmp/tui-parity/run-tui-parity.sh tmp/ux-followup-runner/run-ux-followup.sh`
  - Observed: `.gitignore:60:/tmp/`.

Implementation checklist:

- [ ] Move the TUI parity runner to a tracked path, for example `tests/proof/tui/run-tui-parity.sh`.
- [ ] Move the UX followup runner to a tracked path, for example `tests/proof/tui/run-ux-followup.sh`.
- [ ] Update `.github/workflows/tui-parity-dry-run.yml` to call only tracked files.
- [ ] Add a workflow lint script that extracts `run: bash ...` paths and verifies they are present in `git ls-files`.
- [ ] Stop using `tmp/` for CI-critical scripts; reserve it for disposable generated material only.

## Finding 3: Runtime State Shows End-to-End Failures, Not Just Missing Features

Severity: P0

The ignored `.roko/` runtime state contains concrete failed runs and incomplete
observability. This is not a theoretical architecture gap. The local runtime
recorded a failed Claude run with an empty model field, a provider billing/auth
failure embedded as agent output, and zero token/cost metrics.

Evidence:

- [ ] `git check-ignore -v .roko/events.jsonl`
  - Observed: `.gitignore:33:**/.roko/`.
- [ ] `tail -n 30 .roko/events.jsonl`
  - Observed `agent_spawned` with `"agent_id":"claude"` and `"model":""`.
  - Observed Anthropic HTTP 400 low-credit failure recorded as `agent_output`.
  - Observed `task_completed` with `"outcome":"failed"`.
  - Observed `plan_completed` with `"success":false`.
  - Observed `efficiency_event` records for `input_tokens`, `output_tokens`, and `cost_usd` all set to `0.0`.
- [ ] `sed -n '1,220p' .roko/state/executor.json.bak`
  - Observed `unified-migration-phase0` failed because `task M001 failed after retries`.
  - Observed no assigned agents, no gate results, no files changed, and `merge_attempts: 0`.
- [ ] `sed -n '1,80p' .roko/traces/2026-04-26/d09e369605ac39491bba4c14c6c98741.jsonl`
  - Observed `agent_dispatch` with `"success":false`.
  - Observed `failure_trace` with `root_cause:"tool_handler_error"` and evidence `approval denied for plan=unified-migration-phase0 task=M001 role=implementer`.

Implementation checklist:

- [ ] Make model identity mandatory in `agent_spawned`; reject or label unknown models explicitly instead of emitting `""`.
- [ ] Classify provider failures as first-class structured errors: auth, billing, rate limit, network, model unsupported, CLI missing, local model missing.
- [ ] Persist provider failure classification in runner events, TUI events, HTTP projections, and learning outcomes.
- [ ] Stop treating provider failure text as normal `agent_output` without a structured sibling event.
- [ ] Add token/cost accounting provenance so zero metrics distinguish `provider did not report usage`, `parser failed`, `not billed`, and `not measured`.
- [ ] Make executor snapshots include the last provider, model, failure kind, retry count, and gate evidence for failed tasks.
- [ ] Add `roko inspect run <run-id>` that reads `.roko/events.jsonl`, `.roko/state/executor.json`, traces, episodes, and efficiency files into one diagnosis.
- [ ] Add `roko inspect failures --json` to list failed plans/tasks with root cause, model, provider, retry count, and next action.
- [ ] Add an end-to-end proof that intentionally triggers a provider auth/billing failure and verifies the classified events and HTTP projection output.

## Finding 4: Worktree State Is Too Large and Dirty to Treat as Integrated

Severity: P0

There are 23 direct `.claude/worktrees` checkouts and 186139 files inside that
tree. Several worktree branches are merged into `wp-arch2`, but local dirty
edits remain in worktrees. The largest worktree has 466 dirty paths despite its
branch being merged.

Evidence:

- [ ] `find .claude/worktrees -mindepth 1 -maxdepth 1 -type d -print | wc -l`
  - Observed: `23`.
- [ ] `find ./.claude/worktrees -type f -print | wc -l`
  - Observed: `186139`.
- [ ] Dirty counts by direct worktree:
  - `agent-a4f25ed3c2c6a92f3`: `466`.
  - `tui-parity`: `25`.
  - `agent-aed9d75c`: `8`.
  - `agent-a2bbb3a8`: `7`.
  - `agent-a0a5ebfd`: `6`.
  - `agent-ad8cdf6b`: `5`.
  - `agent-af591515`: `4`.
  - `agent-ac4a30ee`: `3`.
  - `agent-af1f803b`: `2`.
  - `agent-a85f4984`: `1`.
  - `agent-a8665699`: `1`.
- [ ] `git -C .claude/worktrees/agent-a4f25ed3c2c6a92f3 status --short | awk '{print $1}' | sort | uniq -c`
  - Observed: `105 A`, `1 AM`, `59 D`, `301 M`.
- [ ] `git -C .claude/worktrees/agent-a4f25ed3c2c6a92f3 status --short | awk '{print $2}' | cut -d/ -f1-2 | sort | uniq -c | sort -nr | head -20`
  - Observed highest dirty scopes: `crates/roko-cli` 82, `crates/roko-compose` 55, `crates/roko-core` 44, `crates/roko-serve` 37, `crates/roko-gate` 33, `crates/roko-learn` 29, `tmp/new-docs` 27, `docs/08-chain` 26, `tmp/docs-gaps` 25, `crates/roko-agent` 21.
- [ ] `git -C .claude/worktrees/agent-a4f25ed3c2c6a92f3 log -1 --oneline`
  - Observed: `e2b3bd7f Merge pull request #52 from Nunchi-trade/wp-architecture`.
- [ ] `git branch --merged wp-arch2 --format='%(refname:short)' | rg 'worktree-agent-a4f25ed3c2c6a92f3|tui-parity|mr-stream'`
  - Observed `worktree-agent-a4f25ed3c2c6a92f3`, `worktree-tui-parity`, and `mr-stream-*` branches listed as merged.

Implementation checklist:

- [ ] Add `roko worktrees audit --json` that lists each worktree, branch, head, lock status, dirty count, and dirty top-level scopes.
- [ ] Add `roko worktrees collect --dry-run` that produces patches for dirty worktrees without mutating them.
- [ ] Add a merge policy: a branch being merged is not enough; dirty worktree contents must be explicitly archived, applied, or discarded with an audit record.
- [ ] Add CI or preflight that fails if `.claude/worktrees/*` are accidentally included in broad file scans, proof packages, or release artifacts.
- [ ] Add a tracked `tmp/mori-diffs` handoff file for each dirty worktree that states whether it contains source changes, docs-only changes, or generated junk.
- [ ] Do not mark docs complete based on worktree-local changes unless those changes are applied to `wp-arch2` or archived as patches.

## Finding 5: Provider Proof Coverage Still Does Not Match the Provider Set

Severity: P0

The proof harness has a provider matrix, but the current automatic list does not
include `moonshot` or `zai`, even though the repo has first-class Kimi/Moonshot
and GLM/Z.AI examples and tests. This matters because the user specifically has
Anthropic, Moonshot, Z.AI, OpenAI, and Perplexity keys.

Evidence:

- [ ] `sed -n '80,280p' scripts/proof/mori-diffs/prove-runtime-end-to-end.sh`
  - Observed automatic provider list: `claude`, `codex`, `anthropic`, `openai`, `gemini`, `ollama`, `perplexity`.
  - Observed no `moonshot` provider entry.
  - Observed no `zai` provider entry.
- [ ] `rg -n 'providers.moonshot|providers.zai|models.kimi-k2-5|models.glm-5-1' examples demo/demo-resources/provider-routing -g '*.toml'`
  - Observed `examples/roko-kimi.toml`, `examples/roko-glm.toml`, and `demo/demo-resources/provider-routing/roko.toml` configure Moonshot and Z.AI.
- [ ] `rg -n 'kimi|glm|moonshot|zai' crates/roko-agent/tests crates/roko-agent/src/provider/openai_compat.rs`
  - Observed provider tests and adapters for Kimi/Moonshot and GLM/Z.AI.

Implementation checklist:

- [ ] Add `moonshot` and `zai` to the proof provider matrix as OpenAI-compatible HTTP providers.
- [ ] Add credential mapping: `MOONSHOT_API_KEY` for `moonshot`, `ZAI_API_KEY` for `zai`.
- [ ] Add model mapping: `ROKO_PROOF_MOONSHOT_MODEL` defaulting to `kimi-k2.5`, `ROKO_PROOF_ZAI_MODEL` defaulting to `glm-5.1`.
- [ ] Add model keys matching example config: `kimi-k2-5` and `glm-5-1`.
- [ ] Generate a temporary `roko.toml` per provider from the same generic template rather than hardcoding provider-specific branches throughout the proof script.
- [ ] Prove `anthropic`, `moonshot`, `zai`, `openai`, and `perplexity` with real API calls when the corresponding env vars are present.
- [ ] Mark skipped providers as skipped only for missing credentials, missing binaries, or unavailable local models; do not mark an implemented provider as unsupported because the proof harness lacks a path.
- [ ] Persist per-provider proof artifacts with provider, model key, provider kind, base URL, task result, event count, token usage, cost, retry evidence, gate evidence, HTTP projection evidence, and mock status.

## Finding 6: Active Runner Has Provider Bridge Wiring, but It Is Still Not Proven as a Uniform Runtime

Severity: P1

Current `event_loop.rs` has a `ResolvedAgentDispatch::Bridge` path and calls
`AgentDispatcherV2::run_agent_result_bridge`. That is progress. The remaining
problem is that the proof and observability model still treats CLI subprocess
providers and AgentResult providers differently. CLI providers have PID lifecycle
events; bridge providers synthesize events without PID ownership. The runtime can
work, but Mori-like parity requires one lifecycle contract.

Evidence:

- [ ] `sed -n '990,1105p' crates/roko-cli/src/runner/event_loop.rs`
  - Observed `ResolvedAgentDispatch::Cli` and `ResolvedAgentDispatch::Bridge`.
  - Observed bridge dispatch via `AgentDispatcherV2::run_agent_result_bridge`.
- [ ] `sed -n '1535,1628p' crates/roko-cli/src/runner/event_loop.rs`
  - Observed CLI path spawns a process and records `agent_pid`.
  - Observed bridge path sets `agent_pid = None` and emits `AgentDispatchOutcome::Spawned`.
- [ ] `sed -n '500,650p' crates/roko-cli/src/dispatch_v2.rs`
  - Observed comment that bridge dispatch returns `pid: Option<u32>` because event protocol cannot claim OS process ownership.

Implementation checklist:

- [ ] Replace `Spawned` as the only successful dispatch outcome with normalized outcomes: `ProcessSpawned`, `RemoteTurnStarted`, `LocalBridgeStarted`, `Rejected`, `ProviderFailed`.
- [ ] Make `agent_pid` optional in projections and TUI by design, not as an implicit missing field.
- [ ] Add a normalized `AgentRunId` that applies to CLI subprocesses and API providers.
- [ ] Emit the same lifecycle sequence for every provider: dispatch requested, provider resolved, run started, output delta, usage observed, turn completed, run exited, task verdict.
- [ ] Make provider-specific lifecycle details an extension object, not divergent event types.
- [ ] Add a real provider proof that runs one simple code-editing task through each non-mock provider path and validates the normalized lifecycle sequence.

## Finding 7: Merge Queue Is Present, but Merge Is Still Not a Real Git Merge

Severity: P0

The active runner now enqueues and reserves merge requests, but `MergeBranch`
still marks the executor as `MergeSucceeded` without checking out, merging,
validating, or committing a branch. This is a critical gap for Mori-like
orchestration because multi-agent work cannot be considered end-to-end without
real branch/worktree isolation and conflict handling.

Evidence:

- [ ] `sed -n '1788,1868p' crates/roko-cli/src/runner/event_loop.rs`
  - Observed `ctx.merge_queue.enqueue(...)`.
  - Observed `ctx.merge_queue.reserve_next_mergeable()`.
  - Observed direct `ExecutorEvent::MergeSucceeded`.
  - Observed no `git merge`, no worktree checkout, no conflict capture, no post-merge gate, no commit/update-ref.

Implementation checklist:

- [ ] Introduce a `MergeBackend` trait with `prepare`, `merge`, `validate`, `commit_or_update_ref`, `abort`, and `cleanup`.
- [ ] Implement `GitMergeBackend` using a disposable worktree or temporary index.
- [ ] Make `MergeQueue` only schedule/reserve; never decide success without a backend result.
- [ ] On merge conflict, persist conflicted paths, conflict hunks, branch name, base ref, and retry strategy.
- [ ] Run configured post-merge gates before emitting `MergeSucceeded`.
- [ ] Emit `MergeFailed` for backend failure and `MergeBlocked` for file lock conflicts.
- [ ] Add proof with two branches touching disjoint files and verify both merge.
- [ ] Add proof with two branches touching the same file and verify conflict evidence appears in events, TUI bridge, HTTP projection, and resume state.

## Finding 8: Deployment Topology Still Ships Placeholders

Severity: P1

Docker compose includes a `gateway` service and Prometheus scrapes `gateway:8080`,
but there is no `roko-gateway` workspace crate. The gateway image currently
builds the CLI binary as a stand-in and exposes a placeholder port.

Evidence:

- [ ] `sed -n '1,180p' docker/gateway.Dockerfile`
  - Observed TODO: `roko-gateway` crate does not exist.
  - Observed image builds `cargo build --release --bin roko` and copies it to `/roko-gateway`.
  - Observed final command defaults to `--help`.
- [ ] `sed -n '1,220p' docker/docker-compose.yml`
  - Observed `gateway` service points to `docker/gateway.Dockerfile`.
  - Observed `roko` service serves on `9092`.
- [ ] `sed -n '1,160p' docker/prometheus.yml`
  - Observed scrape target `gateway:8080`.
- [ ] `find crates -maxdepth 2 -name Cargo.toml | rg gateway`
  - Observed no `crates/roko-gateway`.

Implementation checklist:

- [ ] Either create a real `roko-gateway` crate or remove the gateway service until it exists.
- [ ] If creating the crate, define `/health`, `/metrics`, `/api/*` ownership, config, auth, and shutdown behavior.
- [ ] Update `docker/gateway.Dockerfile` to build `--bin roko-gateway`.
- [ ] Update compose health checks to verify the gateway serves the intended endpoints, not `--help`.
- [ ] Update Prometheus targets only after `/metrics` exists and is validated.
- [ ] Add a docker smoke proof that starts compose, checks `roko:9092`, `gateway:8080`, Prometheus target health, and one runtime projection endpoint.

## Finding 9: Runtime State Is Ignored but Contains Product-Relevant Data

Severity: P1

`.roko/` is correctly ignored for local runtime data, but the current product has
no clean way to export a minimized, secret-safe proof bundle. The ignored runtime
contains `events.jsonl`, learning files, traces, metrics, plans, TUI logs, and
Mirage state. Without a structured export, users and agents cannot prove a run
without copying ad-hoc ignored files.

Evidence:

- [ ] `find .roko -type f | sort`
  - Observed files under `.roko/learn`, `.roko/state`, `.roko/traces`, `.roko/plans`, `.roko/metrics`, `.roko/runtime`, `.roko/tui.log`.
- [ ] `git check-ignore -v .roko/events.jsonl`
  - Observed `.roko/` ignored.

Implementation checklist:

- [ ] Add `roko export proof --run-id <id> --out <dir>` that exports only safe evidence.
- [ ] Redact prompts, env vars, API keys, wallet private keys, and filesystem paths when requested.
- [ ] Include event logs, executor snapshots, traces, provider outcomes, token/cost summaries, gate outputs, HTTP projection snapshots, and version metadata.
- [ ] Include a manifest with SHA256 for every exported file.
- [ ] Add `roko proof verify <dir>` to validate the bundle without the original workspace.
- [ ] Make proof scripts emit this proof bundle as the primary artifact.

## Finding 10: Root Config Defaults Are Still Backward-Compatible, Not Provider-Complete

Severity: P1

Root `roko.toml` has empty `[providers]` and `[models]`, while `agent.default_model`
and routing defaults name Claude models. The config code has compatibility logic
to infer providers and models, but a clean "all providers" user setup still
requires examples or manual config composition.

Evidence:

- [ ] `sed -n '1,230p' roko.toml`
  - Observed `[providers]` empty.
  - Observed `[models]` empty.
  - Observed `agent.default_model = "claude-sonnet-4-6"`.
  - Observed routing defaults for `claude-haiku-4-5`, `claude-sonnet-4-6`, and `claude-opus-4-6`.
- [ ] `rg -n 'effective_providers|effective_models|backwards_compat' crates/roko-core/src/config/schema.rs`
  - Observed compatibility code that fills defaults.
- [ ] `rg -n 'providers.moonshot|providers.zai|providers.perplexity' examples demo/demo-resources/provider-routing -g '*.toml'`
  - Observed non-root examples carry richer provider config than root config.

Implementation checklist:

- [ ] Add `roko config init --providers anthropic,moonshot,zai,openai,perplexity` to generate a complete provider/model registry.
- [ ] Add `roko config doctor --all-providers` that validates env var presence, base URL reachability, model IDs, and tool-loop capability.
- [ ] Add a global config discovery command that prints exact config layers and redacted credential status.
- [ ] Make proof scripts generate temporary provider-complete config instead of depending on root `roko.toml`.
- [ ] Add a documented provider matrix with model key, provider id, env var, base URL, adapter kind, tool format, streaming support, usage parsing, and known limitations.

## Finding 11: Contracts and Local Chain Artifacts Are Not Cleanly Reproducible

Severity: P2

The contracts tree has ignored vendored dependencies under `contracts/lib` and no
git submodule metadata. Local 31337 broadcast artifacts are ignored while 88888
broadcast artifacts are tracked. This may be intentional, but it needs a
decision because end-to-end proofs that include local chain deployment should not
depend on undeclared vendored state.

Evidence:

- [ ] `git submodule status --recursive`
  - Observed no submodules.
- [ ] `git check-ignore -v contracts/lib/*`
  - Observed `contracts/lib` ignored by `contracts/.gitignore`.
- [ ] `find contracts/broadcast -type f | sort`
  - Observed tracked-looking `88888` broadcast JSON files and local `31337` JSON files.

Implementation checklist:

- [ ] Decide whether `contracts/lib` is managed by Forge install, git submodules, vendored checked-in code, or generated cache.
- [ ] Add a setup script that recreates contracts dependencies from a clean checkout.
- [ ] Normalize broadcast artifact policy: either track deterministic deployment references or ignore all local broadcast output.
- [ ] Add local-chain proof that starts Mirage/Anvil equivalent, deploys contracts, writes addresses, and proves Roko can query them.

## Finding 12: Subagent Results Show Implementation Drift Across Workspaces

Severity: P1

Subagents reported implementation work and audits that are not all visible as
clean tracked changes in the main worktree. Examples include runner v2 server
adoption, orchestrator snapshot APIs, roko-serve projection surfaces, and proof
harness expansion. Some of that appears partially present in dirty main files;
some may only exist in forked workspaces. This is a process risk: "agent says it
implemented" is not equivalent to "merged into `wp-arch2` and reproducible."

Evidence:

- [ ] Main worktree `git status --short --untracked-files=all` shows only four dirty tracked files.
- [ ] Direct `.claude/worktrees` dirty counts show many changes outside the main tree.
- [ ] Subagent summaries claimed edits in `crates/roko-cli/src/serve_runtime.rs`, `crates/roko-orchestrator/src/*`, `crates/roko-serve/src/*`, and `scripts/proof/mori-diffs/prove-runtime-end-to-end.sh`.
- [ ] Main tree still has ignored proof harness location and tracked wrapper mismatch.

Implementation checklist:

- [ ] For each subagent, identify its worktree and produce `git diff --stat`.
- [ ] Categorize each diff as `apply`, `archive as patch`, `superseded`, or `discard`.
- [ ] Apply source changes through normal git patches into `wp-arch2`, not by relying on live worktree state.
- [ ] After applying, run `cargo fmt`, targeted `cargo check`, and proof syntax checks.
- [ ] Update `tmp/mori-diffs` docs only after changes are present in the main worktree or archived as patches.

## Definition of Done for This Audit Area

- [ ] A clean clone can run every tracked proof script without referencing ignored or missing files.
- [ ] CI workflows reference only tracked files.
- [ ] Provider proof matrix covers Anthropic, Moonshot, Z.AI, OpenAI, Perplexity, Claude CLI, Codex CLI, Gemini, and Ollama where configured.
- [ ] Provider proof status distinguishes `proved`, `missing_credentials`, `missing_binary`, `missing_local_model`, `provider_auth_failed`, `provider_billing_failed`, `provider_rate_limited`, and `unsupported`.
- [ ] Runner events use one provider-neutral lifecycle contract for CLI and API providers.
- [ ] Merge success requires a real git merge/worktree backend and post-merge gates.
- [ ] Runtime failures can be inspected through CLI and HTTP endpoints without manually opening `.roko` files.
- [ ] Worktree-local changes are either merged into `wp-arch2`, archived as patches, or explicitly discarded.
- [ ] Docker compose does not ship placeholder services that masquerade as implemented gateway/runtime surfaces.
- [ ] Local chain proof can be reproduced without undeclared ignored dependencies.

## Self-Grade

Initial grade: 9.4 / 10.

Reason: this pass covers the previously missed filesystem/runtime/CI/worktree
layer with concrete commands and implementation checklists, but it does not yet
fully diff every dirty worktree or inspect every unmerged branch. That remaining
work is tractable and should be handled by `roko worktrees audit`/patch export or
a follow-up manual branch audit before claiming 9.8+ completeness.

## 2026-04-27 Deepening Pass - Proof Spine, Clean Clone Invariants, And Status Artifacts

This pass updates the audit against the current checkout and aligns it with the newer architecture docs:

- [28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md](28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md): generated feature/dogfood/UX status reconciliation.
- [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md): canonical implementation order.
- [30-ARCHITECTURAL-SIDE-EFFECT-AUDIT.md](30-ARCHITECTURAL-SIDE-EFFECT-AUDIT.md): side-effect owner manifest and generated inventory.
- [33-CONFIGURATION-PROVIDER-POLICY-AUDIT.md](33-CONFIGURATION-PROVIDER-POLICY-AUDIT.md): provider policy and secret provenance.
- [34-OBSERVABILITY-PROJECTION-QUERY-AUDIT.md](34-OBSERVABILITY-PROJECTION-QUERY-AUDIT.md): event store, projection, query, and proof bundle contracts.

Updated self-grade after this deepening pass: `9.90 / 10`.

Reason: this now defines the tracked proof spine, proof report schema, clean-clone invariants, CI path linting, provider matrix expectations, runtime export contract, and worktree evidence policy. It is not a 10 because the proof export/verify CLI and CI invariant jobs are still implementation tasks, not completed code.

### Current Refresh Evidence

Fresh commands run on 2026-04-27:

```bash
wc -l scripts/proof/mori-diffs/prove-runtime-end-to-end.sh tests/proof/mori-diffs/prove-runtime-end-to-end.sh
git check-ignore -v scripts/proof/mori-diffs/prove-runtime-end-to-end.sh tests/proof/mori-diffs/prove-runtime-end-to-end.sh
git ls-files scripts/proof/mori-diffs/prove-runtime-end-to-end.sh tests/proof/mori-diffs/prove-runtime-end-to-end.sh
bash -n tests/proof/mori-diffs/prove-runtime-end-to-end.sh
bash -n scripts/proof/mori-diffs/prove-runtime-end-to-end.sh
rg -n "moonshot|zai|kimi|glm|perplexity|anthropic|openai|claude|codex|missing_credentials|auth_failed|rate_limited|unsupported" tests/proof/mori-diffs/prove-runtime-end-to-end.sh
rg -n "run: bash (tmp/|scripts/)|tmp/tui-parity|tmp/ux-followup-runner|scripts/proof" .github/workflows tests/proof scripts -g '*.yml' -g '*.yaml' -g '*.sh'
git ls-files | wc -l
find . \( -path './.git' -o -path './target' -o -path './node_modules' -o -path './contracts/cache' -o -path './contracts/out' -o -path './tmp/screenshots' -o -path './.claude/worktrees' \) -prune -o -type f -print | wc -l
find .claude/worktrees -mindepth 1 -maxdepth 1 -type d -print 2>/dev/null | wc -l
```

Observed current facts:

- `tests/proof/mori-diffs/prove-runtime-end-to-end.sh` is now a real tracked proof script with `1178` lines, not only the old five-line wrapper.
- `scripts/proof/mori-diffs/prove-runtime-end-to-end.sh` still exists as an ignored duplicate with `1015` lines.
- `git check-ignore` still reports `.gitignore:94:scripts/` for the ignored script copy.
- `git ls-files` reports only `tests/proof/mori-diffs/prove-runtime-end-to-end.sh` as tracked.
- `bash -n tests/proof/mori-diffs/prove-runtime-end-to-end.sh` passes.
- `bash -n scripts/proof/mori-diffs/prove-runtime-end-to-end.sh` passes.
- The tracked proof script now references `claude`, `codex`, `anthropic`, `openai`, `moonshot`, `zai`, and `perplexity`.
- The tracked proof script validates provider statuses against `proved`, `missing_credentials`, `auth_failed`, `rate_limited`, and `unsupported`.
- `.github/workflows/tui-parity-dry-run.yml` still references missing/ignored `tmp/tui-parity/run-tui-parity.sh` and `tmp/ux-followup-runner/run-ux-followup.sh`.
- Current tracked file count is `1862`.
- Active file count with the old prune set is `37342`.
- Direct `.claude/worktrees` checkout count remains `23`.

### Updated Finding Status

| Finding | Old Status | Current Reconciled Status | Required Action |
| --- | --- | --- | --- |
| Main proof harness ignored | P0 open | partial | Keep tracked harness as canonical; delete or clearly demote ignored duplicate |
| CI references ignored tmp scripts | P0 open | still open | Move scripts to tracked proof path or retire workflow |
| Runtime state shows failures | P0 open | still open | Add structured failure classes, inspect commands, export proof bundles |
| Worktree state too large/dirty | P0 open | still open | Add worktree audit/collect commands and patch archive policy |
| Provider proof misses Moonshot/Z.AI | P0 open | partially fixed in tracked proof script | Execute live proof and keep provider matrix generated from config |
| Provider lifecycle non-uniform | P1 open | still open | Normalize lifecycle events across CLI/API/bridge providers |
| Merge queue fake success | P0 open in original audit | verify against current runner before closing | Require merge backend proof from [39](39-RUNNER-EXECUTION-POLICY-AUDIT.md) |
| Gateway placeholder topology | P1 open | still open unless gateway removed/implemented | Tie to [41](41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md) |
| No proof export bundle | P1 open | still open | Implement `roko export proof` and `roko proof verify` |
| Root config provider completeness | P1 open | still open | Tie to [33](33-CONFIGURATION-PROVIDER-POLICY-AUDIT.md) |
| Contracts reproducibility | P2 open | still open | Decide dependency/broadcast artifact policy |
| Subagent drift across workspaces | P1 open | still open | Use worktree audit/collect and source-of-truth patch archive |

### Proof Spine Contract

The proof spine is the minimum tracked set required before any doc can claim Mori-like runtime parity.

Tracked proof entrypoints:

- [ ] `tests/proof/mori-diffs/prove-runtime-end-to-end.sh` is the canonical runtime proof script.
- [ ] `tests/proof/mori-diffs/prove-feature-matrix-status.sh` should generate the feature/dogfood/UX status ledger described in [28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md](28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md).
- [ ] `tests/proof/mori-diffs/prove-side-effect-owners.sh` should run the side-effect inventory from [30-ARCHITECTURAL-SIDE-EFFECT-AUDIT.md](30-ARCHITECTURAL-SIDE-EFFECT-AUDIT.md).
- [ ] `tests/proof/mori-diffs/prove-projection-query.sh` should run HTTP/TUI/projection parity from [34](34-OBSERVABILITY-PROJECTION-QUERY-AUDIT.md) and [40](40-SERVE-TUI-RUNTIME-ADAPTER-AUDIT.md).
- [ ] `tests/proof/mori-diffs/prove-provider-matrix.sh` may be separate if the current end-to-end script becomes too large.
- [ ] `tests/proof/mori-diffs/prove-resume-merge-gates.sh` should prove crash/resume, merge success, merge conflict, gate retry, and replan behavior.

Forbidden proof topology:

- [ ] A tracked proof script must not exec an ignored `scripts/` implementation.
- [ ] A tracked workflow must not reference `tmp/` files.
- [ ] A proof script must not treat missing credentials as provider success.
- [ ] A proof script must not use mocks when the status is `proved`.
- [ ] A proof script must not read private runtime files for a user-facing claim unless the same data is queryable through `RuntimeQueryService` or an HTTP endpoint.

### Proof Report Schema

Every proof script should write a root report named `proof-report.json` with this shape:

```json
{
  "schema_version": 1,
  "generated_at": "2026-04-27T00:00:00Z",
  "repo_root": "/redacted/or/source-root",
  "git": {
    "head": "HEAD_SHA",
    "branch": "wp-arch2",
    "dirty": true
  },
  "workspace": {
    "path": "/tmp/roko-proof-workspace",
    "mock_mode": false
  },
  "providers": [
    {
      "provider": "moonshot",
      "model": "kimi-k2.5",
      "status": "proved",
      "credential_env": "MOONSHOT_API_KEY",
      "secret_source": "env",
      "artifacts": ["providers/moonshot/events.jsonl"]
    }
  ],
  "runtime": {
    "run_id": "run-123",
    "events": 42,
    "episodes": 1,
    "efficiency_records": 1,
    "prompt_diagnostics": 1,
    "gate_events": 3,
    "merge_events": 0,
    "resume_events": 0
  },
  "http": {
    "base_url": "http://127.0.0.1:0",
    "queries_proved": ["runtime", "providers", "learning", "knowledge", "projections"]
  },
  "status_ledger": "status-reconciliation.json",
  "side_effect_inventory": "side-effects.json",
  "proof_bundle": "proof-bundle/manifest.json"
}
```

Report checklist:

- [ ] Every provider result uses only `proved`, `missing_credentials`, `auth_failed`, `rate_limited`, or `unsupported`.
- [ ] Provider failures include redacted stderr/stdout evidence.
- [ ] `proved` requires non-empty runtime output and provider/model labels in events.
- [ ] Report includes whether the run used mocks.
- [ ] Report includes all artifact paths relative to `ROKO_PROOF_ARTIFACT_ROOT`.
- [ ] Report includes query endpoint responses used for proof.
- [ ] Report includes generated status ledger and side-effect inventory paths when those checks run.

### Clean-Clone Invariants

Add a small invariant script, for example `tests/proof/mori-diffs/check-clean-clone-invariants.sh`, with these checks:

- [ ] Every file referenced by `.github/workflows/*.yml` in a `run: bash path` command exists in `git ls-files`.
- [ ] Every tracked `tests/proof/**/*.sh` passes `bash -n`.
- [ ] No tracked proof script contains `scripts/proof/`, `tmp/`, `.claude/worktrees`, or absolute `/Users/will/` paths unless explicitly allowlisted.
- [ ] No tracked workflow path filter points only at ignored paths.
- [ ] `git check-ignore` does not ignore any tracked proof script implementation.
- [ ] The canonical runtime proof script can print provider matrix/list mode without credentials.
- [ ] The proof artifact root defaults outside the repo.

Proof:

- [ ] The invariant script passes in the current worktree.
- [ ] The invariant script is added to CI.
- [ ] CI failure output names the missing or ignored referenced path.

### Provider Proof Matrix Requirements

The tracked script now includes Moonshot and Z.AI, but implementation is not complete until real proof artifacts exist.

Provider checklist:

- [ ] `claude` uses Claude CLI when the binary is present.
- [ ] `codex` uses Codex CLI when the binary is present.
- [ ] `anthropic` uses `ANTHROPIC_API_KEY`.
- [ ] `openai` uses `OPENAI_API_KEY`.
- [ ] `moonshot` uses `MOONSHOT_API_KEY` and default model `kimi-k2.5`.
- [ ] `zai` uses `ZAI_API_KEY` and default model `glm-5.1`.
- [ ] `perplexity` uses `PERPLEXITY_API_KEY`.
- [ ] Every provider is generated from one provider-template function or data table, not ad hoc branches spread through the proof harness.
- [ ] Each provider writes a provider-specific artifact directory with config, stdout/stderr, runtime events, HTTP query responses, and final status.
- [ ] `unsupported` means the runtime genuinely does not support that provider path, not that the proof harness has no branch.

### Runtime Inspection And Proof Export

Add CLI-level commands so proof scripts and humans do not need to read ignored `.roko` internals manually:

- [ ] `roko inspect run <run-id> --json`.
- [ ] `roko inspect failures --json`.
- [ ] `roko inspect providers --json`.
- [ ] `roko inspect projections --json`.
- [ ] `roko export proof --run-id <run-id> --out <dir>`.
- [ ] `roko proof verify <dir>`.

Export bundle requirements:

- [ ] Include event store records.
- [ ] Include projection snapshots and queried HTTP responses.
- [ ] Include provider lifecycle and failure classification.
- [ ] Include prompt diagnostics and redaction report.
- [ ] Include gate, retry, replan, merge, conflict, resume, feedback, knowledge, and dream evidence when present.
- [ ] Include SHA256 manifest.
- [ ] Redact secrets and optionally redact absolute local paths.
- [ ] Verify without the original workspace.

### Worktree Evidence Policy

The repo still has many external/forked worktrees, and the previous audit showed dirty worktree state can be mistaken for merged implementation.

Policy:

- [ ] A subagent claim is not source truth until the diff is merged into the main worktree or archived as a patch under a tracked path.
- [ ] A branch merged into `wp-arch2` is not enough if its attached worktree remains dirty.
- [ ] Proof scripts must exclude `.claude/worktrees` by default.
- [ ] Proof bundle export must exclude worktrees unless explicitly requested.
- [ ] A worktree audit must record branch, head, dirty count, dirty scopes, and whether each diff is `apply`, `archive`, `superseded`, or `discard`.

Implementation checklist:

- [ ] Add `roko worktrees audit --json`.
- [ ] Add `roko worktrees collect --dry-run --out <dir>`.
- [ ] Add `roko worktrees archive-patches --out <dir>`.
- [ ] Add proof that these commands do not mutate worktrees unless explicitly requested.

### Updated Definition Of Complete

This doc can be considered implemented only when:

- [ ] All tracked proof scripts are self-contained and syntax-checked in CI.
- [ ] No tracked workflow references ignored or missing `tmp/` scripts.
- [ ] The ignored `scripts/proof/mori-diffs/prove-runtime-end-to-end.sh` duplicate is removed or explicitly marked as a developer convenience wrapper around the tracked script.
- [ ] Provider proof covers Claude CLI, Codex CLI, Anthropic, OpenAI, Moonshot, Z.AI, and Perplexity with explicit statuses.
- [ ] Proof reports include provider status, runtime event counts, HTTP query evidence, status ledger path, side-effect inventory path, and proof bundle path.
- [ ] `roko export proof` and `roko proof verify` exist and are used by proof scripts.
- [ ] Runtime failure inspection works without manually opening `.roko` files.
- [ ] Worktree audit/collect prevents dirty forked worktrees from being mistaken for merged work.
- [ ] Docker/gateway topology either reflects real implemented services or is removed from proof claims.
