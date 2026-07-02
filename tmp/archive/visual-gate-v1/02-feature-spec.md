# Part II: Visual gate feature specification

This document defines every type, contract, and behavior for the visual gate. Part I (`01-roko-architecture.md`) covers the full architecture — `Engram`, `Verdict`, the `Gate` trait, rungs, and the gate pipeline. This document references those types but does not redefine them. Every NEW type is defined inline here with full Rust struct definitions.

The audience is an implementation agent with zero access to the roko repository. If a type appears in a code block below, you implement it. If it is mentioned by name without a definition, it lives in Part I.

---

## 6. Problem statement

Roko's gate pipeline verifies code quality: compilation, linting, tests, symbol checks, benchmarks. These gates cannot verify that a UI works or looks right. A component can compile, pass every test, and still be unusable:

- A button clipped on mobile viewports.
- A modal overflowing its container.
- A route throwing a console error on load.
- A form submission returning HTTP 500.
- A hydration error that only appears at runtime.
- A layout that technically renders but looks nothing like what was requested.
- An interactive element that cannot receive focus or be reached by keyboard.

For agents doing frontend work, there is no feedback loop between "code was written" and "the UI is correct." The agent writes a React component, the compile gate passes, the test gate passes, and the task is marked done. Nobody opened a browser. Nobody checked whether the page loads without errors, whether the layout adapts to mobile, whether the user can complete the intended workflow.

The gap is not only "take a screenshot." The gap is "turn browser-observed reality into a gate verdict and a repair loop." Without visual verification, agents cannot self-correct on frontend tasks. They produce code that satisfies type checkers and test harnesses while the actual rendered output is broken.

---

## 7. Goals and non-goals

### Goals

1. Agents can verify their frontend work by rendering it in a real browser.
2. Browser evidence — screenshots, console logs, network requests, layout metrics, accessibility snapshots — is captured automatically on every run.
3. Functional correctness is checked deterministically: page loads, elements exist, interactions work, no errors.
4. Visual quality is judged by an LLM evaluator with a structured rubric.
5. Failures produce specific, actionable feedback that agents can use to retry.
6. The entire flow plugs into the existing gate pipeline as a new `Gate` implementation.
7. Artifacts (screenshots, evidence JSON, evaluation results, traces) are stored for learning and debugging.
8. All attempts become learning data for model routing, prompt experiments, threshold tuning, playbooks, and visual memory.

### Non-goals

- **Pixel-perfect screenshot comparison.** Fragile, not how humans judge UI quality.
- **Full E2E test framework replacement.** This is a gate, not a test runner. Application-owned Playwright/Cypress suites are separate.
- **Non-web UIs.** Mobile native and desktop native are out of scope. Web only for MVP.
- **Real user traffic or production monitoring.**
- **Cross-browser testing beyond Chromium.** Other browsers can be added later via the backend trait.
- **Building a browser engine.** We use Playwright with real Chromium.
- **Agent self-approval.** The implementer agent never judges its own UI. The evaluator is separate.
- **Visual regression SaaS.** This is an internal gate, not a product.

---

## 8. Design principle: deterministic first, visual second

The visual gate uses a five-tier verification order. Cheaper, deterministic checks run first. Expensive, subjective checks run last. A hard failure at any tier stops the pipeline.

```
Tier 1: Infrastructure  → Did the dev server start? Did the page load? (HTTP 200, no crash)
Tier 2: Functional      → Do required elements exist? Do interactions work? (DOM queries, clicks)
Tier 3: Runtime         → Any console errors? Failed network requests? Unhandled exceptions?
Tier 4: Layout          → Horizontal overflow? Clipped text? Overlapping text? Viewport issues?
Tier 5: Visual          → Does it look right? LLM evaluator scores screenshot against criteria.
```

Tiers 1–4 are deterministic. They produce binary pass/fail results from browser APIs. Tier 5 is subjective. It sends screenshots to a vision-capable LLM and receives a numeric score.

Hard failures in tiers 1–4 immediately fail the gate. There is no point scoring a screenshot if the page did not load or a required element is missing.

Tier 5 produces a numeric score compared against a configurable threshold. Below the threshold, the gate fails. Above it, the gate passes and the score is recorded for learning.

The gate must not use a vision model as the first or only judge.

---

## 9. User-facing example

A complete worked example showing the visual gate in action, from task definition through failure and retry to success.

### Task definition (tasks.toml)

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

[[task.ui.journey.step]]
action = "screenshot"
name = "after-create"

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

### Attempt 1: functional failure

The agent implements the modal. Code compiles. Tests pass. The UI gate runs.

The browser runner launches Chromium, navigates to the dashboard, clicks "New project," fills the form, clicks "Create." The form submission hits `POST /api/projects` which returns 500 (the agent forgot to mock the API endpoint). The `no_failed_requests` assertion fires.

The gate returns a hard failure at tier 3 (runtime). Visual evaluation does not run.

Retry feedback sent to the agent:

```markdown
## UI Gate Failure (attempt 1/3)

### Hard failures
- **Runtime**: POST http://localhost:5173/api/projects returned 500

### Evidence
- Screenshots: .roko/ui-runs/F3/001/desktop/create-project/after-create.png
- Console errors: 0
- Failed requests: 1 (POST /api/projects → 500)

### What to fix
The form submission POST to /api/projects returns HTTP 500. Either mock the API endpoint to return a success response, or wire the real backend handler. Do not proceed to visual polish until the API integration works.
```

### Attempt 2: layout failure

The agent fixes the API mock. The POST now returns 201. But on mobile (390×844), the modal overflows the viewport — the document is 431px wide. The `no_horizontal_overflow` assertion fires.

Retry feedback:

```markdown
## UI Gate Failure (attempt 2/3)

### Hard failures
- **Layout**: horizontal overflow on mobile viewport. Document width 431px > viewport width 390px.

### Soft findings
- **layout_correctness**: Modal footer "Create" button is partly below the fold on mobile.

### Evidence
- Screenshots: .roko/ui-runs/F3/002/mobile/create-project/after-create.png
- Console errors: 0
- Failed requests: 0
- Layout: horizontal overflow on mobile, 0 clipped text

### What to fix
1. Fix horizontal overflow on the 390×844 mobile viewport. The modal or its contents are wider than the viewport — check for fixed-width elements, unresponsive padding, or wide content.
2. Keep the modal action row (Create button) visible without scrolling on mobile.
```

### Attempt 3: pass

The agent fixes the modal layout. No overflow. All assertions pass. Visual evaluator runs and scores 8.4/10 (above threshold 7.0). Gate passes.

---

## 10. Data model: task UI spec

### TaskDef extension

The task definition struct gains an optional `ui` field:

```rust
/// Conceptual — the existing TaskDef in the parser gains this field.
pub struct TaskDef {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub tier: String,
    pub files: Vec<String>,
    pub verify: Vec<VerifyStep>,
    pub acceptance: Vec<String>,
    // ... existing fields ...

    /// Optional UI verification spec. When present, UiGate runs after code gates.
    pub ui: Option<UiTaskSpec>,
}
```

### UiTaskSpec

