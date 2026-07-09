# Part III: Implementation guidance

This document covers how to build the visual gate. Part I (`01-roko-architecture.md`) defines the architecture. Part II (`02-feature-spec.md`) defines the data model and contracts. This document tells you what to build, in what order, how to test it, and what to watch out for.

---

## 19. Implementation responsibilities

This section organizes the work by component. Each component is a self-contained piece you can build, test, and wire independently.

### 19.1 Task parser extension

**What.** Extend the task TOML parser to recognize `[task.ui]` sections and deserialize them into `UiTaskSpec`.

**Where.** The task parser lives in the CLI crate. In the roko codebase, this is `crates/roko-cli/src/task_parser.rs` (or a similar module). Add the `UiTaskSpec` types there, or in a shared types module if one exists.

**Changes.**
1. Add `ui: Option<UiTaskSpec>` to the `TaskDef` struct.
2. Add all serde structs from Part II section 10: `UiTaskSpec`, `UiViewport`, `UiJourney`, `UiStep`, `UiAssertion`, `UiScreenshotPolicy`, `UiArtifactRetention`.
3. Add validation:
   - At least one journey if `[task.ui]` is present.
   - Each journey needs at least one step or a `start_url` plus assertions.
   - `target_score` in `0.0..=10.0`.
   - Viewport dimensions positive and ≤ 4096.
   - `Evaluate` steps rejected unless security config allows them.
4. Include UI context (visual_goal, acceptance criteria, viewport names) in fix prompts.

**Tests.**
- Parse a valid `[task.ui]` with all fields populated.
- Parse a minimal `[task.ui]` with only `url` and one journey step.
- Parse a task WITHOUT `[task.ui]` — result is `None`, not an error.
- Reject invalid step types with a clear error message.
- Reject invalid assertion types.
- Parse viewports with and without `device_scale_factor`, `is_mobile`, `has_touch`.
- Validate: missing journeys → error.
- Validate: target_score 11.0 → error.
- Validate: viewport width 0 → error.
- Round-trip: parse → serialize → parse produces identical struct.

### 19.2 UiGate implementation

**What.** A Rust struct implementing the `Gate` trait (from `roko-core`). The `verify()` method runs the five-tier pipeline described in Part II section 15.

**Where.** The gate crate (`crates/roko-gate/`). Add a `ui_gate.rs` module and re-export from `lib.rs`.

**Dependencies.**
- `UiTaskSpec` and all related types from Part II.
- The `Gate` trait and `Verdict` type from Part I (in `roko-core`).
- The browser runner (section 19.3) — invoked as a subprocess.
- The visual evaluator (section 19.4) — invoked via LLM API.

**Key behaviors.**
- If the task has no `[task.ui]`, return `Verdict::pass("ui")` immediately.
- If `gates.ui.enabled = false`, return `Verdict::pass("ui")` immediately.
- Start the dev server if configured. Use an RAII guard (`DevServerHandle`) to kill it on drop — this guarantees cleanup even on early returns.
- Build `BrowserRunSpec` from `UiTaskSpec` + `UiGateConfig`.
- Create the output directory before spawning the runner.
- Spawn the browser runner and read `BrowserRunResult`.
- Walk tiers 1–4, short-circuiting on hard failures.
- Run visual evaluation (tier 5) only if enabled AND all hard checks passed.
- Construct the final `Verdict` with the normalized score.

**Tests.** Use a mock runner — a test script that returns canned JSON, or mock the subprocess call.
- No UI spec → pass.
- Gate disabled → pass.
- Infrastructure failure (runner returns empty viewports) → fail.
- Functional failure (a step has `success: false`) → fail.
- Assertion failure (hard assertion fails) → fail.
- Console error with `hard_fail_on_console_error` → fail.
- Console error without flag → pass (but console captured).
- Horizontal overflow → fail.
- Visual score below threshold → fail.
- Visual score above threshold → pass with score.
- Multiple viewports, one fails → overall fails.
- Dev server timeout → infrastructure fail.

### 19.3 Browser runner (Node.js)

**What.** A standalone Node.js script that reads a `BrowserRunSpec`, runs Playwright journeys, and writes a `BrowserRunResult` plus artifacts.

**Where.** `tools/roko-ui-runner.mjs` at the workspace root.

This is covered in detail in `04-browser-runner.md` (a separate document focused entirely on the runner implementation).

### 19.4 Visual evaluator integration

**What.** A Rust function that sends screenshots to a vision-capable LLM and parses the response into `VisualEvalResult`.

**Where.** Inside the gate crate alongside `UiGate`, or in the CLI crate if LLM dispatch is only available there.

**Prompt template.** The evaluator sends a multi-part message to the vision model:

