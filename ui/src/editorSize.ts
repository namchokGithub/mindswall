export type Size = [number, number];

function gcd(a: number, b: number): number { return b === 0 ? a : gcd(b, a % b); }

/** An aspect ratio such as "16:9", or "2.39:1" when the whole numbers would be large. */
export function aspectLabel([width, height]: Size): string {
  const divisor = gcd(width, height) || 1;
  const [w, h] = [width / divisor, height / divisor];
  return w <= 32 && h <= 32 ? `${w}:${h}` : `${(width / height).toFixed(2)}:1`;
}

/** The other side for `value` that keeps the shape of `base`, or "" when `value` is not a whole number. */
export function linkedSide(value: string, base: Size, changed: "width" | "height"): string {
  if (!/^\d+$/.test(value) || Number(value) === 0) return "";
  const [width, height] = base;
  const other = changed === "width" ? Number(value) * height / width : Number(value) * width / height;
  return String(Math.max(1, Math.round(other)));
}

/** Whether resizing `base` to `target` changes its shape by more than 1%. */
export function isStretched(base: Size, target: Size): boolean {
  const before = base[0] / base[1], after = target[0] / target[1];
  return Math.abs(after - before) / before > 0.01;
}

/**
 * Draw several images at one scale: every box shares the aspect ratio of the
 * largest width and height, and each image takes its share of that box.
 */
export function sharedScale(sizes: Size[]) {
  const width = Math.max(...sizes.map(size => size[0]));
  const height = Math.max(...sizes.map(size => size[1]));
  return {
    aspect: width / height,
    images: sizes.map(([w, h]) => ({ width: `${w / width * 100}%`, height: `${h / height * 100}%` })),
  };
}
