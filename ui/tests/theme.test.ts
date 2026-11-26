import { strict as assert } from "node:assert";
import { test } from "node:test";
import { resolveTheme } from "../src/theme.ts";

test("system follows the Windows app mode; explicit themes do not", () => {
  assert.equal(resolveTheme("system", true), "dark");
  assert.equal(resolveTheme("system", false), "light");
  assert.equal(resolveTheme("light", true), "light");
  assert.equal(resolveTheme("dark", false), "dark");
});