```
System: You are a UI quality evaluator for an automated verification pipeline.
Score the screenshot on each dimension using the rubric below.
Return ONLY valid JSON matching the response schema. No commentary outside the JSON.

User:
[Image: base64-encoded PNG screenshot]

## Task
ID: {task_id}
Title: {task_title}
Acceptance criteria:
{bulleted list}

## Visual goal
{visual_goal or "No specific visual goal provided."}

## Viewport
{viewport_name}: {width}x{height} {mobile_indicator}

## Browser evidence
- Console errors: {count}
- Page errors: {count}
- Failed requests: {count}
- Layout: {overflow yes/no}, {clipped_count} clipped, {overlap_count} overlapping

## Prior attempt (if applicable)
Previous score: {prev_score}/10
Previous findings: {brief summary}
Agent was asked to fix: {what_to_fix summary}

## Scoring rubric
Score each dimension 0–10:
1. Task completion (weight 0.25): Required workflow state visibly achieved?
2. Layout integrity (weight 0.20): No overlap, clipping, broken spacing, bad alignment?
3. Responsive quality (weight 0.15): Works at this viewport size?
4. Interaction clarity (weight 0.10): Controls, focus, states clear?
5. Visual polish (weight 0.10): Typography, spacing, hierarchy intentional?
6. Design-system fit (weight 0.10): Consistent with app conventions?
7. Accessibility affordance (weight 0.10): Contrast, tap targets, labels?

## Response schema
{
  "score": <weighted_total 0-10>,
  "confidence": <0.0-1.0>,
  "rubric_scores": {
    "task_completion": <0-10>,
    "layout_integrity": <0-10>,
    "responsive_quality": <0-10>,
    "interaction_clarity": <0-10>,
    "visual_polish": <0-10>,
    "design_system_fit": <0-10>,
    "accessibility_affordance": <0-10>
  },
  "summary": "<2-3 sentences: what's good, what needs fixing>",
  "findings": [
    {
      "severity": "high" | "medium" | "low",
      "viewport": "<viewport_name>",
      "journey_id": "<journey_id>",
      "screenshot": "<screenshot_path>",
      "area": "<part of UI>",
      "problem": "<concrete observation>",
      "evidence": "<what you see>",
      "suggested_fix": "<actionable instruction>",
      "dimension": "<rubric dimension name>",
      "selector": "<CSS selector if identifiable, null otherwise>"
    }
  ]
}
```

**Parsing.** Extract JSON from LLM response. Strip markdown code fences if present. On invalid JSON, return `VisualEvalResult` with score 0.0 and a parse-failure finding. Never crash the gate.

**Tests.**
- Mock valid JSON response → correct `VisualEvalResult`.
- Mock response with ` ```json ... ``` ` fences → stripped, parsed correctly.
- Mock invalid response → score 0.0 with error finding.
- Verify prompt includes all 7 context pieces.
- Verify weighted score calculation matches rubric weights.

### 19.5 Orchestrator wiring

**What.** Wire `UiGate` into the orchestration loop so that tasks with `[task.ui]` automatically have their UI verified after code gates pass.

**Where.** The orchestrator module (`crates/roko-cli/src/orchestrate.rs`).

**Changes.**

1. **Gate pipeline extension.** After the existing rung pipeline completes (rungs 0–6), check if the task has a UI spec and the gate is enabled. If so, invoke `UiGate::verify()`.

2. **Retry feedback injection.** When `UiGate` returns a failing verdict:
   - Read `result.json` and `eval/*.json` from the attempt's artifact directory.
   - Format the retry feedback (Part II section 17).
   - Include the feedback in the agent's next prompt as a message or system prompt layer.
   - Include screenshot paths so the agent can reference them.

3. **Artifact storage.** Before running the gate, create the attempt directory (`.roko/ui-runs/{task_id}/{attempt:03}/`). After the gate returns, verify artifacts were written. Store the verdict as `verdict.json`. Update `summary.json` with the attempt record.

4. **Attempt tracking.** Use the existing attempt counter. Each UI gate retry increments it. The artifact directory uses the padded attempt number (`001`, `002`, etc.).

5. **Learning events.** After each UI gate run, emit a learning event:
   ```json
   {
     "event": "ui_gate_result",
     "task_id": "F3",
     "attempt": 2,
     "passed": true,
     "visual_score": 8.4,
     "failure_tier": null,
     "failure_classes": [],
     "hard_failure_count": 0,
     "soft_finding_count": 1,
     "duration_ms": 18423,
     "model_implementer": "claude-opus-4-6",
     "model_evaluator": "claude-sonnet-4-6",
     "prompt_variant": "ui-basic",
     "cost_usd": 0.042,
     "timestamp": "2026-04-25T11:35:00Z"
   }
   ```

6. **Dashboard events.** Emit SSE events for the dashboard and control plane:
   ```rust
   ServerEvent::UiGateStarted { plan_id, task_id, run_id }
   ServerEvent::UiGateArtifact { plan_id, task_id, run_id, kind, path }
   ServerEvent::UiGateCompleted { plan_id, task_id, run_id, passed, score }
   ```

### 19.6 HTTP route extensions

**What.** Add routes to the HTTP control plane for triggering and inspecting UI gate results.

**Where.** `crates/roko-serve/src/routes/`. Add a `ui_gate.rs` module.

**Routes.**

