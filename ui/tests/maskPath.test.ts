import { strict as assert } from "node:assert";
import { test } from "node:test";
import { maskFolderPath, maskUserName } from "../src/maskPath.ts";

test("masks the user name in a profile path", () => {
  assert.equal(maskUserName("C:\\Users\\Namchok\\AppData\\Roaming\\MindsWall\\data\\mindswall.log"), "C:\\Users\\******\\AppData\\Roaming\\MindsWall\\data\\mindswall.log");
  assert.equal(maskUserName("d:/users/a/AppData"), "d:/users/******/AppData");
  assert.equal(maskUserName("\\\\?\\C:\\Users\\Long Name\\x"), "\\\\?\\C:\\Users\\******\\x");
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
