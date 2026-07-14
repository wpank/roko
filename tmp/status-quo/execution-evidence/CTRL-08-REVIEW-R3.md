# CTRL-08 r3 independent final review

- **Verdict:** `REJECTED`
- **Candidate:** `ec3ecf2f89f0dd74a6c5e973c9ea4c7185bec30e`
- **Corrected base:** `d0942fc63ef734017736294843e9112b78e8a656`
- **Prior rejected candidates/reviews:** `b9387fe6c3f42209a317a301302b027a6b882042` /
  `b0e21f69f427e738a7198f43ad5d827cf0b7c486`; and
  `ff6dc54afeccf4d06ebd95e476756d2383422205` /
  `87461143496d405a0c3a0adffa9bfa2c278f1bc6`
- **Current integration checked:** `dd611500e7f9051fbdd3843cd20c5472efcfcbb7`
- **Review branch:** `review/CTRL-08-ec3ecf2f-final`
- **Review date:** 2026-07-14

## Independent method and scope

I read the complete master checklist, both prior rejected reviews, the corrected
ownership matrix/evidence/audit, every touched manifest and prose change, the live
task parser/scheduler, and the relevant conductor, worktree, provider-health,
model-call, tool-dispatch, and legacy/runner-v2 production paths. I inspected the
base-to-candidate diff and recreated the counts, dependency graph, writer-order,
strict-validation, scope, and integration checks without a worker script or worker
archive.

The candidate is the direct child of its stated base and changes exactly 24 paths:
13 manifests, eight epic documents, one dated-audit notice, the new ownership
matrix, and worker evidence. It changes no production source, tests, master,
shared index, lockfile, or top-level plan index. `git diff --check` passes.

## Reproduced controls and disposition of the seven review findings

The candidate materially corrects the E08-T09 defect from the r2 review. E47-T09 is
now the sole manifest producer of a `Kind::Metric` Engram tagged
`name=worktree_count`, with its value explicitly taken from
`WorktreeManager::active_count()` at lifecycle/admission transitions in both legacy
PlanRunner and runner-v2. E08-T09 is a pure `React::decide` consumer, depends on the
E47 plan, consumes only that exact metric, emits a warning
`conductor.intervention`, and owns only its threshold configuration. The live code
confirms `WorktreeManager::active_count`, `React::decide`, `Kind::Metric`, and the
tagged intervention pattern are real APIs; there is no conductor `Watcher` trait.

The first five r1 findings are also materially corrected: the E14/E48 direction is
acyclic; all eleven roll-ups have scheduler-recognized plan dependencies; E02-T08
reviews the canonical StateHub boundary; E08-T08 uses the React/ResourcesConfig
contract; and E01-T07 covers both SH02 task isolation and E01-T14 plan aggregation.
The rotation and SH05 task prose/verification are stronger than the rejected
candidate. The provider/rate producer portion of the sixth finding remains
incomplete, however, as detailed below.

Independent reproduction produced:

```text
combined plans: 93 (32 top-level + 55 backlog + 6 self-heal)
unique meta/task plan edges: 135
unresolved local task references: 0
unresolved plan references: 0
cyclic strongly connected components: 0
changed manifests: 13
changed task ID/order/status drift: 0
changed meta plan/total/done/status drift: 0
backlog + self-heal statuses: 33 done, 542 ready, 96 skipped
acceptance roll-ups: 11; each files=[], role=quick-reviewer, with a plan dependency
unordered ready-task same-file pairs in changed max_parallel > 1 plans: 0
tracked TOMLs parsed: 193; errors: 0
plans/INDEX.md SHA-256: 7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44
```

Using the integration-owned CLI against a disposable archive of the immutable
candidate reproduced:

```text
roko plan validate --strict tmp/status-quo/backlog/plans
0 diagnostics in 55 plans; exit 0

roko plan validate --strict tmp/status-quo/self-heal/plans
0 diagnostics in 6 plans; exit 0
```

The disposable archive was removed and the review worktree remained clean with the
sealed index. Mechanical compatibility with current integration also passes:

```text
git merge-base dd611500e ec3ecf2f
d0942fc63ef734017736294843e9112b78e8a656

git merge-tree --write-tree dd611500e ec3ecf2f
0b3c89c8e5c0395083950dc1ad413e0a8e4bdf1a
```

These positive controls prove syntax, graph schedulability, count preservation,
local write ordering, scope, and textual mergeability. They do not prove that the
named producer tasks cover the live provider dispatch paths.

