# 33 - Agent Packet Verification Matrix

Purpose: provide a single verification and acceptance matrix for the low-tier
agent packets in docs `29`-`32`.

## Minimum Verification By Packet

| Packet | Primary command | Static check | Product proof required now |
|---|---|---|---|
| A0-1 fitness script | `bash -n scripts/roko-fitness-checks.sh && bash scripts/roko-fitness-checks.sh` | none | No |
| A0-2 docs status | `bash -n scripts/docs-status-check.sh && bash scripts/docs-status-check.sh` | none | No |
| A0-3 ACP ContentBlock | `cargo test -p roko-acp content_block` | `cargo check -p roko-acp` | Golden fixture only |
| A0-4 ACP session update | `cargo test -p roko-acp send_session_update` | `cargo check -p roko-acp` | Golden fixture only |
| A0-5 ACP failure event | `cargo test -p roko-acp failure` | `cargo check -p roko-acp` | Mocked failure path |
| A0-6 dangerous root config | `rg 'dangerously_skip_permissions\\s*=\\s*true' roko.toml` | no matches | No |
| D1 dispatch plan skeleton | `cargo test -p roko-core dispatch_plan` | `cargo check -p roko-core` | No |
| D2 rename local dispatch plan | none required | `cargo check -p roko-cli` | No |
| D3 resolver wrapper | `cargo test -p roko-agent dispatch_resolver` | `cargo check -p roko-agent` | No |
| D4 model stream event | `cargo test -p roko-agent model_stream` | `cargo check -p roko-agent` | No |
| D5 usage parser | `cargo test -p roko-agent parse_usage` | `cargo check -p roko-agent` | No |
| D6 ACP stream adapter | `cargo test -p roko-acp model_stream_event` | `cargo check -p roko-acp` | No |
| D7 block dispatch_direct | `cargo test -p roko-cli dispatch_direct` | `rg 'dispatch_direct::dispatch_prompt' crates/roko-cli/src` | No |
| R1 CommitOutcome type | `cargo test -p roko-runtime commit_outcome` | `cargo check -p roko-runtime` | No |
| R2 remove noop commit | `cargo test -p roko-runtime commit_no_changes` | `rg '\"noop\"|CommitDone \\{ hash' crates/roko-runtime/src` | No |
| R3 RunLedger skeleton | `cargo test -p roko-runtime run_ledger` | `cargo check -p roko-runtime` | No |
| R4 report from ledger | `cargo test -p roko-runtime workflow_report` | `cargo check -p roko-runtime` | Mocked event-log failure |
| R5 GateStatus type | `cargo test -p roko-gate gate_status` | `cargo check -p roko-gate` | No |
| R6 GateRegistry aliases | `cargo test -p roko-gate gate_registry` | `cargo check -p roko-gate` | No |
| R7 one rung map replaced | `cargo test -p roko-runtime gate_rung` | `cargo check -p roko-runtime` | No |
| R8 ArtifactOutcome adapter | `cargo test -p roko-cli artifact_outcome` | `cargo check -p roko-cli` | No |
| R9 command event DTOs | `cargo test -p roko-serve command_event` | `cargo check -p roko-serve` | No |
| C1 config provenance | `cargo test -p roko-core config_provenance` | `cargo check -p roko-core` | No |
| C2 provider identity | `cargo test -p roko-core provider_identity` | `cargo check -p roko-core` | No |
| C3 dangerous strict validation | crate owner test for `dangerously_skip_permissions` | crate owner check | No |
| C4 dangerous override type | `cargo test -p roko-core dangerous_permission_override` | `cargo check -p roko-core` | No |
| C5 UsageObservation core | none required | `cargo check -p roko-core && cargo check -p roko-agent` | No |
| C6 confidence router rename | `cargo test -p roko-learn record_confidence_outcome` | `cargo check -p roko-learn` | No |
| C7 override context block | `cargo test -p roko-learn override` | `cargo check -p roko-agent` | No |
| C8 parser unknown usage | `cargo test -p roko-agent usage` | `cargo check -p roko-agent` | No |
| C9 config doctor | `cargo run -p roko-cli -- config doctor` | `cargo check -p roko-cli` | Manual CLI smoke |

If a command name does not match existing tests, the agent should add a focused test
with a name close to the packet id and run that exact test.

## Acceptance Language For Runner Reports

Use this structure:

```text
Packet: D5
Status: Changed
Files changed:
- crates/roko-agent/src/translate/openai.rs
- crates/roko-agent/src/usage.rs

Verification:
- cargo test -p roko-agent parse_usage: pass
- cargo check -p roko-agent: pass

Old path:
- Legacy Usage conversion still reachable for compatibility, but new parser preserves unknown.

Notes:
- No provider HTTP or dispatch behavior changed.
```

## Packets That Must Not Be Given To Low-Tier Agents Yet

These are too broad until prior packets land:

- migrate all ACP dispatch to `ModelCallService::stream`;
- delete `dispatch_direct`;
- collapse CLI and core config fully;
- migrate all provider parsers to `UsageObservation`;
- replace all workflow reports with `RunLedger`;
- replace all gate/rung maps with registry;
- wire product-path proof CI as blocking;
- remove legacy orchestrate feature paths.

Convert each of these into smaller mechanical packets only after the prerequisite
types and adapters exist.

## Cross-Packet Dependencies

| Later packet | Requires |
|---|---|
| D3 resolver wrapper | D1, C1, C2 preferred |
| D4 stream events | D1 preferred, C5 preferred |
| D6 ACP stream adapter | D4 |
| D7 dispatch_direct block | D1/D3 preferred, but can be done early as error-only |
| R2 remove noop commit | R1 |
| R4 report from ledger | R3 |
| R7 replace rung map | R6 |
| R8 artifact adapter | R3 or local `ArtifactOutcome` type |
| C7 override context block | C6 preferred |
| C8 unknown usage parser | C5 preferred |
| C9 config doctor | C1/C2 preferred |

## Universal Failure Conditions

A packet should be marked `Failed` or `PartialBlocked` if completion requires any of:

- adding raw provider HTTP or SSE parsing outside `roko-agent` provider ownership;
- adding a new dangerous permission bypass;
- making feedback/safety/budget optional in production;
- changing broad runtime behavior without a test;
- editing unrelated generated files or formatting large untouched files;
- reverting user changes;
- using string sentinels for new state;
- converting unknown telemetry to zero;
- claiming product coverage without live path proof.

## Review Order For A Runner Wave

1. Run A0-1 and A0-2 first, even in inventory mode.
2. Land ACP wire blockers A0-3 through A0-5 before provider streaming migration.
3. Land core skeleton packets D1, R1, R3, R5, C1, C2, C5.
4. Land one proof-of-pattern packet from each track:
   - D5 unknown usage parser;
   - R2 noop commit removal;
   - R6 gate registry alias map;
   - C6 confidence router rename.
5. Only then assign broader migration packets.

