+++
id = "gap-5f4852"
kind = "gap"
title = "Secret-canary persistence test never run for scrubbers/persistent sinks"
status = "open"
triage = "verified"
severity = "p1"
size = "L"
goal = "release"
subsystem = ["roko-fs/observability"]
created = 2026-09-14
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "a17d9d766"
source = "tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-fs/src/observability.rs::RunScrubber", "crates/roko-cli/src/main.rs::build_log_scrubber", "crates/roko-cli/src/main.rs::load_startup_env_files", "crates/roko-cli/tests/secret_canary.rs", "crates/roko-cli/tests/common/mod.rs::setup_sample_plan_workspace"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rn --include='*.rs' 'RunScrubber' crates/ | grep -v 'crates/roko-fs/src/observability.rs' | grep -v 'crates/roko-fs/src/lib.rs' | grep -q . && grep -qw 'fn canary_absent_from_every_file_after_plan_run' crates/roko-cli/tests/secret_canary.rs && cargo test -p roko-cli --test secret_canary"
+++

## Problem

Nothing shows that a secret an agent sees stays out of what roko writes to disk. If a provider
key from `.roko/.env` or `~/.roko/.env` appears in an agent's output, for example because the agent
ran `env` or echoed a variable, it may end up in `.roko/` files in plain text. No test checks
this.

The 2026-09-21 tool audit release gate "Secret canaries are absent from normal output and every
persistent sink" is unchecked. Audit finding T036 says "persistence scrubbers do not receive
configured literal secrets".

At HEAD, `roko_fs::observability::RunScrubber` is the type meant to carry those literal secrets to
every sink (`crates/roko-fs/src/observability.rs:186`), but nothing in production calls it. The only
production scrubber that knows the `.env` values is the stderr log formatter in
`crates/roko-cli/src/main.rs`.

The existing canary suite, `crates/roko-cli/tests/secret_canary.rs`, does not exercise a production
write path. For example, `episode_logger_jsonl_does_not_contain_canary` (:105) writes an episode
that never contained the canary and then asserts that the canary is absent. The "scrubber catches
contamination" checks in `canary_absent_from_all_output_sinks` (:344) scrub a string with the
test's own scrubber (`build_scrubber_from_env_file`, :51). Neither proves that the writers scrub.

## Why it matters

Goal `release` (Public release: security, licence, CI, first run). Leaking API keys into
workspace files, which get shared, committed or uploaded as run bundles, is a security defect for a
public repo, which is why it is p1. This item closes one of the tool-audit release gates. The
sibling gate is `gap-633184` (control events under backpressure). Shared-run transcripts already go
through a scrubber (`crates/roko-cli/src/share.rs::scrub_share_text`,
`crates/roko-serve/src/routes/shared_runs.rs:597`), but that is a separate path.

## Where

- `crates/roko-fs/src/observability.rs`:
  - `RunScrubber::build(&[(name, value)])` (:196) builds an `Arc<LogScrubber>` with built-in regexes
    plus literal values;
  - `RunScrubber::from_env_vars(&[names])` builds the same from environment variable names;
  - it is re-exported at `crates/roko-fs/src/lib.rs:74`, and its only callers are tests (:636).
- `crates/roko-cli/src/main.rs`:
  - `load_startup_env_files` (:4498) loads `~/.roko/.env`, then `.roko/.env`, and returns the
    `(name, value)` pairs as `startup_env_redactions` (:3293);
  - `build_log_scrubber` (:3587) is a copy of `RunScrubber::build` without the `Arc`;
  - it is used only for the stderr `RedactingFormat` layer (:3429), which exists only when verbose
    or `ROKO_LOG`/`RUST_LOG` logging is on.
- `roko_core::obs::LogScrubber` provides `scrub` and `add_literal_value`.
- Production persistent writers on a Graph run, all under `.roko/`. Whether each one scrubs is
  unknown; the test in step 1 will show it:
  - `episodes.jsonl` (`EpisodeLogger`);
  - `learn/efficiency.jsonl`;
  - `learn/gate-failures.jsonl`;
  - `learn/post-gate-reflections.json`;
  - `state/graph/<plan>/activities.jsonl` and `checkpoint.json`;
  - `events.jsonl` (`RokoLayout::events_jsonl_path`);
  - `signals.jsonl`;
  - the knowledge store.
- Sinks named in the old verify that have no production writer at HEAD:
  - `roko_fs::trace_sink::JsonlTraceSink`;
  - `roko_fs::tool_audit::ToolAuditLog`, which has a `ScrubAuditAdapter`;
  - `roko_core::transcript_store::TranscriptStore`.
- Test harness: `crates/roko-cli/tests/common/mod.rs::setup_sample_plan_workspace` (:182) writes a
  mock `claude` script (`MOCK_CLAUDE_SCRIPT`, :172) and points `roko.toml` at it. `smoke.rs`
  (`item_04_plan_runner_reports_non_zero_agent_calls`, :223) runs `roko plan run` with it.

## Current state

- Done:
  - The canary suite exists (244f564e1). It covers `LogScrubber` loading from `.env` and the
    `scrub_share_text` redaction, but only weakly covers the three JSONL sinks, as described above.
  - The stderr logs are scrubbed with the `.env` values when they are shown.
- Not done:
  - no run-wide scrubber reaches persistence writers;
  - `RunScrubber` has no production caller;
  - no test runs a real plan with a leaking provider and scans the output files.
- Unknown: whether a file log layer (as opposed to stderr) exists and scrubs.

## Plan