| Method | Path | Description |
|---|---|---|
| `POST` | `/api/v1/ui-gate/run` | Trigger a UI gate run. Body: `{ "task_id": "F3", "attempt": 1 }` |
| `GET` | `/api/v1/ui-gate/runs` | List all UI runs (summary per task). |
| `GET` | `/api/v1/ui-gate/runs/{task_id}` | Get `UiRunSummary` for a task. |
| `GET` | `/api/v1/ui-gate/runs/{task_id}/attempts/{attempt}` | Get a specific attempt's `BrowserRunResult` + `VisualEvalResult`. |
| `GET` | `/api/v1/ui-gate/runs/{task_id}/attempts/{attempt}/artifacts/{path}` | Serve an artifact file (screenshot as `image/png`, JSON as `application/json`). |
| `POST` | `/api/v1/ui-gate/runs/{task_id}/label` | Human label: accept/reject a run. Body: `{ "accept": false, "reason": "clipped button" }` |
| `GET` | `/api/v1/ui-gate/config` | Return current `[gates.ui]` config. |
| `PUT` | `/api/v1/ui-gate/config` | Update `[gates.ui]` config (thresholds, enabled, etc.). |

### 19.7 Dashboard / TUI integration

**What.** Show UI gate results in the existing ratatui TUI dashboard.

**Where.** `crates/roko-cli/src/tui/views/`.

**What to show.**
- UI gate pass/fail in the gate summary column.
- Visual score (if tier 5 ran).
- Top failure class.
- Artifact paths.
- Retry score history (score improved? regressed?).
- Latest screenshot path (TUI does not render images inline — just the path).

Initial implementation does not need inline image rendering. Just show paths, scores, and failure summaries.

### 19.8 Learning integration

**What.** Feed UI gate outcomes into roko's learning subsystems.

**Where.** `crates/roko-learn/src/`.

**Events to record.**
- **UI gate attempt event** — task ID, attempt, pass/fail, visual score, failure classes, model used, prompt variant, cost, latency.
- **Model feedback** — feed into CascadeRouter so it learns which models succeed at UI tasks.
- **Prompt variant feedback** — feed into ExperimentStore for prompt A/B testing.
- **Adaptive threshold update** — feed into per-rung threshold history (the UiGate is effectively rung 7+).
- **Human label** — when a human accepts/rejects a run via the API, record it for calibration.

---

## 20. Cybernetic feedback loops

This feature is explicitly cybernetic. Each loop has a sensor, comparator, controller, actuator, memory, and reward. There are nine loops. The first (attempt-level repair) is required for MVP. The rest are post-MVP but the data model should support them from the start.

### 20.1 Attempt-level repair loop (MVP)

**Purpose.** Make one task pass.

**Sensor.** Browser result, screenshots, console/page/network evidence, layout findings, accessibility findings, visual score, failure classes.

**Comparator.** Hard pass rules (all deterministic tiers), target visual score (threshold), previous best attempt (highest score so far).

**Controller.** Orchestrator retry policy, UI feedback compressor (Part II section 17), model escalation policy (try a stronger model if repeated failures).

**Actuator.** Implementer agent edits code based on retry feedback.

**Memory.** `.roko/ui-runs/{task}/{attempt}/` — all artifacts per attempt. Gate verdict signals. Task attempt tracker.

**Reward.** Pass/fail. Visual score delta. Hard failure reduction. Cost and latency to convergence.

**Policy.**
```
if infrastructure failure:
    retry runner only (don't ask agent to change code) or fail with infra class
elif hard UI failures:
    repair prompt prioritizes hard browser evidence (tier 2/3/4)
elif visual score below threshold:
    repair prompt prioritizes top visual findings (tier 5)
elif visual score regressed from previous best:
    reference previous best attempt and what changed
else:
    pass
```

### 20.2 Adaptive threshold loop

**Purpose.** Tune visual score strictness over time.

**Sensor.** UI pass/fail history, visual scores, human accept/reject labels, user reverts or bug reports after automated pass.

**Comparator.** False accept rate, false reject rate, average retries to pass, human override rate.

**Controller.** Adaptive threshold policy (EMA of scores, calibrated against human feedback).

**Actuator.** Adjust target visual score. Reclassify finding types (warning vs hard). Adjust when visual scoring is required vs skipped.

**Memory.** `.roko/learn/ui-thresholds.json`

```json
{
  "dashboard": {
    "target_score": 8.4,
    "score_ema": 8.1,
    "false_accept_rate": 0.04,
    "false_reject_rate": 0.11,
    "retry_mean": 1.8,
    "observations": 47
  }
}
```

### 20.3 Model routing loop

**Purpose.** Pick the cheapest model that succeeds on UI tasks.

**Sensor.** Implementer model, evaluator model, task tier, UI complexity, attempts to pass, visual score, hard failure classes, cost, latency.

**Comparator.** First-attempt UI pass rate, cost-adjusted success rate, mean score improvement per retry.

**Controller.** CascadeRouter or contextual bandit (roko already has both).

**Actuator.** Route future UI tasks to better model tier. Route visual evaluation to cheaper/stronger model based on risk.

**Reward.**
```
reward = 1.0 * passed
       + 0.2 * first_attempt_pass
       + 0.1 * clamp(score_delta / 2.0)
       - 0.1 * retry_count
       - 0.05 * normalized_cost
       - 0.1 * hard_failure_count
```

### 20.4 Prompt strategy loop

