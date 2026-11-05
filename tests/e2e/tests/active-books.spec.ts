import { test, installedTest, expect, loadCorpus } from "./fixtures";
import {
  bookEntryOrder,
  cacheContents,
  contentCaches,
  bookOwners,
  bookTrees,
  buildLibraryWithoutBooks,
  corpusEntry,
  firstBookPlacement,
  groupLetter,
  installLibrary,
  openBookGroups,
  waitControlled,
  waitReady,
  waitResults,
} from "./helpers";

const corpus = loadCorpus();
const owners = bookOwners();

/** "1 entry" or "N entries", matching the view's wording. */
const noun = (n: number) => (n === 1 ? "entry" : "entries");

/** A spell every library has, and the book the reader places it in first. */
function target() {
  const entry = corpusEntry("spell", "Fireball");
  const book = firstBookPlacement(entry.id);
  if (!book) throw new Error(`The compiled corpus does not place ${entry.id} in a book.`);
  const source = corpus.catalog.sources.find((candidate) => candidate.id === book.source);
  if (!source) throw new Error(`The compiled catalog has no source ${JSON.stringify(book.source)}.`);
  return { entry, book, source };
}

test("Book visibility persists offline and filters discovery while keeping stars readable", async ({ openProfile }) => {
  const { entry, book, source } = target();
  let context = await openProfile();
  let page = await context.newPage();
  await installLibrary(page);
  await waitControlled(page);
  await page.goto("#/books");
  await waitReady(page);
  await openBookGroups(page);
  const row = page.locator(`.book-directory-item[data-book-id="${book.source}"]`);
  const toggle = row.getByRole("checkbox", { name: `Active ${source.title}`, exact: true });
  await expect(toggle).toBeChecked();
  await page.goto(`#/entry/${entry.id}`);
  await page.locator(".bookmark-toggle").click();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");

  await page.goto("#/books");
  await openBookGroups(page);
  await row.getByRole("checkbox", { name: `Active ${source.title}`, exact: true }).uncheck();
  await expect(row.getByRole("checkbox", { name: `Active ${source.title}`, exact: true })).not.toBeChecked();
  await expect(row.getByRole("checkbox", { name: `Active ${source.title}`, exact: true })).toBeEnabled();
  await context.close();

  context = await openProfile({ offline: true });
  page = await context.newPage();
  await page.goto("#/books");
  await waitReady(page);
  await openBookGroups(page);
  const restored = page.locator(`.book-directory-item[data-book-id="${book.source}"]`);
  await expect(restored.getByRole("checkbox", { name: `Active ${source.title}`, exact: true })).not.toBeChecked();
  await page.goto(`#/browse/kind/spell?l=${encodeURIComponent(groupLetter(entry.title))}`);
  // The reader places an entry in its first book, so an entry that a second book also
  // holds disappears with this one. Only the selected letter group is in the document.
  const expected = corpus.catalog.entries.filter((stub) =>
    stub.kind === "spell" && groupLetter(stub.title) === groupLetter(entry.title) &&
    (owners.get(stub.id) ?? stub.source) !== book.source).length;
  await expect(page.locator('.entry-list a[href^="#/entry/"]')).toHaveCount(expected);
  await expect(page.getByRole("link", { name: entry.title, exact: true })).toHaveCount(0);

  await page.goto("#/search?kind=spell");
  await expect(page.locator(".results .hit").first()).toBeVisible();
  // With only the kind filter the result set is discovery-wide, and no hit may come from
  // the disabled book. The starred entry has no result at all.
  const hits = await page.locator(".results .hit-link").evaluateAll(links => links.map(link => (link as HTMLAnchorElement).hash.replace(/^#\/entry\//, "").split("?")[0]));
  expect(hits.length).toBeGreaterThan(0);
  for (const id of hits) expect(owners.get(id)).not.toBe(book.source);
  await expect(page.locator(`.results a[href="#/entry/${entry.id}"]`)).toHaveCount(0);
  await page.goto("#/");
  await page.locator(".home-bookmarks").getByRole("link", { name: entry.title, exact: true }).click();
  await expect(page.getByRole("heading", { name: entry.title, level: 1, exact: true })).toBeVisible();
  await expect(page.locator(".disabled-book-notice")).toContainText("This book is disabled for search and browsing.");
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
});

const adventureCategories = ["Adventure Path", "Module", "Society"];

/** An entry that an Adventures-tier book places first, so deactivating Adventures hides it. */
function adventureEntry() {
  for (const source of corpus.catalog.sources) {
    if (!adventureCategories.includes(source.category as string)) continue;
    const id = [...owners].find(([, bookId]) => bookId === source.id)?.[0];
    if (id) return { id, source, entry: corpus.entries.get(id)! };
  }
  throw new Error("The compiled corpus has no entry in an Adventures-tier book.");
}

const resultCount = (page: any) => page.locator(".result-count > span").first().innerText();

test("Only Rulebooks and the group controls change search counts, and keep downloads and stars", async ({ page }) => {
  const { id, source, entry } = adventureEntry();
  const adventureBooks = corpus.catalog.sources.filter((row) => adventureCategories.includes(row.category as string));
  await installLibrary(page);
  await waitControlled(page);
  await page.goto(`#/entry/${id}`);
  await page.locator(".bookmark-toggle").click();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  const downloads = await cacheContents(page);
  expect(contentCaches(downloads).length).toBeGreaterThan(0);

  await page.goto("#/search?q=fire");
  await waitResults(page, "fire");
  const everything = await resultCount(page);

  // Every book starts active. The preset is one click, and nothing reloads.
  await page.goto("#/books");
  await expect(page.getByRole("checkbox", { name: /^Active / }).first()).toBeChecked();
  await page.getByRole("button", { name: "Only Rulebooks", exact: true }).click();
  await expect(page.getByRole("status").filter({ hasText: "Only Rulebooks are active." })).toBeVisible();
  await expect(page.locator('.book-tier[data-tier="Adventures"] .tier-active')).toHaveText(
    `0 of ${adventureBooks.length} active`,
  );
  const rulebooks = page.locator('.book-tier[data-tier="Rulebooks"] .tier-active');
  await expect(rulebooks).toHaveText(/^(\d+) of \1 active$/);

  await page.goto("#/search?q=fire");
  await waitResults(page, "fire");
  const rulebooksOnly = await resultCount(page);
  expect(rulebooksOnly).not.toBe(everything);

  // The starred entry and the download survive the deactivation.
  await page.goto(`#/entry/${id}`);
  await expect(page.getByRole("heading", { name: entry.title, level: 1, exact: true })).toBeVisible();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator(".disabled-book-notice")).toBeVisible();
  expect(await cacheContents(page)).toEqual(downloads);
  await page.goto("#/");
  await expect(page.locator(".home-bookmarks").getByRole("link", { name: entry.title, exact: true })).toBeVisible();

  // A group's own control brings its books back in one click, with no reload.
  await page.goto("#/books");
  const adventures = page.locator('.book-tier[data-tier="Adventures"]');
  await expect(adventures.locator("details.tier-details")).not.toHaveAttribute("open", "");
  await adventures.getByRole("button", { name: "Activate all Adventures", exact: true }).click();
  await expect(adventures.locator(".tier-active")).toHaveText(`${adventureBooks.length} of ${adventureBooks.length} active`);
  await page.goto("#/search?q=fire");
  await waitResults(page, "fire");
  expect(await resultCount(page)).not.toBe(rulebooksOnly);
  await page.goto("#/books");
  await adventures.getByRole("button", { name: "Deactivate all Adventures", exact: true }).click();
  await expect(adventures.locator(".tier-active")).toHaveText(`0 of ${adventureBooks.length} active`);
  await expect(page.getByRole("status").filter({ hasText: "Adventures: all" })).toBeVisible();
  // A per-book toggle replaces the bulk message instead of leaving it on screen.
  await page.locator('.book-tier[data-tier="Rulebooks"]').getByRole("checkbox", { name: /^Active / }).first().uncheck();
  await expect(page.locator(".books-status")).toHaveText("");
  await adventures.locator(".tier-summary").click();
  await expect(adventures.getByRole("checkbox", { name: `Active ${source.title}`, exact: true })).not.toBeChecked();
  expect(await cacheContents(page)).toEqual(downloads);
});

// Clean profile on purpose: from a pre-installed profile the reload below can interrupt a
// startup cache read, which Linux WebKit reports as "Context is stopped" (see #121).
test("No active books offers a direct recovery path from Home", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  // Every compiled book disabled is the state the Books screen reaches by unchecking
  // every publication. Write it once instead of toggling hundreds of rows, then verify
  // the reader's recovery path through the screen.
  const books = bookTrees().map((book) => book.source);
  await page.evaluate((disabled) => new Promise<void>((resolve, reject) => {
    const request = indexedDB.open("cast-iron", 1);
    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const db = request.result;
      const tx = db.transaction("meta", "readwrite");
      tx.objectStore("meta").put(disabled, "disabled_books");
      tx.oncomplete = () => { db.close(); resolve(); };
      tx.onerror = () => reject(tx.error);
    };
  }), books);
  await page.reload();
  await expect(page.getByText("No books are active.")).toBeVisible();
  await page.getByRole("link", { name: "Open Books" }).click();
  await expect(page.getByRole("heading", { name: "Books", exact: true })).toBeVisible();
  const first = page.getByRole("checkbox", { name: /^Active / }).first();
  await first.check();
  await expect(first).toBeChecked();
});

