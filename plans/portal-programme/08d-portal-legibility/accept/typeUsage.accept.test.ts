/**
 * Acceptance: no component shrinks text below the meta size (12px) or dims it
 * with opacity — the muted and faint tokens carry secondary text, and their
 * contrast is checked in legibility.accept.test.ts.
 * Copied verbatim from plans/portal-programme/08d-portal-legibility/accept/.
 */
import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const SRC = join(fileURLToPath(new URL('.', import.meta.url)), '..');
const COMPONENTS = join(SRC, 'components');

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return sources(path);
    return /\.tsx$/.test(name) && !/\.test\.tsx$/.test(name) ? [path] : [];
  });
}

function scan(pattern: RegExp, bad: (m: RegExpMatchArray) => boolean): string[] {
  const found: string[] = [];
  for (const file of sources(COMPONENTS)) {
    const text = readFileSync(file, 'utf8');
    for (const m of text.matchAll(pattern)) {
      if (bad(m)) found.push(`${relative(SRC, file)}: ${m[0]}`);
    }
  }
  return found;
}

describe('type and dimming across components', () => {
  it('uses no arbitrary text size under 12px', () => {
    expect(scan(/text-\[(\d+(?:\.\d+)?)px\]/g, (m) => Number(m[1]) < 12)).toEqual([]);
  });

  it('sets no inline font size under 12px or under 1em', () => {
    expect(scan(/fontSize:\s*['"](\d*\.?\d+)(px|em|rem)['"]/g, (m) => (m[2] === 'px' ? Number(m[1]) < 12 : Number(m[1]) < 1))).toEqual([]);
  });

  it('dims no text with an inline opacity', () => {
    expect(scan(/opacity:\s*['"]?(0?\.\d+)/g, () => true)).toEqual([]);
  });
});
