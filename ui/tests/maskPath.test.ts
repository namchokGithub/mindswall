import { strict as assert } from "node:assert";
import { test } from "node:test";
import { maskUserName } from "../src/maskPath.ts";

test("masks the user name in a profile path", () => {
  assert.equal(maskUserName("C:\\Users\\Namchok\\AppData\\Roaming\\MindsWall\\data\\mindswall.log"), "C:\\Users\\******\\AppData\\Roaming\\MindsWall\\data\\mindswall.log");
  assert.equal(maskUserName("d:/users/a/AppData"), "d:/users/******/AppData");
  assert.equal(maskUserName("\\\\?\\C:\\Users\\Long Name\\x"), "\\\\?\\C:\\Users\\******\\x");
});

test("leaves other paths unchanged", () => {
  assert.equal(maskUserName("D:\\Logs\\mindswall.log"), "D:\\Logs\\mindswall.log");
  assert.equal(maskUserName("/home/user/.local/share/mindswall.log"), "/home/user/.local/share/mindswall.log");
});
