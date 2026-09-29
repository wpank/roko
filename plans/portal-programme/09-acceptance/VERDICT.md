# 09-acceptance: verdict

Recorded 2026-09-29 by T04, from the evidence cited below. The tree is the main checkout, branch
`chore/commit-pending-work-2026-09-25` at `f99e45dba`. The portal and roko code are unchanged since
`a17d9d766` (08g). The binary is `target/debug/roko`, built at 10:39 by 09 T01, and the portal export
is `apps/portal/out`, built at 10:37.

## Verdict

**PASS.** The test that defines done holds in a real browser:

- **Fake agent:** `PORTAL-CHECK: PASS (35 checks)`.
- **Real model, on the user's own `roko init` path:** `HELLO-WORLD-REAL: PASS (18 checks)`, twice.
  Each time a fresh folder produced a program that prints `Hello, world!` after two browser actions,
  in 55 s and 54 s.

The portal meets five of the seven budget lines (routes, navigation systems, runtime dependencies,
actions, clicks to the live transcript) and misses both size lines: 122 files and 22,684 lines
against about 65 and about 9,000.

Of the twelve contract asks, eleven landed. **P-3 (validate) is open.** It works over HTTP, but the
portal's call is rejected (400), so the badge reads `valid ✓` for every plan, invalid ones included.
The checks did not catch this; T04 found it. T04 filed 20 new work items (see Gaps filed).

## How the run went (history)

- **08g, before 09.** Every gate of plans 05–08f was green. A free dry run of this plan's browser
  flow (09:40, fake agent) then found three first-run blockers:
  - the first generated plan was not selected;
  - a revision's task did not show;
  - "agent working" and the live marker never appeared.

  Plan 08g fixed them.
- **09 under roko** (10:25:06–10:50:11, exit 1, $1.77, 3 attempts, 2 failed):
  - T01 built roko, the portal export and the demo app.
  - T02 wrote `portal-check.sh` and `browser-flow.cjs`. Its browser check failed twice on
    `live-step` (23/25). The script tested `data-live` as an attribute of the step row, while the
    portal (and its own 08c/08g tests) renders a `[data-live]` badge inside the row.
  - roko's retry, which carried a gpt-4o-mini diagnosis, did not fix it.
  - FailFast then skipped T03 and T04.

  The failed attempt's screenshot is still in the evidence:
  [fail-live-step.png](../../../tmp/portal-audit/evidence/portal-check/fail-live-step.png)
  (10:50:09), with the 10:48 and 10:50 `serve-*.log` files.
