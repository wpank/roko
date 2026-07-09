# PRD: Visual Gate and Browser Feedback Loop for Roko Agents

Status: Draft
Audience: Implementation agent with no prior Roko context
Primary owner: Roko
Primary modules: `roko-core`, `roko-gate`, `roko-cli`, `roko-serve`, `roko-learn`, dashboard
Primary outcome: Roko agents can implement frontend tasks, render and operate the UI in a real browser, capture visual and browser evidence, judge whether the UI works and looks right, and iterate until the task passes objective and visual standards.

## 0. How To Read This Document

This document is intentionally self-contained. It assumes the reader has not seen the Roko repository, the previous conversation, or existing design docs.

Roko is an agentic software engineering system. It takes a request, decomposes it into tasks, dispatches coding agents, verifies their output through gates, feeds failures back into retries, persists every meaningful event as a signal, and learns from the outcomes. The requested feature extends that loop to frontend and visual work.

The practical feature name in this PRD is `UiGate`. `VisualGate` may be used in user-facing copy, but implementation should use a precise name like `UiGate` because the gate verifies more than visual appearance: it also checks browser functionality, console errors, failed requests, layout, accessibility, and task workflows.

## 1. Executive Summary

Roko already verifies code with compile, lint, test, integration, and review gates. That is not enough for frontend work. A UI can compile, pass unit tests, and still be unusable: the button may be clipped on mobile, the modal may overflow, the route may throw a console error, a form submission may 500, or the screen may simply not match the requested product quality.

This PRD defines a first-class browser and visual verification system:

1. A task can declare UI journeys, assertions, viewports, and visual goals.
2. After a frontend agent completes code changes, Roko runs normal gates first.
3. If code-level gates pass, Roko starts or connects to the frontend app.
4. Roko runs a Playwright-based browser runner against the rendered UI.
5. The runner presses buttons, fills inputs, waits for UI state, captures screenshots, records traces, collects console logs, page errors, request/response data, accessibility snapshots, and layout metrics.
6. Deterministic checks run first. These include workflow completion, no console errors, no failed app requests, no horizontal overflow, required text visible, and similar hard assertions.
7. A separate vision-capable evaluator scores screenshots and returns structured findings.
8. `UiGate` converts this evidence into a normal Roko `Verdict`.
9. If the verdict fails, Roko creates a targeted retry prompt that includes exact browser evidence and artifact paths.
10. The agent retries until the UI passes or retry policy stops the task.
11. All attempts become learning data for future routing, prompting, thresholds, playbooks, and visual memory.

The default authoritative backend must be real Chromium via Playwright. Lightweight CDP engines such as Obscura may be useful as optional fast preflight backends, but they should not be the source of truth for visual acceptance until they match Chromium on screenshots, layout, CSS rendering, accessibility, and modern app compatibility.

## 2. What Roko Is

Roko is a Rust toolkit for building agents that build software. Its core loop is:

```text
observe -> plan -> execute -> verify -> learn -> repeat
```

At a lower architectural level, every agent turn follows:

```text
query -> score -> route -> compose -> act -> verify -> write -> react
```

The loop means:

- `query`: retrieve relevant prior signals, task context, code context, memories, and policies.
- `score`: rank the retrieved information by relevance, recency, trust, and utility.
- `route`: choose a model, agent role, backend, tool, or strategy.
- `compose`: build a prompt or work packet from selected context.
- `act`: run the coding agent or tool.
- `verify`: run gates against the result.
- `write`: persist outputs, verdicts, artifacts, and metrics.
- `react`: trigger retries, escalation, replanning, learning, or dashboard updates.

The Visual Gate fits in `verify`, then feeds `write` and `react`.

## 3. Roko Architecture Primer

### 3.1 One Noun: Signal / Engram

Everything important in Roko is represented as a signal. In code, the primary type is commonly called `Engram`.

An `Engram` is:

- content-addressed with a hash
- timestamped
- typed by `Kind`
- scored
- decay-aware
- attributed to a producer
- linked to parent signals through lineage
- tagged with metadata
- persisted for replay and learning

Examples of signals:

- task definitions
- prompts
- agent outputs
- diffs
- gate inputs
- gate verdicts
- failures
- episodes
- learning events
- browser artifacts
- human labels

Signals form a DAG. This is important because a future agent, a dashboard, or a forensic replay can trace:

```text
Task -> Prompt -> AgentOutput -> UiGateInput -> UiGateVerdict -> RetryPrompt -> NextAgentOutput
```

### 3.2 Six Verbs

Roko's architecture is organized around six traits. An implementation agent does not need exact code, but should respect these boundaries.

| Trait | Purpose |
|---|---|
| `Substrate` | Store and query signals. Backends may be memory, file, semantic index, or chain. |
| `Scorer` | Rate signals for relevance, priority, novelty, confidence, or trust. |
| `Gate` | Verify a signal against external reality and produce a `Verdict`. |
| `Router` | Pick among models, agents, tools, prompts, gates, or backends. Learns from outcomes. |
| `Composer` | Pack selected signals into a prompt or tool request. |
| `Policy` | React to patterns, such as retries, escalation, replanning, or learning updates. |

`UiGate` is primarily a `Gate`, but it also creates data for `Router`, `Scorer`, `Composer`, and `Policy`.

### 3.3 Gate Trait

A gate verifies an input signal and returns a verdict. A failure is not an exception. A failure is a normal verdict.

Conceptual Rust shape:

```rust
#[async_trait::async_trait]
pub trait Gate: Send + Sync {
    async fn verify(&self, engram: &Engram, ctx: &Context) -> Verdict;
    fn name(&self) -> &str;
}
```

### 3.4 Verdict Type

A `Verdict` is the verification result. It should include enough evidence for downstream systems to retry intelligently.

Conceptual fields:

```rust
pub struct Verdict {
    pub passed: bool,
    pub reason: String,
    pub gate: String,
    pub score: f32,              // usually 0.0 to 1.0
    pub detail: Option<String>,  // concise diagnostic
    pub error_digest: Option<String>,
    pub duration_ms: u64,
}
```

For `UiGate`, `score` should usually be the normalized visual score if hard checks passed, or `0.0` if hard checks failed. The full browser evidence should live in artifacts, not only in the `detail` string.

### 3.5 Gate Pipeline

Roko composes gates sequentially. The pipeline short-circuits by default: if compile fails, do not waste time running tests or UI checks.

Typical code task pipeline:

```text
compile -> lint -> test -> generated tests -> property/integration -> judge
```

Frontend task pipeline after this feature:

```text
compile -> lint -> test -> ui -> visual judge -> integration
```

The UI gate should run only after basic code gates pass. A browser journey is not useful if the app cannot build or start.

### 3.6 Learning Subsystems