1. Write the failing end-to-end test first:
   `canary_absent_from_every_file_after_plan_run` in `crates/roko-cli/tests/secret_canary.rs`.
   - Start from `setup_sample_plan_workspace`. Set `HOME` to a temp dir so the real
     `~/.roko/.env` is not read.
   - Write `CANARY_SECRET` into the workspace `.roko/.env`.
   - Make the mock `claude` script emit the canary in an assistant text block, in a `tool_use`
     input and in the final `result` text.
   - Run `roko plan run`, capturing stdout and stderr.
   - Assert that the mock really emitted the canary, so the test cannot pass vacuously.
   - Walk every file under the workspace `.roko/` (and any plan worktree roko creates), and
     assert that no file, and neither stdout nor stderr, contains the canary.
2. Build one scrubber per process: replace `build_log_scrubber` with
   `roko_fs::observability::RunScrubber::build` fed from `startup_env_redactions`. Also add the
   values of the provider `api_key_env` variables from the loaded config through
   `RunScrubber::from_env_vars`.
3. Get the scrubber to every writer that step 1 shows leaking. There are two options:
   - (A) Pass `Arc<LogScrubber>` through the runtime services and dispatcher into each writer. This
     is explicit, but it touches many constructors.
   - (B) Install it once in a process-wide `OnceLock` in `roko_core::obs`, and have the shared JSONL
     and JSON persistence helpers scrub string content before writing. This is less plumbing, but it
     is global state, and tests must be able to reset or override it.
   Recommend (B) for the append-only JSONL writers, plus explicit scrubbing where a writer
   serialises a whole struct (`checkpoint.json`). Check first whether roko-fs already has a common
   append helper; that is unknown.
4. Keep the existing tests. Rewrite the tautological ones so that they push contaminated content
   through the real writer, not just through the test scrubber.
5. `JsonlTraceSink`, `ToolAuditLog` and `TranscriptStore` are not written in production. Do not add
   canary tests for them in this item. Record in the test file's module doc that they must gain one
   when they are wired.

## Done when

- `roko plan run` with a canary in `.roko/.env` and a provider that echoes it leaves no copy of the
  canary in any file under `.roko/`, in stdout or in stderr. The new end-to-end test proves this,
  and it fails at HEAD.
- `RunScrubber` (or its replacement) is built once from the configured secrets and used by the
  persistence writers. `build_log_scrubber` is gone.
- Verify (suggested replacement: the old one required canary tests for three sinks that production
  never writes):
  `grep -rn --include='*.rs' 'RunScrubber' crates/ | grep -v 'crates/roko-fs/src/observability.rs' | grep -v 'crates/roko-fs/src/lib.rs' | grep -q . && grep -qw 'fn canary_absent_from_every_file_after_plan_run' crates/roko-cli/tests/secret_canary.rs && cargo test -p roko-cli --test secret_canary`

## Notes

- This is security-relevant persistence work. Scrub values; never log the secret while scrubbing,
  and never write it to test output. Use an obviously fake canary such as `sk-canary-...`.
- Scrubbing must not corrupt JSON. Scrub string fields before serialising, or scrub the serialised
  line only where the replacement marker cannot break quoting. Then re-parse in the test.
- Short values produce false positives. The test helper already skips values under 8 characters;
  keep the same rule in production.
- The test must be hermetic: a temp `HOME`, the mock provider only, and no network.
- It overlaps with any item that changes the Graph persistence writers (`runner/persist.rs`,
  `graph_task_dispatch.rs` feedback writes), so avoid running it in parallel with those.

- 2026-09-30 (wk-canary): Implemented on `work/gap-5f4852` at `551a339e1`. `cargo test -p roko-cli --test secret_canary` passes in the worktree (11 tests); the new plan-run test found the canary in 8 files at `8a88c6267` (episodes, efficiency, gate-failures, post-gate-reflections, neuro/knowledge.jsonl, the file log `roko.log.*`, activities.jsonl, retry-feedback.json) and finds none now. Option B: `roko_core::obs::install_secret_scrubber`, installed once by `RunScrubber::install` in `main`. Persistence scrubs literal secrets only, from JSON strings (`scrub_secrets_in_json[l]`), since the built-in heuristics also match ids such as `mask-the-...`. Every `.env` value of 8+ characters counts as a secret, so a non-secret setting kept in a `.env` file is redacted from records too. Not covered: config secrets other than provider `api_key_env` (provider `extra_headers`, file secrets, `serve.auth.api_key`). Batch check pending.

## Original notes

RunScrubber with literal secrets exists (On main; related T006 ClassifiedRecord redaction), but release gate 'Secret canaries are absent from normal output and every persistent sink' is unchecked (not yet tested).

Imported without verification from:
- `tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register`
- `tmp/archive/tool-audit-2026-09-21/11-IMPLEMENTATION-CHECKLIST.md#release-gate`
- `tmp/archive/tool-audit-2026-09-21/00-INDEX.md#status-note-2026-09-14`

How to verify: Check that configured literal secrets reach RunScrubber in production wiring and that a canary test covers episodes/traces/audit/transcripts.

Verified 2026-09-28: partly done. crates/roko-cli/tests/secret_canary.rs (added in 244f564e1) plants env-file canaries and checks LogScrubber redaction, episodes.jsonl, efficiency.jsonl, gate-failures.jsonl and share transcripts; canary_absent_from_all_output_sinks covers the three JSONL sinks. Still true: RunScrubber (crates/roko-fs/src/observability.rs:186) has no production call site; it is only re-exported at roko-fs/src/lib.rs:74, so configured literal secrets never reach it. No canary covers traces, audit logs or the transcript store. The canary suite was not run in this triage.

Re-checked 2026-09-29 at d9e79e9d8: unchanged. The [[verify]] command (cargo test -p roko-cli --test secret_canary) runs only the existing suite, which exercises neither RunScrubber nor traces, audit logs or the transcript store, so it can pass while this gap is open.
