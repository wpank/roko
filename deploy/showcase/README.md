# Showcase preflight

`preflight.sh` runs S11 §7's hard gates (P1-P14) against a showcase deployment. "Never deploy
before preflight passes": it is the F1 exit check before the passphrase is shared, and the
regression check after every deploy or secret rotation.

```
deploy/showcase/preflight.sh <base-url> [--live-checks]
deploy/showcase/preflight.sh --local
```

- `<base-url>`: the showcase's origin, e.g. `https://roko-showcase.fly.dev`. Over HTTPS, P7 also
  checks the session cookie's `Secure` flag and `__Host-` prefix.
- `--live-checks`: also run P12 (`Fly-Client-IP` is honoured) and P14 (the budget endpoint),
  which only make sense against a real deployment. P12 sends the probe; reading the resulting
  auth-audit entry back is the operator's job, since the script has no access to the deployed
  Machine's filesystem.
- `--local`: build nothing. Starts the locally built `roko` (`ROKO_BIN`, else
  `target/debug/roko`) as a showcase-mode `roko serve` on `127.0.0.1` over plain HTTP, with a
  freshly generated passphrase and admin key, runs every check but the live-only ones, then stops
  it and removes its temporary workspace. Needs a build: `cargo build -p roko-cli`.

## Secrets

`SHOWCASE_PASSPHRASE` and `SHOWCASE_ADMIN_KEY` come from the environment only, never a CLI
argument, and the script never logs them. `ROKO_TEST_PRIVY_JWT` is optional (P3): a real Privy
JWT from `roko login`, when the operator has one; without it, P3 uses a well-formed-but-fake
Bearer JWT instead, since showcase mode must refuse every one regardless (bug-7eef96, G0a).

## Output

One line per check, `PASS`, `FAIL` or `SKIP`, then a summary; the exit code is the number of
`FAIL`s (0 when every check passed). `SKIP` means either a live-only check run without
`--live-checks`, a cookie attribute that needs HTTPS run under `--local`'s plain HTTP, or a route
S11 describes that is not wired yet (`/api/showcase/stream`, P8; `/api/showcase/admin/freeze`,
P9): both answer 404 today, and the check starts holding them to their real behaviour the day
they answer anything else.

## After preflight: fly-smoke

Once preflight passes, `demo/demo-app/playwright.fly-smoke.config.ts` drives a real browser
through login, the Overview and its provenance drawers, and logout (A1-A4); with
`SHOWCASE_EXPECT_COLD=1` it also expects `WakeUpBanner` while a stopped Fly Machine wakes (A5):

```
cd demo/demo-app
SHOWCASE_BASE_URL=https://roko-showcase.fly.dev SHOWCASE_PASSPHRASE=... \
  npx playwright test -c playwright.fly-smoke.config.ts
```

Without `SHOWCASE_BASE_URL` it starts the same kind of local showcase-mode `roko serve` this
script does, for a quick local run.
