import { test, expect } from "./fixtures";
import { cacheContents, hitTitles, installLibrary, waitControlled, waitReady, waitResults } from "./helpers";

test("Offline launch works after a new browser session", async ({ openProfile, server }) => {
  let context = await openProfile();
  let page = await context.newPage();
  await installLibrary(page);
  await waitControlled(page);
  await page.evaluate(() => navigator.serviceWorker.ready);
  expect(Object.keys(await cacheContents(page)).some((n) => n.startsWith("cast-iron-shell-"))).toBe(true);
  await context.close();

  // A new session with no network from its first navigation.
  await server.clearLog();
  context = await openProfile({ offline: true });
  page = await context.newPage();
  await page.goto("./");
  await waitReady(page);
  await expect(page.getByRole("heading", { name: /Find any rule/i })).toBeVisible();
  await page.goto("#/search");
  await page.locator("#search-input").fill("fireball");
  await waitResults(page, "fireball");
  expect((await hitTitles(page))[0]).toBe("Fireball");
  await page.locator(".hit-link", { hasText: /^Fireball/ }).first().click();
  await expect(page.getByRole("heading", { level: 1, name: "Fireball" })).toBeVisible();
  // Chromium's offline mode does not cover the browser's own service worker script check; the
  // server refuses it. Nothing else may be requested.
  expect((await server.requests()).filter((p) => !p.startsWith("sw.js"))).toEqual([]);
});

test("No application feature requires a runtime server", async ({ page, context, server }) => {
  await installLibrary(page);
  await waitControlled(page);
  await page.evaluate(() => navigator.serviceWorker.ready);
  await page.reload();
  await waitReady(page);

  // Online, but every request that reaches the server from here on is a failure of the product.
  await server.clearLog();
  await page.goto("#/search?q=fireball");
  await waitResults(page, "fireball");
  await page.locator(".hit-link", { hasText: /^Fireball/ }).first().click();
  await expect(page.getByRole("heading", { level: 1, name: "Fireball" })).toBeVisible();
  await page.getByRole("button", { name: /Bookmark/ }).click();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  for (const hash of [
    "#/browse",
    "#/browse/kind/spell",
    "#/browse/kind/rule",
    "#/browse/source/core-rulebook",
    "#/entry/core-rulebook/classes/cleric",
    "#/entry/core-rulebook/rules/equipment/weapons",
    "#/bookmarks",
    "#/legal",
  ]) {
    await page.goto(hash);
    await expect(page.locator("main h1").first()).toBeVisible();
  }
  await page.goto("#/settings");
  await page.getByRole("button", { name: "Verify library" }).click();
  await expect(page.getByTestId("verify-status")).toContainText("verified");
  // A new tab and a full reload are served from the service worker too.
  const other = await context.newPage();
  await other.goto("#/entry/core-rulebook/spells/fireball");
  await expect(other.getByRole("heading", { level: 1, name: "Fireball" })).toBeVisible();
  await page.reload();
  await waitReady(page);

  // Only the browser's own check for a newer service worker script is allowed.
  const requests = (await server.requests()).filter((p) => p !== "sw.js");
  expect(requests).toEqual([]);
});
