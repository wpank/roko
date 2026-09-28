/**
 * Acceptance: legibility is computed, not eyeballed.
 *
 * Reads the token values from styles/rosedust.css and styles/tokens.css,
 * resolves var() chains, and checks WCAG 2 contrast: every text colour ≥ 4.5:1
 * on every surface, UI glyphs and focus ≥ 3:1, button text on its fill ≥ 4.5:1.
 * Role accents keep clear of the state hues, the type scale runs plan title >
 * section labels ≥ rows > meta ≥ 12px, and the shell classes use the tokens.
 *
 * Copied verbatim from plans/portal-programme/08d-portal-legibility/accept/.
 */
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const STYLES = join(fileURLToPath(new URL('.', import.meta.url)), '..', 'styles');
const read = (name: string) => readFileSync(join(STYLES, name), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
const rosedust = read('rosedust.css');
const tokensCss = read('tokens.css');
const globals = read('globals.css');

// ── Token resolution ───────────────────────────────────────────────────────────

const vars = new Map<string, string>();
for (const css of [rosedust, tokensCss]) {
  for (const m of css.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/g)) vars.set(m[1]!, m[2]!.trim());
}

function resolve(name: string, seen: string[] = []): string {
  const value = vars.get(name);
  if (value === undefined) throw new Error(`token ${name} is not declared (via ${seen.join(' → ') || 'direct'})`);
  const ref = /^var\((--[a-z0-9-]+)(?:\s*,[^)]*)?\)$/.exec(value);
  if (!ref) return value;
  if (seen.includes(ref[1]!)) throw new Error(`cycle at ${ref[1]}`);
  return resolve(ref[1]!, [...seen, name]);
}

function rgb(name: string): [number, number, number] {
  const hex = resolve(name);
  const m = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(hex);
  if (!m) throw new Error(`${name} resolves to ${hex}, not a hex colour`);
  const h = m[1]!.length === 3 ? [...m[1]!].map((c) => c + c).join('') : m[1]!;
  return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16)) as [number, number, number];
}

function luminance(name: string): number {
  const [r, g, b] = rgb(name).map((c) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  }) as [number, number, number];
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
}

const px = (name: string) => {
  const v = resolve(name);
  const m = /^(\d+(?:\.\d+)?)px$/.exec(v);
  if (!m) throw new Error(`${name} resolves to ${v}, not px`);
  return Number(m[1]);
};

const SURFACES = ['--void', '--bg-raised', '--bg-secondary', '--bg-highlight'];
const ROLES = [...vars.keys()].filter((k) => k.startsWith('--role-'));
const STATES = ['--state-done', '--state-active', '--state-accepted', '--state-failed', '--state-queued', '--state-pending', '--state-skipped'];
const TEXT = ['--text-strong', '--text-muted', '--text-faint', '--text-ghost', '--focus-title', '--accent-error', '--accent-cyan', '--warning', ...STATES, ...ROLES];

function failures(tokens: string[], min: number) {
  const out: string[] = [];
  for (const t of tokens) {
    for (const s of SURFACES) {
      const r = contrast(t, s);
      if (r < min) out.push(`${t} on ${s}: ${r.toFixed(2)}`);
    }
  }
  return out;
}

// ── Tests ──────────────────────────────────────────────────────────────────────