```rust
/// UI verification requirements for a task.
/// Parsed from [task.ui] in tasks.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiTaskSpec {
    /// Base URL of the app under test.
    /// Example: "http://localhost:5173/dashboard"
    pub url: Option<String>,

    /// Shell command to start the dev server.
    /// Example: "npm run dev -- --host 127.0.0.1"
    /// The gate starts this before the browser run and kills it after.
    pub dev_server: Option<String>,

    /// Working directory for the dev server command.
    /// Defaults to the workspace root.
    pub cwd: Option<PathBuf>,

    /// Extra environment variables for the dev server process.
    #[serde(default)]
    pub env: BTreeMap<String, String>,

    /// Target visual score on 0.0–10.0 scale.
    /// The gate fails if the visual evaluator scores below this.
    /// Overrides the global [gates.ui] threshold for this task.
    #[serde(default = "default_ui_target_score")]
    pub target_score: f64,

    /// Maximum retry attempts for this task.
    /// Overrides the global [gates.ui] max_retries for this task.
    #[serde(default = "default_ui_max_attempts")]
    pub max_attempts: u32,

    /// Timeout in milliseconds for a single browser run (all viewports).
    /// Overrides the global [gates.ui] timeout.
    #[serde(default)]
    pub timeout_ms: Option<u64>,

    /// If true, any console.error immediately fails the gate (tier 3 hard failure).
    /// If false, console errors are captured but only fail if a NoConsoleErrors
    /// assertion is present in a journey.
    #[serde(default)]
    pub hard_fail_on_console_error: bool,

    /// If true, any HTTP request returning 4xx/5xx immediately fails the gate.
    /// If false, failed requests are captured but only fail if a NoFailedRequests
    /// assertion is present.
    #[serde(default)]
    pub hard_fail_on_failed_request: bool,

    /// If true, capture and store Playwright accessibility snapshots.
    #[serde(default)]
    pub require_accessibility_snapshot: bool,

    /// Free-form description of what the UI should look like.
    /// Sent to the visual evaluator alongside screenshots.
    #[serde(default)]
    pub visual_goal: Option<String>,

    /// Viewports to test. If empty, uses defaults from [gates.ui] config.
    #[serde(default)]
    pub viewports: Vec<UiViewport>,

    /// UI journeys (user workflows) to execute.
    /// Each journey is an ordered sequence of steps and assertions.
    #[serde(default)]
    pub journeys: Vec<UiJourney>,

    /// Global assertions applied to every journey (in addition to per-journey asserts).
    #[serde(default)]
    pub assertions: Vec<UiAssertion>,

    /// Artifact retention mode. Controls how much evidence is kept on disk.
    #[serde(default)]
    pub artifact_retention: UiArtifactRetention,

    /// Browser backend override.
    /// Default: "playwright-chromium" (from global config).
    #[serde(default)]
    pub backend: Option<String>,

    /// Optional preflight backend (cheap, fast, advisory).
    /// Runs before the authoritative backend as an optimization.
    #[serde(default)]
    pub preflight_backend: Option<String>,

    /// Screenshot policy if not specified per-journey.
    #[serde(default)]
    pub screenshot: UiScreenshotPolicy,
}

fn default_ui_target_score() -> f64 { 7.0 }
fn default_ui_max_attempts() -> u32 { 3 }
```

### UiViewport

```rust
/// A viewport configuration for browser testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiViewport {
    /// Human-readable name (used in artifact paths and reports).
    /// Example: "desktop", "mobile", "tablet"
    pub name: String,

    /// Viewport width in CSS pixels.
    pub width: u32,

    /// Viewport height in CSS pixels.
    pub height: u32,

    /// Device pixel ratio. Default 1.0.
    #[serde(default)]
    pub device_scale_factor: Option<f64>,

    /// Emulate mobile device (affects touch events and meta viewport).
    #[serde(default)]
    pub is_mobile: bool,

    /// Emulate touch events.
    #[serde(default)]
    pub has_touch: bool,
}
```

Default viewports (used when task specifies none):

```toml
[[gates.ui.default_viewports]]
name = "desktop"
width = 1440
height = 900

[[gates.ui.default_viewports]]
name = "mobile"
width = 390
height = 844
is_mobile = true
has_touch = true
```

### UiJourney

A journey is an ordered user workflow: navigate, interact, assert, screenshot.

```rust
/// A user workflow to execute in the browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiJourney {
    /// Unique identifier within the task (used in artifact paths).
    pub id: String,

    /// Human-readable name (used in reports and feedback).
    pub name: String,

    /// URL to navigate to at the start of this journey.
    /// Overrides UiTaskSpec.url for this journey.
    pub start_url: Option<String>,

    /// Path to a Playwright storage state file (cookies, localStorage)
    /// for authenticated journeys.
    #[serde(default)]
    pub auth_state: Option<PathBuf>,

    /// Ordered interaction steps.
    #[serde(default)]
    pub steps: Vec<UiStep>,

    /// Assertions checked after all steps complete.
    /// These are in addition to global UiTaskSpec.assertions.
    #[serde(default)]
    pub asserts: Vec<UiAssertion>,

    /// Screenshot policy for this journey.
    #[serde(default)]
    pub screenshot: UiScreenshotPolicy,
}
```

### UiScreenshotPolicy

```rust
/// When to take screenshots during a journey.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiScreenshotPolicy {
    /// Take a screenshot after the last step and on failure.
    FinalOnly,
    /// Take a screenshot before the first step and after the last step.
    BeforeAndAfter,
    /// Take a screenshot after every step (expensive, good for debugging).
    EveryStep,
    /// Only take screenshots when explicitly requested via Screenshot steps.
    Manual,
}

impl Default for UiScreenshotPolicy {
    fn default() -> Self {
        Self::FinalOnly
    }
}
```

### UiStep

```rust
/// A single interaction step in a UI journey.
/// Steps execute in order. A failing step short-circuits the journey.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum UiStep {
    /// Navigate to a URL.
    Goto {
        url: String,
    },

    /// Click an element. Locator resolution: role+name → label → test_id → text → selector.
    Click {
        role: Option<String>,
        name: Option<String>,
        text: Option<String>,
        selector: Option<String>,
        test_id: Option<String>,
        label: Option<String>,
    },

    /// Fill a text input. Clears existing value first.
    Fill {
        label: Option<String>,
        selector: Option<String>,
        test_id: Option<String>,
        value: String,
    },

    /// Press a keyboard key (e.g., "Enter", "Tab", "Escape").
    Press {
        key: String,
    },

    /// Select an option from a <select> dropdown.
    Select {
        label: Option<String>,
        selector: Option<String>,
        value: String,
    },

    /// Hover over an element.
    Hover {
        selector: Option<String>,
        text: Option<String>,
        role: Option<String>,
        name: Option<String>,
    },

    /// Wait for text to appear anywhere on the page.
    WaitForText {
        text: String,
        timeout_ms: Option<u64>,
    },

    /// Wait for a CSS selector to be present in the DOM.
    WaitForSelector {
        selector: String,
        timeout_ms: Option<u64>,
    },

    /// Wait for the page URL to match a pattern.
    WaitForUrl {
        pattern: String,
        timeout_ms: Option<u64>,
    },

    /// Scroll to an element or coordinates.
    Scroll {
        /// CSS selector to scroll into view.
        selector: Option<String>,
        /// Absolute scroll coordinates (used if no selector).
        x: Option<f64>,
        y: Option<f64>,
    },

    /// Delay execution for a fixed number of milliseconds.
    /// Use sparingly — prefer WaitForSelector or WaitForText.
    Delay {
        ms: u64,
    },

    /// Take a screenshot at this point in the journey.
    Screenshot {
        /// Label used as the filename. Example: "after-create" → "after-create.png"
        name: Option<String>,
        /// If true, capture the full scrollable page. Default: false (viewport only).
        full_page: Option<bool>,
    },

    /// Run an assertion inline as a step (same variants as UiAssertion).
    Assert {
        assertion: UiAssertion,
    },

    /// Execute arbitrary JavaScript in the page context.
    /// Requires explicit opt-in via [gates.ui.security] allow_evaluate_steps = true.
    Evaluate {
        script: String,
        /// If set, deep-compare the script's return value to this JSON.
        expect_json: Option<serde_json::Value>,
    },
}
```

#### Locator resolution order

For user-like actions (Click, Fill, Hover), locators are resolved in this order. The first match wins:

1. **role + accessible name** — `page.getByRole(role, { name })` — most user-like, most stable.
2. **label** — `page.getByLabel(label)` — for form inputs.
3. **test_id** — `page.getByTestId(test_id)` — for data-testid attributes.
4. **visible text** — `page.getByText(text)` — matches visible text content.
5. **CSS selector** — `page.locator(selector)` — least desirable, most brittle.

