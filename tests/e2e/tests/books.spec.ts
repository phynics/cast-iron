import { test, expect, loadCorpus } from "./fixtures";
import { bookTrees, installLibrary, openBookGroups } from "./helpers";

const corpus = loadCorpus();
const sources: any[] = corpus.catalog.sources;
const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** The tier table, restated independently of the Rust code: this is what the page promises. */
const tiers: { label: string; categories: string[]; open: boolean }[] = [
  { label: "Rulebooks", categories: ["RPG"], open: true },
  { label: "Player Companions", categories: ["Player Companion"], open: true },
  { label: "Campaign Setting", categories: ["Campaign Setting"], open: true },
  { label: "Adventures", categories: ["Adventure Path", "Module", "Society"], open: false },
  { label: "Other", categories: [], open: true },
];
const tierOf = (source: any) =>
  tiers.find((tier) => tier.categories.includes(source.category))?.label ?? "Other";
const inTier = (label: string) => sources.filter((source) => tierOf(source) === label);

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

test("Books group by priority tier: Rulebooks first, Adventures collapsed with a count", async ({ page }) => {
  await page.goto("#/books");
  const present = tiers.filter((tier) => inTier(tier.label).length > 0);
  const groups = page.locator(".book-tier");
  await expect(groups).toHaveCount(present.length);
  await expect(page.locator(".book-directory-item")).toHaveCount(sources.length);
  for (const [index, tier] of present.entries()) {
    const group = groups.nth(index);
    const members = inTier(tier.label);
    await expect(group.getByRole("heading", { level: 2, name: `${tier.label} (${members.length})`, exact: true })).toBeVisible();
    const ids = await group
      .locator(".book-directory-item")
      .evaluateAll((items) => items.map((item) => item.getAttribute("data-book-id")));
    expect([...ids].sort()).toEqual(members.map((source) => source.id).sort());
    // Every book starts active, in every tier.
    await expect(group.locator(".tier-active")).toHaveText(`${members.length} of ${members.length} active`);
    expect(await group.locator("details.tier-details").evaluate((el) => (el as HTMLDetailsElement).open)).toBe(tier.open);
  }
  expect(present[0].label).toBe("Rulebooks");

  // Nothing interactive sits inside a summary, and there are no group controls at all: the
  // presets on Active books are the only bulk action.
  await expect(page.locator(".tier-summary button, .tier-summary a, .tier-summary input")).toHaveCount(0);
  await expect(page.locator(".tier-summary > h2")).toHaveCount(present.length);
  await expect(page.getByRole("button", { name: /Activate all|Deactivate all|Only Rulebooks/ })).toHaveCount(0);
  await expect(page.locator(".tier-actions, .books-preset")).toHaveCount(0);
  expect((await page.locator('.book-tier[data-tier="Rulebooks"] .tier-summary').boundingBox())!.height).toBeGreaterThanOrEqual(44);

  // A collapsed group hides its rows until it opens.
  const adventures = page.locator('.book-tier[data-tier="Adventures"]');
  if (await adventures.count()) {
    await expect(adventures.locator(".book-directory-item").first()).toBeHidden();
    await adventures.locator(".tier-summary").click();
    await expect(adventures.locator(".book-directory-item").first()).toBeVisible();
  }
});