Roko learns through:

- gate pass/fail metrics
- model routing outcomes
- prompt experiment outcomes
- cost and latency tracking
- adaptive thresholds
- episode logs
- knowledge distillation
- offline "dream" cycles that cluster episodes and promote playbooks

The Visual Gate should write learning data at every attempt so Roko learns what kinds of agents, prompts, models, and fixes produce good UI outcomes.

## 4. Existing Repo Shape To Expect

An implementation agent working in the Roko repo should expect roughly these modules:

| Area | Likely path | Role |
|---|---|---|
| Core traits and signal types | `crates/roko-core` | `Engram`, `Gate`, `Verdict`, `Context`, config schema |
| Gate implementations | `crates/roko-gate` | compile/test/shell/judge gates, pipeline, thresholds, artifact store |
| CLI and orchestration | `crates/roko-cli` | task parsing, plan execution, retries, dashboard, commands |
| Task parser | `crates/roko-cli/src/task_parser.rs` | parses `tasks.toml` into task structs |
| Plan orchestrator | `crates/roko-cli/src/orchestrate.rs` | dispatches agents and gates, handles retries |
| Existing vision prototype | `crates/roko-cli/src/vision_loop` | screenshot -> vision model -> code rewrite loop |
| HTTP control plane | `crates/roko-serve` | API routes, events, server state |
| Learning | `crates/roko-learn` | episodes, efficiency events, routing, experiments |
| Dashboard/TUI | `crates/roko-cli/src/tui` | gate summaries, logs, plans, agents, learning |

The existing `vision_loop` is useful context but should not be treated as the final design. It currently focuses on iterating a single file based on a screenshot and a vision model. `UiGate` is broader and must be a normal verification gate in the task pipeline.

## 5. Problem Statement

Roko agents need to do frontend work with real acceptance criteria. The current gate stack can verify code correctness but not rendered UI quality.

Current gates can answer:

- Does TypeScript/Rust/Go/etc. compile?
- Do unit tests pass?
- Do integration commands pass?
- Does lint pass?
- Does an LLM judge see obvious code/diff problems?

They cannot reliably answer:

- Does the page load in a browser?
- Does hydration succeed?
- Can a user complete the target workflow?
- Are buttons, forms, tabs, menus, modals, and links actually usable?
- Did the browser emit console errors?
- Did app API requests fail?
- Does the result work on mobile and desktop?
- Is there horizontal overflow?
- Is important text clipped, overlapping, hidden, or unreadable?
- Does the UI show required loading, empty, error, and success states?
- Does the screen visually match the product intent and design system?
- Did a retry regress a previously good visual state?

The gap is not only "take a screenshot." The gap is "turn browser-observed reality into a gate verdict and a repair loop."

## 6. Goals

1. Add first-class UI verification to Roko's gate pipeline.
2. Let tasks describe browser journeys, assertions, viewports, visual goals, and artifact policy.
3. Use real browser automation to operate the rendered UI.
4. Capture structured browser evidence for debugging and learning.
5. Run deterministic checks before subjective visual scoring.
6. Run a separate vision-capable evaluator for visual quality.
7. Produce normal `Verdict` values and normal gate signals.
8. Feed failed UI evidence into retry prompts.
9. Persist artifacts under `.roko/ui-runs`.
10. Emit learning events that improve future model routing, prompts, thresholds, policies, and playbooks.

## 7. Non-Goals

1. Do not replace application-owned Playwright/Cypress test suites.
2. Do not build a full visual regression SaaS.
3. Do not rely only on pixel diffs.
4. Do not let the implementer agent self-approve its own UI.
5. Do not require UI gates for non-UI tasks.
6. Do not build a browser engine.
7. Do not make Obscura or another lightweight browser the authoritative visual backend in the initial version.

## 8. Proposed Feature

Add `UiGate`, a gate that:

1. Reads UI verification requirements from a task or config.
2. Builds a browser run spec.
3. Starts or connects to a dev server.
4. Calls a Playwright runner.
5. Receives a structured browser run result.
6. Optionally runs a visual evaluator over screenshots and evidence.
7. Applies pass/fail rules.
8. Persists artifacts.
9. Returns a `Verdict`.
10. Produces retry feedback.

The gate should be deterministic where possible and visual where necessary.

Hard evidence examples:

- locator not found
- click action timed out
- expected text absent
- `console.error`
- page exception
- failed API request
- horizontal overflow
- critical accessibility violation

Visual evidence examples:

- hierarchy is unclear
- modal feels cramped
- spacing is inconsistent
- primary action is too low on mobile
- color contrast appears poor
- screen does not match requested design quality

Hard failures should block even if the screenshot looks good.

## 9. User-Facing Example

The task file should support UI requirements directly.

```toml
[[task]]
id = "F3"
title = "Build project creation modal"
description = "Implement the modal flow for creating a project from the dashboard."
tier = "integrative"
files = [
  "src/app/dashboard/page.tsx",
  "src/components/project-modal.tsx",
]
acceptance = [
  "User can open the modal from the dashboard",
  "User can enter a project name and create it",
  "Created project appears in the dashboard list",
  "The flow works on desktop and mobile",
]

[task.ui]
url = "http://localhost:5173/dashboard"
dev_server = "npm run dev -- --host 127.0.0.1"
target_score = 8.5
max_attempts = 3
hard_fail_on_console_error = true
hard_fail_on_failed_request = true
artifact_retention = "full"
visual_goal = """
The dashboard should feel like a polished production SaaS interface.
The create-project modal should be balanced, keyboard usable, clear on mobile,
and should show the new project after submission.
"""

[[task.ui.viewport]]
name = "desktop"
width = 1440
height = 900

[[task.ui.viewport]]
name = "mobile"
width = 390
height = 844
is_mobile = true
has_touch = true

[[task.ui.journey]]
id = "create-project"
name = "Create a project from dashboard"
start_url = "http://localhost:5173/dashboard"

[[task.ui.journey.step]]
action = "click"
role = "button"
name = "New project"

[[task.ui.journey.step]]
action = "fill"
label = "Project name"
value = "Demo Project"

[[task.ui.journey.step]]
action = "click"
role = "button"
name = "Create"

[[task.ui.journey.assert]]
type = "text_visible"
text = "Demo Project"

[[task.ui.journey.assert]]
type = "no_console_errors"

[[task.ui.journey.assert]]
type = "no_failed_requests"

[[task.ui.journey.assert]]
type = "no_horizontal_overflow"
```

## 10. Expected Execution Flow