**Purpose.** Learn which prompts produce better UI work.

**Sensor.** Prompt template ID, design context included or not, screenshots included or not, playbooks included or not, outcome metrics.

**Comparator.** Pass rate by variant, visual score by variant, cost by variant, failure class distribution.

**Controller.** Prompt experiment system (roko already has one — `ExperimentStore`).

**Actuator.** Promote winning prompt variants. Demote variants that over-optimize visuals and break behavior.

**Variants to test.**
- `ui-basic`: task + files + acceptance only.
- `ui-design-context`: includes style and existing screen context.
- `ui-mobile-first`: asks agent to implement mobile constraints first.
- `ui-stateful`: explicitly requests loading/empty/error/success states.
- `ui-a11y-first`: emphasizes labels, focus, keyboard, contrast.

**Promotion rule.**
```
promote if:
    n >= 20
    pass_rate_lift >= 8%
    cost_increase <= 15%
    human_override_rate does not worsen
```

### 20.5 Assertion synthesis loop

**Purpose.** Convert repeated visual failures into deterministic checks.

**Sensor.** Repeated vision findings, failure classes, human-confirmed issues, successful fix diffs.

**Comparator.** Frequency, deterministic detectability, false positive rate.

**Controller.** Assertion synthesizer.

**Actuator.** Add generated assertions to future tasks. Add project UI policy.

**Example.**
```
Repeated finding:
  "mobile horizontal overflow" in 11 tasks

Synthesized assertion:
  { type = "no_horizontal_overflow" }

Policy:
  Enable for all tasks with mobile viewport.
```

### 20.6 Visual memory loop

**Purpose.** Learn what this product should look like.

**Sensor.** Approved screenshots, rejected screenshots, component snapshots, visual summaries, human labels.

**Comparator.** Current screenshot similarity to approved examples, deviations from local style, recurring design feedback.

**Controller.** Visual memory retriever, design context composer.

**Actuator.** Include relevant approved screenshots or style summaries in future prompts. Flag deviations from local design language.

**Memory.**
```
.roko/visual-memory/
  approved/
    dashboard.desktop.png
    dashboard.mobile.png
  rejected/
    clipped-modal.mobile.png
  summaries.jsonl
  embeddings.jsonl
  style-guide.md
```

Initial implementation: text summaries only. Image embeddings later.

### 20.7 Browser backend selection loop

**Purpose.** Decide when cheap preflight browsers are worth running.

**Sensor.** Preflight backend result, Chromium result, divergence, runtime, memory.

**Comparator.** False pass rate, false fail rate, time saved.

**Controller.** Backend bandit.

**Actuator.** Enable/disable preflight by task class. Choose backend order.

**Policy.**
```
if task has visual_goal:
    chromium required
if task has only DOM/network assertions:
    preflight may run first
if preflight diverges from chromium too often:
    reduce backend weight
```

### 20.8 Human calibration loop

**Purpose.** Align automated judgment with operator taste.

**Sensor.** Human accepts run, human rejects run, human labels finding useful/not useful, human edits after automated pass.

**Comparator.** Automated pass vs human accept, automated score vs human rating, finding usefulness.

**Controller.** Calibration policy.

**Actuator.** Adjust rubric weights, adjust thresholds, improve evaluator prompt, add project style rules.

**CLI.**
```bash
roko ui-gate label <run_id> --accept
roko ui-gate label <run_id> --reject "mobile button clipped"
roko ui-gate label-finding <finding_id> --useful
```

### 20.9 Dream-cycle distillation loop

**Purpose.** Convert many UI episodes into playbooks.

**Sensor.** Completed UI episodes, failure clusters, fix diffs, prompt variants, model choices.

**Comparator.** Which repair tactics work repeatedly, which failures recur, which playbooks improve outcomes.

**Controller.** Offline dream cycle (roko already has the dream infrastructure in `roko-dreams`).

**Actuator.** Promote playbooks into durable knowledge.

**Example playbooks.**
- "For modal clipping on mobile, use max-height, internal scroll, and a visible action row."
- "For dashboard cards, avoid fixed pixel grids below 640px."
- "For failed POST in UI gate, inspect mock/server route before restyling."

---

## 21. Metrics

### Runtime metrics

```
roko_ui_gate_runs_total{backend, passed}           # Total UI gate runs
roko_ui_gate_duration_ms{backend}                   # Wall-clock time per run
roko_ui_gate_visual_score{task_domain}              # Visual evaluator score
roko_ui_gate_failures_total{class}                  # Failures by class
roko_ui_gate_retries_to_pass                        # Retries before first pass
roko_ui_gate_artifact_bytes                         # Storage per run
roko_ui_gate_preflight_divergence_total{backend}    # Preflight vs Chromium mismatches
```

### Learning metrics

- First-attempt UI pass rate.
- Mean visual score by model.
- Score delta per retry (improving? regressing?).
- False accept/reject rate from human labels.
- Prompt variant lift (pass rate delta between variants).
- Backend cost saved (preflight vs full Chromium).
- Retry count by failure class.
- Time-to-pass by task complexity.

---

## 22. Test strategy

### 22.1 Unit tests

