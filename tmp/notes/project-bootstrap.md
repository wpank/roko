# Project Bootstrap Notes

> **STALE**: Historical notes from the April 2026 project bootstrap. All decisions described
> here remain in effect. See `CLAUDE.md` for current architecture overview.
> Last updated: 2026-08-13

## Initial Setup (2026-04-07)

- Workspace layout: 18 crates under `crates/`
- Core abstractions: Signal + 6 verb traits
- Universal loop: query → score → route → compose → act → verify → write → react
- Reference implementation: mori (108K LOC) in `apps/mori/`

### Key decisions
- Rust-first, async-native (tokio)
- JSONL for signals, TOML for plans
- Claude CLI as primary agent backend
- Safety layer integrated into dispatch, not bolted on
