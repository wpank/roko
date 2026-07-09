# Part V: Worked examples

This document contains complete JSON payloads showing data flowing between components. If you read one file to understand how the system works end-to-end, read this one.

---

## 44. Complete task definition (TOML)

This is the full task as it appears in `tasks.toml`:

```toml
[[task]]
id = "F3"
title = "Build project creation modal"
description = """
Implement the modal flow for creating a project from the dashboard.
The modal should open from a "New project" button, contain a form
with project name input, and submit to POST /api/projects. After
successful creation, the new project should appear in the dashboard list.
"""
tier = "integrative"
deps = []
files = [
    "src/app/dashboard/page.tsx",
    "src/components/project-modal.tsx",
    "src/components/project-form.tsx",
]
verify = ["npm test"]
acceptance = [
    "User can open the modal from the dashboard",
    "User can enter a project name and create it",
    "Created project appears in the dashboard list",
    "The flow works on desktop and mobile",
    "No console errors during the flow",
    "API request succeeds (no 4xx/5xx)",
]

[task.ui]
url = "http://localhost:5173/dashboard"
dev_server = "npm run dev -- --host 127.0.0.1"
hard_fail_on_console_error = true
hard_fail_on_failed_request = true
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
action = "wait_for_text"
text = "Demo Project"
timeout_ms = 5000

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

---

## 45. BrowserRunSpec (spec.json)

The Rust UiGate builds this from the parsed task and config, then writes it to disk and passes the path to the runner.

```json
{
  "schema_version": 1,
  "run_id": "a7b3c1d2-4e5f-6789-abcd-ef0123456789",
  "plan_id": "frontend-plan",
  "task_id": "F3",
  "attempt": 2,
  "backend": "playwright-chromium",
  "base_url": "http://localhost:5173/dashboard",
  "output_dir": ".roko/ui-runs/F3/002",
  "timeout_ms": 120000,
  "save_trace": true,
  "save_har": true,
  "save_video": false,
  "viewports": [
    {
      "name": "desktop",
      "width": 1440,
      "height": 900,
      "device_scale_factor": null,
      "is_mobile": false,
      "has_touch": false
    },
    {
      "name": "mobile",
      "width": 390,
      "height": 844,
      "device_scale_factor": null,
      "is_mobile": true,
      "has_touch": true
    }
  ],
  "journeys": [
    {
      "id": "create-project",
      "name": "Create a project from dashboard",
      "start_url": "http://localhost:5173/dashboard",
      "auth_state": null,
      "steps": [
        { "action": "click", "role": "button", "name": "New project" },
        { "action": "fill", "label": "Project name", "value": "Demo Project" },
        { "action": "click", "role": "button", "name": "Create" },
        { "action": "wait_for_text", "text": "Demo Project", "timeout_ms": 5000 },
        { "action": "screenshot", "name": "after-create" }
      ],
      "asserts": [
        { "type": "text_visible", "text": "Demo Project" },
        { "type": "no_console_errors" },
        { "type": "no_failed_requests" },
        { "type": "no_horizontal_overflow" }
      ],
      "screenshot": "final_only"
    }
  ],
  "security": {
    "allow_external_urls": false,
    "allow_evaluate_steps": false,
    "redact_headers": ["authorization", "cookie", "x-api-key"],
    "redact_text_patterns": ["sk-[A-Za-z0-9_-]+", "Bearer [A-Za-z0-9._-]+"]
  }
}
```

---

## 46. BrowserRunResult — failing (result.json)

This is what the runner produces when attempt 2 has a layout issue on mobile:

```json
{
  "schema_version": 1,
  "run_id": "a7b3c1d2-4e5f-6789-abcd-ef0123456789",
  "plan_id": "frontend-plan",
  "task_id": "F3",
  "attempt": 2,
  "backend": "playwright-chromium",
  "started_at": "2026-04-25T11:32:00Z",
  "duration_ms": 18423,
  "passed": false,
  "summary": "mobile/create-project: horizontal overflow (431px > 390px), 1 failed request",
  "failure_classes": ["layout_overflow", "failed_request"],
  "viewports": [
    {
      "name": "desktop",
      "width": 1440,
      "height": 900,
      "journeys": [
        {
          "id": "create-project",
          "passed": true,
          "final_url": "http://localhost:5173/dashboard",
          "screenshots": [
            "desktop/create-project/after-create.png",
            "desktop/create-project/final.png"
          ],
          "steps": [
            { "index": 0, "action": "click", "success": true, "duration_ms": 342, "error": null },
            { "index": 1, "action": "fill", "success": true, "duration_ms": 187, "error": null },
            { "index": 2, "action": "click", "success": true, "duration_ms": 1203, "error": null },
            { "index": 3, "action": "wait_for_text", "success": true, "duration_ms": 2104, "error": null },
            { "index": 4, "action": "screenshot", "success": true, "duration_ms": 89, "error": null }
          ],
          "assertions": [
            { "name": "text_visible:Demo Project", "passed": true, "severity": "hard", "detail": null },
            { "name": "no_console_errors", "passed": true, "severity": "hard", "detail": null },
            { "name": "no_failed_requests", "passed": true, "severity": "hard", "detail": null },
            { "name": "no_horizontal_overflow", "passed": true, "severity": "hard", "detail": null }
          ],
          "console": [],
          "page_errors": [],
          "requests": [
            { "url": "http://localhost:5173/dashboard", "method": "GET", "status": 200, "failed": false, "failure_text": null, "response_size": 4521 },
            { "url": "http://localhost:5173/api/projects", "method": "POST", "status": 201, "failed": false, "failure_text": null, "response_size": 89 },
            { "url": "http://localhost:5173/api/projects", "method": "GET", "status": 200, "failed": false, "failure_text": null, "response_size": 312 }
          ],
          "layout": {
            "viewport_width": 1440,
            "viewport_height": 900,
            "document_width": 1440,
            "document_height": 1200,
            "horizontal_overflow": false,
            "clipped_text_candidates": [],
            "overlapping_text_candidates": []
          },
          "accessibility": {
            "snapshot_path": "desktop/create-project/a11y.json",
            "violations_path": null,
            "critical": 0,
            "serious": 0
          }
        }
      ]
    },
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
            "mobile/create-project/after-create.png",
            "mobile/create-project/final.png"
          ],
          "steps": [
            { "index": 0, "action": "click", "success": true, "duration_ms": 456, "error": null },
            { "index": 1, "action": "fill", "success": true, "duration_ms": 201, "error": null },
            { "index": 2, "action": "click", "success": true, "duration_ms": 1456, "error": null },
            { "index": 3, "action": "wait_for_text", "success": true, "duration_ms": 2301, "error": null },
            { "index": 4, "action": "screenshot", "success": true, "duration_ms": 102, "error": null }
          ],
          "assertions": [
            { "name": "text_visible:Demo Project", "passed": true, "severity": "hard", "detail": null },
            { "name": "no_console_errors", "passed": true, "severity": "hard", "detail": null },
            {
              "name": "no_failed_requests",
              "passed": false,
              "severity": "hard",
              "detail": "1 failed request(s): POST http://localhost:5173/api/projects → 500"
            },
            {
              "name": "no_horizontal_overflow",
              "passed": false,
              "severity": "hard",
              "detail": "Horizontal overflow: document 431px > viewport 390px"
            }
          ],
          "console": [],
          "page_errors": [],
          "requests": [
            { "url": "http://localhost:5173/dashboard", "method": "GET", "status": 200, "failed": false, "failure_text": null, "response_size": 4521 },
            { "url": "http://localhost:5173/api/projects", "method": "POST", "status": 500, "failed": true, "failure_text": null, "response_size": 42 }
          ],
          "layout": {
            "viewport_width": 390,
            "viewport_height": 844,
            "document_width": 431,
            "document_height": 1600,
            "horizontal_overflow": true,
            "clipped_text_candidates": [
              {
                "text": "Demo Project — A new project for testing the creation flow with a longer description...",
                "rect": { "x": 16, "y": 320, "width": 358, "height": 48 },
                "scroll_width": 420,
                "client_width": 358,
                "scroll_height": 48,
                "client_height": 48
              }
            ],
            "overlapping_text_candidates": []
          },
          "accessibility": {
            "snapshot_path": "mobile/create-project/a11y.json",
            "violations_path": null,
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
    "video": null,
    "screenshots": [
      "desktop/create-project/after-create.png",
      "desktop/create-project/final.png",
      "mobile/create-project/after-create.png",
      "mobile/create-project/final.png"
    ]
  }
}
```

---

## 47. BrowserRunResult — passing (result.json)

After the agent fixes the overflow and API, attempt 3 passes:

```json
{
  "schema_version": 1,
  "run_id": "b8c4d2e3-5f60-7890-bcde-f01234567890",
  "plan_id": "frontend-plan",
  "task_id": "F3",
  "attempt": 3,
  "backend": "playwright-chromium",
  "started_at": "2026-04-25T11:38:00Z",
  "duration_ms": 14201,
  "passed": true,
  "summary": "All journeys passed across 2 viewports",
  "failure_classes": [],
  "viewports": [
    {
      "name": "desktop",
      "width": 1440,
      "height": 900,
      "journeys": [
        {
          "id": "create-project",
          "passed": true,
          "final_url": "http://localhost:5173/dashboard",
          "screenshots": ["desktop/create-project/after-create.png", "desktop/create-project/final.png"],
          "steps": [
            { "index": 0, "action": "click", "success": true, "duration_ms": 298, "error": null },
            { "index": 1, "action": "fill", "success": true, "duration_ms": 156, "error": null },
            { "index": 2, "action": "click", "success": true, "duration_ms": 1102, "error": null },
            { "index": 3, "action": "wait_for_text", "success": true, "duration_ms": 1890, "error": null },
            { "index": 4, "action": "screenshot", "success": true, "duration_ms": 78, "error": null }
          ],
          "assertions": [
            { "name": "text_visible:Demo Project", "passed": true, "severity": "hard", "detail": null },
            { "name": "no_console_errors", "passed": true, "severity": "hard", "detail": null },
            { "name": "no_failed_requests", "passed": true, "severity": "hard", "detail": null },
            { "name": "no_horizontal_overflow", "passed": true, "severity": "hard", "detail": null }
          ],
          "console": [],
          "page_errors": [],
          "requests": [
            { "url": "http://localhost:5173/dashboard", "method": "GET", "status": 200, "failed": false, "failure_text": null, "response_size": 4521 },
            { "url": "http://localhost:5173/api/projects", "method": "POST", "status": 201, "failed": false, "failure_text": null, "response_size": 89 },
            { "url": "http://localhost:5173/api/projects", "method": "GET", "status": 200, "failed": false, "failure_text": null, "response_size": 412 }
          ],
          "layout": {
            "viewport_width": 1440, "viewport_height": 900,
            "document_width": 1440, "document_height": 1100,
            "horizontal_overflow": false,
            "clipped_text_candidates": [],
            "overlapping_text_candidates": []
          },
          "accessibility": { "snapshot_path": "desktop/create-project/a11y.json", "violations_path": null, "critical": 0, "serious": 0 }
        }
      ]
    },
    {
      "name": "mobile",
      "width": 390,
      "height": 844,
      "journeys": [
        {
          "id": "create-project",
          "passed": true,
          "final_url": "http://localhost:5173/dashboard",
          "screenshots": ["mobile/create-project/after-create.png", "mobile/create-project/final.png"],
          "steps": [
            { "index": 0, "action": "click", "success": true, "duration_ms": 412, "error": null },
            { "index": 1, "action": "fill", "success": true, "duration_ms": 189, "error": null },
            { "index": 2, "action": "click", "success": true, "duration_ms": 1345, "error": null },
            { "index": 3, "action": "wait_for_text", "success": true, "duration_ms": 2156, "error": null },
            { "index": 4, "action": "screenshot", "success": true, "duration_ms": 95, "error": null }
          ],
          "assertions": [
            { "name": "text_visible:Demo Project", "passed": true, "severity": "hard", "detail": null },
            { "name": "no_console_errors", "passed": true, "severity": "hard", "detail": null },
            { "name": "no_failed_requests", "passed": true, "severity": "hard", "detail": null },
            { "name": "no_horizontal_overflow", "passed": true, "severity": "hard", "detail": null }
          ],
          "console": [],
          "page_errors": [],
          "requests": [
            { "url": "http://localhost:5173/dashboard", "method": "GET", "status": 200, "failed": false, "failure_text": null, "response_size": 4521 },
            { "url": "http://localhost:5173/api/projects", "method": "POST", "status": 201, "failed": false, "failure_text": null, "response_size": 89 },
            { "url": "http://localhost:5173/api/projects", "method": "GET", "status": 200, "failed": false, "failure_text": null, "response_size": 412 }
          ],
          "layout": {
            "viewport_width": 390, "viewport_height": 844,
            "document_width": 390, "document_height": 1400,
            "horizontal_overflow": false,
            "clipped_text_candidates": [],
            "overlapping_text_candidates": []
          },
          "accessibility": { "snapshot_path": "mobile/create-project/a11y.json", "violations_path": null, "critical": 0, "serious": 0 }
        }
      ]
    }
  ],
  "artifacts": {
    "trace": "trace.zip",
    "har": "network.har",
    "video": null,
    "screenshots": [
      "desktop/create-project/after-create.png", "desktop/create-project/final.png",
      "mobile/create-project/after-create.png", "mobile/create-project/final.png"
    ]
  }
}
```

---

## 48. VisualEvalResult (vision-eval.json)

The visual evaluator sees the passing screenshots and scores them:

```json
{
  "schema_version": 1,
  "model": "claude-sonnet-4-6",
  "score": 8.4,
  "passed": true,
  "threshold": 7.0,
  "confidence": 0.85,
  "rubric_scores": {
    "task_completion": 9.0,
    "layout_integrity": 8.5,
    "responsive_quality": 8.0,
    "interaction_clarity": 8.0,
    "visual_polish": 7.5,
    "design_system_fit": 8.0,
    "accessibility_affordance": 8.5
  },
  "summary": "The modal flow works correctly. All four stat cards render. The form submits and the new project appears in the list. Minor spacing inconsistency between the modal header and body could be tightened.",
  "findings": [
    {
      "severity": "low",
      "viewport": "desktop",
      "journey_id": "create-project",
      "screenshot": "desktop/create-project/after-create.png",
      "area": "modal header",
      "problem": "Gap between modal title and form body is slightly larger than the gap between form fields.",
      "evidence": "Visual measurement shows ~24px gap above the form vs ~16px between fields.",
      "suggested_fix": "Reduce the top padding or margin of the form container to match the inter-field gap.",
      "dimension": "visual_polish",
      "selector": ".modal-body"
    }
  ],
  "screenshot": "desktop/create-project/after-create.png",
  "viewport": "desktop",
  "journey_id": "create-project"
}
```

---

## 49. Retry feedback (feedback.md)

When attempt 2 fails, this markdown is injected into the agent's retry prompt:

```markdown
## UI Gate Failure (attempt 2/3)

