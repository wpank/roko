# Demo App Workflow Audit

Date: 2026-04-28

Scope:

- Demo app source: `demo/demo-app`
- Existing local server: `http://127.0.0.1:6677`
- Vite app used for browser audit: `http://127.0.0.1:5173`
- User-provided demo workspace: `/tmp/roko-demo-1777396797076`
- Fresh browser/CLI artifacts:
  - `/tmp/roko-demo-ui-audit/route-results.json`
  - `/tmp/roko-demo-ui-audit/workflow-results.json`
  - `/tmp/roko-demo-ui-audit/demo-retry-results.json`
  - `/tmp/roko-demo-command-audit-1777400398/summary.tsv`

No source implementation was changed in `demo/demo-app` during this pass. This document is an
issue catalog plus a repair plan.

## Executive Verdict

The demo app is not yet a reliable proof surface for "Roko works like Mori." It is currently a
hybrid of:

- real PTY terminals that run commands,
- frontend-only fallback data,
- stale command scripts copied from older CLI assumptions,
- backend endpoints whose contracts have drifted,
- UI status detection based on terminal scraping,
- and "investor demo mode" health/fallback behavior that masks failures.

The best solution is not to patch individual labels or add more fake data. The shortest credible
path is to turn the demo app into a thin client over explicit workflow contracts:

1. Fix the underlying CLI execution contract first: model/provider selection, nonzero failures,
   real gates, real telemetry, consistent workdir/resume state.
2. Replace implicit terminal scraping for product workflows with typed server workflow APIs and
   structured events.
3. Keep the terminal UI as a manual shell and diagnostic view, not as the primary automation
   engine for demos.
4. Gate every demo scenario with deterministic acceptance checks and artifact links.
5. Make fallback data an explicit "mock mode," never a silent substitute for a live broken API.

Until those contracts exist, the UI can look active while the actual workflow is wrong, empty, or
fake.

## Evidence Collected

### Build

`npm run build` in `demo/demo-app` succeeded. It emitted a large bundle warning: the main JS chunk
is about 1.17 MB after minification, above Vite's 500 KB warning threshold.

This is not the biggest blocker, but it confirms the demo is becoming a large single bundle with
no route-level code splitting.

### Browser Route Smoke

The route pass covered:

- `/`
- `/dashboard`
- `/dashboard/fleet`
- `/dashboard/knowledge`
- `/dashboard/chain`
- `/dashboard/entries`
- `/dashboard/routing`
- `/dashboard/share/test-token`
- `/demo`
- `/terminal`
- `/builder`
- `/explorer`
- `/bench`
- `/bench/run/bench-run-1`
- `/bench/compare`
- `/bench/showroom`
- `/share/test-token`

Raw result file: `/tmp/roko-demo-ui-audit/route-results.json`.

### Browser Workflow Pass

The workflow pass exercised:

- terminal echo
- builder preset run
- bench run and cancel
- bench showroom playback
- demo self-hosting tab
- demo builder tab
- demo race tab
- demo providers tab
- demo explore tab
- demo chat tab
- demo mirage tab

Raw result file: `/tmp/roko-demo-ui-audit/workflow-results.json`.

Additional long-wait retry for providers/explore still hit the same undefined terminal handle
error. Raw result file: `/tmp/roko-demo-ui-audit/demo-retry-results.json`.

### Direct CLI Scenario Pass

The command audit executed the commands behind the demo scenarios in fresh temp workspaces. The
browser terminal resolves `roko` from PATH, so the command audit used the same:

- `/Users/will/.local/bin/roko`
- `roko 0.1.0`

Raw summary file: `/tmp/roko-demo-command-audit-1777400398/summary.tsv`.

## Intended Demo Surface Map

The routes are defined in `demo/demo-app/src/main.tsx`:

- `/`: landing page.
- `/dashboard`: cost/status overview.
- `/dashboard/fleet`: active agent fleet.
- `/dashboard/knowledge`: knowledge graph.
- `/dashboard/chain`: chain/custody placeholder.
- `/dashboard/entries`: knowledge entries table.
- `/dashboard/routing`: cascade router view.
- `/dashboard/share/:token`: shared run transcript.
- `/demo`: scripted multi-pane demo scenarios.
- `/terminal`: manual PTY terminal.
- `/builder`: prompt-to-build workflow.
- `/explorer`: health/status/episodes/events explorer.
- `/bench`: benchmark lab.
- `/bench/run/:id`: benchmark run detail.
- `/bench/compare`: benchmark comparison.
- `/bench/showroom`: animated benchmark showroom.
- `/share/:token`: receipt-style share page.

