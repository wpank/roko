/**
 * Acceptance for gap-195374: the portal is drawn in roko's design system.
 *
 * - Its colours are roko's tokens (roko-tokens.css): every word meets 4.5:1 on every surface it sits on, by day
 *   (aluminium), by night (graphite) and on the black glass; the orange key's words meet 4.5:1 on orange.
 * - The older names the plan view uses (tokens.css) only point at roko's tokens; they define no colour.
 * - Element resets live in `@layer base`, so every class wins over them, and the kit and the portal's frame are in
 *   `@layer components`; every Tailwind colour a component names is declared.
 * - The two faces are self-hosted, with their licences beside them.
 */
import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const STYLES = fileURLToPath(new URL('.', import.meta.url));
const SRC = join(STYLES, '..');
const PUBLIC = join(SRC, '..', 'public');
const read = (name: string) => readFileSync(join(STYLES, name), 'utf8');
const globals = read('globals.css');
const strip = (css: string) => css.replace(/\/\*[\s\S]*?\*\//g, '');

// ── A small CSS walker ─────────────────────────────────────────────────────────

interface Rule {
  selector: string;
  /** Enclosing at-rule preludes, outermost first, e.g. ['@layer base', '@media (…)']. */
  context: string[];
  body: string;
}

function parse(css: string): Rule[] {
  const text = css.replace(/\/\*[\s\S]*?\*\//g, '');
  const rules: Rule[] = [];
  const stack: string[] = [];
  let prelude = '';
  let i = 0;
  while (i < text.length) {
    const ch = text[i]!;
    if (ch === '{') {
      const head = prelude.trim();
      prelude = '';
      if (head.startsWith('@')) {
        stack.push(head.replace(/\s+/g, ' '));
        i += 1;
        continue;
      }
      // A style rule: read its declaration block (no nesting inside).
      const end = text.indexOf('}', i);
      rules.push({ selector: head.replace(/\s+/g, ' '), context: [...stack], body: text.slice(i + 1, end) });
      i = end + 1;
      continue;
    }
    if (ch === '}') {
      stack.pop();
      prelude = '';
      i += 1;
      continue;
    }
    if (ch === ';' && prelude.trim().startsWith('@')) {
      prelude = '';
      i += 1;
      continue;
    }
    prelude += ch;
    i += 1;
  }
  return rules;
}

const rules = parse(globals);
const inLayer = (rule: Rule, layer: string) => rule.context.includes(`@layer ${layer}`);
const themeBlock = /@theme\s*\{([\s\S]*?)\n\}/.exec(globals.replace(/\/\*[\s\S]*?\*\//g, ''))?.[1] ?? '';
const themeColours = new Set([...themeBlock.matchAll(/--color-([a-z0-9-]+)\s*:/g)].map((m) => m[1]!));

/** A selector that styles bare elements (no class or id), e.g. `button`, `*`, `::selection`. */
function isElementSelector(selector: string): boolean {
  return selector.split(',').some((s) => !/[.#]/.test(s.trim()) && !s.trim().startsWith(':root'));
}

function componentRule(selector: string): Rule | undefined {
  return rules.find((r) => inLayer(r, 'components') && r.selector.split(',').map((s) => s.trim()).includes(selector));
}

function declares(rule: Rule | undefined, property: string, value: RegExp): boolean {
  if (!rule) return false;
  return rule.body
    .split(';')
    .map((d) => d.split(':'))
    .some(([p, ...v]) => p?.trim() === property && value.test(v.join(':').trim()));
}

// ── Tailwind colour utilities used by components ────────────────────────────────

const NOT_COLOUR: Record<string, RegExp> = {
  text: /^(xs|sm|base|md|lg|\d?xl|left|center|right|justify|start|end|wrap|nowrap|balance|pretty|ellipsis|clip|\[.*\])$/,
  bg: /^(fixed|local|scroll|clip-.*|origin-.*|repeat.*|no-repeat|cover|contain|auto|center|top|bottom|left|right|none|gradient-.*|linear-.*|radial.*|conic.*|blend-.*|\[.*\])$/,
  border: /^(\d+|x|y|t|r|b|l|s|e|x-\d+|y-\d+|[trblse]-\d+|solid|dashed|dotted|double|hidden|none|collapse|separate|spacing.*|box|\[.*\])$/,
  outline: /^(\d+|none|hidden|solid|dashed|dotted|double|offset-.*|\[.*\])$/,
  ring: /^(\d+|inset|offset-.*|\[.*\])$/,
};

function colourUtilities(source: string): string[] {
  const found: string[] = [];
  for (const literal of source.matchAll(/'([^'\n]*)'|"([^"\n]*)"|`([^`]*)`/g)) {
    const value = literal[1] ?? literal[2] ?? literal[3] ?? '';
    for (const raw of value.split(/\s+/)) {
      const token = raw.replace(/^.*:/, '').replace(/^!/, '').replace(/\/\d+$/, '');
      const m = /^(text|bg|border|border-[trblxyse]|outline|ring)-([a-z][a-z0-9-]*)$/.exec(token);
      if (!m) continue;
      const kind = m[1]!.startsWith('border') ? 'border' : m[1]!;
      const name = m[2]!;
      if (NOT_COLOUR[kind]!.test(name)) continue;
      found.push(`${m[1]}-${name}`);
      if (!['transparent', 'current', 'inherit', 'white', 'black'].includes(name) && !themeColours.has(name)) {
        found.push(`!undeclared:${m[1]}-${name}`);
      }
    }
  }
  return found;
}

function sourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return sourceFiles(path);
    return /\.tsx$/.test(name) && !/\.test\.tsx$/.test(name) ? [path] : [];
  });
}

// ── roko's tokens, day and night ────────────────────────────────────────────────

function block(css: string, selector: string): Map<string, string> {
  const at = css.indexOf(`${selector} {`);
  if (at < 0) throw new Error(`no ${selector} block`);
  const body = css.slice(at, css.indexOf('}', at));
  return new Map([...body.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/g)].map((m) => [m[1]!, m[2]!.trim()]));
}

const tokensCss = strip(read('roko-tokens.css'));
const DAY = block(tokensCss, ':root');
const NIGHT = new Map([...DAY, ...block(tokensCss, ':root[data-theme="dark"]')]);

function hex(theme: Map<string, string>, name: string): string {
  const value = theme.get(name);
  if (!value) throw new Error(`${name} is not a token`);
  const ref = /^var\((--[a-z0-9-]+)\)$/.exec(value);
  return ref ? hex(theme, ref[1]!) : value;
}

function luminance(colour: string): number {
  const h = colour.replace('#', '');
  const [r, g, b] = [0, 2, 4].map((i) => {
    const s = parseInt(h.slice(i, i + 2), 16) / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  }) as [number, number, number];
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(theme: Map<string, string>, fg: string, bg: string): number {
  const [hi, lo] = [luminance(hex(theme, fg)), luminance(hex(theme, bg))].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
}

const WORDS = ['--rk-ink', '--rk-ink-2', '--rk-ink-3', '--rk-live', '--rk-pass', '--rk-fail', '--rk-you-text'];
const PANEL = ['--rk-body', '--rk-module', '--rk-module-2'];
const ON_GLASS = ['--rk-px-on', '--rk-px-dim', '--rk-px-faint', '--rk-px-live', '--rk-px-pass', '--rk-px-you', '--rk-px-fail'];

describe('contrast (WCAG 2 AA)', () => {
  for (const [name, theme] of [['day', DAY], ['night', NIGHT]] as const) {
    it(`gives every word at least 4.5:1 on every panel surface by ${name}`, () => {
      for (const fg of WORDS) for (const bg of PANEL) expect(contrast(theme, fg, bg), `${fg} on ${bg}`).toBeGreaterThanOrEqual(4.5);
    });

    it(`gives a key's words and the orange key's words at least 4.5:1 by ${name}`, () => {
      expect(contrast(theme, '--rk-key-ink', '--rk-key')).toBeGreaterThanOrEqual(4.5);
      expect(contrast(theme, '--rk-you-ink', '--rk-you')).toBeGreaterThanOrEqual(4.5);
    });
  }

  it('gives every reading on the glass at least 4.5:1', () => {
    for (const fg of ON_GLASS) expect(contrast(DAY, fg, '--rk-glass'), fg).toBeGreaterThanOrEqual(4.5);
  });

  it('keeps the glass black in both themes', () => {
    expect(hex(NIGHT, '--rk-glass')).toBe(hex(DAY, '--rk-glass'));
  });
});

describe('the older names', () => {
  it('only point at roko’s tokens and define no colour of their own', () => {
    const aliases = strip(read('tokens.css'));
    expect(aliases.match(/#[0-9a-f]{3,8}\b|rgba?\(|hsla?\(/gi) ?? []).toEqual([]);
  });

  it('leave no ROSEDUST colour in the portal', () => {
    expect(existsSync(join(STYLES, 'rosedust.css'))).toBe(false);
    for (const name of ['globals.css', 'tokens.css', 'portal.css']) expect(read(name)).not.toMatch(/#c06070|#e87070|#8993f0/i);
  });
});

describe('cascade layers', () => {
  it('puts every element-level rule inside @layer base', () => {
    const offenders = rules
      .filter((r) => !r.context.some((c) => c.startsWith('@keyframes')))
      .filter((r) => isElementSelector(r.selector) && !inLayer(r, 'base'))
      .map((r) => r.selector);
    expect(offenders).toEqual([]);
  });

  it('keeps the element resets (buttons, form fields, headings, links)', () => {
    const base = rules.filter((r) => inLayer(r, 'base')).map((r) => r.selector).join(' | ');
    for (const element of ['button', 'input', 'h1', 'a', 'body', '::selection']) expect(base).toContain(element);
  });

  it('draws the keyboard focus ring in orange: focus is yours', () => {
    const focus = rules.find((r) => r.selector === ':focus-visible');
    expect(focus?.body).toMatch(/outline\s*:[^;]*var\(--rk-you\)/);
  });

  it('brings in roko’s tokens and puts the kit and the portal’s frame in the components layer', () => {
    expect(globals).toMatch(/@import "\.\/roko-tokens\.css";/);
    expect(globals).toMatch(/@import "\.\/roko-kit\.css" layer\(components\);/);
    expect(globals).toMatch(/@import "\.\/portal\.css" layer\(components\);/);
  });
});

describe('theme colours', () => {
  it('declares every colour name the components use', () => {
    const undeclared = sourceFiles(join(SRC, 'components'))
      .flatMap((file) => colourUtilities(readFileSync(file, 'utf8')))
      .filter((u) => u.startsWith('!undeclared:'))
      .map((u) => u.slice('!undeclared:'.length));
    expect([...new Set(undeclared)]).toEqual([]);
  });
});

describe('the run card’s pads (bug-173a60)', () => {
  const kit = parse(read('roko-kit.css'));
  const portal = parse(read('portal.css'));
  const all = [...kit, ...portal];

  it('leave a passed pad its plain outline, as the design system’s §4.7 does', () => {
    const coloured = all.filter(
      (r) => /\.rk-pad\[data-state="pass"\]/.test(r.selector) && /(^|;)\s*(border|border-color|outline)\s*:/.test(r.body),
    );
    expect(coloured.map((r) => r.selector)).toEqual([]);
  });

  it('cap a pad at the kit’s 240px, so two steps on a wide page stay pads', () => {
    const width = (rules: Rule[], selector: string) =>
      rules
        .find((r) => r.selector === selector)
        ?.body.split(';')
        .map((d) => d.split(':'))
        .find(([p]) => p?.trim() === 'grid-auto-columns')?.[1]
        ?.trim();
    expect(width(kit, '.rk-seq-grid')).toBe('minmax(152px, 240px)');
    expect(width(portal, '.pt-seq .rk-seq-grid')).toMatch(/240px\)$/);
  });
});

describe('fonts', () => {
  it('self-hosts Archivo and Martian Mono, each with its licence', () => {
    const faces = read('fonts.css');
    for (const [file, licence] of [
      ['archivo-latin.woff2', 'OFL-Archivo.txt'],
      ['martian-mono-latin.woff2', 'OFL-MartianMono.txt'],
    ] as const) {
      expect(faces).toContain(`/fonts/${file}`);
      expect(existsSync(join(PUBLIC, 'fonts', file))).toBe(true);
      expect(readFileSync(join(PUBLIC, 'fonts', licence), 'utf8')).toMatch(/SIL Open Font License, Version 1\.1/);
    }
  });
});
