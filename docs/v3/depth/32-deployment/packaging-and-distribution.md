# 32-deployment/01 -- Packaging and Distribution

> Release pipeline from source commit to installable artifact across five
> channels: crates.io, cargo-binstall, Homebrew, GitHub Releases, and Docker.

**Parent:** [32-DEPLOYMENT](../../32-DEPLOYMENT.md)

**Source:** `release-plz.toml`, `cliff.toml`, `dist-workspace.toml`,
`.github/workflows/release-plz.yml`, `.github/workflows/release.yml`

---

## 1. Distribution Philosophy

Roko is a Rust workspace producing multiple binaries from a single
repository. Different users need different install paths, so one pipeline
feeds five channels without manual intervention.

| Channel | roko-cli | roko-serve | roko-mcp-code |
|---------|----------|------------|---------------|
| `cargo install` | Yes | Yes | Yes |
| `cargo binstall` | Yes | Yes | Yes |
| Homebrew | Yes | Yes | No |
| GitHub Releases (prebuilt) | Yes | Yes | Yes |
| Docker (ghcr.io) | Yes | Yes | No |

The MCP server is a local process spawned by editors over stdio; Docker
distribution is unnecessary. All three products get prebuilt binaries.

---

## 2. What to Publish to crates.io

The workspace contains 39 members. Most are internal implementation
details. Publish only products (user-facing binaries) and the libraries
external Rust projects might depend on.

| Crate | crates.io name | Why publish |
|-------|---------------|-------------|
| `roko-cli` | `roko-cli` | `cargo install roko-cli` -- main user binary |
| `roko-serve` | `roko-serve` | `cargo install roko-serve` -- HTTP API server |
| `roko-core` | `roko-core` | Library: Signal + 12 kernel traits, config |
| `roko-std` | `roko-std` | Library: default trait implementations |
| `roko-agent` | `roko-agent` | Library: 12 LLM provider kinds, MCP client |
| `roko-gate` | `roko-gate` | Library: 19 gates, 7-rung pipeline |
| `roko-compose` | `roko-compose` | Library: prompt assembly, 11 role templates |
| `roko-primitives` | `roko-primitives` | Library: HDC vectors, Hamming similarity |

Non-publishable crates get `publish = false` in their `Cargo.toml`.
The workspace root sets `publish = false` as the default so new crates
are non-publishable unless explicitly opted in.

### Versioning Strategy

Independent versioning per crate. The CLI may be at 0.2.0, roko-core at
0.3.0. Tightly-coupled crates can share a version group via release-plz
configuration. Run `cargo-semver-checks` before any publish to catch
accidental breaking changes.

### Publish Order

crates.io requires dependencies before dependents:

1. `roko-primitives` (zero internal deps)
2. `roko-core` (depends on `roko-primitives`)
3. `roko-std`, `roko-agent`, `roko-gate`, `roko-compose` (depend on core)
4. `roko-cli`, `roko-serve` (depend on the above)

Each Cargo.toml uses the dual-source pattern:

```toml
[dependencies]
roko-core = { version = "0.3", path = "../../crates/roko-core" }
```

Cargo uses the path for local builds and the version for crates.io
resolution. `cargo publish --workspace` resolves dependency order
automatically.

---

## 3. Release Pipeline: release-plz + cargo-dist + git-cliff

Three tools chain together:

1. **release-plz** runs on every push to main. It compares local packages
   against the crates.io registry, auto-bumps versions based on
   conventional commits, runs semver-checks, generates changelogs, and
   opens a Release PR. When merged, it publishes and creates git tags.

2. **cargo-dist** (v0.31+) picks up the tags and builds binaries for 6+
   platform targets, generates installers, creates Homebrew formulae,
   calculates SHA256 checksums, produces CycloneDX SBOMs, and publishes
   a GitHub Release.

3. **git-cliff** generates changelogs from conventional commit messages,
   invoked by release-plz.

### Pipeline Flow

