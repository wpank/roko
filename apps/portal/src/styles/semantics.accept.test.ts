/**
 * Acceptance: state colours say what they mean in a browser (08e), and nothing
 * else borrows them (08f).
 *
 * Running is a calm hue (never the rose of the Run button or a red), failed a
 * clear red, accepted-with-failures a distinct amber, done green — pairwise
 * distinguishable by hue and by CIE76 ΔE. A coloured role keeps well clear of
 * running (≥ 60° and ΔE ≥ 35: orchid was ΔE 24 from indigo and read as
 * running) and ≥ 30° from the other states and info; rare roles are neutral. A
 * count badge is a neutral at AA contrast, never the running hue; the failed
 * count is red at AA. Bars draw one segment per state. The selection outline
 * is neutral. The shell classes behind 08e are checked here too.
 *
 * Replaces 08e's semantics.accept.test.ts (same path).
 * Copied verbatim from plans/portal-programme/08f-final-polish/accept/.
 */
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const STYLES = join(fileURLToPath(new URL('.', import.meta.url)), '..', 'styles');
const read = (name: string) => readFileSync(join(STYLES, name), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
const globals = read('globals.css');

const vars = new Map<string, string>();
for (const css of [read('rosedust.css'), read('tokens.css')]) {
  for (const m of css.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/g)) vars.set(m[1]!, m[2]!.trim());
}

function resolve(name: string, seen: string[] = []): string {
  const value = vars.get(name);
  if (value === undefined) throw new Error(`token ${name} is not declared`);
  const ref = /^var\((--[a-z0-9-]+)(?:\s*,[^)]*)?\)$/.exec(value);
  if (!ref) return value;
  if (seen.includes(ref[1]!)) throw new Error(`cycle at ${ref[1]}`);
  return resolve(ref[1]!, [...seen, name]);
}

function rgb(name: string): [number, number, number] {
  const m = /^#([0-9a-f]{6})$/i.exec(resolve(name));
  if (!m) throw new Error(`${name} is not a #rrggbb colour: ${resolve(name)}`);
  return [0, 2, 4].map((i) => parseInt(m[1]!.slice(i, i + 2), 16)) as [number, number, number];
}

/** HSL hue in degrees and saturation 0..1. */
function hueSat(name: string): { hue: number; sat: number } {
  const [r, g, b] = rgb(name).map((c) => c / 255) as [number, number, number];
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  const d = max - min;
  if (d === 0) return { hue: 0, sat: 0 };
  const sat = d / (1 - Math.abs(2 * l - 1));
  let hue = max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  hue *= 60;
  return { hue: hue < 0 ? hue + 360 : hue, sat };
}

const hueGap = (a: string, b: string) => {
  const d = Math.abs(hueSat(a).hue - hueSat(b).hue) % 360;
  return Math.min(d, 360 - d);
};

function lab(name: string): [number, number, number] {
  const [r, g, b] = rgb(name).map((c) => {
    const s = c / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  }) as [number, number, number];
  const x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047;
  const y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  const z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883;
  const f = (t: number) => (t > 0.008856 ? Math.cbrt(t) : 7.787 * t + 16 / 116);
  return [116 * f(y) - 16, 500 * (f(x) - f(y)), 200 * (f(y) - f(z))];
}

const deltaE = (a: string, b: string) => {
  const [l1, a1, b1] = lab(a);
  const [l2, a2, b2] = lab(b);
  return Math.hypot(l1 - l2, a1 - a2, b1 - b2);
};

/** WCAG 2 contrast ratio between two colour tokens. */
function contrast(a: string, b: string): number {
  const lum = (t: string) => {
    const [r, g, b] = rgb(t).map((c) => {
      const s = c / 255;
      return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
    }) as [number, number, number];
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  };
  const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
}

