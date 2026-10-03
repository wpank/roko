import assert from "node:assert/strict";
import { test } from "node:test";

import { reverse } from "./strings.ts";

test("reverse turns a word around", () => {
  assert.equal(reverse("roko"), "okor");
});

test("reverse keeps an empty string empty", () => {
  assert.equal(reverse(""), "");
});
