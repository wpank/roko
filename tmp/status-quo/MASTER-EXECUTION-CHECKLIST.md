# Roko master execution checklist

> Canonical entry point for long-running, context-free Claude/Codex agents
>
> Repository: /Users/will/dev/nunchi/roko/roko
>
> Initial audit baseline: main at 1649c18b2c3d, audited 2026-07-14
>
> Status: active control document; coordinator and integration agents are its only editors

## 1. Mission and terminal outcome

Drive every accepted item in tmp/status-quo from audited finding or ready task to a
truthful end-to-end result: implemented, tested, independently reviewed, committed,
merged into the designated integration branch, reverified after merge, reflected in
task status and documentation, and free of unaccounted worktrees or dirty changes.

This document is the only starting document an agent needs. It tells the agent what
to read next. It deliberately points to task manifests and primary evidence instead
of copying 575 task definitions into another stale layer.

Do not declare the programme complete merely because code exists, a worker committed,
a narrow test passed, a plan parsed, or a checklist was authored.

DONE means all of the following:

- The complete task acceptance contract is satisfied against current code.
- Every task-specific verify command passes without weakening it.
- Focused tests cover the success, failure, boundary, and recovery behavior appropriate
  to the risk.
- An independent reviewer accepts the exact implementation commit.
- The accepted commit is merged into the configured integration branch.
- The same acceptance checks pass on the post-merge integration commit.
- The task manifest, aggregate count, this checklist, and relevant issue/document
  status are reconciled from that merged proof.
- No required change remains only in an agent branch, worktree, stash, or dirty tree.

SUPERSEDED means a named canonical task fully owns the same outcome, with an explicit
mapping and equivalent-or-stronger acceptance. It never means “we chose not to do it.”

## 2. Current truth at programme start

The July 14 audit found:

| Population | Recorded state |
|---|---|
| Implementation backlog | 6/447 done; 441 ready |
| Self-heal | 26/57 accepted; 9 additional partial; 22 fully open |
| DOC reconciliation | 0/71 done |
| Issue documents | 6 implemented; 21 partial; 46 open; 2 historical |
| Top-level status documents | 96/108 explicitly tied to the old July 8 baseline |
| Backlog strict validation | 23 diagnostics; exit 1 |
| Cross-plan dependencies | 36 noncanonical IDs; 11 external-root references |
| External executable queue incorporated by the roadmap | 0/120 done; 120 ready across P08–P34 plus two side plans |
| Superseded external queue | 66 tasks in self-dev-ux/self-dev-extras; must not execute |

Two old headline defects are fixed: no-flag plan run and resume select Runner v2.
Explicit Graph execution remains a stub.

The root checkout was dirty at audit time, with 18 modified code files, modified
status documents, untracked audit/backlog files, logs, and many existing worktrees.
The first coordinator must re-inspect rather than assume those exact counts remain
current.

## 3. Authority order and context map

When sources disagree, use this authority order:

1. Current production code behavior, tests, Git history, and reproducible runtime proof.
2. This checklist plus accepted per-task evidence on the integration branch.
3. The current task record and acceptance requirements in tasks.toml.
4. The July 14 audit ledgers.
5. Older status-quo prose, roadmaps, checkboxes, and historical claims.

Investigate disagreements and correct stale sources. Never silently select the easier
claim.

Every agent starts here, then reads only the material relevant to its assignment:

