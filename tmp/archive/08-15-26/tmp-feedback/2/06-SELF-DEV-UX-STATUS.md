# Self-Developing UX: Status of All 23 Documents

Source: `tmp/solutions/self-developing/`
Master PRD: `.roko/prd/drafts/self-developing-workflow.md`

## Status Summary

| Status | Count | Docs |
|--------|-------|------|
| NOT STARTED | 4 | 01, 04, 08, 11 |
| PARTIAL | 8 | 02, 03, 05, 06, 10, 12, 14, 22 |
| OPEN | 4 | 19, 20, 21, 23 |
| FIXED | 4 | 15, 16, 17, 18 |
| DEPENDS ON 01 | 2 | 08, 13 |

## Full Inventory

### P0 — Blocks Core Functionality

#### Doc 01: Model Configuration UX — NOT STARTED
**Problem**: 20+ fields per model; deterministic metadata requires manual config.
**Solution**: Builtin model registry (~50 known models pre-configured) + auto-detect API quirks from slug.
**Files**: New `config/model_registry.rs` (~250 lines), modify `agent.rs:304`, `model_selection.rs:362`, `openai_compat.rs:416`
**Blocks**: Docs 08, 13

#### Doc 02: Plan Generation UX — PARTIAL
**Problem**: Weak models fail on TOML, retry 3x with same model, opaque errors.
**Solution**: Auto-escalate haiku→sonnet→opus, show raw output, pass prev error to retry.
**Files**: Modify `prd.rs:1224` (escalation ~60 lines), `prd.rs:1169` (show output ~3 lines), `prd.rs:1240` (retry prompt ~5 lines), `prd.rs:2071` (self-heal ~30 lines)

### P1 — Degrades Daily Experience

#### Doc 03: CLI Output Noise — PARTIAL
**Problem**: Warnings for unused providers, emoji corruption.
**Solution**: Only validate active providers, move debug to `--verbose`.
**Files**: `do_cmd.rs:558` (TTY check), `plan.rs:542` (TTY check), `inline/terminal.rs:228` (CLICOLOR)

#### Doc 04: Zero-Knowledge Onboarding — NOT STARTED
**Problem**: No path from install to self-dev.
**Solution**: `roko setup` wizard, enhanced doctor, next-step prompts.
**Files**: New `setup.rs` (~150 lines), modify `doctor.rs`, `main.rs`

#### Doc 05: Idea → Execution Flow — PARTIAL
**Problem**: 5+ manual commands, each can fail.
**Solution**: `roko do` already wired; `roko develop` wrapper needed.
**Status**: `develop` exists now. TOML self-healing + escalation still needed.

#### Doc 06: Error Recovery — PARTIAL
**Problem**: Crashes give no guidance, no automated recovery.
**Solution**: Classify crashes, pass prev error to retry, auto-escalate.
**Files**: New `AgentCrashClass` enum, modify `prd.rs` (shared with doc 02)

#### Doc 07: `roko develop` Spec — PARTIAL
**Problem**: Need one command from idea to TUI.
**Solution**: Wrapper over `roko do` with approval + auto-TUI.
**Status**: `develop.rs` exists (212 lines). Missing: TOML self-healing, model escalation.

#### Doc 12: ACP/Zed Integration Errors — PARTIAL
**Problem**: max_tokens rejected, model not forwarded to slash commands, poor error UX.
**Solution**: Auto-detect max_completion_tokens, pass --model, format errors for users.
**Files**: `bridge_events.rs:1000-1050` (model forwarding ~50 lines), `bridge_events.rs:1500+` (error formatting ~50 lines), `session.rs` (filter providers ~10 lines)

#### Doc 13: Config That Shouldn't Exist — NOT STARTED (depends on doc 01)
**Problem**: 770 lines of deterministic metadata in roko.toml.
**Solution**: Builtin registry + auto-synthesis from env vars.
**Files**: Same as docs 01, 04, 12.

### P2 — Missing Features

#### Doc 08: Model Discovery — NOT STARTED (depends on doc 01)
**Problem**: Can't discover available models without reading TOML.
**Solution**: `roko models list`, fuzzy matching, shell completion.
**Files**: `config_cmd.rs` (models list ~60 lines), `model_selection.rs` (fuzzy ~50 lines)

#### Doc 09: Unified CLI UX (3 Verbs) — PARTIAL
**Problem**: 42 top-level commands, overlapping intent.
**Solution**: Primary: note, plan, do/develop. Power user: everything else.
**Status**: `note`, `do`, `develop` exist. Missing: `plan "prompt"` direct mode.

#### Doc 10: Terminal Output Corruption — PARTIAL
**Problem**: Long runs corrupt terminal with \r, emoji, ANSI.
**Solution**: TTY detection at sink selection, CLICOLOR support.
**Files**: `do_cmd.rs:558`, `plan.rs:542`, `terminal.rs:228`

#### Doc 11: Context Sources & Editor Integration — NOT STARTED
**Problem**: Can't pass files/folders as context.
**Solution**: `--context <path>` flag, folder walking, ACP `/context` command.
**Files**: New `context_loader.rs` (~150 lines), modify do_cmd.rs, develop.rs, plan.rs, bridge_events.rs

#### Doc 14: Image Support — PARTIAL
**Problem**: ACP advertises image support but images are silently discarded.
**Solution**: Convert Image blocks to backend format, inject into messages.
**See**: [01-IMAGE-SUPPORT.md](01-IMAGE-SUPPORT.md) for full details.

### Architectural (Longer-term)

#### Doc 19: ACP Model Has No Tools — OPEN
**Problem**: ACP dispatch is pure chat; can't read/write files or chain commands.
**Solution**: Pass tool definitions to ACP dispatch.

#### Doc 20: Learning Not Wired in ACP — OPEN
**Problem**: Dream, distillation, experiments are CLI-only.
**Solution**: Wire learning pipeline to ACP dispatch path.

#### Doc 21: Cross-Provider Cascade Error — OPEN
**Problem**: Gemini API key error when using OpenAI.
**Solution**: Pass --model to all slash commands (doc 12 fix).

#### Doc 22: Plan Run TUI Broken — PARTIAL
**Problem**: Execution exits silently; Graph Engine stub, stale snapshot.
**Solution**: Wire Graph Engine, refresh executor snapshots.

#### Doc 23: TUI Plan List Scroll — OPEN
**Problem**: Down-arrow goes off-screen.
**Solution**: Fix scroll viewport in TUI.

### Fixed

| Doc | Title | Fix |
|-----|-------|-----|
| 15 | bare_mode kills commands | Set `bare_mode = false` |
| 16 | resource_link crash | Added `ResourceLink` variant |
| 17 | Decision provenance noise | Removed visible card from UI |
| 18 | Slash commands don't stream | Stream line-by-line |

## Critical Path

```
Doc 01 (builtin registry)     ← P0, enables 08, 13
  ↓
Doc 02 (plan generation)      ← P0, model escalation
  ↓
Doc 05/07 (develop command)   ← P1, unified entry point
  ↓
Doc 11 (context sources)      ← P2, quality improvement
  ↓
Doc 09 (3-verb CLI)           ← P2, UX cleanup
```