```text
1. Planner selects task F3.
2. Composer builds implementation prompt.
3. Implementer agent edits files.
4. Compile/lint/test gates run.
5. If those pass, UiGate runs.
6. UiGate starts dev server if needed.
7. Browser runner executes configured journeys for each viewport.
8. Runner saves screenshots, traces, logs, requests, DOM, a11y, layout metrics.
9. UiGate evaluates hard assertions.
10. Visual evaluator scores screenshots and produces findings.
11. UiGate returns Verdict.
12. If passed, task completes.
13. If failed, orchestrator builds retry prompt from UI feedback.
14. Agent retries with exact evidence.
15. Roko records all attempts for learning.
```

## 11. Design Principle: Deterministic First, Visual Second

The gate must not use a vision model as the first or only judge.

Order:

1. Infrastructure checks:
   - app is reachable
   - dev server started
   - browser launched
   - journey did not time out

2. Functional checks:
   - user actions work
   - required state appears
   - expected URL or content appears
   - network requests succeed

3. Runtime checks:
   - no console errors
   - no page exceptions
   - no hydration errors
   - no failed app requests

4. Layout/accessibility checks:
   - no horizontal overflow
   - no obvious clipped text
   - no obvious text overlap
   - critical a11y constraints pass

5. Visual scoring:
   - visual quality
   - responsive fit
   - design-system fit
   - polish
   - clarity

If hard checks fail, the gate fails regardless of visual score.

## 12. Browser Backend Strategy

### 12.1 Authoritative Backend

Use Playwright with real Chromium as the authoritative backend.

Why:

- real rendering
- screenshots
- traces
- network events
- console events
- page errors
- accessibility snapshots
- viewport emulation
- robust locators
- widely understood test semantics

### 12.2 Optional Backend Interface

Define backend abstraction so future implementations can use alternative engines.

Conceptual shape:

```rust
#[async_trait::async_trait]
pub trait BrowserBackend: Send + Sync {
    async fn run(&self, spec: BrowserRunSpec) -> anyhow::Result<BrowserRunResult>;
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> BrowserCapabilities;
}
```

Possible backends:

- `playwright-chromium`: default and authoritative.
- `playwright-webkit`: optional compatibility backend.
- `playwright-firefox`: optional compatibility backend.
- `external-cdp`: connect to a remote browser provider.
- `obscura-cdp`: optional fast preflight backend only.

### 12.3 Obscura Position

Obscura is a lightweight Rust headless browser with a Chrome DevTools Protocol style interface. It may be useful for:

- fast DOM checks
- cheap scraping
- selector existence
- basic navigation
- network smoke checks
- preflight before running full Chromium

It should not be authoritative at first. A visual gate needs high-fidelity screenshots, layout, CSS, text rendering, accessibility, and real browser compatibility. If a backend cannot produce evidence equivalent to Chromium, it should only be used as a preflight optimization.

Policy:

```text
if task has visual_goal or screenshot assertions:
    authoritative_backend = playwright-chromium

if preflight_backend is configured:
    run it first
    if it hard-fails:
        fail fast or mark as preflight failure depending on config
    if it passes:
        still run authoritative backend before accepting task
```

## 13. Data Model: Task UI Spec

Add an optional `ui` field to task definitions.

Conceptual Rust:

```rust
pub struct TaskDef {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub tier: String,
    pub files: Vec<String>,
    pub verify: Vec<VerifyStep>,
    pub acceptance: Vec<String>,
    pub ui: Option<UiTaskSpec>,
}
```

### 13.1 UiTaskSpec

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiTaskSpec {
    pub url: Option<String>,
    pub dev_server: Option<String>,
    pub cwd: Option<PathBuf>,

    #[serde(default)]
    pub env: BTreeMap<String, String>,

    #[serde(default = "default_ui_target_score")]
    pub target_score: f64,

    #[serde(default = "default_ui_max_attempts")]
    pub max_attempts: u32,

    #[serde(default)]
    pub timeout_ms: Option<u64>,

    #[serde(default)]
    pub hard_fail_on_console_error: bool,

    #[serde(default)]
    pub hard_fail_on_failed_request: bool,

    #[serde(default)]
    pub require_accessibility_snapshot: bool,

    #[serde(default)]
    pub visual_goal: Option<String>,

    #[serde(default)]
    pub viewports: Vec<UiViewport>,

    #[serde(default)]
    pub journeys: Vec<UiJourney>,

    #[serde(default)]
    pub artifact_retention: UiArtifactRetention,

    #[serde(default)]
    pub backend: Option<String>,

    #[serde(default)]
    pub preflight_backend: Option<String>,
}
```

### 13.2 Viewport

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiViewport {
    pub name: String,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub device_scale_factor: Option<f64>,
    #[serde(default)]
    pub is_mobile: bool,
    #[serde(default)]
    pub has_touch: bool,
}
```

Default viewports if task does not specify:

```text
desktop: 1440x900
mobile: 390x844
```

### 13.3 Journey

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiJourney {
    pub id: String,
    pub name: String,
    pub start_url: Option<String>,
    #[serde(default)]
    pub auth_state: Option<PathBuf>,
    #[serde(default)]
    pub steps: Vec<UiStep>,
    #[serde(default)]
    pub asserts: Vec<UiAssertion>,
    #[serde(default)]
    pub screenshot: UiScreenshotPolicy,
}
```

### 13.4 Steps

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum UiStep {
    Goto {
        url: String,
    },
    Click {
        role: Option<String>,
        name: Option<String>,
        text: Option<String>,
        selector: Option<String>,
        test_id: Option<String>,
    },
    Fill {
        label: Option<String>,
        selector: Option<String>,
        test_id: Option<String>,
        value: String,
    },
    Press {
        key: String,
    },
    Select {
        label: Option<String>,
        selector: Option<String>,
        value: String,
    },
    Hover {
        selector: Option<String>,
        text: Option<String>,
    },
    WaitForText {
        text: String,
        timeout_ms: Option<u64>,
    },
    WaitForSelector {
        selector: String,
        timeout_ms: Option<u64>,
    },
    WaitForUrl {
        pattern: String,
        timeout_ms: Option<u64>,
    },
    Evaluate {
        script: String,
    },
    Screenshot {
        name: Option<String>,
        full_page: Option<bool>,
    },
}
```

Selector resolution order for user-like actions:

1. role + accessible name
2. label
3. test id
4. visible text
5. CSS selector

Prefer role, label, and test id. CSS selectors are allowed but less desirable because they are brittle and less user-centered.