Prefer role, label, and test_id. CSS selectors are allowed but brittle and less user-centered.

### UiAssertion

```rust
/// An assertion checked after a journey completes (or inline via Assert step).
/// Hard assertions immediately fail the gate. Soft assertions are recorded.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiAssertion {
    /// Text is visible on the page.
    TextVisible {
        text: String,
    },

    /// Text is NOT visible on the page.
    TextNotVisible {
        text: String,
    },

    /// An element matching the selector exists in the DOM.
    ElementExists {
        selector: String,
    },

    /// An element matching the selector is visible (not hidden, not zero-size).
    ElementVisible {
        selector: String,
    },

    /// Count of elements matching the selector equals expected.
    ElementCount {
        selector: String,
        expected: u32,
    },

    /// Element's textContent contains the given text.
    TextContains {
        selector: String,
        text: String,
    },

    /// Element's trimmed textContent equals the given text exactly.
    TextEquals {
        selector: String,
        text: String,
    },

    /// Element has a specific attribute (optionally with a specific value).
    HasAttribute {
        selector: String,
        attribute: String,
        value: Option<String>,
    },

    /// Element has a specific CSS class.
    HasClass {
        selector: String,
        class: String,
    },

    /// An element with the given ARIA role and accessible name is visible.
    RoleVisible {
        role: String,
        name: String,
    },

    /// The current page URL matches a regex pattern.
    UrlMatches {
        pattern: String,
    },

    /// The page title contains the given text.
    TitleContains {
        text: String,
    },

    /// No console.error messages were emitted during the journey.
    /// When present as an assertion, console errors become hard failures.
    NoConsoleErrors,

    /// No unhandled page exceptions were thrown.
    NoPageErrors,

    /// No HTTP requests returned 4xx/5xx or failed entirely.
    /// `allow` is an optional list of URL patterns to exclude from the check
    /// (e.g., analytics endpoints that are expected to 404 in dev).
    NoFailedRequests {
        #[serde(default)]
        allow: Option<Vec<String>>,
    },

    /// No horizontal overflow (document wider than viewport).
    NoHorizontalOverflow,

    /// No text is clipped by overflow:hidden containers.
    NoClippedText,

    /// No overlapping text elements (heuristic, advisory).
    NoOverlappingText,

    /// Minimum color contrast ratio (WCAG).
    MinContrast {
        ratio: f64,
    },

    /// Accessibility violations below thresholds (requires axe-core).
    AccessibilityViolationsBelow {
        max_critical: u32,
        max_serious: u32,
    },

    /// Visual evaluator score must be at least this value (0.0–10.0).
    /// Normally controlled by config threshold, but can be overridden per-assertion.
    VisualScoreAtLeast {
        score: f64,
    },

    /// Run a custom JavaScript expression. It must return a truthy value.
    /// Requires [gates.ui.security] allow_evaluate_steps = true.
    CustomEvaluate {
        name: String,
        script: String,
        /// If set, deep-compare the script's return value to this JSON.
        expect_json: Option<serde_json::Value>,
    },

    /// Evaluate a JS expression — must return truthy.
    JsExpression {
        script: String,
    },
}
```

### UiArtifactRetention

```rust
/// How much evidence to keep on disk per attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiArtifactRetention {
    /// result.json, vision-eval.json, final screenshots only.
    Minimal,
    /// Minimal + console.json, requests.json, DOM snapshots, layout.json, a11y.json.
    Debug,
    /// Debug + trace.zip, network.har, optional video.
    Full,
}

impl Default for UiArtifactRetention {
    fn default() -> Self {
        Self::Debug
    }
}
```

Default retention by context:

| Context | Pass | Fail |
|---|---|---|
| Local | `debug` | `full` |
| CI | `minimal` | `full` |

### Validation rules

- `[task.ui]` requires at least one journey (or a `url` plus global assertions).
- Each journey requires at least one step or a `start_url` plus assertions.
- `target_score` must be in `0.0..=10.0`.
- Viewport dimensions must be positive and bounded (max 4096).
- `Evaluate` steps and `CustomEvaluate` assertions require explicit config opt-in (`[gates.ui.security] allow_evaluate_steps = true`).
- External URLs (not localhost/127.0.0.1) require explicit opt-in (`[gates.ui.security] allow_external_urls = true`).

---

## 11. Global config: `[gates.ui]`

```toml
[gates.ui]
# Whether the UI gate is active. When false, tasks with [task.ui] are
# still parsed but the gate is skipped.
enabled = true

# Path to the browser runner script. Absolute or relative to workspace root.
runner_path = "tools/roko-ui-runner.mjs"

# Authoritative browser backend.
default_backend = "playwright-chromium"

# Optional fast preflight backend (advisory only).
preflight_backend = ""

# Visual score threshold on 0–10 scale. Below this, the gate fails.
visual_threshold = 7.0

# Whether to run the visual evaluator (tier 5). When false, only
# tiers 1–4 (deterministic checks) run.
visual_eval_enabled = true

# Model to use for visual evaluation. Must support vision (image input).
visual_eval_model = "claude-sonnet-4-6"

# Maximum retry attempts for UI gate failures.
max_retries = 3

# Maximum time in seconds for a single browser run (all viewports combined).
timeout_seconds = 120

# Whether to save Playwright trace files.
save_trace = true

# Whether to save HAR (HTTP Archive) network logs.
save_har = true

# Whether to save video recordings of browser sessions.
save_video = false

# Default artifact retention mode.
artifact_retention = "debug"

# Allowed network hosts beyond localhost.
network_allow = []

# Default viewports when task does not specify its own.
[[gates.ui.default_viewports]]
name = "desktop"
width = 1440
height = 900

[[gates.ui.default_viewports]]
name = "mobile"
width = 390
height = 844
is_mobile = true
has_touch = true

# Security constraints.
[gates.ui.security]
# Allow browser navigation to non-localhost URLs.
allow_external_urls = false
# Allow Evaluate steps and CustomEvaluate assertions (runs arbitrary JS).
allow_evaluate_steps = false
# HTTP headers to redact in stored evidence.
redact_headers = ["authorization", "cookie", "x-api-key"]
# Regex patterns to redact in console logs, network URLs, DOM snapshots.
redact_text_patterns = [
    "sk-[A-Za-z0-9_-]+",
    "Bearer [A-Za-z0-9._-]+",
    "(token|key|secret|password)\\s*[=:]\\s*\\S+",
]
```

### UiGateConfig struct

```rust
/// Configuration for the UI gate, read from [gates.ui] in roko.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiGateConfig {
    pub enabled: bool,
    pub runner_path: String,
    pub default_backend: String,
    pub preflight_backend: String,
    pub visual_threshold: f64,
    pub visual_eval_enabled: bool,
    pub visual_eval_model: String,
    pub max_retries: u32,
    pub timeout_seconds: u32,
    pub save_trace: bool,
    pub save_har: bool,
    pub save_video: bool,
    pub artifact_retention: UiArtifactRetention,
    pub network_allow: Vec<String>,
    pub default_viewports: Vec<UiViewport>,
    pub security: UiSecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSecurityConfig {
    pub allow_external_urls: bool,
    pub allow_evaluate_steps: bool,
    pub redact_headers: Vec<String>,
    pub redact_text_patterns: Vec<String>,
}

impl Default for UiGateConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            runner_path: "tools/roko-ui-runner.mjs".into(),
            default_backend: "playwright-chromium".into(),
            preflight_backend: String::new(),
            visual_threshold: 7.0,
            visual_eval_enabled: true,
            visual_eval_model: "claude-sonnet-4-6".into(),
            max_retries: 3,
            timeout_seconds: 120,
            save_trace: true,
            save_har: true,
            save_video: false,
            artifact_retention: UiArtifactRetention::Debug,
            network_allow: vec![],
            default_viewports: vec![
                UiViewport {
                    name: "desktop".into(),
                    width: 1440,
                    height: 900,
                    device_scale_factor: None,
                    is_mobile: false,
                    has_touch: false,
                },
                UiViewport {
                    name: "mobile".into(),
                    width: 390,
                    height: 844,
                    device_scale_factor: None,
                    is_mobile: true,
                    has_touch: true,
                },
            ],
            security: UiSecurityConfig {
                allow_external_urls: false,
                allow_evaluate_steps: false,
                redact_headers: vec![
                    "authorization".into(),
                    "cookie".into(),
                    "x-api-key".into(),
                ],
                redact_text_patterns: vec![
                    "sk-[A-Za-z0-9_-]+".into(),
                    "Bearer [A-Za-z0-9._-]+".into(),
                ],
            },
        }
    }
}
```

