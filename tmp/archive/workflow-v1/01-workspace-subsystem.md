# PRD-01 — Workspace Subsystem

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25
**Crate**: `roko-workspace` (new) + integrations into `roko-core`, `roko-cli`, `roko-serve`
**Prerequisites**: PRD-00

---

## 1. What a Workspace Is

A workspace is a directory that has been opened as the current project context. It is the DAW-project analog: when you "open" a workspace, every CLI command, every dashboard page, every TUI tab, every API call operates against that workspace's state, configuration, secrets, and artifacts unless explicitly scoped elsewhere.

A workspace is identified by:

- A **root path** (filesystem directory).
- A **`workspace.toml`** at the root declaring metadata, identity, and configuration.
- A **`.roko/`** runtime directory at the root for state, artifacts, episodes, signals, secrets cache, etc.
- A **registry entry** in `~/.roko/workspaces.json` with name, path, last-opened time, tags.

Workspaces are not git-coupled. A workspace may live inside a git repo, span multiple repos, or live entirely outside version control. Multiple workspaces may exist on the same machine and be switched between freely.

---

## 2. Why First-Class

The current model is "wherever you run roko, `.roko/` materializes in the cwd." That model has three problems:

1. **No identity** — there's no way to refer to a specific workspace without its path.
2. **No switching ergonomics** — to operate on workspace B from inside workspace A you must `cd`.
3. **No inheritance / templates** — every new workspace re-bootstraps configuration from scratch.

Promoting workspace to a first-class concept solves all three and unlocks a richer dashboard experience (workspace switcher in topbar, recents, multi-workspace daemon, cross-workspace knowledge propagation).

---

## 3. workspace.toml Schema

Living at `<root>/workspace.toml`, this is the identity and configuration file for a workspace.

```toml
[workspace]
name             = "nunchi-dashboard"           # required, unique within registry
description      = "React+Vite dashboard for Roko/Korai"
created_at       = "2026-04-25T14:30:00Z"
schema_version   = 1

# Optional: workspace inherits from another. Inheritance merges TOML deep, child wins.
extends          = "~/.roko/workspaces/templates/web-app"

# Optional: tags surfaced in the registry for filtering / grouping.
tags             = ["web", "frontend", "nunchi"]

# Optional: identity used when this workspace publishes artifacts to the marketplace.
[workspace.identity]
publisher       = "@wpank"
license_default = "CC-BY-4.0"

# Capabilities granted at the workspace level. Modules requiring these capabilities
# can run; modules requiring un-granted capabilities prompt for explicit user approval
# at first use, or fail closed if running headless.
[workspace.capabilities]
fs.read         = true
fs.write        = true
net             = true
shell           = true
llm             = true
chain.write     = false                         # off by default; deploy workflows escalate

# Configuration for the workspace's deploy targets. Shape defined in PRD-06.
# Per-workspace because deploy targets are project-specific.
[workspace.deploy]
default_target  = "railway"

[[workspace.deploy.target]]
name        = "railway"
kind        = "railway"
project_id  = "${env.RAILWAY_PROJECT_ID}"
service_id  = "${env.RAILWAY_SERVICE_ID}"

[[workspace.deploy.target]]
name        = "fly-staging"
kind        = "fly"
app         = "nunchi-staging"
region      = "ord"

[[workspace.deploy.target]]
name        = "vercel-prod"
kind        = "vercel"
project     = "nunchi"
team        = "nunchi-trade"

# Default models for this workspace. Override the user-level / system defaults.
[workspace.models]
strategist  = "claude-opus-4-7"
researcher  = "claude-sonnet-4-6"
scribe      = "claude-haiku-4-5-20251001"

# Roles known to this workspace. Inherits from user-level role registry.
[workspace.roles]
extends = ["strategist", "researcher", "scribe", "reviewer", "implementer"]

# Workflows enabled in this workspace. Empty = all enabled. Restricted = explicit list.
[workspace.workflows]
mode    = "open"                                # "open" | "explicit"
allow   = ["*"]
deny    = []

# Triggers active in this workspace. Daemon respects this list.
[workspace.triggers]
enabled = ["manual", "cron", "fs-watch", "github", "webhook"]
disabled = []

# Secrets policy. Actual secrets stored in OS keychain or .roko/secrets.enc
[workspace.secrets]
backend = "keychain"                            # "keychain" | "encrypted-file" | "env-only"
require_explicit_grant = true
```

