# Case studies from real runs

> Curated from `field-notes.jsonl` and `snapshots/`. Each case is a short, verifiable story the papers can tell: what
> happened, the evidence, who closed the loop (Roko or a person), and which claim it supports or undercuts. Add a
> case only when its evidence paths resolve. Keep each case under 150 words.

| Id | Title | Date | Plans | Who closed the loop | Paper use | Evidence |
|---|---|---|---|---|---|---|
| CASE-001 | A failed task logged as passed, then honest verdicts | 09-25 → 09-28 | 01 | operator → operator | RQ3; main §1 | FN-20260925-004, FN-20260929-026 |
| CASE-002 | A sibling's half-written file fails a whole-project gate | 09-28 | 08b, 08d | operator → operator (69 min) | V4; main §8 | FN-20260928-030, `2026-09-28/…08b-portal-polish…` |
| CASE-003 | One subscription: session limits and a failover stripped at load | 09-26 → 09-28 | 02, 05 | operator → operator | V3, V7; RQ1 | FN-20260926-016, FN-20260928-013 |
| CASE-004 | Turn caps and timeouts set by guesswork | 09-25 → 09-29 | 01, 03, 05, 03c, 08f | operator → operator; last one roko → roko | main §8 | FN-20260928-012, FN-20260929-022 |
| CASE-005 | Parallel lanes, worktrees and merges made by hand | 09-28 → 09-29 | 03-08e | operator → operator | V5, V4 | FN-20260929-019, FN-20260929-029 |
| CASE-006 | Gates green, product unusable: the browser-pass loop | 09-28 → 09-29 | 05-08g | browser-pass → roko | V6 | FN-20260928-026, `2026-09-28/…08-portal-run…` |
| CASE-007 | Plan defects that only frontier audits caught | 09-25 → 09-28 | 01, 02, 03, 05, 06 | audit → operator | V2; RQ1 | FN-20260926-008, FN-20260926-011 |
| CASE-008 | Role policy denied the tools tasks needed | 09-21 → 09-28 | 01 | operator → operator | RQ2; V2 | FN-20260925-007, FN-20260928-010 |

Abbreviations: `D25` = `tmp/dogfood/2026-09-25-portal-programme-run.md`, `D28` =
`tmp/dogfood/2026-09-28-portal-programme-continuation.md`, `B7` = `tmp/cybernetic-harness/tldr/research/B7-real-run-evidence.md`.
Snapshot directories are relative to `snapshots/`; commits and work items (`work/items/`) are in the roko repo. Backfilled
snapshots keep only each plan's latest run, and for 05 and 08b only the final passing attempts, so failures named below
are often visible only in the notes and logs.

### CASE-001: A failed task logged as passed, then honest verdicts
- **What happened:** Plan 01 T01 (an architect task denied write tools) failed its verify three times, but was
  force-accepted at the review-cycle cap and logged "all graph verify steps passed" over a "BLOCK — implementation not
  applied" verdict. Episodes recorded success; resume would have replayed the BLOCK as output. The 09-28 engine fix
  deleted force-accept, checkpointed verdicts and settled episodes after the gate.
- **Detected by / fixed by:** operator / operator (workaround 09-25, engine fix 09-28).
- **Evidence:** FN-20260925-004/005/006/009/017/018, FN-20260929-026/027; commit 725f21e05; bug-82d47b, bug-521f08,
  bug-06e2d1; D25 P0-1, P0-3, P3-2. The 09-25 run has no snapshot (`2026-09-26/roko__01-backend-plan-service__73179b41`
  is the re-run).
- **Claim it bears on:** companion RQ3: 101/373 recorded successes (27%) had a failing gate before the fix, 0/151 after
  (B7); main-paper §1.
- **What Roko would need to have closed the loop itself:** honest verdicts (built 09-28), plus a run-level check that
  outcomes match the gate log.

