# Error Taxonomy

> The complete classification of failure types in Roko. Every failure belongs to one of
> five classes. Knowing the class tells you what recovery to apply.

**Status**: Shipping
**Crate**: `roko-orchestrator`, `roko-gate`, `roko-agent`
**Depends on**: [00-overview.md](00-overview.md)
**Used by**: [02-recovery-strategies.md](02-recovery-strategies.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Five error classes. Gate verdicts are expected and handled automatically. Infrastructure
and LLM errors are mostly transient. User errors require human intervention. Safety
errors require human review and are never auto-retried.

---

## Error Class × Recovery Strategy Matrix

| Error class | Examples | Auto-retry? | Auto-escalate? | Circuit break? | Requires human? |
|------------|---------|------------|----------------|----------------|----------------|
| Gate verdict | Compile fail, test fail, diff large, semantic rejection | ✓ (up to max_retries) | ✗ | ✗ | ✗ (unless retries exhausted) |
| Infrastructure — transient | Network timeout, 503 from LLM API, lock contention | ✓ | ✗ | ✓ (after threshold) | ✗ |
| Infrastructure — durable | Disk full, disk I/O error, substrate corruption | ✗ | ✗ | ✗ | ✓ |
| User error | Bad config, missing API key, invalid plan TOML | ✗ | ✗ | ✗ | ✓ |
| LLM — rate limit | 429 Too Many Requests | ✓ (with wait + key rotation) | ✗ | ✗ | ✗ |
| LLM — context exceeded | Context window overflow | ✗ | ✓ (longer context model) | ✗ | ✗ |
| LLM — malformed response | JSON parse failure, truncated response | ✓ (1 retry) | ✗ | ✗ | ✗ (usually) |
| LLM — safety refusal | Content policy rejection from provider | ✗ | ✗ | ✗ | ✓ |
| Safety — policy gate | Roko safety policy rejection | ✗ | ✗ | ✗ | ✓ |
| Safety — role auth | Agent attempts unauthorised action | ✗ | ✗ | ✗ | ✓ |
| Safety — taint | Tainted data flows to untrusted sink | ✗ | ✗ | ✗ | ✓ |

---

## Class 1: Gate Verdicts

Gate verdicts are the most common class of failure. They are **expected** — they
represent the agent producing output that does not yet meet the verification criteria.
They are not errors in the exceptional sense; they are part of the normal task loop.

### Sub-types

| Sub-type | Gate | Verdict reason | Example |
|----------|------|----------------|---------|
| `CompileFail` | `compile` | `cargo check` non-zero exit | Type mismatch `E0308` |
| `TestFail` | `test` | `cargo nextest` failures | `assertion failed` |
| `LintFail` | `clippy` | Clippy warning treated as error | `clippy::unwrap_used` |
| `FormatFail` | `format` | `cargo fmt --check` diff | Extra blank line |
| `DiffTooLarge` | `diff` | Diff exceeds configured churn limit | 5,000 line change on a "small fix" |
| `DiffUnexpectedDeletion` | `diff` | Unexpected deletion of key files | `Cargo.lock` deleted |
| `SemanticRejection` | `semantic` | LLM judge score below threshold | Task spec not met |
| `SecurityIssue` | `security` | `cargo-audit` advisory | Known CVE in dependency |
| `CoverageFail` | `coverage` | Coverage below floor | 60% coverage when 70% required |

**Recovery**: automatic retry with iteration memory. After `gate.max_retries`, the task
is marked `Failed(GateFailed)`.

---

## Class 2: Infrastructure Errors

Infrastructure errors are failures in the environment around Roko — disk, network,
process management.

### Transient (auto-retry)

| Sub-type | When | Recovery |
|----------|------|---------|
| `NetworkTimeout` | LLM API call or MCP tool call exceeds timeout | Retry with exponential backoff (3 attempts: 1s, 4s, 16s) |
| `ServiceUnavailable` | 503 from LLM API | Retry after `Retry-After` header delay |
| `LockContention` | `.roko.lock` held by another process | Wait 5s, retry 3 times, then fail |
| `SubprocessCrash` | Agent subprocess exits unexpectedly | Restart subprocess + retry task |

### Durable (requires human)

| Sub-type | When | Recovery |
|----------|------|---------|
| `DiskFull` | `substrate.max_size_gb` hard limit hit | Run `roko substrate gc`; increase disk |
| `DiskIOError` | `write(2)` fails on substrate append | Check disk health; may indicate hardware failure |
| `SubstrateCorruption` | JSONL file not parseable | Restore from backup or `roko substrate repair` |
| `ProcessKilled` | SIGKILL (OOM, admin kill) | Resume from snapshot if available |

---

## Class 3: User Errors

User errors are caused by operator mistakes. They are detected at startup (config
validation) or at first use (plan parsing). They require human action to fix.

| Sub-type | When | Fix |
|----------|------|-----|
| `InvalidConfig` | `roko.toml` parse error or validation failure | Fix the config key (see [configuration/10-config-validation.md](../configuration/10-config-validation.md)) |
| `MissingApiKey` | `ANTHROPIC_API_KEY` not set | Set the env var |
| `InvalidPlanToml` | Plan file has syntax error | Fix the TOML |
| `MissingPlanDependency` | Plan declares a dependency that doesn't exist | Fix the plan DAG |
| `InvalidMcpConfig` | `.mcp.json` has syntax error | Fix the JSON |
| `FileNotFound` | `agent.system_prompt_path` doesn't exist | Create the file or remove the key |

**All user errors fail fast at startup.** Roko does not attempt to work around a bad
config or missing file.

---

## Class 4: LLM Errors

LLM errors are failures from the external model API.

| Sub-type | When | Recovery |
|----------|------|---------|
| `RateLimit` (429) | Too many requests per minute | Wait + key rotation; auto-retry |
| `ContextWindowExceeded` | Prompt + response > model context limit | Escalate to a longer-context model (if available) or truncate context |
| `MalformedResponse` | Response is not valid JSON / missing required fields | Retry once; if fails again, mark as `LlmMalformedResponse` failure |
| `SafetyRefusal` | Provider's own content policy triggered | Human review required; do not retry (will be refused again) |
| `ServiceError` (500) | Internal server error from LLM API | Retry with exponential backoff |
| `AuthenticationError` (401) | API key rejected | Human action: check/rotate key |
| `ModelNotFound` (404) | `agent.model` slug is invalid | Human action: fix `agent.model` |

---

## Class 5: Safety Errors

Safety errors indicate a policy violation or a security boundary was approached. These
are **never auto-retried** and always require human review.

| Sub-type | When | Recovery |
|----------|------|---------|
| `PolicyGateRejection` | Roko safety gate rejects agent output before execution | Review the agent's proposed action; adjust safety policy if legitimate |
| `RoleAuthFailure` | Agent attempts to call a tool it is not authorised to use | Review agent role configuration; may indicate a prompt injection |
| `TaintPropagation` | Tainted data (from untrusted source) flows to a sensitive sink | Review the data flow; may indicate injection attack |
| `CapabilityViolation` | Agent exceeds its declared capability scope | Review the task scope |
| `PromptInjection` | Pre/post-call checks detect injection pattern | Review the input source; quarantine the offending Engram |

Safety errors are logged at `error` level and persisted as Engrams with the `Safety`
kind. They are also emitted as `SafetyViolation` Pulses.

---

## Error Code Reference

Roko error codes follow the pattern `ROKO-<CLASS>-<NUMBER>`:

| Code | Class | Description |
|------|-------|-------------|
| `ROKO-G-001` | Gate | Compile failed |
| `ROKO-G-002` | Gate | Test failed |
| `ROKO-G-003` | Gate | Lint failed |
| `ROKO-G-004` | Gate | Format failed |
| `ROKO-G-005` | Gate | Diff too large |
| `ROKO-G-006` | Gate | Semantic rejection |
| `ROKO-G-007` | Gate | Security issue |
| `ROKO-I-001` | Infra | Network timeout |
| `ROKO-I-002` | Infra | Service unavailable |
| `ROKO-I-010` | Infra | Disk full |
| `ROKO-I-011` | Infra | Substrate corruption |
| `ROKO-U-001` | User | Invalid config |
| `ROKO-U-002` | User | Missing API key |
| `ROKO-L-001` | LLM | Rate limit |
| `ROKO-L-002` | LLM | Context window exceeded |
| `ROKO-L-003` | LLM | Malformed response |
| `ROKO-L-004` | LLM | Safety refusal |
| `ROKO-S-001` | Safety | Policy gate rejection |
| `ROKO-S-002` | Safety | Role auth failure |
| `ROKO-S-003` | Safety | Taint propagation |

---

## See Also

- [02-recovery-strategies.md](02-recovery-strategies.md) — what recovery applies to each class
- [08-observability.md](08-observability.md) — where each error class surfaces
- [09-failure-drill-examples.md](09-failure-drill-examples.md) — concrete examples per class

## Open Questions

- Error code registry is not yet auto-generated from the Rust enum definitions — a `roko errors list` command is planned.
- The `PromptInjection` detection heuristics are not yet documented in detail.