---

## 4. ~/.roko/workspaces.json — The Registry

The registry is a single user-level JSON file tracking all workspaces known to roko on this machine. Updated atomically on every workspace open / create / rename / delete.

```json
{
  "schema_version": 1,
  "active": "nunchi-dashboard",
  "workspaces": {
    "nunchi-dashboard": {
      "path": "/Users/will/dev/nunchi/nunchi-dashboard",
      "created_at": "2026-04-25T14:30:00Z",
      "last_opened_at": "2026-04-25T18:45:12Z",
      "tags": ["web", "frontend"],
      "schema_version": 1,
      "valid": true
    },
    "roko": {
      "path": "/Users/will/dev/nunchi/roko/roko",
      "created_at": "2026-03-01T10:00:00Z",
      "last_opened_at": "2026-04-25T18:50:33Z",
      "tags": ["rust", "kernel"],
      "schema_version": 1,
      "valid": true
    }
  },
  "templates": {
    "web-app":      "~/.roko/workspaces/templates/web-app",
    "rust-crate":   "~/.roko/workspaces/templates/rust-crate",
    "research":     "~/.roko/workspaces/templates/research"
  },
  "history": [
    {"workspace": "roko", "opened_at": "2026-04-25T18:50:33Z"},
    {"workspace": "nunchi-dashboard", "opened_at": "2026-04-25T18:45:12Z"}
  ]
}
```

The registry tracks `valid: bool` per entry — if a workspace path no longer exists, the entry is marked invalid but kept (a `roko workspace prune` cleans them).

---

## 5. CLI Surface

```
roko workspace                                # Show active workspace info
roko workspace open <name|path>               # Set active workspace; updates registry
roko workspace switch <name|path>             # Alias for open
roko workspace new <path> [--template <name>] # Create workspace.toml + .roko/, register
roko workspace list [--tag <tag>] [--json]    # List registered workspaces
roko workspace recent [-n <count>]            # Show recent workspaces
roko workspace info [<name>]                  # Show metadata for active or named workspace
roko workspace rename <old> <new>             # Rename in registry; workspace.toml updated
roko workspace remove <name> [--purge]        # Unregister; --purge deletes .roko/ contents
roko workspace prune                          # Drop registry entries with missing paths
roko workspace export <name> <out.tar.zst>    # Bundle workspace.toml + .roko/ for transfer
roko workspace import <in.tar.zst> [<path>]   # Reconstruct a workspace from a bundle
roko workspace template list                  # List available workspace templates
roko workspace template create <from> <name>  # Save current workspace as a template
```

`roko workspace` with no args prints the current active workspace name, path, and a one-line summary (workflow count, recent runs, capability flags).

Every other roko command implicitly resolves the active workspace. A `--workspace <name>` flag overrides for one invocation. The `ROKO_WORKSPACE` env var pins the workspace for a shell.

---

## 6. Inheritance via `extends`

A workspace TOML may declare `extends = "<path-or-name>"`. At load time the parent TOML is read, the child TOML is deep-merged on top (arrays append unless explicitly overridden, scalars replace, tables merge recursively). Multiple levels of inheritance are allowed; cycles are detected and rejected.

This solves three real needs:
- **Org-wide defaults**: a team workspace template carries shared models, capabilities, deploy targets, and provider configs. Per-project workspaces extend it.
- **Personal preset**: individual developers carry their preferred role models, keyboard shortcuts, and preferred tooling. Their per-project workspaces extend it.
- **Variant workspaces**: a `staging` workspace extends `prod` and overrides only the deploy target.

---

## 7. Templates

A workspace template is a directory containing `workspace.toml` (with `name = "{{name}}"` placeholder), an optional `.roko/` skeleton, and arbitrary scaffolding files. `roko workspace new <path> --template <name>` materializes the template, substitutes placeholders, and registers the new workspace.

User templates live at `~/.roko/workspaces/templates/<name>/`. Built-in templates ship with roko at `<install>/templates/workspaces/`. The marketplace (PRD-12) supports publishing workspace templates.

