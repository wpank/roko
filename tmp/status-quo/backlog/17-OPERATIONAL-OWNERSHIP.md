# Operational ownership and acceptance roll-ups

> Canonical CTRL-08 ownership record. This document supersedes title-based overlap
> guesses in the July 14 audit; manifests remain the executable source of status.
> Corrected after independent rejections `b0e21f69f427e738a7198f43ad5d827cf0b7c486`
> and `87461143496d405a0c3a0adffa9bfa2c278f1bc6`, and
> `ac3cfb8439bd4223663759360ad73a7b18461419`, then reconstructed against
> integration `dd611500e7f9051fbdd3843cd20c5472efcfcbb7`.

## Contract

One task owns each implementation mechanism. A record marked
`ownership = "acceptance-roll-up"` stays `ready` until its named canonical owner is
integrated, then executes only the existing equivalent-or-stronger verification and
acceptance checks as a reviewer. It has no write files and cannot create a parallel
mechanism. `superseded_by` names the exact owner. Distinct consumers retain
implementation scope but declare `uses_owner`; they adapt a canonical mechanism to a
different surface and must not reimplement it.

Informational ownership fields do not schedule anything. Every cross-plan roll-up
uses runtime-enforced `depends_on_plan`; local multi-owner outcomes additionally use
`depends_on`. The combined `plans/`, backlog, and self-heal graph must have no
strongly connected component before dispatch. A backlog run containing SH-backed
E01 roll-ups must include or already have durably completed the named self-heal plan.

No task is marked done or skipped by this reconciliation. All plan totals, task IDs,
and task statuses are preserved.

## Exact ownership matrix

| Behavior / output | Canonical implementation owner | Acceptance roll-up(s) | Distinct retained work |
|---|---|---|---|
| Task-owned worktree, immutable gate input, and plan aggregation | `SH02-T02` for task attempts; `E01-T14` for the accepted-task aggregation branch | `E01-T07`, after both runtime dependencies | `E46-T06` consumes the aggregation branch for GitHub lifecycle; `E47-T06` cleans terminal/orphan worktrees. Neither may create a second worktree ownership model. |
| Symmetric transient dispatch lifecycle and runner retry | `SH05-T02` | `E01-T12` | `E48-T01` wires the lower provider-call `RetryPolicy`; `E14-T09` parses backend `Retry-After` headers. |
| Resumable pre-dispatch cost halt and exact attribution | `SH05-T04` | `E01-T11` | `E48-T04` adds explicit override/legacy adapters; `E48-T08/T09/T10/T12` allocate, project, route, and present budgets; `E17-T07` adapts the canonical state to ACP. |
| GitHub MCP discovery, precedence, and environment config | `E15-T7` | `E01-T13` | `E14-T11` owns catalog metadata; `E46` owns GitHub API/PR/issue operations; `E18-T15` documents their integrated result. |
| Configurable plan-lifecycle GC and pre-run disk refusal | `E47-T03`, `E47-T04` | `E01-T16`, `E01-T15` | `E47-T01/T02` own config/query primitives; `E47-T11` composes cleanup hooks. |
| Runtime disk pressure and live worktree count | `E47-T08` owns disk pressure; `E47-T09` owns aggregate disk accounting/serialization and the exact `name=worktree_count`, `value=WorktreeManager::active_count` Metric producer on legacy and runner-v2 paths | `E08-T08` for disk pressure | `E08-T09` is the sole `React::decide` warning consumer of the worktree-count Metric and owns only its `WatcherThresholds.worktree_count` config; `E09-T11` publishes target size using `E47-T05`. `disk_budget_remaining` is never a worktree count. |
| Episodes/signals/efficiency JSONL rotation | `E47-T07` using `ResourcesConfig.log_rotation_max_mb` (default 100 MB) | `E09-T10` | E47 owns complete timestamped JSONL-generation discovery by readers/GC. `E02-T07` owns retention for events, operational logs, ledger, and backups; `E09-T05/T07` own daily operational-log rotation and bounded event replay. |
| Ephemeral feed/chain heartbeats with durable critical events | `E09-T04` | `E02-T08` | Dashboard/live broadcast remains part of the `E09-T04` acceptance contract; no second serve-side persistence filter is allowed. |
| Per-provider RPM/TPM accounting and live configured enforcement | `E48-T02` defines the canonical limiter/config; `E48-T03` pools and acquires it on both runner model-call paths | `E14-T08`, after both owners | `E17-T08` consumes health/rate state in ACP. ToolDispatcher is not a provider request boundary. |
| Provider-health circuit state, real model outcomes, and CascadeRouter selection | `E48-T05` using one runtime-scoped `Arc<ProviderHealthRegistry>` across legacy dispatch, runner-v2, and routing, with a trait-typed ModelCallService adapter that preserves the dependency direction | `E14-T10`; local durable-recovery acceptance `E48-T11` | `E48-T06/T10` queue retries and apply budget downgrade; `E17-T08` adapts canonical routing status to ACP. No ToolDispatcher outcome mapping, roko-agent→roko-learn production edge, second tracker, EMA, or health score is allowed. |
| GitHub response quota tracking | `E14-T12` | none | Reuse the `E46-T03` client and publish into the E14/E48 health vocabulary; this is not ordinary LLM RPM/TPM accounting. |
| Target directory measurement | `E47-T05` scanner API | none | `E09-T11` alone owns the Prometheus gauge and slow scheduling; it must call the shared scanner rather than walk `target/` independently. |
| CI plan validation and GitHub operator documentation | `E18-T14`, `E18-T15` | none | `E46` owns runtime GitHub workflow operations; E18 owns CI policy and documentation only. |

