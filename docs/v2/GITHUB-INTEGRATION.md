# GitHub Integration

> **Implementation status:** IMPLEMENTED — authenticated GitHub MCP access, typed webhook
> ingestion and trigger graduation, repository diagnostics, and the runner-managed
> branch/PR/issue/CI/merge lifecycle are live.

Roko has two complementary GitHub boundaries:

- `roko-mcp-github` exposes GitHub tools to model-driven MCP sessions.
- `[github]` config identifies the repository used by runner and operator commands.

Inbound GitHub webhooks are a third, independent boundary. They authenticate with
`GITHUB_WEBHOOK_SECRET`; outbound API and MCP calls authenticate with `GITHUB_TOKEN`.

## Credentials

Create a token with only the repository permissions required by the operations you enable.
Provide it through the environment; do not put a plaintext token in `roko.toml` or commit it
to `.mcp.json`.

```bash
export GITHUB_TOKEN="github-token-placeholder"
export GITHUB_WEBHOOK_SECRET="webhook-secret-placeholder"
```

`roko-mcp-github` fails with an actionable error when `GITHUB_TOKEN` is absent or empty.
Webhook delivery fails authentication when the configured secret or
`X-Hub-Signature-256` signature is missing or invalid.
Publishing accepted commits uses `git push origin` with prompts disabled, so configure an
SSH key or credential helper for the remote separately; the token is not placed in git
command arguments.

## Repository configuration

Repository identity and runner workflow preferences belong in `roko.toml`:

```toml
[github]
owner = "my-org"
repo = "my-repo"
default_branch = "main"
auto_pr = false
merge_method = "squash"       # merge | squash | rebase
label_prefix = "roko/"
```

Webhook authentication remains separate:

```toml
[webhooks.github]
secret = "${GITHUB_WEBHOOK_SECRET}"
```

The public ingress endpoint is `POST /webhooks/github`. Roko verifies the GitHub HMAC
signature before persisting or publishing the event. See the
[API reference](API-REFERENCE.md#webhooks) for the HTTP contract.

## MCP setup

The bundled MCP server uses newline-delimited JSON-RPC over stdio:

```json
{
  "servers": [
    {
      "name": "github",
      "transport": "stdio",
      "command": "roko-mcp-github",
      "args": [],
      "env": {
        "GITHUB_TOKEN": "${GITHUB_TOKEN}"
      },
      "tier": "trusted"
    }
  ]
}
```

Save this as `.mcp.json` at the project root. Discovery walks upward from the working
directory and then checks the user-level `.mcp.json`. Explicit Roko/Claude MCP config has
higher precedence. When no GitHub server is configured and a `roko-mcp-github` binary is
discoverable, the CLI writes a generated `.roko/mcp-auto.json` entry for the invocation;
it does not overwrite the user’s configuration.

The MCP catalog includes pull-request, issue, label, repository-file, code-search, commit,
branch, comparison, and Actions-status operations. Provider tool loops advertise only tools
with a live resolver; discovering a definition without an executable client fails closed.

## Branch convention

Runner-managed plan branches use:

```text
roko/plan/<plan-id>
```

Task-attempt worktrees use a more specific internal branch convention. Operators should
filter plan pull requests by the `roko/plan/` prefix and must not treat task-attempt branches
as merge targets.

## Runner workflow

With repository coordinates, a non-empty `GITHUB_TOKEN`, and `auto_pr = true`, a plan run:

1. Creates `roko/plan/<plan-id>` at the run's base commit and opens a draft PR.
2. Posts one structured PR comment for each terminal task gate and a final plan summary.
3. Opens a labeled issue for a terminal task failure and closes it if that task later passes.
4. After the local merge regression passes, pushes the exact cumulative accepted commit to
   the remote plan branch. A rejected push fails closed and leaves the PR open.
5. Polls GitHub CI at 30-second intervals, up to five retries. Only success invokes the
   configured `merge_method`; failure or exhausted pending checks leave the PR open and add
   a diagnostic comment.

GitHub work runs in an ordered background worker, so API calls and CI waits do not block the
runner event loop. Remote errors are visible but do not rewrite the durable local plan result.

Inspect the effective integration without a running server:

```bash
roko github status
roko --json github status
```

The report includes config validity, authentication, open plan PRs with CI state, and open
`<label_prefix>task-failure` issues. A missing token produces a successful local diagnostic
with remote sections marked skipped.

## Webhook signals and subscriptions

Verified GitHub payloads become typed signal kinds such as:

```text
github:pull_request:opened
github:pull_request_review
github:issues:opened
github:check_suite:completed
github:ci:failed
```

Subscriptions match those colon-delimited signal kinds:

```toml
[[subscriptions]]
template = "pr-reviewer"
trigger = "github:pull_request:opened"
concurrency_limit = 2
cooldown_secs = 60
enabled = true

[subscriptions.filter]
repo = ["my-org/my-repo"]
branch = ["main", "release/*"]
```

Webhook handlers authenticate, normalize, persist, and publish. Agent or plan execution is
performed asynchronously by subscribers; the HTTP handler does not run a plan inline.
Exact plan labels graduate issue events to `github:plan:execution_requested`, requested-change
reviews on safe plan branches graduate to `github:plan:replan_requested`, and failed completed
checks graduate to `github:ci:failed`.

## CI plan validation

`.github/workflows/plan-validate.yml` is a blocking pull-request and main-branch gate for
plan TOML changes. It builds `roko-cli` without provider credentials and runs:

```bash
cargo run -p roko-cli -- plan validate --strict plans/
cargo run -p roko-cli -- plan validate --strict tmp/status-quo/backlog/plans/
```

Invalid task IDs, dependency references, schemas, and configured-model references therefore
fail before merge. The job does not use `continue-on-error`.

## Troubleshooting

- `GITHUB_TOKEN is not set`: export a non-empty token in the process environment.
- GitHub returns `401` or `403`: verify token permissions and repository ownership.
- GitHub tools are missing: run `roko doctor`, confirm the binary is on `PATH`, and inspect
  the discovered `.mcp.json` or generated `.roko/mcp-auto.json`.
- Webhooks return `401`: verify `webhooks.github.secret` and the exact raw-body signature.
- A plan is absent from GitHub views: confirm `[github].owner`, `repo`, and the
  `roko/plan/<id>` branch prefix.
- Branch publication fails: confirm `origin` points at the configured repository and has
  non-interactive SSH or credential-helper access; Roko does not force-push.

For general command behavior see [CLI Reference](CLI-REFERENCE.md). For deployment and
secret injection see [Deployment](25-DEPLOYMENT.md).
