import { test, expect, goOffline } from "./fixtures";
import { installLibrary, waitControlled, openBookContents } from "./helpers";


test("Keyboard reading traverses a complete Armor subsystem offline and preserves book history", async ({ page, context, server }) => {
  await installLibrary(page); await waitControlled(page); await goOffline(context, server);
  await openBookContents(page, "Core Rulebook");
  // Follow the visible book tree, rather than inventing a category rules route.
  const combat = page.locator(".toc details").filter({ has: page.locator(":scope > summary .toc-title", { hasText: /^Equipment$/ }) }).first();
  await combat.locator(":scope > summary").press("Enter");
  const armor = combat.locator(".toc-details").filter({ has: page.locator(":scope > summary .toc-title", { hasText: /^Armor$/ }) }).first();
  await armor.locator(":scope > summary").press("Enter");
  await armor.getByRole("link", { name: "Read Armor", exact: true }).press("Enter");
  await expect(page.locator("h1")).toHaveText("Armor");
  const start = page.url();
  const contents = page.locator(".article-contents");
  await contents.locator("summary").press("Enter");
  const links = await contents.locator("a").evaluateAll(els => els.map(a => ({ href: a.getAttribute("href")!, text: a.textContent! })));
  expect(links.length).toBeGreaterThan(10);
  // Every contents stop must reach a uniquely rendered section of this subsystem.
  for (const link of links) {
    const disclosure = page.locator(".article-contents");
    if (!(await disclosure.evaluate((e: HTMLDetailsElement) => e.open))) await disclosure.locator("summary").press("Enter");
    await disclosure.locator(`a[href="${link.href}"]`).press("Enter");
    const anchor = new URLSearchParams(link.href.split("?")[1]).get("a")!;
    await expect.poll(() => page.evaluate(id => {
      const el = document.getElementById(`h-${id}`);
      if (!el) return false;
      const top = el.getBoundingClientRect().top;
      return top >= 0 && top < innerHeight;
    }, anchor)).toBe(true);
  }
  await page.keyboard.press("ControlOrMeta+End");
  const next = page.locator('.book-steps a[rel="next"]');
  await expect(next).toBeVisible();
  await next.press("Enter");
  await expect(page.locator("h1")).not.toHaveText("Armor");
  await page.locator('.book-steps a[rel="prev"]').press("Enter");
  await expect(page.locator("h1")).toHaveText("Armor");
  // Follow an actual typed cross-reference if this subsystem supplies one.
  const cross = page.locator('.entry > .rich a[href^="#/entry/"]').first();
  await expect(cross).toBeAttached();
  await cross.press("Enter");
  const other = page.url();
  expect(other).not.toBe(start);
  await page.goBack(); await expect(page.locator("h1")).toHaveText("Armor");
  await page.goForward(); await expect.poll(() => page.url()).toBe(other);
});

test("Keyboard controls expose focus and native disclosure state at phone and desktop widths", async ({ page, browserName }) => {
  await installLibrary(page);
  // WebKit on macOS uses Option-Tab to include buttons in sequential navigation.
  const tabKey = browserName === "webkit" ? "Alt+Tab" : "Tab";
  for (const width of [390, 1280]) {
    await page.setViewportSize({ width, height: 844 });
    await page.goto("#/");
    // Route changes focus the reading area on the next frame. Test controls after that settles.
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve(null)))));
    await page.getByRole("searchbox", { name: "Search the rules", exact: true }).focus();
    await page.keyboard.press(tabKey);
    const focused = page.locator(":focus");
    await expect(focused).toHaveCount(1);
    expect(await focused.evaluate(el => {
      const s = getComputedStyle(el);
      return s.outlineStyle !== "none" && parseFloat(s.outlineWidth) >= 2;
    })).toBe(true);
    await expect(page.getByRole("navigation", { name: "Primary" }).getByRole("link", { name: "Home", exact: true })).toHaveAttribute("aria-current", "page");
    await page.goto("#/browse/kind/spell");
    const jump = page.getByRole("button", { name: "Jump to F", exact: true });
    await jump.focus(); await page.keyboard.press("Enter");
    await expect.poll(() => page.locator("#letter-F").evaluate(el => el.getBoundingClientRect().top)).toBeGreaterThanOrEqual(0);
    const rect = await jump.boundingBox();
    expect(rect!.width).toBeGreaterThanOrEqual(44); expect(rect!.height).toBeGreaterThanOrEqual(44);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  }
});
