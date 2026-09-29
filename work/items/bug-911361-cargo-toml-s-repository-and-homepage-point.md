+++
id = "bug-911361"
kind = "bug"
title = "Cargo.toml's repository and homepage point at an unrelated GitHub account"
status = "open"
triage = "verified"
severity = "p2"
size = "S"
goal = "release"
subsystem = ["workspace/cargo"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/cybernetic-harness/assessment-2026-09-28/publication-readiness.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/publication-readiness.md"
anchors = ["Cargo.toml:106", "Cargo.toml:107", "crates/roko-cli/src/deployment.rs::new_for_roko", "crates/roko-agent/src/provider/openrouter_meta.rs::HTTP_REFERER", "crates/roko-cli/src/daemon/systemd.rs", "crates/roko-cli/src/share.rs", "Dockerfile", "docker/roko.Dockerfile", "docs/v3/.vitepress/config.ts"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! git grep -q 'github.com/nunchi/roko' -- . ':(exclude)work/' ':(exclude)docs/v1/' && grep -q '^repository = \"https://github.com/wpank/roko\"' Cargo.toml"
+++

## Problem

`Cargo.toml:106-107` sets `[workspace.package]` `repository` and `homepage` to `https://github.com/nunchi/roko`.
`nunchi` is an unrelated person's GitHub account. roko lives at `github.com/wpank/roko`; `git remote -v` shows
`origin https://github.com/wpank/roko.git`.

The same URL is in 34 more tracked files. Several of those uses are not cosmetic:

- **Release-signature identity.** `SigstoreVerifier::new_for_roko` pins cosign's `--certificate-identity` to
  `https://github.com/nunchi/roko/.github/workflows/release.yml@refs/tags/<tag>`. A real roko release signed by
  `wpank/roko`'s workflow would fail verification. One signed by the unrelated repo's workflow would pass.
- **OpenRouter attribution.** `HTTP_REFERER` in `openrouter_meta.rs` is sent on every OpenRouter model-catalog
  request (`fetch_model_metadata`). The example configs put the same URL into `extra_headers`.
- **Generated or published artefacts** point at the wrong owner: the systemd unit's `Documentation=` line, the
  share-markdown footer, the demo app's share page, the `org.opencontainers.image.source` label on all seven Docker
  images (GHCR uses it to link a package to its repo), and the docs site's GitHub icon, "edit this page" link and
  `CrateLink` source links.

Expected: every current reference names the repository roko actually lives in.

## Why it matters

Goal `release`: roko goes public for the Nous application. Crate metadata, container labels, docs links and the
signature identity must name the right owner before the first tag. GitHub redirects a transferred repo, but it
does not redirect identities such as the cosign certificate identity. So changing the owner after `v0.1.0` means
a second release.

The cosign identity is a latent supply-chain hole: it trusts a workflow in an account Will does not control.
Related: `find-8cc7ac` (release goal; CI is red on `main`).

## Where

There is no single code path; the entry points are the workspace manifest and each file below.

- Workspace metadata: `Cargo.toml:106-107`. Inherited by the members that set `repository.workspace = true`:
  roko-agent-server, roko-runtime, roko-dreams, roko-daimon, roko-primitives, roko-neuro, apps/agent-relay, apps/mirage-rs.
- Signature identity: `crates/roko-cli/src/deployment.rs:26` (`SigstoreVerifier::new_for_roko`), its test
  `roko_verifier_uses_release_workflow_identity` (`:75-81`), and the cosign examples in `docs/v3/32-DEPLOYMENT.md:333`
  and `docs/v3/depth/32-deployment/packaging-and-distribution.md:318`.
- OpenRouter: `crates/roko-agent/src/provider/openrouter_meta.rs:14` (`HTTP_REFERER`) and `:382`;
  `crates/roko-agent/tests/openrouter_integration.rs:87,146,178`; `examples/roko-openrouter.toml:34`;
  `examples/adding-a-provider.md:64`.
- Generated text: `crates/roko-cli/src/daemon/systemd.rs:37`, `crates/roko-cli/src/share.rs:308`,
  `demo/demo-app/src/pages/Share.tsx:251`, `plans/demo-full-stack/tasks.toml:360`.
- Container labels: `Dockerfile:53` and `docker/{demo,gateway,mirage-demo,mirage,roko,worker}.Dockerfile`.
- Docs: `docs/v3/.vitepress/config.ts:829,833`, `docs/v3/.vitepress/components/CrateLink.vue:21`,
  `docs/v3/32-DEPLOYMENT.md:143,585`, `docs/v3/35-ARCHITECTURE.md:1042`, three files under `docs/v3/depth/`, and the
  git-dependency snippets in `crates/roko-primitives/README.md:9,12`.
- Rustdoc: `crates/roko-agent/src/safety/path.rs:39-43` has five link definitions that all point at the repo root.
- Test fixtures that also use the owner name `nunchi`: `crates/roko-cli/src/worker/cloud.rs:636-676` and
  `crates/roko-serve/src/events.rs:971-1168`.

## Current state

- The workspace URL has been unchanged since it arrived in `ec398e273` (2026-04-14). At HEAD on 2026-09-29,
  `git grep -l 'github.com/nunchi/roko'` lists 36 files with 56 hits, counting this item file.
- Only the 8 members above inherit the URL. The other crates set no `repository` at all, so "every crate inherits"
  in the original notes overstates it.
- The cargo-dist metadata is latent. `[workspace.metadata.dist]` configures cargo-dist 0.28.1 with the `shell`
  installer, and cargo-dist takes the release hosting location from `repository`. `release.yml`'s header claims
  cargo-dist, but the workflow builds by hand.
- The Sigstore check is also latent. `SigstoreVerifier` is only re-exported (`crates/roko-cli/src/lib.rs:238`),
  nothing calls it, and `release.yml` does not sign.
- Owner decision D-02 comes from the Nous backlog v2 decision list, which is not in the repo. Its options:
  - (a) `wpank`, where the repo already lives. This was recommended: no migration, and the work is clearly Will's.
  - (b) Transfer to `Nunchi-trade` or `nunchi-ai` before `v0.1.0`.

  A `Nunchi-trade/roko` repo also exists. No `dec-` item in `work/` records Will's answer.

## Plan

**Scope** (the open question in the original notes): cover every tracked `github.com/nunchi/roko` URL except in
two places. `work/` holds item text. `docs/v1/` is deprecated and a deletion candidate, and the companion audit
measured it as it is. Other owner-like strings are a separate item; see Notes.

1. **Owner.** Use `wpank`: it is D-02's recommendation and matches `origin`. If Will has chosen a different owner,
   use that everywhere below and in the verify command.
2. **Replace mechanically.** Replace `github.com/nunchi/roko` with `github.com/wpank/roko` in all the files under
   Where except the fixtures. On macOS:
   `git grep -l 'github.com/nunchi/roko' -- . ':(exclude)work/' ':(exclude)docs/v1/' ':(exclude)crates/roko-cli/src/worker/cloud.rs' ':(exclude)crates/roko-serve/src/events.rs' | xargs sed -i '' 's#github.com/nunchi/roko#github.com/wpank/roko#g'`.
   Update the tests in the same change: `deployment.rs:81`, `openrouter_meta.rs:382` and `openrouter_integration.rs`.
3. **Rustdoc links.** In `safety/path.rs:39-43`, delete the five link definitions, or point them at the tool
   modules. A repo-root URL is not a useful target.
4. **Fixtures, by hand.** A blind sed breaks these tests.
   - `cloud.rs` builds URLs from `owner: "nunchi"` and `github_owner: "nunchi"`, and asserts `owner == "nunchi"`
     after `parse_owner_repo("https://github.com/nunchi/roko.git")`.
   - `events.rs` has webhook payloads with `full_name`, `owner.login`, `html_url` and `config.github.owner`.

   In each test block, change the owner and the URLs together, either to the new owner or to a neutral `example-org`.
5. **Run the affected tests:** `cargo test -p roko-agent openrouter`,
   `cargo test -p roko-agent --test openrouter_integration`, `cargo test -p roko-cli deployment`,
   `cargo test -p roko-cli cloud`, `cargo test -p roko-cli systemd` and `cargo test -p roko-serve events`.

## Done when

- `git grep 'github.com/nunchi/roko'` finds nothing outside `work/` and `docs/v1/`.
- `Cargo.toml` `repository` and `homepage` are `https://github.com/wpank/roko`, or the owner Will chose.
- The tests in Plan step 5 pass.
- Verify: `! git grep -q 'github.com/nunchi/roko' -- . ':(exclude)work/' ':(exclude)docs/v1/' && grep -q '^repository = "https://github.com/wpank/roko"' Cargo.toml`

## Notes

- **Out of scope, not tracked by any item on 2026-09-29.** File these separately; they need Will's owner and
  contact decisions (D-02, D-07):
  - **Webhook defaults (a functional bug).** `roko init --cloud` writes live `[[serve.deploy.webhooks]]` entries
    with `owner = "nunchi"` for the repos `roko` and `collaboration` (`crates/roko-cli/src/commands/init.rs:66-76`).
    The same template is in `crates/roko-core/src/config/schema.rs:1458-1465`. After a deploy, roko would try to
    register webhooks on someone else's repos.
  - **`ghcr.io/nunchi-trade`, a third owner name.** It is `IMAGE_PREFIX` in `.github/workflows/docker-publish.yml:10`
    and the default `worker_image` in `crates/roko-core/src/config/serve.rs:696`.
  - **The `nunchi.dev` domain**, whose owner is unknown: the launchd label `dev.nunchi.roko`
    (`crates/roko-cli/src/daemon/launchd.rs:5`; renaming it orphans installed agents, so it needs a migration), the
    git identity `roko@nunchi.dev` (`crates/roko-cli/src/worker/cloud.rs:436-438`) and the relay example
    `wss://relay.nunchi.dev` (`crates/roko-core/src/config/chain.rs:63-69`).
  - **`authors = ["Roko <engineering@roko.dev>"]`** (`Cargo.toml:105`). D-07 recommends changing it unless
    `roko.dev` is Will's.
- **Leave `docs/v1/` alone.** It has more stale owner strings (`brew tap nunchi/roko`, `ghcr.io/nunchi/...`,
  `--repo nunchi/roko`).
- **Will's calls only.** Do not rename or transfer the GitHub repo, push, tag, or change GitHub settings.
- **Parallel work.** Safe alongside most items. It touches many files lightly, so it may conflict with work on the
  workspace `Cargo.toml`, the Dockerfiles (`gap-2122bd`) or `docs/v3`.

## Original notes

`repository` and `homepage` are `https://github.com/nunchi/roko`. `nunchi` is an unrelated person's account; roko lives at `github.com/wpank/roko` (Nous decision D-02 recommends keeping `wpank`). Every crate inherits these URLs, so published metadata and docs links point at the wrong owner.

Re-checked 2026-09-29: unchanged. Beyond Cargo.toml, 34 more tracked files contain github.com/nunchi/roko: code (roko-agent openrouter_meta.rs and safety/path.rs, roko-cli daemon/systemd.rs, deployment.rs, share.rs and worker/cloud.rs, roko-serve events.rs), Dockerfile and docker/*.Dockerfile labels, docs/v1 and docs/v3 (including .vitepress/config.ts and CrateLink.vue), examples/ and plans/demo-full-stack. Decide whether this item covers them or only the workspace metadata.
