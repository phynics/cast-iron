import { test, expect } from "./fixtures";
import { installLibrary, waitControlled, waitReady } from "./helpers";

test("Bookmarks persist across restarts", async ({ openProfile }) => {
  let context = await openProfile();
  let page = await context.newPage();
  await installLibrary(page);
  await waitControlled(page);
  await page.evaluate(() => navigator.serviceWorker.ready);

  await page.goto("#/entry/core-rulebook/spells/fireball");
  await expect(page.getByRole("heading", { level: 1, name: "Fireball" })).toBeVisible();
  await page.locator(".bookmark-toggle").click();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  await page.locator(".picker summary").click();
  await page.locator("#new-collection-entry").fill("Blasters");
  await page.getByRole("button", { name: "Create and add" }).click();
  await expect(page.locator(".picker .check", { hasText: "Blasters" }).locator("input")).toBeChecked();

  await page.goto("#/entry/core-rulebook/spells/magic-missile");
  await page.locator(".bookmark-toggle").click();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  await context.close();

  // A new session, offline from the first navigation.
  context = await openProfile({ offline: true });
  page = await context.newPage();
  await page.goto("#/bookmarks");
  await waitReady(page);
  const list = page.locator(".bm-list");
  await expect(list.locator(".bm")).toHaveCount(2);
  await expect(list).toContainText("Fireball");
  await expect(list).toContainText("Magic Missile");
  await expect(page.locator(".collection-list")).toContainText("Blasters");
  await page.locator("a.collection-link", { hasText: "Blasters" }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Blasters" })).toBeVisible();
  await expect(page.locator(".bm")).toHaveCount(1);
  await expect(page.locator(".bm")).toContainText("Fireball");
  // The entry page agrees.
  await page.goto("#/entry/core-rulebook/spells/fireball");
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
});