const STATES = ['--state-done', '--state-active', '--state-accepted', '--state-failed'];
const RUNNING = '--state-active';
const INFO = '--accent-cyan';
/** Lab chroma: how coloured a token looks (bone and the text greys are below 11). */
const chroma = (t: string) => {
  const [, a, b] = lab(t);
  return Math.hypot(a, b);
};
const coloured = (t: string) => chroma(t) >= 18;
const ROLES = [...vars.keys()].filter((k) => k.startsWith('--role-') && coloured(k));

const inRange = (hue: number, from: number, to: number) => (from <= to ? hue >= from && hue <= to : hue >= from || hue <= to);

describe('state colours', () => {
  it('give each state the hue its meaning needs', () => {
    expect(inRange(hueSat('--state-failed').hue, 345, 15)).toBe(true);
    expect(inRange(hueSat('--state-accepted').hue, 30, 60)).toBe(true);
    expect(inRange(hueSat('--state-done').hue, 90, 150)).toBe(true);
    expect(inRange(hueSat('--state-active').hue, 180, 290)).toBe(true);
  });

  it('keeps every pair of states at least 35° apart and ΔE ≥ 30', () => {
    const close: string[] = [];
    for (let i = 0; i < STATES.length; i++) {
      for (let j = i + 1; j < STATES.length; j++) {
        const [a, b] = [STATES[i]!, STATES[j]!];
        if (hueGap(a, b) < 35 || deltaE(a, b) < 30) {
          close.push(`${a}/${b}: ${hueGap(a, b).toFixed(0)}°, ΔE ${deltaE(a, b).toFixed(1)}`);
        }
      }
    }
    expect(close).toEqual([]);
  });

  it('keeps running away from the Run button’s rose and from info', () => {
    expect(hueGap('--state-active', '--button-primary-bg')).toBeGreaterThanOrEqual(60);
    expect(hueGap('--state-active', INFO)).toBeGreaterThanOrEqual(30);
  });

  it('gives bars their state’s colour, so a running bar is never red', () => {
    for (const t of ['--progress-low', '--progress-mid', '--progress-high']) {
      expect(resolve(t)).toBe(resolve('--state-active'));
    }
  });
});

describe('role and selection colours', () => {
  it('keeps every coloured role well clear of running: at least 60° and ΔE 35', () => {
    const close = ROLES.filter((r) => hueGap(r, RUNNING) < 60 || deltaE(r, RUNNING) < 35).map(
      (r) => `${r}: ${hueGap(r, RUNNING).toFixed(0)}°, ΔE ${deltaE(r, RUNNING).toFixed(1)}`,
    );
    expect(close).toEqual([]);
  });

  it('keeps every coloured role at least 30° from the other states and from info', () => {
    const clashes: string[] = [];
    for (const role of ROLES) {
      for (const other of [...STATES.filter((s) => s !== RUNNING), INFO]) {
        if (hueGap(role, other) < 30) clashes.push(`${role} ~ ${other}: ${hueGap(role, other).toFixed(0)}°`);
      }
    }
    expect(clashes).toEqual([]);
  });

  it('colours the implementer, the role on almost every row, and two more roles', () => {
    expect(coloured('--role-implementer')).toBe(true);
    const distinct = new Set(ROLES.map((r) => resolve(r)));
    expect(distinct.size).toBeGreaterThanOrEqual(3);
  });

  it('draws the selection outline in a neutral, not a state or brand hue', () => {
    expect(hueSat('--focus-border').sat).toBeLessThanOrEqual(0.3);
  });
});