Task-specific settings in `UiTaskSpec` override global settings. For example, `UiTaskSpec.target_score` overrides `UiGateConfig.visual_threshold` for that task.

---

## 12. Browser backend strategy

### Authoritative backend

Use Playwright with real Chromium as the authoritative backend.

Why Chromium:
- Real rendering (CSS, fonts, SVG, canvas, WebGL)
- Pixel-accurate screenshots
- Playwright traces for debugging
- Console and network event capture
- Page error capture
- Accessibility snapshots (via CDP)
- Viewport and device emulation
- Robust locator API (role, label, test-id, text)
- Widely understood test semantics

### Backend trait

Define a backend abstraction so future implementations can use alternative engines:

```rust
/// Conceptual Rust — the browser backend abstraction.
#[async_trait]
pub trait BrowserBackend: Send + Sync {
    /// Execute a browser run and return structured results.
    async fn run(&self, spec: &BrowserRunSpec) -> Result<BrowserRunResult>;

    /// Human-readable backend name.
    fn name(&self) -> &str;

    /// What this backend can do. Used by the gate to decide which
    /// checks are meaningful.
    fn capabilities(&self) -> BrowserCapabilities;
}

/// What a browser backend supports.
pub struct BrowserCapabilities {
    /// Can produce screenshots.
    pub screenshots: bool,
    /// Can capture console events.
    pub console_events: bool,
    /// Can capture network events.
    pub network_events: bool,
    /// Can generate Playwright traces.
    pub traces: bool,
    /// Can generate HAR files.
    pub har: bool,
    /// Can produce accessibility snapshots.
    pub accessibility: bool,
    /// Can accurately emulate viewports.
    pub viewport_emulation: bool,
    /// Can record video.
    pub video: bool,
}
```

Possible backends:
- `playwright-chromium`: default and authoritative.
- `playwright-webkit`: optional cross-browser.
- `playwright-firefox`: optional cross-browser.
- `external-cdp`: connect to a remote browser via Chrome DevTools Protocol.
- `obscura-cdp`: optional fast preflight (lightweight Rust headless browser).

### Obscura position

Obscura (or similar lightweight headless browsers) may be useful for fast DOM checks, selector existence, basic navigation, network smoke checks, and cheap preflight before running full Chromium. It should NOT be authoritative at first. A visual gate needs high-fidelity screenshots, accurate layout, correct CSS rendering, proper text rendering, and real browser compatibility. If a backend cannot produce evidence equivalent to Chromium, it should only be used as a preflight optimization.

Preflight policy:

```
if task has visual_goal or screenshot assertions:
    authoritative_backend = playwright-chromium (always)

if preflight_backend is configured:
    run it first
    if it hard-fails:
        fail fast (skip authoritative) or mark as preflight failure depending on config
    if it passes:
        still run authoritative backend before accepting the task
```

---

## 13. Browser runner contract

The browser runner is a standalone Node.js script. It does NOT call any LLM. It only runs browser automation and writes evidence.

### Contract

- **Location**: `tools/roko-ui-runner.mjs` at the workspace root.
- **Invocation**: `node tools/roko-ui-runner.mjs --spec <path-to-spec.json>`
  - Or via stdin: `echo '<json>' | node tools/roko-ui-runner.mjs`
- **Input**: `BrowserRunSpec` as a JSON file (via `--spec`) or JSON line on stdin.
- **Output**: `BrowserRunResult` as JSON written to `<output_dir>/result.json` and echoed to stdout.
- **Side effects**: writes screenshots, traces, HAR, and evidence files to `output_dir`.
- **Exit code 0**: runner completed (even if UI assertions failed — failure is reported in JSON).
- **Exit code 1**: unrecoverable infrastructure error (missing Playwright, invalid spec JSON, browser launch failure, filesystem error).

The runner should NOT call the LLM. It only runs browser automation and writes evidence. The visual evaluator is separate.

### BrowserRunSpec

```rust
/// Input to the browser runner.
/// Serialized as JSON and passed via --spec or stdin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserRunSpec {
    /// Schema version for forward compatibility.
    pub schema_version: u32, // currently 1

    /// Unique identifier for this run.
    pub run_id: String,

    /// Plan that owns this task (for artifact paths).
    pub plan_id: String,

    /// Task being verified.
    pub task_id: String,

    /// Attempt number (1-indexed).
    pub attempt: u32,

    /// Browser backend to use.
    pub backend: String,

    /// Base URL of the app.
    pub base_url: String,

    /// Directory to write artifacts to (relative to workspace root).
    pub output_dir: String,

    /// Maximum time for the entire run in milliseconds.
    pub timeout_ms: u64,

    /// Whether to save Playwright trace.
    pub save_trace: bool,

    /// Whether to save HAR file.
    pub save_har: bool,

    /// Whether to record video.
    pub save_video: bool,

    /// Viewports to test.
    pub viewports: Vec<UiViewport>,

    /// Journeys to execute (in each viewport).
    pub journeys: Vec<UiJourney>,

    /// Security config (redaction patterns, URL allowlists).
    pub security: UiSecurityConfig,
}
```

### BrowserRunResult

The runner result is structured per-viewport, per-journey. This allows the gate to pinpoint exactly which viewport and which journey failed.

