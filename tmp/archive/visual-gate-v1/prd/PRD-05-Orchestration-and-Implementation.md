# PRD-05: Orchestration, Eight Disjoint Loops, Implementation Plan, and Rollout

**Prerequisites**: PRD-00 through PRD-04.

---

## 1. Overview

How everything wires together. Gate pipeline placement, retry feedback flow, eight disjoint cybernetic loops with conjunctive/Pareto gate composition per Ashby's Law, HTTP routes, dashboard, test strategy, MVP task breakdown with dependencies, rollout plan, acceptance criteria, risks, and open questions.

---

## 2. Gate Pipeline Wiring

### 2.1 Rung Placement

UiGate is rung 7+. Runs after all code-level gates:

```
Rung 0: CompileGate
Rung 1: ClippyGate (lint)
Rung 2: TestGate
Rung 3: SymbolGate
Rung 4: GeneratedTestGate
Rung 5: PropertyTestGate / FactCheckGate
Rung 6: IntegrationGate / LlmJudgeGate
Rung 7: UiGate (hard gates: Tiers 1–4)
Rung 8: UiVisualGate (soft gates: Tier 5 computational + judge panel)
```

Splitting into two rungs allows the pipeline to short-circuit: if hard browser checks fail (rung 7), skip the expensive visual evaluation (rung 8).

### 2.2 Rung Selection Logic

```rust
// In select_rungs():
if task.ui.is_some() && config.gates.ui.enabled {
    selected.push(Rung::UiHard);     // Tiers 1-4
    if config.gates.ui.judge_panel.models.len() > 0 {
        selected.push(Rung::UiVisual); // Tier 5
    }
}

// In run_rung():
Rung::UiHard => {
    let gate = UiGate::new(config.gates.ui.clone());
    gate.verify_hard(engram, ctx).await
}
Rung::UiVisual => {
    let gate = UiVisualGate::new(config.gates.ui.clone());
    gate.verify_visual(engram, ctx).await
}
```

### 2.3 Orchestrator Integration

```rust
// In the main task execution loop, after agent completes:
if let Some(ui_spec) = &task.ui {
    // 1. Start dev server (RAII guard — killed on drop)
    let _server = DevServerHandle::start(&ui_spec.dev_server, ...)?;

    // 2. Build BrowserRunSpec from task + config
    let spec = build_run_spec(ui_spec, &config.gates.ui, &task);

    // 3. Run browser runner → BrowserRunResult
    let browser_result = backend.run(spec).await?;

    // 4. Compute computational metrics (15 metrics)
    let metrics = compute_metrics(&browser_result, &config.gates.ui.metrics);

    // 5. Run hard gate evaluation (Tiers 1-4)
    let hard_verdict = evaluate_hard_gates(&browser_result, &metrics, &config);

    // 6. If hard gates failed, emit feedback and stop
    if !hard_verdict.passed {
        let feedback = build_feedback(&browser_result, &metrics, None, &hard_verdict);
        persist_artifacts(&task, attempt, &browser_result, &metrics, &feedback);
        emit_learning_event(&task, attempt, &hard_verdict, ...);
        inject_feedback_into_retry(&mut retry_prompt, &feedback);
        return hard_verdict;
    }

    // 7. Run visual regression (odiff + dssim) if golden screenshots exist
    let regression = run_visual_regression(&browser_result, &ui_spec.golden_screenshots);

    // 8. Run judge panel (Tier 5) — pairwise BT comparison
    let panel_result = run_judge_panel(
        &browser_result.screenshots(),
        get_anchor_screenshot(&task),
        &config.gates.ui.judge_panel,
        &ui_spec, &task, &metrics,
    ).await;

    // 9. Compose final verdict (conjunctive hard, Pareto soft)
    let verdict = compose_verdict(&hard_verdict, &metrics, &panel_result, &regression);

    // 10. Persist everything
    persist_artifacts(&task, attempt, &browser_result, &metrics,
                      &panel_result, &regression, &verdict);

    // 11. Emit flywheel artifacts
    emit_preference_triples(&panel_result, &task);
    emit_learning_event(&task, attempt, &verdict, ...);
    emit_tensorzero_trace(&task, attempt, ...);

    // 12. If failed, build and inject retry feedback
    if !verdict.passed {
        let feedback = build_feedback(&browser_result, &metrics,
                                      Some(&panel_result), &verdict);
        inject_feedback_into_retry(&mut retry_prompt, &feedback);
    }

    // 13. Update dashboard
    emit_server_event(ServerEvent::UiGateCompleted { ... });

    return verdict;
}
```

