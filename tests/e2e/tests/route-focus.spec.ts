import { installedTest as test, expect, type Page } from "./fixtures";
import { installLibrary } from "./helpers";

/**
 * The frame that a route change schedules to move focus to <main> (#419) yields to a control the
 * reader focused after the navigation began, and a newer navigation cancels an older frame.
 * Decision: docs/decisions/2026-11-17-route-focus-move-yields-to-the-reader.md.
 *
 * Both tests act inside a `hashchange` listener that the page adds after the app's own, so the
 * app's effect has already scheduled its frame when the listener runs. That makes the race
 * deterministic instead of a matter of how fast the host is.
 */

const frames = (page: Page, count = 3) =>
  page.evaluate(
    (count) =>
      new Promise<void>((done) => {
        const tick = (left: number) => (left === 0 ? done() : requestAnimationFrame(() => tick(left - 1)));
        tick(count);
      }),
    count,
  );

test("A control focused after the navigation keeps focus when the frame fires", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/");
  await expect(page.getByRole("link", { name: "Legal and attribution" })).toBeVisible();
  await frames(page);

  await page.evaluate(() => {
    window.addEventListener(
      "hashchange",
      () => document.querySelector<HTMLElement>("footer a[href*='legal']")?.focus(),
      { once: true },
    );
    location.hash = "#/books";
  });
  await expect(page.getByRole("heading", { level: 1, name: /books/i })).toBeVisible();
  await frames(page);
  await page.waitForTimeout(100);

  await expect(page.getByRole("link", { name: "Legal and attribution" })).toBeFocused();
  expect(await page.evaluate(() => document.activeElement?.id)).not.toBe("main");
});

test("Without a focused control, the route change still moves focus to <main>", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/");
  await frames(page);
  await page.evaluate(() => {
    location.hash = "#/books";
  });
  await expect(page.locator("main#main")).toBeFocused();
});

test("A newer navigation cancels the focus move of an older one", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/");
  await frames(page);
  await page.evaluate(() => {
    const w = window as unknown as { mainFocusCalls: number };
    w.mainFocusCalls = 0;
    const focus = HTMLElement.prototype.focus;
    HTMLElement.prototype.focus = function (this: HTMLElement, ...args: Parameters<HTMLElement["focus"]>) {
      if (this.id === "main") w.mainFocusCalls += 1;
      return focus.apply(this, args);
    };
    // Two navigations before the next frame: only the second may move focus.
    location.hash = "#/books";
    location.hash = "#/settings";
  });
  await expect(page.getByRole("heading", { level: 1, name: "Settings" })).toBeVisible();
  await expect(page.locator("main#main")).toBeFocused();
  await frames(page);
  expect(await page.evaluate(() => (window as unknown as { mainFocusCalls: number }).mainFocusCalls)).toBe(1);
});
