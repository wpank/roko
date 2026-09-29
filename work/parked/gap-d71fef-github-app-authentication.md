+++
id = "gap-d71fef"
kind = "gap"
title = "GitHub App Authentication"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-mcp-github"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/431-github-app-authentication.md#431 — GitHub App Authentication"
discovered_from = "audit:tmp/backlog/archive/431-github-app-authentication.md#431 — GitHub App Authentication"
anchors = ["crates/roko-mcp-github/src/lib.rs", "crates/roko-core/src/config/serve.rs", "crates/roko-serve/src/routes/webhooks.rs", "crates/roko-cli/src/commands/github.rs", "crates/roko-core/src/config/schema.rs", "crates/roko-mcp-github/src/jwt.rs", "crates/roko-mcp-github/src/app_auth.rs", "crates/roko-cli/src/commands/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
PAT auth works but couples all automation to one user's token with broad scopes; GitHub App auth provides per-repo permissions, higher rate limits (separate 5000/hr pool per installation), and org-owned identity. Roko's GitHub integration authenticates exclusively via a PAT read from…

Imported without verification from:
- `tmp/backlog/archive/431-github-app-authentication.md#431 — GitHub App Authentication`

Some cited files are gone: `crates/roko-mcp-github/src/app_auth.rs`, `crates/roko-mcp-github/src/jwt.rs`.

How to verify: Check: `[github.app]` config section parsed with `app_id`, `installation_id`, `private_key_path`, `private_key_env`; JWT signing produces valid RS256 JWT accepted by GitHub's `/app` endpoint; Installation access token exchange succeeds and returns… [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]
