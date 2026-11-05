import { test, installedTest, expect, loadCorpus } from "./fixtures";
import { bookTrees, firstEntry, hasBlock, headingAnchors, installLibrary, openBookGroups, waitResults } from "./helpers";

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

installedTest("Mobile viewport shows no horizontal page scroll on key screens", async ({ page }) => {
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

test("Expanded books, their contents tree, and the sort control fit the mobile viewport", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/books");
  await openBookGroups(page);
  const book = bookTrees().find((candidate) => candidate.chapters.some((chapter) => (chapter.children ?? []).length > 0));
  if (!book) throw new Error("The compiled corpus has no book with contents.");
  const row = page.locator(`.book-directory-item[data-book-id="${book.source}"]`);
  await row.locator(".book-directory-row > summary").click();
  const tree = row.getByTestId("book-tree");
  await expect(tree).toBeVisible();
  await tree.locator(".toc-top > li > .toc-details > summary").first().click();
  const treeOverflow = await page.evaluate(
    () => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth,
  );
  expect(treeOverflow).toBeLessThanOrEqual(0);

  await page.getByRole("navigation", { name: "Sort books" }).getByRole("link", { name: "Publication date", exact: true }).click();
  await expect(page.locator(".book-tier").first()).toBeVisible();
  const sortOverflow = await page.evaluate(
    () => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth,
  );
  expect(sortOverflow).toBeLessThanOrEqual(0);
});

test("Books tiers and their bulk controls fit 390 px with full-size touch targets", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/books");
  const groups = page.locator(".book-tier");
  await expect(groups.first()).toBeVisible();
  await expect(groups.first().getByRole("heading", { level: 2 })).toContainText("Rulebooks");
  const adventures = page.locator('.book-tier[data-tier="Adventures"]');
  await expect(adventures.locator("details.tier-details")).not.toHaveAttribute("open", "");

  for (const label of ["Rulebooks", "Adventures"]) {
    const group = page.locator(`.book-tier[data-tier="${label}"]`);
    const heading = group.locator(".tier-summary").boundingBox();
    const buttons = [
      group.getByRole("button", { name: `Activate all ${label}`, exact: true }),
      group.getByRole("button", { name: `Deactivate all ${label}`, exact: true }),
    ];
    for (const button of buttons) {
      await expect(button).toBeVisible();
      const box = (await button.boundingBox())!;
      expect(box.height, `${label} control height`).toBeGreaterThanOrEqual(44);
      expect(box.width, `${label} control width`).toBeGreaterThanOrEqual(44);
      expect(box.x + box.width).toBeLessThanOrEqual(390);
    }
    // The controls sit below the heading row and never cover the disclosure's own target.
    const summary = (await heading)!;
    const first = (await buttons[0].boundingBox())!;
    expect(first.y).toBeGreaterThanOrEqual(summary.y + 44 - 1);
  }
  const preset = page.getByRole("button", { name: "Only Rulebooks", exact: true });
  expect((await preset.boundingBox())!.height).toBeGreaterThanOrEqual(44);

  const overflow = async () =>
    page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(await overflow()).toBeLessThanOrEqual(0);
  await adventures.locator(".tier-summary").click();
  await expect(adventures.locator(".book-directory-item").first()).toBeVisible();
  expect(await overflow()).toBeLessThanOrEqual(0);

  // One tap deactivates the whole group on a phone as well.
  await adventures.getByRole("button", { name: "Deactivate all Adventures", exact: true }).click();
  await expect(adventures.locator(".tier-active")).toContainText(/^0 of \d+ active$/);
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
  // Device pixel ratios round a 44px target slightly below 44 in the layout viewport.
  expect(Math.round(box.width)).toBeGreaterThanOrEqual(44);
  expect(Math.round(box.height)).toBeGreaterThanOrEqual(44);
  const search = (await page.locator("header.app-header .header-search").boundingBox())!;
  expect(search.x + search.width).toBeLessThanOrEqual(box.x);
  const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});

installedTest("Mobile entry header is compact: one metadata line and an icon star", async ({ page }) => {
  await installLibrary(page);
  await page.goto(`#/entry/${statSpell.id}`);
  await expect(page.locator(".stat-block").first()).toBeVisible();
  const top = await page.locator(".stat-block").first().evaluate((el) => el.getBoundingClientRect().top);
  // Title, a short metadata line and the stat block fit in the first 300 px of an 844 px screen.
  // (The old header, with pills and a full-row Star button, put the block near 380 px.) The bound
  // leaves room for a two-line title and a wrapped metadata line.
  expect(top).toBeLessThan(300);
  const star = (await page.locator(".bookmark-toggle").boundingBox())!;
  expect(Math.round(star.width)).toBe(44);
  expect(Math.round(star.height)).toBe(44);
  // A long book title may wrap once; wrapped pills used to take three rows.
  const meta = page.locator(".entry-meta");
  const lineHeight = await meta.evaluate((el) => parseFloat(getComputedStyle(el).lineHeight));
  expect((await meta.boundingBox())!.height).toBeLessThan(lineHeight * 2.6);
});
