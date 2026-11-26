import { strict as assert } from "node:assert";
import { test } from "node:test";
import { shortcutFromKey } from "../src/shortcutCapture.ts";

const key = (code: string, options: Partial<Parameters<typeof shortcutFromKey>[0]> = {}) => ({ code, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, isComposing: false, ...options });

test("captures physical keys and modifiers in the format accepted by the shortcut service", () => {
  assert.equal(shortcutFromKey(key("ArrowRight", { ctrlKey: true, altKey: true })), "Ctrl+Alt+ArrowRight");
  assert.equal(shortcutFromKey(key("KeyA", { ctrlKey: true, shiftKey: true })), "Ctrl+Shift+KeyA");
  assert.equal(shortcutFromKey(key("F12")), "F12");
  assert.equal(shortcutFromKey(key("NumpadAdd", { ctrlKey: true })), "Ctrl+NumpadAdd");
});

test("ignores incomplete, unsupported and IME key events", () => {
  assert.equal(shortcutFromKey(key("ControlLeft", { ctrlKey: true })), null);
  assert.equal(shortcutFromKey(key("Unidentified", { ctrlKey: true })), null);
  assert.equal(shortcutFromKey(key("KeyA", { isComposing: true })), null);
});
