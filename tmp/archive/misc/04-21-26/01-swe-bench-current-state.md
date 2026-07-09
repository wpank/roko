# SWE-bench: Current State

## What Exists

### Python Scripts (`crates/roko-cli/scripts/`)

A complete 3-script evaluation pipeline, added in the initial commit and never modified:

| Script | Lines | Role |
|--------|-------|------|
| `swebench_run.py` | 379 | Main harness — loads SWE-bench-Lite from HuggingFace, oracle file retrieval, generates per-task `roko.toml`, runs `roko run`, extracts unified diff, writes `predictions.jsonl` |
| `swebench_baseline.py` | 195 | Control experiment — raw `ollama run` with no roko harness, same oracle retrieval, for A/B testing |
| `swebench_validate.py` | 226 | Local scorer — `git apply --check`, measures format_valid / apply_check / touches_oracle / patch_bytes |
| `test_swebench_run.py` | 232 | Unit tests for pure functions (oracle parsing, patch extraction, config generation) |
| `README.md` | 358 | Full docs with A/B recipe, ablation study methodology, expected score ranges |

### Ablation Flags

The harness has 4 ablation knobs for scientific attribution:
- `--no-clean-output` — disable ANSI + thinking-trace stripping
- `--no-file-injection` — disable `[[prompt.files]]` sections
- `--no-hard-cap` — disable per-file token caps
- `--minimal-role` — use 1-line role instead of structured instructions

### Rust Stubs

- `BenchmarkRegressionGate` in `crates/roko-gate/src/benchmark_gate.rs` — 118 lines, currently a pass-through stub (always passes, no baseline infrastructure)
- `BenchmarkComparison` struct defined but unused
- References in `research.rs` prompts mention SWE-bench as a research source

### Documentation

- `docs/BENCHMARKS.md` (954 lines) — comprehensive evaluation methodology spec covering SWE-bench, internal metrics, external benchmark positioning, anti-gaming architecture
- `docs/02-agents/08-harness-engineering.md` (226 lines) — Meta-Harness thesis, 6 harness principles, SWE-bench context
- `docs/21-references/14-agent-harnesses-and-tool-use.md` — cites Jimenez et al. (2024), SWE-agent, Meta-Harness
- `tmp/run-anywhere/16-benchmarks-and-evals.md` (728 lines) — extended evaluation design with cost modeling, waste ratio, HAL scaffold-aware evaluation

## The Disconnect

The Python scripts treat roko as a **black box** — they shell out to `roko run`, then scrape `signals.jsonl` for output. This means:

1. **No learning feedback** — scripts never call `record_completed_run()`, so CascadeRouter, playbooks, experiments, adaptive thresholds never fire
2. **No gate pipeline** — scripts extract patches and run `git apply --check` outside roko; the 11-gate, 7-rung pipeline is bypassed entirely
3. **No episode logging** — pass/fail per instance never enters `.roko/episodes.jsonl`
4. **Serial, single-shot** — no retries, no replan-on-failure, no prompt experiment variants
5. **Python dependency** — requires `datasets` library from HuggingFace, `swebench` for official scoring

## Learning Infrastructure That Exists But Isn't Used

All of this is wired into `plan run` but never touched by the SWE-bench scripts:

| Loop | Mechanism | Persists to |
|------|-----------|-------------|
| Model routing | LinUCB contextual bandit (CascadeRouter) | `.roko/learn/cascade-router.json` |
| Gate adaptation | EMA + CUSUM per rung | `.roko/learn/gate-thresholds.json` |
| Prompt A/B | UCB1 experiment store | `.roko/learn/experiments.json` |
| Gate failure replan | Auto-replan after 3 failures | PlanRevision event |
| Skill extraction | Every 10 episodes | `.roko/learn/skills.json` |
| Playbook reuse | Success patterns injected into prompts | `.roko/learn/playbooks/` |
| Episode logging | Full turn recording | `.roko/episodes.jsonl` |
| Efficiency metrics | Per-turn tokens, cost, latency, tools | `.roko/learn/efficiency.jsonl` |
| Regression detection | Per-task metric comparison | `.roko/task-metrics.jsonl` |
| Pattern mining | Cross-episode consolidation (every 20 episodes) | Episodes |

## Expected Score Ranges

From the README (these are realistic for the current single-shot, no-tool-use setup):

| Setup | SWE-bench-Lite % Resolved |
|-------|---------------------------|
| Frontier models (Claude 4.5 Sonnet, GPT-5, with tool use) | 55-80% |
| Frontier models (zero-shot, oracle retrieval) | 20-35% |
| Open 70B models (zero-shot, oracle retrieval) | 3-10% |
| Open 7-30B models (zero-shot, oracle retrieval) | 0-3% |
