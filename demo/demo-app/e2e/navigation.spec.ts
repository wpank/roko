import { test, expect } from '@playwright/test';
import { collectConsoleErrors, expectNoJsErrors } from './helpers';

/** The showcase nav (S10 §4.2); the legacy pages sit under LAB (D23). */
const SHOWCASE_LINKS = [
  { label: 'OVERVIEW', route: '/' },
  { label: 'HEAD-TO-HEAD', route: '/p1/head-to-head' },
  { label: 'AUDITS', route: '/p2/audits' },
  { label: 'REPLAYS', route: '/replays' },
  { label: 'LAB', route: '/lab' },
] as const;

const LAB_LINKS = [
  { label: 'DEMO', route: '/lab/demo' },
  { label: 'DASH', route: '/lab/dashboard' },
  { label: 'BENCH', route: '/lab/bench' },
  { label: 'EXPLORE', route: '/lab/explorer' },
  { label: 'BUILD', route: '/lab/builder' },
  { label: 'TERM', route: '/lab/terminal' },
  { label: 'CONFIG', route: '/lab/settings' },
] as const;

/** Old top-level paths that now redirect into the lab. */
const OLD_PATHS = ['/demo', '/dashboard/fleet', '/bench', '/explorer', '/builder', '/terminal', '/settings'];

function endsWith(route: string): RegExp {
  return new RegExp(`${route.replace(/[/]/g, '\\/')}$`);
}

test.describe('Top navigation', () => {
  test('the showcase nav has five links and the home is the showcase', async ({ page }) => {
    await page.goto('/', { waitUntil: 'domcontentloaded' });
    await expect(page.locator('nav.topnav')).toBeVisible({ timeout: 5000 });

    const links = page.locator('nav.topnav .links .nav-link');
    await expect(links).toHaveCount(SHOWCASE_LINKS.length);
    for (const { label } of SHOWCASE_LINKS) {
      await expect(links.filter({ hasText: label })).toBeVisible();
    }
    await expect(page.locator('[data-showcase-page="overview"]')).toBeVisible();
    await expect(page.locator('nav.labnav')).toHaveCount(0);
  });

  test('brand link navigates to the showcase home', async ({ page }) => {
    await page.goto('/lab/bench', { waitUntil: 'domcontentloaded' });
    await expect(page.locator('nav.topnav')).toBeVisible({ timeout: 5000 });

    const brand = page.locator('a.brand');
    await expect(brand).toBeVisible();
    await expect(brand).toHaveAttribute('href', '/');

    await brand.click();
    await expect(page).toHaveURL(/\/$/);
  });

  for (const { label, route } of SHOWCASE_LINKS.filter((l) => l.route !== '/')) {
    test(`clicking ${label} navigates to ${route}`, async ({ page }) => {
      const errors = collectConsoleErrors(page);
      await page.goto('/', { waitUntil: 'domcontentloaded' });
      await expect(page.locator('nav.topnav')).toBeVisible({ timeout: 5000 });

      await page.locator('nav.topnav .links .nav-link').filter({ hasText: label }).click();
      await expect(page).toHaveURL(endsWith(route));

      expectNoJsErrors(errors);
    });
  }

  test('the lab row lists the seven legacy pages', async ({ page }) => {
    await page.goto('/lab', { waitUntil: 'domcontentloaded' });
    const links = page.locator('nav.labnav .nav-link');
    await expect(links).toHaveCount(LAB_LINKS.length, { timeout: 5000 });
    for (const { label } of LAB_LINKS) {
      await expect(links.filter({ hasText: label })).toBeVisible();
    }
  });

  for (const { label, route } of LAB_LINKS) {
    test(`clicking lab ${label} navigates to ${route}`, async ({ page }) => {
      const errors = collectConsoleErrors(page);
      await page.goto('/lab', { waitUntil: 'domcontentloaded' });
      await expect(page.locator('nav.labnav')).toBeVisible({ timeout: 5000 });

      await page.locator('nav.labnav .nav-link').filter({ hasText: label }).click();
      await expect(page).toHaveURL(endsWith(route));

      expectNoJsErrors(errors);
    });
  }

  for (const path of OLD_PATHS) {
    test(`old path ${path} redirects to /lab${path}`, async ({ page }) => {
      await page.goto(path, { waitUntil: 'domcontentloaded' });
      await expect(page).toHaveURL(endsWith(`/lab${path}`));
    });
  }

  test('the server status pill shows in the lab only', async ({ page }) => {
    await page.goto('/lab/bench', { waitUntil: 'domcontentloaded' });
    const pill = page.locator('span.status-pill');
    await expect(pill).toBeVisible({ timeout: 5000 });
    expect(await pill.textContent()).toMatch(/LIVE|SYNC|SEED/);

    await page.goto('/', { waitUntil: 'domcontentloaded' });
    await expect(page.locator('nav.topnav')).toBeVisible({ timeout: 5000 });
    await expect(page.locator('span.status-pill')).toHaveCount(0);
  });

  test('active nav links have active styling', async ({ page }) => {
    await page.goto('/lab/bench', { waitUntil: 'domcontentloaded' });
    await expect(page.locator('nav.topnav')).toBeVisible({ timeout: 5000 });

    const lab = page.locator('nav.topnav .links .nav-link').filter({ hasText: 'LAB' });
    await expect(lab).toHaveClass(/active/);
    const bench = page.locator('nav.labnav .nav-link').filter({ hasText: 'BENCH' });
    await expect(bench).toHaveClass(/active/);
    const overview = page.locator('nav.topnav .links .nav-link').filter({ hasText: 'OVERVIEW' });
    await expect(overview).not.toHaveClass(/active/);
  });

  test('nav indicator element exists for sliding animation', async ({ page }) => {
    await page.goto('/lab/bench', { waitUntil: 'domcontentloaded' });
    await expect(page.locator('nav.topnav')).toBeVisible({ timeout: 5000 });

    const indicator = page.locator('nav.topnav .nav-indicator');
    await expect(indicator).toBeAttached();
  });
});
