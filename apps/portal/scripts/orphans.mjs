#!/usr/bin/env node
/**
 * orphans.mjs — find unreachable source files in apps/portal/src/
 *
 * Starts from every file under src/app/, follows all imports
 * (static, side-effect, dynamic, CSS @import), and reports every
 * .ts, .tsx, .css file under src/ that is not reached.
 *
 * Test files (*.test.ts, *.test.tsx) are excluded from the orphan
 * check — and they do NOT count as reaching their subject modules.
 * A module imported only by its own test is therefore an orphan.
 *
 * Exit 0: no orphans.  Exit 1: one or more orphans printed to stdout.
 */

import { readFileSync, existsSync, readdirSync, statSync } from 'fs';
import { resolve, dirname, join, relative } from 'path';

// apps/portal/ — one level up from scripts/
const ROOT = new URL('..', import.meta.url).pathname;
const SRC = join(ROOT, 'src');

// ── specifier resolution ────────────────────────────────────────────────────

/** Try each candidate extension/index variant and return the first that exists. */
function tryResolve(base) {
  const candidates = [
    base,
    `${base}.ts`,
    `${base}.tsx`,
    `${base}.css`,
    join(base, 'index.ts'),
    join(base, 'index.tsx'),
  ];
  for (const c of candidates) {
    if (existsSync(c)) return c;
  }
  return null;
}

/**
 * Resolve an import specifier from `fromFile`.
 * Returns an absolute path or null if the specifier is a package name.
 */
function resolveSpecifier(specifier, fromFile) {
  if (specifier.startsWith('@/')) {
    // Alias: @/ → src/
    return tryResolve(join(SRC, specifier.slice(2)));
  }
  if (specifier.startsWith('.')) {
    // Relative
    return tryResolve(resolve(dirname(fromFile), specifier));
  }
  // Package specifier — ignore
  return null;
}

// ── import extraction ───────────────────────────────────────────────────────

/** Extract all import specifiers from a .ts / .tsx source file. */
function extractTsImports(content) {
  const specifiers = [];

  // Static: import ... from '...'  and side-effect: import '...'
  // The optional non-capturing group handles the "... from" part.
  const staticRe =
    /\bimport\s+(?:type\s+)?(?:[^'"]*?\s+from\s+)?['"]([^'"]+)['"]/g;
  let m;
  while ((m = staticRe.exec(content)) !== null) {
    specifiers.push(m[1]);
  }

  // Dynamic: import('...')
  const dynamicRe = /\bimport\s*\(\s*['"]([^'"]+)['"]\s*\)/g;
  while ((m = dynamicRe.exec(content)) !== null) {
    specifiers.push(m[1]);
  }

  return specifiers;
}

/** Extract @import specifiers from a .css file. */
function extractCssImports(content) {
  const specifiers = [];
  const re = /@import\s+["']([^"']+)["']/g;
  let m;
  while ((m = re.exec(content)) !== null) {
    specifiers.push(m[1]);
  }
  return specifiers;
}

// ── file collection ─────────────────────────────────────────────────────────

/** Recursively collect .ts, .tsx, .css files under `dir`. */
function collectFiles(dir, { includeTests = false } = {}) {
  const result = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      result.push(...collectFiles(full, { includeTests }));
    } else {
      const isTs = full.endsWith('.ts') || full.endsWith('.tsx');
      const isCss = full.endsWith('.css');
      if (!isTs && !isCss) continue;
      const isTest = full.endsWith('.test.ts') || full.endsWith('.test.tsx');
      if (isTest && !includeTests) continue;
      result.push(full);
    }
  }
  return result;
}

// ── BFS reachability ────────────────────────────────────────────────────────

/**
 * BFS from `roots`, following imports, and return the set of all reachable
 * files.  Test files are never added to the queue so they cannot "reach"
 * anything on behalf of the production graph.
 */
function findReachable(roots) {
  const visited = new Set();
  const queue = [...roots];

  while (queue.length > 0) {
    const file = queue.shift();
    if (visited.has(file)) continue;
    visited.add(file);

    let content;
    try {
      content = readFileSync(file, 'utf-8');
    } catch {
      continue;
    }

    const specifiers = file.endsWith('.css')
      ? extractCssImports(content)
      : extractTsImports(content);

    for (const spec of specifiers) {
      const resolved = resolveSpecifier(spec, file);
      if (!resolved) continue;
      // Never enqueue test files — they don't count as production reaches.
      const isTest =
        resolved.endsWith('.test.ts') || resolved.endsWith('.test.tsx');
      if (!isTest && !visited.has(resolved)) {
        queue.push(resolved);
      }
    }
  }

  return visited;
}

// ── main ────────────────────────────────────────────────────────────────────

// Roots: all non-test source files directly under src/app/
const appDir = join(SRC, 'app');
const roots = collectFiles(appDir);

const reachable = findReachable(roots);

// Candidates: all non-test source files under src/
const all = collectFiles(SRC);

const orphans = all
  .filter((f) => !reachable.has(f))
  .sort();

if (orphans.length === 0) {
  console.log('no orphans');
  process.exit(0);
} else {
  for (const f of orphans) {
    console.log(relative(ROOT, f));
  }
  process.exit(1);
}
