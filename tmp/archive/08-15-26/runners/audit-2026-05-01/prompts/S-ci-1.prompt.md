# S-ci-1: Allowlist format + initial entries

## Task
Create `scripts/fitness/allowlist.toml` with the allowlist schema and seed it with current findings — each entry has owner, reason, expiry, and links to a migration plan.

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/27-ci-fitness-checks.md` § Phase 1, 5.

## Exact changes

### 1. `scripts/fitness/allowlist.toml`

```toml
# audit-2026-05-01 fitness allowlist
#
# Each entry: a finding the script would otherwise flag, marked as
# acknowledged with a reason and expiry.
#
# Required fields per entry: file, pattern, reason, owner, expires.
# Optional: linked_issue, migration_plan.
#
# An entry is valid only if `expires` is in the future.

[[raw_provider_http]]
file = "crates/roko-cli/src/dispatch_direct.rs"
pattern = "reqwest::Client"
reason = "Legacy direct dispatch path; behind feature gate `legacy-direct-dispatch`. T5-37 quarantines; deletion follow-up tracked."
owner = "@will"
expires = "2026-08-01"
linked_issue = "T5-37"
migration_plan = "tmp/subsystem-audits/implementation-plans/22-dispatch-streaming-completion.md"

[[raw_provider_http]]
file = "crates/roko-serve/src/routes/connectors.rs"
pattern = "reqwest::Client"
reason = "Non-LLM external API calls (GitHub, Linear, Slack). Not subject to ModelCallService."
owner = "@will"
expires = "2027-01-01"

[[raw_provider_http]]
file = "crates/roko-serve/src/routes/agents.rs"
pattern = "reqwest::Client"
reason = "Proxy to externally-registered agents (T0-5 SSRF-validated). Non-LLM forwarding; T5-36c migrates self-hosted path only."
owner = "@will"
expires = "2027-01-01"

[[oversized_function]]
file = "crates/roko-cli/src/orchestrate.rs"
function = "dispatch_agent_with"
lines = 2059
max_lines = 200
reason = "Tracked in T5-35; extraction in 4 slices (a/b/c/d). After completion, function is < 100 lines."
owner = "@will"
expires = "2026-09-01"
linked_issue = "T5-35"

[[dangerous_perms]]
# (no entries; T1-12's strict validator rejects bare flags. Local typed
# overrides are allowed via DangerousPermissionOverride and don't appear
# as findings.)
```

### 2. Seed real findings

After creating the schema, run inventory mode:

```bash
bash scripts/roko-fitness-checks.sh inventory
```

For each finding the script lists, decide:

- **Real follow-up planned**: add an allowlist entry pointing to the plan.
- **Truly intentional and forever**: add with `expires` 1+ year out and a clear reason.
- **Was actually a bug**: don't allowlist; fix in a separate batch.

Commit the allowlist with seeded entries.

## Write Scope
- `scripts/fitness/allowlist.toml` (new)
- `scripts/roko-fitness-checks.sh` (only if extending to load the allowlist; full implementation is S-ci-2)

## Read-Only Context
None.

## Verify

```bash
ls scripts/fitness/allowlist.toml

# Schema validity
toml-lint scripts/fitness/allowlist.toml 2>&1 | head -5
# Or: cargo run -p roko-tooling --bin allowlist-validate scripts/fitness/allowlist.toml
```

## Do NOT

- Do NOT add entries without owner + reason + expiry.
- Do NOT use broad regex patterns. One file/pattern pair per entry.
- Do NOT bundle with S-ci-2/3/4.
- Do NOT add the allowlist to `.gitignore`. It's source-controlled.
- Do NOT set `expires` more than 1 year out without a clear "permanent" justification.
