import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 60_000,
  retries: 1,
  forbidOnly: !!process.env.CI,
  use: {
    baseURL: 'http://localhost:5173',
    headless: true,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  projects: [
    {
      // The legacy specs. Showcase specs run in their own projects.
      name: 'chromium',
      testIgnore: ['**/showcase/**'],
      use: { ...devices['Desktop Chrome'] },
    },
    {
      // S10 §7: the dev server with VITE_ALLOW_FIXTURES=1, serving the fixture bundles of
      // e2e/showcase/bundles at /bundles/. Projects that need another web server (vite
      // preview, roko serve, a remote URL) get their own config file.
      name: 'showcase-fixture',
      testDir: './e2e/showcase',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: {
    command: 'npm run dev',
    port: 5173,
    reuseExistingServer: true,
    timeout: 30_000,
    // The legacy pages ignore the flag; the showcase-fixture project needs it.
    env: { VITE_ALLOW_FIXTURES: '1' },
  },
});
