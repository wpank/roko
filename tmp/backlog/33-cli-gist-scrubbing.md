# 33 — CLI share path: wire scrubbing and reconcile dead code

**Priority**: P1 — Security: `roko run --share` writes unscrubbed transcripts that may contain secrets
**Size**: S (1 day)
**Crates**: `crates/roko-cli/` (`src/run.rs`, `src/share.rs`)
**Depends on**: None

---

## Background

Roko is a Rust agent toolkit. The `roko run` command executes a prompt through the `WorkflowEngine` (agent dispatch → gate pipeline → persist results). The `--share` flag is intended to create a shareable transcript of that run.

When `roko run --share` is invoked, the transcript is written to `.roko/shared/{token}.json` on disk. This file contains the prompt, the agent output, gate results, and model metadata in plain JSON. Any secret that appeared in the prompt or in the agent's output (API keys, bearer tokens, database credentials, etc.) is written to this file without redaction.

Separately, a module `crates/roko-cli/src/share.rs` exists that implements `share_run()` with proper secret scrubbing via `LogScrubber` and optionally uploads to GitHub Gist via `gh gist create`. This module is never called from any production code path — it is dead code. The scrubbing logic it contains is correct and should be the reference implementation.

The security issue is in `run.rs`: the `write_shared_workflow_run` function (called by `roko run --share`) passes transcript content directly to disk without any scrubbing step.

## Current State

1. **`roko run --share` code path** (no scrubbing):

   - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/util.rs`, line 371: `roko_cli::run::write_shared_workflow_run(...)` is called when `share == true`.
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`, line 87: `write_shared_workflow_run` builds a `RunTranscript` from the raw `WorkflowRunReport` and passes it to `write_shared_transcript`.
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`, line 134: `write_shared_transcript` serializes the transcript as JSON and writes it to `.roko/shared/{token}.json`. No scrubbing is applied at any step.
   - The function also prints a local URL (`http://localhost:6677/runs/{token}`) — there is no Gist upload in this path.

2. **Scrubbing reference implementation** (dead code):

   - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/share.rs` contains `scrub_share_text` (line 49) which applies `LogScrubber` and a secondary heuristic pass that redacts long hex (≥32 chars) and long base64 (≥32 chars) strings.
   - `share_run` (line 85) calls `scrub_share_text` on the prompt and output, writes the scrubbed transcript to `.roko/shared/`, and attempts a GitHub Gist upload via `gh gist create`.
   - This function is defined but has **zero callers in production code** — `grep -rn "share_run\|crate::share" crates/roko-cli/src/` (excluding `share.rs` itself) returns no results.

3. **Serve-side scrubbing** (working, different path):

   - `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/shared_runs.rs`, line 506: `scrub_run_transcript` applies `LogScrubber` recursively to all JSON string values in the transcript. This path is invoked for HTTP share requests through the serve API (`/api/runs/{id}/share`), which is separate from the CLI `--share` flag.
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/shared_runs.rs`, line 560: `scrub_share_text` is the serve-side analog of the CLI-side `scrub_share_text` in `share.rs`. Both call `LogScrubber.scrub()` followed by long-string heuristics.

4. **`roko_core::obs::LogScrubber`** is the canonical scrubber used by both implementations. It redacts known secret patterns: API key formats (`sk-ant-*`, `sk-*`), bearer tokens, environment variable assignments (`FOO_KEY=...`), and GitHub/Slack tokens.

## Implementation Plan

**Step 1: Add scrubbing to `write_shared_workflow_run` in `run.rs`**

Open `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`.

Add an import for `LogScrubber`:
```rust
use roko_core::obs::LogScrubber;
```

Add a `scrub_share_text` helper function (mirrors the one already in `share.rs`):
```rust
fn scrub_share_text(text: &str) -> String {
    use std::sync::OnceLock;
    static SCRUBBER: OnceLock<LogScrubber> = OnceLock::new();
    let scrubber = SCRUBBER.get_or_init(LogScrubber::new);
    let redacted = scrubber.scrub(text);
    scrub_long_secret_like_strings(&redacted)
}

fn scrub_long_secret_like_strings(text: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;
    static HEX_RE: OnceLock<Regex> = OnceLock::new();
    static B64_RE: OnceLock<Regex> = OnceLock::new();
    let hex_re = HEX_RE.get_or_init(|| {
        Regex::new(r"(^|[^0-9A-Fa-f])([0-9A-Fa-f]{32,})([^0-9A-Fa-f]|$)").unwrap()
    });
    let b64_re = B64_RE.get_or_init(|| {
        Regex::new(r"(^|[^A-Za-z0-9+/=])([A-Za-z0-9+/=]{32,})([^A-Za-z0-9+/=]|$)").unwrap()
    });
    let redacted = hex_re.replace_all(text, "$1[REDACTED]$3");
    b64_re.replace_all(redacted.as_ref(), "$1[REDACTED]$3").into_owned()
}
```

