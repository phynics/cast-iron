import { test, installedTest, expect, loadCorpus } from "./fixtures";
import {
  cacheContents,
  contentCaches,
  count,
  bookOwners,
  bookTrees,
  buildLibraryWithoutBooks,
  corpusEntry,
  firstBookPlacement,
  groupLetter,
  installLibrary,
  openBookGroups,
  openQuickSearch,
  quickOptions,
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

installedTest("Quick search suggestions follow the active books", async ({ page }) => {
  const { entry, book, source } = target();
  await installLibrary(page);
  /** Types the entry's title and returns the `Kind · Book` lines once suggestions have arrived. */
  const suggestions = async () => {
    await page.goto("#/");
    const layer = await openQuickSearch(page);
    await layer.getByRole("combobox", { name: "Search the rules" }).fill(entry.title);
    await expect(quickOptions(page).last()).toHaveText(`See all results for “${entry.title}”`);
    // Suggestions follow a pause in typing: wait for them, not for a fixed time.
    const lines = layer.locator('[role="option"] .quick-search-meta');
    await expect.poll(() => lines.count()).toBeGreaterThan(0);
    return lines.allInnerTexts();
  };
  const fromBook = (lines: string[]) => lines.some((line) => line.endsWith(` · ${source.title}`));
  // With every book active, the book that holds the entry is suggested.
  expect(fromBook(await suggestions())).toBe(true);

  await page.goto("#/books/manage");
  await openBookGroups(page);
  const toggle = page.locator(`.book-directory-item[data-book-id="${book.source}"]`)
    .getByRole("checkbox", { name: `Active ${source.title}`, exact: true });
  await toggle.uncheck();
  await expect(toggle).not.toBeChecked();
  await expect(toggle).toBeEnabled();

  // The same lookup still answers, from the other books, and offers nothing from the disabled one.
  expect(fromBook(await suggestions())).toBe(false);
});

test("Book visibility persists offline and filters discovery while keeping stars readable", async ({ openProfile }) => {
  const { entry, book, source } = target();
  let context = await openProfile();
  let page = await context.newPage();
  await installLibrary(page);
  await waitControlled(page);
  await page.goto("#/books/manage");
  await waitReady(page);
  await openBookGroups(page);
  const row = page.locator(`.book-directory-item[data-book-id="${book.source}"]`);
  const toggle = row.getByRole("checkbox", { name: `Active ${source.title}`, exact: true });
  await expect(toggle).toBeChecked();
  await page.goto(`#/entry/${entry.id}`);
  await page.locator(".bookmark-toggle").click();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");

  await page.goto("#/books/manage");
  await openBookGroups(page);
  await row.getByRole("checkbox", { name: `Active ${source.title}`, exact: true }).uncheck();
  await expect(row.getByRole("checkbox", { name: `Active ${source.title}`, exact: true })).not.toBeChecked();
  await expect(row.getByRole("checkbox", { name: `Active ${source.title}`, exact: true })).toBeEnabled();
  await context.close();

  context = await openProfile({ offline: true });
  page = await context.newPage();
  await page.goto("#/books/manage");
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

/** Restated independently of the Rust tier table: the categories each tier holds. */
const tierCategories: Record<string, string[]> = {
  rulebooks: ["RPG"],
  companions: ["Player Companion"],
  setting: ["Campaign Setting"],
};
const allBooks: string[] = corpus.catalog.sources.map((row: any) => row.id);
/** The books the corpus marks third party (#189); they are opt-in and start off. */
const thirdParty = new Set<string>(
  corpus.catalog.sources.filter((row: any) => row.third_party).map((row: any) => row.id as string),
);
/** The default active set for a settled profile: every book that is not third party. */
const paizoBooks = allBooks.filter((id) => !thirdParty.has(id));
const idsIn = (...tiers: string[]) =>
  corpus.catalog.sources
    .filter(
      (row: any) =>
        !row.third_party && tiers.some((tier) => tierCategories[tier].includes(row.category)),
    )
    .map((row: any) => row.id as string);
/** The books each offered preset turns on, from the compiled catalog. */
const presets = [
  { name: "Paizo Rulebooks", ids: idsIn("rulebooks") },
  { name: "Paizo Rulebooks & More", ids: idsIn("rulebooks", "companions", "setting") },
  { name: "Paizo Everything", ids: paizoBooks },
  { name: "Everything incl. 3rd party", ids: allBooks },
];
const countLine = (active: number, name?: string) =>
  `${count(active)} of ${count(allBooks.length)} books active` + (name ? ` · ${name}` : "");

installedTest("Manage books opens from Books, and Done returns to the link it came from", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/books");
  await waitReady(page);
  const link = page.getByRole("link", { name: "Manage books", exact: true });
  await link.click();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  const heading = page.getByRole("heading", { level: 1, name: "Manage books", exact: true });
  await expect(heading).toBeVisible();
  // Entering puts focus on the heading, and the Books tab stays current.
  await expect(heading).toBeFocused();
  await expect(page.getByRole("navigation", { name: "Primary" }).getByRole("link", { name: "Books", exact: true }))
    .toHaveAttribute("aria-current", "page");
  // Every control is a 44 px target; the rows carry no bulk control.
  const done = page.getByRole("button", { name: "Done", exact: true });
  expect((await done.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  for (const row of await page.locator(".preset-row").all()) {
    expect((await row.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  }
  await expect(page.getByRole("button", { name: /Activate all|Deactivate all|Only Rulebooks/ })).toHaveCount(0);
  // A checkbox row is one label of at least 56 px, with no chevron and no link.
  const first = page.locator(".book-directory-item").first();
  expect((await first.locator(".toggle-row").boundingBox())!.height).toBeGreaterThanOrEqual(56);
  await expect(first.locator("a, .book-chevron")).toHaveCount(0);

  await done.click();
  await expect(page).toHaveURL(/#\/books$/);
  await expect(page.getByRole("heading", { level: 1, name: "Books", exact: true })).toBeVisible();
  await expect(link).toBeFocused();
  // Done went Back: the Books entry was not stacked a second time.
  await page.goBack();
  await expect(page).not.toHaveURL(/#\/books/);
});

installedTest("Done on a page opened directly replaces it with Books", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/books/manage");
  await expect(page.getByRole("heading", { level: 1, name: "Manage books", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Done", exact: true }).click();
  await expect(page).toHaveURL(/#\/books$/);
  await expect(page.getByRole("link", { name: "Manage books", exact: true })).toBeFocused();
});

installedTest("The old #/books/active address opens Manage books and writes the new address", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/books/active");
  await expect(page.getByRole("heading", { level: 1, name: "Manage books", exact: true })).toBeVisible();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  // The page is live: a preset still applies from the old address.
  await expect(page.getByRole("button", { name: "Paizo Everything", exact: true })).toBeVisible();
});

installedTest("Settings links to Manage books", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/settings");
  const link = page.getByRole("link", { name: "Manage books", exact: true });
  await expect(link).toHaveAttribute("href", "#/books/manage");
  await link.click();
  await expect(page.getByRole("heading", { level: 1, name: "Manage books", exact: true })).toBeVisible();
});

installedTest("A checkbox saves at once, survives a reload, and the count line follows", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await page.goto("#/books/manage");
  await openBookGroups(page);
  const { book, source } = target();
  const toggle = page.locator(`.book-directory-item[data-book-id="${book.source}"]`)
    .getByRole("checkbox", { name: `Active ${source.title}`, exact: true });
  await expect(toggle).toBeChecked();
  await expect(page.locator(".books-count")).toHaveText(countLine(paizoBooks.length));
  await toggle.uncheck();
  await expect(page.locator(".books-count")).toHaveText(countLine(paizoBooks.length - 1));
  // The checkbox never disables itself, so keyboard focus stays on it.
  await expect(toggle).toBeEnabled();
  await page.reload();
  await waitReady(page);
  await openBookGroups(page);
  await expect(toggle).not.toBeChecked();
  await expect(page.locator(".books-count")).toHaveText(countLine(paizoBooks.length - 1));
  // Books names the set Custom, and nothing is saved on the book rows themselves.
  await page.goto("#/books");
  await expect(page.locator(".books-count")).toHaveText(countLine(paizoBooks.length - 1, "Custom"));
});

installedTest("Each preset sets its books, the count line names it, and Undo restores the earlier set", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await page.goto("#/books/manage");
  await waitReady(page);
  // The corpus marks two books third party, so the fourth preset is distinct and offered.
  await expect(page.locator(".preset-row")).toHaveCount(presets.length);
  await expect(page.getByRole("button", { name: "Everything incl. 3rd party", exact: true })).toHaveCount(1);
  const row = (name: string) => page.getByRole("button", { name, exact: true });
  // A settled profile keeps every Paizo book active, which matches Paizo Everything.
  await expect(row("Paizo Everything")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator(".books-count")).toHaveText(countLine(paizoBooks.length));
  await page.goto("#/books");
  await expect(page.locator(".books-count")).toHaveText(countLine(paizoBooks.length, "Paizo Everything"));
  await page.goto("#/books/manage");

  for (const preset of presets) {
    await row(preset.name).click();
    await expect(page.getByRole("status").filter({ hasText: `Applied ${preset.name}.` })).toBeVisible();
    await expect(row(preset.name)).toHaveAttribute("aria-pressed", "true");
    await expect(row(preset.name)).toContainText(`${count(preset.ids.length)} books`);
    await expect(page.locator(".books-count")).toHaveText(countLine(preset.ids.length));
    // The checkboxes agree with the preset, book for book.
    await openBookGroups(page);
    const checked: string[] = await page
      .locator(".book-directory-item")
      .evaluateAll((items) =>
        items
          .filter((item) => (item.querySelector("input") as HTMLInputElement).checked)
          .map((item) => item.getAttribute("data-book-id") as string),
      );
    expect([...checked].sort(), preset.name).toEqual([...preset.ids].sort());
    // Books names the preset in its count line.
    await page.goto("#/books");
    await expect(page.locator(".books-count")).toHaveText(countLine(preset.ids.length, preset.name));
    await page.goto("#/books/manage");
  }

  // Undo: back to Paizo Rulebooks & More after applying Paizo Rulebooks.
  await row("Paizo Rulebooks & More").click();
  await row("Paizo Rulebooks").click();
  await expect(page.getByRole("status").filter({ hasText: "Applied Paizo Rulebooks." })).toBeVisible();
  await page.getByRole("button", { name: "Undo", exact: true }).click();
  await expect(page.getByRole("status").filter({ hasText: "Restored your earlier choice." })).toBeVisible();
  await expect(page.getByRole("button", { name: "Undo", exact: true })).toHaveCount(0);
  await expect(row("Paizo Rulebooks & More")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator(".books-count")).toHaveText(countLine(presets[1].ids.length));

  // A single change ends Undo, and the preset row is no longer pressed: the set is Custom.
  await row("Paizo Rulebooks").click();
  await expect(page.getByRole("button", { name: "Undo", exact: true })).toBeVisible();
  await openBookGroups(page);
  await page.locator(".book-directory-item").first().getByRole("checkbox").uncheck();
  await expect(page.getByRole("button", { name: "Undo", exact: true })).toHaveCount(0);
  await expect(page.locator('.preset-row[aria-pressed="true"]')).toHaveCount(0);
});

installedTest("A preset keeps downloads and stars, and changes what search finds", async ({ page }) => {
  const { id, entry } = adventureEntry();
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

  await page.goto("#/books/manage");
  await page.getByRole("button", { name: "Paizo Rulebooks", exact: true }).click();
  const adventureBooks = corpus.catalog.sources.filter((row) => adventureCategories.includes(row.category as string));
  await expect(page.locator('.book-tier[data-tier="Adventures"] .tier-active')).toHaveText(
    `0 of ${adventureBooks.length} active`,
  );
  await page.goto("#/search?q=fire");
  await waitResults(page, "fire");
  const rulebooksOnly = await resultCount(page);
  expect(rulebooksOnly).not.toBe(everything);

  // The starred entry and the download survive.
  await page.goto(`#/entry/${id}`);
  await expect(page.getByRole("heading", { name: entry.title, level: 1, exact: true })).toBeVisible();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator(".disabled-book-notice")).toBeVisible();
  expect(await cacheContents(page)).toEqual(downloads);
  await page.goto("#/");
  await expect(page.locator(".home-bookmarks").getByRole("link", { name: entry.title, exact: true })).toBeVisible();

  // Undo brings the Adventures back with no reload.
  await page.goto("#/books/manage");
  await page.getByRole("button", { name: "Paizo Everything", exact: true }).click();
  await expect(page.locator('.book-tier[data-tier="Adventures"] .tier-active')).toHaveText(
    `${adventureBooks.length} books`,
  );
  await page.goto("#/search?q=fire");
  await waitResults(page, "fire");
  expect(await resultCount(page)).toBe(everything);
  expect(await cacheContents(page)).toEqual(downloads);
});

installedTest("Two quick preset taps end on exactly the second preset", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/books/manage");
  await expect(page.locator(".preset-row").first()).toBeVisible();
  // Both taps land in one task, before the first write has finished. The second preset must be
  // planned after the first write, or a book that the first turned off stays off.
  await page.evaluate(() => {
    const rows = [...document.querySelectorAll<HTMLButtonElement>(".preset-row")];
    const named = (label: string) => rows.find((row) => row.getAttribute("aria-label") === label)!;
    named("Paizo Rulebooks").click();
    named("Paizo Rulebooks & More").click();
  });
  const second = page.getByRole("button", { name: "Paizo Rulebooks & More", exact: true });
  await expect(second).toHaveAttribute("aria-pressed", "true");
  // Wait for the writes to settle, then check the stored set rather than the screen alone.
  await page.goto("#/books");
  const more = presets[1];
  await expect(page.locator(".books-count")).toHaveText(countLine(more.ids.length, "Paizo Rulebooks & More"));
  await page.reload();
  await expect(page.locator(".books-count")).toHaveText(countLine(more.ids.length, "Paizo Rulebooks & More"));
});

test("A new install starts on Paizo Rulebooks", async ({ page }) => {
  await installLibrary(page, { freshBooks: true });
  const rulebooks = presets[0];
  await page.goto("#/books");
  await expect(page.locator(".books-count")).toHaveText(countLine(rulebooks.ids.length, "Paizo Rulebooks"));
  await expect(page.locator('.book-tier[data-tier="Rulebooks"] .tier-active')).toHaveText(
    `${rulebooks.ids.length} ${rulebooks.ids.length === 1 ? "book" : "books"}`,
  );
  await page.goto("#/books/manage");
  await expect(page.getByRole("button", { name: "Paizo Rulebooks", exact: true })).toHaveAttribute("aria-pressed", "true");
  // The default survives a reload and is written once: a later change is kept, not reset.
  const { book, source } = target();
  await openBookGroups(page);
  const toggle = page.locator(`.book-directory-item[data-book-id="${book.source}"]`)
    .getByRole("checkbox", { name: `Active ${source.title}`, exact: true });
  const wasActive = await toggle.isChecked();
  await toggle.setChecked(!wasActive);
  await page.reload();
  await waitReady(page);
  await openBookGroups(page);
  await expect(toggle).toBeChecked({ checked: !wasActive });
});

test("An install from before the default keeps every Paizo book active, with or without a stored choice", async ({ page }) => {
  // The helper settles the marker, the state of an existing profile. Removing the marker too is a
  // profile that never stored anything and has no marker at all: nothing may change for it. The
  // third-party books are the exception: #189 turns them off on the first load that sees them.
  await installLibrary(page);
  await waitControlled(page);
  await page.evaluate(() => new Promise<void>((resolve, reject) => {
    const request = indexedDB.open("cast-iron");
    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const db = request.result;
      const tx = db.transaction("meta", "readwrite");
      tx.objectStore("meta").delete("book_defaults");
      tx.oncomplete = () => { db.close(); resolve(); };
      tx.onerror = () => reject(tx.error);
    };
  }));
  await page.reload();
  await waitReady(page);
  await page.goto("#/books");
  await expect(page.locator(".books-count")).toHaveText(countLine(paizoBooks.length, "Paizo Everything"));
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
  await page.getByRole("link", { name: "Open Manage books" }).click();
  await expect(page.getByRole("heading", { name: "Manage books", exact: true })).toBeVisible();
  await expect(page.getByText("No books active", { exact: true })).toBeVisible();
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
  await page.goto("#/books/manage");
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
    await expect(page.getByTestId("source-count")).toHaveText(`${count(expected)} ${noun(expected)} in your active books`);

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

installedTest("The book page meta line is the book's size and survives deactivation", async ({ page }) => {
  // The line under the title is the same text as the book's row on Books: a month and year, and
  // the entries the catalog holds for the book, whether or not the book is active.
  const book = bookTrees().find((candidate) => candidate.chapters.length > 0)!;
  const source = corpus.catalog.sources.find((row) => row.id === book.source)!;
  const entries = corpus.catalog.entries.filter((stub) => (owners.get(stub.id) ?? stub.source) === book.source).length;

  await installLibrary(page);
  await waitControlled(page);
  await page.goto("#/books");
  await openBookGroups(page);
  const rowLine = (await page.locator(`.book-directory-item[data-book-id="${book.source}"] .cat-blurb`).innerText()).trim();
  expect(rowLine).toContain(`${count(entries)} ${noun(entries)}`);

  await page.goto(`#/browse/source/${encodeURIComponent(book.source)}`);
  await waitReady(page);
  await expect(page.getByRole("heading", { name: book.title, level: 1, exact: true })).toBeVisible();
  // The old publication address became the book's own.
  await expect(page).toHaveURL(new RegExp(`#/browse/book/${book.source}$`));
  await expect(page.getByTestId("source-count")).toHaveText(rowLine);
  await expect(page.locator(".book > .notice")).toHaveCount(0);
  await expect(page.locator(".toc-top")).toBeVisible();
  await expect(page.getByTestId("book-off-line")).toHaveCount(0);

  // Turn the book off on Manage books; the book page itself has no control.
  await page.goto("#/books/manage");
  await waitReady(page);
  await openBookGroups(page);
  const row = page.locator(`.book-directory-item[data-book-id="${book.source}"]`);
  const toggle = row.getByRole("checkbox", { name: `Active ${source.title}`, exact: true });
  await expect(toggle).toBeChecked();
  await toggle.uncheck();
  await expect(toggle).not.toBeChecked();

  await page.goto(`#/browse/book/${encodeURIComponent(book.source)}`);
  await expect(page.getByRole("heading", { name: book.title, level: 1, exact: true })).toBeVisible();
  // The contents stay readable, so the count line must not change when the book is disabled.
  await expect(page.getByTestId("source-count")).toHaveText(rowLine);
  await expect(page.getByTestId("book-off-line")).toHaveText(
    "This book is turned off. Its entries don't appear in search or lists.",
  );
  await expect(page.locator(".book-active-row")).toHaveCount(0);
  await expect(page.locator(".toc-top")).toBeVisible();
});

installedTest("A search hint offers to include an inactive book without changing the set by itself", async ({ page }) => {
  const { entry, book, source } = target();
  await installLibrary(page);
  await waitControlled(page);
  await page.goto("#/books/manage");
  await openBookGroups(page);
  const toggle = page.locator(`.book-directory-item[data-book-id="${book.source}"]`)
    .getByRole("checkbox", { name: `Active ${source.title}`, exact: true });
  await toggle.uncheck();
  await expect(toggle).not.toBeChecked();

  await page.goto("#/search");
  await page.locator("#search-input").fill(entry.title);
  await waitResults(page, entry.title);
  const hint = page.locator(".inactive-books-hint");
  await expect(hint).toContainText(`inactive books (${source.title})`);
  // Reading the hint changes nothing: the book is still inactive in the stored set.
  await page.goto("#/books/manage");
  await openBookGroups(page);
  await expect(page.locator(`.book-directory-item[data-book-id="${book.source}"]`)
    .getByRole("checkbox", { name: `Active ${source.title}`, exact: true })).not.toBeChecked();

  // The one-tap action is the reader asking, so it includes the book and the entry appears.
  await page.goto("#/search");
  await page.locator("#search-input").fill(entry.title);
  await waitResults(page, entry.title);
  await page.locator(".inactive-books-hint")
    .getByRole("button", { name: `Turn on ${source.title}`, exact: true }).click();
  // Exact id (or id plus an anchor): a nested add-on such as `<id>/mythic` shares the prefix.
  await expect(page.locator(`.results a[href="#/entry/${entry.id}"], .results a[href^="#/entry/${entry.id}#"]`)).toBeVisible();
  await page.goto("#/books/manage");
  await openBookGroups(page);
  await expect(page.locator(`.book-directory-item[data-book-id="${book.source}"]`)
    .getByRole("checkbox", { name: `Active ${source.title}`, exact: true })).toBeChecked();
});