| Purpose | Exact path |
|---|---|
| Overall current assessment | tmp/status-quo/audit-2026-07-14/README.md |
| Top-level 108-file ledger | tmp/status-quo/audit-2026-07-14/TOP-LEVEL-DOCUMENT-AUDIT.md |
| Backlog 102-file ledger | tmp/status-quo/audit-2026-07-14/BACKLOG-ROADMAP-AUDIT.md |
| Issue/self-heal 88-file ledger | tmp/status-quo/audit-2026-07-14/ISSUES-SELF-HEAL-AUDIT.md |
| Self-heal accepted-work audit | tmp/status-quo/self-heal/changelog/AUDIT-2026-07-14.md |
| Self-heal issue mapping | tmp/status-quo/self-heal/COVERAGE.md |
| Self-heal manifests | tmp/status-quo/self-heal/plans/*/tasks.toml |
| Backlog schema | tmp/status-quo/backlog/01-TASK-EXECUTION-SCHEMA.md |
| Backlog manifests | tmp/status-quo/backlog/plans/*/tasks.toml |
| Dependency analysis | tmp/status-quo/backlog/10-EPIC-DEPENDENCY-MATRIX.md |
| Existing operations playbook | tmp/status-quo/backlog/11-EXECUTION-PLAYBOOK.md |
| Definition of done | tmp/status-quo/28-DEFINITION-OF-DONE.md |
| Issue evidence | tmp/status-quo/issues/ |
| External P-plan prerequisites | plans/*/tasks.toml |
| External executable-plan index | plans/INDEX.md |
| External queue ordering/history | plans/_meta/IMPLEMENTATION_ORDER.md |
| Source and tests | Cargo.toml, crates/, apps/, demo/, tests/, .github/workflows/ |
| Per-task implementation evidence | tmp/status-quo/execution-evidence/TASK-ID.md |
| Independent review evidence | tmp/status-quo/execution-evidence/TASK-ID-REVIEW.md |

For every assigned task, the worker must read the entire containing tasks.toml,
then every task.context.read_files path, every named source/test file, the mapped
issue documents, and prior evidence. Line ranges in old documents are hints only;
symbols must be rediscovered in current source.

## 4. Launch parameters and authorization

The coordinator records these values before spawning implementation agents:

| Parameter | Required value |
|---|---|
| INTEGRATION_BRANCH | A dedicated local branch, recommended status-quo/integration-YYYYMMDD |
| BASE_SHA | The verified clean baseline commit |
| WORKTREE_ROOT | A sibling directory outside the checkout, recommended ../roko-agent-worktrees |
| MAX_AGENTS | Maximum available agents, bounded by disjoint ready work |
| ALLOW_MAIN_MERGE | yes/no |
| ALLOW_REMOTE_PUSH | yes/no |
| ALLOW_PR_MERGE | yes/no |
| ALLOW_DEPLOY | yes/no |
| ALLOW_EXTERNAL_MUTATION | yes/no |

Defaults when not explicitly supplied:

- Merge locally into the dedicated integration branch.
- Do not merge into main.
- Do not push, merge remote PRs, publish, deploy, rotate secrets, or mutate external
  services.
- Finish through a fully verified local integration commit and record the single
  remaining external action.

This preserves the user’s ability to authorize a fully local end-to-end programme
without accidentally broadening it into production deployment.

## 5. Canonical status and edit ownership

Use this state progression:

NOT_READY → READY → CLAIMED → IMPLEMENTED_UNREVIEWED → REVIEW_ACCEPTED →
MERGED_UNVERIFIED → DONE

A review may also produce REVIEW_REJECTED. Terminal alternatives are BLOCKED and
SUPERSEDED.

Only the coordinator or integration owner changes canonical state in this document
or changes a task from ready to done in tasks.toml. Workers leave tasks ready and
write evidence. This prevents a passing worker branch from masquerading as merged
completion.

The parser accepts pending, ready, active, done, blocked, and skipped task states.
Do not invent another tasks.toml value. Keep a task ready until reviewed integration;
the richer states above live in coordinator/evidence records. The current CLI has no
plan edit command despite stale documents that mention one, so edit manifests with
the normal repository editing workflow.

Only coordinator/integration agents edit:

- This master checklist.
- Aggregate plan counts.
- Shared status indexes and generated roll-ups.
- Central dependency/ownership registries.

Workers edit production/test files in their assigned write scope and create one
evidence file. Reviewers create a separate review file. Documentation workers must
not edit shared indexes unless explicitly assigned.

## 6. Multi-agent scheduling protocol

Use the maximum safe concurrency, not the maximum possible collision count.

When 30 slots are available, start with one coordinator, one integration/release
owner, up to twenty disjoint implementation workers, and up to eight independent
reviewers. Reallocate idle slots between implementation and review as the queue
changes. Reduce concurrency whenever dependencies, write scopes, public APIs, or
shared manifests require serialization; an idle slot is safer than a conflicting
agent.

For every task assignment the coordinator supplies:

| Field | Meaning |
|---|---|
| TASK_ID | Exact stable task ID |
| PLAN_PATH | Exact tasks.toml |
| BASE_SHA | Integrated prerequisite head |
| BRANCH | Unique agent branch |
| WORKTREE | Unique isolated worktree |
| INTEGRATION_BRANCH | Merge target |
| WRITE_SCOPE | Reserved files/directories |
| DEPENDENCIES | Exact merged task/plan commits |
| CONTEXT | Issues, audits, symbols, and read files |
| ACCEPTANCE | Exact verify commands and additional risk gates |
| EVIDENCE_PATH | Unique worker evidence file |
| REVIEWER | Independent agent/branch |

Before dispatch:

1. Confirm dependencies are DONE on the integration branch.
2. Compare files arrays and public API surfaces with every active assignment.
3. Serialize any overlap in runner event/state/persistence, roko-core public types,
   roko-serve router/auth, workspace Cargo.toml, shared plan manifests, or deletions.
4. Create or verify the branch/worktree from BASE_SHA.
5. Record one owner and one reviewer.
6. Reserve at least one agent slot for review/integration when capacity permits.

Never self-schedule all 55 backlog plans as one root. The current dependency defects
can deadlock a clean run, and logical DAG parallelism is not Git-safe parallelism.

High-conflict surfaces that require one merge owner:

- crates/roko-cli/src/runner/event_loop.rs
- runner state, types, persistence, ownership, and resume modules
- crates/roko-core public types, traits, and config schemas
- crates/roko-serve route/auth assembly
- workspace Cargo.toml and crate registration
- master/status/plan manifests
- E12 deletion and migration paths

## 7. Git, worktree, commit, and merge protocol

### 7.1 Seal the starting checkout

Never use git reset --hard, git clean, blanket stash, checkout of paths, worktree
prune, or deletion of unknown untracked files.

Also never:

- Work directly in the dirty root checkout after it has been sealed.
- Run git add -A or git add .; stage named files only.
- Remove or force-remove a worktree this programme did not create.
- Modify another agent’s branch/worktree.
- Commit transient logs, secrets, runtime state, or unexplained generated files.
- Rewrite shared history or force-push.

Create a recovery bundle outside the repository before making an integration
worktree. Use a unique RUN_ID:

~~~sh
export REPO=/Users/will/dev/nunchi/roko/roko
export RUN_ID=status-quo-$(date -u +%Y%m%dT%H%M%SZ)
export RUN_STATE="$HOME/.local/state/roko/$RUN_ID"
export RUN_ROOT=/Users/will/dev/nunchi/roko/agent-worktrees/$RUN_ID

mkdir -p "$RUN_STATE" "$RUN_ROOT/workers" "$RUN_ROOT/reviews" "$RUN_ROOT/logs"
git -C "$REPO" rev-parse HEAD > "$RUN_STATE/original-head.txt"
git -C "$REPO" branch --show-current > "$RUN_STATE/original-branch.txt"
git -C "$REPO" status --porcelain=v2 --branch > "$RUN_STATE/original-status.txt"
git -C "$REPO" diff --binary HEAD > "$RUN_STATE/tracked-working-tree.patch"
git -C "$REPO" diff --cached --binary > "$RUN_STATE/staged.patch"
git -C "$REPO" ls-files --others --exclude-standard -z > "$RUN_STATE/untracked-files.zlist"
git -C "$REPO" worktree list --porcelain > "$RUN_STATE/preexisting-worktrees.txt"
git -C "$REPO" bundle create "$RUN_STATE/repository.bundle" --all
shasum -a 256 "$RUN_STATE"/*.patch "$RUN_STATE/repository.bundle" \
  > "$RUN_STATE/SHA256SUMS"

export BASE_SHA=$(git -C "$REPO" rev-parse HEAD)
export INTEGRATION_BRANCH=status-quo/integration-$RUN_ID
export INTEGRATION_WT=$RUN_ROOT/integration
git -C "$REPO" worktree add -b "$INTEGRATION_BRANCH" "$INTEGRATION_WT" "$BASE_SHA"
~~~

The untracked-file list is an inventory, not a backup of file contents. Archive any
untracked file that contains unique intended work into RUN_STATE in named groups;
if a broken symlink or changing log prevents archival, record it and continue with
the remaining paths. Never delete/move the original to make archival succeed.

The clean integration branch begins at committed HEAD and does not contain the dirty
delta. Reconstruct or apply only one attributed task-sized subset in a worker
worktree, then review and merge it like any other task. Never apply the whole patch
as one commit.

The first coordinator must:

- [ ] Record git status --short, branch, HEAD, git worktree list, and active agent branches.
- [ ] Save a binary diff backup outside the repository before reorganizing dirty work.
- [ ] Attribute every tracked modification to a task, known prior batch, or unrelated user work.
- [ ] Inventory untracked files; exclude transient logs/symlinks unless intentionally required.
- [ ] Reconstruct coherent existing changes on attributed worker branches and merge
      them into the integration branch only after independent review.
- [ ] Preserve unrelated work without rewriting or absorbing it.
- [ ] Run the relevant focused suites plus formatting and workspace all-target check.
- [ ] Record the clean/deliberately-documented BASE_SHA.
- [ ] Seal the original checkout: later agents may read it but never edit, stage,
      commit, switch, stash, reset, or clean it.

A dirty tree that can be safely isolated is not a blocker.

### 7.2 Worker branches

Recommended naming:

~~~text
agent/TASK-ID-short-purpose
review/TASK-ID
fix/TASK-ID-review-N
~~~

Recommended worktree location:

~~~text
/Users/will/dev/nunchi/roko/agent-worktrees/RUN-ID/workers/TASK-ID
~~~

Do not blindly create worktrees under .claude/worktrees: many already exist. The
coordinator must inspect git worktree list and choose a collision-free location.

The coordinator creates each worker from the current integration tip and records
the exact start SHA before handing it to an agent:

~~~sh
export TASK_ID=SH01-T06C4
export TASK_BRANCH=agent/$TASK_ID-short-purpose
export TASK_WT=$RUN_ROOT/workers/$TASK_ID
export TASK_START_SHA=$(git -C "$INTEGRATION_WT" rev-parse HEAD)
git -C "$REPO" worktree add -b "$TASK_BRANCH" "$TASK_WT" "$TASK_START_SHA"
git -C "$TASK_WT" status --short --branch
~~~

### 7.3 Commits

Each production commit must:

- Contain one coherent task or review fix.
- Include TASK_ID in the subject.
- Avoid unrelated formatting or generated noise.
- Preserve bisectability.
- Name migration/compatibility behavior when relevant.

Suggested subjects:

~~~text
fix(runner): SH02-T03 require durable task commits
feat(auth): E04-T02 deny unknown mutating routes
test(recovery): SH06-T02 cover dirty exact resume
docs(status): DOC-status-quo reconcile engine truth
~~~

Stage only declared files, inspect the staged patch, and include the task ID:

~~~sh
git -C "$TASK_WT" diff --check
git -C "$TASK_WT" add -- path/to/declared-file path/to/declared-test
git -C "$TASK_WT" diff --cached --stat
git -C "$TASK_WT" diff --cached
git -C "$TASK_WT" commit -m "$TASK_ID: concise outcome"
~~~

Evidence/status reconciliation may be a separate coordinator commit. Do not hide
semantic conflict resolution inside bookkeeping.

### 7.4 Independent review and integration

A worker cannot accept its own work.

The reviewer evaluates the exact immutable candidate commit independently. Create a
review branch at that SHA; the only permitted review-branch edit is its evidence
file (plus reviewer-authored adversarial tests only if separately assigned):

~~~sh
export CANDIDATE_SHA=$(git -C "$TASK_WT" rev-parse HEAD)
export CANDIDATE_SHORT=$(git -C "$TASK_WT" rev-parse --short=12 HEAD)
export REVIEW_BRANCH=review/$TASK_ID-$CANDIDATE_SHORT
export REVIEW_WT=$RUN_ROOT/reviews/$TASK_ID-$CANDIDATE_SHORT
git -C "$REPO" worktree add -b "$REVIEW_BRANCH" "$REVIEW_WT" "$CANDIDATE_SHA"
git -C "$REVIEW_WT" status --short --branch
~~~

After independently reaching ACCEPTED, the reviewer commits only the review record:

~~~sh
git -C "$REVIEW_WT" add -- tmp/status-quo/execution-evidence/$TASK_ID-REVIEW.md
git -C "$REVIEW_WT" diff --cached
git -C "$REVIEW_WT" commit -m "review($TASK_ID): accept $CANDIDATE_SHORT"
~~~

A rejected review is sent back with exact findings and retained until its replacement
review captures the disposition. It is never merged as acceptance.

The integration owner then:

1. Confirms prerequisites and ACCEPTED review.
2. Merges in dependency order with auditable history.
3. Resolves conflicts semantically, never mechanically with ours/theirs.
4. Obtains renewed review when conflict resolution changes behavior.
5. Runs targeted checks after each risky merge.
6. Runs the wave gate on the integrated head.
7. Changes manifest/master status only after post-merge proof.
8. Records implementation, review, and integration commits.

Default local integration preserves the reviewed task commits:

~~~sh
git -C "$INTEGRATION_WT" status --porcelain=v1
git -C "$INTEGRATION_WT" merge --no-ff "$REVIEW_BRANCH" \
  -m "merge($TASK_ID): integrate accepted task"
~~~

If a merge conflicts, abort it. The owning worker updates from the integration
branch, resolves the semantic conflict in its worktree, reruns acceptance, and gets
the new candidate commit independently reviewed. The integrator must not improvise
behavioral conflict resolution.

After merge, require the reviewed commit to be an ancestor of the integration branch
and rerun acceptance there before status changes.

Merge conflicts and ordinary regressions are work, not blockers.

### 7.5 Cleanup and final target merge

Only the coordinator removes worktrees created by this RUN_ID, and only after the
candidate/review evidence is committed, the worktree is clean, and the accepted
candidate is an ancestor of the integration branch. Never remove a pre-existing
worktree. Retain task branches until final release proof is complete.

Merging the integration branch into main is permitted only when ALLOW_MAIN_MERGE=yes,
the original checkout has been fully reconciled and is clean, and release gates pass.
Otherwise a verified local integration commit is the terminal authorized result.
When authorized and safe:

~~~sh
git -C "$REPO" status --porcelain=v1
git -C "$REPO" switch main
git -C "$REPO" merge --no-ff "$INTEGRATION_BRANCH" \
  -m "merge(status-quo): complete audited remediation programme"
~~~

Rerun final release proof on main. Push/tag/publish/deploy only when its separate
launch flag explicitly authorizes that exact action.

## 8. Evidence contract

Every worker creates tmp/status-quo/execution-evidence/TASK-ID.md:

~~~markdown
# TASK-ID implementation evidence

Assignment:
- Plan:
- Base SHA:
- Branch/worktree:
- Integration branch:
- Reserved write scope:

Requirement:
- Original defect or missing behavior:
- Acceptance requirements:
- Explicit non-goals:
- Dependencies and their integration commits:

Reproduction:
- Pre-fix command:
- Expected:
- Actual:

Implementation:
- Design and invariants:
- Files/symbols changed:
- Compatibility/migration:
- Failure/recovery/security behavior:

Verification:
- Command:
- Exit/result:
- Command:
- Exit/result:

Review readiness:
- Implementation commit:
- Diff scope reviewed:
- Known limitations:
- Required reviewer focus:

Integration:
- Review evidence:
- Integration commit:
- Post-merge commands/results:
- Final status:
~~~

Every reviewer creates TASK-ID-REVIEW.md with candidate commit, independent
reproduction, changed-line/production-path review, commands/results, adversarial
checks, verdict, confidence, and exact required next action.

A status-only reconciliation for already-present code still requires a fresh evidence
file, exact implementation commit(s), complete acceptance, independent review, and
post-merge verification.

## 9. Blocker and persistence contract

Retry the same failed operation no more than three times. Every retry must test a
materially different hypothesis or remedy. Do not loop on unchanged external state.

Valid blockers:

- Missing credentials/permission for a required external system.
- A destructive, irreversible, security-sensitive, or public-API decision that the
  repository cannot resolve.
- An unmet prerequisite actively owned elsewhere.
- An unavoidable file/branch ownership collision.
- Reproducible infrastructure failure after three distinct remedies.
- A required external service is unavailable and no faithful local substitute exists.

Not blockers:

- Code complexity or task size.
- A failing test.
- Reviewer rejection.
- Lack of prior context.
- Uncertainty answerable from the repository.
- A safely isolatable dirty worktree.
- Context-window limits.

A blocked evidence record must include task, current commit, exact command/error,
three remedies, required outside action, resumption command, and independent work
that can continue.

Before context exhaustion, commit only safe coherent work or leave the branch
unchanged, update evidence with the exact next action, reread this document and the
task manifest in a fresh context, and continue. The coordinator keeps other eligible
lanes active.

## 10. Dependency-ordered master checklist

Only coordinator/integration agents check these boxes after merged verification.

### Wave 0 — preserve truth and repair the control plane

These may use parallel analysis/implementation agents after the baseline is safely
sealed.

- [x] CTRL-01 Preserve and attribute the existing dirty checkout; establish BASE_SHA.
- [ ] CTRL-02 Review, test, split, commit, and integrate the 18-file July 14 precursor work.
- [x] CTRL-03 Canonicalize 36 invalid depends_on_plan names to exact meta.plan IDs.
- [x] CTRL-04 Resolve 11 external dependencies on P08/P09/P16/P19/P22/P23/P25/P28.
- [x] CTRL-05 Resolve E11’s absent architecture-core-queue prerequisite.
- [x] CTRL-06 Make strict validation distinguish intended creation outputs from missing prerequisites.
- [x] CTRL-07 Reduce backlog strict validation from 23 diagnostics to zero without placeholders.
- [x] CTRL-08 Deduplicate overlapping E01/SH/E02/E09/E14/E46–E48 ownership.
- [x] CTRL-09 Convert DOC-v2-core into an acceptance roll-up or supersede duplicate product tasks.
- [x] CTRL-10 Establish execution-evidence and reviewer evidence conventions.
- [x] CTRL-11 Rebuild target/debug/roko from integrated current source.
- [x] CTRL-12 Confirm self-heal and backlog strict validation both exit zero.
- [x] CTRL-13 Confirm zero unresolved internal/external plan IDs for the chosen execution root.
- [x] CTRL-14 Verify status-quo-authoring-gaps remains explicitly superseded (96/96
      skipped), with per-epic plan coverage proving why it must never execute.
- [x] CTRL-15 Reconcile all 120 executable tasks represented by plans/INDEX.md into
      canonical epic/self-heal ownership or an explicitly retained plan task.
- [ ] CTRL-16 Remove or repair stale implementation-order references to absent
      architecture-core-queue, dry-run-flag, and live-demo-phase1/phase2 plan roots;
      do not manufacture empty plan directories.

Wave gate:

~~~sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
target/debug/roko plan validate --strict tmp/status-quo/self-heal/plans
target/debug/roko plan validate --strict tmp/status-quo/backlog/plans
~~~

### Cross-wave ledger — external P08–P34 and side queues

The status-quo roadmap incorporates these 120 tasks even though their manifests live
under plans/. For each entry, inspect the full manifest and current code, then either
complete it or supersede each task with a named canonical SH/E task and
equivalent-or-stronger merged proof. A shallow/duplicate plan must not run raw, but
it also must not remain silently ready. The adjacent wave is the initial ownership
hint; Wave 0 may correct it from actual file/dependency evidence.

- [ ] P08-search-command-fix (4) — plans/P08-search-command-fix/tasks.toml; Wave 8/E16.
- [ ] P09-tool-alias-fix (3) — plans/P09-tool-alias-fix/tasks.toml; Wave 8/E14/E16.
- [ ] P10-slash-command-flags (5) — plans/P10-slash-command-flags/tasks.toml; Wave 9/E10.
- [ ] P11-runner-v2-default (5) — plans/P11-runner-v2-default/tasks.toml; Wave 7/E01.
- [ ] P12-runner-parallelism (5) — plans/P12-runner-parallelism/tasks.toml; Waves 2/7 SH02/E01.
- [ ] P13-rate-limit-retry (4) — plans/P13-rate-limit-retry/tasks.toml; Waves 5/8 SH05/E14.
- [ ] P14-gate-rung-fix (3) — plans/P14-gate-rung-fix/tasks.toml; Wave 8/E05.
- [ ] P15-error-recovery-wiring (5) — plans/P15-error-recovery-wiring/tasks.toml; Waves 2/7 SH02/E01.
- [ ] P16-safety-contracts (5) — plans/P16-safety-contracts/tasks.toml; Wave 7/E04.
- [ ] P17-cli-output-format (6) — plans/P17-cli-output-format/tasks.toml; Waves 4/9 SH04/E10.
- [ ] P18-tui-agent-data (5) — plans/P18-tui-agent-data/tasks.toml; Waves 4/9 SH04/E10.
- [ ] P19-cascade-router-acp (6) — plans/P19-cascade-router-acp/tasks.toml; Wave 10/E17.
- [ ] P20-zero-config (5) — plans/P20-zero-config/tasks.toml; Wave 8/E15/E18.
- [ ] P21-acp-streaming (5) — plans/P21-acp-streaming/tasks.toml; Wave 10/E17.
- [ ] P22-acp-tool-permission (5) — plans/P22-acp-tool-permission/tasks.toml; Wave 7/E04/E17.
- [ ] P23-prd-pipeline-fix (6) — plans/P23-prd-pipeline-fix/tasks.toml; Wave 8/E16.
- [ ] P24-workspace-paths (4) — plans/P24-workspace-paths/tasks.toml; Waves 8/10 E18/E43.
- [ ] P25-mcp-acp-passthrough (4) — plans/P25-mcp-acp-passthrough/tasks.toml; Wave 10/E17.
- [ ] P26-hdc-similarity-lookup (4) — plans/P26-hdc-similarity-lookup/tasks.toml; Waves 9/10 E07/E24.
- [ ] P27-provider-error-ux (4) — plans/P27-provider-error-ux/tasks.toml; Wave 8/E14.
- [ ] P28-image-support (5) — plans/P28-image-support/tasks.toml; Wave 10/E17.
- [ ] P29-develop-command-wire (3) — plans/P29-develop-command-wire/tasks.toml; Wave 10/E18.
- [ ] P30-onboarding-doctor (4) — plans/P30-onboarding-doctor/tasks.toml; Wave 10/E18.
- [ ] P31-note-and-context (3) — plans/P31-note-and-context/tasks.toml; Wave 9/E07.
- [ ] P32-cli-polish (2) — plans/P32-cli-polish/tasks.toml; Waves 9/10 E10/E18.
- [ ] P33-model-ux (1) — plans/P33-model-ux/tasks.toml; Wave 8/E14.
- [ ] P34-verification-sweep (4) — plans/P34-verification-sweep/tasks.toml; final Wave 13 gate.
- [ ] architecture-defi-critical-path (3) — plans/architecture-defi-critical-path/tasks.toml;
      after E11 restores or replaces its missing architecture-core prerequisite.
- [ ] e2e-smoke (2) — plans/e2e-smoke/tasks.toml; final Wave 13 gate.
- [ ] self-dev-ux (55) and self-dev-extras (11) remain superseded, with every
      acceptance outcome mapped into the executable queue; never execute them raw.

The ledger is closed only when plans/INDEX.md truthfully reports no unexplained ready
work and P34/e2e-smoke pass against the final integrated product.

### Wave 1 — finish SH01 lifecycle truth

Single runner owner; do not parallelize these shared paths.

Plan: tmp/status-quo/self-heal/plans/SH01-runner-lifecycle/tasks.toml

- [x] SH01-T06C4 Expire lost effects and close timeout races.
- [ ] SH01-T07 Reconcile truthful run and plan summaries.
- [ ] SH01 reads 28/28 done after review, merge, and post-merge verification.
- [ ] Issues 06, 42, 46, 47, and 64 have precise merged dispositions/evidence.

### Wave 2 — SH02 isolation, commits, and crash recovery

Keep max_parallel=1 until the full plan proves otherwise.

Plan: tmp/status-quo/self-heal/plans/SH02-isolation-recovery/tasks.toml

- [ ] SH02-T01 Enforce effective per-plan concurrency.
- [ ] SH02-T02 Create task-owned worktrees and immutable gate inputs.
- [ ] SH02-T05 Replace spawn polling with queued capacity wakeups.
- [ ] SH02-T03 Require a durable task commit before success.
- [ ] SH02-T04 Make worktree resume/reacquisition idempotent.
- [ ] SH02-T06 Clean crash locks and recover dirty worktrees.
- [ ] SH02 reads 6/6 done after integrated acceptance.
- [ ] Concurrency remains capped until the SH06 release gate passes.

Parallel lane allowed: E04 relay/auth tasks confined to roko-serve. Do not overlap
agent-safety/runner files without explicit reservations.

### Wave 3 — SH03 persistence and replay integrity

Plan: tmp/status-quo/self-heal/plans/SH03-persistence-integrity/tasks.toml

- [ ] SH03-T01 Persist complete terminal snapshots after reconciliation.
- [ ] SH03-T02 Add rotating transition checkpoints.
- [ ] SH03-T03 Make the lifecycle ledger complete and idempotent.
- [ ] SH03-T05 Recover and clean atomic-write debris after T02.
- [ ] SH03-T04 Repair fresh-run seeded task semantics after T01/T03.
- [ ] SH03-T06 Make StateHub publication ordered/recoverable after T03.
- [ ] SH03 reads 6/6 done after integrated acceptance.

T02/T03 and T04/T06 are logically parallel, but must serialize if persistence files
or public event contracts overlap.

### Wave 4 — SH04 runtime telemetry and connected TUI

Plan: tmp/status-quo/self-heal/plans/SH04-runtime-telemetry-tui/tasks.toml

- [ ] SH04-T01 Use structured agent identity and attribution.
- [ ] SH04-T02 Preserve typed output channels and severity.
- [ ] SH04-T06 Fix dashboard route layout and phase invariants.
- [ ] SH04-T03 Connect approval TUI to structured runner events.
- [ ] SH04-T04 Expose preflight progress and runner diagnoses.
- [ ] SH04-T05 Add agent liveness and estimated/final token reconciliation.
- [ ] SH04-T08 Write operational event-health logs.
- [ ] SH04-T07 Make Git refresh asynchronous and bounded.
- [ ] SH04 reads 8/8 done after integrated acceptance.

T07 may proceed independently after SH03. Other tasks parallelize only with disjoint
reserved TUI/projection/bridge files.

### Wave 5 — SH05 config, dispatch, supervision, and budgets

Plan: tmp/status-quo/self-heal/plans/SH05-config-dispatch/tasks.toml

- [ ] SH05-T01 Fail fast on ambiguous model configuration.
- [ ] SH05-T02 Normalize dispatch lifecycle and transient retry.
- [ ] SH05-T03 Harden process supervision and cancellation.
- [ ] SH05-T04 Enforce and attribute cost budgets.
- [ ] SH05 reads 4/4 done after integrated acceptance.

T03/T04 may be parallel only if file reservations are disjoint.

### Wave 6 — SH06 release gate

Plan: tmp/status-quo/self-heal/plans/SH06-regression-harness/tasks.toml

- [ ] SH06-T01 Build deterministic crash-chain replay fixture.
- [ ] SH06-T03 Test connected TUI responsiveness and invariants.
- [ ] SH06-T02 Test interruption, dirty worktree, and exact resume after T01.
- [ ] SH06-T04 Run subsystem regression/quality gates after T01/T02/T03.
- [ ] SH06-T05 Prove Roko completes its own deterministic smoke repair.
- [ ] SH06 reads 5/5 done.
- [ ] Self-heal reads 57/57 done.
- [ ] Autonomous concurrency increase is explicitly approved by this integrated proof.

Required suite includes the task manifest commands plus:

~~~sh
cargo test -p roko-cli --test runner_crash_recovery -- --nocapture
cargo test -p roko-cli --test resume_cycle_e2e
cargo test -p roko-cli --test tui_tabs
cargo test -p roko-core dashboard_snapshot
cargo test -p roko-cli -p roko-runtime -p roko-agent -p roko-core -p roko-serve
~~~

If a named test target does not yet exist, its creation is required work, not a
reason to delete the gate.

### Wave 7 — reconcile E01 and finish security

- [ ] Audit E01-T07–T16 against SH02/SH05 and E46–E48.
- [ ] Complete or explicitly supersede every E01 task with equivalent proof.
- [ ] E01-execution-engine reads 16/16 done or has reviewed supersession mappings.
- [ ] Complete all 19 E04-security-perimeter tasks.
- [ ] Relay HTTP and WS routes require authentication.
- [ ] Unknown mutating routes deny by default.
- [ ] Route/scope manifest is generated and tested.
- [ ] Default Claude CLI execution has a proved safety boundary.
- [ ] ACP mutation tools fail closed pending permission.
- [ ] Scrubbing blocks secrets without issue 66 false positives.
- [ ] Custody records detect tampering.
- [ ] Security adversarial integration tests pass post-merge.

Plans:

- tmp/status-quo/backlog/plans/E01-execution-engine/tasks.toml
- tmp/status-quo/backlog/plans/E04-security-perimeter/tasks.toml
- plans/P16-safety-contracts/tasks.toml
- plans/P22-acp-tool-permission/tasks.toml

### Wave 8 — correctness and convergence tracks

After Wave 7, run disjoint tracks with one owner/worktree per plan.

Track A, sequential:

- [ ] E03-type-consolidation — 7 tasks.
- [ ] E02-STORAGE-CONVERGENCE — 12 tasks, after E03.
- [ ] E05-gate-adaptivity-live — 8 tasks, after E01 and E02 where declared.
- [ ] E06-COMPOSE-UNIFY — 9 tasks, after E01 and SH foundations.

Track B, sequential:

- [ ] E14-providers-tools — 12 tasks.
- [ ] E15-mcp-config — 7 tasks.
- [ ] Reconcile/complete P08-search-command-fix — 4 tasks.
- [ ] Reconcile/complete P09-tool-alias-fix — 3 tasks.
- [ ] Reconcile/complete P23-prd-pipeline-fix — 6 tasks.
- [ ] E16-prd-self-hosting-gaps — 2 tasks after those parents.

Track C:

- [ ] E18-DOCS-CONFIG-OPS implementation tasks T01–T09 and T14.
- [ ] Defer E18 documentation tasks T10–T13/T15 until final truth convergence.

Wave gate:

- [ ] One canonical durable path per concern.
- [ ] Complete state migration/recovery proof.
- [ ] No positive learning from stub/skipped gates.
- [ ] Canonical prompt path is used by Runner v2.
- [ ] Provider/tool parity and bounded retry pass.
- [ ] MCP tools/config/env reach the actual agent.
- [ ] PRD-to-parseable-plan smoke succeeds.

### Wave 9 — kernel and completeness foundations

Eligible parallel roots, subject to file reservations:

- [ ] E07-learning-knowledge — 10 tasks.
- [ ] E08-conductor-supervision — 9 tasks.
- [ ] E09-OBSERVABILITY — 11 tasks.
- [ ] E10-FRONTEND-CONTRACT — 7 tasks after E03.
- [ ] E11-chain-isfr prerequisite/design recovery — 5 tasks.
- [ ] E19-signal-protocol — 10 tasks.
- [ ] E20-cell-unification — 10 tasks.

E19/E20 both touch roko-core. Freeze public vocabulary/trait ownership first or run
them serially.

Then:

- [ ] E21-graph-engine — 10 tasks after E19/E20.
- [ ] E22-execution-runtime — 10 tasks after E21.
- [ ] Explicit Graph execution dispatches real work or truthfully refuses unsupported behavior.
- [ ] Graph snapshots, replay, cancellation, gates, and budgets pass end to end.

Long-horizon spec debt is deliberately non-gating and starts only after its actual
parents settle:

- [ ] E13-T01 defines Lens/LensScope after E09-T09.
- [ ] E13-T02 adapts MetricRegistry after E13-T01 and E09-T01.
- [ ] E13-T03 records the Cell/Block naming decision after E01; it performs no rename.
- [ ] E13-SPEC-DEBT-V2 reads 3/3 done and does not absorb work owned by other epics.

### Wave 10 — agent and infrastructure expansion

Run only after the named parents are DONE:

- [ ] E23-agent-cognitive-autonomy — 10 tasks after E19/E20/E22.
- [ ] E24-memory-advanced — 10 tasks after E07/E22.
- [ ] E25-learning-loops-advanced — 10 tasks after E07.
- [ ] E26-inference-gateway — 12 tasks after E14; one workspace/crate owner.
- [ ] E27-feeds-system — 10 tasks after E19/E20.
- [ ] E28-groups-coordination — 8 tasks after E20.
- [ ] E29-connectivity-relay — 9 tasks after E04.
- [ ] E30-extension-system — 9 tasks after E20.
- [ ] E31-trigger-system — 8 tasks after E08.
- [ ] E32-tool-plugin-ecosystem — 8 tasks after E14/E15.
- [ ] E33-telemetry-lens — 9 tasks after E09/E10.
- [ ] E34-security-ifc — 8 tasks after E04.
- [ ] E35-auth-protocol — 8 tasks after E04.
- [ ] E42-config-evolution — 8 tasks after E19.
- [ ] E44-cross-cut-functors — 8 tasks after E19/E20.
- [ ] E37-surfaces — 9 tasks after E09/E33.
- [ ] E43-deployment-portability — 8 tasks after E18 implementation.
- [ ] Reconcile/complete P19-cascade-router-acp — 6 tasks.
- [ ] Reconcile/complete P25-mcp-acp-passthrough — 4 tasks.
- [ ] Reconcile/complete P28-image-support — 5 tasks.
- [ ] E17-acp-completion — 8 tasks after E04/E07/E14/P19/P22/P25/P28.

Operational epics with corrected real dependencies:

- [ ] E46-github-workflow-integration — 12 tasks after E01/E04/E15.
- [ ] E47-resource-disk-management — 11 tasks after E01/E02.
- [ ] E48-rate-limit-budgeting — 12 tasks after E14/E26.
- [ ] Operational overlap has one implementation owner and no duplicate mechanisms.

### Wave 11 — economy and chain

After E11 and relevant infrastructure:

- [ ] E39-registries-identity — 8 tasks after E11.
- [ ] E36-payments — 8 tasks after E11/E29.
- [ ] E40-arenas-evals — 8 tasks after E25/E39.
- [ ] E41-defi-products — 8 tasks after E11/E39.
- [ ] E38-marketplace — 9 tasks after E36/E39.

Parallelize leaf features, but use one owner for chain schemas/contracts and shared
registry types.

### Wave 12 — destructive cleanup and Mori parity

Plan: tmp/status-quo/backlog/plans/E12-DEAD-CODE-CLEANUP/tasks.toml

- [ ] E12 T01–T05 and T09 pass consumer audits and named prerequisites.
- [ ] E12-T06 runs only after E01/E04/E08.
- [ ] E12-T07 runs only after E05/E06/E08.
- [ ] E12-T08 runs only after T07.
- [ ] Every deletion has full workspace proof before and after its own commit.
- [ ] E12 reads 9/9 done.
- [ ] E45-orchestrator-mori-parity — 10 tasks after E01/E12.
- [ ] No legacy behavior remains solely in deleted/quarantined code.

Never combine unrelated legacy deletions in one unverifiable commit.

### Wave 13 — documentation and final truth convergence

Implementation must stabilize before current-reference rewrites. Historical files get
clear baseline/supersession banners rather than rewritten history.

DOC reconciliation plans:

- [ ] DOC-status-quo-corpus — 12 tasks.
- [ ] DOC-v1-kernel — 8 tasks.
- [ ] DOC-v1-cognition — 7 tasks after canonical dependency repair.
- [ ] DOC-v1-ecosystem — 10 tasks.
- [ ] DOC-v2-depth — 24 tasks.
- [ ] DOC-v2-core is a deduplicated acceptance roll-up, not a second implementation stream.
- [ ] E18 T10–T13/T15 complete after product truth stabilizes.
- [ ] All 108 original top-level documents have a current or historical disposition.
- [ ] Issues 60–67 are added to self-heal coverage.
- [ ] Issue states derive from merged task evidence.
- [ ] Counts derive from manifests/scripts with one owner.
- [ ] CLI help, README, deployment examples, engine/resume semantics, and paths agree.
- [ ] All local Markdown links and anchors resolve.
- [ ] Generated manifests are regenerated and drift-checked in CI.

## 11. Milestone and release gates

At every wave:

- Task-specific verify commands.
- Affected-crate format/check/test.
- Focused regressions for the behavior.
- Independent review.
- Post-merge rerun.
- No unaccounted dirty state.

At self-heal, major milestone, and final release boundaries:

~~~sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
target/debug/roko plan validate --strict tmp/status-quo/self-heal/plans
target/debug/roko plan validate --strict tmp/status-quo/backlog/plans
~~~

Frontend where affected:

~~~sh
cd demo/demo-app
npm run build
npm run e2e
~~~

Final behavioral proof:

- [ ] Default plan run performs real work and never fabricates success.
- [ ] Explicit Graph execution is real or truthfully unsupported.
- [ ] Crash/restart/resume converges to the uninterrupted result.
- [ ] Task attempts own immutable worktrees/diffs/commits through terminalization.
- [ ] No orphan processes, branches, locks, claims, or worktrees remain.
- [ ] Relay and unknown mutations fail closed without authorization.
- [ ] Safety, cost, rate, and resource limits are enforced before side effects.
- [ ] Connected TUI/API/CLI projections agree with durable state.
- [ ] Fresh deterministic self-host repair produces owned changes, gates, commits,
      terminal ledger/snapshot, and clean resume.
- [ ] Deployment and clean-checkout instructions work as documented.
- [ ] Documentation describes the exact integration commit.
- [ ] Every required task is DONE or explicitly SUPERSEDED with proof.
- [ ] Every accepted branch is merged; integration worktree is clean.
- [ ] If authorized, integration is merged into main and remote/release identifiers recorded.

Do not suppress warnings, ignore tests, relax thresholds, or remove gates merely to
obtain green. Existing warning debt is work to resolve before the final -D warnings
gate can pass.

## 12. Copy/paste assignment envelope

The coordinator fills this and appends the worker or reviewer prompt below it:

~~~text
ROKO ASSIGNMENT
ROLE=<implementation|review|integration|docs>
TASK_ID=<exact task ID>
PLAN_PATH=<exact tasks.toml>
BASE_SHA=<verified integration head>
BRANCH=<unique branch>
WORKTREE=<absolute path>
INTEGRATION_BRANCH=<target>
WRITE_SCOPE=<reserved files/directories>
DEPENDENCIES=<task IDs and integration commits>
CONTEXT=<audit/issue/evidence paths>
ACCEPTANCE=<exact commands/gates>
EVIDENCE_PATH=<unique path>
REMOTE_ACTIONS_AUTHORIZED=<yes/no and exact scope>
~~~

## 13. Coordinator prompt

~~~text
You are the execution coordinator for the Roko remediation programme.

Repository:
/Users/will/dev/nunchi/roko/roko

FIRST read this entire canonical document:
tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md

Assume no other context. The checklist is the live control plane; current code,
tests, and Git history remain implementation truth.

Keep the programme running until every in-scope task reaches a truthful terminal
outcome. Use the maximum safe available concurrency, but dispatch siblings only
when prerequisites are integrated and write sets/public APIs are disjoint.

Inspect branch, HEAD, dirty state, existing worktrees/branches, manifests, evidence,
and checklist before dispatch. Preserve unknown work. Repair control-plane defects
in Wave 0 before broad execution. Assign each task one worker, one independent
reviewer, one branch/worktree, one reserved write set, exact acceptance, and evidence
paths. Never let workers edit the master or approve themselves.

Merge in dependency order into the configured integration branch. Reverify after
every merge. Update canonical state only after post-merge proof. Keep independent
work flowing while a task is validly blocked. Retry an unchanged failure no more
than three times with distinct remedies.

Continue across agent turns and context windows. Before context exhaustion, leave
a durable coordinator checkpoint in this document and continue in a fresh context.
Do not stop because a wave is large, tests fail, review rejects work, or prior
context is missing.

Declare completion only when all required tasks are DONE or explicitly SUPERSEDED,
all accepted work is merged, all release gates pass on the integrated tree,
documentation describes that exact tree, and no unaccounted dirty/unmerged work
remains.

Do not push, merge remote PRs, deploy, publish, rotate secrets, or mutate external
systems unless the launch parameters explicitly authorize the exact action.
~~~

## 14. Implementation worker prompt

~~~text
You own one Roko implementation task.

Repository:
/Users/will/dev/nunchi/roko/roko

Use the filled ROKO ASSIGNMENT envelope provided above.

FIRST read all of:
1. tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md
2. The entire assigned PLAN_PATH
3. The exact TASK_ID record and every task.context.read_files path
4. Mapped issue/audit/prior-evidence paths
5. Current source/tests/Git history in the assignment

Assume no prior context. Before editing, restate the defect, expected behavior,
acceptance, dependencies, non-goals, and reserved write scope in your evidence file.

Work only in the assigned worktree/branch. Preserve unrelated changes. Do not edit
the master checklist or mark the task done.

Reproduce the defect or missing behavior where possible. Add/identify a regression
that fails for the correct reason. Inspect adjacent abstractions. Implement the
smallest coherent production design that closes the root cause, including applicable
failure, recovery, concurrency, persistence, security, observability, configuration,
and compatibility behavior. Do not add parallel frameworks, fake success, silent
fallbacks, TODO-only paths, test-only implementations, or unowned stubs.

Run every task verify command, focused tests, affected-crate checks/tests, formatting,
and broader risk-proportionate gates. Never weaken a failing test or acceptance
command. If metadata is stale, prove it and repair it minimally with equal-or-stronger
proof.

Review the diff for scope, secrets, debug output, panics, ignored errors, races,
unbounded growth, nondeterminism, and unnecessary API change. Commit coherent work
with TASK_ID in the subject. Complete the evidence file with exact commands/results
and implementation commit.

Obtain independent review. Address every rejection and rerun checks. Follow the
task through integration fixes when asked. A worker commit is not DONE.

Continue until the task is merged and post-merge verified or validly BLOCKED under
the master contract. Complexity, failing tests, review rejection, or context limits
are not blockers. Never discard unknown work, force-push, rewrite shared history, or
perform unauthorized remote/external actions.
~~~

## 15. Independent reviewer prompt

~~~text
You are the independent acceptance reviewer for one Roko task. You did not implement
it and must not rely on the worker summary.

Repository:
/Users/will/dev/nunchi/roko/roko

Use the filled ROKO ASSIGNMENT envelope. FIRST read:
- tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md
- The exact task/plan and primary issue/audit evidence
- Dependency commits
- Worker evidence
- Candidate diff, relevant unchanged call sites, and tests

Write tmp/status-quo/execution-evidence/TASK-ID-REVIEW.md.

Reconstruct the requirement from primary evidence. Reproduce the original defect
when practical. Inspect every changed line and trace the production path end to end.
Check correctness, error semantics, lifecycle ownership, concurrency, persistence,
restart, security, resources, compatibility, observability, and maintainability as
applicable.

Look for test-only fixes, weakened assertions, ignored failures, fake success, dead
code, TODOs, unsafe defaults, hidden fallbacks, duplicate abstractions, and behavior
not exercised by tests. Run acceptance independently and add adversarial tests when
risk warrants.

Return exactly ACCEPTED, REJECTED, or BLOCKED. ACCEPTED means no required next action.
REJECTED includes file/symbol references, reproduction, expected/actual behavior,
severity, and smallest correction. BLOCKED must meet the master blocker contract.

Do not edit production code, the master, or task status. Send required corrections
back to the worker, then review the new immutable candidate. Continue
review/correction cycles until ACCEPTED or valid BLOCKED.
~~~

## 16. Integration and release prompt

~~~text
You own integration and release gates for one Roko wave/milestone.

Repository:
/Users/will/dev/nunchi/roko/roko

FIRST read tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md, then the assigned wave,
candidate/review evidence, dependency commits, manifests, and current Git graph.

Only you and the coordinator may update the master and canonical task counts.

Require each candidate to have integrated prerequisites, an exact implementation
commit, independent ACCEPTED review of that commit, complete evidence, and no
unexplained dirty files. Merge in dependency order with auditable history. Resolve
conflicts semantically; renew review if behavior changes.

Run targeted checks after each risky merge, then all wave/milestone gates. Treat
regressions as implementation work: isolate, fix, independently review, merge, and
rerun. Never suppress warnings, skip tests, or relax thresholds.

Update task/meta/master state only after post-merge evidence. Record integration
commits and commands. Check unmerged accepted branches, orphan worktrees, stale
evidence, duplicates, unresolved blockers, and dirty state before closing a wave.

Continue until the wave is fully integrated/verified or all remaining paths are
validly blocked. Remote actions require explicit launch authorization; otherwise
finish at a verified local integration commit without claiming remote release.
~~~

## 17. Documentation reconciliation prompt

~~~text
You are a Roko documentation reconciliation agent.

Repository:
/Users/will/dev/nunchi/roko/roko

Use the assignment envelope. FIRST read:
- tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md
- The assigned DOC task and source-coverage ledger
- tmp/status-quo/audit-2026-07-14/TOP-LEVEL-DOCUMENT-AUDIT.md
- Linked merged implementation/review evidence
- Current source, tests, CLI help, schemas/config, and Git commits

Confirm implementation dependencies are integrated. Never document branch-only or
speculative behavior as current. Derive facts from current code and reproducible
commands. Use old audits to locate drift, not as unverified truth.

Preserve historical snapshots with clear baseline/supersession banners. Update
current references completely: behavior, commands, paths, defaults, counts, diagrams,
examples, limitations, and migration notes. Designate canonical sources and remove
contradictory duplicate claims.

Never claim implemented, fixed, secure, production-ready, or complete without merged
proof. Verify runnable examples, links, anchors, paths, hashes, engine/resume claims,
CLI flags, totals, and labels. Time-sensitive external facts require current primary
official sources recorded with date.

Do not edit shared indexes or the master unless assigned. Commit coherent DOC-task
changes, obtain independent review, merge, and rerun link/task acceptance. Continue
through review/integration fixes; corpus size and stale prose are not blockers.
~~~

## 18. Coordinator continuation checkpoint

Before ending any coordinator context, update this section on the integration branch:

- Run ID: `status-quo-20260714T073140Z`
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`
- Integration HEAD before this checkpoint commit: `cd185001b`
- Base SHA: `3041d095d4daebed2c9e05c63eacb18e668e37e3`
- Worktree root: `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z`
- Maximum agents: 4 (one coordinator plus three reusable worker/review/integration slots)
- Authorization: `ALLOW_MAIN_MERGE=yes`; `ALLOW_REMOTE_PUSH=no`; `ALLOW_PR_MERGE=no`; `ALLOW_DEPLOY=no`; `ALLOW_EXTERNAL_MUTATION=no`
- Last completed wave/task: `CTRL-15` is DONE after corrected independent acceptance, ordered integration (`736290140`, `e355800f4`, `d98b15af0`), and current post-merge proof. Wave 0 is active; only `CTRL-02` and `CTRL-16` remain open, with seven bounded precursor clusters integrated for CTRL-02.
- Active assignments and worktrees: file-disjoint CTRL-02 StateHub/SSE and config-dispatch precursor reconstructions are in implementation. CTRL-16 r1 was rejected by `b59e497e7` for stale live-demo script calls and incomplete residue mapping; its bounded correction is ready to resume.
- Accepted but unmerged commits: none. The historical deadline candidate `c2f3f18fb945` and review `739750232d54` remain retained evidence but were superseded by the reviewed integration-based reconstruction after merge preflight found their semantic conflict.
- Current blockers and exact resumption commands: the original checkout's existing `.git` is read-only in this environment, so the integration branch is owned by `coordinator.git`; retry final import only after `git -C /Users/will/dev/nunchi/roko/roko branch <probe> 3041d095d` can create and delete a local probe branch without `Operation not permitted`.
- Next dependency-ready tasks: correct and freshly review CTRL-16, review/merge the two active CTRL-02 clusters, continue the remaining dependency-ordered precursor attributions recorded by the CTRL-02 census, and advance SH01-T07 as the next serialized event-loop writer only after Wave 0.
- Last global gate results: integrated CTRL-15 post-merge proof reports 93 plans, 881 tasks, 345 raw/169 unique task-runtime edges plus 2 raw/2 unique metadata edges (347/171 all-declared), 849 same-plan references, zero unresolved references/SCCs, an exact 120-row ownership bijection (99 retained/21 roll-ups), separate 24-task architecture queue, and 193/193 parsed TOMLs. Disposable strict results are backlog `0/55`, self-heal `0/6`, and top-level 94 bounded `PLAN_031` diagnostics; regenerated and tracked `plans/INDEX.md` both have SHA-256 `27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8` and report 30 executable plans/144 tasks. After the six integrated CTRL-02 precursor clusters, post-merge focused gates pass for runtime ledger `7/7`, event loop `51/51`, projection `7/7`, TUI `245/245`, atomic writes `7/7`, persistence `8/8`, `cargo check -p roko-cli --all-targets`, formatting, and diff cleanliness. The shared binary must be rebuilt after the latest source integrations before release proof.
- Dirty/untracked state: the sealed original remains unchanged. Its standard inventory contains 23 attributed visible control-plane paths, 56 ignored canonical backlog manifests, and 15 preserved unrelated artifacts. The later-discovered top-level P08-P34/side queue, `.roko/GAPS.md`, and five architecture source copies are preserved in external `ignored-canonical-control-plane.tar.gz` (SHA-256 `01c10b4565c1a897c92ced109c7f351fcb35513816860d094efa446da62c34e0`); 34 sealed files plus one canonical recovered architecture queue are reviewed and merged, and CTRL-15 now reconciles the historical/current count boundary. The obsolete uncommitted CTRL-09 draft was byte-proved equivalent to the accepted canonical blobs, restored with `apply_patch`, and its clean worktree removed. Integration was clean at `cd185001b` before this coordinator-only checkpoint edit; every active worker remains confined to its recorded scope.
- Remote actions still unauthorized/required: all push, PR merge, deploy, publish, secret rotation, and external-service mutation remain unauthorized; none is required for local programme execution.

The next coordinator must reread the full document, verify this checkpoint against
Git and evidence, then continue. The checkpoint is a hint, never a substitute for
repository truth.
