import assert from "node:assert/strict";
import { test } from "node:test";

import { shout } from "./strings.ts";

test("shout upper-cases every letter", () => {
  assert.equal(shout("ship it"), "SHIP IT");
});