Test individual functions in isolation with no browser or network.

- Parse `[task.ui]` from TOML — all field combinations.
- Parse missing `[task.ui]` → returns `None`.
- Validate: missing journeys → error.
- Validate: score bounds (0–10 range, reject 11.0).
- Validate: viewport dimensions (reject 0, reject 5000).
- Convert `BrowserRunResult` → `Verdict` for each failure class.
- Classify UI failure classes from browser evidence.
- Compress UI feedback into retry prompt (≤ 3 items in "what to fix").
- Redact secrets from console messages and network URLs.
- Merge hard assertions and visual eval into final verdict.
- Normalize visual score from 0–10 to 0–1.
- Serialize/deserialize all types (round-trip JSON).
- `UiFailureClass` → string serialization round-trip.
- Retention mode selection based on context (local vs CI, pass vs fail).

### 22.2 Runner tests

Create a fixture app at `tests/fixtures/ui-app/` with static HTML pages that exercise each scenario.

**Fixture app structure.**
```
tests/fixtures/ui-app/
  package.json                # { "scripts": { "dev": "npx serve -p 3456 ." } }
  index.html                  # Landing page with links to all test pages
  pages/
    stats-grid.html           # 4 stat cards in a grid (passing case)
    missing-card.html         # 3 stat cards (element count failure)
    console-error.html        # Page that throws console.error on load
    page-error.html           # Page that throws an unhandled exception
    network-error.html        # Page that makes a failing XHR (500)
    overflow.html             # Page with horizontal overflow (wide table)
    clipped-text.html         # Page with overflow:hidden clipping text
    overlapping-text.html     # Page with absolutely positioned overlapping text
    form.html                 # Form that POSTs on submit
    slow-load.html            # Page that takes 5 seconds to render
    hydration-error.html      # Page that simulates hydration mismatch
    auth-required.html        # Page that returns 401 on an API call
```

**Test scenarios.**

| Scenario | Input | Expected |
|---|---|---|
| Passing flow | Navigate, wait, click, screenshot | `passed: true`, screenshot exists |
| Missing locator | Wait for `#does-not-exist` | Step failure with error message |
| Console error | Navigate to console-error page | `ConsoleMessage` with level "error" captured |
| Page error | Navigate to page-error page | `PageError` captured |
| Failed request | Navigate to network-error page | `NetworkRequest` with `failed: true` or `status: 500` |
| Horizontal overflow | Navigate to overflow page | `layout.horizontal_overflow: true` |
| Clipped text | Navigate to clipped-text page | `clipped_text_candidates` non-empty |
| Overlapping text | Navigate to overlapping-text page | `overlapping_text_candidates` non-empty |
| Form submission | Fill form, click submit | POST request captured in `requests` |
| Timeout | 2000ms timeout, slow-load page | `passed: false`, timeout error |
| Multiple viewports | Desktop + mobile | Both viewport results present |
| Named screenshot | Screenshot step with `name: "after-create"` | File `after-create.png` exists |
| Self-test | `--self-test` flag | Exits 0, outputs `{"self_test": "pass"}` |

### 22.3 Integration tests

Test the full pipeline from `UiGate::verify()` through browser run and back.

- `UiGate` with passing task → `Verdict::pass("ui")` with score.
- `UiGate` with failing task → `Verdict::fail("ui")` with artifact paths in detail.
- `UiGate` with no `[task.ui]` → `Verdict::pass("ui")` immediately.
- `UiGate` with `enabled: false` → `Verdict::pass("ui")`.
- Orchestrator feeds UI feedback into retry prompt (check prompt contains "UI Gate Failure").
- Gate verdict signal is persisted (check `.roko/ui-runs/{task}/verdict.json` exists).
- `summary.json` is updated after each attempt.
- Dev server is started and killed correctly (check port is free after test).
- Visual evaluator is called only when tiers 1–4 pass.
- Multiple attempts → attempt directories `001/`, `002/` created.

### 22.4 Golden tests

Golden files capture expected outputs for specific inputs. Store them in `tests/golden/ui-gate/`.

For each golden test case:

```
tests/golden/ui-gate/{case-name}/
  spec.json               # BrowserRunSpec input
  result.json              # Expected BrowserRunResult
  vision-eval.json         # Expected VisualEvalResult (if tier 5 ran)
  feedback.md              # Expected retry feedback (if gate failed)
```

**Golden test cases.**
- `pass-desktop`: basic passing run, one viewport, no visual eval.
- `pass-visual`: passing run with visual eval above threshold.
- `fail-missing-element`: functional failure, no visual eval.
- `fail-console-error`: runtime failure with NoConsoleErrors assertion.
- `fail-network-error`: runtime failure with hard_fail_on_failed_request.
- `fail-overflow`: layout failure (horizontal overflow).
- `fail-visual-score`: tiers 1–4 pass, visual score below threshold.
- `fail-infrastructure`: dev server timeout, no browser run.
- `multi-viewport-fail`: one viewport passes, one fails.

---

## 23. Security and safety

### Browser sandboxing

The browser runner executes untrusted agent-generated code in a real browser. Sandboxing is mandatory.

