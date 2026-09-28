#!/usr/bin/env node
/**
 * vitest-min.mjs — minimum-count test gate helper.
 *
 * Usage: node scripts/vitest-min.mjs <test-file> <min>
 *
 * Runs vitest on <test-file> and exits:
 *   0  — vitest succeeded, no failed tests, and passed count >= <min>
 *   1  — vitest failed OR numFailedTests > 0 OR numPassedTests < min
 *   2  — bad usage (missing args, min is not a positive integer)
 */

import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const [, , testFile, minArg] = process.argv;

if (!testFile) {
  process.stderr.write('Usage: node scripts/vitest-min.mjs <test-file> <min>\n');
  process.exit(2);
}

const min = Number(minArg);
if (!Number.isInteger(min) || min < 1) {
  process.stderr.write(
    `Error: <min> must be a positive integer, got: ${JSON.stringify(minArg)}\n`,
  );
  process.exit(2);
}

const jsonOut = join(tmpdir(), `vitest-min-${process.pid}-${Date.now()}.json`);

const result = spawnSync(
  './node_modules/.bin/vitest',
  [
    'run',
    testFile,
    '--reporter=default',
    '--reporter=json',
    `--outputFile.json=${jsonOut}`,
  ],
  { stdio: 'inherit' },
);

if (result.status !== 0) {
  process.exit(1);
}

let report;
try {
  report = JSON.parse(readFileSync(jsonOut, 'utf8'));
} catch {
  process.stderr.write(`Error: could not read vitest JSON report at ${jsonOut}\n`);
  process.exit(1);
}

const { numFailedTests = 0, numPassedTests = 0 } = report;

if (numFailedTests !== 0 || numPassedTests < min) {
  process.exit(1);
}

process.exit(0);
