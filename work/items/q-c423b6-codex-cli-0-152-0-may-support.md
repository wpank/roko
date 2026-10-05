+++
id = "q-c423b6"
kind = "question"
title = "codex-cli 0.152.0 may support deny_read sandboxing for the audit vault; confirming it needs a local probe"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-agent/provider"]
created = 2026-10-05
updated = 2026-10-05
last_verified = 2026-10-05
source = "wave-19 follow-up reports 2026-10-05 (gap-0714c4, work/gap-d10a97)"
discovered_from = "gap-0714c4 (done on work/gap-d10a97; own Progress note names this stronger mitigation, unverified)"
anchors = ["crates/roko-agent/src/provider/claude_cli.rs::codex_sandbox_args", "crates/roko-agent/src/exec.rs::CodexOperationPolicy"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

The codex 0.152.0 binary (`gap-0714c4`'s own research, on `work/gap-d10a97`, not yet merged)
appears to support deny-read rules that could confine Codex's reads away from the audit vault
— something roko's current Codex wiring has no equivalent of (`gap-0714c4`'s own finding:
`--sandbox workspace-write` confines writes and network, never reads). Confirmed independently
here, read-only (`strings` against the real native binary, not the Node.js launcher wrapper, at
`~/.nvm/versions/node/v22.22.2/lib/node_modules/@openai/codex/node_modules/@openai/codex-darwin-arm64/vendor/aarch64-apple-darwin/bin/codex`,
version `codex-cli 0.152.0` — matching exactly):

- The literal config key `permissions.filesystem.deny_read` appears in the binary's strings.
- `struct RawFilesystemRequirementsToml with 6 elements` lists a `deny_read` field alongside
  `workspace_roots`/`filesystem`/`network`/`description`/`extends`.
- A macOS Seatbelt profile fragment is present verbatim: `(deny file-read* (regex #"...`.
- A Windows-specific resolver path is also present: `windows-sandbox-rs/src/deny_read_resolver.rs`.
- Critically, one string suggests this may be gated to an enterprise/managed config tier, not a
  plain user profile: `` `permissions.filesystem` is reserved for requirements-level filesystem
  constraints and cannot define a profile `` — i.e. there appear to be two different surfaces
  (a per-profile `deny_read` under `RawFilesystemRequirementsToml`, and a separate
  managed/"requirements"-level `permissions.filesystem.deny_read`), and it's not clear from the
  binary's strings alone which, if either, a normal (non-managed) codex-cli config can actually
  set.

So the capability looks real, but its user-facing syntax, whether it's reachable from a plain
`config.toml` at all (vs. requiring an enterprise-managed deployment), and whether it would
actually confine reads the way roko would need, are all unconfirmed. Confirming this properly
needs an actual local codex sandbox run with a candidate `deny_read` config, no model call
required (purely a sandbox-behavior test) — but running codex in any capacity, even a $0 local
sandbox probe, is the kind of action this project's norms ask to clear with Will first.

## Why it matters

Goal: cybernetic, same goal as `gap-0714c4`/S05's audit-vault confinement. If this works, it
would close the one remaining gap `gap-0714c4` left open ("a Codex-run agent can read the audit
vault... regardless of sandbox level") with an actual OS-level read sandbox, matching what
bash and Claude Code already get, instead of only the reactive, after-the-start broker check
`gap-0714c4` pinned as the current mitigation.

## Where

- codex-cli 0.152.0's native binary (not the Node launcher) — `permissions.filesystem.deny_read`,
  `RawFilesystemRequirementsToml::deny_read`, the Seatbelt `deny file-read*` fragment.
- `crates/roko-agent/src/provider/claude_cli.rs::codex_sandbox_args`,
  `codex_writable_roots` (where a confirmed syntax would get wired in).
- `crates/roko-agent/src/exec.rs::CodexOperationPolicy` (the current, weaker, reactive
  mitigation `gap-0714c4` pinned; not to be removed if this lands, since it's still the
  backstop for anything a read sandbox doesn't cover).

## Plan (decision needed)

- Will's go-ahead to run a local codex sandbox probe: start a codex session with a candidate
  `deny_read`/`permissions.filesystem.deny_read` config entry targeting a throwaway test path
  (not the real audit vault, to avoid any risk during the probe itself), make no model call,
  and observe whether a read of that path is actually denied at the OS/sandbox level.
- If it works and is reachable from a plain (non-enterprise-managed) config: wire it into
  `codex_sandbox_args` to deny-read the audit vault, alongside the existing bash/Claude Code
  mechanisms.
- If it doesn't work, or needs enterprise-managed config roko can't ship: document that
  explicitly (next to `gap-0714c4`'s own doc comment) so a future reader doesn't re-discover
  the same dead end.

## Done when

Will authorizes (or declines) the probe; if authorized and it confirms the syntax works, it's
wired into `codex_sandbox_args` with a regression test; if not, the dead end is documented.

## Notes

- 2026-10-05 (wave-19 follow-up, gap-0714c4, work/gap-d10a97 not yet merged): confirmed
  independently via a read-only `strings` pass against the real native codex binary (not just
  the branch's own "its strings include..." note) — no codex process was started or invoked.
  Filed as `kind = "question"` per the instruction; no `[[verify]]` command since this needs
  Will's decision before any code or test exists to write.
