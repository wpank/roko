Status: stub · budget 950 words · owner gap-ac4646

# 4 The golden path, step by step

[[TODO: Write this section to gap-ac4646's plan, in about 950 words. Waits for: gap-35a614. It contains "Figure 2" and at least 11 tags of the form `TAG@<sha>` that match the appendix. Keep line 1 and set it to `draft`, then replace everything below the heading, this list of claims included. Conventions and canonical numbers: `README.md` in this directory.]]

## Claims

| Id | Kind | Claim | Writer's source | Cite in the text as |
|---|---|---|---|---|
| GP1 | design | Step 1, author: a frontier planner writes the plan, and the author edits and regenerates it quickly. Today the frontier planner is selectable only on CLI paths, the generator truncates the PRD at 8,000 characters and reads at most 5 files, and the best plans were written outside Roko. Principle: DP7. | tldr/04 step 1; B1 | The tag from the appendix; spec-e57870 (E8) |
| GP2 | design | Step 2, compile the spec: exact context, an executor tier, acceptance ids bound to checks, and planner-written tests proven red on the base. Today context packs work, acceptance criteria are prompt text, and pinned `accept/` tests are a hand convention. Principles: DP3, DP4. | tldr/04 step 2; B1; B4 | `render_declared_context`; the tag; spec-e57870 (E8) |
| GP3 | design | Step 3, size and split: task size is set by the executor tier's measured pass rate, and parallel siblings get disjoint write sets. Today the generator aims for the fewest cohesive tasks. Principles: DP1, DP2. | tldr/04 step 3; B1; B2 | A MISSING tag; spec-e57870 (E8); spec-a78d57 (E7) |
| GP4 | design | Step 4, route: the cheapest capable model per task, from a role × tier ladder. Today a person picks the model, and tier names never reach the router. Principles: DP5, DP8. | tldr/04 step 4; B3 | A MISSING tag; spec-98f76d (E5) |
| GP5 | design | Step 5, schedule: ready-queue dispatch, as wide as the DAG and the write sets allow. The ready queue and failure isolation merged on 09-29; the `max_parallel` default and write-set admission are still open. Principle: DP2. | tldr/04 step 5; README "Rows that moved" | Commits `445a60d0d`, `3e7552acd`, `abc1f4b27`; the tag; spec-a78d57 (E7) |
| GP6 | design | Step 6, isolate: each task in its own workspace, with secrets out of reach. Today tasks share the working tree by default, and there is no OS sandbox; the child-environment allowlist merged in `1d923e377`. | tldr/04 step 6; B2; B4 | The tag; bug-7d7200 (closed); spec-a0e40a (E6); spec-ba7bea (E3) |
| GP7 | design | Step 7, verify: visible checks plus checks the agent cannot see or edit. Today only the task's own verify commands run; verdicts are honest since 09-28 (N4), but `Unverified` still counts as success. Principles: DP3, DP4. | tldr/04 step 7; B4; B7 | The tag; spec-e9d7ec (E2); spec-9230a9 (E9); M4 in spec-6ac537 |
| GP8 | design | Step 8, recover: two cheap attempts, then one rung up, then split or replan. Today retries with gate feedback run on the same model, and adaptive thresholds now set retry budgets; escalation was deleted with Runner-v2, and `ReplanController` and `SplitTask` have no caller. Principle: DP5. | tldr/04 step 8; B3; C1 | Commits `99adacd6d`, `41c7ffbd6`; the tag; spec-98f76d (E5); spec-edda86 (E10) |
| GP9 | design | Step 9, integrate: per-task commits on a plan branch, a merge queue, then a whole-plan gate. Today nothing merges on the Graph path (`accept_attempt` and `MergeQueue` have no production caller), and there is no plan-level verify. Principle: DP6. | tldr/04 step 9; B2; B7 | The tags; spec-a0e40a (E6) |
| GP10 | design | Step 10, review: an optional approve-before-merge hold with a per-task diff. Today `--approval` only opens the TUI, and the per-task diff route returns 404 on Graph runs. | tldr/04 step 10; B8 | The tag. No epic in PLAN §3 covers it (tldr/05 proposal 15): name the item, or say that none exists |
| GP11 | design | Step 11, learn: route, size and specify better next time, from verified outcomes. On the portal runs the router saw one pinned model, so it learned nothing about alternatives. | tldr/04 step 11; B5; B7 | The tags from the appendix (several loops were re-wired on 09-29); spec-6ac537 (E17) |
| GP12 | status | A summary table of the 11 steps with their tags, and the plain statement that no real task has yet run on a cheap executor with escalation. | tldr/04 "Where it stands"; B7 | An UNPROVEN tag; N3 |
| GP13 | figure | Figure 2: the loop, with each step marked by its tag. The section carries its caption and a fenced text diagram; gap-d1d92c draws the SVG. | tldr/04 loop table | `figures/fig2-golden-path.svg` |
| GP14 | number | Plan statistics such as "about 106 of 135 plans are sequential" are recomputed from tracked `plans/` or a frozen snapshot, not copied from the tldr. | tldr/04 step 5; B2 | A footnote naming the commit and the command |

## Sources

Writer inputs; the paths under `tmp/` are gitignored and are never cited in the text.

- tldr/04: `tmp/cybernetic-harness/tldr/04-FRONTIER-PLANS-CHEAP-EXECUTES.md`. The 11-step loop, the eight design rules, the real-run numbers, the three-arm test.
- tldr/03: `tmp/cybernetic-harness/tldr/03-MECHANISMS.md`. Every mechanism with basis, status and verdict.
- tldr/05: `tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md`. Scorecard V1-V10, proposals P0-P3, parking, doc corrections, decisions.
- B1: `tmp/cybernetic-harness/tldr/research/B1-plan-authoring.md`. Plan authoring in the code.
- B2: `tmp/cybernetic-harness/tldr/research/B2-dag-worktrees-merge.md`. DAG, worktrees and merge in the code.
- B3: `tmp/cybernetic-harness/tldr/research/B3-routing-cost.md`. Routing and cost in the code.
- B4: `tmp/cybernetic-harness/tldr/research/B4-gates-qa-safety.md`. Gates, QA and safety in the code.
- B5: `tmp/cybernetic-harness/tldr/research/B5-learning-loops-v2-vs-graph.md`. The 16 learning loops, Runner-v2 against Graph.
- B7: `tmp/cybernetic-harness/tldr/research/B7-real-run-evidence.md`. Real-run evidence: the portal build, false greens, concurrency.
- B8: `tmp/cybernetic-harness/tldr/research/B8-domain-agnostic-and-steering.md`. Domains and steering.
- C1: `tmp/cybernetic-harness/tldr/research/C1-research-planning-decomposition-cascades.md`. Literature: planning, decomposition, cascades, integration.
- PLAN: `tmp/cybernetic-harness/workstreams/PLAN.md`. The author's answers (section 1) and the epics (section 3).
- matrix: `docs/whitepaper/appendix-status-matrix.md`. The status matrix (gap-35a614): every tag at one pinned commit.
