+++
id = "spec-90f395"
kind = "spec"
title = "GitLab and Gitea Webhook Ingress Support"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/routes"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/419-gitlab-gitea-webhook-support.md#419 — GitLab and Gitea Webhook Ingress Support"
discovered_from = "audit:tmp/backlog/archive/419-gitlab-gitea-webhook-support.md#419 — GitLab and Gitea Webhook Ingress Support"
anchors = ["crates/roko-serve/src/routes/webhooks.rs", "crates/roko-core/src/config/serve.rs", "webhooks.rs", "crates/roko-core/src/config/schema.rs", "crates/roko-core/src/signal_kinds.rs", "WebhooksConfig", "GithubWebhookConfig", "verify_github_signature()"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
limits trigger system to GitHub-only forge integration. The webhook ingress layer in `crates/roko-serve/src/routes/webhooks.rs` currently supports three providers:

Imported without verification from:
- `tmp/backlog/archive/419-gitlab-gitea-webhook-support.md#419 — GitLab and Gitea Webhook Ingress Support`

How to verify: Check: `POST /webhooks/gitlab` verifies `X-Gitlab-Token` and produces typed signals.; `POST /webhooks/gitea` verifies `X-Gitea-Signature` (HMAC-SHA256) and produces; GitLab push, MR (open/merge/close), pipeline, issue, and note events map to [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]
