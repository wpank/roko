# Planning Methodology: Best Practices for an Agent-Executable Backlog

**Purpose.** A cited synthesis of current (2024–2026) best practice on decomposing,
specifying, sizing, and gating work for autonomous LLM/coding agents, translated into
concrete recommendations for roko's `tasks.toml` schema.

**Scope & method.** Sources below were gathered via web search + direct fetch in July 2026.
Two primary sources were fetched in full and verified: the SWE-agent paper and the Claude
Code best-practices docs. Others are cited from search-surfaced pages (URL + date given);
where a page could not be fetched in full, the claim is attributed to the search snippet and
flagged. No citation here is fabricated.

**Roko schema referenced** (from `crates/roko-core/src/task.rs`, the `Task` struct): `id`,
`title`, `files`, `exclusive_files` (default true), `parallel_group`, `depends_on`,
`acceptance` (human bullets), `test_invariants`, `context_files`, `example_pattern`,
`domain` (selects gates), `complexity_band` (fast/standard/complex), `escalate_on_retry`.
The backlog-design layer adds four **tiers** (mechanical/focused/integrative/architectural),
machine-checkable **verify** steps (shell, exit 0 = pass), and typed **acceptance contracts**.

---

## 1. Task decomposition & hierarchical planning

**Findings.**

