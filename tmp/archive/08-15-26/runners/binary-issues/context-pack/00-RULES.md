# Binary Issues — Universal Rules

## CRITICAL: Do NOT compile or run tests

**DO NOT** run `cargo check`, `cargo build`, `cargo test`, `cargo clippy`, `cargo run`, `rustc`, `rustfmt`, or `cargo fmt`. Compilation is handled by a separate validation pipeline AFTER your changes are merged. Running cargo wastes minutes per batch. Read the source files to verify types and signatures.

## Default-build assumption

This runner targets the **default** `cargo build -p roko-cli` (workspace `default = []`). That means:

- Code inside `#[cfg(feature = "legacy-orchestrate")]` (e.g. `crates/roko-cli/src/orchestrate.rs`, parts of `crates/roko-cli/src/run.rs`, the legacy half of `crates/roko-cli/src/dispatch_direct.rs`) is **NOT** in scope. If your prompt sends you there by accident, stop.
- The active chat path is `ChatAgentSession` (`crates/roko-cli/src/chat_session.rs`) + `chat_inline.rs`.
- The active run path is `WorkflowEngine` (`crates/roko-runtime/src/workflow_engine.rs`).
- `dispatch_direct.rs` lives only for the legacy feature flag — do not extend it.

## Universal anti-patterns (apply to every batch)

A1. **Confirmation theater.** A handler that prints "set X to Y" without making X actually be Y is worse than no handler. Either do the thing or return an error.

A2. **Silent error swallowing.** `let _ = foo()` is allowed only when followed by an inline comment that justifies why the failure mode is harmless. Otherwise log at minimum `tracing::warn!`.

A3. **Hardcoded literals where config exists.** If a value is already a config field, read from config. If it's not, leave the literal but pull it into a `pub const` near the top of the module so it is at least discoverable.

A4. **Don't re-introduce duplication.** If two functions render the same UI or call the same provider, extract the shared helper — never copy-paste a fix into both.

A5. **No new top-level crates.** Behavior belongs in the existing crate that already owns the concept (`roko-agent`, `roko-runtime`, `roko-serve`, `roko-cli`, `roko-core`, `roko-learn`).

A6. **Test gates only via `#[cfg(test)]`.** Anything labeled "permissive", "test only", or "for tests" must be `#[cfg(test)]`-gated, not just documented as such.

A7. **One trait, one definition.** Do not introduce a second `AffectPolicy`, `HttpPoster`, `Agent`, or other core trait. They live in `roko-core` / `roko-agent`.

A8. **Editor-only diffs.** Restrict each batch's writes to the files in its `scope`. If you need to read another file, list it under `also_read` and Read it; do NOT modify it.

## Security rules (BI_SEC group)

S1. **Default is safe.** Any boolean named `dangerous*`, `skip_*`, `allow_*`, or `permissive*` defaults to the safe value. The unsafe value is opt-in only via explicit user config or a `#[cfg(test)]` constructor.

S2. **Public means public.** `gh gist create --public` puts the user's transcript on the open internet. Default to `--secret` and run `LogScrubber` over the payload first.

S3. **Allowlist over blocklist.** Terminal command acceptance must be a strict allowlist (`bash`, `zsh`, `sh`, `fish`, `python`, plus an explicit operator-controlled extension). Never accept arbitrary `program` strings from request bodies.

S4. **Resource caps everywhere.** Anything that spawns a process, opens an FD, or holds memory needs a per-source cap and an idle TTL.

S5. **Block ≠ Warn.** A safety violation that is "secret leak" or "forbidden write" is a `Block`. `Warn` is for ambiguity, not for known-bad patterns.

## Phantom-feature rules (BI_PHN group)

P1. **Built means called.** A struct, trait, or method that has zero non-test callers is a phantom feature. The fix is either to add the call site (preferred) or to delete the code with a one-line ADR comment.

P2. **Persist what you learn.** Every learning state (router weights, cost table, gate thresholds, cascade snapshot) must round-trip through disk on `Drop`/shutdown AND load on startup. Persisting one direction is worse than persisting neither.

