import { strict as assert } from "node:assert";
import { test } from "node:test";
import { aspectLabel, isStretched, linkedSide, sharedScale } from "../src/editorSize.ts";

test("names common aspect ratios", () => {
  assert.equal(aspectLabel([1920, 1080]), "16:9");
  assert.equal(aspectLabel([4000, 3000]), "4:3");
  assert.equal(aspectLabel([1080, 1920]), "9:16");
  assert.equal(aspectLabel([2390, 1000]), "2.39:1");
});

test("keeps the shape when one side changes", () => {
  assert.equal(linkedSide("1920", [4000, 3000], "width"), "1440");
  assert.equal(linkedSide("1080", [4000, 3000], "height"), "1440");
  assert.equal(linkedSide("", [4000, 3000], "width"), "");
  assert.equal(linkedSide("0", [4000, 3000], "width"), "");
  assert.equal(linkedSide("1", [4000, 1], "width"), "1");
});

test("detects stretched sizes", () => {
  assert.equal(isStretched([4000, 3000], [1920, 1440]), false);
  assert.equal(isStretched([4000, 3000], [1920, 1080]), true);
});

test("draws images at one scale", () => {
  const scale = sharedScale([[4000, 3000], [1920, 1440]]);
  assert.equal(scale.aspect, 4000 / 3000);
  assert.deepEqual(scale.images[0], { width: "100%", height: "100%" });
  assert.deepEqual(scale.images[1], { width: "48%", height: "48%" });
});