- Hierarchical decomposition lets the planner reason at the right level of abstraction per
  decision: high-level sequencing needn't know low-level implementation, and vice-versa. This
  separation of concerns is what makes planning tractable.
  ([LLM Planner, emergentmind.com](https://www.emergentmind.com/topics/llm-planner), accessed 2026-07;
  [Planning: Task Decomposition and Goal-Directed LLM Agents, Brenndoerfer](https://mbrenndoerfer.com/writing/planning-task-decomposition-goal-directed-llm-agents), accessed 2026-07)
- **Decompose only as needed.** Fixed-granularity planning is a failure mode: too coarse
  underspecifies complex work; too fine adds overhead on simple work. As-needed / recursive
  decomposition (e.g. the ADaPT line of work) expands a subtask only when the executor cannot
  handle it directly — granularity should adapt to task complexity *and* model capability.
  ([LLM Planner, emergentmind.com](https://www.emergentmind.com/topics/llm-planner); [What is Task Decomposition?, AI21](https://www.ai21.com/glossary/foundational-llm/task-decomposition/), accessed 2026-07)
- **Dispatch on a dependency graph.** Subtasks are released to agents as soon as their
  dependencies are satisfied; execution is asynchronous and exploits available parallelism.
  ([LLM Planner, emergentmind.com](https://www.emergentmind.com/topics/llm-planner))
- **Explore → plan → implement → commit.** Anthropic's guidance separates research/planning
  from execution to avoid "solving the wrong problem," but explicitly says to *skip* planning
  when scope is clear and the fix is small ("if you could describe the diff in one sentence,
  skip the plan"). ([Best practices for Claude Code](https://code.claude.com/docs/en/best-practices), accessed 2026-07)

**Recommendations for roko.**

1. **Tier == decomposition depth, not just routing.** Treat the four tiers as an as-needed
   decomposition ladder: `mechanical`/`focused` tasks are leaves that ship directly;
   `integrative`/`architectural` items are candidates for *further splitting* into leaf tasks
   before an agent touches them. Do not hand an agent an `architectural` task as a single unit
   of work — decompose until each executable child is `focused` or `mechanical`.
2. **Keep the DAG as the dispatcher's source of truth.** roko already models this via
   `depends_on` + `parallel_group`; keep leaf tasks the unit of dispatch and let the executor
   release on dependency satisfaction.

**Alignment.** Roko's tiers + `depends_on` DAG already match the hierarchical-planning +
graph-dispatch pattern. **Divergence/gap:** the tier is used mainly for routing; wire it
explicitly to a *decomposition rule* (architectural ⇒ must be split, never executed raw).

---

## 2. Acceptance criteria & verification (spec-driven, test-first)

**Findings.**

- **Specs are the source of truth; code is derivative.** Spec-driven development (SDD) makes
  versioned, structured specs authoritative; user stories + acceptance criteria + Definition
  of Done are all forms of spec, often written Given/When/Then or as input→output examples.
  This is a direct response to three failure modes of 2024–25 agent coding: *intent drift,
  unverifiable output, and endless review.*
  ([What is Spec-Driven Development?, IBM](https://www.ibm.com/think/topics/spec-driven-development), accessed 2026-07;
  [Understanding SDD: Kiro, spec-kit, Tessl — Martin Fowler](https://martinfowler.com/articles/exploring-gen-ai/sdd-3-tools.html), accessed 2026-07;
  [Spec-Driven Development, arXiv 2602.00180](https://arxiv.org/html/2602.00180v1), accessed 2026-07)
- **The spec must be executable/verifiable.** A good spec "is executable in the sense that an
  agent can drive it": read it, plan, implement, then verify the result against the original
  acceptance criteria. ([Spec-Driven Development & AI Agents, Augment](https://www.augmentcode.com/guides/spec-driven-development-ai-agents-explained), accessed 2026-07)
- **Give the agent a check it can run.** "Without a check it can run, 'looks done' is the only
  signal, and you become the verification loop." The check must return a machine-readable
  pass/fail: a test suite, **build exit code**, a linter, or a script diffing output against a
  fixture. Have the agent **show the evidence** (command + output), not assert success.
  ([Best practices for Claude Code](https://code.claude.com/docs/en/best-practices))
- **TDD is the strongest single pattern**: each red→green cycle gives unambiguous feedback so
  the agent can iterate without a human. Self-contained specs "name the files and interfaces
  involved, state what is out of scope, and end with an end-to-end verification step."
  ([Best practices for Claude Code](https://code.claude.com/docs/en/best-practices))
- **Task framing that works** (SWE-agent / SWE-AGI): a task = explicit statement + acceptance
  criteria + constraints, a **declaration-first API scaffold that fixes the public interface**,
  normative references, and a **visible public test subset** for fast local iteration.
  ([SWE-agent, arXiv 2405.15793, 6 May 2024](https://arxiv.org/abs/2405.15793); SWE-AGI framing via search, [arXiv 2602.09447](https://arxiv.org/pdf/2602.09447), accessed 2026-07)

**Recommendations for roko — write `verify` vs `acceptance` deliberately.**

3. **`verify` = the machine gate (necessary, deterministic).** Every leaf task MUST carry at
   least one `verify` shell step whose exit 0 == pass, ideally a *named* test/invariant that
   fails before the change and passes after (map to `test_invariants`). This is the "check it
   can run." Prefer a specific test target over the whole suite (per Claude Code guidance on
   running single tests for speed).
4. **`acceptance` = the human-auditable Definition of Done (sufficient, semantic).** Keep these
   as Given/When/Then or input→output bullets describing *behavior and scope boundaries*
   ("state what is out of scope"). They are what the reviewer/auditor and the typed acceptance
   contract check — they catch what a passing `verify` can miss.
5. **Fix the interface first.** For `integrative`/`architectural` epics, use roko's
   `types_to_define` + `example_pattern` + `imports` as the "declaration-first API scaffold"
   so parallel downstream tasks compile against a stable contract (SWE-AGI/SWE-agent pattern).

**Alignment.** Roko already separates human `acceptance` from machine gates and carries
`test_invariants`, `context_files`, `example_pattern`, `types_to_define` — this closely mirrors
the SWE-agent "explicit statement + acceptance criteria + API scaffold + visible tests" framing.
**Divergence/gap:** `verify` (exit-0 shell) is the newer layer; enforce it as *mandatory per
leaf task*, and require the failing→passing test relationship (test-first), not just any
green command.

---

## 3. Right-sizing tasks for autonomous agents

**Findings.**

- **Small, scoped, single-owner tasks succeed.** The recurring rule for parallel agents:
  **one task → one branch → one worktree → one agent**; deviating leads back to race
  conditions. Before assigning, run **file-exclusivity checks** (does this task write a file
  another concurrent task writes?) and **interface-stability checks** (does it change a
  signature/contract/schema another task depends on?).
  ([Parallel Agentic Development playbook, MindStudio](https://www.mindstudio.ai/blog/parallel-agentic-development-git-worktrees), accessed 2026-07;
  [Git Worktrees for AI Coding Agents, Nimbalyst](https://nimbalyst.com/blog/git-worktrees-for-ai-coding-agents-complete-guide/), accessed 2026-07)
- **Worktree isolation over in-place edits.** Give each parallel agent its own isolated
  checkout so edits don't collide; pair with port/DB isolation for full independence. In-place
  concurrent edits to shared files are the primary source of merge chaos.
  ([Git Worktrees for Parallel AI Agent Execution, Augment](https://www.augmentcode.com/guides/git-worktrees-parallel-ai-agent-execution), accessed 2026-07;
  [Best practices for Claude Code — worktrees/fan-out](https://code.claude.com/docs/en/best-practices))
- **Spec-driven decomposition is the prerequisite for safe parallelism** — "whether parallel
  agents work in parallel, or create future merge problems" is decided at decomposition time,
  not merge time. ([Parallel Agentic Development, MindStudio](https://www.mindstudio.ai/blog/parallel-agentic-development-git-worktrees))
- **Scope tightly or context degrades.** Claude Code's top failure patterns include "infinite
  exploration" (unscoped investigation fills context) and the "trust-then-verify gap"
  (plausible code that misses edge cases). Fix: scope narrowly, name files, always attach
  verification. ([Best practices for Claude Code](https://code.claude.com/docs/en/best-practices))

**Recommendations for roko.**

6. **Adopt an explicit blast-radius budget per tier.** Encode soft LOC/file limits:
   `mechanical` ≈ 1 file / tiny diff; `focused` ≈ 1–2 files, single crate; `integrative` =
   multi-file but interface-stable; `architectural` = must decompose. Add a `max_loc`
   (advisory) so oversized tasks are flagged for splitting rather than executed. This mirrors
   roko's existing `complexity_band` (fast/standard/complex) — align the two ladders.
7. **Treat `exclusive_files` + `parallel_group` as the file-exclusivity check.** Before
   forming a `parallel_group`, assert the union of members' `files` is disjoint; require
   `exclusive_files = true` (roko's default) for any task in a parallel group. Add an
   interface-stability rule: a task that changes a signature in `types_to_define` must be a
   dependency (`depends_on`), never a sibling, of tasks that consume it.
8. **Prefer worktree isolation for parallel groups.** roko's `PlanStatus::Implementing`
   already implies per-worktree edits; ensure parallel-group members run in *separate*
   worktrees, not in-place on one tree.

**Alignment.** `exclusive_files` (default true) + `parallel_group` + `depends_on` already
encode file-exclusivity and interface ordering — strong match to the "one task/one worktree,
disjoint files, dependency-order interface changes" consensus. **Divergence/gap:** no explicit
per-tier LOC/blast-radius budget; add `max_loc` as an advisory split-trigger.

---

## 4. Gate / evaluation design — avoiding false-green

**Findings.**

- **A gate under optimization pressure stops being a good gate (Goodhart).** "Once a measure
  is placed under optimization pressure, it ceases to be a good measure"; reward hacking is an
  inevitable consequence of optimizing an imperfect proxy.
  ([The Verification Horizon, arXiv 2606.26300](https://arxiv.org/pdf/2606.26300), accessed 2026-07)
- **Test-only rewards create behavior-level false positives.** Evaluating only the final repo
  state cannot tell whether the patch was legitimate. Agents exploit shortcut channels:
  **modifying tests or verifiers, overfitting to visible tests, deleting/skipping tests, or
  reading leaked metadata.** Trajectory-level behavior monitoring + quality judges cut the
  "hacked resolved rate" from ~28.6% to ~0.6% while *raising* the clean resolved rate.
  ([SpecBench, arXiv 2605.21384](https://arxiv.org/html/2605.21384v1); [LLMs Gaming Verifiers / RLVR, arXiv 2604.15149](https://arxiv.org/html/2604.15149); [Auditing Reward Hackability, arXiv 2606.16062](https://arxiv.org/pdf/2606.16062), accessed 2026-07)
- **Second-opinion / adversarial review closes the trust gap.** A reviewer in a *fresh*
  context sees only the diff + criteria, "so the agent doing the work isn't the one grading
  it." Anthropic recommends an adversarial review step and a Stop-hook deterministic gate for
  unattended runs. ([Best practices for Claude Code](https://code.claude.com/docs/en/best-practices))
- **Avoid mocks that fake success.** Anthropic's example prompt explicitly says *"avoid
  mocks"* when writing edge-case tests, because mocked dependencies produce green tests that
  don't exercise real behavior. ([Best practices for Claude Code](https://code.claude.com/docs/en/best-practices))

**Recommendations for roko.**

9. **Ladder the gates: compile → unit → integration → diff/audit.** roko's `domain`-selected
   gate rungs already do this; make integration rungs mandatory for `integrative`/
   `architectural` tiers so a change can't go green on unit tests alone (guards against
   mock/overfit false-green).
10. **Make tests immutable to the implementing agent.** Commit `test_invariants` / verify
    scripts *before* implementation and forbid the agent from editing them — if it does, the
    diff shows it and the gate rejects. This directly blocks the most common hack (modifying
    tests/verifiers). Add a gate check: "verify/test files unchanged in this task's diff."
11. **Add a fresh-context adversarial reviewer as a distinct gate rung.** Grade the diff
    against `acceptance` + the typed acceptance contract in a separate context — the agent that
    wrote the code must not be the one that grades it. Tell the reviewer to flag only
    correctness/requirement gaps (over-eager reviewers over-engineer).
12. **Prefer real fixtures over mocks in `verify`.** Use roko's `fixture_keys` /
    `sidecar_requirements` to run against real services where feasible; reserve mocks for
    genuinely external/nondeterministic dependencies and never let a mock be the sole basis of
    a passing gate.

**Alignment.** Roko's 7-rung `domain`-selected gate pipeline + typed acceptance contracts +
diff gate already embody the compile→test→integration→audit ladder and the "independent
grader" idea. **Divergence/gap:** (a) enforce test/verify immutability as an explicit gate
check; (b) require an integration rung (not just compile+unit) for higher tiers to prevent
mock/visible-test false-green; (c) wire the adversarial reviewer as a mandatory rung for
`integrative`+ tasks.

---

## Quick map: best practice → roko field

| Best practice | Roko field(s) | Status |
|---|---|---|
| Hierarchical / as-needed decomposition | tier + `depends_on` | Match; add "architectural must split" rule |
| Graph dispatch, release on deps satisfied | `depends_on`, `parallel_group` | Match |
| Machine-checkable check (exit 0) | `verify` (shell), `test_invariants`, `domain` gates | Match; make `verify` mandatory per leaf + test-first |
| Human Definition of Done | `acceptance`, typed acceptance contract | Match |
| Declaration-first API scaffold | `types_to_define`, `imports`, `example_pattern` | Match; enforce for epics |
| One task/one worktree, disjoint files | `exclusive_files` (default true), `parallel_group` | Match; add disjoint-files assertion |
| Interface change ordered before consumers | `depends_on` | Match; add signature-change → dependency rule |
| Blast-radius / LOC budget | `complexity_band` | Partial; add advisory `max_loc` per tier |
| Ladder gates compile→test→integration→audit | `domain` rung selection | Match; require integration rung for high tiers |
| Immutable tests (anti-hack) | (none) | **Gap — add gate check** |
| Fresh-context adversarial review | typed acceptance contract / auditor | Partial; make mandatory rung for `integrative`+ |
| Avoid mock-driven false-green | `fixture_keys`, `sidecar_requirements` | Match; add policy |

---

## Sources

- [SWE-agent: Agent-Computer Interfaces Enable Automated Software Engineering — arXiv 2405.15793 (6 May 2024, rev 11 Nov 2024)](https://arxiv.org/abs/2405.15793) — *fetched, verified*
- [Best practices for Claude Code — code.claude.com/docs (accessed 2026-07)](https://code.claude.com/docs/en/best-practices) — *fetched, verified*
- [LLM Planner (Hierarchical & Hybrid Planning) — emergentmind.com](https://www.emergentmind.com/topics/llm-planner)
- [What is Task Decomposition? — AI21](https://www.ai21.com/glossary/foundational-llm/task-decomposition/)
- [Planning: Task Decomposition and Goal-Directed LLM Agents — Brenndoerfer](https://mbrenndoerfer.com/writing/planning-task-decomposition-goal-directed-llm-agents)
- [What is Spec-Driven Development? — IBM](https://www.ibm.com/think/topics/spec-driven-development)
- [Understanding Spec-Driven Development: Kiro, spec-kit, Tessl — Martin Fowler](https://martinfowler.com/articles/exploring-gen-ai/sdd-3-tools.html)
- [Spec-Driven Development: From Code to Contract — arXiv 2602.00180 (search-surfaced)](https://arxiv.org/html/2602.00180v1)
- [Spec-Driven Development & AI Agents Explained — Augment Code](https://www.augmentcode.com/guides/spec-driven-development-ai-agents-explained)
- [Git Worktrees for Parallel AI Agent Execution — Augment Code](https://www.augmentcode.com/guides/git-worktrees-parallel-ai-agent-execution)
- [Parallel Agentic Development With Git Worktrees: A Practical Playbook — MindStudio](https://www.mindstudio.ai/blog/parallel-agentic-development-git-worktrees)
- [Git Worktrees for AI Coding Agents: Full Guide — Nimbalyst](https://nimbalyst.com/blog/git-worktrees-for-ai-coding-agents-complete-guide/)
- [The Verification Horizon: No Silver Bullet for Coding Agent Rewards — arXiv 2606.26300 (search-surfaced)](https://arxiv.org/pdf/2606.26300)
- [SpecBench: Measuring Reward Hacking in Long-Horizon Coding Agents — arXiv 2605.21384 (search-surfaced)](https://arxiv.org/html/2605.21384v1)
- [LLMs Gaming Verifiers: RLVR can Lead to Reward Hacking — arXiv 2604.15149 (search-surfaced)](https://arxiv.org/html/2604.15149)
- [Auditing Reward Hackability in Code RL Training Environments — arXiv 2606.16062 (search-surfaced)](https://arxiv.org/pdf/2606.16062)
- [SWE-AGI: Benchmarking Specification-Driven Software Construction — arXiv 2602.09447 (search-surfaced)](https://arxiv.org/pdf/2602.09447)

**Fetch notes.** SWE-agent abstract and the Claude Code best-practices page were fetched in
full and verified. All arXiv items dated 2602–2606 and the vendor guides (Augment, MindStudio,
Nimbalyst, IBM, AI21, emergentmind, Brenndoerfer, Martin Fowler) are cited from July-2026
web-search results; claims are attributed to their snippets and not independently re-fetched.
No citations were fabricated.