test("Books sort inside each tier, and the URL keeps the choice", async ({ page }) => {
  await page.goto("#/books");
  const sortNav = page.getByRole("navigation", { name: "Sort books" });
  await expect(sortNav.getByRole("link", { name: "Title", exact: true })).toHaveAttribute("aria-current", "true");

  // Sort options are navigation: a 6 px rectangle, never a pill.
  const sortLinks = page.locator(".sort-options a");
  await expect(page.locator('.sort-options a[aria-current="true"]')).toHaveCount(1);
  for (const link of await sortLinks.all()) {
    const radius = await link.evaluate((el) => getComputedStyle(el).borderTopLeftRadius);
    expect(radius).not.toBe("999px");
    expect(parseFloat(radius)).toBeLessThan(10);
    const box = await link.boundingBox();
    expect(box?.height ?? 0).toBeGreaterThanOrEqual(44);
  }

  await sortNav.getByRole("link", { name: "Category", exact: true }).click();
  await expect(page).toHaveURL(/#\/books\?sort=category$/);
  const adventureRanks = ["Adventure Path", "Module", "Society"];
  const adventures = page.locator('.book-tier[data-tier="Adventures"]');
  const ids: string[] = await adventures
    .locator(".book-directory-item")
    .evaluateAll((items) => items.map((item) => item.getAttribute("data-book-id") as string));
  const ranks = ids.map((id) => adventureRanks.indexOf(sources.find((source) => source.id === id)?.category));
  expect(ranks).toEqual([...ranks].sort((a, b) => a - b));

  // Back and Forward restore the sort state.
  await page.goBack();
  await expect(page).toHaveURL(/#\/books$/);
  await page.goForward();
  await expect(page).toHaveURL(/#\/books\?sort=category$/);

  await sortNav.getByRole("link", { name: "Publication date", exact: true }).click();
  await expect(page).toHaveURL(/#\/books\?sort=date$/);
  for (const tier of tiers) {
    const group = page.locator(`.book-tier[data-tier="${tier.label}"]`);
    if (!(await group.count())) continue;
    const dates: string[] = (
      await group
        .locator(".book-directory-item")
        .evaluateAll((items) => items.map((item) => item.getAttribute("data-book-id") as string))
    )
      .map((id) => sources.find((source) => source.id === id)?.publication_date)
      .filter((date): date is string => /^\d{4}-\d{2}-\d{2}$/.test(date ?? ""));
    expect([...dates].sort().reverse(), tier.label).toEqual(dates);
  }

  await sortNav.getByRole("link", { name: "Library order", exact: true }).click();
  await expect(page).toHaveURL(/#\/books\?sort=library$/);
  await expect(page.locator(".book-tier")).toHaveCount(0);
  expect(
    await page
      .locator(".book-directory-item")
      .evaluateAll((items) => items.map((item) => item.getAttribute("data-book-id"))),
  ).toEqual(sources.map((source) => source.id));

  await sortNav.getByRole("link", { name: "Title", exact: true }).click();
  await expect(page).toHaveURL(/#\/books$/);
  await expect(page.locator(".book-tier").first()).toBeVisible();
});

test("A book row is one link to its book page, with no checkbox, no inline tree and no Read link", async ({ page }) => {
  const book = bookTrees().find((candidate) => candidate.chapters.length > 0)!;
  await page.goto("#/books");
  await openBookGroups(page);
  const item = page.locator(`.book-directory-item[data-book-id="${book.source}"]`);
  const link = item.locator("a.book-directory-row");
  await expect(link).toHaveAttribute("href", `#/browse/book/${book.source}`);
  await expect(link).toContainText(book.title);

  // Nothing on the list expands, and nothing is called "Read ...".
  await expect(page.locator(".book-directory-item details")).toHaveCount(0);
  await expect(page.locator(".book-tree, [data-testid=book-tree], .book-tree-root")).toHaveCount(0);
  await expect(page.locator(".book-directory-item").getByRole("link", { name: /^Read / })).toHaveCount(0);

  // A row is a link and nothing else: no checkbox, no switch, one 44 px target.
  await expect(item.locator("input")).toHaveCount(0);
  await expect(page.locator(".book-directory-item input")).toHaveCount(0);
  const linkBox = (await link.boundingBox())!;
  expect(linkBox.height).toBeGreaterThanOrEqual(44);

  await link.click();
  await expect(page).toHaveURL(new RegExp(`#/browse/book/${escape(book.source)}`));
  await expect(page.getByRole("heading", { level: 1, name: book.title, exact: true })).toBeVisible();
  await expect(page.getByRole("navigation", { name: new RegExp(`Contents of ${escape(book.title)}`) })).toBeVisible();
});

test("Back from a book page returns to Books at the same scroll position", async ({ page }) => {
  await page.goto("#/books");
  await expect(page.locator(".book-tier").first()).toBeVisible();
  // Use the groups as they open by default: a collapsed group is not part of the saved position.
  const items = page.locator("details.tier-details[open] .book-directory-item");
  const count = await items.count();
  expect(count).toBeGreaterThan(8);
  const target = items.nth(count - 1);
  const id = (await target.getAttribute("data-book-id"))!;
  await target.scrollIntoViewIfNeeded();
  await page.evaluate(() => window.scrollBy(0, 40));
  const before = await page.evaluate(() => Math.round(window.scrollY));
  expect(before).toBeGreaterThan(200);
  await target.locator("a.book-directory-row").click();
  await expect(page).toHaveURL(new RegExp(`#/browse/book/${escape(id)}`));
  await page.goBack();
  await expect(page).toHaveURL(/#\/books$/);
  await expect(page.locator(".book-tier").first()).toBeVisible();
  await expect
    .poll(async () => Math.abs((await page.evaluate(() => Math.round(window.scrollY))) - before), { timeout: 5000 })
    .toBeLessThanOrEqual(2);
});

test("A book page shows a hierarchical contents tree and builds closed branches lazily", async ({ page }) => {
  const book =
    bookTrees().find((candidate) =>
      candidate.chapters.some((chapter) =>
        (chapter.children ?? []).some((child) => (child.children ?? []).length > 0),
      ),
    ) ?? bookTrees().find((candidate) => candidate.chapters.some((chapter) => (chapter.children ?? []).length > 0));
  if (!book) throw new Error("The compiled corpus has no book with nested contents.");

  await page.goto(`#/browse/book/${book.source}`);
  const tree = page.getByRole("navigation", { name: new RegExp(`Contents of ${escape(book.title)}`) });
  await expect(tree).toBeVisible();

  const chapterIndex = book.chapters.findIndex((chapter) => (chapter.children ?? []).length > 0);
  const chapter = book.chapters[chapterIndex];
  const chapterRow = tree.locator(".toc-top > li").nth(chapterIndex);
  const chapterSummary = chapterRow.locator(":scope > .toc-details > summary");
  await expect(chapterSummary).toContainText(chapter.title);

  // A closed chapter has no child rows in the document until it opens.
  await expect(chapterRow.locator(":scope > .toc-details > ul")).toHaveCount(0);
  await chapterSummary.click();
  await expect(chapterRow.locator(":scope > .toc-details > ul")).toBeVisible();

  // The disclosure is keyboard operable and reports its state through the native element.
  const details = chapterSummary.locator("xpath=..");
  const openBefore = await details.evaluate((element) => (element as HTMLDetailsElement).open);
  await chapterSummary.focus();
  await chapterSummary.press("Enter");
  await expect
    .poll(() => details.evaluate((element) => (element as HTMLDetailsElement).open))
    .toBe(!openBefore);
  await chapterSummary.press("Enter");
  await expect
    .poll(() => details.evaluate((element) => (element as HTMLDetailsElement).open))
    .toBe(openBefore);

  // A section within the chapter opens the same way and reveals its own children.
  const branch = chapterRow.locator(":scope > .toc-details > ul > li.toc-branch").first();
  if (await branch.count()) {
    const branchChildren = branch.locator(":scope > .toc-details > ul");
    await expect(branchChildren).toHaveCount(0);
    await branch.locator(":scope > .toc-details > summary").click();
    await expect(branchChildren).toBeVisible();
  }
});

test("A chapter toggle that Back overtakes does not rewrite the Books entry", async ({ page }) => {
  const book = bookTrees().find((candidate) => candidate.chapters.some((chapter) => (chapter.children ?? []).length > 0))!;
  await page.goto("#/books");
  await openBookGroups(page);
  // The race is a matter of task order, so repeat it.
  for (let round = 0; round < 6; round++) {
    await page.locator(`.book-directory-item[data-book-id="${book.source}"] a.book-directory-row`).click();
    const summary = page.locator(".toc-top > li > .toc-details > summary").first();
    await expect(summary).toBeVisible();
    // The browser fires `toggle` in a task after the click. Ask for Back in the same task, so the
    // traversal can land before the event does: the late handler must not write the book's URL
    // over the Books entry.
    await summary.evaluate((element) => {
      element.click();
      history.back();
    });
    await expect(page.locator(".book-tier").first()).toBeVisible();
    // Let any deferred toggle event run, then check that Books is still what the entry shows.
    await page.evaluate(() => new Promise<void>((resolve) => setTimeout(resolve, 300)));
    await expect(page).toHaveURL(/#\/books$/);
    await expect(page.locator(".book-tier").first()).toBeVisible();
  }
});
