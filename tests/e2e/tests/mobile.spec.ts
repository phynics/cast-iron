import { test, expect, loadCorpus } from "./fixtures";
import { bookTrees, firstEntry, hasBlock, headingAnchors, installLibrary, waitResults } from "./helpers";

// Runs in the mobile-chromium project (390 px wide). Layout rules live in docs/decisions.md.
// Every screen names a target the compiled corpus actually holds; the two libraries differ
// in ids, sources, and counts, so none of these may be a literal entry id.
const corpus = loadCorpus();
const entries = [...corpus.entries.values()];
const statSpell = firstEntry("a spell with a stat block", (entry) => entry.kind === "spell" && hasBlock(entry, "stat_block"));
const tableEntry = firstEntry("an entry with a table", (entry) => hasBlock(entry, "table"));
const headingEntry = firstEntry("an entry with a heading", (entry) => headingAnchors(entry).length > 0);
const creature = firstEntry("a creature with a stat block", (entry) => entry.kind === "creature" && hasBlock(entry, "stat_block"));
const source = bookTrees()[0].source;
const classLevels = new Map<string, number>();
for (const entry of entries) {
  for (const level of entry.fields?.levels ?? []) classLevels.set(level.class, (classLevels.get(level.class) ?? 0) + 1);
}
const [listClass, listCount] = [...classLevels.entries()].sort((a, b) => b[1] - a[1])[0];
const listSpell = entries.find((entry) => entry.kind === "spell" &&
  entry.fields?.levels?.some((level: any) => level.class === listClass))!;
const listLevel = listSpell.fields.levels.find((level: any) => level.class === listClass).level;

const screens: [string, string][] = [
  ["#/", "#home-title"],
  ["#/search?q=fire", ".results .hit"],
  ["#/browse", "a.cat"],
  ["#/browse/kind/spell", ".entry-list li"],
  ["#/books", ".book-directory-row"],
  [`#/browse/source/${encodeURIComponent(source)}`, ".toc"],
  [`#/spells/${encodeURIComponent(listClass)}/${listLevel}`, ".level-group"],
  ["#/feats/chains", ".chain-head"],
  [`#/entry/${statSpell.id}`, ".stat-block"],
  [`#/entry/${tableEntry.id}`, ".rules-table"],
  [`#/entry/${headingEntry.id}`, ".rich h2"],
  [`#/entry/${creature.id}`, ".stat-block"],
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
