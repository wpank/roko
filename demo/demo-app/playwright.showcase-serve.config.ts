import { defineConfig, devices } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { cpSync, existsSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, join, resolve } from 'node:path';

/**
 * R1-serve (S10 §7, 9333): the showcase served by `roko serve` at /demo, with the `api` source
 * and the server's checksum loader, on its own port.
 *
 * The bundle root holds a replay bundle and a tampered copy of it. The replay bundle is the
 * pilot's when ROKO_SHOWCASE_PILOT_BUNDLE names its directory, else one `build_bundle.py` makes
 * from ViabilityBench's fixture records. This is not showcase mode: auth is off on loopback, as
 * for a local operator; showcase-mode auth is playwright.showcase-auth.config.ts's.
 *
 * Needs a built `roko` (`target/debug/roko`, or ROKO_BIN), `python3`, and the demo app built for
 * serve: `VITE_SHOWCASE_SOURCE=api npm run build` (dist/, base /demo/).
 */

const PORT = Number(process.env.ROKO_SHOWCASE_SERVE_PORT ?? '6689');
const ORIGIN = `http://127.0.0.1:${PORT}`;
const REPO = resolve('../..');
const ROKO = resolve(process.env.ROKO_BIN ?? join(REPO, 'target/debug/roko'));
const PILOT = process.env.ROKO_SHOWCASE_PILOT_BUNDLE;
// The spec reads which bundle is the good one: workers inherit the runner's environment.
process.env.ROKO_SHOWCASE_SERVE_BUNDLE = PILOT ? basename(PILOT) : 'b-fixture-p1';

const PREREQUISITES: [string, string][] = [
  [ROKO, 'cargo build -p roko-cli'],
  [resolve('dist/index.html'), 'VITE_SHOWCASE_SOURCE=api npm run build'],
];
for (const [path, fix] of PREREQUISITES) {
  if (!existsSync(path)) throw new Error(`${path} is missing: run \`${fix}\` first`);
}

/** Stage the good bundle and `b-tampered`, a copy whose metrics changed after sealing. */
function stageBundles(root: string): void {
  const good = process.env.ROKO_SHOWCASE_SERVE_BUNDLE ?? 'b-fixture-p1';
  if (PILOT) {
    cpSync(PILOT, join(root, good), { recursive: true });
  } else {
    const showcase = join(REPO, 'benchmarks/viabilitybench/showcase');
    execFileSync('python3', [
      join(showcase, 'build_bundle.py'),
      '--experiment', 'FIXTURE-P1',
      '--results', join(showcase, 'fixtures/results'),
      '--out', join(root, good),
      '--bundle-id', good,
      '--title', 'Fixture P1 replay',
      '--created-at', '2026-10-04T00:00:00Z',
      '--featured',
    ]);
  }
  cpSync(join(root, good), join(root, 'b-tampered'), { recursive: true });
  const metrics = join(root, 'b-tampered', 'data', 'metrics.jsonl');
  writeFileSync(metrics, `${readFileSync(metrics, 'utf8')}\n`);
}

/** A workspace whose roko.toml points the showcase at the staged bundles. */
function serveWorkspace(): string {
  const dir = mkdtempSync(join(tmpdir(), 'roko-showcase-serve-'));
  const bundles = join(dir, 'bundles');
  stageBundles(bundles);
  const config = [
    '[serve.auth]',
    'enabled = false',
    '',
    '[showcase]',
    `bundle_root = ${JSON.stringify(bundles)}`,
    '',
  ];
  writeFileSync(join(dir, 'roko.toml'), config.join('\n'));
  return dir;
}

export default defineConfig({
  testDir: './e2e/showcase',
  testMatch: 'serve-manifest.spec.ts',
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
  webServer: {
    command: `"${ROKO}" serve --bind 127.0.0.1 --port ${PORT} --workdir "${serveWorkspace()}"`,
    url: `${ORIGIN}/ready`,
    reuseExistingServer: false,
    timeout: 60_000,
  },
});
