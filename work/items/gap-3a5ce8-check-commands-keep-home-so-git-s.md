+++
id = "gap-3a5ce8"
kind = "gap"
title = "Check commands keep HOME, so git's credential helper (osxkeychain) and ~/.ssh keys still let a check push to GitHub as the person"
status = "open"
triage = "unverified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-core/child_env"]
created = 2026-10-16
updated = 2026-10-16
source = "tmp/hermes-roko-v5/findings/roko-credfix.md (roko-credfix, 2026-10-16)"
discovered_from = "bug-3c4600: gh's login is hidden from checks, git's is not"
anchors = ["crates/roko-core/src/child_env.rs::apply_check_env"]
links = { depends_on = [], blocks = [], related = ["bug-3c4600"], supersedes = [], duplicate_of = "" }
+++

## Problem

bug-3c4600 keeps the person's `gh` login away from check commands (an empty `GH_CONFIG_DIR`, no GitHub tokens). Check
commands still keep `HOME` and `USER`, so git's own credentials remain reachable:

- **HTTPS**: git's credential helper. On macOS that is Apple git's default `credential.helper osxkeychain`, set in
  `/Library/Developer/CommandLineTools/usr/share/git-core/gitconfig`, and it is what Will's machine uses. It hands a
  stored `github.com` login to `git push https://github.com/...` without a prompt.
- **SSH**: unencrypted keys in `~/.ssh`, or passphrases macOS keeps in the keychain (`UseKeychain`), let
  `git push git@github.com:...` authenticate. `SSH_AUTH_SOCK` is not on the gate allowlist, but the key files are
  readable. Will's `~/.ssh` holds 16 `id_*` files.

So a check, or an agent-written test that a check runs, can still push to GitHub (or another git host) as the person.

## Why it matters

It is the same class as bug-3c4600: a check roko runs on its own can act on GitHub as the person without asking.

## Plan (options; Will decides, because each option can break checks that fetch private git dependencies)

1. Disable git credential helpers in check commands. Set `GIT_CONFIG_COUNT=1`, `GIT_CONFIG_KEY_0=credential.helper`
   and `GIT_CONFIG_VALUE_0=` (an empty helper resets the list), plus `GIT_TERMINAL_PROMPT=0`. This breaks private HTTPS
   git dependencies that a check's cargo or npm fetches. A `[gates]` switch would turn it off.
2. For SSH, set `GIT_SSH_COMMAND="ssh -o BatchMode=yes -o IdentitiesOnly=yes -o IdentityFile=none"`, so that git over
   SSH in a check uses no key. This breaks private SSH git dependencies, and needs the same switch.
3. Make checks fetch nothing: `CARGO_NET_OFFLINE=true` when the lockfile and vendor or registry caches allow it. This
   is narrower and does not cover a test that runs `git push` itself.

## Done when

A check whose command is `git ls-remote https://github.com/<private repo>` (and its SSH form) cannot authenticate on a
machine where the person's git can, unless the chosen opt-out is set.

## Notes

Filed by roko-credfix while fixing bug-3c4600 (hermes-roko-v5 round, 2026-10-16). See
`tmp/hermes-roko-v5/findings/roko-credfix.md`.
