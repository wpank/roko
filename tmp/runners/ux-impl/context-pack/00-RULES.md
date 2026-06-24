# Rules

You are modifying the `roko` Rust workspace (~30 crates). Follow these
rules without exception.

## Mandatory

1. **One implementation, one code path.** Never duplicate types, traits,
   or logic that exists elsewhere. Before defining anything, grep for
   the symbol. If it lives in `crates/roko-core/src/foundation.rs`,
   import it.

2. **Wire, don't build.** If the type/trait/function exists but isn't
   called, wire it up. The codebase has the pieces — connect them.

3. **No stubs that silently pass.** Never return `Ok(())`,
   `Verdict::pass`, `Default::default()` for "success" without doing
   real work. If functionality isn't ready, return an explicit error.

4. **Foundation types are canonical.** `crates/roko-core/src/foundation.rs`
   owns: `ModelCallRequest`, `ModelCallResult`, `ModelCaller`,
   `PromptSpec`, `PromptAssembler`, `FeedbackEvent`, `FeedbackSink`,
   `GateConfig`, `GateService`, `EventConsumer`, `Effect`,
   `EffectExecutor`, `AffectPolicy`, `DispatchModulation`. Don't define
   local copies.

5. **Aggregator surface lives on roko-serve.** New `/api/*` endpoints go
   in `crates/roko-serve/src/routes/aggregator.rs`. Never add new REST
   routes to `apps/mirage-rs/`.

6. **Agents talk to each other directly.** roko-serve does not proxy
   agent-to-agent traffic. The Agent Card's endpoint URLs are the
   contract.

7. **Stay inside `scope`.** A batch's `scope` array (in
   `batches.toml`) is the only set of files you may modify. `also_read`
   files are read-only context. Any change outside scope is a violation.

8. **Public API stability.** If you change a function signature called
   from another crate, update every caller in the same batch. The wave
   gate (`cargo check --workspace + clippy`) catches the regression
   anyway, but do not externalise the breakage.

9. **`tokio::sync::Mutex` across `.await`, never `std::sync::Mutex`.**
   See AP-7 in `02-ANTI-PATTERNS.md`.

10. **No new dependencies without a clear win.** If something can be
    done with a workspace-existing crate, use it. New `cargo add` lines
    must be justified in the batch prompt.

## Style

- `anyhow::Result` for errors in CLI / runtime / serve. Typed errors in
  `roko-core` and other library-style crates.
- Derive `Serialize, Deserialize` for any type that crosses a
  persistence boundary (JSONL, on-disk JSON, HTTP body).
- Prefer `tracing::warn!` / `tracing::debug!` to `eprintln!` /
  `println!` outside CLI presentation paths.
- No doc comments, tests, or clippy annotations unless the batch
  prompt asks for them.
- Comments explain *why*, not *what*. A reviewer will reject obvious
  narrative comments.

## What to do when stuck

1. Re-read the batch prompt's "Acceptance Criteria" section.
2. Re-read `01-ARCHITECTURE.md` to confirm the layer you're touching.
3. If the prompt's instructions don't fit the actual code (drift), stop
   and report the discrepancy in the commit message. **Do not improvise
   a fix that breaks the contract** — the runner will fail the wave
   gate and a human will diagnose faster than you can.

## What never to do

- Modify `roko.toml`, `~/.claude/`, or any settings file as a side
  effect.
- Skip the wave gate by silencing clippy with `#[allow(clippy::*)]`.
- Add `unwrap()` to library crates (`roko-core`, `roko-runtime`,
  `roko-gate`, `roko-compose`).
- Restart the `bardo→roko` rename. The archive at `bardo-backup/` is
  read-only.
- Touch any file under `tmp/` as part of code work — `tmp/` is
  documentation and runner scratch space.
