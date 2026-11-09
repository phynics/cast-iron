import { installedTest as test, expect, loadCorpus } from "./fixtures";
import {
  bookReadingOrder,
  bookTrees,
  installLibrary,
  openBookEntry,
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

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

test("An entry opened from a book sets its continue point on the book page and Home, and both survive a reload", async ({ page }) => {
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

  await page.goto("#/");
  await waitReady(page);
  const homeRow = page.getByTestId("home-continue");
  await expect(homeRow).toBeVisible();
  await expect(homeRow).toContainText(`Continue ${book.title}`);
  // It is the first row of Recently viewed, above the ordinary visit rows.
  await expect(page.locator(".home-recent > li").first()).toHaveAttribute("data-testid", "home-continue");
  // Opening it keeps the book context, which is what makes the point move forward.
  await expect(homeRow.getByRole("link")).toHaveAttribute("href", new RegExp(`#/entry/.*[?&]book=${book.source}`));

  // The stored point, not a page signal, is what a reload reads.
  await waitHomeIdle(page);
  await page.reload();
  await waitReady(page);
  await waitHomeIdle(page);
  await expect(page.getByTestId("home-continue")).toContainText(`Continue ${book.title}`);

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
  await expect(page.getByTestId("home-continue")).toHaveCount(0);
});

test("Clear history removes the continue point and it stays gone after a reload", async ({ page }) => {
  const { book, entry } = placedEntry();
  await openBookEntry(page, book, entry.id);
  await expect(page.getByRole("heading", { level: 1, name: entry.title, exact: true })).toBeVisible();
  await expectStoredPoint(page, book.source, entry.id);

  await page.goto("#/");
  await waitReady(page);
  await waitHomeIdle(page);
  await expect(page.getByTestId("home-continue")).toBeVisible();
  await page.getByRole("button", { name: "Clear", exact: true }).click();
  await expect(page.getByTestId("home-continue")).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Recently viewed" })).toHaveCount(0);
  await expect.poll(async () => (await storedContinuePoints(page)).length).toBe(0);

  await waitHomeIdle(page);
  await page.reload();
  await waitReady(page);
  await waitHomeIdle(page);
  await expect(page.getByTestId("home-continue")).toHaveCount(0);
  await page.goto(`#/browse/book/${book.source}`);
  await expect(page.getByTestId("continue-reading")).toHaveCount(0);
});
