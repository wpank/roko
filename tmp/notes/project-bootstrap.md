# Project Bootstrap Notes

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