### 13.5 Assertions

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiAssertion {
    TextVisible { text: String },
    TextNotVisible { text: String },
    UrlMatches { pattern: String },
    LocatorVisible { selector: String },
    RoleVisible { role: String, name: String },
    NoConsoleErrors,
    NoPageErrors,
    NoFailedRequests { allow: Option<Vec<String>> },
    NoHorizontalOverflow,
    NoOverlappingText,
    NoClippedText,
    MinContrast { ratio: f64 },
    AccessibilityViolationsBelow { max_critical: u32, max_serious: u32 },
    VisualScoreAtLeast { score: f64 },
    CustomEvaluate { name: String, script: String, expect_json: serde_json::Value },
}
```

Validation:

- `[task.ui]` requires at least one journey.
- Each journey requires at least one step or a start URL plus assertions.
- `target_score` must be in `0.0..=10.0`.
- Viewport dimensions must be positive and bounded.
- `Evaluate` steps require explicit config opt-in.
- External URLs require explicit config opt-in.

## 14. Global Config

Add global UI gate config under gate settings.

```toml
[gates.ui]
enabled = true
default_backend = "playwright-chromium"
preflight_backend = ""
target_score = 8.5
max_attempts = 3
timeout_ms = 120000
fail_on_console_error = true
fail_on_page_error = true
fail_on_failed_request = true
save_trace = true
save_har = true
save_video = false
vision_model = "claude-opus-4-6"
artifact_retention = "debug"

[[gates.ui.viewport]]
name = "desktop"
width = 1440
height = 900

[[gates.ui.viewport]]
name = "mobile"
width = 390
height = 844
is_mobile = true
has_touch = true

[gates.ui.security]
allow_external_urls = false
allow_evaluate_steps = false
redact_headers = ["authorization", "cookie", "x-api-key"]
redact_text_patterns = ["sk-[A-Za-z0-9_-]+", "Bearer [A-Za-z0-9._-]+"]
```

Task-specific settings override global settings.

## 15. Browser Runner Contract

The browser runner should be a small tool that accepts JSON and writes JSON plus artifacts. The initial implementation can be a Node script using Playwright, for example:

```text
tools/roko-ui-runner.mjs
```

Command:

```bash
node tools/roko-ui-runner.mjs --spec .roko/ui-runs/<plan>/<task>/<attempt>/spec.json
```

The runner should not call the LLM. It only runs browser automation and writes evidence.

### 15.1 BrowserRunSpec

`spec.json`:

```json
{
  "schema_version": 1,
  "run_id": "uuid",
  "plan_id": "frontend-plan",
  "task_id": "F3",
  "attempt": 2,
  "backend": "playwright-chromium",
  "base_url": "http://localhost:5173",
  "output_dir": ".roko/ui-runs/frontend-plan/F3/002",
  "timeout_ms": 120000,
  "save_trace": true,
  "save_har": true,
  "save_video": false,
  "viewports": [
    { "name": "desktop", "width": 1440, "height": 900 },
    { "name": "mobile", "width": 390, "height": 844, "is_mobile": true, "has_touch": true }
  ],
  "journeys": [
    {
      "id": "create-project",
      "name": "Create a project",
      "start_url": "http://localhost:5173/dashboard",
      "steps": [
        { "action": "click", "role": "button", "name": "New project" },
        { "action": "fill", "label": "Project name", "value": "Demo Project" },
        { "action": "click", "role": "button", "name": "Create" }
      ],
      "assertions": [
        { "type": "text_visible", "text": "Demo Project" },
        { "type": "no_console_errors" },
        { "type": "no_failed_requests" },
        { "type": "no_horizontal_overflow" }
      ]
    }
  ]
}
```

### 15.2 BrowserRunResult

`result.json`:

```json
{
  "schema_version": 1,
  "run_id": "uuid",
  "plan_id": "frontend-plan",
  "task_id": "F3",
  "attempt": 2,
  "backend": "playwright-chromium",
  "started_at": "2026-04-25T11:32:00Z",
  "duration_ms": 18423,
  "passed": false,
  "summary": "1 failed request and mobile layout overflow",
  "failure_classes": ["failed_request", "layout_overflow"],
  "viewports": [
    {
      "name": "mobile",
      "width": 390,
      "height": 844,
      "journeys": [
        {
          "id": "create-project",
          "passed": false,
          "final_url": "http://localhost:5173/dashboard",
          "screenshots": [
            "mobile/create-project/final.png"
          ],
          "assertions": [
            {
              "name": "text_visible:Demo Project",
              "passed": true,
              "severity": "hard"
            },
            {
              "name": "no_horizontal_overflow",
              "passed": false,
              "severity": "hard",
              "detail": "document width 431 exceeds viewport width 390"
            }
          ],
          "console": [],
          "page_errors": [],
          "requests": [
            {
              "url": "http://localhost:5173/api/projects",
              "method": "POST",
              "status": 500,
              "failed": true
            }
          ],
          "layout": {
            "horizontal_overflow": true,
            "document_width": 431,
            "viewport_width": 390,
            "overlapping_text_candidates": [],
            "clipped_text_candidates": []
          },
          "accessibility": {
            "snapshot_path": "mobile/create-project/a11y.json",
            "violations_path": "mobile/create-project/axe.json",
            "critical": 0,
            "serious": 0
          }
        }
      ]
    }
  ],
  "artifacts": {
    "trace": "trace.zip",
    "har": "network.har",
    "screenshots": [
      "mobile/create-project/final.png"
    ]
  }
}
```

The runner result should represent only observed browser facts and deterministic assertion results. Visual model findings are written separately.

## 16. Playwright Runner Implementation Details

The runner should:

1. Parse `--spec`.
2. Create output directory.
3. Launch Chromium with requested options.
4. For each viewport:
   - create browser context
   - set viewport and mobile/touch options
   - enable tracing if configured
   - attach console, pageerror, request, response, requestfailed listeners
   - run each journey
   - capture before/final screenshots
   - collect DOM/text/layout/a11y artifacts
   - evaluate assertions
5. Stop trace/HAR.
6. Write `result.json`.
7. Exit `0` if runner infrastructure succeeded, even if UI assertions failed.
8. Exit nonzero only for runner infrastructure errors such as bad spec, browser launch failure, or filesystem failure.

Reason for exit policy: UI failure is a gate verdict, not a crashed tool.

### 16.1 Event Capture

Capture console:

```javascript
page.on("console", msg => {
  consoleEvents.push({
    type: msg.type(),
    text: msg.text(),
    location: msg.location()
  });
});
```

Capture page errors:

```javascript
page.on("pageerror", error => {
  pageErrors.push({
    message: error.message,
    stack: error.stack || ""
  });
});
```

Capture network:

```javascript
page.on("request", request => { ... });
page.on("response", response => { ... });
page.on("requestfailed", request => { ... });
```

### 16.2 Locator Resolution

Use Playwright locators:

```javascript
function resolveLocator(page, target) {
  if (target.role && target.name) {
    return page.getByRole(target.role, { name: target.name });
  }
  if (target.label) {
    return page.getByLabel(target.label);
  }
  if (target.test_id) {
    return page.getByTestId(target.test_id);
  }
  if (target.text) {
    return page.getByText(target.text);
  }
  if (target.selector) {
    return page.locator(target.selector);
  }
  throw new Error("step has no locator target");
}
```

### 16.3 Layout Collector

Use browser JS to collect layout facts:

```javascript
const layout = await page.evaluate(() => {
  const doc = document.documentElement;
  const body = document.body;
  const viewportWidth = window.innerWidth;
  const documentWidth = Math.max(
    doc.scrollWidth || 0,
    body ? body.scrollWidth || 0 : 0
  );

  const clipped = [];
  for (const el of Array.from(document.querySelectorAll("body *"))) {
    const style = window.getComputedStyle(el);
    if (style.visibility === "hidden" || style.display === "none") continue;
    const text = (el.textContent || "").trim();
    if (!text) continue;
    if (el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1) {
      const rect = el.getBoundingClientRect();
      clipped.push({
        text: text.slice(0, 120),
        rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
        scrollWidth: el.scrollWidth,
        clientWidth: el.clientWidth,
        scrollHeight: el.scrollHeight,
        clientHeight: el.clientHeight
      });
    }
  }

  return {
    viewport_width: viewportWidth,
    viewport_height: window.innerHeight,
    document_width: documentWidth,
    document_height: Math.max(doc.scrollHeight || 0, body ? body.scrollHeight || 0 : 0),
    horizontal_overflow: documentWidth > viewportWidth + 1,
    clipped_text_candidates: clipped.slice(0, 20)
  };
});
```

Overlap detection should be heuristic and advisory at first, because text node rectangles can be noisy.

### 16.4 Accessibility

Initial:

- Use Playwright `page.accessibility.snapshot()` if available.
- Optionally inject axe-core when configured.

Do not make axe-core a hard dependency for the first minimal runner unless the repo already has it. Design the result schema to accept axe results later.

## 17. Visual Evaluator

The visual evaluator is separate from the browser runner. It receives screenshots and evidence, then returns a strict JSON evaluation. It should not edit code in `UiGate` mode.

The existing Roko vision prototype can be refactored into two modes:

1. `evaluate_and_rewrite`: existing manual vision loop behavior.
2. `evaluate_only`: new gate behavior.

If provider dependencies make it awkward for `roko-gate` to call models directly, let `roko-cli` orchestrator run the visual evaluator after `UiGate` browser evidence is collected. The gate architecture should still record the final result as a gate verdict.

### 17.1 Visual Eval Input

Include:

- task id and title
- task description
- acceptance criteria
- UI visual goal
- viewport names and dimensions
- screenshot paths or data URIs
- hard assertion results
- console/page/network summary
- prior attempt history if any
- local design context if available

### 17.2 Visual Eval Output

`vision-eval.json`:

```json
{
  "schema_version": 1,
  "model": "claude-opus-4-6",
  "score": 7.4,
  "passed": false,
  "threshold": 8.5,
  "confidence": 0.82,
  "rubric_scores": {
    "task_completion": 0.85,
    "layout_integrity": 0.62,
    "responsive_quality": 0.55,
    "interaction_clarity": 0.76,
    "visual_polish": 0.76,
    "design_system_fit": 0.80,
    "accessibility_affordance": 0.70
  },
  "findings": [
    {
      "severity": "high",
      "viewport": "mobile",
      "journey_id": "create-project",
      "screenshot": "mobile/create-project/final.png",
      "area": "modal footer",
      "problem": "The primary action is partly below the viewport.",
      "evidence": "The bottom of the Create button is clipped in the mobile screenshot.",
      "suggested_fix": "Constrain modal height, add internal scrolling, and keep actions visible."
    }
  ]
}
```

### 17.3 Rubric

Use a fixed rubric:

| Dimension | Weight | Meaning |
|---|---:|---|
| Task completion | 0.25 | Required workflow state is visibly achieved. |
| Layout integrity | 0.20 | No obvious overlap, clipping, broken spacing, bad alignment. |
| Responsive quality | 0.15 | Works across configured viewports. |
| Interaction clarity | 0.10 | Controls, focus, loading, empty, error, and success states are clear. |
| Visual polish | 0.10 | Typography, spacing, hierarchy, balance. |
| Design-system fit | 0.10 | Fits existing app conventions. |
| Accessibility affordance | 0.10 | Contrast, tap targets, labels, keyboard-visible states. |

Final visual score is `0.0..10.0`.

## 18. Pass/Fail Semantics

`UiGate` passes if:

1. The app was reachable.
2. Every required journey completed.
3. Every hard assertion passed.
4. No disallowed console errors occurred.
5. No disallowed page errors occurred.
6. No disallowed failed requests occurred.
7. Required screenshots were captured.
8. Required accessibility thresholds passed.
9. Visual score met `target_score` if visual scoring is enabled.

`UiGate` fails if any hard check fails. A high visual score cannot override hard failures.

Suggested normalized verdict score:

```text
if infrastructure_failed:
    score = 0.0
