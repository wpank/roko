import { defineConfig, devices } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

/**
 * The auth gate (S10 §7, S11 §4.3, 9332): a showcase-mode `roko serve` on its own port, serving
 * the built demo app at /demo with a test passphrase and a short lockout (3 failures, 2 s). It
 * has its own config so its web server never starts for other projects.
 *
 * Needs a built `roko` (`target/debug/roko`, or ROKO_BIN) and the demo app built for serve:
 * `VITE_SHOWCASE_SOURCE=api npm run build` (dist/, base /demo/).
 */

const PORT = Number(process.env.ROKO_SHOWCASE_AUTH_PORT ?? '6688');
const ORIGIN = `http://127.0.0.1:${PORT}`;
const ROKO = resolve(process.env.ROKO_BIN ?? '../../target/debug/roko');
// The spec reads it too: workers inherit the runner's environment.
process.env.ROKO_SHOWCASE_TEST_PASSPHRASE ??= 'auth gate test passphrase';

const PREREQUISITES: [string, string][] = [
  [ROKO, 'cargo build -p roko-cli'],
  [resolve('dist/index.html'), 'VITE_SHOWCASE_SOURCE=api npm run build'],
];
for (const [path, fix] of PREREQUISITES) {
  if (!existsSync(path)) throw new Error(`${path} is missing: run \`${fix}\` first`);
}

/** A workspace whose roko.toml turns showcase mode on for this origin, over plain HTTP. */
function showcaseWorkspace(): string {
  const dir = mkdtempSync(join(tmpdir(), 'roko-showcase-auth-'));
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
    'session = { cookie_name = "roko_session", cookie_secure = false }',
    'login = { per_ip_max_failures = 3, per_ip_block_secs = 2, block_backoff_max_secs = 2 }',
    '',
  ];
  writeFileSync(join(dir, 'roko.toml'), config.join('\n'));
  return dir;
}

/** The test passphrase's PHC string, made by the binary under test. */
const passphraseHash = execFileSync(ROKO, ['showcase', 'passphrase', 'hash'], {
  input: process.env.ROKO_SHOWCASE_TEST_PASSPHRASE,
})
  .toString()
  .trim();

export default defineConfig({
  testDir: './e2e/showcase',
  testMatch: 'auth-gate.spec.ts',
  timeout: 60_000,
  // The server keeps its lockout counters between tests, so they run once, in order.
  retries: 0,
  workers: 1,
  use: {
    ...devices['Desktop Chrome'],
    baseURL: ORIGIN,
    headless: true,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  webServer: {
    command: `"${ROKO}" serve --bind 127.0.0.1 --port ${PORT} --workdir "${showcaseWorkspace()}"`,
    url: `${ORIGIN}/ready`,
    reuseExistingServer: false,
    timeout: 60_000,
    env: { ROKO_SHOWCASE_PASSPHRASE_HASH: passphraseHash },
  },
});
