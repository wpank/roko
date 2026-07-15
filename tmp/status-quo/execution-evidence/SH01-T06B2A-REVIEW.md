# SH01-T06B2A independent review

## Assignment

- Candidate: `c71eb14f1aaa78a13375273a0981ccf166ead637`
- Candidate parent: `1649c18b2c3d2b3602bfe17398b0e1454a19c5ef`
- Review branch: `review/SH01-T06B2A-c71eb14f1aaa`
- Authorized write scope: this review evidence only

## Independent reconstruction and production-path review

I read the complete current master checklist, complete SH01 manifest, issues 28 and
47, worker evidence, candidate diff, the full ownership module, the exact matching
precursor `3041d095d` hunk, and every unchanged production call to `claim_phase`,
`claim_cancellation_exact`, and `claim_for_cleanup`.

At the parent commit, both `claim_cancellation_exact` and shared `take_resource`
marked a slot claimed, allocated/stored a nonce, and advanced `next_nonce` before
attempting `resource.take()`. A resource-less slot therefore returned `Ineligible`
after partial mutation. The cancellation path additionally set `Cancelling` before
the failed take.

The candidate checks `slot.resource.is_none()` before every one of those mutations.
Only after eligibility succeeds does `advance_claim_nonce` atomically set `claimed`,
allocate and advance the nonce, and store `claim_nonce`; cancellation state then
changes and the already-proved-present payload moves into the claim. The exclusive
mutable borrow makes the following `take().unwrap()` safe: there is no intervening
operation capable of removing the payload.

Consequences independently verified from code and tests:

- Failed no-resource cancellation and ordinary take leave owner identity, phase,
  effect, timing, agent metadata, cancellation state, `claimed`, `claim_nonce`,
  resource, and allocator nonce unchanged.
- Surviving aggregate agent metadata is derived from those unchanged owners, so it
  is unchanged as well.
- Phase/effect mismatch, already-claimed, occupied insert, stale nonce, duplicate
  completion, and replacement-effect behavior remain linear and are covered by the
  unchanged focused suite.
- All production callers are unchanged. Exact event callers still validate
  attempt/phase/effect; cleanup callers still reject claimed or missing resources
  before the shared take.
- Candidate scope is exactly `attempt_ownership.rs` plus worker evidence. It does
  not import the precursor's registry `resource` to `resource_mut` replacement,
  event-loop changes, manifest changes, or other C4 work. The pre-existing
  `AttemptClaim::resource_mut` method is not a candidate change.

## Independent reproduction and commands

- Isolated parent checkout plus only the candidate regression test:
  `cargo test -p roko-cli --lib runner::attempt_ownership::tests::missing_resource_claims_leave_slot_and_nonce_unchanged -- --exact`
  — exit 101. The first post-cancellation assertion observed `next_nonce = 42`
  instead of `41`, reproducing the partial mutation against the exact parent.
- Same exact command on the candidate — exit 0; 1 passed.
- `cargo test -p roko-cli --lib runner::attempt_ownership` — exit 0; 26 passed,
  0 failed.
- `cargo check -p roko-cli --lib` — exit 0 after supplying the existing integration
  frontend `dist` as a temporary read-only symlink. The first attempt failed only
  because this review worktree lacked the generated RustEmbed directory.
- `cargo clippy -p roko-cli --lib -- -D warnings` — exit 0 with the same temporary
  frontend asset setup.
- `rustfmt --edition 2024 --check crates/roko-cli/src/runner/attempt_ownership.rs` —
  exit 0.
- `git diff --check 1649c18b2c3d2b3602bfe17398b0e1454a19c5ef..c71eb14f1aaa78a13375273a0981ccf166ead637`
  — exit 0.
- The temporary parent checkout and frontend symlink were removed; the review
  worktree was clean before creating this evidence.

## Verdict

`ACCEPTED`

Confidence: high. The exact parent failure was independently reproduced, the
candidate fixes the mutation ordering without changing callers or APIs, adversarial
stale/occupied behavior remains green, and no out-of-scope C4 hunk entered the
candidate. No implementation correction is required. Integration and post-merge
verification are the coordinator's next actions.
