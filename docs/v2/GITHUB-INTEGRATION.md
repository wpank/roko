# GitHub Integration

> **Implementation status:** PARTIAL — authenticated GitHub MCP access, typed webhook
> ingestion and trigger graduation, and repository diagnostics (`roko github status`) are
> live. Of the runner workflow, plan runs only file task-failure issues: they create no
> branches or pull requests on GitHub, post no comments, push nothing, and neither poll CI
> nor merge. See [Runner workflow](#runner-workflow).

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
Plan runs do not push yet (see [Runner workflow](#runner-workflow)). The delivery service's
publish step, which plan runs leave off, runs `git push origin` with prompts disabled, so it
would need an SSH key or credential helper for the remote; the token is never placed in git
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

These branches are local: plan runs integrate plans on a local `roko/batch/<run-id>` branch
and push nothing. Task-attempt worktrees use a more specific internal branch convention.
Operators who open plan pull requests themselves should name the head branch with the
`roko/plan/` prefix, which `roko github status` filters on, and must not treat task-attempt
branches as merge targets.

## Runner workflow

With repository coordinates, a non-empty `GITHUB_TOKEN`, and `auto_pr = true`, a plan run
opens one issue labelled `<label_prefix>task-failure` for each task it leaves failed. The
issue quotes the task's details and its error, with known secrets and key-shaped strings
redacted. A run that was interrupted or cancelled opens none. A failed GitHub call is logged
and does not change the plan's result. Nothing records the issue yet, so a task that fails
again in a later run gets another issue, and none is closed when its task passes.

The rest of the workflow is not built yet:

| Step | Status |
|---|---|
| Create `roko/plan/<plan-id>` on GitHub and open a draft PR | Not wired: `GitHubOps::create_plan_branch` and `open_pr` have no caller |
| Comment on the PR for each terminal task gate and the plan (`auto_update_prs`) | Not built |
| Close a task-failure issue once its task passes | Not built |
| Push the accepted commit to the remote plan branch after the merge regression | Off: batch delivery runs with `publish: false` |
| Poll CI and merge with the configured `merge_method` | Not wired: `check_ci_status` and `merge_pr` have no caller |

The work item gap-cd51b7 tracks these steps.

Inspect the effective integration without a running server:

```bash
roko github status
roko --json github status
```

The report includes config validity, authentication, open `roko/plan/` PRs with CI state,
and open `<label_prefix>task-failure` issues. A missing token produces a successful local
diagnostic with remote sections marked skipped.

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

`.github/workflows/plan-validate.yml` runs on pull requests, and on pushes to `main`, that
touch plan files or the plan-validation code. It builds `roko-cli` without provider
credentials, validates every tracked plan directory except test fixtures and
`plans/archive/`, and then checks the generated plans index for drift:

```bash
target/debug/roko plan validate <plan-dir>   # once per tracked tasks.toml
target/debug/roko plan index --check --workdir .
```

A plan that fails validation, or a stale index, fails the job; the job does not use
`continue-on-error`.

## Troubleshooting

- `GITHUB_TOKEN is not set`: export a non-empty token in the process environment.
- GitHub returns `401` or `403`: verify token permissions and repository ownership.
- GitHub tools are missing: run `roko doctor`, confirm the binary is on `PATH`, and inspect
  the discovered `.mcp.json` or generated `.roko/mcp-auto.json`.
- Webhooks return `401`: verify `webhooks.github.secret` and the exact raw-body signature.
- A failed task opened no issue: confirm `auto_pr = true`, `[github].owner` and `repo`, and
  `GITHUB_TOKEN`. With `auto_pr` on, the run log says why GitHub automation is off.
- A plan is absent from GitHub views: plan runs do not push plan branches or open PRs yet.

For general command behavior see [CLI Reference](CLI-REFERENCE.md). For deployment and
secret injection see [Deployment](25-DEPLOYMENT.md).
