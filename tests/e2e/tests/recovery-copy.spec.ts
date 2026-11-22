import { test, installedTest, expect, loadCorpus, type Page } from "./fixtures";
import { buildLibraryWithoutBooks, installLibrary, openBookGroups, waitControlled, waitReady } from "./helpers";

const corpus = loadCorpus();

/** The words of each recovery message (#445). A player reads these at the table. */
const RECOVERY_HEADING = "Your book choices can't be read";
const RECOVERY_WHAT =
  "Cast Iron couldn't read which books you have turned on. Your downloaded library and your stars are safe.";
const RECOVERY_NEXT =
  "Try again. If that doesn't help, reset your book choices: every book turns on, except any third-party books you haven't seen before, which stay off. You can change this in Manage books.";
const RECOVERY_RESET_FAILED = "Your book choices could not be reset, so nothing was changed. Try again.";
const BOOK_UNAVAILABLE =
  "This book isn't in your library. The link may be old or mistyped.";
const BOOK_REMOVED =
  "That book is no longer in the library. Its entries are still in the books they came from.";
const MISSING_BOOK = "A book in this link isn't in your library, so it can't match anything.";
const KIND_LIST_NOTICE = "That book isn't in your library, so this list shows all spells.";

/** Words that belong to the engine or the store, never to a player. */
const DEVELOPER_WORDS = /TypeError|Reflect|local storage error|IndexedDB|schema|manifest|slot|does-not-exist|no-such-book/i;

/** Writes one stored value (`undefined` deletes it) in the meta store the app reads at start. */
async function putMeta(page: Page, key: string, value: unknown) {
  await page.evaluate(
    ([name, stored]) =>
      new Promise<void>((resolve, reject) => {
        const request = indexedDB.open("cast-iron", 1);
        request.onerror = () => reject(request.error);
        request.onsuccess = () => {
          const db = request.result;
          const tx = db.transaction("meta", "readwrite");
          if (stored === null) tx.objectStore("meta").delete(name as string);
          else tx.objectStore("meta").put(stored, name as string);
          tx.oncomplete = () => {
            db.close();
            resolve();
          };
          tx.onerror = () => reject(tx.error);
        };
      }),
    [key, value === undefined ? null : value] as const,
  );
}

/** Writes the stored book choice that the recovery path reads. */
const storeDisabledBooks = (page: Page, value: unknown) => putMeta(page, "disabled_books", value);

/** The books the corpus marks third party (#189): opt-in, and off for a reader who has not seen them. */
const thirdParty: any[] = corpus.catalog.sources.filter((row: any) => row.third_party);

/** What the reader can see in the main region, without a closed disclosure's text. */
const visibleText = (page: Page) => page.locator("main").evaluate((el) => (el as HTMLElement).innerText);

/**
 * With `PR_SHOTS=<dir>`, saves the page at 390 px and 1280 px in both themes. Off by default,
 * so the suite writes nothing.
 */