## Shared API and file serialization

These boundaries are mandatory even when DAG dependencies permit concurrency:

| Hot surface | Sole implementation lane | Later consumers |
|---|---|---|
| `runner/event_loop.rs`, runner ownership/state | SH01/SH02/SH05 in wave order | E46/E47/E48 adapters only after the named SH owner is integrated |
| `worktree_count` Metric Engram | `E47-T09` using `WorktreeManager::active_count` on both runner paths after SH05 | `E08-T09` pure React warning consumer |
| `roko-core` config vocabulary | E48 for provider quotas/health; E47 for resources; E08-T09, after E47, for only `WatcherThresholds.worktree_count` | E14 acceptance and E17 ACP consume provider state; no lane redefines another lane's fields |
| `CascadeRouter` provider-health choice and exact registry instance receiving LLM success/429/529/timeout outcomes | `E48-T05` | E14 acceptance, E48 later actions/outcomes, and E17 ACP selection |
| `roko-fs` disk scan/rotation | `E47-T02/T05/T07` | E09 metrics and E02 retention consume |
| GitHub MCP configuration | `E15-T7` | E01 acceptance, E46 operations, E18 docs |
| GitHub API client and remote mutations | `E46-T03/T05/T06` | E14 quota observation and E18 docs |

## Preserved non-overlaps

The following similarly named work is intentionally distinct:

- E02 storage convergence owns canonical paths, migrations, cold archival, and
  retention; E47 owns disk-pressure mechanics and cleanup execution.
- E09 owns metrics, event persistence policy, and observability exports; E47 owns
  reusable disk scanning and JSONL rotation.
- E14 owns provider/backend primitives and acceptance of provider-health routing;
  E48-T02/T03 jointly own configured live limiter enforcement and E48-T05 owns the
  canonical health-registry identity across actual LLM call outcomes and routing,
  cross-agent pooling,
  queueing, budget policy, projections, and orchestration consumers.
- E15 owns MCP configuration/discovery; E46 owns authenticated GitHub operations.
- E17 owns ACP-specific enforcement and presentation adapters; it must consume the
  canonical SH/E14/E48 mechanisms.
- E18 owns CI and documentation truth, not product mechanisms.

Any future task that needs a mapped output must depend on or consume the owner above;
adding another implementation under a roll-up ID is a control-plane regression.
