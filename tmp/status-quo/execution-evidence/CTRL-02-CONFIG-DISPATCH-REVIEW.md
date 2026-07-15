# CTRL-02 config-dispatch independent review

## Assignment and candidate identity

- Task: `CTRL-02-CONFIG-DISPATCH`, independent precursor review.
- Assigned base: `b59e497e71143212dfca0f50dbbfac3ce4a47844`.
- Exact repository candidate reviewed:
  `91992857f160097b4e850708dfe7b606f262ee0a`.
- Review branch/worktree: `review/CTRL-02-config-dispatch-91992857` /
  `reviews/CTRL-02-config-dispatch-91992857`.

The expanded SHA in the author handoff was mistyped outside the repository.
The committed implementation evidence contains no false candidate SHA: it
deliberately delegates immutable identity to this independent record. No
candidate evidence correction is required for that external typo.

I read the complete master; the full SH01, SH05, E04, E18, E42, P17, and P20
manifests; issues 23 and 55; the July 14 audit and self-heal audit; the complete
candidate evidence and diff; the exact five-file `1649c18b2..3041d095d`
precursor history; and current loader, schema, startup, route, watcher, daemon,
state, runner, and test call sites. The candidate changes exactly four paths:

```text
crates/roko-core/src/config/loader.rs
crates/roko-serve/src/lib.rs
crates/roko-serve/src/routes/config.rs
tmp/status-quo/execution-evidence/CTRL-02-CONFIG-DISPATCH.md
```

Reserved `config/mod.rs` and `config/timeouts.rs` are unchanged. The five
historical production files are byte-unchanged between `3041d095d` and the
assigned base. No manifest, master, index, lockfile, runner, status, or
unrelated StateHub hunk changed.

## Findings

### High: public state-based serve startup bypasses model validation

`ServerBuilder::start_background` now validates before its own state creation,
restore, spawn, and bind work. The separate public production entry point
`run_server_with_state` in `crates/roko-serve/src/lib.rs:787` does not call the
validator at all. It reads the unvalidated config and then, before binding,
restores state and starts the config watcher, PRD subscriber, event bridges,
state saver, job runner, and cold archival timer. A caller can therefore
construct an `AppState` containing duplicate slugs or unresolved dispatch
references and start live background work through this API.

This is not a hypothetical dead branch: it is a public API with integration
tests and the repository's serve audit identifies it as a distinct startup
profile. It violates the requirement that serve startup reject invalid model
state before bind, state restoration, or task spawn. The candidate's single
`ServerBuilder` test cannot cover this path, and the evidence's boundary claim
is consequently too broad.

Required correction: make `run_server_with_state` validate and canonicalize a
cloned state config as its first fallible operation, store the canonical config
only after validation, and add a regression proving duplicate/unresolved state
returns the configuration error before restore, spawn, bind, or filesystem
mutation. Prefer a shared startup helper so the two public startup profiles
cannot drift again.

### High: PUT and reload have no shared transaction/TOCTOU boundary

`update_config` in `crates/roko-serve/src/routes/config.rs:70-119` independently
loads the ArcSwap snapshot, merges, validates, performs an asynchronous raw
file write, awaits propagation writes, and only then swaps live state.
`reload_config_from_disk` independently reads disk and swaps state, and both the
HTTP reload route and background watcher can invoke it. There is no mutex,
generation check, compare-and-swap, or state-owned transaction shared by these
mutation paths.

Two concurrent disjoint PUTs can both read snapshot S and each persist a config
derived only from S, losing one accepted update. A valid interleaving can also
leave file and live state different: A writes file A and pauses during an
awaited propagation; B writes file B and stores state B; A resumes and stores
state A. PUT racing explicit/watcher reload has the same class of stale-snapshot
problem. Per-request validation before mutation does not close this gap.

This fails the assigned concurrent-update/TOCTOU, canonical persistence, and
file/state agreement checks. The new tests exercise only serial calls.

Required correction: introduce one state-owned asynchronous config mutation
transaction used by PUT, explicit reload, and watcher reload. Hold ownership
across current-snapshot selection, merge/load, validation, serialization,
atomic main-file replacement where applicable, and live-state commit. Add a
deterministic barrier-based concurrent test proving two disjoint PUT patches
both survive and that returned config, `roko.toml`, and live state agree. Add a
PUT-versus-reload test proving stale work cannot overwrite the winning
generation. Do not silently discard failed ephemeral-workspace propagation;
report it separately without rolling live/file truth backward.

