/**
 * Acceptance: the ROSEDUST styles actually apply.
 *
 * Tailwind v4 puts its utilities in `@layer utilities`; any rule outside a
 * layer beats every layered rule, so un-layered element resets (`button {…}`,
 * `* { border-color }`) silently override utility classes such as `bg-rose`.
 * Element rules therefore live in `@layer base`, the portal's own component
 * classes in `@layer components`, and every colour utility used by a component
 * must name a colour the theme declares.
 *
 * Copied verbatim from plans/portal-programme/08b-portal-polish/accept/.
 */
import { describe, expect, it } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const STYLES = fileURLToPath(new URL('.', import.meta.url));
const SRC = join(STYLES, '..');
const globals = readFileSync(join(STYLES, 'globals.css'), 'utf8');
const tokens = readFileSync(join(STYLES, 'tokens.css'), 'utf8');

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

// ── Tests ───────────────────────────────────────────────────────────────────────

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
    for (const element of ['button', 'input', 'h1', 'a', 'body', '::selection']) {
      expect(base).toContain(element);
    }
  });

  it('draws the keyboard focus ring with the focus token', () => {
    const focus = rules.find((r) => r.selector === ':focus-visible');
    expect(focus?.body).toMatch(/outline\s*:[^;]*var\(--focus-border\)/);
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

  it('declares the warning, base and subtle colours', () => {
    for (const name of ['accent-warn', 'bg-base', 'bg-subtle', 'state-done', 'state-active', 'state-failed']) {
      expect(themeColours.has(name)).toBe(true);
    }
  });
});

describe('layout tokens', () => {
  it('gives the rail room for a name beside its numbers', () => {
    const width = /--rail-width\s*:\s*(\d+)px/.exec(tokens);
    expect(Number(width?.[1])).toBeGreaterThanOrEqual(288);
  });

  it('declares the header height and the short implementer role alias', () => {
    expect(tokens).toMatch(/--header-height\s*:\s*\d+px/);
    expect(tokens).toMatch(/--role-impl\s*:\s*var\(--role-implementer\)/);
  });
});

describe('component classes', () => {
  it('lays the header out as one row', () => {
    const header = componentRule('.rd-header');
    expect(declares(header, 'display', /^flex$/)).toBe(true);
    expect(declares(header, 'flex-wrap', /^nowrap$/)).toBe(true);
    expect(declares(header, 'height', /var\(--header-height\)/)).toBe(true);
  });

  it('lays the run band out as three cells with titles', () => {
    expect(declares(componentRule('.rd-band'), 'display', /^grid$/)).toBe(true);
    expect(declares(componentRule('.rd-band'), 'grid-template-columns', /repeat\(3|minmax/)).toBe(true);
    expect(componentRule('.rd-band__cell')).toBeDefined();
    expect(declares(componentRule('.rd-band__title'), 'text-transform', /^uppercase$/)).toBe(true);
  });

  it('gives a rail row fixed columns and an ellipsised name', () => {
    expect(declares(componentRule('.rd-plan-row'), 'display', /^grid$/)).toBe(true);
    expect(declares(componentRule('.rd-plan-row'), 'grid-template-columns', /minmax\(0,\s*1fr\)/)).toBe(true);
    const name = componentRule('.rd-plan-row__name');
    expect(declares(name, 'overflow', /^hidden$/)).toBe(true);
    expect(declares(name, 'text-overflow', /^ellipsis$/)).toBe(true);
    expect(componentRule('.rd-plan-row__wait')).toBeDefined();
  });

  it('styles the status line and the alert row', () => {
    expect(declares(componentRule('.rd-status'), 'display', /^flex$/)).toBe(true);
    expect(declares(componentRule('.rd-alert'), 'display', /^flex$/)).toBe(true);
  });

  it('uses tokens, never colour literals', () => {
    const literals = rules
      .filter((r) => inLayer(r, 'components'))
      .filter((r) => /#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(/.test(r.body))
      .map((r) => r.selector);
    expect(literals).toEqual([]);
  });
});