else if hard_failures_exist:
    score = min(0.49, hard_assertion_pass_ratio * 0.49)
else if visual_eval_enabled:
    score = visual_score / 10.0
else:
    score = 1.0
```

## 19. Artifact Layout

Persist every attempt under `.roko/ui-runs`.

```text
.roko/ui-runs/
  <plan_id>/
    <task_id>/
      001/
        spec.json
        result.json
        vision-eval.json
        feedback.md
        trace.zip
        network.har
        console.json
        requests.json
        page-errors.json
        desktop/
          create-project/
            before.png
            final.png
            dom.html
            text.txt
            a11y.json
            axe.json
            layout.json
        mobile/
          create-project/
            final.png
            dom.html
            text.txt
            a11y.json
            axe.json
            layout.json
      002/
        ...
```

Retention modes:

| Mode | Keeps |
|---|---|
| `minimal` | `result.json`, `vision-eval.json`, final screenshots |
| `debug` | minimal plus console, requests, DOM, layout, a11y |
| `full` | debug plus trace, HAR, optional video |

Defaults:

- local pass: `debug`
- local fail: `full`
- CI pass: `minimal`
- CI fail: `full`

## 20. Retry Feedback

A failed UI gate must become concise, actionable retry feedback.

Example:

```text
Your previous attempt failed the UI gate.

Task:
F3 Build project creation modal

Hard failures:
1. mobile/create-project: horizontal overflow. document width 431, viewport width 390.
2. desktop/create-project: POST /api/projects returned 500.

Visual findings:
1. high: mobile modal footer. Create button is clipped below viewport.
   Screenshot: .roko/ui-runs/frontend-plan/F3/002/mobile/create-project/final.png
   Suggested fix: constrain modal height, add internal scrolling, keep actions visible.

