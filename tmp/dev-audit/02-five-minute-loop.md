# Five-minute development loop

## Service-level objective

For an eligible XS/local task:

- p50 patch-to-evidence at or below 300 seconds.
- p95 at or below 600 seconds.
- Exactly one real dispatch per task attempt.
- No hidden continuation after the deadline.
- A complete, queryable evidence bundle whether the result passes, fails, or escalates.

The deadline is a control mechanism, not an instruction to pretend unfinished work is done.

Current status: the original opt-in patch/deadline/verification-owner subset is on `main` at
`a58bdbacb`. The expanded integration adds bounded/cohesive plan policy, impact-selected reverse
dependents, safe endpoint and opt-in CLI/text/PNG evidence, wake-driven hard-deadline scheduling,
first-class timeout-diff gate salvage, protected cache maintenance, explicit offline index repair,
and fixed-SHA benchmark orchestration. Final batched verification and representative cold/warm
benchmark repetitions remain open; see
[11-implementation-status.md](11-implementation-status.md).

## Eligibility

FAST mode is appropriate when all of these are true:

- The goal is one observable outcome.
- The patch is expected to touch at most about three closely related files.
- No durable format, migration, authentication, authorization, payment, or sandbox boundary changes.
- No scheduler/concurrency invariant changes.
- Public API impact is local or already enumerated.
- Verification can be expressed as a structural check, one target-aware compile, and one direct
  behavior probe.

If impact analysis discovers more scope, upgrade the tier or split the task before provider work.

## Phase budget

| Time | Owner | Work | Hard rule |
|---|---|---|---|
| 0:00–0:20 | Harness | Preflight, run ID, resource guard, diff/base capture, risk tier | No workspace warm |
| 0:20–0:40 | Planner | Resolve exact files/symbols and choose one gate | Missing context fails now |
| 0:40–2:10 | Agent | Read exact context and make the smallest patch | No Cargo; 90s edit budget |
| 2:10–2:30 | Harness | Diff, format, structural checks | Stop on scope violation |
| 2:30–3:30 | Harness | One target-aware check or exact test | One compiler owner |
| 3:30–4:20 | Harness | Real CLI/API/TUI/browser smoke | Capture output/log/screenshot |
| 4:20–5:00 | Harness | Risk review, bundle validation, commit or escalation | Exactly one terminal result |

For a deterministic one-line task, the edit phase should be a direct transformation or learned
reflex and finish in seconds.

## Agent/harness split

The current workflow asks the agent to edit, compile, test, interpret build failures, and then
wait for the runner to compile again. FAST mode changes ownership:

Agent:

- Reads supplied exact locations.
- Searches only when a named symbol is unresolved.
- Applies one coherent patch.
- Inspects the diff.
- Returns a structured handoff.

Harness:

- Owns Cargo, test selection, process timeouts, endpoints, screenshots, logs, and commits.
- Chooses one semantic verification pipeline from the changed targets.
- Can cancel verification without losing the patch.
- Records all phase spans and terminal state.

This keeps slow command execution outside the provider's paid/session timeout and makes it
observable and reusable.

## Task contract

Each FAST task must provide:

- Goal: one observable result.
- Base commit and dirty-state policy.
- Risk tier.
- Exact allowed files.
- Exact target symbols with line anchors or supplied snippets.
- One to three acceptance assertions.
- Explicit non-goals.
- Change budget: files, approximate LOC, public API/schema allowance.
- Verification owner and selected command.
- Escalation conditions.

The reusable contract is in [prompts/fast-implement.md](prompts/fast-implement.md).

OpenAI's Codex guidance similarly recommends a clear Goal, Context, Constraints, and Done section,
durable concise AGENTS.md instructions, focused subagents, and lower reasoning for scoped work:
[Codex best practices](https://learn.chatgpt.com/guides/best-practices.md).

## Task sizing

Avoid both extremes:

- Do not give one agent an unbounded architecture project.
- Do not split a 150-line cohesive feature into seven serial sessions that each rebuild the same
  binary.

Use one session when changes share the same context, files, compile target, and evidence path.
Split only on an actual dependency boundary, risk boundary, or independently verifiable outcome.

For doctor-network-v2, the enforced FAST shaping target is:

1. One cohesive doctor network implementation task covering types, helpers, runner wiring, and
   focused unit behavior.
2. One CLI/output smoke task if needed.

T1's enum variant should be a deterministic pre-edit, not a premium-model task. The integration
now rejects same-file microtask fragmentation and duplicate verification, bounds task/read/range
budgets, and keeps exact PRD/task artifacts in the generated plan. A general deterministic
transformation/reflex broker is still separate work.

## Model and reasoning policy

- Mechanical/deterministic: no LLM where possible.
- Clear repeatable patch: fastest capable model, low reasoning.
- Normal localized implementation: balanced model, low or medium reasoning.
- Broad/high-risk architecture: frontier model and separate RELEASE lane.

Do not route every task to the most expensive/slower model by default. The current configuration
routes the Codex alias to gpt-5.6-sol even for mechanical work. Benchmark model quality and
latency by fixture rather than selecting by prestige.

Codex also supports a fast mode for supported models at a credit tradeoff:
[Codex speed](https://learn.chatgpt.com/docs/agent-configuration/speed.md). That is an optional
latency lever after orchestration waste is fixed; it cannot compensate for 600-second cold builds.

## Prompt/context policy

- Put durable project rules in a short root AGENTS.md.
- Put directory-specific rules in deeper AGENTS.md files.
- Keep stable prefix content first and dynamic task/diff content last for prompt-cache reuse.
- Supply exact snippets instead of asking an agent to rediscover a 900K-line workspace.
- Do not inject global logs, histories, learning entries, or full source maps unless selected by
  the task's named symbols and risk.
- Set an exploration budget: normally at most six read/search/edit tool calls before patch.
- Require a reason before searching outside allowed files.

Codex automatically discovers scoped AGENTS.md files with root-to-current-directory precedence:
[AGENTS.md documentation](https://learn.chatgpt.com/docs/agent-configuration/agents-md.md).

## Deadline behavior

At 90 seconds without a patch:

- Stop exploration.
- Preserve reads and hypotheses.
- Return needs_context, needs_decomposition, or blocked with the exact missing item.

At five minutes without verified evidence:

- Terminate the command/process tree.
- Preserve the worktree and diff.
- Write the terminal reason and first blocker.
- Do not increment provider-call counters for attempts that never launched.
- Do not mark a task failed solely because the agent's optional self-verification was still
  compiling; let the runner validate the existing patch once.

The expanded integration implements that salvage lifecycle. After confirmed provider cleanup, a
non-empty mutable/safe diff is content-fingerprinted from its immutable base plus exact bytes and
modes, receives the normal post-dispatch safety check, and enters ordinary gate ownership without
another provider launch. Resume revalidates the fingerprint before starting the gate. Empty,
conflicted, read-only, unsafe, changed, or cleanup-unconfirmed diffs settle without certification.

## FAST versus RELEASE

| Concern | FAST | RELEASE |
|---|---|---|
| Goal | Immediate development feedback | Merge/deploy confidence |
| Provider sessions | One | As required by risk |
| Compilation | One impacted target | Impacted reverse dependents |
| Tests | None or one exact test | Focused suites, broader async CI |
| Clippy | Usually deferred | Impacted package once |
| Runtime proof | Required for behavior changes | Required plus regression proof |
| Time limit | Five-minute SLO | Explicit larger budget |
| Failure | Bundle + escalation | Blocks merge |
