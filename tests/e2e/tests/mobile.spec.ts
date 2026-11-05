import { test, expect } from "./fixtures";
import { installLibrary, waitResults } from "./helpers";

// Runs in the mobile-chromium project (390 px wide). Layout rules live in docs/decisions.md.
const screens: [string, string][] = [
  ["#/", "#home-title"],
  ["#/search?q=fire", ".results .hit"],
  ["#/browse", "a.cat"],
  ["#/browse/kind/spell", ".entry-list li"],
  ["#/books", ".book-directory-row"],
  ["#/browse/source/bestiary", ".toc"],
  ["#/spells/Wizard/3", ".level-group"],
  ["#/feats/chains", ".chain-head"],
  ["#/entry/core-rulebook/spells/fireball", ".stat-block"],
  ["#/entry/core-rulebook/rules/equipment/weapons", ".rules-table"],
  ["#/entry/core-rulebook/classes/cleric", ".rich h2"],
  ["#/entry/bestiary/monsters/aasimar", ".stat-block"],
  ["#/bookmarks", "#home-title"],
  ["#/settings", "[data-testid=library-facts]"],
  ["#/legal", ".legal-text"],
];

test("Mobile viewport shows no horizontal page scroll on key screens", async ({ page }) => {
  expect(page.viewportSize()?.width).toBe(390);
  await installLibrary(page);
  const wide: string[] = [];
  for (const [hash, ready] of screens) {
    await page.goto(hash);
    if (hash.startsWith("#/search")) await waitResults(page, "fire");
    await expect(page.locator(ready).first(), hash).toBeVisible();
    const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
    if (overflow > 0) wide.push(`${hash} overflows by ${overflow}px`);
  }
  expect(wide).toEqual([]);
});

test("Mobile header Install button is icon-only, a full touch target, and crowds nothing", async ({ page, context }) => {
  await context.addInitScript({
    content: `document.addEventListener("DOMContentLoaded", () => {
      const event = new Event("beforeinstallprompt", { cancelable: true });
      event.prompt = async () => {};
      event.userChoice = Promise.resolve({ outcome: "dismissed" });
      window.dispatchEvent(event);
    });`,
  });
  await installLibrary(page);
  const button = page.locator("header.app-header").getByRole("button", { name: "Install Cast Iron as an app" });
  await expect(button).toBeVisible();
  await expect(button.locator(".header-action-label")).toBeHidden();
  const box = (await button.boundingBox())!;
  expect(box.width).toBeGreaterThanOrEqual(44);
  expect(box.height).toBeGreaterThanOrEqual(44);
  const search = (await page.locator("header.app-header .header-search").boundingBox())!;
  expect(search.x + search.width).toBeLessThanOrEqual(box.x);
  const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});
