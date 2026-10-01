+++
id = "bug-a5f181"
kind = "bug"
title = "Native Gemini records its configured slug as the model, because it doesn't parse modelVersion"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/gemini"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "01851b630"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report, checked on work/bug-b8af02 at c4c6e86a7)"
anchors = ["crates/roko-agent/src/gemini/native.rs", "crates/roko-agent/src/translate/mod.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["bug-31438d", "bug-2379dc", "bug-afcf63"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn gemini_native_records_the_model_version' crates/roko-agent/src/ && cargo test -p roko-agent --lib gemini_native_records_the_model_version"
+++

## Problem

bug-31438d's work reads the served model from responses. The shared translator already takes Gemini's `modelVersion` (`translate/mod.rs:436-452`). The native Gemini adapter builds its own response with `model: Some(self.model.slug.clone())` (`gemini/native.rs:195`), so native Gemini calls record the configured slug, not the model that served them.

## Why it matters

One settled record per attempt (epic spec-b7303f): a pin check can't see a substitution on native Gemini, and a record states a "reported" model the provider never reported. bug-2379dc is the same for CLI adapters.

## Where

The response construction in `gemini/native.rs`.

## Plan

1. Take the model from the response's `modelVersion`, and leave it unknown when absent.
2. Add `gemini_native_records_the_model_version`.

## Done when

- [ ] Native Gemini records carry `modelVersion` as the served model.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/bug-739dcc` at `01851b630`; cargo verification deferred to the batch check. `gemini_native_records_the_model_version` (targeted `cargo test` passed). `GenerateContentResponse` parses `modelVersion`; the adapter's `usage_obs.model` is it, or None when the response names none, and failures report None. The configured slug stays on the `model` tag, and `ChatResponse.metadata.model_used` is the reported model, as other translators set it. The tool loop's native Gemini stream carries `modelVersion` on its Usage event, for bug-bfd241's collector.
