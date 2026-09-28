+++
id = "gap-e3bfc3"
kind = "gap"
title = "[evals 3.2] Documented performance targets not enforced in CI"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["ci/benches"]
created = 2026-09-02
updated = 2026-09-28
source = "tmp/archive/evals-audit/12-gaps-and-residuals.md#3.2 Performance Targets Not Enforced"
discovered_from = "audit:tmp/archive/evals-audit/12-gaps-and-residuals.md#3.2 Performance Targets Not Enforced"
anchors = ["crates/roko-cli/benches/", "BenchmarkRegressionGate", "scripts/check-metrics.sh"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
HDC search <1ms, demurrage tick <10ms, system prompt assembly <50ms, gate pipeline <5s have no CI assertion; Criterion benches (engram/hdc/tui_data_pipeline) run without threshold comparison.

Imported without verification from:
- `tmp/archive/evals-audit/12-gaps-and-residuals.md#3.2 Performance Targets Not Enforced`

Some cited files are gone: `crates/roko-cli/benches/`.

How to verify: Check .github/workflows and Makefile for bench threshold enforcement.
