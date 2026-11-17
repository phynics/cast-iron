import { test, installedTest, expect, loadCorpus, type Page } from "./fixtures";
import { buildLibraryWithoutBooks, installLibrary, waitControlled, waitReady } from "./helpers";

const corpus = loadCorpus();

/** The words of each recovery message (#445). A player reads these at the table. */
const RECOVERY_HEADING = "Your book choices can't be read";
const RECOVERY_WHAT =
  "Cast Iron couldn't read which books you have turned on. Your downloaded library and your stars are safe.";
const BOOK_UNAVAILABLE =
  "This book isn't in your library. The link may be old or mistyped.";
const BOOK_REMOVED =
  "That book is no longer in the library. Its entries are still in the books they came from.";
const MISSING_BOOK = "A book in this link isn't in your library, so it can't match anything.";
const KIND_LIST_NOTICE = "That book isn't in your library, so this list shows all spells.";

/** Words that belong to the engine or the store, never to a player. */
const DEVELOPER_WORDS = /TypeError|Reflect|local storage error|IndexedDB|schema|manifest|slot|does-not-exist|no-such-book/i;

/** Writes the stored book choice that the recovery path reads. */
async function storeDisabledBooks(page: Page, value: unknown) {
  await page.evaluate(
    (stored) =>
      new Promise<void>((resolve, reject) => {
        const request = indexedDB.open("cast-iron", 1);
        request.onerror = () => reject(request.error);
        request.onsuccess = () => {
          const db = request.result;
          const tx = db.transaction("meta", "readwrite");
          tx.objectStore("meta").put(stored, "disabled_books");
          tx.oncomplete = () => {
            db.close();
            resolve();
          };
          tx.onerror = () => reject(tx.error);
        };
      }),
    value,
  );
}

/** What the reader can see in the main region, without a closed disclosure's text. */
const visibleText = (page: Page) => page.locator("main").evaluate((el) => (el as HTMLElement).innerText);

/**
 * With `PR_SHOTS=<dir>`, saves the page at 390 px in both themes. Off by default, so the suite
 * writes nothing.
 */
async function capture(page: Page, name: string) {
  const dir = process.env.PR_SHOTS;
  if (!dir) return;
  await page.setViewportSize({ width: 390, height: 844 });
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    await page.screenshot({ path: `${dir}/${name}-${scheme}.png` });
  }
}