Task: F3 Build project creation modal

### Hard failures
- **Runtime**: mobile/create-project: 1 failed request(s): POST http://localhost:5173/api/projects → 500
- **Layout**: mobile/create-project: Horizontal overflow: document 431px > viewport 390px

### Soft findings
- **low** (layout_correctness): mobile/create-project: Clipped text in project card description

### Browser evidence
- Console errors: 0
- Page errors: 0
- Failed requests: 1 (POST /api/projects → 500)
- Layout: horizontal overflow on mobile (431 > 390), 1 clipped text
- Screenshots:
  - .roko/ui-runs/F3/002/desktop/create-project/after-create.png
  - .roko/ui-runs/F3/002/mobile/create-project/after-create.png
- Trace: .roko/ui-runs/F3/002/trace.zip

### What to fix
1. Fix the POST /api/projects endpoint to return a success response on mobile. The same endpoint works on desktop — check whether the mobile request sends different headers or body.
2. Remove horizontal overflow at 390×844 mobile viewport. The modal or its contents are 431px wide — check for fixed-width elements or unresponsive padding in the modal or dashboard layout.
3. Do not regress desktop behavior (desktop passes all checks).
```

---

## 50. UiRunSummary (summary.json)

After all attempts complete:

```json
{
  "task_id": "F3",
  "attempt_count": 3,
  "passed": true,
  "attempts": [
    {
      "attempt": 1,
      "passed": false,
      "failure_tier": 3,
      "failure_classes": ["failed_request"],
      "visual_score": null,
      "hard_failure_count": 1,
      "soft_finding_count": 0,
      "duration_ms": 12340,
      "timestamp": "2026-04-25T11:28:00Z"
    },
    {
      "attempt": 2,
      "passed": false,
      "failure_tier": 4,
      "failure_classes": ["layout_overflow", "failed_request"],
      "visual_score": null,
      "hard_failure_count": 2,
      "soft_finding_count": 1,
      "duration_ms": 18423,
      "timestamp": "2026-04-25T11:32:00Z"
    },
    {
      "attempt": 3,
      "passed": true,
      "failure_tier": null,
      "failure_classes": [],
      "visual_score": 8.4,
      "hard_failure_count": 0,
      "soft_finding_count": 1,
      "duration_ms": 14201,
      "timestamp": "2026-04-25T11:38:00Z"
    }
  ]
}
```

---

## 51. Learning event

Emitted after each UI gate run, consumed by CascadeRouter, ExperimentStore, and adaptive thresholds:

```json
{
  "event": "ui_gate_result",
  "task_id": "F3",
  "plan_id": "frontend-plan",
  "attempt": 3,
  "passed": true,
  "visual_score": 8.4,
  "failure_tier": null,
  "failure_classes": [],
  "hard_failure_count": 0,
  "soft_finding_count": 1,
  "duration_ms": 14201,
  "model_implementer": "claude-opus-4-6",
  "model_evaluator": "claude-sonnet-4-6",
  "prompt_variant": "ui-basic",
  "viewports_tested": ["desktop", "mobile"],
  "journeys_tested": ["create-project"],
  "cost_usd": 0.042,
  "timestamp": "2026-04-25T11:38:00Z"
}
```

---

## 52. Artifact directory listing

After all 3 attempts, the artifact directory looks like:

```
.roko/ui-runs/F3/
  001/
    spec.json
    result.json
    feedback.md
    trace.zip
    network.har
    console.json
    requests.json
    page-errors.json
    desktop/
      create-project/
        final.png
        a11y.json
        layout.json
    mobile/
      create-project/
        final.png
        a11y.json
        layout.json
  002/
    spec.json
    result.json
    feedback.md
    trace.zip
    network.har
    console.json
    requests.json
    page-errors.json
    desktop/
      create-project/
        after-create.png
        final.png
        a11y.json
        layout.json
    mobile/
      create-project/
        after-create.png
        final.png
        a11y.json
        layout.json
  003/
    spec.json
    result.json
    trace.zip
    network.har
    console.json
    requests.json
    page-errors.json
    desktop/
      create-project/
        after-create.png
        final.png
        a11y.json
        layout.json
    mobile/
      create-project/
        after-create.png
        final.png
        a11y.json
        layout.json
    eval/
      desktop-create-project-after-create.json
      mobile-create-project-after-create.json
  verdict.json
  summary.json