The scripted scenarios are in `demo/demo-app/src/lib/scenarios.ts`:

- Self-Hosting: `roko prd idea`, `roko prd draft new`, `roko prd plan`, `roko status`,
  `roko learn all`.
- Build: setup a temp workspace only; comment says the actual build is triggered externally.
- Cost Race: two panes running `roko run "...calculator..." --no-replan` and `roko run`.
- Providers: four panes running `roko run "...server" --provider <name>`.
- Explore: status, doctor, PRD, learning, config, knowledge, explain commands.
- Chat: runs bare `roko`, sends text and slash commands.
- Mirage: clears a terminal only.

## Workflow Results

### Landing

Status: renders, but the numbers are partly theatrical.

Observed:

- Shows active agents, episodes, gate pass, cost, C-factor, and product copy.
- Browser console saw a 404 resource load, likely favicon or static asset.

Issue:

- The landing page can look healthy even when backend endpoint details are broken, because other
  app hooks silently fall back to demo data.

Fix:

- Treat the landing page as a read-only product intro or connect it to one typed `/api/demo/state`
  payload. Do not mix endpoint-by-endpoint fallback data with live health.

### Dashboard Overview

Status: renders, but the values are internally inconsistent.

Observed from route smoke:

- `STATUS Online`
- `0/0 providers`
- `EPISODES 2`
- `0 gates passed`
- `GATE PASS RATE NaN%`
- `TOTAL COST $1.42`
- `ACTIVE AGENTS 3`

Direct server health showed providers total 0, active agents 3, episodes total 2, gates passed 0,
gates failed 0, and no real cost total. The dashboard is mixing real server state with fallback
demo metrics.

Fix:

- Define one dashboard DTO that either comes completely from live projections or completely from
  mock mode.
- Guard all division by zero.
- Surface "no gate data yet" instead of `NaN%`.
- Stop showing hardcoded cost as live cost.

### Fleet

Status: renders, but it mixes live and fallback data.

Observed:

- Three active agents: `RELAY-DEMO`, `ENV-OVERRIDE`, `DEMO-1`.
- Each has model `GLM-5.1`, reputation 0, tasks 0, cost 0.
- Top stat says `TASKS DONE 827`.

Issue:

- The top stat is fallback/demo data while the table appears to be live.

Fix:

- Compute fleet summary from the same list rendered in the table.
- If fallback mode is desired, show an explicit "demo data" badge.

### Knowledge Graph

Status: renders, but contradicts itself.

Observed:

- Top stats: `NODES 18`, `EDGES 28`, `DOMAINS 5`.
- Graph body: `0 NODES / 0 EDGES`.
- Direct `/api/knowledge/entries` and `/api/knowledge/edges` returned empty lists.

Issue:

- Header metrics use fallback data while graph data uses live empty data.

Fix:

- Fetch graph state through one endpoint or normalize both entries and stats from the same source.
- Show empty state when live data is empty, not fallback header numbers.

### Chain

Status: intentionally placeholder, but it reads as more real than it is.

Observed:

- Shows `STATUS Phase 2`.
- Shows a hardcoded witness hash and features.

Fix:

- Label it as a roadmap/placeholder, or hide it from "all workflows work" demos until there is a
  real chain anchoring path.

### Entries

Status: honest empty state.

Observed:

- `0 TOTAL`
- `No knowledge entries found`

Fix:

- This is acceptable if paired with the Knowledge Graph fix so the app does not simultaneously
  claim 18 nodes.

### Routing

Status: renders, but the data is thin and possibly partly static.

Observed:

- `1 MODELS`, `9 OBSERVATIONS`, `33% AVG CONFIDENCE`.
- Role-to-model table is large and static-looking.

Fix:

- Separate live router observations from configured/default role routing.
- Add a "last updated/source" indicator.
- Make `total observations` and role table derive from a typed projection.

