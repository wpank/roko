+++
id = "bug-996dc9"
kind = "bug"
title = "cargo-deny fails release PR #89 on RustSec advisories published since #87: tokio, rustls-webpki, webpki-roots, webpki-root-certs, rustix and libsqlite3-sys (via rusqlite 0.35) have fixes; crypto-bigint, pin-utils, chrono and chrono-tz have none"
status = "done"
triage = "verified"
severity = "p1"
goal = "release"
size = "M"
subsystem = ["ci"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
last_verified_rev = "2580237ae"
source = "round v8: the lead's CI drift job for release PR #89 (run 44516751327)"
discovered_from = "agent:team-lead (round v8)"
anchors = ["deny.toml", "Cargo.lock", "crates/roko-index/src/sqlite.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-9dd205"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo deny check"

[closed]
at = 2026-11-19
at_ts = "2026-11-19T12:56:58Z"
commit = "2580237ae"
by = "roko-planner-v8"
via = "manual"
executor = "claude-agent"
size = "M"
forced = false
evidence = "cargo update took tokio 1.53.1 -> 1.54.1, rustls-webpki 0.103.15 -> 0.103.17, webpki-roots and webpki-root-certs 1.0.9 -> 1.0.12 and rustix 1.1.4 -> 1.1.6; rusqlite 0.35 -> 0.41.1 (with fallible_uint, roko-index unchanged) brought libsqlite3-sys 0.39.1 (SQLite 3.54.1); deny.toml reviews RUSTSEC-2026-0387 (crypto-bigint 0.5, no semver fix), -0441 (pin-utils), -0543 (chrono) and -0555 (chrono-tz) with their paths. In a CI-like environment (heavy.sh, scratch HOME, TMPDIR outside the repo, CI=true, CARGO_INCREMENTAL=0, line-tables-only debug info): cargo deny check advisories ok, bans ok, licenses ok, sources ok; nightly fmt --check clean; cargo clippy --workspace --no-deps -D warnings clean; cargo test --workspace --no-fail-fast 283 test binaries, 18165 passed, 0 failed, 83 ignored."
+++

## Problem

The "Supply-chain audit" check (`cargo deny check`, `.github/workflows/deny.yml`) fails on release PR #89
(`release/2026-11-18`, befee2f9b), run 44516751327. RustSec advisories published since #87 fail it:

| Advisory | Crate (in Cargo.lock) | Kind | Fix |
|---|---|---|---|
| RUSTSEC-2026-0381 | webpki-roots 1.0.x | vulnerability: distrusted emSign roots | ≥ 1.0.10 |
| RUSTSEC-2026-0382 | rustls-webpki 0.103.x | vulnerability: distrusted intermediates | ≥ 0.103.16 |
| RUSTSEC-2026-0383 | webpki-root-certs 1.0.x | vulnerability: distrusted emSign roots | ≥ 1.0.10 |
| RUSTSEC-2026-0386 | tokio 1.x | unsound: `AsyncFd::new` I/O safety | ≥ 1.51.6 < 1.52, or ≥ 1.53.3 |
| RUSTSEC-2026-0486 | rustix 1.x | unsound: xattr on a zero-length buffer | ≥ 1.1.6 |
| RUSTSEC-2026-0459 | libsqlite3-sys 0.33.0 | vulnerability: SQLite < 3.54.0 (CVE-2026-119375) | libsqlite3-sys ≥ 0.39, with rusqlite 0.41 |
| RUSTSEC-2026-0387 | crypto-bigint 0.5.5 | vulnerability: timing variability | 0.7.6 only, which elliptic-curve 0.13's `^0.5` precludes |
| RUSTSEC-2026-0441 | pin-utils | unmaintained | none |
| RUSTSEC-2026-0543 | chrono | unmaintained | none |
| RUSTSEC-2026-0555 | chrono-tz | unmaintained | none |

The workspace pins `rusqlite = "0.35"`, whose `libsqlite3-sys = "^0.33"` can't take the fixed SQLite.

## Why it matters

- Will wants GitHub up to date, and #89 can't go green while the audit fails.
- Five of the advisories have a fix within semver, so not taking it ships known TLS trust and I/O-safety defects.
- Goal: release.

## Where

- `Cargo.lock`: the fixed releases.
- `Cargo.toml`: `rusqlite`, used only by `crates/roko-index/src/sqlite.rs`.
- `deny.toml`: `[advisories] ignore`, reviewed entries with a reason each.

## Plan

1. Run `cargo update -p` for tokio, rustls-webpki, webpki-roots, webpki-root-certs and rustix, to their fixed
   releases.
2. Bump `rusqlite` to 0.41 (libsqlite3-sys 0.39, SQLite 3.54.x), adapting `roko-index/src/sqlite.rs` to its API.
3. Add a reviewed ignore for each advisory with no fix, giving its id and the path that pulls the crate in:
   crypto-bigint (only via alloy's elliptic-curve 0.13, in the parked `alloy-backend`), pin-utils (only via alloy's
   wasmtimer), chrono and chrono-tz (unmaintained, no known vulnerability).
4. Run `cargo deny check` to see all four sections pass: advisories, bans, licenses and sources.
5. Check in a CI-like environment: `cargo +nightly fmt --all -- --check`, `cargo clippy --workspace --no-deps -- -D
   warnings`, and `cargo test --workspace --no-fail-fast`, through heavy.sh, with a scratch HOME and TMPDIR outside
   the repo, `CI=true` and `CARGO_INCREMENTAL=0`.

## Done when

- The `[[verify]]` command passes.

## Notes

- The lead asked for this (round v8), for release PR #89. The run's log is in `tmp/hermes-roko-v8/roko-planner-v8/ci89/`.
- Related: bug-9dd205 (an earlier reviewed ignore).
