import { test, expect, loadCorpus } from "./fixtures";
import { bookTrees, installLibrary } from "./helpers";

const corpus = loadCorpus();
const sources: any[] = corpus.catalog.sources;
const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

const categoryOf = (source: any) => source.category ?? "Unknown category";
const yearOf = (source: any) =>
  /^\d{4}-\d{2}-\d{2}$/.test(source.publication_date ?? "")
    ? source.publication_date.slice(0, 4)
    : "Unknown date";

const knownCategories = [
  "Adventure Path",
  "Campaign Setting",
  "Module",
  "Player Companion",
  "RPG",
  "Miscellaneous",
];

/** The product-line order the view promises, then other categories, then Unknown. */
function expectedCategories() {
  const present = new Set(sources.map(categoryOf));
  return [
    ...knownCategories.filter((category) => present.has(category)),
    ...[...present]
      .filter((category) => !knownCategories.includes(category) && category !== "Unknown category")
      .sort(),
    ...(present.has("Unknown category") ? ["Unknown category"] : []),
  ];
}

/** Newest year first, with the explicit Unknown group last. */
function expectedYears() {
  const present = new Set(sources.map(yearOf));
  return [
    ...[...present].filter((year) => year !== "Unknown date").sort().reverse(),
    ...(present.has("Unknown date") ? ["Unknown date"] : []),
  ];
}

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

test("Books sort by category and publication date, and the URL keeps the choice", async ({ page }) => {
  await page.goto("#/books");
  const rows = page.locator(".book-directory-item");
  await expect(rows).toHaveCount(sources.length);
  expect(
    await rows.evaluateAll((items) => items.map((item) => item.getAttribute("data-book-id"))),
  ).toEqual(sources.map((source) => source.id));

  const sortNav = page.getByRole("navigation", { name: "Sort books" });
  await sortNav.getByRole("link", { name: "Category", exact: true }).click();
  await expect(page).toHaveURL(/#\/books\?sort=category$/);

  const categoryGroups = page.locator(".book-group");
  await expect(categoryGroups).toHaveCount(expectedCategories().length);
  for (const [index, category] of expectedCategories().entries()) {
    const group = categoryGroups.nth(index);
    await expect(group.locator(".book-group-head")).toContainText(category);
    const ids = await group
      .locator(".book-directory-item")
      .evaluateAll((items) => items.map((item) => item.getAttribute("data-book-id")));
    expect(ids.length).toBeGreaterThan(0);
    for (const id of ids) {
      expect(categoryOf(sources.find((source) => source.id === id))).toBe(category);
    }
  }

  // Back and Forward navigate between the two sort states, not just the links.
  await page.goBack();
  await expect(page).toHaveURL(/#\/books$/);
  await expect(page.locator(".book-group")).toHaveCount(0);
  await page.goForward();
  await expect(page).toHaveURL(/#\/books\?sort=category$/);

  await sortNav.getByRole("link", { name: "Publication date", exact: true }).click();
  await expect(page).toHaveURL(/#\/books\?sort=date$/);
  const dateGroups = page.locator(".book-group");
  await expect(dateGroups).toHaveCount(expectedYears().length);
  for (const [index, year] of expectedYears().entries()) {
    const group = dateGroups.nth(index);
    await expect(group.locator(".book-group-head")).toContainText(year);
    const ids = await group
      .locator(".book-directory-item")
      .evaluateAll((items) => items.map((item) => item.getAttribute("data-book-id")));
    expect(ids.length).toBeGreaterThan(0);
    for (const id of ids) expect(yearOf(sources.find((source) => source.id === id))).toBe(year);
    if (year !== "Unknown date") {
      const dates = ids.map(
        (id) => sources.find((source) => source.id === id)?.publication_date as string,
      );
      expect(dates.every((date) => date.startsWith(year))).toBe(true);
      expect([...dates].sort().reverse()).toEqual(dates);
    }
  }

  await sortNav.getByRole("link", { name: "Library order", exact: true }).click();
  await expect(page).toHaveURL(/#\/books$/);
  await expect(page.locator(".book-group")).toHaveCount(0);
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