### CASE-002: A sibling's half-written file fails a whole-project gate
- **What happened:** Plan 08b ran four tasks in one tree. T08's whole-project `tsc --noEmit` failed all three attempts on
  sibling T12's half-edited `PlanView.tsx`, and FailFast skipped T09, T15 and T16. The operator saw it 69 minutes later,
  resumed, and had settle-and-reverify added to the engine; 08d then ran four wide with no false failures.
- **Detected by / fixed by:** operator / operator (69 min to detect).
- **Evidence:** FN-20260928-030, FN-20260928-031; `2026-09-28/roko-portal2-wt__08b-portal-polish__c3ecdde4` (final
  attempts only), `2026-09-28/roko-portal2-wt__08d-portal-legibility__bd43f566`; commit 3049b7fcf; gap-4f3063; D28
  §18:40, §21:10.
- **Claim it bears on:** V4; main-paper §8, one regulator moving from the operator into Roko.
- **What Roko would need to have closed the loop itself:** settle-and-reverify (built), plus a supervisor that reports a
  non-zero exit at once (gap-ebd656).

### CASE-003: One subscription: session limits and a failover stripped at load
- **What happened:** Every portal attempt ran on one Claude CLI subscription. On 09-26 its limit stopped plan 02 (T08 failed
  three times in 2 s each, logged only as `exit 1`). On 09-28 roko detected the limit and quarantined the provider, but
  failover never fired: the config loader had silently stripped `routing.fallback_models`. The supervising session's own
  subagents stopped on the same limit.
- **Detected by / fixed by:** operator / operator; roko detected the exhaustion on 09-28, and a subagent found the loader
  bug about 30 min later.
- **Evidence:** FN-20260926-016, FN-20260928-013/014, FN-20260929-030; `2026-09-26/roko__02-backend-plan-execution__3a9f0f08`,
  `2026-09-28/roko-portal-wt__05-portal-foundation__adcb6046`; commit 725f21e05; find-229e9c; D28 §14:09.
- **Claim it bears on:** V3, V7 (no cheap-model evidence); companion RQ1.
- **What Roko would need to have closed the loop itself:** config tests that load real TOML, and unpinned models so that
  routing has a second arm.

### CASE-004: Turn caps and timeouts set by guesswork
- **What happened:** On 09-25 turns were unbounded (T02: 312 s, 304,769 cache-read tokens). The 09-28 caps (10/10/20/30)
  made 3 of plan 05's first 8 attempts exit 1, recorded as `turns: 0`; the operator raised them to 40/60/90/120. The
  600 s timeout killed 01-T10 and 08f-T05 twice, and operators raised timeouts by hand. On 09-29 roko itself resumed
  03c-T20 with its cap raised from 90 to 135.
