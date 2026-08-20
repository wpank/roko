# 51 — Serve Agent Name Validation Hardening

**Priority**: P1 — security: agent names accept Unicode homoglyphs and shell metacharacters
**Size**: S (half day)
**Crates**: `crates/roko-serve` (`src/routes/agents.rs`)
**Depends on**: None

---

## Background

The `POST /api/agents/create` endpoint writes a TOML manifest to `.roko/agents/<name>/manifest.toml` and registers the agent in the discovery registry. The endpoint validates the agent name against path traversal attacks (rejects `/`, `\`, `..`, multi-component paths, and verifies the canonicalized path stays under `.roko/agents/`). TOML injection is prevented by serializing the manifest through `toml::to_string_pretty()` rather than string interpolation.

However, the validation uses a structural (path-component) approach and does not restrict the character set of the name itself. This allows names that are problematic for reasons other than path traversal:

- **Unicode homoglyphs and control characters**: A name like `\u{200B}admin` (zero-width space + "admin") passes all current checks and creates a directory that looks identical to `admin` in many terminals. Zero-width joiners, bidirectional override characters, and other invisible Unicode can cause confusion or deception.
- **Shell metacharacters**: Names containing `$`, `` ` ``, `|`, `;`, `&`, `(`, `)` pass validation. While these do not cause path traversal, they can cause problems when agent names are interpolated into shell commands by downstream tooling (e.g., scripts, CI pipelines, the `roko agent start` command that constructs a command line from the agent ID).
- **No explicit length cap**: Agent names can be up to the filesystem's maximum filename length (typically 255 bytes), but there is no explicit cap enforced by the API. Very long names are accepted and may cause issues with combined path lengths on some systems.
- **Domain field unvalidated**: The `req.domain` field defaults to `"general"` but accepts any string from the `CreateAgentRequest`. It is used as a key in a `BTreeMap<String, toml::value::Table>` and as a match arm value in `create_agent()`. While TOML serialization safely quotes it, an empty domain or one with invalid characters for TOML bare keys could confuse downstream tooling.

Note: The domain field currently has an allowlist check in `CreateAgentRequest::validate_payload()` (line 618) that accepts only `["coding", "research", "chain", "general"]`. That covers the API-facing domain field. The agent ID used in other endpoints (e.g., `POST /api/agents/{id}/start`) passes through `resolve_agent_dir()` without the same domain check — the ID comes from the filesystem directory name already created during `create_agent`.

## Current State

1. `resolve_agent_dir()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/agents.rs:773` validates agent names:
   - Rejects empty/whitespace-only names (line 775)
   - Rejects names containing `/` or `\` (line 778)
   - Rejects `.` and `..` (line 783)
   - Requires exactly one `Normal` path component (lines 787-795)
   - Canonicalizes the parent directory and verifies containment (lines 804-812)

2. `CreateAgentRequest` at line 576 has a `#[validate]` attribute for `name` that enforces `length(min = 1, max = 128)` and `validate_non_blank`. Length limit is 128 characters.

3. `CreateAgentRequest::validate_payload()` at line 616 checks `domain` against a hardcoded allowlist of 4 values: `["coding", "research", "chain", "general"]`.

4. TOML manifest serialization at line 671 uses `toml::to_string_pretty(&manifest_struct)`. This is safe against TOML injection. Test `agent_manifest_prompt_cannot_inject_toml_table` at line 2007 verifies this.

5. Path traversal tests are at lines 1930-1963. The test at line 1930 covers: `".."`, `"../"`, `"../etc"`, `"../../../etc"`, `"..\\..\\windows"`, `"/etc/passwd"`, `"./hidden"`, `"name/with/slashes"`, `""`, `"   "`. All correctly return 400.

6. The test at line 1958 asserts that `"research-bot.v2"` (a name with a dot) is accepted. The new allowlist (Step 1 below) must decide whether dots are permitted. The current name in tests uses `.v2` — this would be rejected by the strict ASCII alphanumeric+hyphen+underscore pattern. Decide whether dots should be allowed and document the choice.

7. The `agent_config_rejects_path_traversal_ids` test at line 2099 covers the `{id}` route parameter path (used in `start_agent`, `stop_agent`, etc.), which also calls `resolve_agent_dir()`.

## Implementation Plan

**Step 1: Add character-set allowlist to `resolve_agent_dir()`**

Add a character-set check immediately after the existing structural checks in `resolve_agent_dir()`. Use a regex or a manual character check:

```rust
// After the structural checks (line 795), before canonicalization:
fn is_valid_agent_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}
if !trimmed.chars().next().map(char::is_alphanumeric).unwrap_or(false) {
    return Err(ApiError::bad_request(
        "agent name must start with an alphanumeric character"
    ));
}
if !trimmed.chars().all(is_valid_agent_name_char) {
    return Err(ApiError::bad_request(
        "agent name must contain only ASCII alphanumeric characters, hyphens, and underscores"
    ));
}
if !trimmed.is_ascii() {
    return Err(ApiError::bad_request(
        "agent name must contain only ASCII characters"
    ));
}
```

Note: the `is_ascii()` check covers control characters and non-ASCII bytes. The `is_valid_agent_name_char` check covers shell metacharacters. The `is_alphanumeric()` start check covers names starting with `-` or `_`.

Decision point on dots: the existing test at line 1961 uses `"research-bot.v2"`. If dots are rejected by the new allowlist, that test must be updated to use `"research-bot-v2"` or dots must be added to the allowlist. This spec recommends rejecting dots for simplicity (dot-separated names can be confused with file extensions), but document the choice clearly.

**Step 2: Add an explicit length cap**

Update the `#[validate(length(min = 1, max = 128))]` annotation on `name` in `CreateAgentRequest` (line 580) to `max = 64`. The 128-character limit is unnecessarily generous for filesystem-backed names.

Also add a redundant check in `resolve_agent_dir()` after the character-set check:
```rust
if trimmed.len() > 64 {
    return Err(ApiError::bad_request("agent name must not exceed 64 characters"));
}
```

**Step 3: Add a specific error message for non-ASCII input**

The character-set check in Step 1 catches non-ASCII but gives a generic message. Add a specific check that runs first and gives an "ASCII only" message:

```rust
if !trimmed.is_ascii() {
    return Err(ApiError::bad_request(
        "agent name must contain only ASCII characters (no Unicode, accented letters, or special symbols)"
    ));
}
```

**Step 4: Update or add tests**

The existing path-traversal test at line 1930 covers structural attacks. Add a new test function `agent_manifest_resolve_agent_dir_rejects_invalid_chars` that covers:

- Names with Unicode: `"\u{200B}admin"` (zero-width space), `"adm\u{00EF}n"` (accented char) → 400
- Names with shell metacharacters: `"$foo"`, `"a;b"`, `"a|b"`, `"a(b)"`, `"a&b"`, `` "a`b" `` → 400
- Names with control characters: `"a\x01b"`, `"a\x7fb"` → 400
- Names longer than 64 characters → 400
- Valid names: `"my-agent-1"`, `"test_agent"`, `"abc"` → resolved without error
- Update `agent_manifest_resolve_agent_dir_accepts_simple_names` at line 1958 to use `"research-bot-v2"` instead of `"research-bot.v2"` if dots are rejected

## Acceptance Criteria

1. Agent names are restricted to `[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}` (starts with alphanumeric, followed by alphanumeric/hyphen/underscore, max 64 characters total).
2. Names with Unicode characters return 400 with "ASCII only" message.
3. Names with shell metacharacters (`$`, `` ` ``, `|`, `;`, `&`, `(`, `)`) return 400.
4. Names with control characters return 400.
5. Names longer than 64 characters return 400.
6. Empty or whitespace-only names return 400 (already handled, verify unchanged).
7. Path traversal names continue to return 400 (verify existing tests still pass).
8. Valid names (`my-agent-1`, `test_agent`, `abc`) are accepted and resolve correctly.
9. The domain field remains restricted to its existing 4-value allowlist.
10. `cargo test -p roko-serve` passes.
11. Manual: `curl -X POST localhost:6677/api/agents/create -d '{"name":"$evil","domain":"general"}' -H 'Content-Type: application/json'` returns 400.

## Verification Checklist

- [ ] Run the new character-set test and verify all invalid inputs return 400 with informative messages
- [ ] Run the existing path-traversal test at line 1930 and verify it still passes
- [ ] Run the existing TOML-injection test at line 2007 and verify it still passes
- [ ] Run `curl -X POST localhost:6677/api/agents/create -d '{"name":"my-agent-1","domain":"general"}' -H 'Content-Type: application/json'` — returns 201
- [ ] Run `curl -X POST localhost:6677/api/agents/create -d '{"name":"$evil","domain":"general"}' -H 'Content-Type: application/json'` — returns 400
- [ ] Run `curl -X POST localhost:6677/api/agents/create -d '{"name":"très-bien","domain":"general"}' -H 'Content-Type: application/json'` — returns 400 with "ASCII only" message
- [ ] Run `cargo test -p roko-serve` — all tests pass

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/agents.rs` | Add character-set allowlist and explicit length cap to `resolve_agent_dir()` (starting at line 773); add specific "ASCII only" error message; update `CreateAgentRequest` `name` validation to `max = 64` (line 581); add new test function for character-set rejection cases; update `agent_manifest_resolve_agent_dir_accepts_simple_names` at line 1958 if dots are disallowed |
