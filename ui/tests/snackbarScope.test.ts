import { strict as assert } from "node:assert";
import { test } from "node:test";
import { createSnackbarScope } from "../src/snackbarScope.ts";

test("ignores results from a tab visited before navigation, including after returning", () => {
  const scope = createSnackbarScope("overview");
  const previousRequest = scope.capture("overview");
  assert.equal(scope.accepts(previousRequest), true);
  scope.navigate("settings");
  assert.equal(scope.accepts(previousRequest), false);
  const settingsRequest = scope.capture("settings");
  assert.equal(scope.accepts(settingsRequest), true);
  scope.navigate("overview");
  assert.equal(scope.accepts(previousRequest), false);
  assert.equal(scope.accepts(settingsRequest), false);
  assert.equal(scope.accepts(scope.capture("overview")), true);
});

test("selecting the current tab does not invalidate its pending result", () => {
  const scope = createSnackbarScope("settings");
  const request = scope.capture("settings");
  scope.navigate("settings");
  assert.equal(scope.accepts(request), true);
  assert.equal(scope.accepts(scope.capture("gallery")), false);
});
