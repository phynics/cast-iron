import { test, expect } from "./fixtures";
import { installLibrary, waitResults } from "./helpers";

// Runs in the mobile-chromium project (390 px wide). Layout rules live in docs/decisions.md.
const screens: [string, string][] = [
  ["#/", "#home-title"],
  ["#/search?q=fire", ".results .hit"],
  ["#/browse", "a.tile"],
  ["#/browse/kind/spell", ".entry-list li"],
  ["#/browse/kind/rule", ".tree"],
  ["#/browse/source/bestiary", ".kind-group"],
  ["#/entry/core-rulebook/spells/fireball", ".stat-block"],
  ["#/entry/core-rulebook/rules/equipment/weapons", ".rules-table"],
  ["#/entry/core-rulebook/classes/cleric", ".rich h2"],
  ["#/entry/bestiary/monsters/aasimar", ".stat-block"],
  ["#/bookmarks", ".empty"],
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