### Terminal

Status: basic PTY works.

Observed:

- Browser workflow typed `echo ROKO_TERMINAL_OK`; output appeared.
- The terminal started in `.../roko/.roko`, not the repository root.

Source cause:

- PTY sessions default to `AppState.terminal_sessions.workdir` in `crates/roko-serve/src/terminal.rs`.
- WebSocket terminal creation calls `create_session(80, 24, None, None)`, so the frontend cannot
  pass a desired workdir.

Why this matters:

- The CLI has auto-correction for running inside `.roko`, so commands may silently run against the
  parent repo instead of the temp demo workspace. The builder workflow hit this exact failure mode.

Fix:

- Add a terminal session creation contract that accepts a workdir.
- For WebSocket auto-create, allow `?workdir=` or create via REST first and bind WS to that
  session.
- Default the app terminal to the project root, not `.roko`.
- Display the effective workdir in the UI and fail workflow setup if it differs from the intended
  demo workspace.

### Builder Page

Status: UI renders, command runs, but the workflow is not trustworthy.

Observed browser workflow:

- The page displayed model `Claude Opus 4`.
- Clicking the calculator preset ran `roko run "Build a CLI calculator in Rust"`.
- The run started from `.../roko/.roko`, then CLI auto-corrected to the Roko repo root.
- Actual provider/model selected by the CLI was `glm-5.1` via OpenAI-compatible ZAI, not the UI
  selected Claude Opus.
- It failed with HTTP 429 from ZAI.
- The UI still showed `BUILDING...`, pending gates, and 0 files after the terminal prompt returned.

Source causes:

- `Builder.tsx` stores `selectedModel`, but `submitTask()` builds only
  `${getRoko()} run "${text.trim()}"`. It never passes the selected model.
- Workspace setup is asynchronous and not enforced before the preset buttons can run.
- `showCmd()` determines completion by prompt scraping and does not expose command exit status or
  structured run result.
- File detection is regex-based over terminal text.

Fix:

- Disable build inputs until workspace setup completes and the effective cwd is shown.
- Pass model/provider through a real CLI contract once the CLI honors it.
- Better: call a typed backend workflow endpoint such as `POST /api/workflows/build`, then stream
  structured events.
- Make terminal output a transcript, not the source of truth for status.
- Show failed state when the CLI reports `workflow halted`, even if the process exits 0.
- Display artifact paths and changed files from a manifest, not from regex matches.

### Demo: Self-Hosting

Status: partially starts, then direct command evidence shows later steps fail.

Observed browser workflow:

- Created `/tmp/roko-demo-1777400519670`.
- Ran `roko prd idea "Wire SystemPromptBuilder into orchestrate.rs"`.
- Still showed Step 1/5 after 15 seconds.

Direct command audit:

- `roko prd idea` succeeded.
- `roko prd draft new system-prompt-wiring` failed in about 2 seconds:
  - Claude CLI Opus process returned 0 bytes.
  - Exit 1.
  - Scaffold preserved.
- `roko prd plan system-prompt-wiring` failed in about 1 second:
  - Claude CLI Opus process returned 0 bytes.
  - Exit 2.
- `roko status` showed a failed plan signal and cost as `$-0.0000`.
- `roko learn all` showed 2 failed episodes, 0% success, and $0.00 total.

Fix:

- Do not present self-hosting as "Roko develops itself" from a blank temp workspace.
- Decide the scenario mode:
  - real Roko self-hosting: run against the Roko repo and require repo-grounded PRD/plan
    validation;
  - blank project PRD demo: label it as a new-project PRD flow.
- Surface PRD/plan agent failures as failed steps with links to logs/artifacts.
- Fix Claude CLI failure reporting so the 0-byte/exit-1 root cause is visible.
- Fix negative-zero cost and unknown usage recording.

### Demo: Build Tab

Status: broken as a demo scenario.

Observed:

- The scenario has `promptBar: true`, but `Demo.tsx` does not render a prompt bar for scenario
  tabs.
- Running the tab hit `TypeError: Cannot read properties of undefined (reading 'ws')`.

Source cause:

- `scenarios.ts` builder run only calls `setupWorkspace()` and says actual build is triggered
  externally.
