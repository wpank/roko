# Visual gate and browser feedback loop for Roko agents

## Status

Draft

## Audience

Implementation agent with no prior Roko context. You do not have access to the Roko repository. Everything you need is in these documents.

## What this is

These documents specify a visual verification gate for Roko — a system that lets AI agents implement frontend tasks, run the result in a real browser, capture functional and visual evidence, judge quality, and retry with actionable feedback until the UI passes.

The gate fits into Roko's existing gate pipeline. Roko is a Rust toolkit for building agents that build themselves. Its core loop runs: query → score → route → compose → act → verify → write → react. The "verify" step is where gates live. This feature adds a new gate — `UiGate` — that verifies frontend work by rendering it and looking at it.

## Documents

Read these in order. Each builds on the previous.

| # | File | What it covers | ~Lines |
|---|------|----------------|--------|
| 01 | `01-roko-architecture.md` | What Roko is, the Engram type, six verb traits with exact signatures, universal loop, gate system (Verdict, 7-rung pipeline, GatePipeline, adaptive thresholds, ratchet, replan-on-failure), orchestration loop (plans, tasks, dispatch, learning), how to write a new gate | ~930 |
| 02 | `02-feature-spec.md` | Problem statement, goals, design principle (deterministic first), complete data model (UiTaskSpec, UiViewport, UiJourney, UiStep with 13 variants, UiAssertion with 20+ variants, UiScreenshotPolicy, UiArtifactRetention), global config with security section, browser backend strategy (BrowserBackend trait, Obscura position), browser runner contract (BrowserRunSpec, BrowserRunResult with per-viewport per-journey structure, all evidence types, UiFailureClass enum), visual evaluator (7-dimension rubric, VisualEvalResult), UiGate verify() flow with complete pseudocode, pass/fail semantics, retry feedback format with 3 examples, artifact layout with retention modes | ~1800 |
| 03 | `03-implementation.md` | Implementation responsibilities organized by component (task parser, UiGate, browser runner, visual evaluator, orchestrator wiring, HTTP routes, dashboard, learning), 9 cybernetic feedback loops with sensor/comparator/controller/actuator pattern, metrics, test strategy (unit, runner, integration, golden), security and safety (sandboxing, redaction, network policy), MVP task breakdown (VG-01 through VG-09 with dependency graph), acceptance criteria, rollout plan, risks, open questions, definition of done | ~1300 |
| 04 | `04-browser-runner.md` | Complete standalone spec for the Node.js Playwright runner: invocation modes, implementation outline with code, viewport execution, journey execution, step execution for all 13 step types, locator resolution order, layout collector JS snippet, accessibility collection, assertion evaluation for all assertion types, secret redaction, error handling, dependencies | ~800 |
| 05 | `05-worked-examples.md` | Complete JSON payloads: task TOML, BrowserRunSpec, BrowserRunResult (failing), BrowserRunResult (passing), VisualEvalResult, retry feedback, UiRunSummary, learning event, artifact directory listing, 35-step end-to-end flow summary | ~700 |

## Conventions

These hold across all documents.

**Types.** All Rust types are defined inline. You do not need to look them up elsewhere. Code blocks marked "Conceptual Rust" show the intended type shape — they may not compile as-is but define the contract.

**Config.** TOML blocks show configuration format for `roko.toml`.

**Key terms:**

- **Engram** — the universal data type (a content-addressed, scored, decaying signal). Defined in `01-roko-architecture.md`.
- **Verdict** — the output of any gate (pass/fail + score + detail). Defined in `01-roko-architecture.md`.
- **Gate** — the trait you will implement. Defined in `01-roko-architecture.md`.
- **Hard failure** — the gate returns `passed: false` immediately. No further checks run.
- **Soft failure** — the finding is recorded but does not fail the gate on its own.
- **Tier** — one of 5 verification levels: Infrastructure (1), Functional (2), Runtime (3), Layout (4), Visual (5).
- **Rung** — a position in the gate pipeline. Existing rungs are 0–6. UiGate is rung 7+.
- **Journey** — an ordered sequence of user interactions (click, fill, screenshot, assert).

## What you are building

A new `UiGate` that implements the `Gate` trait, plus seven supporting pieces:

1. **Task parser extension** — reads `[ui]` sections from task TOML so agents know what to verify
2. **Browser runner** — a Node.js/Playwright subprocess that executes UI journeys, captures screenshots, collects console logs, network requests, layout metrics, accessibility snapshots
3. **Visual evaluator** — sends screenshots + context to an LLM for scoring against a 7-dimension rubric
4. **Orchestration wiring** — rung placement in the gate pipeline, retry feedback injection, artifact storage
5. **HTTP route extensions** — new endpoints on the control plane for triggering and inspecting visual gate results
6. **Config schema extensions** — new sections in `roko.toml` for browser runner settings, visual thresholds, security
7. **Learning integration** — emit events for model routing, prompt experiments, threshold tuning

## Primary outcome

When this feature is complete, Roko agents can:

implement a frontend task → render it in a real browser → interact with UI elements → capture screenshots and browser evidence → judge functional correctness and visual quality → fail with specific, actionable feedback → retry with that feedback → pass when hard criteria are met and visual score exceeds threshold → store artifacts for future learning.

That is the full loop. The documents tell you how to build each part.

## Where to start

1. Read `01-roko-architecture.md` first. It gives you enough Roko context to understand the gate system and where UiGate plugs in.
2. Read `02-feature-spec.md` for the complete data model, contracts, and the UiGate verify() flow.
3. Read `04-browser-runner.md` for the Node.js runner implementation details.
4. Read `03-implementation.md` for the task breakdown and build order.
5. Read `05-worked-examples.md` to see complete JSON payloads flowing through the system.