**Alternatively**, move `scrub_share_text` and `scrub_long_secret_like_strings` from `share.rs` to `roko_core::obs` (or `roko_cli::share`) and make them `pub` so `run.rs` can call them without duplicating the regex logic.

In `write_shared_workflow_run`, apply scrubbing before building the `RunTranscript`:

```rust
pub fn write_shared_workflow_run(
    workdir: &std::path::Path,
    prompt: &str,
    agent: &str,
    role: &str,
    report: &WorkflowRunReport,
) -> anyhow::Result<String> {
    let token = roko_core::generate_share_token();
    let (report_agent, report_role) = workflow_report_agent_role(report);
    // Scrub secrets from prompt and output before writing to disk.
    let scrubbed_prompt = scrub_share_text(prompt);
    let scrubbed_output = report.output.as_str();
    let scrubbed_output = if scrubbed_output.is_empty() {
        None
    } else {
        Some(scrub_share_text(scrubbed_output))
    };
    let transcript = roko_serve::routes::shared_runs::RunTranscript {
        // ... same as before but using scrubbed_prompt and scrubbed_output
        prompt: scrubbed_prompt,
        output: scrubbed_output,
        // ... other fields unchanged
    };
    write_shared_transcript(workdir, &transcript)
}
```

**Step 2: Wire `share_run` in `share.rs` or remove it**

The `share_run` function in `share.rs` has proper scrubbing and Gist upload but is dead code. Two options:

Option A (Recommended): Delete `share.rs` entirely since its functionality is now covered by the fixed `write_shared_workflow_run`, or leave the file but mark it clearly as unused infrastructure. The Gist upload feature it provides (`gh gist create`) is not currently exposed by any CLI command.

Option B: Wire `share_run` into `run_inline.rs` (the one-shot inline mode) so it is actually called. This gives Gist upload as a feature. Only do this if Gist upload is a desired feature; if not, Option A avoids accumulating dead code.

**Step 3: Add a test to `run.rs`**

Add a unit test that calls `write_shared_workflow_run` with a prompt and output containing an API key pattern, reads the written JSON file, and asserts the key does not appear:

```rust
#[test]
fn write_shared_workflow_run_scrubs_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let report = WorkflowRunReport {
        output: "Result: ANTHROPIC_API_KEY=sk-ant-secret123 was used".to_string(),
        // ... other fields at defaults
    };
    write_shared_workflow_run(
        dir.path(),
        "Use ANTHROPIC_API_KEY=sk-ant-secret123",
        "agent",
        "implementer",
        &report,
    ).unwrap();
    let files: Vec<_> = std::fs::read_dir(dir.path().join(".roko/shared")).unwrap().collect();
    let content = std::fs::read_to_string(files[0].unwrap().path()).unwrap();
    assert!(!content.contains("sk-ant-secret123"), "API key leaked into shared transcript");
    assert!(content.contains("[REDACTED]"), "no redaction marker found");
}
```

## Acceptance Criteria

1. `roko run --share "Use ANTHROPIC_API_KEY=sk-ant-test123"` writes a transcript to `.roko/shared/` that does not contain `sk-ant-test123`.
2. The written JSON contains `[REDACTED]` in place of any redacted value.
3. A unit test in `run.rs` covers secret scrubbing through `write_shared_workflow_run`.
4. `cargo test -p roko-cli 2>&1 | grep -E "test result|FAILED"` shows zero failures.
5. `cargo clippy -p roko-cli -- -D warnings` is clean.
6. The `share.rs` module is either removed (if Gist upload is out of scope) or clearly documented as infrastructure for a not-yet-exposed feature.

## Verification Checklist

- [ ] Run `grep -n "scrub" crates/roko-cli/src/run.rs` — confirm at least one `scrub_share_text` call appears
- [ ] Run `roko run --share "My key is ANTHROPIC_API_KEY=sk-ant-testkey999"` in a roko workspace
- [ ] Run `cat .roko/shared/*.json | grep sk-ant-testkey999` — confirm zero matches
- [ ] Run `cat .roko/shared/*.json | grep REDACTED` — confirm at least one `[REDACTED]` entry
- [ ] Run `cargo test -p roko-cli -- --test-threads=1 2>&1 | tail -5` — confirm zero failures
- [ ] Run `cargo clippy -p roko-cli -- -D warnings 2>&1 | tail -5` — confirm no warnings
- [ ] Verify `share.rs` is either removed or has a `//! NOTE: Not yet wired` header comment

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs` | Add `scrub_share_text` helper; apply scrubbing in `write_shared_workflow_run` before building `RunTranscript`; add unit test |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/share.rs` | Either delete (if Gist upload is not being wired) or add a module-level doc comment noting it is unused infrastructure |
