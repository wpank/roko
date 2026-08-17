# 29 - Wave 0 Guardrails and ACP Wire Packets

Purpose: create small, low-risk tasks that stop new drift and unblock ACP validation
before the larger dispatch/runtime redesign begins.

Each packet is self-contained. Do not combine packets unless a human reviewer asks.
Use `28-agent-tasking-playbook.md` as the assignment template.

Wave 0 anti-patterns to avoid:

- Do not "fix" safety or ACP by adding alternate wire formats or alternate bypasses.
- Do not make guard scripts green by hiding known violations.
- Do not replace a broken contract with aliases that accept both correct and incorrect
  output in production.
- Do not mark a blocker resolved unless the product-facing ACP/config path is covered
  by a test or static check.

## A0-1: Add Fitness Check Script In Inventory Mode

Context files:

- `tmp/subsystem-audits/05-01/26-enforcement-and-runner-controls.md`
- `tmp/subsystem-audits/AGENT-FAILURE-PATTERNS.md`

Write scope:

- `scripts/roko-fitness-checks.sh`
- `scripts/fitness/allowlist.toml`

Mechanical steps:

1. Create `scripts/roko-fitness-checks.sh` with `set -uo pipefail`, but inventory
   mode must not fail the whole script yet.
2. Add named checks that print counts for:
   - raw provider HTTP outside provider adapters;
   - dangerous permission bypass;
   - provider API env reads outside approved boundaries;
   - unknown-to-zero telemetry patterns;
   - path-based modules;
   - sentinel success/noop strings.
3. Create `scripts/fitness/allowlist.toml` with documented placeholder schema:
   `pattern_id`, `path`, `reason`, `owner`, `expires`.
4. Make the script print `FITNESS INVENTORY COMPLETE`.

Do not:

- Do not edit CI yet.
- Do not try to fix violations.
- Do not add broad `grep -v` exceptions inside the script. Use allowlist comments/schema.

Anti-patterns:

- Do not make the first script so strict that it blocks all work before baseline review.
- Do not hide failures by redirecting output to `/dev/null`.

Verification:

```bash
bash -n scripts/roko-fitness-checks.sh
bash scripts/roko-fitness-checks.sh
```

Acceptance:

- Script runs from repo root.
- It reports all named check sections.
- It exits zero in inventory mode.

## A0-2: Add Docs Status Drift Check

Context files:

- `tmp/subsystem-audits/05-01/26-enforcement-and-runner-controls.md`
- `tmp/subsystem-audits/ANTI-PATTERNS-V2.md`

Write scope:

- `scripts/docs-status-check.sh`

Mechanical steps:

1. Create a shell script that scans docs for new status words:
   `Resolved`, `Done`, `Wired`, `LiveInAllProductPaths`, `ProvenByE2E`.
2. For now, inventory mode prints matching lines and does not fail.
3. Ignore historical mentions only by printing a note that allowlist support is
   future work.
4. Print recommended vocabulary: `Built`, `WiredInOnePath`, `LiveInAllProductPaths`,
   `RetiredOldPath`, `ProvenByE2E`.

Do not:

- Do not rewrite existing docs in this packet.
- Do not make the script block CI yet.

Verification:

```bash
bash -n scripts/docs-status-check.sh
bash scripts/docs-status-check.sh
```

Acceptance:

- Script identifies status claims and exits zero.
- Output names the approved vocabulary.

## A0-3: ACP ContentBlock Wire Golden Test

Context files:

- `tmp/subsystem-audits/05-01/01-protocol-serialization.md`
- `tmp/subsystem-audits/05-01/13-acp-provider-regression.md`

Write scope:

- `crates/roko-acp/src/types.rs`
- ACP tests in the same crate

Mechanical steps:

1. Find `ContentBlock::Text` serialization.
2. Add a test that serializes a text block and asserts outbound JSON has
   `"type": "text"` if the external ACP fixture/spec confirms that spelling.
3. Keep inbound deserialization tolerant only if existing tests require aliases.
4. If current code emits `"content"`, change outbound serialization to `"text"`.
5. Update or add one regression test for inbound alias behavior if kept.

Do not:

- Do not redesign ACP message flow.
- Do not touch provider dispatch.
- Do not change unrelated ACP structs.

Anti-patterns:

- Do not lock in local tests against an unverified external contract.
- Do not use aliases for outbound canonical format.

Verification:

```bash
cargo test -p roko-acp content_block
cargo check -p roko-acp
```

Acceptance:

- Golden test proves outbound text block shape.
- Existing ACP type tests pass.

## A0-4: ACP `send_session_update` Flat Payload Test

Context files:

- `tmp/subsystem-audits/05-01/01-protocol-serialization.md`
- `tmp/subsystem-audits/05-01/02-bridge-events.md`

Write scope:

- `crates/roko-acp/src/bridge_events.rs`
- ACP tests in the same crate

Mechanical steps:

1. Locate `send_session_update`.
2. Add a test that builds a session update and asserts `sessionId` and update fields
   are in the same payload object expected by the ACP client.
3. If the current implementation nests the update under `update`, change only that
   serialization assembly.
4. Keep the existing `SessionUpdate` enum shape unless the test proves it must change.

Do not:

- Do not change provider or cognitive dispatch code.
- Do not change unrelated event mappings.

Verification:

```bash
cargo test -p roko-acp send_session_update
cargo check -p roko-acp
```

Acceptance:

- Test fails on double-nested payload and passes on flat payload.

## A0-5: ACP Typed Failure Instead Of Normal Completion

Context files:

- `tmp/subsystem-audits/05-01/02-bridge-events.md`
- `tmp/subsystem-audits/05-01/13-acp-provider-regression.md`

Write scope:

- `crates/roko-acp/src/bridge_events.rs`
- local ACP event tests

Mechanical steps:

1. Add or reuse a typed failure event for cognitive/model dispatch failure.
2. Change missing-auth/provider-error branches so they emit failure, not
   content text followed by `Complete { EndTurn }`.
3. Add a test using a simulated missing auth/provider failure.
4. Preserve user-visible error text, but attach it to failure status.

Do not:

- Do not add another provider client.
- Do not change successful stream completion behavior.
- Do not swallow errors with `let _ =`.

Verification:

```bash
cargo test -p roko-acp failure
cargo check -p roko-acp
```

Acceptance:

- Failed dispatch cannot produce a normal complete event in the test.

## A0-6: Root Dangerous Permission Guard

Context files:

- `tmp/subsystem-audits/05-01/09-safety-bypass.md`
- `tmp/subsystem-audits/05-01/15-config-safety-regression.md`
- `tmp/subsystem-audits/05-01/22-config-schema-redesign.md`

Write scope:

- `roko.toml`
- a config validation test if one already exists nearby

Mechanical steps:

1. Change root `[runner] dangerously_skip_permissions` to `false` or remove it.
2. If existing config tests cover root config loading, add an assertion that shared
   root config does not enable dangerous skip.
3. If no nearby test exists, only change `roko.toml` and report that test coverage
   is deferred to the config/safety packet.

Do not:

- Do not add a new bypass field elsewhere.
- Do not modify local user override files.
- Do not change unrelated config values.

Verification:

```bash
rg 'dangerously_skip_permissions\\s*=\\s*true' roko.toml
```

Expected result: no matches.

Acceptance:

- Shared root config no longer enables dangerous permission bypass.
