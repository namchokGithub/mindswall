import { strict as assert } from "node:assert";
import { test } from "node:test";
import { maskFolderPath, maskUserName } from "../src/maskPath.ts";

test("masks the user name in a profile path", () => {
  assert.equal(maskUserName("C:\\Users\\Namchok\\AppData\\Roaming\\MindsWall\\data\\mindswall.log"), "C:\\Users\\******\\AppData\\Roaming\\MindsWall\\data\\mindswall.log");
  assert.equal(maskUserName("d:/users/a/AppData"), "d:/users/******/AppData");
  assert.equal(maskUserName("\\\\?\\C:\\Users\\Long Name\\x"), "\\\\?\\C:\\Users\\******\\x");
});

test("masks user names anywhere in a log line", () => {
  const line = String.raw`2026-11-27T10:00:00Z  INFO mindswall_core::service: wallpaper changed path="C:\\Users\\Namchok\\Pictures\\a.jpg" ram_mb=17`;
  assert.equal(maskUserName(line), String.raw`2026-11-27T10:00:00Z  INFO mindswall_core::service: wallpaper changed path="C:\\Users\\******\\Pictures\\a.jpg" ram_mb=17`);
  assert.equal(maskUserName(String.raw`from C:\Users\a\x to D:/Users/b/y`), String.raw`from C:\Users\******\x to D:/Users/******/y`);
  assert.equal(maskUserName(String.raw`path="C:\Users\Long Name"`), String.raw`path="C:\Users\******"`);
});

test("leaves other paths unchanged", () => {
  assert.equal(maskUserName("D:\\Logs\\mindswall.log"), "D:\\Logs\\mindswall.log");
  assert.equal(maskUserName("/home/user/.local/share/mindswall.log"), "/home/user/.local/share/mindswall.log");
});

test("shows only the last folder of a folder path", () => {
  assert.equal(maskFolderPath("D:\\coding\\mindswall\\example.wallpaper"), "******\\example.wallpaper");
  assert.equal(maskFolderPath("C:\\Users\\Namchok\\Pictures\\"), "******\\Pictures");
  assert.equal(maskFolderPath("\\\\server\\share\\walls"), "******\\walls");
  assert.equal(maskFolderPath("d:/a/b"), "******/b");
});

test("leaves drive folders and roots unchanged", () => {
  assert.equal(maskFolderPath("D:\\wallpapers"), "D:\\wallpapers");
  assert.equal(maskFolderPath("D:\\"), "D:\\");
  assert.equal(maskFolderPath("\\\\?\\D:\\wallpapers"), "\\\\?\\D:\\wallpapers");
  assert.equal(maskFolderPath("wallpapers"), "wallpapers");
});
