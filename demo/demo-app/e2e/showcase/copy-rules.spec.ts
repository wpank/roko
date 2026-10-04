import { expect, test, type Page } from '@playwright/test';
import { SHOWCASE_ROUTES, expectFixtureServer, openShowcaseRoute } from './fixtures';

/**
 * Copy no showcase route may carry (S10 SC5, thesis §6): a priority claim, and the legacy
 * metaphors.
 */
const BANNED: { phrase: string; pattern: RegExp }[] = [
  { phrase: '"first"', pattern: /\bfirst\b/i },
  { phrase: 'immune', pattern: /\bimmun(e|ity)\b/i },
  { phrase: 'dreams', pattern: /\bdream/i },
  { phrase: 'daimon', pattern: /\bdaimon/i },
  { phrase: 'demurrage', pattern: /\bdemurrage\b/i },
  { phrase: 'cortical', pattern: /\bcortical\b/i },
];

/** Allowed in the body, never as the headline. */
const NOT_THE_HEADLINE = /\bself[- ]improving\b/i;

/** What a visitor reads or hears: the visible text, the document title and the labels. */
async function pageCopy(page: Page): Promise<string> {
  return page.evaluate(() => {
    const labels = Array.from(
      document.querySelectorAll('[title], [aria-label], [alt], [placeholder]'),
      (node) => ['title', 'aria-label', 'alt', 'placeholder']
        .map((name) => node.getAttribute(name) ?? '')
        .join('\n'),
    );
    return [document.title, document.body.innerText, ...labels].join('\n');
  });
}

/** The lines of `copy` that `pattern` matches, so a failure shows the offending copy. */
function matching(copy: string, pattern: RegExp): string[] {
  return copy.split('\n').filter((line) => pattern.test(line)).map((line) => line.trim());
}

test.describe('showcase copy rules (S10 SC5)', () => {
  test.beforeAll(async ({ request }) => {
    await expectFixtureServer(request);
  });

  for (const route of SHOWCASE_ROUTES) {
    test(`${route.path}: no banned phrase, and no "self-improving" headline`, async ({ page }) => {
      await openShowcaseRoute(page, route);
      const copy = await pageCopy(page);
      for (const { phrase, pattern } of BANNED) {
        expect(matching(copy, pattern), `${route.path} says ${phrase}`).toEqual([]);
      }
      for (const headline of await page.locator('h1, .sc-hero').allInnerTexts()) {
        expect(headline, `${route.path} headline`).not.toMatch(NOT_THE_HEADLINE);
      }
    });
  }

  test('the showcase headline says "cybernetic"', async ({ page }) => {
    await openShowcaseRoute(page, SHOWCASE_ROUTES[0]);
    await expect(page.locator('[data-showcase-page="overview"] h1')).toHaveText(/\bcybernetic\b/i);
  });
});
