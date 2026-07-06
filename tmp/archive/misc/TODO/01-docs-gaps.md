# docs-gaps/ — Gap Checklist Audit

**Directory**: `tmp/docs-gaps/`
**Status**: STALE — code ahead of checkboxes
**Files**: 30 markdown files + index

## Summary

1,437 / 1,690 items checked (**85%**). All P0 and P1 items complete. Code has advanced past what checkboxes reflect — spot-checks found ~7/15 unchecked items are actually implemented.

## Per-File Status

| File | Checked | Unchecked | % | Notes |
|------|---------|-----------|---|-------|
| 01-type-corrections.md | 9 | 0 | 100% | |
| 02-missing-kernel-types.md | 34 | 0 | 100% | |
| 03-trait-migrations.md | 26 | 0 | 100% | |
| 04-config-schema.md | 23 | 0 | 100% | |
| 05-integrations.md | 34 | 0 | 100% | |
| 06-naming-fixes.md | 20 | 0 | 100% | |
| 07-advanced-systems.md | 63 | 12 | 84% | Demurrage trait DONE but unchecked |
| 08-infrastructure.md | 18 | 9 | 67% | |
| 10-orchestration.md | 64 | 5 | 93% | verify_merkle_tree genuinely missing |
| 11-agents.md | 29 | 17 | 63% | SafetyLayer edge paths need audit |
| 12-composition.md | 41 | 1 | 98% | KL divergence partial |
| 13-verification.md | 43 | 13 | 77% | ProcessRewardModel DONE but unchecked |
| 14-learning.md | 59 | 6 | 91% | |
| 15-neuro.md | 62 | 8 | 89% | Tier promotion exists |
| 16-conductor.md | 55 | 5 | 92% | |
| 17-chain.md | 50 | 20 | 71% | Phase 2+ deferred (correct) |
| 18-daimon.md | 35 | 16 | 69% | Stubs in phase2_stubs.rs |
| 19-dreams.md | 68 | 22 | 76% | Dream view widget DONE |
| 20-safety.md | 50 | 23 | 69% | SafetyLayer pre-exec hooks DONE |
| 21-interfaces.md | 50 | 35 | 59% | Genuine gaps — UI/scaffolder features |
| 22-coordination.md | 35 | 9 | 80% | |
| 23-identity-economy.md | 44 | 4 | 92% | |
| 24-code-intelligence.md | 40 | 11 | 78% | MCP code crate exists |
| 25-heartbeat.md | 74 | 5 | 94% | |
| 26-lifecycle.md | 68 | 4 | 94% | |
| 27-tools.md | 51 | 11 | 82% | |
| 28-deployment.md | 45 | 16 | 74% | |
| 29-technical-analysis.md | 88 | 1 | 99% | |
| 30-hallucination-audit.md | 159 | 0 | 100% | |

## Checklist — Items Implemented But Not Marked

These should be checked off after verification:

- [ ] Mark Demurrage trait as done (07-advanced-systems.md AS-06) — `crates/roko-core/src/demurrage.rs`
- [ ] Mark ProcessRewardModel as done (13-verification.md) — `crates/roko-gate/src/process_reward.rs`
- [ ] Mark KORAI token as done (17-chain.md CHAIN-01) — `crates/roko-chain/src/korai_token.rs`
- [ ] Mark dream rendering as done (19-dreams.md) — `crates/roko-cli/src/tui/widgets/dream_view.rs`
- [ ] Mark SafetyLayer pre-exec hooks as done (20-safety.md) — `crates/roko-agent/src/safety/mod.rs:216`
- [ ] Mark MCP code crate as done (24-code-intelligence.md) — `crates/roko-mcp-code/src/`
- [ ] Mark tier promotion as done (15-neuro.md) — `crates/roko-neuro/`
- [ ] Audit remaining ~50 items that may also be silently implemented

## Checklist — Genuinely Not Done

- [ ] `verify_merkle_tree` function (10-orchestration.md) — 0 code
- [ ] Epistemic value from KL divergence (12-composition.md COMP-08) — stub only
- [ ] Crate confidence in gate parsing (18-daimon.md DAIM-03) — stub only
- [ ] Full interface/scaffolder features (21-interfaces.md) — 35 items, weakest file
- [ ] Chain features (17-chain.md) — 20 items, Phase 2+ deferred

## Source Files

- **Gap checklist**: `tmp/docs-gaps/*.md`
- **Index**: `tmp/docs-gaps/00-INDEX.md`
