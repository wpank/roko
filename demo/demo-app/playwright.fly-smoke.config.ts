import { defineConfig, devices, type PlaywrightTestConfig } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

/**
 * fly-smoke (S10 §7 A1-A5; S11.T8; 9338): the F1 exit check after a deploy, run once preflight
 * (9337) passes. Against a real Fly showcase when SHOWCASE_BASE_URL is set (no web server; the
 * passphrase comes from SHOWCASE_PASSPHRASE in the environment only, never a default here), else
 * the same local showcase-mode `roko serve` as 9332's playwright.showcase-auth.config.ts, with a
 * staged replay bundle (9333's build) so the Overview has tiles to render.
 *
 * Needs a built `roko` (`target/debug/roko`, or ROKO_BIN) and the demo app built for serve
 * (`VITE_SHOWCASE_SOURCE=api npm run build`) only for the local run; a live run against
 * SHOWCASE_BASE_URL needs neither.
 */

const LIVE_URL = process.env.SHOWCASE_BASE_URL;
const PORT = Number(process.env.ROKO_SHOWCASE_FLY_SMOKE_PORT ?? '6690');
const ORIGIN = LIVE_URL ?? `http://127.0.0.1:${PORT}`;
const REPO = resolve('../..');
const ROKO = resolve(process.env.ROKO_BIN ?? join(REPO, 'target/debug/roko'));

/** Build one fixture replay bundle (the same call as 9333's config) so the Overview has tiles. */
function stageBundle(root: string): string {
  const bundleId = 'b-fixture-p1';
  const showcase = join(REPO, 'benchmarks/viabilitybench/showcase');
  execFileSync('python3', [
    join(showcase, 'build_bundle.py'),
    '--experiment', 'FIXTURE-P1',
    '--results', join(showcase, 'fixtures/results'),
    '--out', join(root, bundleId),
    '--bundle-id', bundleId,
    '--title', 'Fixture P1 replay',
    '--created-at', '2026-10-04T00:00:00Z',
    '--featured',
  ]);
  return bundleId;
}

/**
 * A workspace whose roko.toml turns showcase mode on for `ORIGIN` over plain HTTP, with a staged
 * bundle and 9332's test lockout (3 failures, 2 s, so a wrong-passphrase check never locks out
 * the real one for long).
 */
function flySmokeWorkspace(): string {
  const dir = mkdtempSync(join(tmpdir(), 'roko-fly-smoke-'));
  const bundles = join(dir, 'bundles');
  stageBundle(bundles);
  const config = [
    '[serve]',
    'public_routes = ["health", "ready"]',
    '',
    '[serve.auth]',
    'enabled = true',
    'enforcement_mode = "enforce"',
    '',
    '[showcase]',
    'enabled = true',
    `public_origin = "${ORIGIN}"`,
    `bundle_root = ${JSON.stringify(bundles)}`,
    'session = { cookie_name = "roko_session", cookie_secure = false }',
    'login = { per_ip_max_failures = 3, per_ip_block_secs = 2, block_backoff_max_secs = 2 }',
    '',
  ];
  writeFileSync(join(dir, 'roko.toml'), config.join('\n'));
  return dir;
}

/** `undefined` against a live showcase (no server to start); a local one otherwise. */
function webServerConfig(): PlaywrightTestConfig['webServer'] {
  if (LIVE_URL) return undefined;
  const PREREQUISITES: [string, string][] = [
    [ROKO, 'cargo build -p roko-cli'],
    [resolve('dist/index.html'), 'VITE_SHOWCASE_SOURCE=api npm run build'],
  ];
  for (const [path, fix] of PREREQUISITES) {
    if (!existsSync(path)) throw new Error(`${path} is missing: run \`${fix}\` first`);
  }
  // The spec reads it too: workers inherit the runner's environment.
  process.env.SHOWCASE_PASSPHRASE ??= 'fly smoke test passphrase';
  const passphraseHash = execFileSync(ROKO, ['showcase', 'passphrase', 'hash'], {
    input: process.env.SHOWCASE_PASSPHRASE,
  })
    .toString()
    .trim();
  return {
    command: `"${ROKO}" serve --bind 127.0.0.1 --port ${PORT} --workdir "${flySmokeWorkspace()}"`,
    url: `${ORIGIN}/ready`,
    reuseExistingServer: false,
    timeout: 60_000,
    env: { ROKO_SHOWCASE_PASSPHRASE_HASH: passphraseHash },
  };
}

export default defineConfig({
  testDir: './e2e/showcase',
  testMatch: 'fly-smoke.spec.ts',
  timeout: 60_000,
  retries: 0,
  workers: 1,
  use: {
    ...devices['Desktop Chrome'],
    baseURL: ORIGIN,
    headless: true,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  webServer: webServerConfig(),
});
