# M003 — Wire ExtensionChain into orchestrate.rs

## Objective
The ExtensionChain exists in the codebase but is not called from the main orchestration
loop. Wire it into the dispatch path so extensions can intercept agent execution.

## Scope
- Crates: `roko-cli`, `roko-core`
- Files: `crates/roko-cli/src/orchestrate.rs`, relevant extension chain source
- Phase ref: 01-PHASE-0-PREP.md §0.1

## Steps
1. Find the ExtensionChain implementation:
   `grep -rn 'ExtensionChain\|extension_chain\|struct Extension' crates/ --include='*.rs' | grep -v target/`

2. Find the dispatch site in orchestrate.rs where agent execution happens:
   `grep -n 'dispatch_agent\|agent.*dispatch\|run_agent' crates/roko-cli/src/orchestrate.rs`

3. If ExtensionChain exists and has a `run` or `execute` method:
   - Import it at the top of orchestrate.rs
   - Call it around the agent dispatch (before and/or after, depending on its API)
   - Pass appropriate context (task, agent config, etc.)

4. If ExtensionChain does NOT exist (the audit found code patterns but not a concrete struct):
   - Document what's missing in a comment: `// TODO(M-future): ExtensionChain not yet implemented`
   - Skip wiring for now

5. Ensure the wiring is conditional — if no extensions are configured, the path should be a no-op.

## Verification
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
```

## What NOT to do
- Do NOT implement ExtensionChain from scratch — only wire existing code
- Do NOT change the agent dispatch signature
- Do NOT add new crate dependencies