describe('contrast (WCAG 2 AA)', () => {
  it('gives every text colour at least 4.5:1 on every surface', () => {
    expect(ROLES.length).toBeGreaterThanOrEqual(6);
    expect(failures(TEXT, 4.5)).toEqual([]);
  });

  it('gives the focus ring and progress colours at least 3:1', () => {
    expect(failures(['--focus-border', '--progress-low', '--progress-mid', '--progress-high'], 3)).toEqual([]);
  });

  it('gives button text at least 4.5:1 on its fill', () => {
    expect(contrast('--button-primary-fg', '--button-primary-bg')).toBeGreaterThanOrEqual(4.5);
    expect(contrast('--button-primary-fg', '--button-primary-hover')).toBeGreaterThanOrEqual(4.5);
    expect(contrast('--danger-fill-fg', '--danger-fill')).toBeGreaterThanOrEqual(4.5);
  });

  it('keeps borders on their own line token, not a text colour', () => {
    expect(vars.get('--border-default')).toBe('var(--line)');
    expect(resolve('--line')).toMatch(/^#/);
  });
});

describe('colour semantics', () => {
  it('never gives a role the colour of a state', () => {
    const stateHues = new Set(
      ['--state-done', '--state-active', '--state-accepted', '--state-failed', '--rose', '--rose-bright', '--ember', '--warning', '--sage'].map(
        (t) => resolve(t).toLowerCase(),
      ),
    );
    const clashes = ROLES.filter((r) => r !== '--role-other' && stateHues.has(resolve(r).toLowerCase()));
    expect(clashes).toEqual([]);
  });

  it('keeps the short role names as aliases', () => {
    expect(vars.get('--role-impl')).toBe('var(--role-implementer)');
    expect(vars.get('--role-reviewer')).toBe('var(--role-quick-reviewer)');
  });
});

describe('type scale', () => {
  it('runs plan title > section labels ≥ rows > meta, with nothing under 12px', () => {
    const [title, section, row, meta] = ['--type-title', '--type-section', '--type-row', '--type-meta'].map(px) as [number, number, number, number];
    expect(title).toBeGreaterThan(section);
    expect(section).toBeGreaterThanOrEqual(row);
    expect(row).toBeGreaterThan(meta);
    expect(meta).toBeGreaterThanOrEqual(12);
    expect(px('--text-xs')).toBeGreaterThanOrEqual(12);
  });
});

describe('shell classes', () => {
  const block = (selector: string) => {
    const re = new RegExp(`${selector.replace(/[.[\]"=]/g, (c) => `\\${c}`)}\\s*\\{([^}]*)\\}`);
    return re.exec(globals)?.[1] ?? '';
  };

  it('aligns the run band to the rail and fixes its height', () => {
    expect(block('.rd-band')).toMatch(/grid-template-columns:\s*var\(--rail-width\)/);
    expect(block('.rd-band')).toMatch(/(^|[;\s])height:\s*var\(--band-height\)/);
    expect(vars.get('--band-height')).toMatch(/^\d+px$/);
  });

  it('gives the header cancel the same danger fill as the Cancel button', () => {
    expect(block('.rd-header__cancel')).toMatch(/background:\s*var\(--danger-fill\)/);
    expect(block('.rd-header__cancel')).toMatch(/color:\s*var\(--danger-fill-fg\)/);
    expect(globals).toMatch(/--color-danger-fill:\s*var\(--danger-fill\)/);
    expect(globals).toMatch(/--color-button-primary-fg:\s*var\(--button-primary-fg\)/);
  });

  it('styles the one "not supported" notice and the info alert alike', () => {
    expect(block('.rd-notice')).toMatch(/border-left:[^;]*var\(--accent-cyan\)/);
    expect(block('.rd-alert[data-severity="info"]')).toMatch(/var\(--accent-cyan\)/);
  });

  it('maps the type classes to the type scale', () => {
    expect(block('.rd-title')).toMatch(/font-size:\s*var\(--type-title\)/);
    expect(block('.rd-section')).toMatch(/font-size:\s*var\(--type-section\)/);
    expect(block('.rd-row')).toMatch(/font-size:\s*var\(--type-row\)/);
    expect(block('.rd-meta')).toMatch(/font-size:\s*var\(--type-meta\)/);
  });

  it('draws rail bars at least 4px tall', () => {
    const height = /height:\s*(\d+)px/.exec(block('.rd-plan-row__bar'));
    expect(Number(height?.[1])).toBeGreaterThanOrEqual(4);
  });
});