```
push to main
  -> release-plz detects changes
  -> opens Release PR (version bumps + changelog)
  -> merge PR
  -> release-plz publishes to crates.io + creates git tags
  -> cargo-dist builds binaries + GitHub Release + Homebrew tap
```

### Monorepo Tag Strategy

Each package gets its own tag and GitHub Release:

| Tag Format | Behavior |
|------------|----------|
| `roko-cli-v0.3.0` | Builds only `roko-cli` binaries |
| `roko-serve-v0.1.0` | Builds only `roko-serve` binaries |
| `roko-core-v0.3.0` | Publishes library to crates.io only |

Multiple tags can be pushed simultaneously to release several packages.

### Configuration: release-plz.toml

```toml
[workspace]
changelog_config = "cliff.toml"
allow_dirty = ["ci"]

[[package]]
name = "roko-cli"
changelog_include = ["roko-core", "roko-std"]
publish = true

[[package]]
name = "roko-serve"
publish = true

[[package]]
name = "roko-core"
publish = true

[[package]]
name = "roko-agent"
version_group = "roko-agent-libs"
publish = true

[[package]]
name = "roko-*"
release = false
publish = false
```

### Configuration: cliff.toml

```toml
[changelog]
body = """
{% for group, commits in commits | group_by(attribute="group") %}
### {{ group | upper_first }}
{% for commit in commits %}
- {{ commit.message | upper_first }} ({{ commit.id | truncate(length=7, end="") }})
{% endfor %}
{% endfor %}
"""

[git]
conventional_commits = true
commit_parsers = [
    { message = "^feat", group = "Features" },
    { message = "^fix", group = "Bug Fixes" },
    { message = "^perf", group = "Performance" },
    { message = "^refactor", group = "Refactor" },
    { message = "^doc", group = "Documentation" },
    { message = "^chore", skip = true },
    { message = "^ci", skip = true },
]
```

---

## 4. CI Workflows

### Release PR Workflow

`.github/workflows/release-plz.yml` runs on push to main, opens Release
PRs. The `RELEASE_PLZ_TOKEN` must be a Personal Access Token, not the
default `GITHUB_TOKEN`, because the default token cannot trigger downstream
workflows (cargo-dist).

### Binary Release Workflow

`.github/workflows/release.yml` is auto-generated by `cargo dist init`.
Triggers on tags, builds for all target platforms, creates GitHub Releases,
updates the Homebrew tap. Regenerate with `cargo dist generate`.

### Docker Image Workflow

`.github/workflows/docker.yml` triggers on tag push. Builds multi-arch
images and pushes to `ghcr.io/nunchi/`.

---

## 5. cargo-dist Configuration

```toml
[dist]
cargo-dist-version = "0.31.0"
ci = "github"
targets = [
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-gnu",
    "aarch64-unknown-linux-musl",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
]
installers = ["shell", "powershell", "homebrew"]
tap = "nunchi/homebrew-roko"
install-path = "~/.cargo/bin"
install-updater = true
sbom = true
github-attestations = true
github-actions-pinning = true
unix-archive = ".tar.gz"
windows-archive = ".zip"
pr-run-mode = "skip"
```

GitHub Actions pinning (v0.29.0+) pins action versions to commit SHAs
instead of mutable tags, preventing compromised upstream actions from
injecting malicious steps.

---

## 6. Self-Update via axoupdater

axoupdater uses install receipts (JSON metadata created by the installer)
to detect when updates are available. Startup prints a notification
without auto-updating:

```rust
if let Ok(updater) = axoupdater::AxoUpdater::new_for("roko-cli") {
    if let Ok(Some(release)) = updater.load_receipt()
        .and_then(|u| u.is_update_needed_sync())
    {
        eprintln!(
            "roko {} available (current: {}). Run: roko update",
            release.version(),
            env!("CARGO_PKG_VERSION")
        );
    }
}
```

An explicit `roko update` subcommand runs the actual update.

---

## 7. Homebrew Tap