### 2.4 Retry Feedback Injection

```rust
fn inject_feedback_into_retry(prompt: &mut Prompt, feedback: &UiFeedback) {
    // Per Liu et al. "Lost in the Middle" (TACL 2024):
    // Put actionable items at the BEGINNING (highest attention)
    prompt.prepend_section("required_fixes", &feedback.what_to_fix);

    // Evidence in the middle
    prompt.add_section("ui_gate_evidence", &feedback.evidence_summary);

    // Screenshot paths as references
    for path in &feedback.screenshot_paths {
        prompt.add_artifact_reference("screenshot", path);
    }

    // For vision-capable models: embed screenshot as base64
    if model_supports_vision(current_model) && feedback.screenshot_paths.len() <= 2 {
        for path in &feedback.screenshot_paths {
            prompt.embed_image(path);
        }
    }
}
```

---

## 3. Eight Disjoint Cybernetic Loops

**Source**: Ashby's Law of Requisite Variety — V(R) ≥ V(D), the regulator's variety must equal or exceed the disturbance's. Single-signal eval has insufficient regulator variety against the manifold of bad UIs. Conant-Ashby's good-regulator theorem: every good regulator must be a model of the system. Ensembling partial models approximates a sufficient one.

Eight loops in parallel, each with own sensor/comparator/controller/gate. **No signal sharing between loops.** Gate composition: conjunctive on hard gates, Pareto on soft. Never weighted-sum (per Moskovitz et al. ICLR 2024).

### Loop A — Tokens/Static (Hard Gate)

**Sensor**: DOM + tokens.json adherence, stylelint violations.
**Comparator**: Token coverage ≥ 0.9. Token adherence score ≥ threshold.
**Controller**: Token violation reporter.
**Actuator**: Feedback lists specific elements with wrong token values.
**Gate type**: Hard. Failure blocks.

### Loop B — Accessibility (Hard Gate, Tiers 1+2)

**Sensor**: axe-core (max config) + IBM achecker + tab-order graph + focus-visible contrast.
**Comparator**: Critical/serious violations = 0. Tab order complete, no traps. DOM order ≈ visual order (Levenshtein).
**Controller**: Accessibility violation reporter (EARL+JSON-LD format).
**Actuator**: Feedback includes specific selectors, WCAG success criteria, and fix hints.
**Gate type**: Hard. Failure blocks.

### Loop C — Layout Metrics (Soft, Threshold Band)

**Sensor**: AIM Feature Congestion, Grid Quality, alignment score, grid adherence, modular scale, element density, text/whitespace ratio, visual balance.
**Comparator**: Each metric has an independent threshold band. Not weighted-sum.
**Controller**: Layout metric analyzer.
**Actuator**: Feedback identifies specific elements violating layout constraints.
**Gate type**: Soft, Pareto frontier. No single metric compensates for another.

### Loop D — Visual Regression (Soft, Reference-Anchored)

**Sensor**: odiff diff percentage + dssim score vs golden screenshots.
**Comparator**: odiff < 0.1% OR (odiff > 0.1% AND dssim < 0.01 → noise → pass).
**Controller**: Regression analyzer.
**Actuator**: Diff image highlights changed regions.
**Gate type**: Soft. Only applicable when golden references exist.