```rust
/// Output from the browser runner.
/// Written to <output_dir>/result.json and echoed to stdout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserRunResult {
    /// Schema version.
    pub schema_version: u32,

    /// Run ID (matches spec).
    pub run_id: String,

    /// Plan ID.
    pub plan_id: String,

    /// Task ID.
    pub task_id: String,

    /// Attempt number.
    pub attempt: u32,

    /// Backend used.
    pub backend: String,

    /// ISO 8601 timestamp of when the run started.
    pub started_at: String,

    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,

    /// Did all assertions across all viewports pass?
    pub passed: bool,

    /// Human-readable summary of what happened.
    pub summary: String,

    /// Stable failure class identifiers for downstream learning.
    pub failure_classes: Vec<String>,

    /// Per-viewport results.
    pub viewports: Vec<ViewportResult>,

    /// Paths to trace/HAR/video artifacts (relative to output_dir).
    pub artifacts: RunArtifacts,
}

/// Results for a single viewport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportResult {
    pub name: String,
    pub width: u32,
    pub height: u32,

    /// Per-journey results within this viewport.
    pub journeys: Vec<JourneyResult>,
}

/// Results for a single journey within a viewport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyResult {
    /// Journey ID.
    pub id: String,

    /// Did all steps and assertions pass?
    pub passed: bool,

    /// Final URL after journey execution.
    pub final_url: String,

    /// Paths to screenshots taken during this journey (relative to output_dir).
    pub screenshots: Vec<String>,

    /// Per-step results.
    pub steps: Vec<StepResult>,

    /// Assertion results.
    pub assertions: Vec<AssertionResult>,

    /// Console messages captured during this journey.
    pub console: Vec<ConsoleMessage>,

    /// Unhandled page errors (exceptions).
    pub page_errors: Vec<PageError>,

    /// Network requests and responses.
    pub requests: Vec<NetworkRequest>,

    /// Layout metrics collected after the journey.
    pub layout: LayoutMetrics,

    /// Accessibility data (if configured).
    pub accessibility: Option<AccessibilityResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// 0-indexed step number.
    pub index: u32,
    /// The step action (e.g., "click", "fill", "screenshot").
    pub action: String,
    /// Did this step succeed?
    pub success: bool,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Error message if the step failed.
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    /// Assertion description (e.g., "text_visible:Demo Project").
    pub name: String,
    /// Did this assertion pass?
    pub passed: bool,
    /// "hard" or "soft".
    pub severity: String,
    /// Diagnostic detail if the assertion failed.
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleMessage {
    /// Console API type: "log", "warn", "error", "info", "debug".
    pub level: String,
    /// The text content of the message.
    pub text: String,
    /// Source URL where the message was emitted.
    pub url: Option<String>,
    /// Line number in the source.
    pub line_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageError {
    /// The error message.
    pub message: String,
    /// Stack trace if available.
    pub stack: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    /// Request URL.
    pub url: String,
    /// HTTP method (GET, POST, etc.).
    pub method: String,
    /// Response status code (null if request never completed).
    pub status: Option<u32>,
    /// True if the request failed entirely (network error, aborted).
    pub failed: bool,
    /// Error text from Playwright if the request failed.
    pub failure_text: Option<String>,
    /// Response body size in bytes (if available).
    pub response_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutMetrics {
    /// Browser viewport width in CSS pixels.
    pub viewport_width: u32,
    /// Browser viewport height in CSS pixels.
    pub viewport_height: u32,
    /// Full document width (including overflow).
    pub document_width: u32,
    /// Full document height (including overflow).
    pub document_height: u32,
    /// True if document_width > viewport_width + 1.
    pub horizontal_overflow: bool,
    /// Elements where scrollWidth > clientWidth or scrollHeight > clientHeight.
    pub clipped_text_candidates: Vec<ClippedTextCandidate>,
    /// Elements whose bounding rectangles overlap (heuristic, advisory).
    pub overlapping_text_candidates: Vec<OverlappingTextCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClippedTextCandidate {
    /// Truncated text content (first 120 chars).
    pub text: String,
    /// Bounding rectangle from getBoundingClientRect().
    pub rect: DomRect,
    pub scroll_width: u32,
    pub client_width: u32,
    pub scroll_height: u32,
    pub client_height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlappingTextCandidate {
    /// Text of the first overlapping element.
    pub text_a: String,
    /// Text of the second overlapping element.
    pub text_b: String,
    /// Bounding rectangles of both elements.
    pub rect_a: DomRect,
    pub rect_b: DomRect,
    /// Overlap area in square CSS pixels.
    pub overlap_area: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityResult {
    /// Path to Playwright accessibility snapshot JSON (relative to output_dir).
    pub snapshot_path: Option<String>,
    /// Path to axe-core violations JSON (relative to output_dir).
    pub violations_path: Option<String>,
    /// Number of critical axe-core violations.
    pub critical: u32,
    /// Number of serious axe-core violations.
    pub serious: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunArtifacts {
    /// Path to Playwright trace file (relative to output_dir).
    pub trace: Option<String>,
    /// Path to HAR file (relative to output_dir).
    pub har: Option<String>,
    /// Path to video file (relative to output_dir).
    pub video: Option<String>,
    /// All screenshot paths (relative to output_dir).
    pub screenshots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotCapture {
    /// File path relative to output_dir.
    pub path: String,
    /// Label from the Screenshot step or auto-generated.
    pub label: String,
    /// Viewport name this screenshot was taken in.
    pub viewport: String,
    /// Journey ID this screenshot belongs to.
    pub journey_id: String,
}
```

### Failure classes

Use stable string identifiers so learning systems can aggregate failures:

```rust
/// Stable failure class identifiers.
/// These appear in BrowserRunResult.failure_classes and in learning events.
pub enum UiFailureClass {
    AppUnavailable,        // Tier 1: app URL returned non-200
    DevServerFailed,       // Tier 1: dev server command exited or timed out
    BrowserLaunchFailed,   // Tier 1: Chromium failed to start
    NavigationFailed,      // Tier 1: page.goto() failed
    LocatorNotFound,       // Tier 2: element matching locator not found
    ActionTimeout,         // Tier 2: click/fill/etc. timed out
    AssertionFailed,       // Tier 2: explicit assertion failed
    ConsoleError,          // Tier 3: console.error emitted
    PageError,             // Tier 3: unhandled exception
    FailedRequest,         // Tier 3: HTTP 4xx/5xx or network failure
    AuthRequired,          // Tier 3: received 401/403 on a required request
    HydrationError,        // Tier 3: React/Next.js hydration mismatch
    LayoutOverflow,        // Tier 4: horizontal overflow
    TextClipped,           // Tier 4: text clipped by overflow:hidden
    TextOverlap,           // Tier 4: overlapping text elements
    A11yCritical,          // Tier 4: critical accessibility violation
    VisualScoreLow,        // Tier 5: visual score below threshold
    VisualRegression,      // Tier 5: visual score decreased from previous attempt
    RunnerInfrastructure,  // Meta: runner itself crashed/timed out
}
```

Use these in: result JSON, verdict detail, feedback, metrics, learning events, dashboard summaries, retry policy.

---

## 14. Visual evaluator

The visual evaluator is separate from the browser runner. It receives screenshots and evidence, then returns a strict JSON evaluation. It does NOT edit code — that is the implementer agent's job. The evaluator is called by UiGate after the browser run completes and all deterministic checks pass.

### Input assembly

For each screenshot, the evaluator builds a prompt containing six pieces of context:

1. **The task context.** Task ID, title, description, and acceptance criteria. This tells the evaluator what was requested.

2. **The visual goal.** The `visual_goal` string from `UiTaskSpec`. This tells the evaluator what the UI should look like.

3. **The screenshot.** Base64-encoded PNG image, sent as an image content block.

4. **The viewport.** Name, dimensions, and whether it's mobile. This helps the evaluator judge responsiveness.

5. **Browser evidence summary.** A text block listing: console errors (count and first 3), failed network requests (count and first 3), layout issues (horizontal overflow yes/no, clipped text count, overlapping text count). This gives the evaluator context about runtime problems even though they were already checked deterministically — it helps the LLM calibrate its score.

6. **The scoring rubric.** The seven dimensions and their weights (defined below). The evaluator is asked to score each dimension individually, then the gate computes the weighted total.

7. **Prior attempt context.** If this is a retry (attempt > 1), include the previous attempt's visual score, findings, and what the agent was asked to fix. This helps the evaluator detect regressions.

### Scoring rubric

The evaluator scores the screenshot on a 0–10 scale across seven dimensions:

| Dimension | Weight | What it measures |
|---|---|---|
| Task completion | 0.25 | Required workflow state is visibly achieved. All acceptance criteria appear satisfied in the screenshot. |
| Layout integrity | 0.20 | No obvious overlap, clipping, broken spacing, bad alignment. Elements positioned correctly. |
| Responsive quality | 0.15 | Works correctly at the given viewport size. No breakpoint artifacts. |
| Interaction clarity | 0.10 | Controls, focus, loading, empty, error, and success states are clear and usable. |
| Visual polish | 0.10 | Typography, spacing, hierarchy, balance, borders, shadows look intentional. |
| Design-system fit | 0.10 | Fits the existing app conventions (if any context provided). Consistent visual language. |
| Accessibility affordance | 0.10 | Contrast sufficient, tap targets appropriately sized, focus indicators present, semantic structure visible. |

Weighted total:

```
total = (task_completion * 0.25)
      + (layout_integrity * 0.20)
      + (responsive_quality * 0.15)
      + (interaction_clarity * 0.10)
      + (visual_polish * 0.10)
      + (design_system_fit * 0.10)
      + (accessibility_affordance * 0.10)
```

### Score bands