describe('shell classes', () => {
  const blocks = (selector: string) => {
    const re = new RegExp(`(^|[}\\s])${selector.replace(/[.[\]"=]/g, (c) => `\\${c}`)}\\s*\\{([^}]*)\\}`, 'g');
    return [...globals.matchAll(re)].map((m) => m[2] ?? '');
  };
  const block = (selector: string) => blocks(selector)[0] ?? '';

  it('pulses the running glyph gently, and not under reduced motion', () => {
    expect(globals).toMatch(/@keyframes\s+rd-pulse\s*\{/);
    expect(blocks('.rd-pulse').some((b) => /animation:[^;]*rd-pulse/.test(b))).toBe(true);
    expect(globals).toMatch(/@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{\s*\.rd-pulse\s*\{[^}]*animation:\s*none/);
  });

  it('halves the run band to at most 64px, one head line per cell and no scrolling', () => {
    const height = /^(\d+)px$/.exec(resolve('--band-height'));
    expect(Number(height?.[1])).toBeLessThanOrEqual(64);
    expect(block('.rd-band__head')).toMatch(/display:\s*flex/);
    expect(block('.rd-band__head')).toMatch(/white-space:\s*nowrap/);
    expect(blocks('.rd-band__cell').some((b) => /overflow:\s*hidden/.test(b))).toBe(true);
  });

  it('lays the task table on eight fixed tracks with one flexible title column', () => {
    const cols = /grid-template-columns:\s*([^;]+)/.exec(block('.rd-task-row'))?.[1] ?? '';
    expect(block('.rd-task-row')).toMatch(/display:\s*grid/);
    const tracks = cols.match(/minmax\([^)]*\)|[^\s]+/g) ?? [];
    expect(tracks).toHaveLength(8);
    expect(tracks.filter((t) => /fr/.test(t))).toEqual(['minmax(0, 1fr)']);
    expect(tracks.some((t) => /^(auto|min-content|max-content|fit-content)/.test(t))).toBe(false);
  });

  it('puts the task row on the type scale: glyph and title at row size, the rest at meta', () => {
    expect(block('.rd-task-row')).toMatch(/font-size:\s*var\(--type-row\)/);
    expect(block('.rd-task-row__num')).toMatch(/font-size:\s*var\(--type-meta\)/);
    expect(block('.rd-task-row__num')).toMatch(/text-align:\s*right/);
    expect(block('.rd-task-row__meta')).toMatch(/font-size:\s*var\(--type-meta\)/);
  });

  it('sets the stream body at row size with room between lines', () => {
    expect(block('.rd-stream-body')).toMatch(/font-size:\s*var\(--type-row\)/);
    expect(block('.rd-stream-body')).toMatch(/line-height:\s*var\(--leading-relaxed\)/);
    expect(block('.rd-stream-body pre')).toMatch(/font-size:\s*inherit/);
  });

  it('draws a count badge in a neutral at AA contrast, never the running hue', () => {
    const tokenOf = (block: string, prop: string) =>
      new RegExp(`(?:^|[;\\s])${prop}:\\s*var\\((--[a-z0-9-]+)\\)`).exec(block)?.[1] ?? '';
    const bg = tokenOf(block('.rd-badge'), 'background');
    const fg = tokenOf(block('.rd-badge'), 'color');
    expect(bg).not.toBe('');
    expect(fg).not.toBe('');
    expect(coloured(bg)).toBe(false);
    expect(contrast(bg, fg)).toBeGreaterThanOrEqual(4.5);
    const failedBg = tokenOf(block('.rd-badge--failed'), 'background');
    expect(resolve(failedBg)).toBe(resolve('--state-failed'));
    expect(contrast(failedBg, fg)).toBeGreaterThanOrEqual(4.5);
  });

  it('draws each bar segment in its state colour, the bars as rows of segments', () => {
    for (const state of ['done', 'accepted', 'failed', 'active']) {
      expect(block(`.rd-seg[data-segment="${state}"]`)).toMatch(new RegExp(`background:\\s*var\\(--state-${state}\\)`));
    }
    expect(block('.rd-plan-row__bar')).toMatch(/display:\s*flex/);
    expect(block('.rd-progress')).toMatch(/display:\s*flex/);
  });

  it('gives the notice a compact close and an error variant', () => {
    expect(block('.rd-notice__close')).toMatch(/padding:/);
    expect(block('.rd-notice__close')).not.toMatch(/width:\s*100%/);
    expect(block('.rd-notice[data-notice="error"]')).toMatch(/var\(--state-failed\)/);
  });
});
