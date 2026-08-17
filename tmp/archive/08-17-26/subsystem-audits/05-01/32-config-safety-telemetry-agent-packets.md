# 32 - Config, Safety, Telemetry, and Learning Agent Packets

Purpose: break `25-config-safety-telemetry-plan.md` into mechanical packets.
Use `28-agent-tasking-playbook.md` as the assignment template. These packets should
make unsafe or unknown states explicit before changing broad runtime behavior.

Config/safety/telemetry anti-patterns to avoid:

- Do not make dangerous permission bypasses valid in shared config just because local
  development needs an escape hatch.
- Do not read env vars from arbitrary downstream crates when provenance/config should
  own the decision.
- Do not turn missing usage, cost, context, provider id, or model id into zero or an
  empty string.
- Do not label confidence-only feedback as contextual learning.
- Do not synthesize fake routing context to satisfy a learning API.
- Do not add config mutation to inspection/doctor commands.

## C1: Add Config Provenance Types

Context files:

- `tmp/subsystem-audits/05-01/22-config-schema-redesign.md`
- `tmp/subsystem-audits/05-01/25-config-safety-telemetry-plan.md`

Write scope:

- `crates/roko-core/src/config/`
- tests in `roko-core`

Mechanical steps:

1. Add `ConfigProvenance`, `ValidatedConfig`, and `ResolvedRuntimeConfig` structs.
2. Keep them unused or minimally wired.
3. Add unit tests constructing provenance for file, default, migration, env, local
   override, and CLI override.

Do not:

- Do not migrate CLI config yet.
- Do not change config loading behavior yet.

Verification:

```bash
cargo test -p roko-core config_provenance
cargo check -p roko-core
```

Acceptance:

- Provenance types exist and can represent source/reason for config values.

## C2: Add Provider Identity And Transport Types

Context files:

- `tmp/subsystem-audits/05-01/22-config-schema-redesign.md`
- `tmp/subsystem-audits/05-01/25-config-safety-telemetry-plan.md`

Write scope:

- `crates/roko-core/src/config/provider.rs` or nearby provider config module
- tests

Mechanical steps:

1. Add `ProviderId`, `ModelAlias`, `BackendModelSlug`.
2. Add `ProviderTransport`, `ProviderAuth`, `ProviderDefinition`,
   `ModelDefinition`.
3. Add tests for invalid empty ids if constructors exist.

Do not:

- Do not change existing provider config parsing.
- Do not infer transport from command names in this packet.

Verification:

```bash
cargo test -p roko-core provider_identity
cargo check -p roko-core
```

Acceptance:

- Provider id, kind, transport, auth, model alias, and backend slug can be represented
  separately.

## C3: Add Strict Config Validation For Dangerous Root Permission

Context files:

- `tmp/subsystem-audits/05-01/09-safety-bypass.md`
- `tmp/subsystem-audits/05-01/15-config-safety-regression.md`
- `tmp/subsystem-audits/05-01/25-config-safety-telemetry-plan.md`

Write scope:

- config validation module in `roko-core` or `roko-cli`
- tests

Mechanical steps:

1. Add a strict validation function that rejects shared/root config with
   `dangerously_skip_permissions = true`.
2. Allow test fixtures only when path or mode is explicitly test/local.
3. Add one test that root/shared config fails.
4. Add one test that false/absent passes.

Do not:

- Do not implement local override expiry yet.
- Do not change all defaults in this packet unless required by tests.

Verification:

```bash
cargo test -p roko-cli dangerously_skip_permissions
cargo test -p roko-core dangerously_skip_permissions
```

Run whichever crate owns the validator.

Acceptance:

- Strict validation can reject dangerous shared config.

## C4: Add Local Dangerous Override Type

Context files:

- `tmp/subsystem-audits/05-01/25-config-safety-telemetry-plan.md`

Write scope:

- config/safety module that owns permission policy
- tests

Mechanical steps:

1. Add `DangerousPermissionOverride` with enabled, scope, reason, expires_at,
   ack_env, source.
2. Add validation method requiring:
   - non-empty reason;
   - non-empty scope;
   - future expiry;
   - local override source;
   - acknowledgement env name present.
3. Add tests for each missing requirement.

Do not:

- Do not wire env var reading yet.
- Do not make production accept overrides.

Verification:

```bash
cargo test -p roko-core dangerous_permission_override
```

Acceptance:

- Override policy is typed and validates mechanically.

## C5: Move UsageObservation To Core

Context files:

- `tmp/subsystem-audits/05-01/20-learning-telemetry-redesign.md`
- `tmp/subsystem-audits/05-01/25-config-safety-telemetry-plan.md`

Write scope:

- `crates/roko-core`
- `crates/roko-agent/src/usage.rs`
- imports in crates that currently use `roko-agent::usage::UsageObservation`

Mechanical steps:

1. Move or duplicate `UsageObservation` and `UsageSource` into `roko-core`.
2. Re-export from `roko-agent/src/usage.rs` temporarily to avoid broad breakage.
3. Update imports in one or more crates mechanically if easy.
4. Add core unit test proving optional fields serialize as absent/null according to
   existing serde convention.

Do not:

- Do not change provider parsers yet.
- Do not remove compatibility re-export yet.

Verification:

```bash
cargo check -p roko-core
cargo check -p roko-agent
```

Acceptance:

- Core owns `UsageObservation`; agent compatibility still compiles.

## C6: Rename Confidence-Only Router API

Context files:

- `tmp/subsystem-audits/05-01/20-learning-telemetry-redesign.md`
- `tmp/subsystem-audits/05-01/25-config-safety-telemetry-plan.md`

Write scope:

- `crates/roko-learn/src/cascade_router.rs`
- direct callers/tests

Mechanical steps:

1. Rename `record_outcome` to `record_confidence_outcome`.
2. Keep a deprecated wrapper only if too many callers break, but wrapper should call
   the renamed method and include a concrete removal tracking issue or comment naming
   the follow-up packet.
3. Update tests to assert this path does not increment LinUCB observations.

Do not:

- Do not change reward math.
- Do not wire real context in this packet.

Verification:

```bash
cargo test -p roko-learn record_confidence_outcome
cargo check -p roko-learn
```

Acceptance:

- API name no longer implies contextual learning.

## C7: Block RoutingContext::default In Override Learning

Context files:

- `tmp/subsystem-audits/05-01/20-learning-telemetry-redesign.md`
- `tmp/subsystem-audits/05-01/25-config-safety-telemetry-plan.md`

Write scope:

- `crates/roko-agent/src/model_call_service.rs`
- `crates/roko-learn/src/cascade_router.rs`
- tests

Mechanical steps:

1. Find force-backend override recorder path using `RoutingContext::default()`.
2. Change no-context override to confidence-only observation.
3. Add a test that override without context does not increment LinUCB observations.
4. If a real context is available, preserve contextual override behavior.

Do not:

- Do not invent fake context features.
- Do not use default context as a placeholder.

Verification:

```bash
cargo test -p roko-learn override
cargo check -p roko-agent
```

Acceptance:

- Override learning without real context is not contextual learning.

## C8: Provider Parser Unknown Usage Test

Context files:

- `tmp/subsystem-audits/05-01/20-learning-telemetry-redesign.md`
- `tmp/subsystem-audits/05-01/25-config-safety-telemetry-plan.md`

Write scope:

- one provider parser module
- tests

Mechanical steps:

1. Add tests for provider response with no usage block and provider response with
   explicit zero usage.
2. If current parser returns concrete `Usage`, add a new parser function returning
   `UsageObservation` and make tests target it.
3. Do not migrate call sites unless trivial.

Do not:

- Do not zero-fill missing usage in new tests.
- Do not change all providers at once.

Verification:

```bash
cargo test -p roko-agent usage
```

Acceptance:

- There is at least one provider-level proof that unknown and zero differ.

## C9: Config Doctor Skeleton

Context files:

- `tmp/subsystem-audits/05-01/22-config-schema-redesign.md`
- `tmp/subsystem-audits/05-01/25-config-safety-telemetry-plan.md`

Write scope:

- `crates/roko-cli/src/config.rs`
- CLI command registration if config subcommands already exist
- tests if command tests exist

Mechanical steps:

1. Add `roko config doctor` command stub if config command structure exists.
2. Print current config path, config version, schema version, provider count,
   model count, and whether dangerous permissions are present.
3. Exit zero unless parsing fails.

Do not:

- Do not implement full migration.
- Do not make doctor mutate config.

Verification:

```bash
cargo check -p roko-cli
cargo run -p roko-cli -- config doctor
```

Acceptance:

- A user can inspect basic config health without changing files.