- `Demo.tsx` does not implement the scenario prompt bar behavior.
- `buildContext()` filters `h !== null`, which allows `undefined` through.

Fix:

- Either remove this tab and route users to `/builder`, or implement the prompt bar and acceptance
  states.
- Fix terminal handle readiness before any scenario can run.

### Demo: Cost Race

Status: UI tab fails; direct CLI commands do not demonstrate a race.

Observed:

- Browser tab hit undefined terminal handle error.
- Direct command audit ran both commands:
  - `roko run "Build a CLI calculator in Rust" --no-replan`
  - `roko run "Build a CLI calculator in Rust"`
- Both failed immediately due missing `ANTHROPIC_API_KEY`.
- Both exited 0 despite `workflow halted`.
- The `--no-replan` difference never mattered because both died before useful execution.

Fix:

- Fix CLI exit code semantics first: halted workflow must be nonzero or structured failed.
- Make model/provider preflight happen before the scenario starts.
- Use deterministic, configured models for both arms.
- Emit structured run summaries with cost/tokens/gates so the UI can compare real outcomes.

### Demo: Providers

Status: hard-broken against current CLI.

Observed:

- Browser tab hit undefined terminal handle error even after an 8-second wait.
- Direct command audit showed every provider command exits 2:
  - `error: unexpected argument '--provider' found`
  - CLI hint: `--role`

Source cause:

- The scenario still calls `roko run "...server" --provider zhipu/openai/anthropic/moonshot`.
- Current CLI does not expose `--provider` for `roko run`.

Fix:

- Decide the real CLI contract:
  - implement `--provider` as a hard override, or
  - update demo commands to the supported `--model`/provider config workflow.
- Add `roko config providers test` or equivalent preflight for each provider before dispatch.
- Do not run four live provider jobs unless keys and rate limits are known good.
- Record unconfigured providers as skipped, not failed or fake-success.

### Demo: Explore

Status: UI tab fails; direct commands mostly run but are empty or misleading.

Observed direct command audit:

- `status`: empty workspace, 0 signals.
- `doctor`: ok, serve health skipped.
- `prd list`: none.
- `learn all`: no useful history.
- `learn tune gates`: no gate thresholds found.
- `config providers list`: says Claude CLI command missing, while PRD path used Claude CLI.
- `config validate`: emits schema v1 warning, then says `Result: 0 warnings, 0 errors`.
- `knowledge stats`: 0 entries.
- `knowledge query "routing"`: no matches.
- `roko explain "cascade routing"`: prints `unknown topic: cascade routing`, exits 0.

Fix:

- Seed the scenario with a known workspace that has episodes, plans, gates, knowledge, and config.
- Fix `config validate` so schema migration warnings count as warnings or are clearly separated.
- Make `roko explain` return nonzero for unknown topics, or provide a suggestion for `routing`.
- Do not claim "18 crates, 85 routes, 100+ commands" in a blank temp workspace unless the commands
  actually inspect the Roko repo.

### Demo: Chat

Status: UI tab fails before chat starts; underlying workflow needs a real chat session contract.

Observed:

- Browser tab hit undefined terminal handle error.
- Scenario then would run bare `roko`, wait for a chat-like prompt, send natural language, and send
  slash commands.

Risk:

- If bare `roko` does not enter the intended chat mode, the scenario may just type natural
  language into a shell or a stuck TUI.
- Prompt detection is regex-based and not a reliable chat protocol.

Fix:

- Implement the `ChatAgentSession` repair described in `FINAL-SOLUTION.md`.
- Add a typed browser chat endpoint or structured PTY protocol for chat events.
- Ensure `/status`, `/model`, `/system`, `/effort`, and `/reset` mutate real session state.
- Add a deterministic acceptance test: follow-up turn uses resume/session continuity.

### Demo: Mirage

Status: only clears a terminal; no real mirage workflow is wired.

Observed:

- Browser tab hit undefined terminal handle error.
- Scenario has `mirageBar: true`, but `Demo.tsx` does not render mirage controls.
- `run()` waits for a WS and clears the terminal. It does not fork a chain or stream blocks.

Fix:

- Hide the tab until an actual mirage backend exists, or implement:
  - RPC URL input,
  - chain/fork config,
  - start/stop controls,
  - block stream,
  - transaction list,
  - error state.