| Score | Meaning |
|---|---|
| 9.0–10.0 | Excellent. Production-ready. |
| 7.0–8.9 | Good. Meets requirements with minor polish issues. |
| 5.0–6.9 | Acceptable. Functional but needs work. |
| 3.0–4.9 | Poor. Significant visual problems. |
| 0.0–2.9 | Broken. Layout is fundamentally wrong. |

### VisualEvalResult

```rust
/// Result from the tier 5 visual evaluator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualEvalResult {
    /// Schema version.
    pub schema_version: u32,

    /// Model used for evaluation (e.g., "claude-sonnet-4-6").
    pub model: String,

    /// Weighted total score on 0.0–10.0 scale.
    pub score: f64,

    /// Whether this score meets the threshold.
    pub passed: bool,

    /// The threshold used for this evaluation.
    pub threshold: f64,

    /// Evaluator's self-reported confidence in the score (0.0–1.0).
    pub confidence: f64,

    /// Per-dimension scores.
    pub rubric_scores: VisualRubricScores,

    /// LLM's textual summary: what looks good, what needs fixing.
    pub summary: String,

    /// Specific findings with severity and suggested fixes.
    pub findings: Vec<VisualFinding>,

    /// Which screenshot was evaluated (relative path).
    pub screenshot: String,

    /// Viewport name this screenshot was taken in.
    pub viewport: String,

    /// Journey ID this screenshot belongs to.
    pub journey_id: String,
}

/// Per-dimension scores from the visual evaluator.
/// Each field is 0.0–10.0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualRubricScores {
    pub task_completion: f64,
    pub layout_integrity: f64,
    pub responsive_quality: f64,
    pub interaction_clarity: f64,
    pub visual_polish: f64,
    pub design_system_fit: f64,
    pub accessibility_affordance: f64,
}

/// A specific issue identified by the visual evaluator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualFinding {
    /// Severity: "high", "medium", or "low".
    /// "high" findings may fail the gate on their own.
    pub severity: String,

    /// Which viewport this finding applies to.
    pub viewport: String,

    /// Which journey this finding applies to.
    pub journey_id: String,

    /// Which screenshot this finding was observed in.
    pub screenshot: String,

    /// The part of the UI affected (e.g., "modal footer", "nav bar").
    pub area: String,

    /// What is wrong. Written as a concrete observation.
    pub problem: String,

    /// What the evaluator saw that led to this finding.
    pub evidence: String,

    /// Suggested fix. Written as an actionable instruction.
    pub suggested_fix: String,

    /// Which rubric dimension this finding relates to.
    pub dimension: Option<String>,

    /// Optional CSS selector for the problematic element.
    pub selector: Option<String>,
}
```

### Model selection

Use the model configured in `[gates.ui]` (`visual_eval_model`). Default: `claude-sonnet-4-6`. The evaluator needs vision capability and structured output. It does not need to be the most expensive model — Sonnet-class is usually sufficient.

### JSON parsing

Extract JSON from the LLM response. If the response contains markdown code fences (` ```json ... ``` `), strip them. If the JSON is invalid, return a `VisualEvalResult` with score 0.0, confidence 0.0, and a finding noting the parse failure. Do not crash the gate on an invalid evaluator response.

---

## 15. UiGate: putting it together

This section describes the complete `verify()` flow. This is the core implementation.

### UiGate struct

```rust
/// The UI verification gate.
/// Implements the Gate trait from roko-core.
pub struct UiGate {
    /// Path to the browser runner script.
    runner_path: PathBuf,

    /// Configuration from [gates.ui] in roko.toml.
    config: UiGateConfig,
}
```

### Gate trait implementation

The `verify()` method runs all five tiers in order, short-circuiting on hard failures.