Built-in templates ship in v1:
- `default` — minimal workspace.toml, `.roko/` skeleton, no opinion.
- `rust-crate` — Rust development; cargo + clippy + test workflows pre-enabled.
- `web-app` — frontend development; lint + e2e + visual-gate workflows pre-enabled.
- `research` — long-form research workspace; web-enrich + citation-check + knowledge-store pre-enabled.
- `multi-agent` — workspace tuned for fleets of agents; daemon defaults to running, fleet UI default-on.

---

## 8. Multi-Workspace Daemon

`roko daemon` is extended to host triggers across multiple workspaces simultaneously. The daemon reads `~/.roko/workspaces.json`, watches every registered workspace for `workspace.toml` changes, and fans out trigger registration accordingly. A single daemon process serves all workspaces on the machine.

`roko daemon status` shows per-workspace trigger counts and last-fired timestamps. `roko daemon pause --workspace <name>` halts a single workspace's triggers without affecting others.

---

## 9. Workspace Identity & Cross-Workspace Knowledge

Each workspace has a stable ULID assigned at creation, persisted in `workspace.toml` and the registry. Artifacts produced by one workspace can be imported into another (`roko workspace import-artifact`) preserving lineage. The neuro/knowledge store may be configured to share across workspaces, share within tags, or remain isolated.

```toml
[workspace.knowledge]
share_with    = "tag:nunchi"     # share with all workspaces tagged "nunchi"
import_from   = ["roko"]         # selectively read knowledge from named workspaces
```

This is what enables "the more workspaces, the more synergistic" — knowledge accreted in `roko` can inform agents running in `nunchi-dashboard` if the user opts in.

---

## 10. Dashboard Surface (preview)

Detailed in PRD-10. Briefly:
- **Topbar workspace switcher** with recents, search, last-opened time, "+ New Workspace" CTA.
- **Workspace settings page** mirroring the TOML schema as a form.
- **Cross-workspace activity** option in the Pulse dashboard.
- **Workspace templates browser** under the marketplace surface.

---

## 11. Migration of Existing `.roko/` Directories

For users who already have `.roko/` from earlier versions:

- On first run of v-with-this-PRD, `roko workspace adopt` (auto-prompted) walks the cwd looking for `.roko/`. If found, it generates a minimal `workspace.toml` deriving the name from the cwd, registers it, and proceeds as if `roko workspace open <cwd>` had been run.
- The existing `.roko/` contents are preserved untouched. Only `workspace.toml` is added at the root.
- No mass migration of old PRD/plan files is performed; those are read by their respective Workflows on demand under their existing on-disk format.

---

## 12. Acceptance Criteria

| Criterion | Verification |
|---|---|
| `roko workspace new <path>` creates `workspace.toml` and `.roko/`, registers it, sets active. | `cat <path>/workspace.toml`; `roko workspace info` returns it. |
| `roko workspace switch <name>` updates `~/.roko/workspaces.json` `active` field. | `jq .active ~/.roko/workspaces.json`. |
| `roko workspace list --json` returns the full registry as JSON. | Output validates against schema. |
| Inheritance (`extends`) deep-merges; child overrides parent; cycles rejected. | Unit + integration tests in `roko-workspace`. |
| `roko workspace export` and `import` round-trip a workspace including `.roko/`. | Diff of pre-export vs post-import is zero. |
| Daemon hosts triggers from multiple workspaces concurrently. | `roko daemon status` shows >1 workspace; trigger fires verified per-workspace. |
| Capabilities on `workspace.toml` gate Module execution. | Module requiring `chain.write` fails when capability is `false`. |
| Workspace deletion (`--purge`) removes registry entry and `.roko/`; `workspace.toml` is preserved. | `ls <path>` shows `workspace.toml`; registry no longer lists it. |
| Cross-workspace knowledge sharing respects `share_with` / `import_from`. | Knowledge entry created in A appears in B with appropriate filter. |

---

## 13. Open Questions

- Should `workspace.toml` support `secrets.encrypted` inline blobs (age-encrypted, per-workspace key)? Currently leaning toward keychain-only.
- Should the registry sync across machines via a user-controlled remote (S3, Tigris, the user's own server)? Out of scope for v1.
- Should there be a "scratch workspace" concept for one-shot CLI runs that don't want a persistent `workspace.toml`? Probably yes — `--workspace=:scratch` resolves to a `~/.roko/scratch/` ephemeral workspace.