P3. **JSONL has consumers.** A producer that writes to a JSONL file with no in-process consumer is half a feature. Either spawn the consumer task on startup OR document the external consumer (with name and command) in code comments.

P4. **Routes match clients.** Frontend `fetch('/api/foo')` and backend `Router::route("/api/foo", ...)` must be string-identical. Mismatch = silent 404. The fix is to pin the route in a shared constant.

## Slash-command rules (BI_CMD group)

C1. **Inline execution preferred.** A slash command that prints "run `roko run …` in a terminal" is a UX failure — the user is already in the chat, they expect the action to happen. Execute inline via the same `WorkflowEngine` / `ModelCallService` the CLI uses.

C2. **Output streams to the chat buffer.** Do not capture-then-print: forward the engine's lifecycle events through `term.push_lines` as they arrive.

C3. **Config writes go through `RokoConfig::merge_runtime_overlay` (or equivalent).** Never `fs::write` `roko.toml` directly from a slash handler.

C4. **`--dry-run` means not-applied.** A flag named `--dry-run` must compute and display, not write. A flag without that name and that does `--dry-run` behavior is a bug.

## Subprocess rules (BI_SUB group)

U1. **Every spawn has a timeout.** No `Command::output()` or `child.wait()` without a `tokio::time::timeout` or equivalent.

U2. **Stderr is captured, not inherited.** `Stdio::inherit()` for stderr means subprocess noise leaks into the user's TUI. Use `Stdio::piped()` and forward to `tracing` or a log file.

U3. **Long-running tasks get cancellation tokens.** Pass `CancellationToken` from `tokio_util::sync::CancellationToken` (already a workspace dep). The chat loop must hand the token to the dispatcher; the dispatcher must check it between turns and propagate.

U4. **Handles are stored, not orphaned.** `tokio::spawn(...)` whose return value is dropped means the handle can never be aborted. Store in `JoinSet` or `JoinHandle` and abort on shutdown.

U5. **`eprintln!` belongs in `main()` only.** Library code uses `tracing::warn!` / `tracing::error!`. The TUI swallows raw `eprintln!` and corrupts the screen.

## Hardcoded-value rules (BI_HRD group)

H1. **Single source of truth.** Model defaults, base URLs, API versions, and pricing live in `roko-core/src/config/` (presets) or in provider config. Library modules read; they do not own.

H2. **Hardcoded as fallback only.** A `pub const DEFAULT_FOO: &str = "..."` is fine — but the function consuming it must accept an override and fall back to the const, not read the const directly.

H3. **Per-role / per-provider knobs.** `max_tokens`, `temperature`, `effort`, and timeout values live on the provider/role config, not in the dispatch function body.

## Mutex / unwrap rules (BI_MTX group)

M1. **`parking_lot::Mutex` over `std::sync::Mutex`.** No poisoning, lower overhead. The crate is already a workspace dep — verify by reading `Cargo.toml` of the target crate.

M2. **`if let Some(x) = ...` over `.is_some() && .unwrap()`.** TOCTOU is real even on `&self` — the value behind `&Option` can be cleared by another task between the check and the unwrap.

M3. **`expect("just registered")` is a panic.** It works until it doesn't. Replace with `ok_or_else(|| /* typed error */)`.

M4. **Crate-level lint allows hide bugs.** A `#![allow(clippy::expect_used, dead_code, unused_variables)]` at the top of a crate suppresses the signal that catches Mutex poisons, dead code, and shadowing. Remove the allow, fix the call sites individually.

## Commit message rules

Every batch commit must include:

- A title that begins with the batch ID, e.g. `BI_04: default dangerously_skip_permissions to false`.
- A body that references the source MASTER-INDEX section (e.g. `Closes MASTER-INDEX § S5.8`).
- The `ISSUE-TRACKER.md` checkbox flip in the same commit.

## Verification (mechanical, no cargo)

For every batch, the prompt provides a `Verify` block with `rg` / `grep` invocations that check the change landed. Run those — they're fast and don't shell out to cargo. Compilation happens later in the validation pipeline.