Browser evidence:
- console errors: 0
- page errors: 0
- failed requests: 1
- trace: .roko/ui-runs/frontend-plan/F3/002/trace.zip

Required next attempt:
- Fix the failed request or mock/create the expected success path.
- Remove horizontal overflow at 390x844.
- Keep the modal action row visible on mobile.
- Do not regress desktop behavior.
```

Implementation notes:

- Reuse the existing gate-feedback pattern.
- Include screenshot paths always.
- Embed image data only when invoking a vision-capable repair agent.
- Prefer top 3 to 5 findings.
- Include previous best attempt when a retry regresses.
- Preserve hard failure ordering: functional before visual polish.

## 21. Implementation Responsibilities

### 21.1 `roko-cli/src/task_parser.rs`

Add:

- `ui: Option<UiTaskSpec>` to task model.
- Serde structs for UI spec.
- Validation for UI fields.
- Tests for TOML parsing and invalid UI specs.
- Include UI context in fix prompts.

### 21.2 `roko-gate`

Add:

- `ui_gate.rs`
- `UiGate`
- `UiGateConfig`
- `BrowserRunSpec`
- `BrowserRunResult`
- `UiFailureClass`
- `UiFeedback`
- `ui_feedback_for_agent`
- verdict conversion tests

`UiGate` may shell out to the runner instead of embedding Playwright in Rust.

### 21.3 `tools/roko-ui-runner.mjs`

Add:

- Playwright JSON runner.
- Browser evidence capture.
- Deterministic assertion engine.
- Artifact writer.
- Self-test mode.

### 21.4 `roko-cli/src/orchestrate.rs`

Wire:

- select `UiGate` for tasks with `ui` specs.
- run after compile/lint/test.
- persist artifacts.
- attach `UiGate` verdict to task tracker.
- feed UI feedback into retry prompt.
- emit dashboard and learning events.

### 21.5 `roko-cli/src/vision_loop`

Refactor:

- separate evaluate-only visual scoring from rewrite loop.
- strict JSON parsing for visual eval output.
- add tests for visual eval parsing.

### 21.6 `roko-serve`

Add routes later:

```text
POST /api/ui-gate/run
GET  /api/ui-gate/runs
GET  /api/ui-gate/runs/{run_id}
GET  /api/ui-gate/runs/{run_id}/artifacts/{path}
POST /api/ui-gate/runs/{run_id}/label
```

Add events:

```rust
ServerEvent::UiGateStarted { plan_id, task_id, run_id }
ServerEvent::UiGateArtifact { plan_id, task_id, run_id, kind, path }
ServerEvent::UiGateCompleted { plan_id, task_id, run_id, passed, score }
```

### 21.7 Dashboard

Show:

- UI gate pass/fail in gate summary.
- visual score.
- top failure class.
- artifact paths.
- retry score history.
- latest screenshot path.

Initial TUI does not need inline image rendering.

### 21.8 `roko-learn`

Add learning events for:

- UI gate attempt.
- visual score.
- failure classes.
- model used.
- prompt variant used.
- cost and latency.
- human labels.

## 22. Failure Taxonomy

Use stable classes so learning systems can aggregate failures.

```rust
pub enum UiFailureClass {
    AppUnavailable,
    DevServerFailed,
    BrowserLaunchFailed,
    NavigationFailed,
    LocatorNotFound,
    ActionTimeout,
    AssertionFailed,
    ConsoleError,
    PageError,
    FailedRequest,
    AuthRequired,
    HydrationError,
    LayoutOverflow,
    TextClipped,
    TextOverlap,
    A11yCritical,
    VisualScoreLow,
    VisualRegression,
    RunnerInfrastructure,
}
```

Use these classes in:

- result JSON
- verdict detail
- feedback
- metrics
- learning events
- dashboard failure summaries
- retry policy

## 23. Security and Safety

Browser automation can click buttons and send requests. Guardrails are required.

Default policy:

- allow localhost URLs
- block external URLs unless allowlisted
- redact secrets from logs, headers, HAR, DOM, and feedback
- do not persist cookies unless explicitly configured
- kill dev server process group after gate
- set strict timeouts
- require opt-in for arbitrary evaluate scripts
- do not upload screenshots to external model providers unless configured

Redaction:

```toml
[gates.ui.security]
redact_headers = ["authorization", "cookie", "x-api-key"]
redact_text_patterns = [
  "sk-[A-Za-z0-9_-]+",
  "Bearer [A-Za-z0-9._-]+"
]
```

## 24. Metrics

Runtime metrics:

```text
roko_ui_gate_runs_total{backend,passed}
roko_ui_gate_duration_ms{backend}
roko_ui_gate_visual_score{task_domain}
roko_ui_gate_failures_total{class}
roko_ui_gate_retries_to_pass
roko_ui_gate_artifact_bytes
roko_ui_gate_preflight_divergence_total{backend}
```

Learning metrics:

- first-attempt UI pass rate
- mean visual score by model
- score delta per retry
- false accept/reject rate from human labels
- prompt variant lift
- backend cost saved
- retry count by failure class

## 25. Cybernetic Feedback Loops

This feature should be explicitly cybernetic. Each loop has sensors, comparator, controller, actuator, memory, and reward.

### 25.1 Attempt-Level Repair Loop

Purpose: Make one task pass.

Sensor:

- browser result
- screenshots
- console/page/network evidence
- layout/a11y findings
- visual score
- failure classes

Comparator:

- hard pass rules
- target visual score
- previous best attempt

Controller:

- orchestrator retry policy
- UI feedback compressor
- model escalation policy

Actuator:

- implementer agent edits code

Memory:

- `.roko/ui-runs/<plan>/<task>/<attempt>`
- gate verdict signals
- task attempt tracker

Reward:

- pass/fail
- visual score delta
- hard failure reduction
- cost and latency to convergence

Policy:

```text
if infrastructure failure:
    retry runner only or fail with infrastructure class
elif hard UI failures:
    repair prompt prioritizes hard browser evidence
elif visual score below threshold:
    repair prompt prioritizes top visual findings
elif visual score regressed:
    restore or reference previous best attempt
else:
    pass
