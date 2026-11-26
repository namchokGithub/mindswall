import { strict as assert } from "node:assert";
import { test } from "node:test";
import { createThumbnailQueue } from "../src/thumbnailQueue.ts";

test("limits concurrent loads and skips thumbnails for an inactive page", async () => {
  const enqueue = createThumbnailQueue(2);
  const releases: Array<() => void> = [];
  let running = 0;
  let peak = 0;
  const load = () => new Promise<void>(resolve => {
    running++;
    peak = Math.max(peak, running);
    releases.push(() => { running--; resolve(); });
  });
  let visible = true;
  const first = enqueue(load, () => true);
  const second = enqueue(load, () => true);
  const cancelled = enqueue(load, () => visible).catch(() => undefined);
  const last = enqueue(load, () => true);
  await new Promise(resolve => setTimeout(resolve, 0));
  assert.equal(releases.length, 2);
  visible = false;
  releases[0]();
  await new Promise(resolve => setTimeout(resolve, 0));
  assert.equal(releases.length, 3);
  releases[1]();
  releases[2]();
  await Promise.all([first, second, cancelled, last]);
  assert.equal(peak, 2);
});