cargo-dist auto-generates the Homebrew formula and pushes it to the tap
repository. The formula includes SHA256 checksums, SPDX license
translation, and `brew style --fix` compliance. Shell completions are
installed automatically via `clap_complete` output.

```bash
brew tap nunchi/roko
brew install roko-cli
brew install roko-serve
```

---

## 8. cargo-binstall Support

Each published binary crate includes binstall metadata:

```toml
[package.metadata.binstall]
pkg-url = "{ repo }/releases/download/roko-cli-v{ version }/roko-cli-{ version }-{ target }.tar.gz"
bin-dir = "roko-cli-{ version }-{ target }/{ bin }{ binary-ext }"
pkg-fmt = "tgz"

[package.metadata.binstall.overrides]
"x86_64-pc-windows-msvc" = { pkg-fmt = "zip" }
```

`cargo binstall roko-cli` downloads the prebuilt binary instead of
compiling -- approximately 10 seconds instead of 3+ minutes.

---

## 9. Supply Chain Security

The release pipeline incorporates defense-in-depth: binary signing, SBOM
generation, dependency auditing, and provenance attestations.

| Layer | Tool | What It Checks |
|-------|------|----------------|
| Source dependencies | cargo-deny | Licenses, advisories, bans |
| Audit trail | cargo-vet | Human audit attestations |
| Binary composition | cargo-auditable | Embedded dep manifest |
| Release artifacts | CycloneDX SBOM | Full dependency tree |
| Binary authenticity | Sigstore/cosign | Keyless signing + Rekor |
| Build provenance | SLSA attestation | Source-to-binary traceability |

### Sigstore Binary Signing

All release binaries are signed using Sigstore keyless signing via GitHub
Actions OIDC. No long-lived private keys. Each signing event produces an
ephemeral certificate from Fulcio, recorded in the Rekor transparency log.

Users verify downloaded binaries:

```bash
cosign verify-blob roko-cli-0.3.0-x86_64-unknown-linux-musl.tar.gz \
  --bundle roko-cli-0.3.0-x86_64-unknown-linux-musl.tar.gz.sigstore.json \
  --certificate-identity="https://github.com/nunchi/roko/..." \
  --certificate-oidc-issuer="https://token.actions.githubusercontent.com"
```

### cargo-deny and cargo-vet

Two complementary tools enforce policy in CI:

- **cargo-deny** checks licenses, advisories, bans, and source
  restrictions. Unknown registries and unknown git sources are denied.
- **cargo-vet** verifies every transitive dependency has been audited by a
  trusted source (Mozilla, Google, Bytecode Alliance import registries).

### SLSA Provenance

cargo-dist v0.30.0+ generates GitHub Artifact Attestations automatically,
providing SLSA Level 2 provenance -- a signed statement that a specific
artifact was produced by a specific CI workflow from a specific commit.

---

## 10. Shell Completions

All tools use `clap_complete` for shell completions:

```bash
roko completions bash > ~/.local/share/bash-completion/completions/roko
roko completions zsh > ~/.zfunc/_roko
roko completions fish > ~/.config/fish/completions/roko.fish
```

The Homebrew formula installs completions automatically. The shell
installer adds a note about manual completion setup.

---

## 11. Install UX: What Users Type

```bash
# Rust developers
cargo install roko-cli
roko init --global

# Fast prebuilt binary
cargo binstall roko-cli

# macOS / Linux without Rust
brew tap nunchi/roko
brew install roko-cli

# Docker
docker compose -f docker/docker-compose.yml up

# Cloud
roko deploy railway
roko deploy fly
```

---

## 12. Implementation Status

> **Implementation status:** The release pipeline (release-plz, cargo-dist,
> git-cliff) is designed but not configured. Native builds work. No
> packages have been published to crates.io. The Homebrew tap does not
> exist yet. Docker images are designed but not built. Shell completions
> are wired via `roko completions <shell>`. The `roko deploy` subcommands
> exist for Railway and Fly.io targets. Supply chain security tooling
> (cargo-deny, cargo-vet, Sigstore) is designed.