- **Finished directly** (Will's call, about 10:35: no more roko for the remaining work). The
  supervisor fixed three things:
  - the selector;
  - `check "…" [ -n "$A" ] && [ "$A" -le 2 ]`, whose `&&` escaped `check` (an empty count gave
    `[: : integer expected`);
  - the fake agent's working window, raised from 8 s to 20 s.

  The supervisor then wrote and ran T03. T04 (this file) was done by an agent in the main tree, not
  by roko. With the lead's approval, T04 re-ran T03's check once, at 11:40, to read the patched
  cost lookup.
- Sources: `tmp/dogfood/2026-09-28-portal-programme-continuation.md` (09:40 and 10:55 entries), and
  the supervisor's run logs.

## Check 1: PORTAL-CHECK (deterministic agent, free)

T04 re-ran `bash plans/portal-programme/09-acceptance/portal-check.sh` from 10:58:02 to 10:59:33
CEST: **91 s, exit 0**. The supervisor's earlier run (10:51:18–10:52:44, about 87 s) printed the same
35 lines.

```
PASS 03c: launch token present in serve.log
PASS serve.log announces tool_steps live mode
PASS / serves the portal (name="roko-portal")
PASS first /_next/static JS answers 200
PASS /demo/ serves the demo app (/demo/assets/ in HTML)
PASS GET /api/plans is 401 without credential
PASS GET /api/plans returns [] with credential
PASS browser open
PASS browser signed-in
PASS browser first-run
PASS browser generate
PASS browser run
PASS browser running
PASS browser live-step
PASS browser reload
PASS browser done
PASS browser revise
PASS browser run-again
PASS hello/hello-bin prints hello world
PASS BROWSER-ACTIONS <= 2
PASS sse: task_started for plan a-rust-app-that-prints-hello-world
PASS sse: live Write hello/main.rs before task_completed
PASS sse: agent_heartbeat for plan a-rust-app-that-prints-hello-world
PASS sse: task_completed outcome=passed
PASS sse: run_completed succeeded with positive duration_ms
PASS sse: at least 2 plan_started events (two runs: generate + run-again)
PASS server stops on Ctrl-C (flow)
PASS parallel: browser open
PASS parallel: browser run-all
PASS parallel: browser parallel-running
PASS parallel: browser parallel-done
PASS sse: second plan_started before first plan_completed
PASS out/par-a.txt exists
PASS out/par-b.txt exists
PASS server stops on Ctrl-C (parallel)
PORTAL-CHECK: PASS (35 checks)
```

- **Browser actions: 2.** `browser-flow.cjs` increments its counter once in step 4 (generate) and once
  in step 5 (run), and both passed; the check asserts `<= 2`. The script's own
  `BROWSER-ACTIONS` line stays in the scratch workspace and is not copied to the evidence.
- **Run times** (from the capture's `run_completed`): 21.8 s for the first run, 41.6 s for Run again
  (T99 and T01, each held 20 s by the fake agent), and 8.6 s for the two-plan parallel set.
- **Cost:** $0 real spend. The fake agent reports a fixture $0.0012 per task, which puts $0.006 on
  the stream for 5 tasks.
- **Screenshots:**
  [1-generated](../../../tmp/portal-audit/evidence/portal-check/1-generated.png) ·
  [2-running](../../../tmp/portal-audit/evidence/portal-check/2-running.png) ·
  [3-done](../../../tmp/portal-audit/evidence/portal-check/3-done.png) ·
  [4-revised](../../../tmp/portal-audit/evidence/portal-check/4-revised.png) ·
  [5-parallel](../../../tmp/portal-audit/evidence/portal-check/5-parallel.png).
  Captures: `events.sse`, `par-events.sse`, `browser-flow.json`, `browser-parallel.json` and the
  `serve-*.log` files, all in `tmp/portal-audit/evidence/portal-check/`. `tmp/` is gitignored, so
  these links resolve only in the main checkout.
- **Browser console:**
  - The flow logged 2 × 404 and 6 × 400; the parallel flow logged 1 × 404 and 1 × 400.
  - Every 400 is the portal's validate call (bug-48494b). A T04 probe that logged each response
    ≥ 400 saw only `POST /api/plans/<id>/validate` → 400, at generate, run, revise and run-again.
  - The 404s fit the generation poll (bug-64fb48). The one 404 in the parallel flow, where no plan is
    generated, was not reproduced.

## Check 2: HELLO-WORLD-REAL (real model, the user's path)

The check ran twice, and passed both times. The supervisor ran it first. T04 ran it once more, with
the lead's approval, to read the patched cost lookup. **This section quotes the re-run.**

**The re-run** (`bash plans/portal-programme/09-acceptance/hello-world-real.sh`) ran from 11:40:17
to 11:41:11 CEST: **54 s, exit 0**. It used the same binary and portal export, and a fresh `git init`
and `roko init --profile rust` folder (`/private/tmp/roko-hello-emvj2b`, removed on pass). There
were no fixtures, no `ROKO_SPA_DIR`, and auth was left as `roko init` sets it.

```
PASS serve.log prints the portal link with a launch token
PASS / serves the portal with no extra configuration
PASS an API request without a credential answers 401
PASS browser open
PASS browser signed-in
PASS browser first-run
PASS browser generate
PASS browser run
PASS browser running
PASS browser reload
PASS browser done
PASS the browser reported the generated plan's slug
PASS BROWSER-ACTIONS <= 2
PASS sse: task_started for a-rust-app-that-prints-hello-world
PASS sse: agent_output for a-rust-app-that-prints-hello-world
PASS sse: task_completed outcome=passed for a-rust-app-that-prints-hello-world
PASS sse: run_completed outcome=succeeded
PASS the program the plan built prints hello world
HELLO-WORLD-REAL cost_usd_total=0.061694398522377014
HELLO-WORLD-REAL: PASS (18 checks)
```

- **Browser actions: 2** (`BROWSER-ACTIONS 2` in `browser-real.out`).
- **Model:** claude-sonnet-4-6 via `claude_cli`, for both generation and the task.
- **Wall time:** 54 s end to end.
  - Generation took 20.7 s (`plan_generate.started` at 1790674822582 ms → `.completed` at
    1790674843290 ms; `serve.log`: `agent=20528ms`). It produced a one-task plan, 1,113 bytes, in
    `.roko/plans/a-rust-app-that-prints-hello-world/` (the folder has no `plans/`).
  - The run took 18.7 s (`run_completed duration_ms 18701`).
  - `program.out` holds `Hello, world!`.
- **Cost:** the patched lookup read **`cost_usd_total` = $0.0617** from `GET /api/statehub/snapshot`.
  - That is exactly the task's one `efficiency_event cost_usd` (0.061694398522377014), for 25 input,
    820 output, 13,578 cache-read and 7,541 cache-write tokens.
  - So the snapshot total leaves out the 20.5 s generation turn, which roko reports nowhere
    (gap-a6e2c3).
  - Real spend of this run: $0.0617 plus an unmeasured generation turn.
- **Screenshots:**
  [1-generated](../../../tmp/portal-audit/evidence/hello-world-real/1-generated.png) ·
  [2-running](../../../tmp/portal-audit/evidence/hello-world-real/2-running.png) ·
  [3-done](../../../tmp/portal-audit/evidence/hello-world-real/3-done.png).
  The rest of the evidence is in `tmp/portal-audit/evidence/hello-world-real/`: `events.sse`,
  `serve.log`, `browser-real.json`, `browser-real.out`, `tasks.toml`, `program/` and `summary.txt`.
- **Browser console:** 22 × 404, one per second of generation (bug-64fb48), and 3 × 400 (validate,
  bug-48494b).

**The first run** (supervisor) ran from 10:54:06 to 10:55:01 CEST: 55 s, `HWR-EXIT rc=0`, in
`/private/tmp/roko-hello-JdC7dN`.

- It printed the same 18 PASS lines and `HELLO-WORLD-REAL: PASS (18 checks)`, with
  `BROWSER-ACTIONS 2`.
- Generation took 29.4 s (`agent=29341ms`) and the run 16.6 s (`run_completed duration_ms 16611`).
  `cargo run` printed `Hello, world!`.
- The task cost $0.0587: 33 input, 1,018 output, 17,315 cache-read and 6,362 cache-write tokens.
- Its `cost_usd_total=unknown` came from the pre-patch lookup, which read `stats` at the top level of
  a response that wraps the dashboard in a state frame.
- Browser console: 30 × 404 and 3 × 400.
- T04 copied its evidence, with the run's log, to `tmp/portal-audit/evidence/hello-world-real-run1/`
  before the re-run overwrote `hello-world-real/`:
  [1-generated](../../../tmp/portal-audit/evidence/hello-world-real-run1/1-generated.png) ·
  [2-running](../../../tmp/portal-audit/evidence/hello-world-real-run1/2-running.png) ·
  [3-done](../../../tmp/portal-audit/evidence/hello-world-real-run1/3-done.png).

**Seen in both runs:**

- The screened transcript runs messages together. First run: "…of the repository.No `Cargo.toml`
  exists yet…"; re-run: "…the workspace state.No `Cargo.toml` exists yet…" (bug-2116ec).
- Live tool-step targets are absolute paths (gap-fa61f8).
- `plan_completed` is published twice (bug-08d912).
- The first plan was written to the legacy `.roko/plans/` (q-4299a9).

## Programme cost and time

Per-plan cost of the roko-run plans, read from each worktree's `.roko` data (all agent attempts
claude-sonnet-4-6):

| plan | status | tasks done | cost $ | attempts | failed | agent-min |
|---|---|---:|---:|---:|---:|---:|
| 01-backend-plan-service | succeeded | 10 | 3.85 | 27 | 4 | 55 |
| 02-backend-plan-execution | failed | 7 | 9.59 | 15 | 8 | 35 |
| 03-backend-live-events | succeeded | 7 | 6.40 | 7 | 0 | 63 |
| 03b-backend-workspace-server | succeeded | 18 | 29.31 | 22 | 4 | 374 |
| 03c-backend-local-access | succeeded | 20 | 37.82 | 21 | 1 | 434 |
| 04-backend-plan-authoring | succeeded | 15 | 20.20 | 15 | 0 | 212 |
| 04b-backend-plan-revision | succeeded | 6 | 4.70 | 6 | 0 | 76 |
| 05-portal-foundation | succeeded | 14 | 9.15 | 17 | 3 | 37 |
| 06-portal-shell | succeeded | 9 | 4.97 | 9 | 0 | 22 |
| 07-portal-compose | succeeded | 10 | 7.44 | 10 | 0 | 47 |
| 08-portal-run | succeeded | 10 | 6.32 | 10 | 0 | 30 |
| 08b-portal-polish | succeeded | 16 | 11.34 | 22 | 6 | 49 |
| 08c-portal-live-steps | succeeded | 5 | 3.23 | 5 | 0 | 16 |
| 08d-portal-legibility | succeeded | 11 | 8.25 | 11 | 0 | 34 |
| 08e-portal-refine | succeeded | 7 | 3.73 | 8 | 1 | 26 |
| 08f-final-polish | succeeded | 6 | 3.73 | 9 | 3 | 47 |
| 08g-first-run | succeeded | 3 | 1.00 | 3 | 0 | 4 |
| 09-acceptance | failed | 1 | 1.77 | 3 | 2 | 22 |
| **total** | | | **172.79** | **220** | **32** | **1,583** |

The two real-model checks spent $0.0587 and $0.0617 on their tasks, plus two unmeasured generation
turns. Not included: the supervisor's and helper agents' direct work (fixes, reviews, this verdict), which
roko does not meter.

## Portal against the budget (`tmp/portal-audit/02-DESIGN.md` §14)

| | Before (2026-09-28) | Target | Measured 2026-09-29 | |
|---|---|---|---|---|
| Routes | 34 | 1 | **1**: `src/app/page.tsx` (`find apps/portal/src/app -name page.tsx`) | met |
| Navigation systems | 3 | 0 | **0**: one route, no links or router. The rail's `<nav>` is the plan list; selection is `?plan=` state | met |
| Files in `src/` | 111, no tests | ~65, 17 of them unit-test files | **122**: 61 test files (`*.test.ts(x)`), 61 others | **missed** (1.9×) |
| LOC in `src/` | 33,286 | ~9,000, about a quarter tests | **22,684**: 9,688 in tests, 12,996 in the rest (`cat \| wc -l`) | **missed** (2.5×; non-test code 1.9× the ~6,750 implied) |
| Runtime dependencies | 11 | 6 | **6**: `@tanstack/react-query`, `clsx`, `next`, `react`, `react-dom`, `zustand` (14 dev dependencies) | met |
| Actions to build hello world | 5+ (fails) | 2 in an empty folder | **2** in both checks | met |
| Clicks to the live transcript | 3 (in a drawer) | 0 | **0**: step `running` requires `[data-region="transcript"]` with no click | met |

Tests: `npm test` in `apps/portal` passed **61 test files and 676 tests** (vitest 5.0.2, 9.08 s, run
at 10:58:23). The size miss is filed as find-243752.

## Contract asks P-1 to P-12 (`tmp/portal-audit/03-CONTRACT.md` §6, §6.1)

**11 landed, 1 open (P-3).**

| Ask | Status | Evidence | Residual items |
|---|---|---|---|
| P-1 launch without a key | **landed** | ACCESS-CHECK PASS (24) (03c). PORTAL-CHECK: "03c: launch token present", "GET /api/plans is 401 without credential", "returns [] with credential", "browser signed-in", "browser reload". HELLO-WORLD-REAL on the `roko init` path: "serve.log prints the portal link with a launch token", "an API request without a credential answers 401", "browser signed-in", "browser reload" | find-c8527b, gap-eb4a65, **bug-f47afb** (new) |
| P-2 generation reports plan and failure | **landed** | AUTHORING-CHECK PASS (33) (04). "browser generate" in both checks. Real capture: `plan_generate.started`/`.completed` with `plan_id`. A failed generation is unit-tested only, not run in a browser: `operation.test.ts` covers a failed operation, `noticeClose.accept.test.tsx` a refused request | bug-a0f01e, **bug-64fb48**, **gap-a6e2c3** (new) |
| P-3 validate, optionally unsaved text | **open** | Server side: AUTHORING-CHECK (no body, `{toml}`). End to end it fails. The portal posts `{}` and gets `400 invalid_json` (every `browser-*.json`). The server returns `errors`/`warnings` as counts, not the contract's arrays. The badge shows `valid ✓` on failure: a T04 probe opened a plan with an unknown dependency and saw `VALID ✓` with Run enabled | **bug-48494b** (new, p2) |
| P-4 edit as text | **landed** (server) | AUTHORING-CHECK: GET/PUT source, byte-identical after a rejected save, 409 while running. No browser check opens the editor | **gap-cb9274**, **bug-0522e8** (new) |
| P-5 revise in place | **landed** | REVISION-CHECK PASS (15) (04b). PORTAL-CHECK "browser revise" (T99 appears) and "browser run-again" (T99 done). Capture: `plan_revise.started`/`.completed` | gap-3bea93, gap-b3e513 |
| P-6 run several plans | **landed** | WORKSPACE-SERVER-CHECK PASS (58) (03b). PORTAL-CHECK "parallel: browser run-all", "parallel-running" (`2 running`, agents of two plans), "parallel-done", "sse: second plan_started before first plan_completed", `out/par-a.txt`, `out/par-b.txt` | find-8872ad, **bug-979636** (new) |
| P-7 retry / run again | **landed** | Run again: PORTAL-CHECK "browser run-again", "at least 2 plan_started events". Retry (`{resume: true}`): WORKSPACE-SERVER-CHECK ("a resumed run of a plan the CLI completed succeeds"); not exercised in a browser | gap-b07969 |
| P-8 server runs publish to the hub | **landed** | LIVE-EVENTS-CHECK PASS (12) (03). PORTAL-CHECK sse lines (task_started, live Write before task_completed, agent_heartbeat, task_completed passed, run_completed with duration). HELLO-WORLD-REAL sse lines | gap-8a1fb3, find-0d280d, **bug-08d912**, **bug-2116ec**, **gap-fa61f8** (new) |
| P-9 workspace identity | **landed** | T04 probe: `GET /api/status` → `{"workdir": …, "git_branch": "main"}`. The header reads `<workspace> · main` in the screenshots (for example 1-generated); "browser signed-in" checks the workspace name | — |
| P-10 one-command launch | **landed** | PORTAL-CHECK "/ serves the portal", "first /_next/static JS answers 200", "/demo/ serves the demo app". HELLO-WORLD-REAL "/ serves the portal with no extra configuration" (`ROKO_SPA_DIR` unset) | gap-2122bd |
| P-11 task detail and estimates | **landed** | AUTHORING-CHECK / contract AS BUILT 04 §B. Not added, by decision: `path` on `PlanSummaryDto`. In the browser, the status line shows `parallel 1`, from `max_parallel` (1-generated) | — |
| P-12 byte-slicing panic | **landed** (code) | `crates/roko-cli/src/graph_task_dispatch.rs` floors the tail's start to a char boundary at both truncation sites (:2854–2862, :2957–2963) and keeps the `[...truncated]\n` prefix. No test covers a multi-byte boundary there; not filed | — |

## Design elements (`02-DESIGN.md` §2–§12)

Three read-only audits checked 93 design elements against `apps/portal/src`: 72 built (a few with
caveats that are filed below, such as the group ▶ and the disabled-button tooltips), 15 partial,
3 deviating and 3 not built. T04 re-checked every partial or unbuilt element in the source,
and three live (validation, restart alert, console errors), before filing it:

| Element | Finding | Item |
|---|---|---|
| §4.2 `valid ✓`, §4.1 Run disabled on errors, §8 validation alert | the badge is false; Run is never disabled for validation; the alert is hard-wired off | bug-48494b |
| §3, §9, §12 (the filter is a view) | the filter changes what a group ▶ runs, Run all's confirm count, the header's run summary, and the selection | bug-979636 |
| §2, §12 header cost | shows the server's lifetime spend | bug-b69a47 |
| §4.1, §4a unsaved text | the `r` key runs past it; the Edit and Revise buttons discard it | bug-0522e8 |
| §0, §8 sign-in after a restart | "Lost the server; reconnecting." (seen live for 60 s) | bug-f47afb |
| §3, §6 rule 1 after a reload | times, cost and accepted counts are lost; the snapshot lacks them | gap-bfd447 |
| §6 rule 1 (amber) | the header is never amber; an all-dispatched running plan is not amber | gap-63e0b6 |
| §7 empty states | the all-dispatched, finished and stopped-at sentences never render; Ready lacks waves | gap-6ff814 |
| §9 keys and selection | Esc does not close Revise; an unknown `?task=` is kept; a later-starting plan is auto-selected | gap-76bd0b |
| §5, §11 stream | passed-step output and unreached steps are not shown; the failed row lacks the digest; the truncation note is hidden; an empty transcript is blank | gap-bd33b2 |
| §4a editor diagnostics | no rule id; task ids are not links | gap-cb9274 |
| disabled-button reasons | tooltips can never show (`pointer-events: none`) | bug-5e71d0 |
| §14 budget | the file and line targets are missed | find-243752 |

Deviations that later plans chose, not filed:

- the rail's unknown time is blank instead of `·` (08d, "rows without placeholder dots");
- role·model shows before dispatch (asserted by `TaskList.accept.test.tsx`);
- colour tests assert looser thresholds (≥35°, ΔE ≥30) than the design's text (42°, 56).

Skipped tasks drawn as done is the existing bug-7e1b6b.

## Other findings from the evidence

- **The checks do not look at the console, and never open an invalid plan.** That is how the validate
  400s passed every preview and both checks. The CI gap (gap-e5bbd6) proposes failing on console
  errors.
- **No CI runs the portal.** No workflow runs its unit suite, type check, export or this plan's
  browser check (gap-e5bbd6). Every gate that caught a portal defect in this programme ran inside a
  roko plan or a manual dry run.
- **Generation lands in `.roko/plans/`** in a workspace without `plans/`, a location the code calls
  legacy. A decision is needed (q-4299a9).
- **09 T02's failure** was a spec ambiguity ("carrying `[data-live]`") in a check script, not a
  product defect. It is recorded above, not filed.

## Gaps filed

New items (`discovered_from = "plan:portal-programme/09-acceptance#T04"`), in `work/items/`:

| id | kind | sev | title |
|---|---|---|---|
| bug-48494b | bug | p2 | Portal validation badge shows valid for invalid plans: the validate call gets 400 and a failed request renders as valid |
| bug-979636 | bug | p2 | The rail filter changes what runs and what is selected: group Run starts only matching plans, and a hidden selection is cleared |
| gap-e5bbd6 | gap | p2 | No CI job runs the portal: its unit tests, type check, static export and the fake-agent browser check run only inside roko plans |
| bug-b69a47 | bug | p3 | The portal header shows the server's lifetime cost instead of the running run's cost |
| bug-0522e8 | bug | p3 | Unsaved plan edits are unprotected: the r key runs the saved plan, and the Edit or Revise buttons discard the text without asking |
| bug-f47afb | bug | p3 | After roko serve restarts, the portal says 'Lost the server; reconnecting.' instead of asking for the new sign-in link |
| bug-08d912 | bug | p3 | A single-plan server run publishes plan_completed twice, the second after run_completed |
| gap-a6e2c3 | gap | p3 | Plan generation and revision spend never reaches the event stream, so the portal and stats.cost_usd_total leave it out |
| bug-64fb48 | bug | p3 | While a plan generates, the portal requests the unwritten plan every second, logging one 404 per second |
| gap-bfd447 | gap | p3 | After a reload the portal loses each plan's times, cost and accepted count because the snapshot's plan state lacks them |
| gap-63e0b6 | gap | p3 | Design rule 'green means verified' is partly built: the header is never amber, nor is a running plan whose tasks are all dispatched |
| gap-6ff814 | gap | p3 | The portal never shows the design's run summaries: the all-dispatched, finished and stopped-at sentences are unreachable |
| gap-76bd0b | gap | p3 | Portal keys and selection deviate from design section 9: Esc leaves Revise open, an unknown task id is kept, a later-starting plan is auto-selected |
| gap-bd33b2 | gap | p3 | Portal stream deviates from design sections 5 and 11: passed checks hide output, unreached steps are unlisted, the truncation note is hidden |
| gap-cb9274 | gap | p3 | Plan editor diagnostics show no rule id and no link to the task (design section 4a) |
| bug-5e71d0 | bug | p3 | Disabled portal buttons explain themselves only in tooltips that can never show (pointer-events: none) |
| find-243752 | finding | p3 | The portal is over its design budget: 22,684 lines in 122 files against about 9,000 in about 65 |
| gap-fa61f8 | gap | p3 | Live tool-step targets are absolute paths for real providers, so every step carries the full workspace path |
| bug-2116ec | bug | p3 | The screened transcript joins separate assistant messages with no separator |
| q-4299a9 | question | p3 | Should a plan generated in a workspace without plans/ be written to plans/ rather than the legacy .roko/plans/? |

Each has anchors. Nineteen have a `[[verify]]` that fails today: the harness ones were run against
this binary, and the others' static guards fail; `python3 tools/work.py verify … --static-only`
reports FAIL for each. The question has none.

Existing open items cited, not duplicated: find-c8527b, gap-eb4a65, bug-a0f01e, gap-3bea93,
gap-b3e513, find-8872ad, gap-b07969, gap-8a1fb3, find-0d280d, gap-2122bd, bug-7e1b6b, gap-4171e8,
gap-655d19, gap-082a14, bug-9f340c. Also cited: the parked find-6a5b62 and spec-87be33.

## Not determined

- The fake flow's exact `BROWSER-ACTIONS` line is not kept in the evidence. It is derived as 2 from
  the script.
- The real runs' generation cost is unknown. The re-run's snapshot total equals its task cost alone
  (gap-a6e2c3).
- The one 404 in the parallel flow is unexplained.
- Where the screened text is joined (bug-2116ec) was narrowed to the Claude CLI deltas and the path
  to `agent_output`, but not pinned down.
