+++
id = "gap-ed511d"
kind = "gap"
title = "docs/v3's auth pages, docker/RAILWAY.md and config set --help still put secrets in roko.toml, which roko now refuses"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["docs/v3", "docker", "roko-cli/main"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report, checked on work/gap-e9660f at 6820f1c2d)"
anchors = ["docs/v3/24-AUTH.md", "docs/v3/26-HTTP-API.md", "docs/v3/depth/24-auth/cli-credentials.md", "docker/RAILWAY.md", "crates/roko-cli/src/main.rs"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = ["gap-e9660f"], blocks = [], related = ["gap-e9660f", "bug-524a3b", "bug-8f8704", "dec-648cce"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qE 'api_key *= *\"' docs/v3/24-AUTH.md docs/v3/26-HTTP-API.md docs/v3/depth/24-auth/cli-credentials.md docker/RAILWAY.md && ! grep -q 'in the chosen layer' crates/roko-cli/src/main.rs"
+++

## Problem

gap-e9660f's branch keeps secrets out of `roko.toml`: agents are refused a config file that holds one. The documentation still tells users to put them there:

- `docs/v3/24-AUTH.md` and `docs/v3/26-HTTP-API.md` show `[serve.auth]` with `api_key = …`;
- `docs/v3/depth/24-auth/cli-credentials.md` (reported by wk-guard2);
- `docker/RAILWAY.md` copies the project `roko.toml` into the image as the runtime config (:30);
- `roko config set --help` says it sets a key "in the chosen layer" (`main.rs:2882`), without saying that secret keys go to `.roko/.env`.

## Why it matters

Release blockers: users who follow the docs end up with a config roko refuses, or with a secret baked into an image.

## Where

The files above.

## Plan

1. Rewrite the examples to use `ROKO__SERVE__AUTH__API_KEY` in `.roko/.env`, or `${VAR}` once bug-8f8704 lands.
2. Say what `config set` does with secret keys (bug-524a3b).

## Done when

- [ ] No doc or help text puts a secret in `roko.toml`.
- [ ] The `[[verify]]` command passes.