```rust
#[async_trait]
impl Gate for UiGate {
    async fn verify(&self, engram: &Engram, ctx: &Context) -> Verdict {
        let started = std::time::Instant::now();

        // ---------------------------------------------------------------
        // Step 0: Check if this task has a UI spec.
        // ---------------------------------------------------------------
        let ui_spec = match extract_ui_spec(engram) {
            Some(spec) => spec,
            None => return Verdict::pass("ui").with_duration(elapsed_ms(started)),
        };

        // Check if UI gate is enabled.
        if !self.config.enabled {
            return Verdict::pass("ui").with_duration(elapsed_ms(started));
        }

        // ---------------------------------------------------------------
        // Step 1: Resolve viewports.
        // ---------------------------------------------------------------
        let viewports = if ui_spec.viewports.is_empty() {
            self.config.default_viewports.clone()
        } else {
            ui_spec.viewports.clone()
        };

        // ---------------------------------------------------------------
        // Step 2: Start dev server (if configured).
        // ---------------------------------------------------------------
        // DevServerHandle is an RAII guard. When dropped, the dev server
        // process is killed. This guarantees cleanup on early return.
        let _dev_server = if let Some(ref cmd) = ui_spec.dev_server {
            let url = ui_spec.url.as_deref().unwrap_or("http://localhost:3000");
            match start_dev_server(cmd, url, ui_spec.cwd.as_deref()).await {
                Ok(handle) => Some(handle),
                Err(e) => return Verdict::fail("ui", format!("Dev server failed: {e}"))
                    .with_duration(elapsed_ms(started)),
            }
        } else {
            None
        };

        // ---------------------------------------------------------------
        // Step 3: Build BrowserRunSpec.
        // ---------------------------------------------------------------
        let task_id = extract_task_id(engram);
        let attempt = extract_attempt(ctx);
        let output_dir = format!(
            ".roko/ui-runs/{}/{:03}",
            task_id, attempt
        );

        let spec = BrowserRunSpec {
            schema_version: 1,
            run_id: generate_uuid(),
            plan_id: extract_plan_id(ctx),
            task_id: task_id.clone(),
            attempt,
            backend: ui_spec.backend.clone()
                .unwrap_or_else(|| self.config.default_backend.clone()),
            base_url: ui_spec.url.clone().unwrap_or_default(),
            output_dir: output_dir.clone(),
            timeout_ms: ui_spec.timeout_ms
                .unwrap_or((self.config.timeout_seconds as u64) * 1000),
            save_trace: self.config.save_trace,
            save_har: self.config.save_har,
            save_video: self.config.save_video,
            viewports: viewports.clone(),
            journeys: ui_spec.journeys.clone(),
            security: self.config.security.clone(),
        };

        // ---------------------------------------------------------------
        // Step 4: Run browser.
        // ---------------------------------------------------------------
        let run_result = match run_browser(&self.runner_path, &spec).await {
            Ok(result) => result,
            Err(e) => return Verdict::fail("ui", format!("Browser runner error: {e}"))
                .with_duration(elapsed_ms(started)),
        };

        // ---------------------------------------------------------------
        // TIER 1: Infrastructure
        // ---------------------------------------------------------------
        // Did the browser launch? Did navigation succeed?
        if !run_result.passed && run_result.viewports.is_empty() {
            return Verdict::fail("ui", format!(
                "Infrastructure failure: {}",
                run_result.summary
            ))
            .with_detail(&serde_json::to_string(&run_result).unwrap_or_default())
            .with_duration(elapsed_ms(started));
        }

        let mut all_findings: Vec<VisualFinding> = Vec::new();
        let mut min_visual_score: f64 = 10.0;
        let mut has_hard_failure = false;
        let mut failure_reason = String::new();

        // ---------------------------------------------------------------
        // TIERS 2–4: Functional, Runtime, Layout (per viewport, per journey)
        // ---------------------------------------------------------------
        for vp_result in &run_result.viewports {
            for journey_result in &vp_result.journeys {
                // TIER 2: Functional — did steps and assertions pass?
                for step in &journey_result.steps {
                    if !step.success {
                        return Verdict::fail("ui", format!(
                            "Step {} ({}) failed in {}/{}: {}",
                            step.index, step.action,
                            vp_result.name, journey_result.id,
                            step.error.as_deref().unwrap_or("unknown")
                        ))
                        .with_detail(&serde_json::to_string(&run_result).unwrap_or_default())
                        .with_duration(elapsed_ms(started));
                    }
                }

                for assertion in &journey_result.assertions {
                    if !assertion.passed && assertion.severity == "hard" {
                        return Verdict::fail("ui", format!(
                            "Assertion '{}' failed in {}/{}: {}",
                            assertion.name,
                            vp_result.name, journey_result.id,
                            assertion.detail.as_deref().unwrap_or("no detail")
                        ))
                        .with_detail(&serde_json::to_string(&run_result).unwrap_or_default())
                        .with_duration(elapsed_ms(started));
                    }
                }

                // TIER 3: Runtime — console errors and network failures.
                let console_errors: Vec<_> = journey_result.console.iter()
                    .filter(|m| m.level == "error")
                    .collect();

                if !console_errors.is_empty() && ui_spec.hard_fail_on_console_error {
                    return Verdict::fail("ui", format!(
                        "{} console error(s) in {}/{}",
                        console_errors.len(),
                        vp_result.name, journey_result.id,
                    ))
                    .with_detail(&serde_json::to_string(&run_result).unwrap_or_default())
                    .with_duration(elapsed_ms(started));
                }

                let failed_requests: Vec<_> = journey_result.requests.iter()
                    .filter(|r| r.failed || r.status.map_or(false, |s| s >= 400))
                    .collect();

                if !failed_requests.is_empty() && ui_spec.hard_fail_on_failed_request {
                    return Verdict::fail("ui", format!(
                        "{} failed request(s) in {}/{}",
                        failed_requests.len(),
                        vp_result.name, journey_result.id,
                    ))
                    .with_detail(&serde_json::to_string(&run_result).unwrap_or_default())
                    .with_duration(elapsed_ms(started));
                }

                // TIER 4: Layout
                if journey_result.layout.horizontal_overflow {
                    return Verdict::fail("ui", format!(
                        "Horizontal overflow in {}/{}: document {}px > viewport {}px",
                        vp_result.name, journey_result.id,
                        journey_result.layout.document_width,
                        journey_result.layout.viewport_width,
                    ))
                    .with_detail(&serde_json::to_string(&run_result).unwrap_or_default())
                    .with_duration(elapsed_ms(started));
                }

                // Accessibility violations (if configured)
                if let Some(ref a11y) = journey_result.accessibility {
                    // Check global assertions for AccessibilityViolationsBelow
                    for assertion in ui_spec.assertions.iter().chain(ui_spec.journeys.iter()
                        .find(|j| j.id == journey_result.id)
                        .map(|j| j.asserts.iter())
                        .unwrap_or_else(|| [].iter()))
                    {
                        if let UiAssertion::AccessibilityViolationsBelow {
                            max_critical, max_serious
                        } = assertion {
                            if a11y.critical > *max_critical || a11y.serious > *max_serious {
                                return Verdict::fail("ui", format!(
                                    "Accessibility: {} critical, {} serious violations \
                                     (max: {} critical, {} serious)",
                                    a11y.critical, a11y.serious,
                                    max_critical, max_serious,
                                ))
                                .with_detail(&serde_json::to_string(&run_result).unwrap_or_default())
                                .with_duration(elapsed_ms(started));
                            }
                        }
                    }
                }
            }
        }

        // ---------------------------------------------------------------
        // TIER 5: Visual evaluation
        // ---------------------------------------------------------------
        if self.config.visual_eval_enabled {
            for vp_result in &run_result.viewports {
                for journey_result in &vp_result.journeys {
                    for screenshot_path in &journey_result.screenshots {
                        let eval_result = match run_visual_eval(
                            screenshot_path,
                            &ui_spec,
                            &vp_result.name,
                            &journey_result.id,
                            &run_result,
                            &self.config,
                            ctx,
                        ).await {
                            Ok(r) => r,
                            Err(e) => {
                                // Visual eval failure is not a gate crash —
                                // record it and continue.
                                all_findings.push(VisualFinding {
                                    severity: "low".into(),
                                    viewport: vp_result.name.clone(),
                                    journey_id: journey_result.id.clone(),
                                    screenshot: screenshot_path.clone(),
                                    area: "evaluator".into(),
                                    problem: format!("Visual evaluator failed: {e}"),
                                    evidence: String::new(),
                                    suggested_fix: String::new(),
                                    dimension: None,
                                    selector: None,
                                });
                                continue;
                            }
                        };

                        min_visual_score = min_visual_score.min(eval_result.score);
                        all_findings.extend(eval_result.findings.clone());
                    }
                }
            }
        }

        // ---------------------------------------------------------------
        // Step 5: Construct final verdict.
        // ---------------------------------------------------------------
        drop(_dev_server); // Kill dev server explicitly.

        let threshold = ui_spec.target_score.max(self.config.visual_threshold);
        let visual_passed = !self.config.visual_eval_enabled
            || min_visual_score >= threshold;
        let has_high_findings = all_findings.iter()
            .any(|f| f.severity == "high");

        if !visual_passed || has_high_findings {
            Verdict::fail("ui", format!(
                "Visual score {:.1}/10 (threshold {:.1})",
                min_visual_score, threshold,
            ))
            .with_score((min_visual_score / 10.0) as f32)
            .with_detail(&serde_json::to_string(&run_result).unwrap_or_default())
            .with_duration(elapsed_ms(started))
        } else {
            Verdict::pass("ui")
                .with_score((min_visual_score / 10.0) as f32)
                .with_duration(elapsed_ms(started))
        }
    }

    fn name(&self) -> &str {
        "ui"
    }
}
```

### Helper functions

**`extract_ui_spec(engram: &Engram) -> Option<UiTaskSpec>`** — Deserialize from the engram's JSON body. Return `None` if absent or if deserialization fails.

**`start_dev_server(cmd: &str, url: &str, cwd: Option<&Path>) -> Result<DevServerHandle>`** — Spawn the command. Poll the URL every 500ms. Return an RAII handle that kills the process on drop. Fail after 30 seconds.

**`run_browser(runner_path: &Path, spec: &BrowserRunSpec) -> Result<BrowserRunResult>`** — Create the output directory. Write `spec.json`. Spawn `node <runner_path> --spec <path>`. Wait for exit. Read `result.json`. Kill the process if it exceeds the timeout.

**`run_visual_eval(...) -> Result<VisualEvalResult>`** — Read screenshot from disk. Encode as base64 PNG. Build evaluation prompt. Send to vision model. Parse response. Write to `<output_dir>/eval/<screenshot_name>.json`.

### Rung placement

The UiGate runs AFTER all existing gate rungs. In the 7-rung pipeline (Compile → Lint → Test → Symbol → GeneratedTest → PropertyTest → Integration), the UiGate is rung 7 or a standalone gate invoked after the pipeline completes.

The rung selector includes the UiGate when all three conditions are true:

1. The task has a `[task.ui]` section.
2. `gates.ui.enabled = true` in the workspace config.
3. The task's complexity band includes UI verification (Standard or higher).

If any condition is false, the UiGate is skipped.

---

## 16. Pass/fail semantics

### Failure class table

