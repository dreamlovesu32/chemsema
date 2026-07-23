import assert from "node:assert/strict";
import { createUiActionRunner } from "../viewer/ui_action_runner.js";

const notifications = [];
const failures = [];
const traces = [];
const loggedErrors = [];
let recovered = false;
const originalConsoleError = console.error;
console.error = (...args) => loggedErrors.push(args);
const runner = createUiActionRunner({
  notify: (message) => notifications.push(message),
  trace: async (scope, detail) => traces.push({ scope, detail }),
  onFailure: (failure) => failures.push(failure),
  isAbortError: (error) => error?.name === "AbortError",
});

await assert.rejects(
  runner.run("save-document", async () => {
    throw new Error("disk unavailable");
  }, {
    recover: async () => {
      recovered = true;
    },
  }),
  /disk unavailable/,
);
assert.equal(recovered, true);
assert.deepEqual(notifications, ["disk unavailable"]);
assert.equal(failures.length, 1);
assert.equal(failures[0].scope, "save-document");
assert.equal(traces.length, 1);

const abort = new Error("cancelled");
abort.name = "AbortError";
assert.equal(await runner.run("open-document", async () => {
  throw abort;
}), undefined);
assert.equal(failures.length, 1, "user cancellation must not be reported as a failure");

const eventFailures = [];
const eventRunner = createUiActionRunner({
  notify: (message) => eventFailures.push(message),
});
const listener = eventRunner.listener("toolbar-action", async () => {
  throw new Error("toolbar failed");
});
try {
  listener({ type: "click" });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.deepEqual(eventFailures, ["toolbar failed"]);
  assert.equal(loggedErrors.length, 2);
} finally {
  console.error = originalConsoleError;
}

console.log("[ui-action-runner-regression] ok");