**Process isolation.** The Playwright browser runs as a separate process tree. The Rust side communicates via JSON files (spec.json → result.json). The browser runner must not have write access outside its output directory.

**Network policy.** By default, the browser is allowed to connect to:
- `localhost` and `127.0.0.1` (any port) — for the dev server.
- No external hosts.

If `network_allow` is configured in `[gates.ui]`, those hosts are additionally allowed. Block all other outbound connections via Playwright's `context.route()` API.

**Timeout enforcement.** Every browser run has a hard timeout (default 120 seconds, configurable). The Rust side kills the runner subprocess if it exceeds this. Do not trust the runner to self-terminate.

**Resource limits.** Cap artifact storage per run:
- Maximum 50 screenshots per run.
- Maximum 50MB total artifact size per attempt.
- Maximum 200 console messages captured (oldest dropped).
- Maximum 200 network requests captured (oldest dropped).
- Maximum 20 clipped text candidates.

**Evaluate step gating.** `Evaluate` steps and `CustomEvaluate` assertions run arbitrary JavaScript. They are disabled by default and require explicit opt-in: `[gates.ui.security] allow_evaluate_steps = true`.

**External URL gating.** Navigation to non-localhost URLs is blocked by default. Requires `[gates.ui.security] allow_external_urls = true`.

### Secret redaction

Screenshots may capture rendered secrets. Console messages and network URLs are text and can be pattern-matched.

1. **Text evidence redaction.** Apply regex patterns from `[gates.ui.security] redact_text_patterns` to:
   - Console message text.
   - Network request URLs.
   - DOM snapshot text.
   - HAR file content.
   Replace matches with `[REDACTED]`.

2. **Header redaction.** Remove or redact headers listed in `[gates.ui.security] redact_headers` from stored network evidence and HAR files.

3. **Screenshot policy.** Do not attempt to OCR and redact screenshot pixels (too fragile). Instead:
   - Task specs should exclude pages that display secrets.
   - Dev servers should use mock/test credentials.
   - If the page URL contains `token=`, `key=`, `secret=`, or `password=` query params, log a warning in the verdict detail.

4. **Model provider policy.** Screenshots are sent to the visual evaluator (LLM). This means they leave the local machine. The security config should document which model provider receives screenshots.

### File system safety

The runner writes to a single output directory: `.roko/ui-runs/{task-id}/{attempt}/`. The Rust side creates this before spawning the runner and passes it as a parameter. After the run, verify no symlinks were created pointing outside the output directory.

### Dev server cleanup

The `DevServerHandle` RAII guard kills the dev server on drop. On Unix: send SIGTERM, wait 5 seconds, send SIGKILL. On Windows: `taskkill /F /T`. The gate must not leak dev server processes.

---

## 24. MVP scope and task breakdown

The MVP is the smallest useful slice: an agent implements a frontend task, the UI gate runs a browser journey, checks deterministic assertions, and fails with actionable feedback. Visual evaluation (tier 5) is included in the schema but can be wired in a second slice.

### Task order

