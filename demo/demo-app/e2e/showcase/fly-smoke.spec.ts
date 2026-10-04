import { expect, test, type Page } from '@playwright/test';

/**
 * fly-smoke (S10 §7 A1-A5; S11.T8; 9338): `npx playwright test -c playwright.fly-smoke.config.ts`,
 * against a real Fly showcase (SHOWCASE_BASE_URL) or the same local showcase-mode `roko serve` as
 * 9332's auth-gate.spec.ts. A1 logs in, A2-A3 render the Overview and open a provenance drawer, A4
 * logs out; A5 (only with SHOWCASE_EXPECT_COLD=1) expects WakeUpBanner on a cold Fly Machine.
 */

const PASSPHRASE = process.env.SHOWCASE_PASSPHRASE ?? '';

/** Submit `passphrase` on the login page and return the status of the login request. */
async function login(page: Page, passphrase: string): Promise<number> {
  await page.goto('/demo/login');
  await page.getByLabel('Passphrase').fill(passphrase);
  const [response] = await Promise.all([
    page.waitForResponse(
      (res) => res.url().endsWith('/api/auth/session') && res.request().method() === 'POST',
    ),
    page.getByRole('button', { name: 'Log in' }).click(),
  ]);
  return response.status();
}

test.beforeAll(() => {
  if (!PASSPHRASE) throw new Error('SHOWCASE_PASSPHRASE is required (never a CLI argument)');
});

test('A1-A4: log in, the Overview renders with a provenance drawer, log out', async (
  { page },
  testInfo,
) => {
  const started = Date.now();
  expect(await login(page, PASSPHRASE)).toBe(204);
  await expect(page).toHaveURL(/\/demo\/$/);
  const overview = page.locator('[data-showcase-page="overview"]');
  await expect(overview).toBeVisible({ timeout: 15_000 });
  testInfo.annotations.push({
    type: 'time-to-first-render-ms',
    description: String(Date.now() - started),
  });

  // A2-A3: the Overview's tiles each open a provenance drawer (golden-views.spec.ts's SC1 check).
  const tiles = overview.locator('[data-tile]');
  await expect(tiles.first()).toBeVisible({ timeout: 15_000 });
  expect(await tiles.count()).toBeGreaterThan(0);
  await tiles.first().locator('.sc-prov-btn').click();
  const drawer = page.locator('.sc-drawer[role="dialog"]');
  await expect(drawer).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(drawer).toHaveCount(0);

  // A4: logging out revokes the session and returns to the login page.
  const cookies = await page.context().cookies();
  const session = cookies.find((cookie) => cookie.name === 'roko_session');
  expect(session?.value).toBeTruthy();
  const origin = new URL(page.url()).origin;
  const out = await page.request.delete('/api/auth/session', {
    headers: { 'X-Roko-CSRF': '1', Origin: origin },
  });
  expect(out.status()).toBe(204);
  await page.goto('/demo/');
  await expect(page).toHaveURL(/\/demo\/login\?next=/, { timeout: 20_000 });
});

test('A5: a cold Fly Machine shows WakeUpBanner while it wakes', async ({ page }) => {
  test.skip(
    process.env.SHOWCASE_EXPECT_COLD !== '1',
    'needs SHOWCASE_EXPECT_COLD=1 against a stopped Machine',
  );
  await page.goto('/demo/login');
  // The banner appears only once the first session probe outlasts WakeUpBanner's own delay
  // (WAKE_DELAY_MS): a real cold start, not a fast local or already-warm one.
  await expect(page.locator('[data-wakeup]')).toBeVisible({ timeout: 30_000 });
  expect(await login(page, PASSPHRASE)).toBe(204);
  await expect(page.locator('[data-showcase-page="overview"]')).toBeVisible({ timeout: 30_000 });
});
