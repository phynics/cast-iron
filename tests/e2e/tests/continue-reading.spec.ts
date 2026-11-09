import { installedTest as test, expect, loadCorpus } from "./fixtures";
import {
  bookReadingOrder,
  bookTrees,
  installLibrary,
  openBookEntry,
  openQuickSearch,
  quickGroup,
  searchFor,
  storedMeta,
  waitHomeIdle,
  waitReady,
} from "./helpers";

const corpus = loadCorpus();

/** A book with an entry Contents shows, and that entry. */
function placedEntry() {
  for (const book of bookTrees()) {
    const order = bookReadingOrder(book);
    if (order.length === 0) continue;
    const entry = corpus.entries.get(order[0]);
    if (entry) return { book, entry };
  }
  throw new Error("The compiled corpus has no book entry to continue from.");
}

async function storedContinuePoints(page: import("@playwright/test").Page) {
  return ((await storedMeta(page, "continue_points")) ?? []) as {
    source: string;
    entry_id: string;
    read_at: number;
  }[];
}

async function expectStoredPoint(page: import("@playwright/test").Page, source: string, entryId: string) {
  await expect
    .poll(async () => (await storedContinuePoints(page)).find((point) => point.source === source)?.entry_id)
    .toBe(entryId);
}

/** The Continue row of Quick search's Recently viewed group. */
function continueRow(page: import("@playwright/test").Page) {
  return quickGroup(page, "Recently viewed").filter({ has: page.locator(".quick-search-name", { hasText: /^Continue / }) });
}

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

test("An entry opened from a book sets its continue point on the book page and in Quick search, and both survive a reload", async ({ page }) => {
  const { book, entry } = placedEntry();
  await openBookEntry(page, book, entry.id);
  await expect(page.getByRole("heading", { level: 1, name: entry.title, exact: true })).toBeVisible();
  // The visit arrived from the chapter page, so the entry URL carries its book.
  await expect(page).toHaveURL(/[?&]book=/);
  await expectStoredPoint(page, book.source, entry.id);

  await page.goto(`#/browse/book/${book.source}`);
  const bookLink = page.getByTestId("continue-reading");
  await expect(bookLink).toBeVisible();
  await expect(bookLink).toContainText(`Continue reading: ${entry.title}`);
  await expect(bookLink.getByRole("link")).toHaveAttribute(
    "href",
    new RegExp(`#/entry/${entry.id.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\?.*book=${book.source}`),
  );

  // Home no longer shows reading history at all.
  await page.goto("#/");
  await waitReady(page);
  await waitHomeIdle(page);
  await expect(page.getByTestId("home-continue")).toHaveCount(0);
  await expect(page.locator(".home-recent")).toHaveCount(0);

  // Quick search shows it as the first row of Recently viewed.
  await openQuickSearch(page);
  const row = continueRow(page);
  await expect(row).toHaveCount(1);
  await expect(row.first()).toContainText(`Continue ${book.title}`);
  await expect(row.first().locator(".quick-search-meta")).toHaveText(entry.title);
  await expect(quickGroup(page, "Recently viewed").first()).toContainText(`Continue ${book.title}`);

  // Opening it keeps the book context, which is what makes the point move forward.
  await row.first().click();
  await expect(page).toHaveURL(new RegExp(`#/entry/.*[?&]book=${book.source}`));
  await expect(page.getByRole("heading", { level: 1, name: entry.title, exact: true })).toBeVisible();

  // The stored point, not a page signal, is what a reload reads.
  await page.goto("#/");
  await waitHomeIdle(page);
  await page.reload();
  await waitReady(page);
  await waitHomeIdle(page);
  await openQuickSearch(page);
  await expect(continueRow(page)).toHaveCount(1);
  await expect(continueRow(page).first()).toContainText(`Continue ${book.title}`);
  await page.keyboard.press("Escape");

  await page.goto(`#/browse/book/${book.source}`);
  await expect(page.getByTestId("continue-reading")).toContainText(`Continue reading: ${entry.title}`);
});

test("An entry opened from search leaves the continue point alone", async ({ page }) => {
  const { book, entry } = placedEntry();
  await searchFor(page, entry.title);
  const result = page.locator(`.results a[href="#/entry/${entry.id}"]`).first();
  await expect(result).toBeVisible();
  await result.click();
  await expect(page.getByRole("heading", { level: 1, name: entry.title, exact: true })).toBeVisible();
  // A search hit has no book context in its URL.
  await expect(page).not.toHaveURL(/[?&]book=/);

  await expect.poll(async () => (await storedContinuePoints(page)).length).toBe(0);
  await page.goto("#/");
  await waitReady(page);
  await waitHomeIdle(page);
  await openQuickSearch(page);
  // The visit is a recently viewed row, but no Continue row exists for it.
  await expect(quickGroup(page, "Recently viewed")).toHaveCount(1);
  await expect(continueRow(page)).toHaveCount(0);
  await page.keyboard.press("Escape");
});

test("Clear in Recently viewed removes the visit and the continue point, and both stay gone after a reload", async ({ page }) => {
  const { book, entry } = placedEntry();
  await openBookEntry(page, book, entry.id);
  await expect(page.getByRole("heading", { level: 1, name: entry.title, exact: true })).toBeVisible();
  await expectStoredPoint(page, book.source, entry.id);

  await page.goto("#/");
  await waitReady(page);
  await waitHomeIdle(page);
  const layer = await openQuickSearch(page);
  await expect(continueRow(page)).toHaveCount(1);
  // Each group clears itself: the Clear next to the Recently viewed heading.
  await layer
    .locator(".quick-search-heading-row", { hasText: "Recently viewed" })
    .getByRole("button", { name: "Clear", exact: true })
    .click();
  await expect(page.getByRole("heading", { name: "Recently viewed" })).toHaveCount(0);
  await expect(continueRow(page)).toHaveCount(0);
  await expect.poll(async () => (await storedContinuePoints(page)).length).toBe(0);
  await page.keyboard.press("Escape");

  await waitHomeIdle(page);
  await page.reload();
  await waitReady(page);
  await waitHomeIdle(page);
  await openQuickSearch(page);
  await expect(page.getByRole("heading", { name: "Recently viewed" })).toHaveCount(0);
  await expect(continueRow(page)).toHaveCount(0);
  await page.keyboard.press("Escape");

  await page.goto(`#/browse/book/${book.source}`);
  await expect(page.getByTestId("continue-reading")).toHaveCount(0);
});