- Do not include it in "all demos work" acceptance until it has a real command/API path.

### Bench Lab

Status: visually rich, but the main live run path is fake due API contract drift.

Observed route:

- Shows fallback suites, fallback models, fallback history.
- `Run Benchmark` posts to `/api/bench/runs`.
- Server returns 405 because backend route is singular `POST /api/bench/run`.
- UI still creates a fake active run id `demo-...`, opens SSE, and shows `RUNNING`.
- Cancel posts `/api/bench/runs/:id/cancel`, which does not exist, then marks UI run cancelled.

Source causes:

- Frontend `useBench.ts` expects arrays:
  - `GET /api/bench/suites` as `BenchSuite[]`
  - `GET /api/bench/models` as `BenchModel[]`
  - `GET /api/bench/runs` as `BenchRun[]`
- Backend returns wrappers:
  - `{ suites: [...] }`
  - `{ models: [...] }`
  - `{ total, offset, limit, runs: [...] }`
- Frontend starts runs at `/api/bench/runs`; backend starts at `/api/bench/run`.
- Frontend polls `/api/bench/runs/:id`; backend detail is `/api/bench/run/:id`.
- Frontend cancels with POST `/api/bench/runs/:id/cancel`; backend cancels/deletes with
  `DELETE /api/bench/run/:id`.
- `useApiWithFallback.post()` swallows POST errors and returns `{}`, so the UI fabricates a
  run id instead of failing.

Additional backend issue:

- A real `POST /api/bench/run` followed by `DELETE /api/bench/run/:id` marked the run file
  cancelled, but `/api/bench/runs` still listed the same id as `running`.
- Completed index entries also showed `started_at: 0` in the list while detail had real
  timestamps. The index update path is losing preserved fields.

Fix:

- Pick one REST contract and generate frontend types from it or share OpenAPI/JSON schemas.
- Stop swallowing POST errors.
- Run creation must fail visibly on 405/404/schema mismatch.
- Align frontend to backend:
  - `POST /api/bench/run`
  - `GET /api/bench/run/:id`
  - `GET /api/bench/run/:id/status`
  - `DELETE /api/bench/run/:id`
  - unwrap list payloads.
- Or change backend to match plural frontend, but do it once and document it.
- Fix backend index updates for cancel and completion timestamps.
- Filter SSE by run id or include run id filtering client-side.

### Bench Run Detail

Status: wrong API path and bad loading state.

Observed:

- `/bench/run/bench-run-1` stays on `Loading run bench-run-1...`.
- Direct request to `/api/bench/runs/bench-run-1` returned the SPA HTML shell with 200, not JSON,
  because the API route does not exist and static fallback handled it.

Source cause:

- `BenchRunDetail.tsx` fetches `/api/bench/runs/${id}`.
- Fallback only finds hardcoded demo ids `br-001` and `br-002`.
- Unknown ids never transition to a not-found state.

Fix:

- Use `/api/bench/run/${id}`.
- Detect non-JSON responses.
- Add explicit not-found/error state after fetch fallback fails.
- Link history rows to real run ids returned by the server.

### Bench Compare

Status: renders, but only compares fallback demo runs.

Observed:

- Shows `br-001` and `br-002`.

Fix:

- Either label it "demo comparison" or load real runs from `/api/bench/runs` and compare through
  `/api/bench/runs/compare?ids=...`.

### Bench Showroom

Status: works as an animation, but it is mock-only.

Observed:

- Play/stop/reset works.
- It uses preconfigured demo scenario data, not live run state.

Fix:

- Keep it only as a showroom/mock page, explicitly separate from live benchmark proof.
- Do not use it as evidence that benchmark execution works.

### Explorer

Status: renders health/status/episodes/events, but it inherits fallback and projection ambiguity.

Observed:

- Health page showed active plans 0, active agents 3, providers blank/0.

Fix:

- Make Explorer an API contract debugger:
  - show endpoint name,
  - HTTP status,
  - live/fallback mode,
  - schema validation result,
  - raw payload expansion.
- This would make it useful for diagnosing the exact problems found in this audit.

### Share Pages

Status: routes mismatch and no complete share workflow visible.

Observed:

- `/share/test-token` calls `/api/share/test-token` and shows receipt not found.
- `/dashboard/share/test-token` calls `/api/shared/test-token` and shows shared run not found.

Backend routes:

- `POST /api/runs/{id}/share`
- `GET /api/shared/{token}`
- `GET /runs/{id}` HTML

Fix:

- Delete or redirect `/api/share/:token`; use `/api/shared/:token`.
- Add a visible "share this run" action that calls `POST /api/runs/:id/share`.
- Ensure returned URLs match frontend routes. Backend currently returns `/runs/{token}`, while the
  React app has `/share/:token` and `/dashboard/share/:token`.
- Decide one public share route.

## Cross-Cutting Issue Register

### D01: Silent fallback hides broken live workflows

Evidence:

- `useApiWithFallback` returns demo data for many endpoint failures and returns `{}` for failed
  POSTs.
- Bench run POST failed with 405, but UI still showed a running benchmark.
- Dashboard/knowledge/fleet mixed live and fallback state.

Fix:

- Add explicit modes: `live`, `mock`, `offline`.
- In live mode, endpoint failure is visible and blocks fake success.
- Validate response schemas before rendering.
- Remove fallback from POST/mutation paths entirely.

### D02: Server health intentionally lies on first failure

Evidence:

- `useServerHealth` says investor demo mode reports `connected` on first failed health check.

Fix:

- Remove this behavior from normal app code.
- If needed, create a separate `VITE_DEMO_MOCK=1` mode with visible mock labeling.

### D03: WebSocket error handler can mark a broken terminal connected

Evidence:

- `useTerminal.ts` `ws.onerror` sets status to `connected` when `!handle.ws`.

Fix:

- Set `disconnected` or `error`.
- Expose last error and reconnect count.
- Block scenario play while any required terminal is not actually open.

### D04: Demo scenario handle readiness is broken

Evidence:

- Browser workflows for Build, Race, Providers, Explore, Chat, and Mirage all threw
  `Cannot read properties of undefined (reading 'ws')`.
- Long-wait retry for Providers and Explore still hit the same error.
- `Demo.tsx` filters only `h !== null`, which allows `undefined`.

Fix:

- Filter with `h != null`.
- Require `entries.length === scenario.panes`.
- Require each `entry.ws.readyState === WebSocket.OPEN`.
- Disable Play until handles are ready.
- Show a clear "waiting for terminals" state.

### D05: Terminal scraping is the wrong product contract

Evidence:

- `showCmd()` waits for shell prompt regex and scrapes text for gates/cost/tokens.
- Builder remained `BUILDING...` after a terminal-level failure.
- Costs/tokens/gates are detected via regex over 4096-char rolling output.

Fix:

- Add structured command/workflow events from the backend.
- Terminal transcript can remain visible, but state comes from events:
  - command started,
  - model selected,
  - provider preflight,
  - file changed,
  - gate started/completed,
  - run failed/succeeded,
  - cost/usage known/unknown.

### D06: CLI failure semantics are misleading

Evidence:

- `roko run` halted on missing `ANTHROPIC_API_KEY` but exited 0.
- Provider commands correctly exited 2 for bad args.
- `roko explain "cascade routing"` reported unknown topic but exited 0.

Fix:

- A workflow halt must be nonzero unless explicitly `--allow-failure`.
- Unknown explain topic should be nonzero or produce a structured suggestion result.
- Add `--json` or event stream output for demo automation.

### D07: Model/provider selection is not controlled by the UI or flags

Evidence:

- Builder UI selected Claude Opus, but CLI run used GLM/ZAI in browser.
- Direct fresh temp builder used Anthropic Sonnet and failed missing key.
- Prior dogfood found multiple commands ignoring `--model`.

Fix:

- Implement one effective model/provider selection contract across CLI commands.
- UI should pass a model id only when the CLI/server honors it.
- Render "effective model" from returned workflow state, not from the dropdown alone.

### D08: `roko init` emits a demo-hostile v1 config with no-op gates

Evidence:

- Fresh temp workspaces get schema v1 warnings.
- Default gate is `program = "true"`.
- Config validate says 0 warnings after a schema v1 warning.

Fix:

- Make `roko init` emit current schema/provider tables.
- For language profiles, emit real gates.
- If no real gates can be configured, mark gates as skipped/not configured.
- Count schema migration warnings consistently.

### D09: Cost/usage telemetry is wrong

Evidence:

- Self-host failed episodes recorded $0.00.
- Dashboard showed `$-0.0000`.
- Bench summaries had cost 0 even for completed live runs.
- Prior demo PRD runs recorded hundreds of thousands of output chars with zero tokens/cost.

Fix:

- Parse Claude CLI result usage where available.
- Store unknown usage as null, not zero.
- Fix negative-zero formatting.
- Emit one cost event per attempt.
- Do not let learning/routing treat unknown as free.

### D10: Demo workflows run in ambiguous workdirs

Evidence:

- `/terminal` and `/builder` start in `.roko`.
- Demo scenarios intend temp workspaces, but manual/preset actions can run before setup completes.
- CLI auto-corrected from `.roko` to the Roko repo root during Builder run.

Fix:

- Make the effective workdir explicit and immutable per workflow run.
- Backend workflow API should create and own temp workspaces.
- UI should show workspace path and artifact links.
- Disable actions until setup is complete.

### D11: API route contracts are not tested end-to-end

Evidence:

- Bench plural/singular mismatch.
- Share `/api/share` vs `/api/shared` mismatch.
- `/api/bench/runs/:id` fell through to SPA HTML with 200.

Fix:

- Add contract tests that load the built React app against `roko serve` and assert:
  - no unexpected 4xx/5xx,
  - no HTML returned for API fetches,
  - schema validation passes,
  - all mutations produce real backend state.

### D12: Backend static fallback masks missing API routes

Evidence:

- Direct `/api/bench/runs/bench-run-1` returned `index.html` with 200.

Fix:

- Ensure `/api/*` unmatched routes return JSON 404, never SPA HTML.
- Add a content-type guard in frontend `useApi`.

### D13: Bench SSE is not scoped to the requested run id

Evidence:

- Frontend connects to `/api/bench/events?bench_id=...`.
- Backend `bench_events_sse` does not read the query; it emits all bench events.

Fix:

- Add query parsing/filtering by run id, or make client filter every event by `run_id`.
- Include replay/cursor behavior if the UI reconnects.

### D14: Security posture is unsafe for a browser demo if exposed

Evidence:

- Route module comment says PTY terminal sessions for web UI have no auth.
- Health/auth config shows serve auth disabled.
- CORS headers allow `*`.

Fix:

- Bind terminal routes to localhost-only by default.
- Require auth or a session token for terminal and mutation routes.
- Do not ship unauthenticated PTY when binding public interfaces.

### D15: The app has no proof harness for its promised workflows

Evidence:

- Most tabs rendered, but several workflows were fake, broken, or empty.
- No automated test caught the route/API drift.

Fix:

- Add a Playwright suite that runs against a real `roko serve` and asserts every demo scenario
  reaches a terminal state:
  - completed,
  - failed with useful error,
  - or explicitly skipped because preflight failed.
- Store screenshots, console logs, API traces, and backend run ids.

## Best Possible Solution From This Pass

The fastest path to a trustworthy demo is a staged repair, not a broad UI rewrite.

### P0: Stop fake success

Checklist:

- Remove silent fallback from mutation paths.
- Add explicit mock/live mode.
- Make endpoint failures visible.
- Return JSON 404 for unmatched `/api/*`.
- Fix `useServerHealth` and `useTerminal` false-connected states.
- Fix CLI workflow halt exit statuses or expose structured failed state.

Why first:

- Until the product stops pretending failed paths are live, every later demo can produce false
  confidence.

### P1: Align backend/frontend API contracts

Checklist:

- Fix bench routes and response shapes.
- Fix share route names and returned URLs.
- Add schema validation in frontend.
- Add API contract tests.
- Fix bench index updates for cancel/completion timestamps.

Why second:

- Bench is currently the cleanest place to prove a typed live workflow, and it is mostly blocked
  by contract drift.

### P2: Make workspace/session ownership explicit

Checklist:

- Add terminal/workflow workdir parameter.
- Default terminals to repo root unless scenario creates a temp root.
- Block UI actions until setup finishes.
- Store run workspace and artifact links.
- Fix demo terminal handle readiness.

