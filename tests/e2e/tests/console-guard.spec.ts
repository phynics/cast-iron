import { test, expect } from "./fixtures";

// The console guard's tolerance for WebKit's "Context is stopped" page error, with a real page.
// The test raises the exact message itself, so the boundaries are exact: the error is tolerated
// while the main frame navigates away, and counted after the new document commits and after a
// navigation that failed. Service workers are blocked so that `page.route` sees the navigations.
// The window's own order rules are in `teardown-window.spec.ts`.

test.use({ serviceWorkers: "block" });

// A document with no script: the app's own start-up is not interrupted by the navigations below.
const PLAIN = "manifest.webmanifest";

const raise = (page: import("@playwright/test").Page) =>
  page.evaluate(() => {
    setTimeout(() => {
      throw new Error("Context is stopped");
    }, 0);
  });

test.beforeEach(({ browserName }) => {
  test.skip(browserName !== "webkit", "the guard tolerates this message on WebKit only");
});

test("The error is tolerated while the page navigates away, and recorded", async ({ page, guard }) => {
  let release!: () => void;
  const held = new Promise<void>((resolve) => (release = resolve));
  let requested!: () => void;
  const seen = new Promise<void>((resolve) => (requested = resolve));
  await page.route("**/hold", async (route) => {
    requested();
    await held;
    await route.fulfill({ status: 200, contentType: "text/html", body: "<!doctype html><title>held</title>" });
  });
  await page.goto(PLAIN);
  const navigation = page.goto("hold");
  await seen;
  await raise(page);
  await expect.poll(() => guard.teardown.length).toBe(1);
  expect(guard.teardown[0]).toContain("/cast-iron/hold");
  expect(guard.problems()).toEqual([]);
  release();
  await navigation;
});

test("The same error after the new document committed fails the test", async ({ page, guard }) => {
  await page.goto(PLAIN);
  await raise(page);
  await expect.poll(() => guard.console.length).toBe(1);
  expect(guard.problems()).toEqual(["pageerror: Context is stopped"]);
  expect(guard.teardown).toEqual([]);
  guard.console.length = 0;
});

test("The same error on a page that survived a failed navigation fails the test", async ({ page, guard }) => {
  await page.route("**/aborted", (route) => route.abort());
  await page.goto(PLAIN);
  await page.goto("aborted").catch(() => {});
  await raise(page);
  await expect.poll(() => guard.console.length).toBe(1);
  expect(guard.problems()).toEqual(["pageerror: Context is stopped"]);
  expect(guard.teardown).toEqual([]);
  guard.console.length = 0;
});
