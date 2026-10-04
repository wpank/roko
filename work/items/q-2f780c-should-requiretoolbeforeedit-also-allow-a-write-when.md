+++
id = "q-2f780c"
kind = "question"
title = "Should RequireToolBeforeEdit also allow a write when the file's content was already injected into the prompt?"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["roko-agent/safety"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-8/9 follow-up reports, filed 2026-10-04 (bug-ef82eb, gap-2e455d)"
discovered_from = "bug-ef82eb, gap-2e455d"
anchors = ["crates/roko-agent/src/safety/contract.rs::GovernanceRule"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

`GovernanceRule::RequireToolBeforeEdit` (`crates/roko-agent/src/safety/contract.rs:646,794-821`) refuses a write
to an *existing* file unless the agent already called the named read tool (`read_file`, typically) in this
session. The check, `has_prior_tool` (`contract.rs:1122-1132`), looks only at `ctx.external_actions` — the
session's recorded tool-call history — and has no way to know whether the file's content was already present in
the prompt itself (e.g. injected context, a `context_layer` section listing files in scope with their contents).
`is_new_file` (the one exemption the rule already has) only covers files that don't exist yet; it says nothing
about files the agent never called a tool for but already has the text of.

A shakedown test exposed this concretely: a stub agent wrote `calc/ops.py` without calling `read_file` first, so
the write was refused and the attempt ended `pre_verify:no_changes` (bug-ef82eb, done — the triggering case).
`gap-2e455d` (done) made the no-changes feedback at least name the refused edit, but didn't change the policy
itself.

## Why it matters

Goal: release/truth (safety policy correctness). If roko's own prompt assembly can inject a file's full content
directly into context (as `context_layer` does for "files in scope," per earlier batches' research on
`gap-c8bfc8`), an agent that edits exactly that file without a separate `read_file` call isn't violating the
policy's actual security intent ("don't overwrite what you haven't seen") — it already has seen the content,
just not through a tool call. The policy currently can't distinguish that from genuinely blind overwriting.

## Where

- `crates/roko-agent/src/safety/contract.rs::GovernanceRule::RequireToolBeforeEdit`, `has_prior_tool`.
- Whatever assembles `context_layer`/injects file content into the prompt (`crates/roko-compose/src/system_prompt_builder.rs`
  or the dispatch-side prompt builder) — the thing this policy would need to check against if extended.

## Why this needs Will

Two options, each a real security/UX trade-off:

1. **Keep requiring a read tool call first, always.** Simple, auditable, matches the rule's name literally — but
   produces false-positive refusals (and `no_changes` attempts) whenever the agent reasonably relies on content
   already in its context, as the shakedown case showed.
2. **Also allow a write when the file's content was already injected into the prompt.** Matches the policy's
   actual intent more closely, but needs a reliable way to know, at enforcement time, exactly which files (and
   which version of their content) were injected — if that tracking is imperfect, it could open a real gap the
   rule exists to close.

## Notes

- Discovered from bug-ef82eb (done) and gap-2e455d (done); neither changed the policy itself, which is what this
  question is about.
