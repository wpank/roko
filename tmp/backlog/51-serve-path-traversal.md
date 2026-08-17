# Serve Agent Name Validation Hardening

**Priority**: P1 — security
**Size**: S (half day)
**Crate**: `crates/roko-serve/src/routes/agents.rs`

---

## Problem

The `POST /api/agents/create` endpoint validates agent names against path traversal
attacks (refuse separators, `..`, non-Normal components, and verify canonicalized paths
stay under `.roko/agents/`). However, the validation uses a blocklist approach that
permits names containing characters that are problematic on some filesystems or that
could confuse downstream consumers:

1. Unicode homoglyphs, control characters, and zero-width characters are accepted.
   A name like `\u{200B}admin` (zero-width space + "admin") passes validation and
   creates a directory that looks identical to `admin` in many terminals.
2. Names containing shell metacharacters (`$`, `` ` ``, `|`, `;`, `&`, `(`, `)`) are
   accepted. While these don't cause path traversal, they can cause problems when
   agent names are interpolated into shell commands by downstream tooling.
3. The `req.domain` field has no validation at all — it is used as a TOML sub-table
   key via `BTreeMap<String, toml::value::Table>`. While `toml::to_string_pretty`
   safely quotes the key, an empty or excessively long domain string is accepted.
4. Agent names can be up to the filesystem's maximum filename length (typically 255
   bytes), but there is no explicit length cap. Very long names can cause issues with
   path length limits on some systems.

---

## Section A: Current State

**A1.** `resolve_agent_dir()` at `crates/roko-serve/src/routes/agents.rs` line 773
validates the agent name:
- Rejects empty names (line 775)
- Rejects names containing `/` or `\` (line 778)
- Rejects `.` and `..` (line 783)
- Requires exactly one `Normal` path component (line 788)
- Canonicalizes the parent directory and verifies containment (line 804)

**A2.** The TOML manifest is built via `toml::to_string_pretty(&AgentManifest)` at
line 671, which safely quotes all string values. This defeats TOML injection attacks.
Tests at lines 2018-2037 verify that hostile prompts are safely serialized.

**A3.** Path traversal via `../` is caught and tested at lines 1976-1990.

**A4.** No character-set restriction exists on the agent name. The check is purely
structural (path components), not character-level.

**A5.** `req.domain` is used at line 666 as a key in `BTreeMap<String, toml::value::Table>`.
No validation is applied to it.

---

## Section B: What To Do

**B1.** Add a character-set allowlist for agent names: `[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}`.
This is a strict ASCII alphanumeric + hyphen + underscore regex, 1-64 characters, starting
with an alphanumeric character. Reject anything else with 400 Bad Request and a message
that explains the allowed character set.

**B2.** Add the same character-set validation for `req.domain`: `[a-zA-Z][a-zA-Z0-9_-]{0,63}`.
Domains must start with a letter (to be valid TOML bare keys and valid identifiers in
downstream tooling).

**B3.** Add explicit length caps: agent name max 64 characters, domain max 64 characters.
Return 400 with a clear message when exceeded.

**B4.** Reject agent names that contain ASCII control characters (0x00-0x1F, 0x7F)
or non-ASCII bytes. The allowlist in B1 achieves this, but add an explicit check with
a specific error message for non-ASCII input so international users get a clear
explanation rather than a generic "invalid character" error.

**B5.** Add a test that exercises the new validation:
- Names with Unicode homoglyphs → 400
- Names with shell metacharacters (`$foo`, `a;b`) → 400
- Names with control characters → 400
- Names longer than 64 characters → 400
- Empty domain → 400
- Domain with non-letter start (`1domain`) → 400
- Valid names (`my-agent-1`, `test_agent`) → 201

---

## Acceptance criteria

- [ ] Agent names restricted to `[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}`
- [ ] Domain names restricted to `[a-zA-Z][a-zA-Z0-9_-]{0,63}`
- [ ] 400 response with clear error message for each rejection case
- [ ] Non-ASCII input gets a specific "ASCII only" error message
- [ ] Existing path-traversal and TOML-injection tests still pass
- [ ] New tests cover Unicode homoglyphs, shell metacharacters, control characters, length limits, and domain validation
- [ ] Manual verification: `curl -X POST localhost:6677/api/agents/create -d '{"name":"$evil","domain":"d"}' -H 'Content-Type: application/json'` returns 400

### Not in scope
- Changing the agent manifest schema or TOML structure
- Symlink following policy
- Renaming or migrating existing agents with non-conforming names
- Adding Unicode/i18n support for agent names
