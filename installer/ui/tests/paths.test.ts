import { test } from "node:test";
import assert from "node:assert/strict";
import { isAbsoluteWindowsPath, needsAdmin } from "../src/paths.ts";

test("only full Windows paths are accepted", () => {
  assert.ok(isAbsoluteWindowsPath("C:\\Users\\You\\Apps\\MindsWall"));
  assert.ok(isAbsoluteWindowsPath("  d:\\MindsWall "));
  assert.ok(isAbsoluteWindowsPath("\\\\server\\share\\MindsWall"));
  assert.ok(!isAbsoluteWindowsPath("MindsWall"));
  assert.ok(!isAbsoluteWindowsPath("C:MindsWall"));
  assert.ok(!isAbsoluteWindowsPath("/home/you"));
});

test("folders that need administrator rights are flagged", () => {
  assert.ok(needsAdmin("C:\\Program Files\\MindsWall"));
  assert.ok(needsAdmin("c:\\program files (x86)\\MindsWall"));
  assert.ok(needsAdmin("C:\\Windows"));
  assert.ok(!needsAdmin("C:\\Users\\You\\AppData\\Local\\Programs\\MindsWall"));
  assert.ok(!needsAdmin("D:\\Program Files Portable\\MindsWall"));
});
