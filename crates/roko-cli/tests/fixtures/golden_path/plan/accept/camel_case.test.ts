import assert from "node:assert/strict";
import { test } from "node:test";

import { camelCase } from "./strings.ts";

test("camelCase joins words with capitals", () => {
  assert.equal(camelCase("golden path run"), "goldenPathRun");
});

test("camelCase treats hyphens and underscores as spaces", () => {
  assert.equal(camelCase("cheap-model_ladder"), "cheapModelLadder");
});

test("camelCase lower-cases the first word", () => {
  assert.equal(camelCase("Roko Plans"), "rokoPlans");
});
