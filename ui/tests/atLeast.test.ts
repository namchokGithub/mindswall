import { strict as assert } from "node:assert";
import { test } from "node:test";
import { atLeast } from "../src/atLeast.ts";

test("a fast task still takes the minimum time", async () => {
  const started = Date.now();
  assert.equal(await atLeast(Promise.resolve(7), 60), 7);
  assert.ok(Date.now() - started >= 55);
});

test("a slow task is not delayed further", async () => {
  const started = Date.now();
  await atLeast(new Promise(resolve => setTimeout(resolve, 80)), 20);
  assert.ok(Date.now() - started < 150);
});

test("an error is passed on after the minimum time", async () => {
  const started = Date.now();
  await assert.rejects(atLeast(Promise.reject(new Error("failed")), 60), /failed/);
  assert.ok(Date.now() - started >= 55);
});
