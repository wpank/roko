import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { camelCase } from "./strings.ts";

// `reverse` as the seed has it, line for line. The task keeps it as it is,
// doc comment included.
const SEED_REVERSE = [
  "/** `text` with its characters in reverse order. */",
  "export function reverse(text: string): string {",
  '  return [...text].reverse().join("");',
  "}",
];

test("camelCase joins words with capitals", () => {
  assert.equal(camelCase("golden path run"), "goldenPathRun");
});

test("camelCase treats hyphens and underscores as spaces", () => {
  assert.equal(camelCase("cheap-model_ladder"), "cheapModelLadder");
});

test("camelCase lower-cases the first word", () => {
  assert.equal(camelCase("Roko Plans"), "rokoPlans");
});

test("reverse is kept as it is, doc comment included", () => {
  const lines = readFileSync(new URL("./strings.ts", import.meta.url), "utf8").split("\n");
  const start = lines.indexOf(SEED_REVERSE[0]);
  assert.notEqual(start, -1, "the doc comment of reverse is gone or changed");
  assert.deepEqual(lines.slice(start, start + SEED_REVERSE.length), SEED_REVERSE);
});