installedTest("Unreadable book choices say what happened and keep the way out close", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await storeDisabledBooks(page, "invalid old value");
  await page.reload();

  await expect(page.getByRole("heading", { level: 1, name: RECOVERY_HEADING, exact: true })).toBeVisible();
  // The alert holds the plain explanation and the reassurance, and nothing from the engine.
  await expect(page.getByRole("alert")).toHaveText(RECOVERY_WHAT);
  expect(await visibleText(page)).not.toMatch(DEVELOPER_WORDS);

  // Try again and Reset sit between the reassurance and the technical details.
  const retry = page.getByRole("button", { name: "Try again", exact: true });
  const reset = page.getByRole("button", { name: "Reset book choices", exact: true });
  const details = page.locator(".recovery-details > summary");
  await expect(retry).toBeVisible();
  const [alertBox, retryBox, resetBox, detailsBox] = await Promise.all(
    [page.getByRole("alert"), retry, reset, details].map((locator) => locator.boundingBox()),
  );
  expect(alertBox!.y).toBeLessThan(retryBox!.y);
  expect(resetBox!.y).toBeLessThan(detailsBox!.y);
  expect(retryBox!.height).toBeGreaterThanOrEqual(44);
  // Close: the buttons follow the explanation, not the bottom of the page.
  expect(retryBox!.y - (alertBox!.y + alertBox!.height)).toBeLessThan(200);

  // The engine's own text is still there for a bug report, behind the disclosure.
  await expect(page.locator(".recovery-details code")).toBeHidden();
  await details.click();
  await expect(page.locator(".recovery-details code")).not.toBeEmpty();
  await details.click();
  await capture(page, "recovery-unreadable-choices");

  // The value is still unreadable, so Try again lands on the same screen with nothing lost.
  await retry.click();
  await expect(page.getByRole("heading", { level: 1, name: RECOVERY_HEADING, exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Download", exact: true })).toHaveCount(0);

  // Reset turns every book back on and opens the library.
  await page.getByRole("button", { name: "Reset book choices", exact: true }).click();
  await waitReady(page);
  await page.goto("#/books");
  await expect(page.getByRole("heading", { level: 1, name: "Books", exact: true })).toBeVisible();
});

installedTest("A stored choice that names books the library lacks opens normally", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await storeDisabledBooks(page, ["a-book-from-another-library"]);
  await page.reload();
  await waitReady(page);
  await page.goto("#/books");
  await expect(page.getByRole("heading", { level: 1, name: "Books", exact: true })).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(await visibleText(page)).not.toContain("a-book-from-another-library");
});

test("A book the library does not hold is Book unavailable, with a way out", async ({ page }) => {
  await installLibrary(page);
  for (const address of [
    "#/browse/book/does-not-exist",
    "#/browse/book/does-not-exist/some-chapter",
    "#/browse/source/does-not-exist",
  ]) {
    await page.goto("#/");
    await page.goto(address);
    const main = page.locator("main");
    await expect(main.getByRole("heading", { level: 1, name: "Book unavailable", exact: true }), address).toBeVisible();
    await expect(main.getByRole("alert")).toContainText(BOOK_UNAVAILABLE);
    // The slug is not a title, and the page shows no contents for it.
    expect(await visibleText(page)).not.toMatch(DEVELOPER_WORDS);
    await expect(main.getByText("no chapter order")).toHaveCount(0);
    await expect(page).toHaveTitle(/Book unavailable/);
  }
  await page.goto("#/browse/book/does-not-exist");
  await capture(page, "book-unavailable");
  await page.getByRole("link", { name: "Browse all books", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Books", exact: true })).toBeVisible();
  await page.goBack();
  await page.getByRole("link", { name: "go to Home", exact: true }).click();
  await expect(page.locator('a.nav-link[aria-current="page"]')).toHaveText("Home");
});

test("A real book without chapter order keeps its own explanation", async ({ page, server }) => {
  const book = corpus.catalog.sources.find((source: any) => source.id === "core-rulebook") ?? corpus.catalog.sources[0];
  const variant = buildLibraryWithoutBooks();
  try {
    await server.overlay(variant.dir);
    await installLibrary(page);
    await waitControlled(page);
    await page.goto(`#/browse/book/${encodeURIComponent(book.id)}`);
    await expect(page.getByRole("heading", { level: 1, name: book.title, exact: true })).toBeVisible();
    await expect(page.getByText("This library has no chapter order. Open Settings to check for a library update.")).toBeVisible();
    await expect(page.getByRole("heading", { name: "Book unavailable" })).toHaveCount(0);
  } finally {
    variant.cleanup();
  }
});

test("A removed book's old address opens Books and says so", async ({ page }) => {
  await installLibrary(page);
  for (const address of [
    "#/browse/book/pathfinder-comics-3",
    "#/browse/book/pathfinder-comics-3/page-order",
    "#/browse/source/pathfinder-worldscape-2",
  ]) {
    await page.goto("#/");
    await page.goto(address);
    await expect(page.getByRole("heading", { level: 1, name: "Books", exact: true }), address).toBeVisible();
    await expect(page.getByTestId("book-removed")).toHaveText(BOOK_REMOVED);
    await expect(page.getByRole("alert")).toHaveCount(1);
    expect(await visibleText(page)).not.toMatch(DEVELOPER_WORDS);
  }
  await page.goto("#/browse/book/pathfinder-comics-3");
  await capture(page, "book-removed");
  // Leaving the address leaves the line behind.
  await page.getByRole("link", { name: "Manage books" }).click();
  await page.goto("#/books");
  await expect(page.getByTestId("book-removed")).toHaveCount(0);
});

test("An unknown book in a list or search link is named in plain words", async ({ page }) => {
  await installLibrary(page);
  // A kind list still shows the whole list, and says why.
  await page.goto("#/browse/kind/spell?source=no-such-book");
  const main = page.locator("main");
  await expect(main.getByText(KIND_LIST_NOTICE)).toBeVisible();
  expect(await visibleText(page)).not.toMatch(DEVELOPER_WORDS);
  await capture(page, "kind-list-unknown-book");

  // Search names the book once, and one tap takes it out of the search.
  await page.goto("#/search?q=fireball&source=no-such-book");
  const hint = page.getByTestId("missing-book-hint");
  await expect(hint).toContainText(MISSING_BOOK);
  await expect(page.locator(".result-count")).toContainText("No results");
  expect(await visibleText(page)).not.toMatch(DEVELOPER_WORDS);
  await capture(page, "search-unknown-book");
  await hint.getByRole("button", { name: "Remove the missing book from this search" }).click();
  await expect(hint).toHaveCount(0);
  await expect(page).toHaveURL(/#\/search\?q=fireball$/);
  await expect(page.locator(".result-count")).not.toContainText("No results");
});