### Loop E — Saliency Intent (Soft)

**Sensor**: DeepGaze IIE + UMSI++ ensemble saliency map.
**Comparator**: Primary CTA in top-3 saliency peaks (S_cta > 0.7). Banner-blindness check.
**Controller**: Saliency analyzer.
**Actuator**: Feedback shows what draws attention vs what should draw attention.
**Gate type**: Soft. Expensive (GPU inference). Enable post-MVP.

### Loop F — Heuristic LLM Judge (Soft, Ensemble Vote)

**Sensor**: Disjoint-family panel (Claude Opus + LLaVA-Critic-72B + Prometheus-Vision).
**Comparator**: BT score against anchor. Rubric scores per dimension.
**Controller**: Panel aggregator with trimmed-mean, position-swap-and-discard.
**Actuator**: Bounding-box-grounded findings with fix suggestions.
**Gate type**: Soft, but contributes to Pareto.

### Loop G — Behavioral Probe (Hard Gate)

**Sensor**: Playwright journey execution. Steps succeed/fail. Assertions pass/fail.
**Comparator**: All steps succeed. All hard assertions pass.
**Controller**: Journey result analyzer.
**Actuator**: Feedback identifies specific failed steps and assertions.
**Gate type**: Hard. Failure blocks.

### Loop H — Performance (Hard Gate, Tier 3+5)

**Sensor**: LHCI 5-run median + LoAF-instrumented INP + reduced-motion differential.
**Comparator**: LCP ≤ 2500ms, CLS ≤ 0.10, TBT ≤ 200ms. Reduced-motion variant differs from default.
**Controller**: Performance analyzer.
**Actuator**: Feedback identifies specific performance bottlenecks.
**Gate type**: Hard (Tier 3). Warn-then-promote on Tier 5 (reduced-motion).

### Composition Rule

```
overall_pass = (A.passed AND B.passed AND G.passed AND H.passed)  // conjunctive hard
               AND pareto_acceptable(C, D, E, F)                   // Pareto soft

pareto_acceptable(loops):
    for each soft loop:
        if loop.score < loop.floor_threshold:
            return false
    if F.bt_score < target_bt_score:
        return false
    return true
```

---

## 4. HTTP Routes and Dashboard

### 4.1 Routes

```
POST /api/ui-gate/run                             # Manual trigger
GET  /api/ui-gate/runs                            # List runs
GET  /api/ui-gate/runs/{run_id}                   # Get result
GET  /api/ui-gate/runs/{run_id}/artifacts/{path}  # Serve artifact
POST /api/ui-gate/runs/{run_id}/label             # Human label
GET  /api/ui-gate/metrics                          # Aggregate metrics
GET  /api/ui-gate/canary                          # Canary set status
```

### 4.2 Server Events

```rust
ServerEvent::UiGateStarted { plan_id, task_id, run_id }
ServerEvent::UiGateHardComplete { plan_id, task_id, run_id, passed, tier_results }
ServerEvent::UiGateMetrics { plan_id, task_id, run_id, metrics_summary }
ServerEvent::UiGatePanelComplete { plan_id, task_id, run_id, bt_score, preferred }
ServerEvent::UiGateCompleted { plan_id, task_id, run_id, passed, score }
ServerEvent::UiGateArtifact { plan_id, task_id, run_id, kind, path }
```

### 4.3 Dashboard Display

TUI shows: pass/fail, BT score, worst hard gate tier, worst computational metric, top failure class, retry score history (e.g., "--- → 6.9 → 8.8 ✓"), artifact paths. Web dashboard additionally renders screenshots inline and saliency heatmaps.

---

## 5. Test Strategy

### 5.1 Unit Tests

