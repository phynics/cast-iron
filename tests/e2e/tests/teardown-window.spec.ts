import { test, expect } from "@playwright/test";
import { TeardownWindow } from "./teardown-window";

// The window in which the console guard tolerates WebKit's "Context is stopped" page error. No
// browser runs here: the cases are the event orders that the guard feeds into the window.

const request = (name: string) => ({ name });

test("A navigation opens the window and the new document's commit closes it", () => {
  const window = new TeardownWindow();
  expect(window.navigating).toBeNull();
  const nav = request("nav");
  window.begin(nav, "http://127.0.0.1/cast-iron/");
  expect(window.navigating).toBe("http://127.0.0.1/cast-iron/");
  window.commit();
  expect(window.navigating).toBeNull();
});

test("A failed or cancelled navigation closes the window, so the surviving page is guarded again", () => {
  const window = new TeardownWindow();
  const nav = request("nav");
  window.begin(nav, "http://127.0.0.1/cast-iron/");
  window.fail(nav);
  expect(window.navigating).toBeNull();
});

test("A superseded request failing does not close the window of the newer one", () => {
  const window = new TeardownWindow();
  const first = request("first");
  const second = request("second");
  window.begin(first, "http://127.0.0.1/cast-iron/#/a");
  window.begin(second, "http://127.0.0.1/cast-iron/#/b");
  window.fail(first);
  expect(window.navigating).toBe("http://127.0.0.1/cast-iron/#/b");
  window.fail(second);
  expect(window.navigating).toBeNull();
});

test("A redirect target replaces the tracked request, and its failure closes the window", () => {
  const window = new TeardownWindow();
  const original = request("original");
  const redirected = request("redirected");
  window.begin(original, "http://127.0.0.1/cast-iron/old");
  window.begin(redirected, "http://127.0.0.1/cast-iron/new");
  expect(window.navigating).toBe("http://127.0.0.1/cast-iron/new");
  window.fail(redirected);
  expect(window.navigating).toBeNull();
});

test("A commit with no navigation request, as a hash change does, leaves the window closed", () => {
  const window = new TeardownWindow();
  window.commit();
  expect(window.navigating).toBeNull();
});

test("After the commit a later failure of the same request does not reopen the window", () => {
  const window = new TeardownWindow();
  const nav = request("nav");
  window.begin(nav, "http://127.0.0.1/cast-iron/");
  window.commit();
  window.fail(nav);
  expect(window.navigating).toBeNull();
});