```

Note: `feedback.md` is only present in failing attempts (001, 002). `eval/` is only present in the attempt where tier 5 ran (003). `verdict.json` always reflects the most recent attempt.

---

## 53. End-to-end flow summary

```
Step  1: Orchestrator reads task F3 from tasks.toml. Has [task.ui]. ✓
Step  2: 9-layer SystemPromptBuilder assembles implementer prompt.
Step  3: CascadeRouter picks claude-opus-4-6 for this integrative task.
Step  4: Agent runs, creates 3 files, writes React components.
Step  5: CompileGate (rung 0) → pass.
Step  6: LintGate (rung 1) → pass.
Step  7: TestGate (rung 2) → pass.
Step  8: UiGate starts. Dev server spawned (npm run dev).
Step  9: Polls http://localhost:5173 every 500ms. HTTP 200 after 3s.
Step 10: Browser runner spawned: node tools/roko-ui-runner.mjs --spec ...
Step 11: Runner executes journey in desktop viewport. Steps pass.
Step 12: Runner executes journey in mobile viewport. POST /api/projects → 500.
Step 13: Runner writes result.json (passed: false, failure: failed_request).
Step 14: UiGate reads result.json. Tier 3 hard failure. Returns Verdict::fail.
Step 15: Dev server killed (RAII guard drops).
Step 16: Orchestrator writes artifacts to .roko/ui-runs/F3/001/.
Step 17: Orchestrator formats retry feedback.
Step 18: Agent retries with feedback in prompt.
Step 19: Agent fixes API mock. Also introduces CSS that causes mobile overflow.
Step 20: Code gates pass again.
Step 21: UiGate runs. Desktop passes. Mobile: POST 201 ✓, but overflow 431 > 390.
Step 22: Tier 4 hard failure. Verdict::fail.
Step 23: Retry feedback includes both desktop success and mobile overflow.
Step 24: Agent fixes modal width. Removes fixed-width padding.
Step 25: Code gates pass.
Step 26: UiGate runs. Both viewports: all steps pass, all assertions pass.
Step 27: Tier 5: Visual evaluator receives 4 screenshots.
Step 28: Evaluator scores 8.4/10 (above 7.0 threshold). 1 soft finding.
Step 29: UiGate returns Verdict::pass with score 0.84.
Step 30: Dev server killed.
Step 31: Artifacts stored in .roko/ui-runs/F3/003/.
Step 32: Learning event emitted (model, score, attempts, cost).
Step 33: CascadeRouter records: opus succeeded on UI task in 3 attempts.
Step 34: Episode logged to .roko/episodes.jsonl.
Step 35: Dashboard shows F3 as passed, visual score 8.4, 3 attempts.
```