- Parse valid/invalid `[task.ui]` TOML variants
- All 22 assertion types serialize/deserialize correctly
- BrowserRunResult → Verdict conversion for all failure modes
- UiFailureClass classification from result data
- Feedback markdown generation with truncation rules
- Secret redaction correctness
- Token adherence scoring algorithm with known inputs
- APCA Lc computation for known color pairs
- BT MLE computation for known preference data
- Trimmed-mean aggregation
- Position-swap consistency detection

### 5.2 Runner Tests

Fixture app at `tests/fixtures/ui-app/`:
- Passing click flow
- Missing locator → step failure
- Console error → NoConsoleErrors assertion fails
- Page error → NoPageErrors assertion fails
- Failed request → NoFailedRequests assertion fails
- Horizontal overflow → NoHorizontalOverflow assertion fails
- Clipped modal on mobile → layout metrics detect
- Form submission → text appears
- APCA violation → contrast check detects
- Performance budget exceeded → CWV check detects
- Self-test mode: `node tools/roko-ui-runner.mjs --self-test` exits 0

### 5.3 Integration Tests

- UiGate → runner → pass verdict
- UiGate → runner → hard fail verdict with artifacts
- Orchestrator injects UI feedback into retry prompt
- Judge panel runs with position swap and produces consistent result
- Dev server starts and kills correctly (process group)
- Computational metrics computed and persisted
- Learning events written to JSONL
- Dashboard receives SSE events
- Flywheel artifacts emitted (preference triples, traces)

### 5.4 Golden Tests

```
tests/golden/
  spec.json
  result.json
  metrics.json
  hard-gates.json
  panel-result.json
  feedback.md
  preferences.jsonl
  learning-event.json
```

---

## 6. MVP Task Breakdown

### 6.1 Dependency Graph

```
VG-01: Task parser extension (UiTaskSpec types + validation)
  ↓
VG-02: Playwright JSON runner (tools/roko-ui-runner.mjs)
  ↓
VG-03: UiGate hard gates (Tiers 1–4, axe-core, layout, assertions)
  ↓           ↓
VG-04: Wire into orchestrator    VG-05: Feedback compressor
  ↓                                ↓
VG-06: Computational metrics engine (15 metrics, APCA, AIM)
  ↓
VG-07: Token extraction + adherence scoring
  ↓
VG-08: Judge panel (BT aggregation, disjoint families)
  ↓
VG-09: Visual regression (odiff + dssim)
  ↓
VG-10: Persist artifacts + dashboard integration
  ↓
VG-11: Learning events + flywheel step 1 (trace capture)
  ↓
VG-12: Preference mining (flywheel step 3)
  ↓
VG-13: Curriculum-from-failures (flywheel step 5)
  ↓
VG-14: MIPROv2 optimization loop (flywheel step 6)
  ↓
VG-15: Canary set + Krippendorff monitoring
  ↓
VG-16: RFT post-processor training pipeline (flywheel step 7)
```

### 6.2 MVP Scope (VG-01 through VG-05)

The smallest useful slice:
1. Parse `[task.ui]` with all types.
2. Playwright runner executes journeys and captures evidence.
3. Hard gate evaluation (axe-core, console, network, layout).
4. Wire into orchestrator for tasks with `[task.ui]`.
5. Retry feedback from browser evidence.

Visual evaluator, computational metrics, judge panel, token adherence, and flywheel are second+ slices. But all schemas designed from the start.

---

## 7. Rollout Plan

| Stage | What | Gate |
|---|---|---|
| 1 | Manual command only | `roko ui-gate run --spec spec.json` |
| 2 | Opt-in per task | `[task.ui]` present → run |
| 3 | Project-level enable | `[gates.ui] enabled = true` |
| 4 | Computational metrics + APCA | Tiers 1–4 fully operational |
| 5 | Judge panel + BT scoring | Tier 5 operational |
| 6 | Token extraction + adherence | Design system enforcement |
| 7 | Visual regression (odiff) | Change detection |
| 8 | Flywheel steps 1–3 | Traces + preferences |
| 9 | Human calibration + canary | Krippendorff monitoring |
| 10 | Flywheel steps 4–6 | Pattern extraction + MIPROv2 |
| 11 | RFT post-processor | Month 7+ |

