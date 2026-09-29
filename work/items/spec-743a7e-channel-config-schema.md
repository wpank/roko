+++
id = "spec-743a7e"
kind = "spec"
title = "Channel Config Schema"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "features"
subsystem = ["roko-core/config"]
created = 2026-09-21
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/backlog/archive/426-channel-config-schema.md#426 — Channel Config Schema"
discovered_from = "audit:tmp/backlog/archive/426-channel-config-schema.md#426 — Channel Config Schema"
anchors = ["crates/roko-core/src/config/mod.rs", "crates/roko-core/src/config/channels.rs::ChannelConfig", "crates/roko-core/src/config/platforms.rs::PlatformConfig", "crates/roko-core/src/config/schema.rs::RokoConfig", "crates/roko-core/src/config/loader.rs::build_schema_tree", "crates/roko-core/src/config/validation.rs", "crates/roko-cli/src/commands/config_cmd.rs", "crates/roko-cli/src/doctor.rs::run_doctor"]
links = { depends_on = [], blocks = [], related = ["gap-8d80d3", "spec-8bc165"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qE '^\\s*pub mod channels;' crates/roko-core/src/config/mod.rs && grep -qE '^\\s*pub channels:' crates/roko-core/src/config/schema.rs && grep -rqw 'fn roko_config_parses_channels_section' crates/roko-core/src/ && cargo test -p roko-core roko_config_parses_channels_section"
+++

## Problem

`roko.toml` has no working way to declare messaging platforms or channels (Telegram, Discord,
Slack, Matrix, WhatsApp, Mattermost). `RokoConfig` (`crates/roko-core/src/config/schema.rs:92`)
has no `channels` or `platforms` field. When loading, the loader diagnoses any `[channels]`,
`[[channels]]` or `[[platforms]]` table as an unknown key and strips it (`strip_unknown_fields`,
`crates/roko-core/src/config/loader.rs:607`, `:2043`). There is also no `roko config channels list`
command and no channel check in `roko doctor`.

Commit 9c6ec420c (2026-09-25) added `crates/roko-core/src/config/channels.rs` and
`crates/roko-core/src/config/platforms.rs`, but `crates/roko-core/src/config/mod.rs` never declares
them. They are orphan files that are not compiled. Nobody knows whether they compile.

The spec this item was imported from (backlog #426) asks for a single config section that defines
what a channel is, how its credentials are resolved, and how enabled channels are validated at
startup. It is the base that the channel adapters build on.

## Why it matters

Goal `features`. Every channel feature depends on a stable config schema, so this item blocks:
- `gap-63055e`: the platforms and output transports umbrella;
- `gap-8d80d3`: the channel-to-reactive-agent binding. Its orphan `ChannelBindingRouter` already
  imports `roko_core::config::channels::ChannelConfig`;
- `spec-b3ab28`: Telegram;
- `spec-b4ba72`: the `ChatBridge` abstraction.

The orphan `crates/roko-runtime/src/platforms.rs:57` re-exports
`roko_core::config::platforms::{AgentIdentity, PlatformConfig, PlatformSecretRef}`. That module
cannot compile until roko-core declares `pub mod platforms`.

## Where

- `crates/roko-core/src/config/mod.rs` has the module list (:10-34). It is missing `platforms` and
  `channels`.
- `crates/roko-core/src/config/platforms.rs` is orphan. It holds `PlatformSecretRef` (:34, either
  `{ env = "X" }` or a literal, with `resolve()` at :48), `AgentIdentity` and `PlatformConfig`
  (:125: `id`, `kind: String` (:134), `token`, `description`, `enabled`, `max_retries`,
  `retry_delay_ms`, `identity`, `extra` (:175)). The format is `[[platforms]]`.
- `crates/roko-core/src/config/channels.rs` is orphan. It holds `ChannelConfig` (:35:
  `platform_id`, `channel_id`, `agent_name: Option<String>`, `trigger_bindings: Vec<String>`) plus
  `has_action()` and `display_name()`. The format is `[[channels]]`.
- `crates/roko-core/src/config/schema.rs::RokoConfig` (:92) is where the new fields go. Its
  `Default` impl is at :422, and `agents: Vec<AgentDefinition>` (:164) is what `agent_name` must
  match.
- `crates/roko-core/src/config/loader.rs`:
  - `build_schema_tree` (:1431) must get sentinel entries for the new sections, as
    `subscriptions` does at :1519. Without them, the loader strips the section.
  - `validate_known_config_paths` (:1417) reports unknown keys.
  - `merge_global_config_into` (:2104) merges the global and project configs.
  - `SECRET_KEY_FRAGMENTS` (:1771) drives redaction. It includes `token`.
- `crates/roko-core/src/config/validation.rs` is where the channel validation goes.
- The CLI config subcommand enum is in `crates/roko-cli/src/main.rs`. Add `Channels` next to
  `Subscriptions` (:2960). The handler goes in `crates/roko-cli/src/commands/config_cmd.rs`, next to
  `ConfigSubscriptionCmd::List` (:228).
- In `crates/roko-cli/src/doctor.rs`, `run_doctor` (:216) collects the `check_*` functions.
- Orphan consumers in roko-runtime, out of scope here, show the shape they expect:
  `platforms.rs::PlatformRegistry::from_configs(&[PlatformConfig])` (:414),
  `adapters/mod.rs::platform_bridge_for_config`, which matches on `config.kind.as_str()`, and
  `channel_binding.rs::ChannelBindingRouter`.

## Current state

- At HEAD, none of the spec's acceptance criteria are met in compiled code.
- The orphan code uses a different, two-level shape from the spec. `[[platforms]]` holds
  connections and credentials. `[[channels]]` binds one platform channel to an agent or to trigger
  bindings. The spec instead wanted one map, `channels: HashMap<String, ChannelConfig>`, whose
  entries carry `name`, a `kind: ChannelKind` enum with 7 variants, `enabled`, `access_control` and
  a flattened per-platform struct.
- The current `[[verify]]` encodes the spec's shape (`HashMap<String, ChannelConfig>` plus
  `pub enum ChannelKind`), so it does not fit the recommended plan below.
- Spec details worth keeping: credentials are named by an env var, never stored inline.
  `ChannelAccessControl { allowed_user_ids, admin_user_ids, require_approval }`. Validation rules:
  an enabled channel with no credential field is an error, and one whose env var is unset is a
  warning. There is at most one binding per (kind, bot token env) pair.
- The CLI table the spec wants: `Name  Kind  Enabled  Credentials` with rows like
  `telegram  telegram  yes  TELEGRAM_BOT_TOKEN ✓`.

## Plan

The design choice:
- (A) Implement the spec's single `HashMap<String, ChannelConfig>` with flattened per-platform
  structs, then rewrite the orphan roko-runtime code to match.
- (B) Keep the orphan two-level shape (`[[platforms]]` plus `[[channels]]`), which the roko-runtime
  registry, the adapters and `ChannelBindingRouter` already use, and add the spec's missing parts
  to it.

Recommend (B). It matches all the existing consumers. It also keeps credentials (one per platform)
apart from routing (many channels per platform), which is what the spec's "one binding per bot
token" rule was reaching for.

1. Declare `pub mod platforms;` and `pub mod channels;` in `config/mod.rs` and re-export
   `PlatformConfig`, `PlatformSecretRef`, `AgentIdentity` and `ChannelConfig`. Fix any compile errors
   in the two files. They have never been compiled.
2. Add a typed kind, `pub enum ChannelKind { Telegram, Discord, Slack, Matrix, WhatsApp, Mattermost,
   Custom }`, with `#[serde(rename_all = "snake_case")]` and `#[serde(rename = "whatsapp")]` on
   `WhatsApp`, because snake_case alone produces `whats_app`. Use it as the type of
   `PlatformConfig::kind`. If you would rather keep `kind: String` so that the orphan
   `platform_bridge_for_config` still matches, validate the string against the same list.
3. Add `#[serde(default)] pub access_control: ChannelAccessControl` to `ChannelConfig`. Document
   what an empty `allowed_user_ids` means. Recommend deny-all, following the fail-closed rule in
   `CLAUDE.md`.
4. Add `#[serde(default, skip_serializing_if = "Vec::is_empty")] pub platforms: Vec<PlatformConfig>`
   and `pub channels: Vec<ChannelConfig>` to `RokoConfig`, and update its `Default` impl. Add
   sentinel entries in `build_schema_tree`, or the loader strips both sections. Check how
   `merge_global_config_into` handles `Vec` sections such as `subscriptions`, and do the same.
5. Validation in `validation.rs`:
   - platform `id`s are unique;
   - each channel's `platform_id` names a declared platform;
   - `agent_name` names an `[[agents]]` entry;
   - an enabled platform must have `token`. It is an error if the field is missing and a warning if
     `PlatformSecretRef::resolve` fails;
   - a `Literal` token (a secret in a file) gives a warning;
   - no two platforms share (kind, token env);
   - a channel whose `has_action()` is false gives a warning.
6. Add `roko config channels list`, with one row per channel: name (`display_name()`), kind,
   enabled, and credential status (env var name plus set or unset). Never print a secret value.
   Follow the sibling `subscriptions list` for any JSON flag.
7. In `doctor.rs`, add `check_channels(&LoadedConfig) -> Vec<DoctorCheck>` and call it from
   `run_doctor`. It reports credential status for each enabled platform, warns when platforms are
   configured but none is enabled, and fails when an enabled platform has no credential.
8. Tests:
   - `roko_config_parses_channels_section` loads a `roko.toml` string with two platforms and two
     channels through the real loader path (`deserialize_migrated_toml` in loader tests, or
     `load_config_file` on a temp file) and asserts that both sections survive;
   - validation tests, one per rule;
   - a redaction test: `serialize_effective_redacted` hides a literal token but still shows the env
     var name in `token = { env = "X" }`.

## Done when

- `[[platforms]]` and `[[channels]]` in `roko.toml` load into `RokoConfig` without "unknown key"
  diagnostics, and `roko config show` displays them with secrets redacted.
- Invalid references (unknown `platform_id` or `agent_name`) and missing credentials produce the
  validation errors or warnings above.
- `roko config channels list` and `roko doctor` report per-channel status.
- Verify (suggested replacement for the current shape-specific command):
  `grep -qE '^\s*pub mod channels;' crates/roko-core/src/config/mod.rs && grep -qE '^\s*pub channels:' crates/roko-core/src/config/schema.rs && grep -rqw 'fn roko_config_parses_channels_section' crates/roko-core/src/ && cargo test -p roko-core roko_config_parses_channels_section`

## Notes

- Scope is the config schema, validation, the CLI list and doctor only. Do not wire the roko-runtime
  orphan modules (`platforms.rs`, `adapters/`, `channel_binding.rs`, `delivery.rs` and others from
  9c6ec420c). That work belongs to `gap-63055e`, `gap-8d80d3`, `spec-b4ba72` and `spec-b3ab28`.
  The adapters are stubs that send nothing.
- If you choose (A), you must also update the `[[verify]]` command, the orphan roko-runtime
  consumers and `gap-8d80d3`'s plan.
- Secrets: env-var references only in examples and docs. Never log or print token values.
- This touches the shared config schema and loader (`schema.rs`, `loader.rs`), as does `gap-7a3527`.
  Do not run it in parallel with other config-schema items. The 2026-09-28 check saw uncommitted
  concurrent edits in `schema.rs`, so run `git status` first.
- The spec said size S. It is M, because of the orphan code, the schema-tree sentinels and the
  merge behaviour.

## Original notes

foundation for all channel/webhook composability; every channel adapter and cross-platform feature depends on this. Roko has per-platform config scattered across backlog items (#224, #225) and the existing `PlatformConfig` / `PlatformKind` concepts in #224, but there is no committed `[channels]`…

Imported without verification from:
- `tmp/backlog/archive/426-channel-config-schema.md#426 — Channel Config Schema`

Some cited files are gone: `crates/roko-cli/src/commands/config.rs`, `crates/roko-core/src/config/channel.rs`.

How to verify: Check: `ChannelKind` enum with 7 variants exists in `roko-core`; `ChannelConfig` struct with `name`, `kind`, `enabled`, `access_control`, and; `RokoConfig.channels` field is a `HashMap<String, ChannelConfig>` [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]

Verified 2026-09-28: commit 9c6ec420c added crates/roko-core/src/config/channels.rs (a `[[channels]]` table; ChannelConfig {platform_id, channel_id, agent_name, trigger_bindings}) and config/platforms.rs. Neither is declared in config/mod.rs, so both are orphan and uncompiled. RokoConfig (config/schema.rs, which has uncommitted concurrent edits) has no channels field, and the spec's ChannelKind (7 variants) and HashMap<String, ChannelConfig> shape is not implemented. The import's 'gone' warning for config/channel.rs is only a path difference (channels.rs).
