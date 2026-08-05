# Release Readiness: Go / No-Go Decision

Source inventory: `demo/release-readiness/inventory.json`

---

## Recommendation

**CONDITIONAL GO**

The release pipeline is structurally present—`Cargo.toml` declares cargo-dist 0.28.1 with four cross-compilation targets, `.github/workflows/release.yml` builds and publishes release binaries on tag push, `release-plz.toml` automates version bumping and changelog generation, and `CHANGELOG.md` follows Keep a Changelog with a substantial `[Unreleased]` section awaiting promotion. However, five unresolved unknowns (see **Blocking unknowns**) prevent an unconditional GO. The release may proceed once every item in **Required checks** is verified and each unknown in **Blocking unknowns** is resolved or explicitly accepted by the decision owner. No hard correctness failures were found in the inspected sources; the risk is omission and misalignment, not breakage.

---

## Evidence

All observations below are sourced from `demo/release-readiness/inventory.json`, which inspected four files:

1. **`Cargo.toml`** — `[workspace.package]` declares `edition`, `rust-version`, `license`, `authors`, `repository`, and `homepage` but does **not** include a `version` key. Per-crate versions must be specified individually or are absent entirely. `[workspace.metadata.dist]` configures cargo-dist 0.28.1 with GitHub CI, a shell installer, and four targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`.

2. **`CHANGELOG.md`** — Follows Keep a Changelog 1.1.0 / Semantic Versioning. The only released version is `[0.1.0] - 2026-04-05`. An extensive `[Unreleased]` section lists additions from §33 through §43 but has not been promoted to a versioned entry.

3. **`release-plz.toml`** — Configured with `publish=false`, `allow_dirty=true`, `registry_update=false`, `release_commits` matching `^(feat|fix|perf|refactor|docs|test|chore)`, and a single `[[package]]` entry for `roko-cli` whose `changelog_include` spans `roko-core`, `roko-agent`, `roko-gate`, `roko-compose`, `roko-learn`, `roko-serve`, and `roko-cli`. References `changelog_config = "cliff.toml"` for git-cliff–driven changelog generation.

4. **`.github/workflows/release.yml`** — Triggers on tag pushes matching `v[0-9]+.*` and on `workflow_dispatch`. Manually runs `cargo build --release` for `roko-cli` and `roko-mcp-code` across a four-target matrix, then creates a GitHub Release via `softprops/action-gh-release@v2` using the default `GITHUB_TOKEN`. The workflow does **not** run tests, clippy, or format checks before publishing binaries. The changelog step attempts `git-cliff --latest`, falling back to raw `git log` if the binary is absent.

---

## Blocking unknowns

- [ ] No version key is present in [workspace.package] in Cargo.toml; the workspace version cannot be determined from this file and is recorded as null.
- [ ] The existence and contents of cliff.toml, referenced by both release-plz.toml and the release workflow, have not been verified in the inspected sources.
- [ ] Whether any git tags matching the pattern v[0-9]+.* have been pushed is not determinable from the four inspected files; the release workflow's tag trigger may never have fired.
- [ ] Whether individual crate Cargo.toml files declare their own version fields independently of the workspace is not covered by the workspace-level inspection.
- [ ] Whether a separate CI workflow runs tests, clippy, and format checks and gates on them before a tag push is not evident from these four release-specific sources.

---

## Required checks

- [ ] Verify `[workspace.package].version` (or per-crate version fields) is set and matches the intended release version.
- [ ] Confirm `cliff.toml` exists at the repository root and is syntactically valid for git-cliff before relying on automated changelog generation.
- [ ] Ensure the `CHANGELOG.md` `[Unreleased]` section is promoted to a versioned entry matching the tag before pushing a release tag.
- [ ] Verify that the release workflow's tag pattern `v[0-9]+.*` is consistent with the versioning scheme that release-plz will produce.
- [ ] Confirm all four declared build targets compile successfully with `cargo build --release` before creating a release tag.
- [ ] Verify that a quality-gate CI workflow (`cargo test`, clippy, rustfmt) runs and passes prior to tag push, since the release workflow itself performs no quality checks.

---

## Rollback triggers

- If `cargo build --release` fails on any of the four declared targets after a release tag is pushed, delete the GitHub Release and its tag, then investigate the build failure before re-tagging.
- If the promoted `CHANGELOG.md` entry does not match the tag version (e.g., tag is `v0.2.0` but changelog still shows `[Unreleased]`), roll back the tag and re-promote the changelog entry.
- If `cliff.toml` is missing or malformed and the release workflow falls back to raw `git log` output, treat the generated `RELEASE_NOTES.md` as invalid: delete the release, fix `cliff.toml`, and re-run.
- If the quality-gate CI workflow (tests, clippy, rustfmt) is absent or fails after tag push, block the release by deleting the tag until a passing quality-gate run is confirmed.

---

## Decision owner

**Release Engineering Lead** — the role responsible for release pipeline integrity, version gating, and artifact promotion across all declared build targets.