```

### 25.2 Adaptive Threshold Loop

Purpose: Tune strictness over time.

Sensor:

- UI pass/fail history
- visual scores
- human accept/reject labels
- later user reverts or bug reports

Comparator:

- false accept rate
- false reject rate
- average retries to pass
- human override rate

Controller:

- adaptive threshold policy

Actuator:

- adjust target scores
- reclassify warning vs hard finding types
- adjust when visual scoring is required

Memory:

```text
.roko/learn/ui-thresholds.json
```

Example:

```json
{
  "dashboard": {
    "target_score": 8.4,
    "score_ema": 8.1,
    "false_accept_rate": 0.04,
    "false_reject_rate": 0.11,
    "retry_mean": 1.8
  }
}
```

### 25.3 Model Routing Loop

Purpose: Pick the cheapest model that succeeds on UI tasks.

Sensor:

- implementer model
- evaluator model
- task tier
- UI complexity
- attempts to pass
- visual score
- hard failure classes
- cost
- latency

Comparator:

- first-attempt UI pass rate
- cost-adjusted success rate
- mean score improvement per retry

Controller:

- cascade router or contextual bandit

Actuator:

- route future UI tasks to better model tier
- route visual evaluation to cheaper/stronger model based on risk

Reward:

```text
reward = 1.0 * passed
       + 0.2 * first_attempt_pass
       + 0.1 * clamp(score_delta / 2.0)
       - 0.1 * retry_count
       - 0.05 * normalized_cost
       - 0.1 * hard_failure_count
```

### 25.4 Prompt Strategy Loop

Purpose: Learn which prompts produce better UI work.

Sensor:

- prompt template id
- design context included or not
- screenshots included or not
- playbooks included or not
- outcome metrics

Comparator:

- pass rate by variant
- visual score by variant
- cost by variant
- failure class distribution

Controller:

- prompt experiment system

Actuator:

- promote winning prompt variants
- demote variants that over-optimize visuals and break behavior

Variants to test:

- `ui-basic`: task plus files plus acceptance only
- `ui-design-context`: includes style and existing screen context
- `ui-mobile-first`: asks agent to implement mobile constraints first
- `ui-stateful`: explicitly requests loading/empty/error/success states
- `ui-a11y-first`: emphasizes labels, focus, keyboard, contrast

Promotion rule:

```text
promote if:
    n >= 20
    pass_rate_lift >= 8%
    cost_increase <= 15%
    human_override_rate does not worsen
```

### 25.5 Assertion Synthesis Loop

Purpose: Convert repeated visual failures into deterministic checks.

Sensor:

- repeated vision findings
- failure classes
- human-confirmed issues
- successful fix diffs

Comparator:

- frequency
- deterministic detectability
- false positive rate

Controller:

- assertion synthesizer

Actuator:

- add generated assertions to future tasks
- add project UI policy

Example:

```text
Repeated finding:
  "mobile horizontal overflow" in 11 tasks

Synthesized assertion:
  { type = "no_horizontal_overflow" }

Policy:
  Enable for all tasks with mobile viewport.
```

### 25.6 Visual Memory Loop

Purpose: Learn what this product should look like.

Sensor:

- approved screenshots
- rejected screenshots
- component snapshots
- visual summaries
- human labels

Comparator:

- current screenshot similarity to approved examples
- deviations from local style
- recurring design feedback

Controller:

- visual memory retriever
- design context composer

Actuator:

- include relevant approved screenshots or style summaries in future prompts
- flag deviations from local design language

Memory:

```text
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

Initial implementation may store text summaries only. Later implementation can add image embeddings.

### 25.7 Browser Backend Selection Loop

Purpose: Decide when cheap preflight is worth running.

Sensor:

- preflight backend result
- Chromium result
- divergence
- runtime
- memory

Comparator:

- false pass rate
- false fail rate
- time saved

Controller:

- backend bandit

Actuator:

- enable/disable preflight by task class
- choose backend order

Policy:

```text
if task has visual_goal:
    chromium required
if task has only DOM/network assertions:
    preflight may run first
if preflight diverges from chromium too often:
    reduce backend weight
```

### 25.8 Human Calibration Loop

Purpose: Align automated judgment with operator taste.

Sensor:

- human accepts run
- human rejects run
- human labels finding useful/not useful
- human edits after automated pass

Comparator:

- automated pass vs human accept
- automated score vs human rating
- finding usefulness

Controller:

- calibration policy

Actuator:

- adjust rubric weights
- adjust thresholds
- improve evaluator prompt
- add project style rules

CLI:

```bash
roko ui-gate label <run_id> --accept
roko ui-gate label <run_id> --reject "mobile button clipped"
roko ui-gate label-finding <finding_id> --useful
```

### 25.9 Dream-Cycle Distillation Loop

Purpose: Convert many UI episodes into playbooks.

Sensor:

- completed UI episodes
- failure clusters
- fix diffs
- prompt variants
- model choices

Comparator:

- which repair tactics work repeatedly
- which failures recur
- which playbooks improve outcomes

Controller:

- offline dream cycle

Actuator:

- promote playbooks into durable knowledge

Example playbooks:

- "For modal clipping on mobile, use max-height, internal scroll, and a visible action row."
- "For dashboard cards, avoid fixed pixel grids below 640px."
- "For failed POST in UI gate, inspect mock/server route before restyling."

## 26. Test Strategy

### 26.1 Unit Tests

- parse `[task.ui]`
- validate missing journeys
- validate score bounds
- convert `BrowserRunResult` to `Verdict`
- classify UI failure classes
- compress UI feedback
- redact secrets
- merge hard assertions and visual eval

### 26.2 Runner Tests

Create a small fixture app:

```text
tests/fixtures/ui-app/
  package.json
  index.html
  src/
```

Scenarios:

- passing click flow
- missing locator
- console error
- page error
- failed request
- horizontal overflow
- clipped mobile modal
- form submission

### 26.3 Integration Tests

- `UiGate` calls runner and returns pass.
- `UiGate` calls runner and returns fail with artifact paths.
- orchestrator includes UI feedback in retry prompt.
- gate verdict signal is persisted.
- dashboard receives UI gate event.

### 26.4 Golden Tests

Golden files:

- `spec.json`
- `result.json`
- `vision-eval.json`
- `feedback.md`

## 27. MVP Scope

The smallest useful slice:

1. Add UI task spec parsing.
2. Add Playwright JSON runner.
3. Add `UiGate` that shells to runner.
4. Save `result.json` and screenshots.
5. Convert result to `Verdict`.
6. Add retry feedback.
7. Wire `UiGate` into orchestrator for tasks with `[task.ui]`.

Visual evaluator can be second slice if necessary, but the schema should be designed from the start.

## 28. Suggested Task Breakdown