installedTest("Invalid book choices recover without removing the library or stars", async ({ page }) => {
  const { entry, book, source } = target();
  await installLibrary(page);
  await waitControlled(page);
  await page.goto(`#/entry/${entry.id}`);
  await page.locator(".bookmark-toggle").click();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  await page.evaluate(() => new Promise<void>((resolve, reject) => {
    const request = indexedDB.open("cast-iron", 1);
    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const db = request.result;
      const tx = db.transaction("meta", "readwrite");
      tx.objectStore("meta").put("invalid old value", "disabled_books");
      tx.oncomplete = () => { db.close(); resolve(); };
      tx.onerror = () => reject(tx.error);
    };
  }));
  await page.reload();
  await expect(page.getByRole("heading", { name: "Book choices could not be opened" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Download", exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "Reset active books" }).click();
  await waitReady(page);
  await page.goto("#/");
  await expect(page.locator(".home-bookmarks").getByRole("link", { name: entry.title, exact: true })).toBeVisible();
  await page.goto("#/books");
  await openBookGroups(page);
  await expect(page.getByRole("checkbox", { name: `Active ${source.title}`, exact: true })).toBeChecked();
});

test("A source page count matches the entries its kind groups render", async ({ page, server }) => {
  const { book } = target();
  // Without a book order the old source page renders the kind groups. The canonical build
  // has books, so serve a manifest that leaves books.json out and install from that overlay.
  const variant = buildLibraryWithoutBooks();
  try {
    await server.overlay(variant.dir);
    await installLibrary(page);
    await waitControlled(page);
    await page.goto(`#/browse/source/${encodeURIComponent(book.source)}`);
    await waitReady(page);
    await expect(page.getByRole("heading", { name: book.title, level: 1, exact: true })).toBeVisible();

    // Without books every entry keeps its catalog source, which is what the groups show.
    const expected = corpus.catalog.entries.filter((stub) => stub.source === book.source).length;
    expect(expected).toBeGreaterThan(0);
    await expect(page.getByTestId("source-count")).toHaveText(`${expected} ${noun(expected)} in your active books`);

    for (const group of await page.locator(".kind-group").all()) {
      if (!(await group.evaluate((el) => (el as HTMLDetailsElement).open))) {
        await group.locator("summary").click();
      }
      await expect(group.locator(".entry-list")).toBeVisible();
    }
    await expect(page.locator('.kind-groups a[href^="#/entry/"]')).toHaveCount(expected);

    // Deactivate through the same stored choice the Books screen writes. The page is a
    // legacy source page here, so it has no checkbox of its own.
    await page.evaluate((disabled) => new Promise<void>((resolve, reject) => {
      const request = indexedDB.open("cast-iron", 1);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        const db = request.result;
        const tx = db.transaction("meta", "readwrite");
        tx.objectStore("meta").put(disabled, "disabled_books");
        tx.oncomplete = () => { db.close(); resolve(); };
        tx.onerror = () => reject(tx.error);
      };
    }), [book.source]);
    await page.reload();
    await waitReady(page);
    await expect(page.getByTestId("source-count")).toHaveText("0 entries in your active books");
    await expect(page.locator('.kind-groups a[href^="#/entry/"]')).toHaveCount(0);
  } finally {
    variant.cleanup();
  }
});

installedTest("Book page count follows its contents and survives deactivation", async ({ page }) => {
  // Find a book whose contents tree and SourceInfo total disagree. The canonical library has
  // several; the PSRD carry-over library does not always differ for Fireball's first book.
  // A header that used the publication total or the active-book set would disagree with the
  // page, which is the property this test proves.
  const candidate = bookTrees()
    .map((book) => {
      const source = corpus.catalog.sources.find((row) => row.id === book.source);
      return source ? { book, source, distinct: new Set(bookEntryOrder(book)).size } : null;
    })
    .find((row) => row && row.distinct !== row.source.entry_count);
  expect(candidate, "the test needs a book whose contents differ from its publication total").toBeTruthy();
  const { book, source, distinct } = candidate!;

  await installLibrary(page);
  await waitControlled(page);
  await page.goto(`#/browse/source/${encodeURIComponent(book.source)}`);
  await waitReady(page);
  await expect(page.getByRole("heading", { name: book.title, level: 1, exact: true })).toBeVisible();
  await expect(page.getByTestId("source-count")).toHaveText(`${distinct} ${noun(distinct)} in this book`);
  await expect(page.locator(".book > .notice")).toHaveCount(0);
  await expect(page.locator(".toc-top")).toBeVisible();

  await page.goto("#/books");
  await waitReady(page);
  await openBookGroups(page);
  const row = page.locator(`.book-directory-item[data-book-id="${book.source}"]`);
  const toggle = row.getByRole("checkbox", { name: `Active ${source.title}`, exact: true });
  await toggle.uncheck();
  await expect(toggle).not.toBeChecked();

  await page.goto(`#/browse/source/${encodeURIComponent(book.source)}`);
  await expect(page.getByRole("heading", { name: book.title, level: 1, exact: true })).toBeVisible();
  // The contents stay readable, so the count must not change when the book is disabled.
  await expect(page.getByTestId("source-count")).toHaveText(`${distinct} ${noun(distinct)} in this book`);
  await expect(page.locator(".book > .notice")).toContainText("This book is disabled for search and category browsing.");
  await expect(page.locator(".toc-top")).toBeVisible();
});