### Medium: nested unresolved-reference error selection is nondeterministic

`normalize_and_validate_dispatch_models` sorts duplicate-slug owners, but it
iterates `agent.tier_models` and `agent.roles` directly. Both fields are
`HashMap`s. When more than one tier or role is invalid, the first returned
`UnresolvedModel` depends on randomized map iteration rather than a stable
field order. The candidate test supplies only one invalid tier and one invalid
role in separate configs, so it cannot detect this.

An isolated 1,000-construction reproduction using the same two-entry standard
`HashMap` iteration observed both possible first fields:

```text
first_invalid_tiers={"alpha", "omega"}
```

That contradicts the implementation evidence's deterministic-rejection claim
and makes HTTP/startup errors unstable across processes.

Required correction: collect and sort tier and role keys before validating and
mutating their values. Add multiple-invalid-tier and multiple-invalid-role
tests that assert the exact stable first field across repeated maps with
different insertion order.

## Verified behavior and bounded ownership

The core normalization semantics that were exercised are otherwise sound:
exact model keys take precedence over another profile's slug; unique slugs
canonicalize default, fallback, tier, and role fields; duplicate owners are
sorted in their error; and an empty model registry preserves the legacy path.
Invalid serial PUT/reload checks happen before their own file/state mutation,
the inherited error variants map to HTTP 400, successful serial PUT responses
and persistence use canonical keys, and existing response masking remains
after canonical serialization. These positives do not close the three findings
above.

The candidate also correctly leaves SH05-T01 and the adjacent E42/E04/E18/P17/
P20 tasks open. It does not claim config migrations/invariants/provenance,
redaction convergence, warning-level changes, same-provider duplicate-slug
policy, zero-config synthesis, or CLI dual-config convergence.

## Independent commands and results

```text
candidate identity: 91992857f160097b4e850708dfe7b606f262ee0a
cumulative candidate scope: exact four paths listed above
git diff --check: pass
cargo fmt --all -- --check: pass
3041d095d..b59e497e7 five-file drift: none
reserved config/mod.rs and config/timeouts.rs candidate diff: none

cargo test -p roko-core config::loader -- --nocapture
  exit 0; 22 passed, 0 failed
cargo test -p roko-core config::timeouts -- --nocapture
  exit 0; 7 passed, 0 failed

cargo test -p roko-core config -- --test-threads=1
  exit 101; 192 passed, 4 failed
  failures:
    config::cache::tests::watched_cache_sees_file_change
    config::schema::tests::effective_models_backwards_compat
    config::schema::tests::effective_providers_backwards_compat
    config::schema::tests::project_model_profiles_have_explicit_max_output
```

The four broad failures reproduce the author's report exactly. The three
schema failures require repository-root `roko.toml`, which is absent from both
base and candidate. The cache failure is in an unchanged test/source file and
reproduced after the five-second watcher wait. Candidate diff against both
`schema.rs` and `cache.rs` is empty, so these are verified baseline failures,
not candidate regressions.

I also started the focused `roko-serve` route test independently. It rebuilt a
large dependency stack and reached the final link without a compiler diagnostic;
after the release-blocking source paths above were established, I interrupted
that reviewer-local build rather than treating it as a pass or failure. The
author's serve results therefore remain author evidence, not an independent
green claim in this record. The review does not depend on that unfinished
positive gate because each finding is in production code outside or beyond the
covered serial tests.

All reviewer-created build artifacts are outside the review worktree. The
review worktree was clean before this record was added; no production or
control-plane file was edited.

## Verdict

**REJECTED** for exact candidate
`91992857f160097b4e850708dfe7b606f262ee0a`.

Confidence: high. The candidate improves the serial builder and route paths and
truthfully remains a bounded precursor, but one public startup still admits
invalid dispatch state, mutable config lacks concurrency ownership, and nested
error selection is not deterministic. Correct all three, update the evidence's
boundary/determinism proof, complete the focused serve gates, and submit a new
immutable candidate for independent review.
