# CLI Gist Scrubbing

**Priority**: P1 — security, secrets leak to public Gists
**Size**: S (1 day)
**Crate**: `crates/roko-cli/src/` (CLI share path)

---

## Problem

`roko run --share` uploads agent transcripts to GitHub Gists without scrubbing secrets.
The serve-side share endpoint (`/api/runs/{id}/share`) goes through auth middleware and
has scrubbing logic, but the CLI path bypasses this entirely. API keys, tokens, and other
secrets present in agent output can leak to public Gists.

This is a concrete security issue: any user running `roko run --share` on a prompt whose
output contains an API key, database credential, or bearer token will publish that secret
to a publicly-accessible GitHub Gist.

---

## Section A: Current State

**A1.** The CLI `--share` flag triggers a Gist upload path somewhere in
`crates/roko-cli/src/`. Search for `--share`, `gist`, and `upload` to locate the exact
code path. The transcript content is sent to the GitHub Gists API with no
pre-processing.

**A2.** The serve-side share endpoint at `crates/roko-serve/src/routes/shared_runs.rs`
already has proper `auth_routes()` middleware and content scrubbing. This is the
reference implementation.

**A3.** Secrets can appear in transcripts from multiple sources:
- roko.toml configuration values (API keys in `[providers]` sections)
- Environment variables echoed by agent tool calls (`$ANTHROPIC_API_KEY`, etc.)
- Agent output that includes credentials found in files or config
- MCP server connection strings

---

## Section B: What To Do

**B1.** Locate the CLI Gist upload path. Search `crates/roko-cli/src/` for `--share`,
`gist`, and any GitHub API upload calls.

**B2.** Read the scrubbing logic in `crates/roko-serve/src/routes/shared_runs.rs`.
Understand what patterns it redacts and how.

**B3.** Extract the scrubbing logic into a shared function (or reuse it if it is already
in a shared crate). Apply it to the CLI Gist upload path so that transcript content is
scrubbed before upload.

**B4.** Ensure the scrubbing covers at minimum:
- All secret values from `roko.toml` (provider API keys, tokens)
- Common environment variable patterns (`*_API_KEY`, `*_TOKEN`, `*_SECRET`)
- Any values loaded via `roko config set-secret` / `roko config secrets`

---

## Acceptance criteria

- [ ] `roko run --share` scrubs secrets from transcript before Gist upload
- [ ] Scrubbing logic matches or reuses the serve-side implementation in `shared_runs.rs`
- [ ] Provider API keys from `roko.toml` are redacted in uploaded Gists
- [ ] Environment variable secrets (`*_API_KEY`, `*_TOKEN`, `*_SECRET`) are redacted
- [ ] Secrets set via `roko config set-secret` are redacted
- [ ] Existing `cargo test -p roko-cli` passes with no regressions
- [ ] Manual verification: run `roko run --share` with a prompt that would expose an API key in output, confirm the Gist contains redacted values (e.g., `[REDACTED]`)

### Not in scope
- Changing the serve-side share endpoint (already has scrubbing)
- Adding new secret detection heuristics beyond what the serve-side already does
- Encrypting Gist content or changing Gist visibility settings