---

## 8. Acceptance Criteria

### MVP (Stages 1–3)

1. Task declares `[task.ui]` with viewports, journeys, steps, assertions.
2. Playwright runner executes journeys and captures screenshots, console, network, layout.
3. UiGate returns pass/fail Verdict.
4. Failed gate includes actionable feedback with artifact paths.
5. Orchestrator retries with UI feedback.
6. Artifacts persist under `.roko/ui-runs`.
7. Hard browser failures override any other score.

### Full System (Stages 4–11)

1. 15 computational metrics computed on every render.
2. APCA contrast computed per text element.
3. axe-core + IBM Equal Access both run.
4. Judge panel uses pairwise BT with position swap.
5. Token adherence scored with area-weighted algorithm.
6. Visual regression via odiff + dssim.
7. Core Web Vitals collected via LHCI median-of-5.
8. Reduced-motion compliance tested differentially.
9. Canary set evaluated with Krippendorff α monitoring.
10. Preference triples emitted for BT training pool.
11. Curriculum-from-failures generates synthetic training tasks.
12. RFT post-processor trained on ≥10k repair pairs.

---

## 9. Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Judge panel latency (30–90s) | Slow feedback loop | Skip panel when hard gates fail. Cache anchor screenshots. |
| APCA computation complexity | Composited colors hard to extract | Use CDP compositing APIs. Fall back to WCAG ratio if APCA unavailable. |
| Saliency models need GPU | High infrastructure cost | Defer saliency to post-MVP. CPU fallback with longer timeout. |
| Panel judges disagree fundamentally | Unreliable verdicts | Trimmed mean + position swap. Human review on disagreement. |
| Token adherence Goodharts | Agent emits tokens without semantic correctness | Pair with behavioral assertions. Token type-correctness lint (Kong pattern). |
| Canary set too small | Unreliable Krippendorff α | Start with 200, grow to 500. Minimum 3 annotators per item. |
| RFT data insufficient at month 7 | Post-processor undertrained | Lower threshold to 5k pairs. Use WebDev Arena 10k as supplementary data. |
| odiff baseline drift | False regressions from font rendering changes | Pin container fonts. Re-baseline on Chrome major version upgrade. |

---

## 10. Open Questions

1. Should chromiumoxide replace Playwright Node entirely, or coexist as separate backends?
2. How should the canary set be constructed — manual curation, sampling from production, or synthetic?
3. Should the judge panel run on every attempt or only when computational metrics are within threshold?
4. What's the minimum viable anchor set for bootstrapping BT comparison?
5. Should AIM metrics run in Rust (ported) or as a Python subprocess?
6. How should the system handle tasks where no design tokens file exists?
7. Should reduced-motion compliance be a hard gate from day one or promoted after a grace period?
8. How should preference triples from different sources (panel, user-edit, user-select) be weighted in BT training?

---

## 11. Definition of Done

This system is done when it can: take a frontend task → implement through an agent → render in a real browser → execute user journeys → compute 15 deterministic metrics → evaluate APCA contrast per text element → audit accessibility via axe-core + IBM Equal Access → measure Core Web Vitals → score token adherence against design system → detect visual regression against golden references → run a disjoint-family judge panel with pairwise Bradley-Terry comparison → compose a conjunctive/Pareto verdict → fail with concrete, grounded feedback → retry with that feedback → pass when all hard gates and Pareto soft gates are satisfied → emit labeled artifacts into a self-improving flywheel → mine preference data for BT training → extract AST patterns nightly → generate curriculum from failures → optimize pipeline configuration via MIPROv2 → eventually RFT a post-processor that compounds permanently → and maintain Goodhart resistance through canary monitoring, eval rotation, and adversarial red-teaming.
