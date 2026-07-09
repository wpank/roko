# Self-Developing UX Improvements

The roko self-development workflow is broken for anyone who doesn't already know how everything works. These documents capture the problems and propose solutions.

## Documents

| # | File | Problem Area | Status |
|---|------|-------------|--------|
| 01 | [model-config-ux.md](01-model-config-ux.md) | Model selection & provider config is a maze | Open |
| 02 | [plan-generation-ux.md](02-plan-generation-ux.md) | `prd plan` fails opaquely, no feedback loop | Open |
| 03 | [cli-noise.md](03-cli-noise.md) | Warnings, log lines, and output formatting | Open |
| 04 | [zero-knowledge-onboarding.md](04-zero-knowledge-onboarding.md) | No path from "I installed roko" to "roko is developing itself" | Open |
| 05 | [idea-to-execution-flow.md](05-idea-to-execution-flow.md) | The braindump→PRD→plan→execute pipeline has too many manual steps and failure modes | Open |
| 06 | [error-recovery.md](06-error-recovery.md) | When things fail, roko should help you fix it, not just print a hint | Open |
| 07 | [roko-develop-spec.md](07-roko-develop-spec.md) | `roko develop` — the one command that does everything | Open |
| 08 | [model-discovery-ergonomics.md](08-model-discovery-ergonomics.md) | Tab completion, fuzzy matching, `roko models` list | Open |
| 09 | [unified-cli-ux.md](09-unified-cli-ux.md) | 3 verbs: `note`, `plan`, `do` — stop proliferating commands | Open |
| 10 | [terminal-output-corruption.md](10-terminal-output-corruption.md) | Spinners/emoji corrupt terminal, garbled output after long runs | Open |
| 11 | [context-sources-and-editor-integration.md](11-context-sources-and-editor-integration.md) | `--context` flag, Zed ACP integration, @-file mentions | Open |
| 12 | [acp-zed-errors.md](12-acp-zed-errors.md) | `max_tokens` bug fix, ACP error UX, default model for editor | Partial |
| 13 | [config-should-not-exist.md](13-config-should-not-exist.md) | Most config is deterministic metadata that should be auto-inferred | Open |
| 14 | [image-support.md](14-image-support.md) | Vision/image support for ACP + all agent backends | **Partial** |
| 15 | [bare-mode-kills-commands.md](15-bare-mode-kills-commands.md) | `bare_mode = true` strips 42+ slash commands from Zed | **Fixed** |
| 16 | [resource-link-crash.md](16-resource-link-crash.md) | `resource_link` content block crashes ACP deserialization | **Fixed** |
| 17 | [decision-provenance-noise.md](17-decision-provenance-noise.md) | "Decision provenance" tool card clutters every Zed response | **Fixed** |
| 18 | [slash-command-no-streaming.md](18-slash-command-no-streaming.md) | Slash commands buffer output — UI hangs with no feedback | **Fixed** |
| 19 | [acp-model-has-no-tools.md](19-acp-model-has-no-tools.md) | ACP model is pure chat — can't read/write files or chain commands | Open |
| 20 | [learning-not-wired-in-acp.md](20-learning-not-wired-in-acp.md) | Self-learning features (dream, distillation, experiments) are CLI-only | Open |
| 21 | [cross-provider-cascade-error.md](21-cross-provider-cascade-error.md) | Gemini API key error when using OpenAI — model not forwarded to CLI | Open |
| 22 | [plan-run-tui-broken.md](22-plan-run-tui-broken.md) | Plan run exits silently: stale snapshot, Graph Engine stub, hidden stderr | Partial |
| 23 | [tui-plan-list-scroll.md](23-tui-plan-list-scroll.md) | Plans list down-arrow goes off-screen instead of scrolling viewport | Open |

## Fixes Applied This Session (2026-05-06)

| What | Change | Files |
|------|--------|-------|
| bare_mode | `false` in both roko.toml and ~/.roko/config.toml | Config only |
| resource_link + Image | Added to ContentBlock enum + all match sites | types.rs, event_forward.rs, bridge_events.rs |
| image: true | ACP reports image capability to Zed | handler.rs:287 |
| Provenance card | Removed visible tool-call card from Zed UI | bridge_events.rs, runner.rs |
| Slash streaming | Output streams line-by-line instead of buffering | bridge_events.rs |

## Core Thesis

Roko's self-development loop should feel like talking to a colleague, not like configuring Kubernetes. The current UX requires:
- Reading source code to understand what model slugs are valid
- Editing TOML by hand for basic operations
- Knowing internal concepts (providers vs models vs backends vs slugs)
- Tolerating noisy output that obscures real problems
- Retrying failed commands with no guidance on what to change
- Using slash commands manually because the model can't chain them
- Understanding that `bare_mode` exists and kills features

The goal: `roko develop "I want cursor composer 2 support"` should Just Work.

## Priority Ranking

### Critical (blocks basic usability)
- **19**: ACP model has no tools — pure chatbot, not an agent
- **21**: Cross-provider cascade error — model not forwarded to CLI
- **12**: ACP errors not actionable — users can't self-diagnose

### High (degrades experience significantly)
- **14**: Image support — Zed shows upload button but images are discarded
- **20**: Learning not wired — cybernetic loop is CLI-only
- **05/07/09**: Unified CLI UX — too many commands, too many steps
- **01/13**: Config complexity — users shouldn't edit TOML for basic operations

### Medium (annoying but workable)
- **02**: Plan generation fails opaquely
- **06**: Error recovery is "print hint, give up"
- **08**: Model discovery — no tab completion, no fuzzy matching
- **10**: Terminal corruption on long runs
- **11**: `--context` flag doesn't exist yet

### Low (nice to have)
- **03**: CLI noise (warnings from unused providers)
- **04**: Zero-knowledge onboarding wizard
