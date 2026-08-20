# 153 — Automated Visual Assessment Loop

**Priority**: P3 — capstone integration that ties screenshot capture, comparison, and plan generation into a self-improving visual quality loop
**Size**: L (3-5 days)
**Crates**: `crates/roko-cli/src/commands/screenshot.rs`, `crates/roko-cli/src/runner/event_loop.rs`
**Depends on**: #111 (screenshot command), #112 (continuous screenshots), #151 (PNG rendering), #152 (screenshot diff engine)
**Sources**: `tmp/mori-old/IMPLEMENTATION-CHECKLIST.md` S4.3

---

## Background

Roko is a self-developing system: it reads PRDs, generates implementation plans, executes tasks via LLM agents, validates with gates, and learns from outcomes. The TUI is one of its primary surfaces, and the predecessor system (Mori) had a polished visual design (ROSEDUST palette, information-dense header, plan tree widget, error digest panel).

Individual pieces exist for visual self-assessment:
- `roko screenshot` (#111) captures headless TUI snapshots
- `roko plan run --screenshots` (#112) captures during execution
- PNG rendering (#151) enables pixel-level visual inspection
- Screenshot diff engine (#152) compares snapshots

What is missing is the orchestrated loop that ties these together: capture current state, assess against Mori reference, identify gaps, generate fix plans, execute, and verify improvement. This spec describes that end-to-end workflow as a CLI command and a documented recipe.

## Current State

- `roko screenshot` captures TUI tab snapshots headlessly (text; PNG via #151).
- `roko screenshot --compare` compares snapshots against a reference directory (#152).
- `roko prd idea/draft/plan` generates implementation plans from descriptions.
- `roko plan run --express --screenshots` executes plans with auto-fix and continuous screenshots.
- Mori reference screenshots at `tmp/mori-old/screenshots/` (17 PNGs) with detailed text descriptions in `tmp/mori-old/MORI-TUI-SCREENSHOTS.md`.
- No existing command or workflow ties these together automatically.

## Implementation Plan

1. **Add `roko screenshot assess` subcommand**: A single command that runs the full assessment loop against Mori reference screenshots:
   ```
   roko screenshot assess                              # compare against mori references
   roko screenshot assess --reference <dir>            # compare against custom reference
   roko screenshot assess --generate-plan              # also generate a PRD for identified gaps
   ```

   How it works:
   a. Capture current TUI state via `roko screenshot`.
   b. Run `--compare` against reference directory (default: `tmp/mori-old/screenshots/` with `--reference-mode`).
   c. Parse the diff report to identify visual gaps.
   d. Generate a structured assessment report listing gaps by category:
      - Color palette compliance (comparing against ROSEDUST spec).
      - Layout structure (widget presence/absence/positioning).
      - Information density (text content per screen area).
      - Interactivity cues (keybind hints, selection indicators).
   e. When `--generate-plan` is set, create a PRD idea for each identified gap via `roko prd idea`.

2. **Assessment report format**: Write to `<output-dir>/assessment-report.json`:
   ```json
   {
     "timestamp": "2026-08-19T14:00:00Z",
     "reference": "tmp/mori-old/screenshots/",
     "overall_similarity": 72.5,
     "gaps": [
       {
         "category": "color_palette",
         "severity": "high",
         "description": "F1 dashboard uses default grey background instead of ROSEDUST warm rose",
         "tabs_affected": ["f01-dashboard", "f02-plans"],
         "suggested_fix": "Port ROSEDUST palette (backlog #123)"
       }
     ],
     "prd_ideas_created": ["port-rosedust-palette", "add-plan-tree-widget"]
   }
   ```

3. **Post-plan-run assessment hook**: Add an optional `--assess-after` flag to `roko plan run` that automatically runs the visual assessment after all plans complete:
   ```
   roko plan run plans/tui-fixes/ --screenshots --assess-after
   ```
   This captures a baseline at start, runs the plans, captures after, runs assessment, and writes a before/after comparison report.

4. **Assessment comparison report**: When `--assess-after` is used, generate a `before-after-report.json` showing improvement metrics:
   ```json
   {
     "before_similarity": 65.2,
     "after_similarity": 78.8,
     "improvement": 13.6,
     "gaps_resolved": ["color_palette"],
     "gaps_remaining": ["plan_tree_widget", "error_digest"]
   }
   ```

5. **Document the manual loop recipe**: Create a runbook section in the spec documenting how Claude can run the assessment loop manually step by step using existing CLI commands, for cases where the automated command is not sufficient.

## Acceptance Criteria

1. `roko screenshot assess` produces an assessment report comparing current TUI to Mori references.
2. The assessment report categorizes gaps (palette, layout, density, interactivity).
3. `--generate-plan` creates PRD ideas for identified gaps.
4. `roko plan run --assess-after` produces a before/after comparison report.
5. The improvement delta (before vs after similarity) is calculated and reported.
6. The manual loop recipe is documented and executable step-by-step.

## Verification Checklist

- [ ] `roko screenshot assess` runs without error and produces `assessment-report.json`.
- [ ] Assessment identifies at least one gap when TUI differs from Mori screenshots.
- [ ] `--generate-plan` creates at least one PRD idea from the assessment.
- [ ] `roko plan run --assess-after` captures before/after screenshots and generates comparison.
- [ ] The manual recipe can be followed by a human or agent to complete a full assessment cycle.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/commands/screenshot.rs` | Add `assess` subcommand, `--generate-plan`, `--reference` |
| `crates/roko-cli/src/tui/snapshot.rs` | Add `run_assessment()`, `AssessmentReport` struct |
| `crates/roko-cli/src/runner/event_loop.rs` | Add `--assess-after` flag handling at plan completion |
| `crates/roko-cli/src/commands/plan.rs` | Wire `--assess-after` into plan run CLI |