Why third:

- Workdir ambiguity caused the builder and self-hosting evidence to be untrustworthy.

### P3: Repair CLI execution contracts

Checklist:

- One effective model/provider selection path.
- Current config emitted by `roko init`.
- Real gate profiles or skipped gates.
- Unknown cost as null.
- Nonzero failures.
- JSON/event output for automation.

Why fourth:

- Demo UI cannot truthfully show model, cost, gates, files, or result until CLI contracts are
  truthful.

### P4: Replace scripted PTY demos with typed workflow runs

Checklist:

- `POST /api/demo/scenarios/:id/run`.
- `GET /api/demo/runs/:id`.
- `GET /api/demo/runs/:id/events`.
- Terminal transcript attached as an artifact, not scraped for truth.
- Scenario definitions live server-side or shared as typed data.
- Every scenario has preflight, run, terminal state, and acceptance checks.

Why fifth:

- This preserves the current visual demo style while making it deterministic and debuggable.

### P5: Rebuild scenario content around real, attainable proofs

Checklist:

- Self-hosting: run in the Roko repo or relabel as new-project PRD.
- Build: one small deterministic repo, one configured model, real files/gates.
- Race: two real strategies with structured summaries.
- Providers: preflight providers and skip unavailable ones.
- Explore: seeded workspace with real episodes/knowledge.
- Chat: only after `ChatAgentSession` lands.
- Mirage: hide until real backend exists.
- Bench: live typed benchmark flow.

Why sixth:

- The current script content advertises features that the current CLI/API cannot prove.

### P6: Add demo acceptance tests

Checklist:

- Build test.
- Route smoke test.
- API schema test.
- Browser workflow test for each route.
- CLI scenario test for each scripted command.
- Artifacts written under a timestamped `/tmp/roko-demo-audit-*` folder.

Why seventh:

- This prevents the same drift from returning.

## Demo Readiness Acceptance Criteria

Do not call the demo app ready until all of these are true:

- `npm run build` passes.
- Browser console has no unexpected errors on route smoke.
- No route shows mixed live/fallback state without a visible mock label.
- `/api/*` never returns SPA HTML.
- `/terminal` starts in the expected workdir.
- `/builder` waits for workspace setup, honors selected model, and reaches success or failed.
- `/demo` every tab either completes, fails with useful evidence, or is explicitly disabled.
- Provider demo uses current CLI flags or a real backend provider override.
- Bench run creates a real backend run id.
- Bench cancel updates both detail and list state.
- Bench detail loads real run ids and shows not-found for missing ids.
- Share creation and share viewing use one route contract.
- CLI workflow halts are represented as failures, not success.
- Cost, tokens, gates, and changed files come from structured data, not terminal regexes.
- Playwright proof artifacts are retained for every demo run.

## Files Most Worth Reading Before Implementation

- `demo/demo-app/src/main.tsx`
- `demo/demo-app/src/pages/Demo.tsx`
- `demo/demo-app/src/lib/scenarios.ts`
- `demo/demo-app/src/hooks/useTerminal.ts`
- `demo/demo-app/src/hooks/useTerminalSession.ts`
- `demo/demo-app/src/pages/Builder.tsx`
- `demo/demo-app/src/hooks/useBench.ts`
- `demo/demo-app/src/hooks/useBenchSSE.ts`
- `demo/demo-app/src/hooks/useApiWithFallback.ts`
- `demo/demo-app/src/hooks/useServerHealth.ts`
- `demo/demo-app/src/pages/BenchRunDetail.tsx`
- `demo/demo-app/src/pages/Share.tsx`
- `demo/demo-app/src/pages/dashboard/ShareView.tsx`
- `crates/roko-serve/src/routes/bench.rs`
- `crates/roko-serve/src/routes/shared_runs.rs`
- `crates/roko-serve/src/terminal.rs`

## Bottom Line

The demo app should become the proof harness for the shortest path to Mori parity. Today it is
mostly a showcase shell around unproven or broken workflows. The repair should start by making
truth visible, aligning typed API contracts, and moving product workflows off terminal scraping.
After that, the UI can be an excellent end-to-end demonstration because every pane can point to a
real run id, real artifacts, real gates, and real failure details.