## Findings

### 1. High - E48-T05 assigns LLM provider outcomes to the tool dispatcher

The corrected ownership matrix says E48-T05 is the sole provider-health routing
owner and requires every dispatch success/retriable failure to update the existing
`ProviderHealthRegistry`. But E48-T05's only outcome-side write path is
`crates/roko-agent/src/dispatcher/mod.rs`, and its E14-T10 roll-up verifies
`record_success|record_failure` in that same file.

That module is `ToolDispatcher`: its documented and implemented pipeline dispatches
`ToolCall` through schema validation, permissions, and `ToolHandler::execute`. It is
not an LLM/model/provider dispatcher, contains no `ProviderHealthRegistry`, and
cannot observe provider request success, 429/529, or timeouts. The real model-call
surfaces include `roko-agent/src/model_call_service.rs`, the provider adapters, the
legacy `dispatch_agent_with` path in `orchestrate.rs`, and the learning/runtime
feedback path. Current `orchestrate.rs` already consults provider health and current
learning code records provider outcomes; none of that makes the tool dispatcher an
appropriate canonical outcome boundary.

Reproduction:

```text
head -n 20 crates/roko-agent/src/dispatcher/mod.rs
//! Tool dispatcher ... runs a parsed ToolCall ... invokes the handler ...

rg 'ProviderHealthRegistry|record_success|record_failure' \
  crates/roko-agent/src/dispatcher/mod.rs
# no matches

rg 'ModelCallService|dispatch_agent_with|ProviderHealthRegistry' \
  crates/roko-agent/src/model_call_service.rs crates/roko-cli/src/orchestrate.rs
# live model/provider paths match
```

Consequently, E48-T05 can implement router filtering while never recording real
provider outcomes, and E14-T10's structural gate can be made green by putting dead
or unrelated text in the tool dispatcher. The claimed equivalent-or-stronger
canonical producer is not executable end to end.

**Required correction:** map E48-T05's files, context, symbols, and verification to
the actual LLM/provider outcome boundary (and both runner entry paths where
applicable), preserving dependency/file serialization. Require focused tests that
feed real success, rate-limit/server-error, timeout, Open, and HalfOpen outcomes
through that boundary and assert the same registry used by CascadeRouter is updated.
Remove the tool dispatcher from this provider-health contract unless a separate,
demonstrated provider call actually exists there.

### 2. High - E14-T08 can accept an unwired limiter and names the same stale dispatcher

E14-T08 is mapped only to E48-T02 and its executable checks inspect only
`rate_limit.rs` plus config and isolated rate-limit tests. E48-T02's declared files
define/configure the limiter; E48-T03 is the separate task that threads the shared
limiter into `model_call_service`/provider/orchestration and calls `acquire` before
each LLM request. E14-T08's context nevertheless still calls
`roko-agent/src/dispatcher/mod.rs` an agent/provider dispatch path.

Therefore a configured but completely dormant `ProviderRateLimiter` satisfies every
E14-T08 verify command. Waiting for the whole E48 plan prevents early execution but
does not make E14-T08 verify the original enforcement outcome, and the exact
`superseded_by = "E48-T02"` claim remains incomplete.

In addition, the newly strengthened `acceptance = [...]` blocks for E14-T08,
E14-T10, and E48-T05 occur after the last `[[task.verify]]`. TOML therefore places
them inside that verify table. `VerifyStep` has no acceptance field and serde ignores
the unknown key; `TaskDef.acceptance` is empty. The detailed prose does not repair
the missing executable check, and strict validation does not report this placement.

**Required correction:** represent the canonical rate outcome as E48-T02 plus the
live E48-T03 wiring (or make one exact task own both), update E14-T08's context away
from `ToolDispatcher`, and add a structural/focused test that proves the configured
shared limiter is called on the live LLM request path and gates configured RPM/TPM.
Place any retained task acceptance at task scope, and make the executable verify
commands/focused tests assert the behavior rather than relying on ignored nested
metadata.

## Verdict and next action

`REJECTED`. Do not merge
`ec3ecf2f89f0dd74a6c5e973c9ea4c7185bec30e` as accepted CTRL-08 work. Preserve the
positive graph/count/E08 producer-consumer corrections, repair the two live
provider/rate ownership contracts above on a new immutable candidate, rerun all
existing graph/status/writer/strict/scope/integration controls, and obtain a fresh
independent review.