```toml
[[task]]
id = "VG-01"
title = "Add UI task spec parsing"
files = ["crates/roko-cli/src/task_parser.rs"]
acceptance = ["tasks.toml supports [task.ui], viewports, journeys, steps, assertions"]
verify = [{ phase = "test", command = "cargo test -p roko-cli task_parser" }]

[[task]]
id = "VG-02"
title = "Create Playwright JSON runner"
files = ["tools/roko-ui-runner.mjs"]
acceptance = ["runner accepts spec.json and writes result.json with screenshots and logs"]
verify = [{ phase = "node", command = "node tools/roko-ui-runner.mjs --self-test" }]

[[task]]
id = "VG-03"
title = "Implement UiGate"
files = ["crates/roko-gate/src/ui_gate.rs", "crates/roko-gate/src/lib.rs"]
acceptance = ["UiGate shells to runner and returns pass/fail Verdict"]
verify = [{ phase = "test", command = "cargo test -p roko-gate ui_gate" }]

[[task]]
id = "VG-04"
title = "Wire UiGate into orchestrator"
files = ["crates/roko-cli/src/orchestrate.rs"]
acceptance = ["tasks with ui specs run UiGate after existing code gates"]
verify = [{ phase = "test", command = "cargo test -p roko-cli orchestrate" }]

[[task]]
id = "VG-05"
title = "Add UI gate feedback compressor"
files = ["crates/roko-gate/src/ui_gate.rs", "crates/roko-cli/src/orchestrate.rs"]
acceptance = ["failed UI gate produces concise retry prompt with artifacts and findings"]
verify = [{ phase = "test", command = "cargo test -p roko-gate ui_feedback" }]

[[task]]
id = "VG-06"
title = "Add evaluate-only visual scorer"
files = ["crates/roko-cli/src/vision_loop/evaluator.rs", "crates/roko-cli/src/vision_loop/mod.rs"]
acceptance = ["visual evaluator returns score/findings JSON without rewriting code"]
verify = [{ phase = "test", command = "cargo test -p roko-cli vision_loop" }]

[[task]]
id = "VG-07"
title = "Persist and surface UI artifacts"
files = ["crates/roko-cli/src/orchestrate.rs", "crates/roko-serve/src/events.rs", "crates/roko-cli/src/tui/views/dashboard_view.rs"]
acceptance = ["dashboard and gate history show UI gate status and artifact paths"]
verify = [{ phase = "test", command = "cargo test -p roko-cli dashboard_view" }]

[[task]]
id = "VG-08"
title = "Record UI learning events"
files = ["crates/roko-learn/src", "crates/roko-core/src/dashboard_snapshot.rs"]
acceptance = ["UI outcomes feed routing and prompt experiment metrics"]
verify = [{ phase = "test", command = "cargo test -p roko-learn" }]
```

## 29. Acceptance Criteria

MVP is complete when:

1. A task can declare `[task.ui]`.
2. Roko can parse UI journeys, viewports, steps, and assertions.
3. A Playwright runner can execute the journey.
4. The runner captures screenshots, console events, page errors, requests, and layout data.
5. `UiGate` returns a normal `Verdict`.
6. A failed UI gate includes actionable feedback and artifact paths.
7. Roko retries a failed frontend task using UI feedback.
8. UI artifacts persist under `.roko/ui-runs`.
9. Gate history and dashboard show UI gate outcomes.
10. Hard browser failures override visual score.

Post-MVP is complete when:

1. Visual evaluator runs in evaluate-only mode.
2. Human labels calibrate thresholds.
3. Prompt and model routing learn from UI outcomes.
4. Repeated findings become playbooks or generated assertions.
5. Optional preflight backends can run and track divergence from Chromium.

## 30. Rollout Plan

Stage 1: Manual command only.

```bash
roko ui-gate run --spec .roko/ui-runs/example/spec.json
```

Stage 2: Opt-in per task.

```text
If [task.ui] exists, run UiGate.
```

Stage 3: Project-level enable.

```toml
[gates.ui]
enabled = true
```

Stage 4: Visual evaluator enabled.

Stage 5: Learning events enabled.

Stage 6: Human calibration and visual memory enabled.

Stage 7: Optional preflight backend experimentation.

## 31. Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Visual evaluator is subjective | Bad pass/fail | hard checks first, fixed rubric, human calibration |
| Browser runner flakes | Wasted retries | traces, timeouts, infrastructure failure class |
| Screenshots leak secrets | Security issue | redaction and explicit provider config |
| UI gate slows execution | Poor throughput | run only for UI tasks, use preflight, cache dev server |
| Agent optimizes for screenshots only | Behavior regressions | hard journeys and network checks before visual score |
| Generated journeys are weak | false confidence | require acceptance criteria and synthesize assertions from failures |
| Obscura diverges from Chromium | incorrect preflight | preflight only and track divergence |

## 32. Open Questions

1. Should `UiGate` itself call the vision evaluator, or should orchestrator combine browser result plus visual result into a final verdict?
2. Should the Playwright runner live in `tools/`, `crates/roko-cli/assets`, or a separate package?
3. How often should screenshots be embedded in prompts versus referenced by local path?
4. Should UI gates run before or after generated behavioral tests?
5. How should Roko infer journeys from natural language acceptance criteria?
6. What should be the default policy for external network calls?
7. Should visual memory store raw screenshots, summaries, embeddings, or all three?
8. How should Roko compare screenshots across runs without brittle pixel diffs?

## 33. Implementation Guidance For Another Agent

If you are implementing this from scratch in the Roko repo:

1. Start with the browser runner and JSON schemas. Do not start with the visual model.
2. Add parser structs and tests for `[task.ui]`.
3. Add `UiGate` as a normal gate. Keep failure as `Verdict`, not thrown error.
4. Wire the gate only for tasks that declare `ui`.
5. Persist artifacts before returning a verdict.
6. Build retry feedback from `result.json`.
7. Only then add visual scoring.
8. Only then add learning events.
9. Keep Obscura or alternate CDP backends behind a backend trait and do not use them for final acceptance initially.

The most important behavior is this:

```text
agent edits code
basic gates pass
browser gate observes rendered UI
browser/visual evidence fails
agent receives concrete evidence
agent retries
browser gate observes again
Roko learns from the outcome
```

## 34. Example End-to-End Episode

```text
Attempt 1:
  Agent builds modal.
  Compile passes.
  Tests pass.
  UiGate fails:
    - mobile horizontal overflow
    - Create button clipped
    - visual score 6.9

Attempt 2:
  Agent fixes modal layout.
  Compile passes.
  Tests pass.
  UiGate fails:
    - POST /api/projects returns 500
    - visual score 8.6

Attempt 3:
  Agent fixes API integration/mock.
  Compile passes.
  Tests pass.
  UiGate passes:
    - hard assertions pass
    - visual score 8.8

React:
  Persist winning artifacts.
  Reward implementer model for convergence.
  Record mobile modal clipping fix pattern.
  Update prompt strategy stats.
  Dashboard shows UI gate pass and score history.
```

## 35. Definition of Done

This feature is done when Roko can take a frontend task, implement it through an agent, run the rendered app in a real browser, press the relevant controls, capture functional and visual evidence, fail with actionable feedback when the UI is wrong, retry with that feedback, pass when the UI meets hard and visual criteria, and store enough data for future agents to improve their first-attempt frontend quality.