```toml
[[task]]
id = "VG-01"
title = "Add UI task spec types and parser"
description = """
Define UiTaskSpec, UiViewport, UiJourney, UiStep, UiAssertion, UiScreenshotPolicy,
UiArtifactRetention types. Add TOML parsing support for [task.ui] sections with
validation.
"""
acceptance = [
    "All types from Part II section 10 are defined with serde derives",
    "tasks.toml with [task.ui] parses into UiTaskSpec",
    "tasks.toml without [task.ui] returns None",
    "Invalid step/assertion types produce clear errors",
    "Validation: missing journeys, out-of-range scores, zero-width viewports → error",
]
verify = [{ phase = "test", command = "cargo test task_parser" }]

[[task]]
id = "VG-02"
title = "Create Playwright JSON browser runner"
description = """
Node.js script at tools/roko-ui-runner.mjs. Reads BrowserRunSpec from --spec flag.
Runs Playwright journeys with Chromium. Writes BrowserRunResult to result.json.
Captures screenshots, console messages, network requests, layout metrics.
See 04-browser-runner.md for full implementation spec.
"""
acceptance = [
    "Runner reads spec.json via --spec flag",
    "Runner writes result.json to output_dir",
    "Screenshots saved to viewport/journey/ subdirectories",
    "Console messages captured with level, text, url",
    "Network requests captured with url, method, status, failed",
    "Layout metrics detect horizontal overflow and clipped text",
    "Self-test mode works (--self-test flag → exit 0)",
    "Exit 0 on UI failure (failure in JSON, not exit code)",
    "Exit 1 only on infrastructure errors",
]
verify = [{ phase = "node", command = "node tools/roko-ui-runner.mjs --self-test" }]

[[task]]
id = "VG-03"
title = "Implement UiGate with tiers 1–4"
depends_on = ["VG-01", "VG-02"]
description = """
Rust UiGate implementing Gate trait. Tiers 1–4 only (infrastructure, functional,
runtime, layout). Shells out to VG-02 runner. Returns Verdict.
"""
acceptance = [
    "UiGate implements Gate trait from roko-core",
    "No-UI-spec tasks return Verdict::pass immediately",
    "Disabled gate returns Verdict::pass",
    "Infrastructure failure → hard fail",
    "Step failure → hard fail",
    "Console error with hard_fail_on_console_error → hard fail",
    "Horizontal overflow → hard fail",
    "Passing journey → Verdict::pass with evidence stored",
    "Dev server started and killed via RAII guard",
]
verify = [{ phase = "test", command = "cargo test ui_gate" }]

[[task]]
id = "VG-04"
title = "Wire UiGate into orchestrator"
depends_on = ["VG-03"]
description = """
Add UiGate to the gate pipeline for tasks with [task.ui]. Run after existing
code gates. Store artifacts in .roko/ui-runs/. Update summary.json.
"""
acceptance = [
    "Tasks with [task.ui] run UiGate after code gates",
    "Tasks without [task.ui] are unaffected",
    "Artifacts stored in .roko/ui-runs/{task_id}/{attempt:03}/",
    "verdict.json written after each attempt",
    "summary.json updated with attempt record",
]
verify = [{ phase = "test", command = "cargo test orchestrate" }]

[[task]]
id = "VG-05"
title = "Add UI gate retry feedback"
depends_on = ["VG-04"]
description = """
When UiGate fails, format structured retry feedback per Part II section 17.
Inject into agent's next prompt. Include screenshot paths.
"""
acceptance = [
    "Failed gate produces markdown feedback with sections",
    "Hard failures listed first",
    "Soft findings listed with severity and dimension",
    "'What to fix' section has ≤ 3 items",
    "Screenshot paths included in evidence section",
    "Agent receives feedback in retry prompt",
]
verify = [{ phase = "test", command = "cargo test ui_feedback" }]

[[task]]
id = "VG-06"
title = "Add visual evaluator (tier 5)"
depends_on = ["VG-03"]
description = """
Send screenshots to vision LLM for scoring. Parse VisualEvalResult.
Wire into UiGate as tier 5. Only runs when tiers 1–4 pass and
visual_eval_enabled is true.
"""
acceptance = [
    "Evaluator sends screenshot + rubric to vision model",
    "Response parsed into VisualEvalResult with 7 dimension scores",
    "Weighted total computed per rubric weights",
    "Score below threshold fails the gate",
    "Score above threshold passes with score recorded",
    "Invalid LLM response → score 0 with parse-failure finding",
    "Eval results stored in eval/ subdirectory",
]
verify = [{ phase = "test", command = "cargo test visual_eval" }]

[[task]]
id = "VG-07"
title = "Add config schema and [gates.ui] support"
depends_on = ["VG-03"]
description = """
Add [gates.ui] section to roko.toml schema. Parse into UiGateConfig.
Include security sub-section. Wire config into UiGate construction.
"""
acceptance = [
    "[gates.ui] parsed from roko.toml into UiGateConfig",
    "Missing section uses defaults (enabled=false)",
    "Security sub-section parsed (redact_headers, redact_text_patterns, etc.)",
    "All config fields respected at runtime",
    "Task-level overrides work (target_score, timeout, etc.)",
]
verify = [{ phase = "test", command = "cargo test ui_gate_config" }]

[[task]]
id = "VG-08"
title = "Add HTTP routes and dashboard integration"
depends_on = ["VG-04"]
description = """
Add REST endpoints for triggering and inspecting UI gate runs.
Add dashboard events. Show UI gate status in TUI.
"""
acceptance = [
    "POST /api/v1/ui-gate/run triggers a run",
    "GET /api/v1/ui-gate/runs/{task_id} returns summary",
    "GET /api/v1/ui-gate/runs/{task_id}/attempts/{n}/artifacts/screenshots/* serves PNG",
    "SSE events emitted for gate start/complete",
    "TUI shows UI gate pass/fail and visual score",
]
verify = [{ phase = "test", command = "cargo test ui_gate_routes" }]

[[task]]
id = "VG-09"
title = "Add learning events for UI outcomes"
depends_on = ["VG-04"]
description = """
Emit learning events after each UI gate run. Feed into cascade router,
experiment store, and adaptive thresholds.
"""
acceptance = [
    "Learning event emitted with all fields (task, attempt, score, model, cost, etc.)",
    "CascadeRouter receives feedback for UI task outcomes",
    "Adaptive thresholds updated for UI gate rung",
    "Human labels recordable via API route",
]
verify = [{ phase = "test", command = "cargo test ui_learning" }]
```

### Dependency graph

```
VG-01 (types/parser)──────────────────────────┐
  │                                            │
  │    VG-02 (browser runner) ← parallel ──────┤
  │       │                                    │
  └───────┴──→ VG-03 (UiGate tiers 1–4)       │
                  │                            │
                  ├──→ VG-04 (orchestrator) ←──┘
                  │       │
                  │       ├──→ VG-05 (retry feedback)
                  │       │
                  │       ├──→ VG-08 (HTTP routes + dashboard)
                  │       │
                  │       └──→ VG-09 (learning events)
                  │
                  ├──→ VG-06 (visual evaluator, tier 5)
                  │
                  └──→ VG-07 (config schema)
```

### MVP boundary

**MVP (required for first useful slice):** VG-01 through VG-05 plus VG-07.