async function capture(page: Page, name: string) {
  const dir = process.env.PR_SHOTS;
  if (!dir) return;
  for (const width of [390, 1280]) {
    await page.setViewportSize({ width, height: width === 390 ? 844 : 900 });
    for (const scheme of ["light", "dark"] as const) {
      await page.emulateMedia({ colorScheme: scheme });
      await page.screenshot({ path: `${dir}/${name}-${width}-${scheme}.png` });
    }
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
  await expect(page.getByText(RECOVERY_NEXT, { exact: true })).toBeVisible();
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
  // The disclosure carries the shared chevron, which turns when it opens.
  const chevron = details.locator("svg.disclosure-chevron");
  await expect(chevron).toBeVisible();
  const closed = await chevron.evaluate((el) => getComputedStyle(el).transform);
  await details.click();
  expect(await chevron.evaluate((el) => getComputedStyle(el).transform)).not.toBe(closed);
  await expect(page.locator(".recovery-details code")).not.toBeEmpty();
  await details.click();
  await expect(page.locator(".recovery-details code")).toBeHidden();
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

/** The `Active {title}` checkbox for every book that Manage books lists, by title. */
async function activeStates(page: Page, titles: string[]) {
  await page.goto("#/books/manage");
  await openBookGroups(page);
  const states: Record<string, boolean> = {};
  for (const title of titles) {
    states[title] = await page.getByRole("checkbox", { name: `Active ${title}`, exact: true }).isChecked();
  }
  return states;
}

installedTest("Reset book choices does what the copy says about third-party books", async ({ page }) => {
  expect(thirdParty.length).toBeGreaterThan(0);
  const paizo = corpus.catalog.sources.find((row: any) => row.id === "core-rulebook")!;
  const titles = [paizo.title, ...thirdParty.map((row: any) => row.title as string)];
  await installLibrary(page);
  await waitControlled(page);

  // The device has seen the third-party books: reset turns every book on, them included.
  await putMeta(page, "known_third_party", thirdParty.map((row: any) => row.id));
  await storeDisabledBooks(page, "invalid old value");
  await page.reload();
  await expect(page.getByText(RECOVERY_NEXT, { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Reset book choices", exact: true }).click();
  await waitReady(page);
  const seen = await activeStates(page, titles);
  expect(seen).toEqual(Object.fromEntries(titles.map((title) => [title, true])));

  // The device has not seen them: reset turns the rest on and they stay off.
  await putMeta(page, "known_third_party", undefined);
  await storeDisabledBooks(page, "invalid old value");
  await page.reload();
  await page.getByRole("button", { name: "Reset book choices", exact: true }).click();
  await waitReady(page);
  const unseen = await activeStates(page, titles);
  expect(unseen).toEqual({
    [paizo.title]: true,
    ...Object.fromEntries(thirdParty.map((row: any) => [row.title, false])),
  });
});

installedTest("A refused reset says nothing was changed and leaves the screen usable", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await storeDisabledBooks(page, "invalid old value");
  // From here on the browser refuses to write the book choice, as a full or locked store does.
  await page.addInitScript(() => {
    const put = IDBObjectStore.prototype.put;
    IDBObjectStore.prototype.put = function (value: unknown, key?: IDBValidKey) {
      if (key === "disabled_books") throw new DOMException("The store refused the write.", "QuotaExceededError");
      return put.call(this, value, key);
    };
  });
  await page.reload();
  const reset = page.getByRole("button", { name: "Reset book choices", exact: true });
  await reset.click();
  await expect(page.getByRole("alert").filter({ hasText: RECOVERY_RESET_FAILED })).toBeVisible();
  expect(await visibleText(page)).not.toMatch(DEVELOPER_WORDS);
  // The screen stays: its buttons work again and the explanation is still there.
  await expect(reset).toBeEnabled();
  await expect(page.getByRole("button", { name: "Try again", exact: true })).toBeEnabled();
  await expect(page.getByRole("heading", { level: 1, name: RECOVERY_HEADING, exact: true })).toBeVisible();
  await capture(page, "recovery-reset-failed");
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
    // Books still shows, so this is a notice and not an alert.
    await expect(page.getByTestId("book-removed")).toHaveClass(/notice/);
    await expect(page.getByRole("alert")).toHaveCount(0);
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

test("Removing a missing book from a search keeps the other restrictions", async ({ page }) => {
  await installLibrary(page);
  const params = () => new URLSearchParams(new URL(page.url()).hash.split("?")[1] ?? "");
  await page.goto("#/search?q=fireball&kind=spell&source=core-rulebook&source=no-such-book&facet=school:evocation");
  const hint = page.getByTestId("missing-book-hint");
  await expect(hint).toBeVisible();
  // The known book still matches, so the page does not claim an empty search.
  const fireball = page.locator(".results").getByRole("link", { name: /Fireball/ }).first();
  await expect(fireball).toBeVisible();
  const before = page.url();

  await hint.getByRole("button", { name: "Remove the missing book from this search" }).click();
  await expect(hint).toHaveCount(0);
  const kept = params();
  expect(kept.get("q")).toBe("fireball");
  expect(kept.get("kind")).toBe("spell");
  expect(kept.getAll("source")).toEqual(["core-rulebook"]);
  expect(kept.get("facet")).toBe("school:evocation");
  await expect(fireball).toBeVisible();

  // Reload keeps the cleaned search.
  await page.reload();
  await expect(page.getByTestId("missing-book-hint")).toHaveCount(0);
  expect(params().getAll("source")).toEqual(["core-rulebook"]);
  await expect(fireball).toBeVisible();

  // Back returns to the earlier address, which still names the missing book.
  await page.goBack();
  await expect.poll(() => page.url()).toBe(before);
  await expect(page.getByTestId("missing-book-hint")).toBeVisible();
});

const corebook = corpus.catalog.sources.find((row: any) => row.id === "core-rulebook")!;

/** Words that name the old book-state vocabulary (#582): a reader turns books on and off. */
const OLD_STATE_WORDS = /disabled|enabled|reactivate/i;

test("An unknown entry says so in plain words and offers a search, not Settings", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/entry/ref/spell/does-not-exist");
  const main = page.locator("main");
  await expect(main.getByRole("heading", { level: 1, name: "Entry unavailable", exact: true })).toBeVisible();
  await expect(main.getByRole("alert")).toHaveCount(1);
  await expect(main.getByRole("alert")).toContainText(
    "This entry isn't in your library. The link may come from another version of Cast Iron.",
  );
  const text = await visibleText(page);
  expect(text).not.toContain("ref/");
  expect(text).not.toMatch(DEVELOPER_WORDS);
  expect(text).not.toMatch(OLD_STATE_WORDS);
  await expect(main.getByRole("link", { name: "Open Settings" })).toHaveCount(0);
  await expect(main.getByRole("link", { name: "Search for it", exact: true })).toHaveAttribute(
    "href",
    "#/search?q=does%20not%20exist",
  );
  await capture(page, "entry-unavailable");
  await main.getByRole("link", { name: "go to Home", exact: true }).click();
  await expect(page.locator('a.nav-link[aria-current="page"]')).toHaveText("Home");
});

test("A page that does not exist is Page not found, without the address", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/nope/here");
  const main = page.locator("main");
  await expect(main.getByRole("heading", { level: 1, name: "Page not found", exact: true })).toBeVisible();
  await expect(main.getByRole("alert")).toHaveCount(1);
  await expect(main.getByRole("alert")).toHaveText("There is nothing at this address. The link may be old or mistyped.");
  expect(await visibleText(page)).not.toContain("#/nope");
  await expect(main.getByRole("link", { name: "Search the rules", exact: true })).toHaveAttribute("href", "#/search");
  await capture(page, "page-not-found");
  await main.getByRole("link", { name: "go to Home", exact: true }).click();
  await expect(page.locator('a.nav-link[aria-current="page"]')).toHaveText("Home");
});

test("A chapter that the book does not have is Chapter unavailable, with its own title", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/book/core-rulebook");
  await expect(page.getByRole("heading", { level: 1, name: corebook.title, exact: true })).toBeVisible();
  await page.goto("#/browse/book/core-rulebook/zzz");
  const main = page.locator("main");
  await expect(main.getByRole("heading", { level: 1, name: "Chapter unavailable", exact: true })).toBeVisible();
  await expect(main.getByRole("alert")).toHaveCount(1);
  await expect(page).toHaveTitle(/^Chapter unavailable/);
  expect(await visibleText(page)).not.toMatch(DEVELOPER_WORDS);
  await capture(page, "chapter-unavailable");
  await main.getByRole("link", { name: "Open the book", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: corebook.title, exact: true })).toBeVisible();
});

test("An entry of a turned-off book says so, and mentions Starred only when it is starred", async ({ page }) => {
  expect(thirdParty.length).toBeGreaterThan(0);
  const stub = corpus.catalog.entries.find((candidate: any) => candidate.source === thirdParty[0].id)!;
  await installLibrary(page);
  await waitControlled(page);
  await page.goto(`#/entry/${stub.id}`);
  const notice = page.locator(".disabled-book-notice");
  await expect(notice).toContainText("This entry's book is turned off, so it doesn't appear in search or lists.");
  await expect(notice.getByRole("link", { name: "Manage books", exact: true })).toHaveAttribute("href", "#/books/manage");
  await expect(notice).not.toContainText("Starred");
  expect(await visibleText(page)).not.toMatch(OLD_STATE_WORDS);
  await page.locator(".bookmark-toggle").click();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  await expect(notice).toContainText("It stays in your Starred list.");
  await capture(page, "entry-book-turned-off");
});
