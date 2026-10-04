import { expect, test, type Page } from '@playwright/test';

/**
 * The auth gate against a showcase-mode `roko serve` (S10 §7, S11 §4.3, 9332):
 * `npx playwright test -c playwright.showcase-auth.config.ts`. The server's lockout is 3
 * failures and 2 s, and it keeps its counters, so the tests run in order.
 */

test.describe.configure({ mode: 'serial' });

const PASSPHRASE = process.env.ROKO_SHOWCASE_TEST_PASSPHRASE ?? '';
const WRONG = 'not the passphrase at all';

/** Submit `passphrase` on the login page and return the status of the login request. */
async function submit(page: Page, passphrase: string): Promise<number> {
  await page.getByLabel('Passphrase').fill(passphrase);
  const [response] = await Promise.all([
    page.waitForResponse(
      (res) => res.url().endsWith('/api/auth/session') && res.request().method() === 'POST',
    ),
    page.getByRole('button', { name: 'Log in' }).click(),
  ]);
  return response.status();
}

test('a visitor without a session is sent to the login page', async ({ page }) => {
  await page.goto('/demo/');
  await expect(page).toHaveURL(/\/demo\/login\?next=/, { timeout: 20_000 });
  await expect(page.locator('[data-showcase-page="login"]')).toBeVisible();
});

test('a wrong passphrase shows invalid_passphrase', async ({ page }) => {
  await page.goto('/demo/login');
  expect(await submit(page, WRONG)).toBe(401);
  await expect(page.locator('[data-login-error="invalid_passphrase"]')).toBeVisible();
});

test('repeated failures lock the login, and the page says when to retry', async ({ page }) => {
  await page.goto('/demo/login');
  // One failure came from the test before: two more reach the limit of three.
  expect(await submit(page, WRONG)).toBe(401);
  expect(await submit(page, WRONG)).toBe(401);
  expect(await submit(page, WRONG)).toBe(429);
  const locked = page.locator('[data-login-error="login_locked"]');
  await expect(locked).toBeVisible();
  await expect(locked).toContainText('Try again in');
});

test('the right passphrase opens the showcase', async ({ page }) => {
  // The 2 s block lifts first.
  await page.waitForTimeout(2_500);
  await page.goto('/demo/login?next=%2Fdemo%2F');
  expect(await submit(page, PASSPHRASE)).toBe(204);
  await expect(page).toHaveURL(/\/demo\/$/);
  await expect(page.locator('[data-showcase-page="overview"]')).toBeVisible({ timeout: 15_000 });
});

test('logging out revokes the old cookie', async ({ page }) => {
  await page.goto('/demo/login');
  expect(await submit(page, PASSPHRASE)).toBe(204);
  const cookies = await page.context().cookies();
  const session = cookies.find((cookie) => cookie.name === 'roko_session');
  expect(session?.value).toBeTruthy();
  const origin = new URL(page.url()).origin;

  const out = await page.request.delete('/api/auth/session', {
    headers: { 'X-Roko-CSRF': '1', Origin: origin },
  });
  expect(out.status()).toBe(204);
  const reused = await page.request.get('/api/showcase/manifest', {
    headers: { Cookie: `roko_session=${session?.value}` },
  });
  expect(reused.status()).toBe(401);
});
