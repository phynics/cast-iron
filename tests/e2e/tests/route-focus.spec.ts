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

/** Counts the calls that put focus on <main> and the calls that scroll the window, from now on. */
const spyOnMoves = (page: Page) =>
  page.evaluate(() => {
    const w = window as unknown as { moves: { main: number; scroll: number } };
    w.moves = { main: 0, scroll: 0 };
    const focus = HTMLElement.prototype.focus;
    HTMLElement.prototype.focus = function (this: HTMLElement, ...args: Parameters<HTMLElement["focus"]>) {
      if (this.id === "main") w.moves.main += 1;
      return focus.apply(this, args);
    };
    const scrollTo = window.scrollTo.bind(window);
    window.scrollTo = ((...args: unknown[]) => {
      w.moves.scroll += 1;
      return (scrollTo as (...a: unknown[]) => void)(...args);
    }) as typeof window.scrollTo;
  });
const moves = (page: Page) =>
  page.evaluate(() => (window as unknown as { moves: { main: number; scroll: number } }).moves);

// Each start has a known focus that is not <main>: nothing, or a link that stays on the page.
for (const start of ["nothing focused", "a footer link focused"]) {
  test(`With ${start}, the route change still moves focus to <main>`, async ({ page }) => {
    await installLibrary(page);
    await page.goto("#/");
    await expect(page.getByRole("link", { name: "Legal and attribution" })).toBeVisible();
    await frames(page);
    await page.evaluate((start) => {
      const active = document.activeElement as HTMLElement | null;
      if (start === "nothing focused") active?.blur();
      else document.querySelector<HTMLElement>("footer a[href*='legal']")?.focus();
    }, start);
    const before = await page.evaluate(() => document.activeElement?.id ?? document.activeElement?.tagName);
    expect(before).not.toBe("main");
    await spyOnMoves(page);

    await page.evaluate(() => {
      location.hash = "#/books";
    });
    await expect(page.locator("main#main")).toBeFocused();
    expect((await moves(page)).main).toBe(1);
  });
}

test("A newer navigation cancels the focus move and the scroll of an older one", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/");
  await frames(page);
  await spyOnMoves(page);
  // Hold the animation frames, so that the order is the test's and not the host's.
  await page.evaluate(() => {
    const w = window as unknown as {
      held: FrameRequestCallback[];
      release: () => void;
    };
    const original = window.requestAnimationFrame.bind(window);
    w.held = [];
    window.requestAnimationFrame = (callback) => w.held.push(callback);
    w.release = () => {
      window.requestAnimationFrame = original;
      const callbacks = w.held.splice(0);
      for (const callback of callbacks) callback(performance.now());
    };
  });
  const held = () => page.evaluate(() => (window as unknown as { held: unknown[] }).held.length);

  // Books is processed and its frame is waiting.
  await page.evaluate(() => {
    location.hash = "#/books";
  });
  await expect(page.getByRole("heading", { level: 1, name: "Books" })).toBeVisible();
  const afterBooks = await held();
  expect(afterBooks).toBeGreaterThan(0);

  // Settings is processed and queued its own frame behind the first.
  await page.evaluate(() => {
    location.hash = "#/settings";
  });
  await expect(page.getByRole("heading", { level: 1, name: "Settings" })).toBeVisible();
  expect(await held()).toBeGreaterThan(afterBooks);
  expect(await moves(page)).toEqual({ main: 0, scroll: 0 });

  // Release the frames in the order they were queued: only the latest navigation acts.
  await page.evaluate(() => (window as unknown as { release: () => void }).release());
  await expect(page.locator("main#main")).toBeFocused();
  await frames(page);
  expect(await moves(page)).toEqual({ main: 1, scroll: 1 });
});

// The route level replaces a page instead of patching it (decision
// docs/decisions/2026-11-23-page-views-are-replaced-not-patched.md). Two entries are the same kind of
// page, so this is the case where the old page's nodes could be reused: nothing the reader did on
// the first entry may show on the second, and focus and scroll are those of a new page.
test("From one entry to another the page is new: focus on <main>, the top, and no disclosure carried over", async ({ page }) => {
  await installLibrary(page);
  const openDisclosures = () => page.locator("main details[open]").count();
  const title = page.getByTestId("entry-title");

  // What the second entry looks like when it is opened on its own.
  await page.goto("#/entry/ref/feat/mobility");
  await expect(title).toHaveText("Mobility");
  await frames(page);
  const fresh = await openDisclosures();

  await page.goto("#/entry/ref/feat/dodge");
  await expect(title).toHaveText("Dodge");
  await frames(page);
  // Open every disclosure of the first entry, focus a link in it, and scroll down.
  const closed = page.locator("main details:not([open]) > summary");
  for (let left = await closed.count(); left > 0; left -= 1) await closed.first().click();
  expect(await openDisclosures()).toBeGreaterThan(0);
  await page.locator("main article a[href]").first().focus();
  await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));

  await page.evaluate(() => {
    location.hash = "#/entry/ref/feat/mobility";
  });
  await expect(title).toHaveText("Mobility");
  await expect(page.locator("main#main")).toBeFocused();
  await frames(page);
  expect(await page.evaluate(() => window.scrollY)).toBe(0);
  expect(await openDisclosures()).toBe(fresh);
});