This gives you: task parsing, browser runner, deterministic gate (tiers 1–4), orchestrator wiring, retry feedback, and config. An agent can implement a frontend task, the UI gate runs it in a browser, captures evidence, fails with feedback, and the agent retries.

**Second slice:** VG-06 (visual evaluator) adds tier 5 scoring.

**Third slice:** VG-08 (routes/dashboard) and VG-09 (learning) complete the integration.

---

## 25. Acceptance criteria

### MVP is complete when

1. A task can declare `[task.ui]` with viewports, journeys, steps, and assertions.
2. Roko can parse the `[task.ui]` section into typed Rust structs with validation.
3. A Playwright runner can execute the journey and return structured results.
4. The runner captures screenshots, console events, page errors, network requests, and layout data.
5. `UiGate` implements the `Gate` trait and returns a normal `Verdict`.
6. A failed UI gate includes actionable retry feedback with artifact paths.
7. Roko retries a failed frontend task using UI feedback.
8. UI artifacts persist under `.roko/ui-runs/` with deterministic paths.
9. Hard browser failures short-circuit before visual scoring.
10. The gate can be enabled/disabled via `[gates.ui]` config.
11. Secret redaction applies to stored evidence.

### Post-MVP is complete when

1. Visual evaluator (tier 5) runs and produces scored `VisualEvalResult`.
2. Human labels calibrate thresholds via `roko ui-gate label`.
3. Prompt and model routing learn from UI outcomes.
4. Repeated findings become playbooks or generated assertions.
5. Optional preflight backends can run and track divergence from Chromium.
6. HTTP routes serve UI run results and screenshots.
7. Dashboard/TUI shows UI gate outcomes and score history.

---

## 26. Rollout plan

| Stage | What | When |
|---|---|---|
| 1 | Manual command: `roko ui-gate run --spec spec.json` | After VG-03 |
| 2 | Opt-in per task: if `[task.ui]` exists, run UiGate | After VG-04 |
| 3 | Project-level enable: `[gates.ui] enabled = true` | After VG-07 |
| 4 | Visual evaluator enabled (tier 5) | After VG-06 |
| 5 | Learning events and dashboard integration | After VG-08 + VG-09 |
| 6 | Human calibration and visual memory | Post-MVP |
| 7 | Preflight backend experimentation | Post-MVP |

---

## 27. Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Visual evaluator is subjective | Bad pass/fail decisions | Hard checks first (tiers 1–4); fixed rubric with 7 weighted dimensions; human calibration loop |
| Browser runner flakes | Wasted retries | Detailed traces; timeout enforcement; infrastructure failure class separated from code failures |
| Screenshots leak secrets | Security issue | Pattern-based text redaction; header redaction; network policy blocks external hosts |
| UI gate slows execution | Poor throughput | Only runs for tasks with `[task.ui]`; skip visual eval when disabled; cache dev server across viewports |
| Agent optimizes for screenshots only | Behavior regressions | Hard journeys and network checks (tiers 2–3) BEFORE visual scoring (tier 5) |
| Generated journeys are weak | False confidence | Require explicit acceptance criteria; assertion synthesis promotes repeated failures to checks |
| Preflight diverges from Chromium | Incorrect preflight | Preflight is advisory only; track divergence; disable when divergence exceeds threshold |
| Runner subprocess hangs | Blocked pipeline | Hard timeout with process kill; RAII guard for dev server; separate runner timeout |
| Agent produces non-functional but visually passable code | Broken behavior with high score | Deterministic first principle: tiers 1–4 must pass before tier 5 runs |

---

## 28. Open questions

1. Should `UiGate` itself call the visual evaluator, or should the orchestrator combine browser result + visual result into a final verdict? (Current design: UiGate calls evaluator internally.)

2. Should the Playwright runner live in `tools/`, `crates/roko-cli/assets/`, or a separate npm package? (Current design: `tools/roko-ui-runner.mjs`.)

3. How often should screenshots be embedded in prompts vs referenced by path? (Current design: paths in retry feedback, base64 only for visual evaluator.)

4. Should UI gates run before or after generated behavioral tests? (Current design: after all code gates.)

5. How should Roko infer journeys from natural language acceptance criteria? (Not in MVP; future work.)

6. What should be the default policy for external network calls during browser runs? (Current design: block all except localhost.)

7. Should visual memory store raw screenshots, summaries, embeddings, or all three? (Current design: text summaries initially, embeddings later.)

8. How should Roko compare screenshots across runs without brittle pixel diffs? (Current design: LLM evaluation, not pixel comparison.)

9. Should the runner support Playwright's `--headed` mode for debugging? (Probably yes as a config flag, but not in MVP.)

10. Should the visual evaluator see multiple screenshots at once (all viewports for a journey) or one at a time? (Current design: one at a time, but batching could improve context.)

---

## 29. Definition of done

This feature is done when Roko can take a frontend task, implement it through an agent, run the rendered app in a real browser, press the relevant controls, capture functional and visual evidence, fail with actionable feedback when the UI is wrong, retry with that feedback, pass when the UI meets hard and visual criteria, and store enough data for future agents to improve their first-attempt frontend quality.
