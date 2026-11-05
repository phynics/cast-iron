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

  // A collapsed group hides its rows but keeps both controls one click away.
  const adventures = page.locator('.book-tier[data-tier="Adventures"]');
  if (await adventures.count()) {
    await expect(adventures.locator(".book-directory-item").first()).toBeHidden();
    await expect(adventures.getByRole("button", { name: "Deactivate all Adventures", exact: true })).toBeVisible();
    await expect(adventures.getByRole("button", { name: "Activate all Adventures", exact: true })).toBeVisible();
    await adventures.locator(".tier-summary").click();
    await expect(adventures.locator(".book-directory-item").first()).toBeVisible();
  }
});

test("Books sort inside each tier, and the URL keeps the choice", async ({ page }) => {
  await page.goto("#/books");
  const sortNav = page.getByRole("navigation", { name: "Sort books" });
  await expect(sortNav.getByRole("link", { name: "Title", exact: true })).toHaveAttribute("aria-current", "true");

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

test("Expanded books show a hierarchical contents tree and build closed branches lazily", async ({ page }) => {
  const book =
    bookTrees().find((candidate) =>
      candidate.chapters.some((chapter) =>
        (chapter.children ?? []).some((child) => (child.children ?? []).length > 0),
      ),
    ) ?? bookTrees().find((candidate) => candidate.chapters.some((chapter) => (chapter.children ?? []).length > 0));
  if (!book) throw new Error("The compiled corpus has no book with nested contents.");

  await page.goto("#/books");
  await openBookGroups(page);
  const row = page.locator(`.book-directory-item[data-book-id="${book.source}"]`);
  await row.locator(".book-directory-row > summary").click();
  const tree = row.getByTestId("book-tree");
  await expect(tree).toBeVisible();
  const root = tree.locator(".book-tree-root");
  await expect(root).toHaveText(`Read ${book.title}`);
  await expect(tree).toHaveAttribute("aria-label", new RegExp(`Contents of ${escape(book.title)}`));

  const chapterIndex = book.chapters.findIndex((chapter) => (chapter.children ?? []).length > 0);
  const chapter = book.chapters[chapterIndex];
  const chapterRow = tree.locator(".toc-top > li").nth(chapterIndex);
  const chapterSummary = chapterRow.locator(":scope > .toc-details > summary");
  await expect(chapterSummary).toContainText(chapter.title);

  // A closed chapter has no child rows in the document until it opens.
  await expect(chapterRow.locator(":scope > .toc-details > ul")).toHaveCount(0);
  await chapterSummary.click();
  await expect(chapterRow.locator(":scope > .toc-details > ul")).toBeVisible();
  await expect(chapterRow.locator(":scope > .toc-details > ul > li")).toHaveCount(
    chapter.children.length + (chapter.entry ? 1 : 0),
  );

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

  // The root is the doorway into the full contents, the same as before the tree was added.
  await root.click();
  await expect(page).toHaveURL(new RegExp(`#/browse/book/${escape(book.source)}`));
  await expect(page.getByRole("navigation", { name: new RegExp(`Contents of ${escape(book.title)}`) })).toBeVisible();
});