| Class | Tier | Severity | Effect |
|---|---|---|---|
| Infrastructure failure (page won't load, browser crashes) | 1 | Hard | Immediate fail. No further checks. |
| Dev server won't start | 1 | Hard | Immediate fail. |
| Functional assertion failure (wrong count, missing element) | 2 | Hard | Immediate fail. |
| Step failure (click on missing element, timeout) | 2 | Hard | Immediate fail. |
| Console error (when hard_fail_on_console_error or NoConsoleErrors assertion) | 3 | Hard | Immediate fail. |
| Network failure (when hard_fail_on_failed_request or NoFailedRequests assertion) | 3 | Hard | Immediate fail. |
| Console error (no assertion, not hard_fail) | 3 | Soft | Captured. Does not fail. |
| Network failure (no assertion, not hard_fail) | 3 | Soft | Captured. Does not fail. |
| Horizontal overflow | 4 | Hard | Immediate fail. |
| Clipped text | 4 | Soft | Captured as finding. |
| Overlapping text | 4 | Soft | Captured as finding (heuristic, advisory). |
| Accessibility critical violation | 4 | Hard (if assertion) | Fail if AccessibilityViolationsBelow assertion present. |
| Visual score below threshold | 5 | Hard (derived) | Fails the gate. |
| Visual finding (high severity) | 5 | Hard | Fails the gate. |
| Visual finding (medium/low) | 5 | Soft | Captured. Does not fail alone. |

### Short-circuit behavior

The gate short-circuits at the first hard failure. When tier 2 fails, tiers 3–5 do not run. This saves time and avoids confusing the agent with irrelevant findings.

### Score normalization

The visual evaluator scores 0–10. The `Verdict.score` field uses 0–1. The gate normalizes: `verdict_score = eval_score / 10.0`. Threshold comparison happens on the 0–10 scale for readability.

### Normalized verdict score formula

```
if infrastructure_failed:
    score = 0.0
elif hard_failures_exist:
    score = min(0.49, hard_assertion_pass_ratio * 0.49)
elif visual_eval_enabled:
    score = visual_score / 10.0
else:
    score = 1.0
```

---

## 17. Retry feedback format

When the gate fails, the orchestrator feeds structured retry feedback to the agent. The feedback quality determines whether the agent can fix the problem. This is the single most important part of the loop.

### Format

```markdown
## UI Gate Failure (attempt {n}/{max})

Task: {task_id} {task_title}

### Hard failures
{for each hard failure:}
- **{tier_name}**: {viewport}/{journey}: {description}

### Soft findings
{for each soft finding:}
- **{severity}** ({dimension}): {viewport}/{journey}: {description}

### Visual score
{score}/10 (threshold {threshold})
{evaluator summary}

### Browser evidence
- Console errors: {count} ({first 3 error messages})
- Page errors: {count}
- Failed requests: {count} ({first 3 URLs with status codes})
- Layout: {overflow yes/no, clipped count, overlap count}
- Screenshots: {list of paths}
- Trace: {path if available}

### What to fix
{Synthesized from hard failures and lowest-scoring dimensions.
Prioritized list of 1–3 specific, actionable changes.}
```

### Rules for "what to fix"

1. Lead with the hard failure if one exists. The agent cannot pass until it is resolved.
2. Limit to three items. More than three overwhelms the agent.
3. Be concrete. "Fix grid gap to be uniform between all cards (check CSS grid or flex gap property)" beats "Fix spacing."
4. Reference specific elements or selectors when available.
5. Do not repeat the evidence section — summarize what changed and what to do.
6. Include previous best attempt when a retry regresses.
7. Preserve hard failure ordering: functional before visual polish.

### Example: functional + visual failure

```markdown
## UI Gate Failure (attempt 2/3)

Task: F3 Build project creation modal

### Hard failures
- **Layout**: mobile/create-project: horizontal overflow. Document width 431px > viewport width 390px.

### Soft findings
- **high** (layout_integrity): mobile/create-project: Modal footer "Create" button is partly below the viewport.
  Screenshot: .roko/ui-runs/F3/002/mobile/create-project/final.png
  Suggested fix: Constrain modal height, add internal scrolling, keep actions visible.

### Browser evidence
- Console errors: 0
- Page errors: 0
- Failed requests: 0
- Layout: horizontal overflow on mobile, 0 clipped text
- Screenshots: .roko/ui-runs/F3/002/mobile/create-project/final.png, .roko/ui-runs/F3/002/desktop/create-project/final.png
- Trace: .roko/ui-runs/F3/002/trace.zip

### What to fix
1. Remove horizontal overflow at 390×844 mobile viewport. The modal or its contents are wider than the screen — check for fixed-width elements or unresponsive padding.
2. Keep the modal action row (Create button) visible without scrolling on mobile.
3. Do not regress desktop behavior.
```

### Example: infrastructure failure

```markdown
## UI Gate Failure (attempt 1/3)

Task: F3 Build project creation modal

### Hard failures
- **Infrastructure**: Dev server at http://localhost:5173 did not respond within 30 seconds.

### Browser evidence
- Screenshots: none (browser did not launch)
- Console errors: N/A
- Failed requests: N/A

### What to fix
The dev server command `npm run dev -- --host 127.0.0.1` did not produce an HTTP 200 at http://localhost:5173 within 30 seconds. Check that the dev server starts without errors, the port is correct, and no other process is using port 5173.
```

---

## 18. Artifact layout

All UI run artifacts are stored under `.roko/ui-runs/`. The directory structure is deterministic.

```
.roko/ui-runs/
  {task-id}/
    001/                           # Attempt 1
      spec.json                    # BrowserRunSpec sent to runner
      result.json                  # BrowserRunResult from runner
      vision-eval.json             # Merged VisualEvalResult (all screenshots)
      feedback.md                  # Retry feedback (if gate failed)
      trace.zip                    # Playwright trace (if configured)
      network.har                  # HAR file (if configured)
      console.json                 # Array of ConsoleMessage objects
      requests.json                # Array of NetworkRequest objects
      page-errors.json             # Array of PageError objects
      desktop/                     # Per-viewport directory
        create-project/            # Per-journey directory
          before.png               # Screenshot before first step
          final.png                # Screenshot after last step
          after-create.png         # Named screenshot from Screenshot step
          dom.html                 # Serialized DOM snapshot
          text.txt                 # Extracted visible text
          a11y.json                # Playwright accessibility snapshot
          axe.json                 # axe-core violation report
          layout.json              # LayoutMetrics
      mobile/
        create-project/
          final.png
          dom.html
          text.txt
          a11y.json
          axe.json
          layout.json
      eval/                        # Per-screenshot visual eval results
        desktop-create-project-final.json
        mobile-create-project-final.json
    002/                           # Attempt 2
      ...
    verdict.json                   # Final Verdict for most recent attempt
    summary.json                   # Aggregate across all attempts
```

### UiRunSummary

```rust
/// Aggregate summary across all UI gate attempts for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiRunSummary {
    pub task_id: String,
    pub attempt_count: u32,
    pub passed: bool,
    pub attempts: Vec<UiAttemptRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiAttemptRecord {
    /// Attempt number (1-indexed).
    pub attempt: u32,
    /// Whether this attempt passed.
    pub passed: bool,
    /// Failure tier if failed (1–5), null if passed.
    pub failure_tier: Option<u8>,
    /// Failure classes for this attempt.
    pub failure_classes: Vec<String>,
    /// Visual score if tier 5 ran.
    pub visual_score: Option<f64>,
    /// Number of hard failures.
    pub hard_failure_count: u32,
    /// Number of soft findings.
    pub soft_finding_count: u32,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// ISO 8601 timestamp.
    pub timestamp: String,
}
```

### Retention modes

| Mode | Keeps |
|---|---|
| `minimal` | `result.json`, `vision-eval.json`, final screenshots only |
| `debug` | Minimal + `console.json`, `requests.json`, `page-errors.json`, DOM snapshots, `layout.json`, `a11y.json` |
| `full` | Debug + `trace.zip`, `network.har`, optional video, all intermediate screenshots |

### What this structure supports

- **Debugging.** Inspect exactly what happened on each attempt — the spec sent, the result received, screenshots taken, evaluator assessment. The trace file can be opened in Playwright's trace viewer.
- **Learning.** Compare across attempts to see what the agent changed and whether scores improved. Feed into cascade router and prompt experiments.
- **Audit.** Every verdict is traceable to evidence. The screenshot the evaluator scored is on disk. Console errors and network requests are in JSON files.
- **Retry context.** The feedback formatter reads `result.json` and `eval/*.json` to build the "what to fix" section.
