# Security Considerations

> How to handle secrets, what the `.roko/` directory contains, and what an operator
> must protect in a production Roko deployment.

**Status**: Shipping
**Crate**: `roko-cli`, `roko-runtime`
**Depends on**: [08-environment-variables.md](08-environment-variables.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

1. API keys go in environment variables — never in `roko.toml`.
2. Add `.roko/` to `.gitignore`.
3. Commit `roko.toml` and `.mcp.json` — they contain no secrets if you follow rule 1.
4. The `.roko/` directory contains state that must not be world-readable on shared systems.

---

## The `.roko/` Directory

`roko init` creates `.roko/` in the project root. Its contents:

```
.roko/
  state/
    executor.json          ← current plan execution state (resume point)
    *.snapshot             ← per-task snapshots (created on crash/cancel)
  logs/
    roko.log               ← structured log output
    roko.log.1             ← rotated log
  substrate/
    engrams.jsonl          ← all persisted Engrams
    episodes.jsonl         ← learning episodes
    playbook-rules.jsonl   ← promoted patterns
  episodes/
    *.jsonl                ← per-run episode records
  .roko.lock               ← process lock (prevents concurrent roko processes)
```

### What To Keep Private

| Path | Sensitivity | Why |
|------|-------------|-----|
| `.roko/state/executor.json` | Medium | Contains task descriptions, file paths, and partial LLM outputs |
| `.roko/substrate/engrams.jsonl` | Medium–High | Contains all agent outputs, including any data the agent retrieved |
| `.roko/logs/roko.log` | Low–Medium | Contains task names, gate results, and timing data. May contain file paths |
| `.roko/episodes/*.jsonl` | Low | Task metadata, token counts, costs. No content |
| `.roko/.roko.lock` | None | Ephemeral; recreated on start |

On multi-user systems, ensure `.roko/` is readable only by the process owner:

```bash
chmod 700 .roko/
```

### `.gitignore` Entry

```
# .gitignore
.roko/
```

Never commit `.roko/` to version control. It contains ephemeral state and potentially
sensitive task outputs.

---

## Secrets: What Goes Where

### Never in `roko.toml`

The following must never appear in `roko.toml`:

- LLM API keys (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, etc.)
- Database passwords (in `substrate.data_dir` or custom MCP server configs)
- GitHub tokens, cloud provider credentials
- Any bearer token or secret

`roko.toml` is designed to be committed to version control. If it contains a secret,
that secret is now in your git history (and potentially in GitHub, CI logs, etc.).

### In Environment Variables

Set API keys via environment variables:

```bash
# Local development: use a .env file (add to .gitignore)
export ANTHROPIC_API_KEY=sk-ant-...

# Production: use your secrets manager
# AWS: use Secrets Manager or Parameter Store
# Kubernetes: use a Secret object
# Vault: use the Vault agent injector
```

### In `.mcp.json`

`.mcp.json` supports `${VAR_NAME}` interpolation. Use this for any secret that an MCP
server needs:

```json
{
  "mcpServers": {
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_TOKEN": "${GITHUB_TOKEN}"
      }
    }
  }
}
```

`GITHUB_TOKEN` comes from the environment. `.mcp.json` itself contains no secret and
is safe to commit.

---

## LLM Backend API Key Security

### Key Rotation (Anthropic)

For high-traffic deployments, set multiple Anthropic API keys to enable round-robin
rotation on rate limits:

```bash
export ANTHROPIC_API_KEY=sk-ant-key1
export ANTHROPIC_API_KEY_2=sk-ant-key2
export ANTHROPIC_API_KEY_3=sk-ant-key3
```

Roko cycles through available keys on 429 (rate limit) responses. This reduces the
blast radius of a single key being compromised — each key has its own usage tier and
revocation is independent.

### Key Scoping

Where possible, use API keys with the minimum required permissions:

- For Roko's standard use, a key with access to the chat completions API is sufficient.
- The key does not need access to fine-tuning, billing, or user management APIs.

### Key Rotation Policy

Rotate API keys:

- After any team member departure.
- After any suspected exposure (commit, log, screenshot).
- On a scheduled basis (every 90 days is a reasonable default for team deployments).

---

## Process Isolation

Roko agents run as child processes of the `roko` binary. The safety layer in `roko-agent`
applies role-based access control and pre/post-call checks, but these are application-level
controls. For production deployments, pair them with OS-level isolation:

- **Containers**: Run `roko` in a container with a read-only filesystem where possible,
  and explicit volume mounts for `.roko/` and the project directory.
- **Namespaces/seccomp**: The `shell` built-in tool runs arbitrary commands. If agents
  will use it, restrict the process with seccomp or a restrictive AppArmor/SELinux
  profile.
- **Network isolation**: If agents should not have arbitrary outbound network access,
  restrict egress at the network level. Roko's LLM backend calls are the only required
  outbound connections (plus any MCP server dependencies).

---

## The `.roko.lock` File

`.roko/.roko.lock` prevents two `roko` processes from running against the same project
directory simultaneously. It contains the PID of the running process.

If Roko crashes without cleaning up the lock, the next invocation will warn that the
lock is stale and offer to remove it:

```
Warning: .roko/.roko.lock exists (PID 12345). Previous process appears to have crashed.
Remove the lock file and resume? [y/N]
```

Do not remove the lock manually unless you are certain no other `roko` process is
running against this directory.

---

## Playbook and Episode Security

The playbook (`playbook.toml`) and the episode store are derived from task executions.
In environments where task content is confidential:

- Store the episode store and playbook on encrypted volumes.
- Restrict read access to the episode store to the `roko` process user.
- Do not commit `playbook.toml` to public repositories if the patterns reveal internal
  project details.

For shared team deployments, a shared playbook path on a restricted NFS or S3 mount
is appropriate:

```toml
[learn]
playbook_path = "/mnt/roko-secrets/playbook.toml"
episode_store = "/mnt/roko-secrets/episodes"
```

---

## See Also

- [08-environment-variables.md](08-environment-variables.md) — complete env var reference
- [07-mcp-config.md](07-mcp-config.md) — environment variable interpolation in `.mcp.json`
- [operations/error-handling/01-error-taxonomy.md](../error-handling/01-error-taxonomy.md) — safety error class

## Open Questions

- Encryption-at-rest for the substrate (encrypted JSONL or SQLite with SQLCipher) is not yet implemented.
- Audit log tamper protection (HMAC-chained log entries) is not yet implemented.
- Roko does not yet have an explicit "zero-trust" network mode that restricts LLM calls to a specific egress allowlist.