- **Detected by / fixed by:** operator / operator; for 03c-T20, roko / roko.
- **Evidence:** FN-20260925-015, FN-20260928-012, FN-20260926-003, FN-20260926-014, FN-20260929-008, FN-20260929-022;
  `2026-09-29/roko-backend-wt__03c-backend-local-access__4ba59f34` (holds T20's failed attempt); gap-3870d9, find-43768e.
- **Claim it bears on:** main-paper §8 (autonomy index); B7 failure mode 6.
- **What Roko would need to have closed the loop itself:** caps and timeouts set from the p95 of successful tasks in each
  tier, with classified resumption (built for turn caps, not for timeouts).

### CASE-005: Parallel lanes, worktrees and merges made by hand
- **What happened:** The 09-28 parallel-plan design kept one shared tree to save disk. In practice the operator built four
  worktrees with pinned binaries in tmux, pruned disk twice to fit them, and merged five times by hand. Three merges had
  semantic breaks the text merge could not see (`GraphPlanRunParams`, `AgentOptions`, DTO fields), and 59 files needed
  rustfmt because no lane gate ran it.
- **Detected by / fixed by:** operator / operator.
- **Evidence:** FN-20260928-033/011/017/042/019/039/032, FN-20260929-019/020/024/021/029; commits 3d0ee4d02, f7c542b5a,
  98e0af4d6, 4ca38b5c8, 188c43c8d; gap-3b5361, bug-a3760a; `tmp/portal-audit/06-PARALLEL-PLANS.md` §1–2.
- **Claim it bears on:** V5; V4 (the mean concurrency of 1.59 came from the operator's lanes).
- **What Roko would need to have closed the loop itself:** a worktree per plan, plus an integration gate (fmt, clippy,
  tests) on the merged tree.

### CASE-006: Gates green, product unusable: the browser-pass loop
- **What happened:** Plans 05–08 passed every typecheck, unit-test and build gate. A browser preview against a real server
  then found about 20 defects, including a blocker: nothing could run twice. Later passes found about 12, 5 and 4; the
  first three each became a plan that roko built, and 08d turned legibility into a computed WCAG-AA gate. On 09-29 a
  browser smoke found three more hello-world blockers.
- **Detected by / fixed by:** browser-pass / roko; the plans were authored from the findings by the operator's subagent.
- **Evidence:** FN-20260928-026/027/034/035/036/037/038, FN-20260929-001; `2026-09-28/roko-portal-wt__08-portal-run__cc747bbb`,
  `2026-09-28/roko-portal2-wt__08e-portal-refine__92438855`; commits 54704f451, dabd1975f, 2bf9b05bb, a17d9d766.
- **Claim it bears on:** V6 (gates check parts, not the product).
- **What Roko would need to have closed the loop itself:** a default final gate for UI plans: a browser drive against
  the real server, with the real event order.

### CASE-007: Plan defects that only frontier audits caught
- **What happened:** Two audit rounds before plans 02–08 ran found that plan 02 crossed a crate boundary the wrong way (T03
  could never compile; FailFast would skip T04–T09). They also found two gates that correct code would fail, two vacuous
  gates, and a regression shipped past plan 01's single-file gate. That was about 65 edits to 87 tasks. The backend
  plans' 36 references to a gitignored file passed `--strict` in the main tree and failed in a worktree.
- **Detected by / fixed by:** audit, operator / operator; roko's validator caught only the gitignored file.
- **Evidence:** FN-20260926-006/008/009/010/011, FN-20260925-019, FN-20260928-020;
  `2026-09-26/roko__02-backend-plan-execution__3a9f0f08`; find-70edcb; D25 Addendum 2 (R-1..R-6).
- **Claim it bears on:** V2; companion RQ1.
- **What Roko would need to have closed the loop itself:** a `plan validate` that runs each verify step on the base tree
  (vacuity), checks crate boundaries and runs in the plan's own tree.

### CASE-008: Role policy denied the tools tasks needed
- **What happened:** Architect tasks were denied write tools at three layers, while `plan_generate.rs` told authors that
  architects can write; quick-reviewer fell back to deny-all. Plan 01 T01 wrote a review instead of code (CASE-001). The
  operator rewrote 14 of 87 tasks as implementer; the plan-generation role table now comes from the contracts, and a new
  rule (PLAN_036) rejects read-only roles that own files. On 09-28 implementers on API models could never edit: tool-loop
  reads were not recorded.
- **Detected by / fixed by:** operator / operator.
- **Evidence:** FN-20260925-007, FN-20260928-010, FN-20260921-001 (09-21 validator gap, open); bug-f43cf8,
  bug-e37197, bug-db607b; commit 725f21e05; D25 P0-2.
- **Claim it bears on:** companion RQ2 (documented role table contradicted enforcement); V2.
- **What Roko would need to have closed the loop itself:** one source of truth for role capability, and a validator rule
  tying each task's files and verify steps to its role's tools.

## Template

```
### CASE-NNN: <title>
- **What happened:** two or three sentences.
- **Detected by / fixed by:** roko | operator | user | browser-pass … (minutes to detect, minutes to fix)
- **Evidence:** snapshot dir, field-note ids, commits, work items, dogfood log section
- **Claim it bears on:** e.g. V6 (gates check parts, not the product), companion RQ3, main-paper §8
- **What Roko would need to have closed the loop itself:** the mechanism (e.g. whole-plan gate, M4 audit), or "none plausible"
```
