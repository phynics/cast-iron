import fs from "node:fs";
import path from "node:path";
import { test, installedTest, expect, loadCorpus, ROOT } from "./fixtures";
import { count, expectPreinstalled, installLibrary, openBookGroups, openQuickSearch, quickSearch, waitReady } from "./helpers";

/**
 * Share the active book set as a code (#408). The page reads and writes a code through Rust; this
 * spec states the format a second time, in TypeScript, from `docs/decisions.md` and the module
 * doc of `crates/web/src/book_code.rs`, so it checks the app against the written format and not
 * against itself.
 */

const corpus = loadCorpus();
const allBooks: string[] = corpus.catalog.sources.map((row: any) => row.id);
const thirdParty = new Set<string>(
  corpus.catalog.sources.filter((row: any) => row.third_party).map((row: any) => row.id as string),
);
const paizoBooks = allBooks.filter((id) => !thirdParty.has(id));
const rulebooks = corpus.catalog.sources
  .filter((row: any) => !row.third_party && row.category === "RPG")
  .map((row: any) => row.id as string);

/** The permanent slot of every book: its line among the id lines of `published-books.txt`. */
const slotFile = fs
  .readFileSync(path.join(ROOT, "sources/published-books.txt"), "utf8")
  .split("\n")
  .filter((line) => line !== "" && !line.startsWith("#"));
const slotOf = new Map(slotFile.map((id, index) => [id, index]));
const idOfSlot = new Map(slotFile.map((id, index) => [index, id]));
/** The slot file can reserve slots ahead of the pinned library. A code describes the slots up to the highest one the library holds. */
const librarySlots = Math.max(...corpus.catalog.sources.map((row: any) => row.share_slot as number)) + 1;

const more = corpus.catalog.sources
  .filter((row: any) => !row.third_party && ["RPG", "Player Companion", "Campaign Setting"].includes(row.category))
  .map((row: any) => row.id as string);

const countLine = (active: number, name?: string) =>
  `${count(active)} of ${count(allBooks.length)} books active` + (name ? ` · ${name}` : "");

/** The count line of Manage books: it names no preset. */
const activeLine = (active: number) => `${count(active)} of ${count(allBooks.length)} books active`;

// ---- the format, again, in TypeScript -------------------------------------------------------

const ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

function checkValue(values: number[]): number {
  let weight = 1;
  let sum = 0;
  for (const value of values) {
    sum = (sum + (value + 1) * weight) % 32;
    weight = (weight * 3) % 32;
  }
  return sum;
}

function gamma(n: number): boolean[] {
  const binary = n.toString(2);
  return [...Array(binary.length - 1).fill(false), ...[...binary].map((bit) => bit === "1")];
}

/** A code in bit mode with a given version. The app also reads the run mode; it writes the shorter. */
function bitCode(described: number, on: Set<number>, version = 1): string {
  const bits = [true, ...gamma(described + 1)];
  for (let slot = 0; slot < described; slot++) bits.push(on.has(slot));
  const values: number[] = [version];
  for (let at = 0; at < bits.length; at += 5) {
    let value = 0;
    for (let i = 0; i < 5; i++) value = (value << 1) | (bits[at + i] ? 1 : 0);
    values.push(value);
  }
  values.push(checkValue(values));
  return values.map((value) => ALPHABET[value]).join("");
}

/** The slots a code turns on, read from the written format. */
function readCode(text: string): { described: number; on: Set<number> } {
  const values = [...text.toUpperCase().replace(/[\s-]/g, "").replace(/[IL]/g, "1").replace(/O/g, "0")].map(
    (character) => {
      const value = ALPHABET.indexOf(character);
      if (value < 0) throw new Error(`not a code character: ${character}`);
      return value;
    },
  );
  const last = values.pop()!;
  expect(checkValue(values), "the check character").toBe(last);
  expect(values[0], "the version").toBe(1);
  const bits = values.slice(1).flatMap((value) => [4, 3, 2, 1, 0].map((shift) => ((value >> shift) & 1) === 1));
  let at = 0;
  const next = () => bits[at++];
  const readGamma = () => {
    let zeros = 0;
    while (!next()) zeros++;
    let n = 1;
    for (let i = 0; i < zeros; i++) n = (n << 1) | (next() ? 1 : 0);
    return n;
  };
  const on = new Set<number>();
  let described = 0;
  if (next()) {
    described = readGamma() - 1;
    for (let slot = 0; slot < described; slot++) if (next()) on.add(slot);
  } else {
    let onRun = false;
    let first = true;
    const padding = () => bits.length - at < 5 && bits.slice(at).every((bit) => !bit);
    while (first || !padding()) {
      const n = readGamma();
      const length = first ? n - 1 : n;
      first = false;
      if (onRun) for (let slot = described; slot < described + length; slot++) on.add(slot);
      described += length;
      onRun = !onRun;
    }
  }
  return { described, on };
}

/** The ids a code turns on, in a library with these books. */
const idsOf = (text: string) => [...readCode(text).on].map((slot) => idOfSlot.get(slot)!).sort();

/** The code for a set of ids, in bit mode. */
const codeFor = (ids: string[], version = 1) =>
  bitCode(slotFile.length, new Set(ids.map((id) => slotOf.get(id)!)), version);

// ---- the page --------------------------------------------------------------------------------

const countText = (page: any) => page.locator(".books-count");
const disclosure = (page: any) => page.locator(".share-set");

async function openShare(page: any) {
  await visit(page, "#/books/manage");
  await waitReady(page);
  const details = disclosure(page);
  await expect(details).toBeVisible();
  if (!(await details.evaluate((element: HTMLDetailsElement) => element.open))) {
    await details.locator("summary").click();
  }
  await expect(details.getByTestId("share-code")).toBeVisible();
}

const shownCode = async (page: any) => (await page.getByTestId("share-code").innerText()).trim();

/**
 * Goes to a route of the app unless the page is already there. WebKit reloads the document when
 * `page.goto` names the URL the page has (Chromium treats it as a no-op), and a reload drops the
 * in-memory Undo and costs a whole start-up. A test that wants a reload says `page.reload()`.
 */
async function visit(page: any, hash: string) {
  if (new URL(page.url()).hash === hash) return;
  await page.goto(hash);
}

/** The ids that Manage books shows as checked. */
async function checkedBooks(page: any): Promise<string[]> {
  await visit(page, "#/books/manage");
  await waitReady(page);
  await openBookGroups(page);
  const ids: string[] = await page
    .locator(".book-directory-item")
    .evaluateAll((items: Element[]) =>
      items
        .filter((item) => (item.querySelector("input") as HTMLInputElement).checked)
        .map((item) => item.getAttribute("data-book-id") as string),
    );
  return ids.sort();
}

async function enterCode(page: any, text: string) {
  await page.getByLabel("Enter a code", { exact: true }).fill(text);
  await page.getByRole("button", { name: "Review", exact: true }).click();
}

/** Opens the section, if closed, and reviews a code. */
async function enterCodeOpen(page: any, text: string) {
  const details = disclosure(page);
  if (!(await details.evaluate((element: HTMLDetailsElement) => element.open))) {
    await details.locator("summary").click();
  }
  await enterCode(page, text);
}

async function applyPreset(page: any, name: string) {
  await visit(page, "#/books/manage");
  await waitReady(page);
  await page.getByRole("button", { name, exact: true }).click();
  await expect(page.getByRole("status").filter({ hasText: `Applied ${name}.` })).toBeVisible();
}

const shareUndo = (page: any) => page.locator(".share-status").getByRole("button", { name: "Undo", exact: true });
const presetUndo = (page: any) =>
  page.locator('section[aria-labelledby="presets-heading"]').getByRole("button", { name: "Undo", exact: true });
const usePreset = (page: any, name: string) => page.getByRole("button", { name, exact: true });
const applied = (page: any, name: string) => page.getByRole("status").filter({ hasText: `Applied ${name}.` });

/**
 * Holds every save of the book choice that starts after the call, until `release()`. The save is
 * the browser's own: the page's transaction stays open on a chain of reads, so the write is half
 * done (the app's earlier-write flag is up, its revision has not moved) with no timer to race.
 * `release({ fail: true })` aborts the held transactions instead, as a store that gave up does.
 * `writes()` counts every choice that was saved since the call, the ones that start after the
 * release included.
 */
async function holdWrites(page: any) {
  await page.evaluate(() => {
    const w = window as any;
    // The browser's own `put`, kept across calls: a later call replaces the wrapper of the
    // earlier one instead of stacking on it, so a write is counted once.
    w.__put ??= IDBObjectStore.prototype.put;
    const put = w.__put;
    let open = true;
    w.__held = 0;
    w.__saved = 0;
    w.__held_transactions = [];
    w.__release = (fail: boolean) => {
      open = false;
      if (fail) for (const transaction of w.__held_transactions) transaction.abort();
    };
    IDBObjectStore.prototype.put = function (value: unknown, key?: IDBValidKey) {
      const request = put.call(this, value, key);
      if (key === "disabled_books") {
        w.__saved++;
        if (open) {
          w.__held++;
          w.__held_transactions.push(this.transaction);
          const store = this;
          const keep = () => {
            if (open) store.get(key as IDBValidKey).onsuccess = keep;
          };
          keep();
        }
      }
      return request;
    };
  });
  return {
    held: (n: number) => page.waitForFunction((count: number) => (window as any).__held >= count, n),
    release: ({ fail = false } = {}) => page.evaluate((fail: boolean) => (window as any).__release(fail), fail),
    writes: (): Promise<number> => page.evaluate(() => (window as any).__saved),
  };
}

/**
 * From now on the browser aborts the transaction of every save of the book choice as soon as the
 * save is requested, before the request answers: a store that gave up while the write was pending.
 * The page's request fails first, so the save returns before it awaits the transaction.
 */
async function abortWrites(page: any) {
  await page.evaluate(() => {
    const put = IDBObjectStore.prototype.put;
    IDBObjectStore.prototype.put = function (value: unknown, key?: IDBValidKey) {
      const request = put.call(this, value, key);
      if (key === "disabled_books") this.transaction.abort();
      return request;
    };
  });
}

/**
 * Waits a moment after a write that was queued behind another one has had its turn. The app
 * starts a queued write within a few milliseconds of the slot freeing, and a write that runs
 * opens its transaction at once; this one is created after the 100 ms wait, so it completes after
 * any such write has committed. This is a heuristic, not an observation: nothing tells the test
 * that the queued write started. A test that needs to say a write did NOT happen pairs it with
 * `hold.writes()`, which keeps counting after the release, and with the final selection.
 */
async function afterQueuedWrites(page: any) {
  await page.evaluate(
    () =>
      new Promise<void>((resolve, reject) => {
        setTimeout(() => {
          const open = indexedDB.open("cast-iron");
          open.onerror = () => reject(open.error);
          open.onsuccess = () => {
            const db = open.result;
            const tx = db.transaction("meta", "readwrite");
            tx.objectStore("meta").get("disabled_books");
            tx.oncomplete = () => {
              db.close();
              resolve();
            };
            tx.onerror = () => reject(tx.error);
          };
        }, 100);
      }),
  );
}

/** From now on the browser refuses to save the book choice, as a full or locked store does. */
async function refuseWrites(page: any) {
  await page.evaluate(() => {
    const w = window as any;
    const put = IDBObjectStore.prototype.put;
    w.__allow = () => {
      IDBObjectStore.prototype.put = put;
    };
    IDBObjectStore.prototype.put = function (value: unknown, key?: IDBValidKey) {
      if (key === "disabled_books") throw new DOMException("The store refused the write.", "QuotaExceededError");
      return put.call(this, value, key);
    };
  });
  return { allow: () => page.evaluate(() => (window as any).__allow()) };
}

test("The slots in the catalog are the lines of published-books.txt", async () => {
  expect(slotFile.length).toBeGreaterThanOrEqual(allBooks.length);
  expect(new Set(slotFile).size).toBe(slotFile.length);
  for (const row of corpus.catalog.sources) {
    expect(row.share_slot, row.id).toBe(slotOf.get(row.id));
  }
});

installedTest("Manage books shows the code of the active set, and Copy puts it on the clipboard", async ({ page, context, browserName }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks & More");
  // One more change, so the set is a custom one.
  const extra = rulebooks[rulebooks.length - 1];
  await visit(page, "#/books/manage");
  await openBookGroups(page);
  const row = page.locator(`.book-directory-item[data-book-id="${extra}"]`).getByRole("checkbox");
  await row.uncheck();
  await expect(row).not.toBeChecked();

  await openShare(page);
  const code = await shownCode(page);
  // Groups of four after the version, uppercase Crockford, a check character at the end.
  expect(code).toMatch(/^[0-9A-HJKMNP-TV-Z]{1,4}(-[0-9A-HJKMNP-TV-Z]{1,4})*$/);
  expect(code.replace(/-/g, "")[0]).toBe("1");
  // Read by the written format, the code names the same books that Manage books shows as on.
  expect(idsOf(code)).toEqual(await checkedBooks(page));
  expect(readCode(code).described).toBe(librarySlots);
  await openShare(page);

  // The open section shows the one disclosure chevron of the app, turned down: once the 0.12 s
  // turn is over, the same quarter turn that every other open disclosure ends on.
  const chevron = disclosure(page).locator("summary > .disclosure-chevron");
  await expect(chevron).toHaveCount(1);
  await expect
    .poll(() => chevron.evaluate((element: Element) => getComputedStyle(element).transform))
    .toBe("matrix(0, 1, -1, 0, 0, 0)");
  await disclosure(page).locator("summary").click();
  await expect
    .poll(() => chevron.evaluate((element: Element) => getComputedStyle(element).transform))
    .toBe("none");
  await openShare(page);

  // Every control is a 44 px target; nothing scrolls sideways.
  const targets = [
    disclosure(page).locator("summary"),
    page.getByRole("button", { name: "Copy code", exact: true }),
    page.getByRole("button", { name: "Copy link", exact: true }),
    page.getByLabel("Enter a code", { exact: true }),
    page.getByRole("button", { name: "Review", exact: true }),
  ];
  for (const target of targets) expect((await target.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);

  if (browserName === "chromium") {
    await context.grantPermissions(["clipboard-read", "clipboard-write"]);
    await page.getByRole("button", { name: "Copy code", exact: true }).click();
    await expect(page.getByRole("status").filter({ hasText: "Copied the code." })).toBeVisible();
    expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(code);
    await page.getByRole("button", { name: "Copy link", exact: true }).click();
    await expect(page.getByRole("status").filter({ hasText: "Copied the link." })).toBeVisible();
    const link = await page.evaluate(() => navigator.clipboard.readText());
    expect(link).toBe(`${new URL(page.url()).origin}/cast-iron/#/books/manage?set=${code}`);
    // A link opened on this page asks a new question, so the note about the old code goes.
    await page.goto(`#/books/manage?set=${codeFor(rulebooks)}`);
    await expect(page.getByRole("heading", { name: "Use this set of books?", exact: true })).toBeVisible();
    await expect(page.getByText("Copied the link.")).toHaveCount(0);
    await expect(page.getByText("Copied the code.")).toHaveCount(0);
  }
});

installedTest("A code made on one profile gives a fresh profile the exact same set", async ({ page, browser, baseURL, guard }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks & More");
  const dropped = rulebooks[0];
  await visit(page, "#/books/manage");
  await openBookGroups(page);
  await page.locator(`.book-directory-item[data-book-id="${dropped}"]`).getByRole("checkbox").uncheck();
  const expected = await checkedBooks(page);
  expect(expected).not.toContain(dropped);
  await openShare(page);
  const code = await shownCode(page);

  // A fresh browser profile: no library, no choice. It installs with the first-run default.
  const fresh = await browser.newContext({ baseURL, serviceWorkers: "allow", viewport: page.viewportSize() });
  guard.watch(fresh);
  try {
    const other = await fresh.newPage();
    // This profile really is new, so it installs the library: the pre-installed guard stands down.
    expectPreinstalled(false);
    await installLibrary(other, { freshBooks: true });
    expectPreinstalled(true);
    await other.goto("#/books");
    await waitReady(other);
    await expect(countText(other)).toHaveText(countLine(rulebooks.length, "Paizo Rulebooks"));
    // Books has no share section; Manage books has it.
    await expect(disclosure(other)).toHaveCount(0);
    await other.goto("#/books/manage");
    await waitReady(other);
    await expect(countText(other)).toHaveText(activeLine(rulebooks.length));
    await disclosure(other).locator("summary").click();
    await enterCode(other, code);
    await expect(other.getByRole("heading", { name: "Use this set of books?", exact: true })).toBeVisible();
    // Nothing has changed yet.
    await expect(countText(other)).toHaveText(activeLine(rulebooks.length));
    await other.getByRole("button", { name: "Use this set", exact: true }).click();
    await expect(countText(other)).toHaveText(activeLine(expected.length));
    expect(await checkedBooks(other)).toEqual(expected);
    // The other profile's code is now the same code.
    await openShare(other);
    expect(await shownCode(other)).toBe(code);
  } finally {
    await fresh.close();
  }
});

installedTest("A link asks first, applies on confirmation, and Undo restores the earlier set", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  const code = codeFor(paizoBooks);
  await page.goto(`#/books/manage?set=${code}`);
  await waitReady(page);

  const heading = page.getByRole("heading", { name: "Use this set of books?", exact: true });
  await expect(heading).toBeVisible();
  await expect(heading).toBeFocused();
  const turnsOn = paizoBooks.length - rulebooks.length;
  await expect(page.getByTestId("share-summary")).toHaveText(`Turns on ${count(turnsOn)} books.`);
  await expect(page.getByTestId("share-result")).toHaveText(
    `Afterwards: ${countLine(paizoBooks.length, "Paizo Everything")}`,
  );
  await expect(page.getByTestId("share-skipped")).toHaveCount(0);
  // The link only asks: the books and the address are as they were.
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  await expect(page).toHaveURL(/#\/books\/manage\?set=/);
  // Nothing is saved by opening the link, so a reload asks again.
  await page.reload();
  await waitReady(page);
  await expect(heading).toBeVisible();
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));

  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));
  await expect(heading).toHaveCount(0);
  await expect(page).toHaveURL(/#\/books\/manage$/);
  // The status names the preset the set made.
  await expect(page.getByRole("status").filter({ hasText: "Applied Paizo Everything." })).toBeVisible();

  await page.getByRole("button", { name: "Undo", exact: true }).click();
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  await expect(page.getByRole("status").filter({ hasText: "Restored your earlier choice." })).toBeVisible();
  await expect(page.getByRole("button", { name: "Undo", exact: true })).toHaveCount(0);
  // The answer survives a reload: it was written once.
  await page.reload();
  await waitReady(page);
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
});

installedTest("A link made when the section sat on Books still opens the question on Manage books", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  await visit(page, "#/settings");
  await expect(page.getByRole("heading", { level: 1, name: "Settings", exact: true })).toBeVisible();
  await page.goto(`#/books?set=${codeFor(paizoBooks)}`);
  await waitReady(page);
  const heading = page.getByRole("heading", { name: "Use this set of books?", exact: true });
  await expect(heading).toBeVisible();
  await expect(heading).toBeFocused();
  await expect(page.getByRole("heading", { level: 1, name: "Manage books", exact: true })).toBeVisible();
  await expect(page).toHaveURL(/#\/books\/manage\?set=/);
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));
  await expect(page).toHaveURL(/#\/books\/manage$/);
  // Done leaves for Books, which names the set the link made.
  await page.getByRole("button", { name: "Done", exact: true }).click();
  await expect(page).toHaveURL(/#\/books$/);
  await expect(countText(page)).toHaveText(countLine(paizoBooks.length, "Paizo Everything"));
});

installedTest("Books has no share section, and its first group starts under the Manage books link", async ({ page }) => {
  await installLibrary(page);
  await visit(page, "#/books");
  await waitReady(page);
  await expect(disclosure(page)).toHaveCount(0);
  await expect(page.getByText("Share these books or enter a code")).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Manage books", exact: true })).toBeVisible();
  // The count line, the link, then the first group: nothing between them.
  const order = await page.locator(".wide").evaluate((wide: Element) => {
    const children = [...wide.children];
    const index = (selector: string) => children.findIndex((child) => child.matches(selector));
    return { count: index(".books-count"), link: index(".filter-row"), group: index("section, .tier-group, h2") };
  });
  expect(order.count).toBeGreaterThan(0);
  expect(order.link).toBe(order.count + 1);
  expect(order.group).toBe(order.link + 1);
});

installedTest("Another change on Manage books ends the share Undo, and a share ends the preset Undo", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  const undo = page.getByRole("button", { name: "Undo", exact: true });
  await enterCodeOpen(page, codeFor(paizoBooks));
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(undo).toHaveCount(1);
  await expect(shareUndo(page)).toBeVisible();
  await expect(applied(page, "Paizo Everything")).toBeVisible();
  // A preset replaces the share's Undo: one Undo line at a time.
  await usePreset(page, "Paizo Rulebooks").click();
  await expect(applied(page, "Paizo Rulebooks")).toBeVisible();
  await expect(undo).toHaveCount(1);
  await expect(presetUndo(page)).toBeVisible();
  await expect(shareUndo(page)).toHaveCount(0);
  // The Undo that is left restores the set before the preset, which was the shared one.
  await presetUndo(page).click();
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));
  await expect(page.getByRole("status").filter({ hasText: "Restored your earlier choice." })).toBeVisible();
  await expect(undo).toHaveCount(0);

  // A share replaces the preset's Undo, and the share's own Undo restores the set before it.
  await usePreset(page, "Paizo Rulebooks").click();
  await expect(presetUndo(page)).toBeVisible();
  await enterCodeOpen(page, codeFor(paizoBooks));
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(undo).toHaveCount(1);
  await expect(shareUndo(page)).toBeVisible();
  await expect(presetUndo(page)).toHaveCount(0);
  await expect(applied(page, "Paizo Everything")).toBeVisible();
  await expect(applied(page, "Paizo Rulebooks")).toHaveCount(0);
  await shareUndo(page).click();
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  await expect(page.getByRole("status").filter({ hasText: "Restored your earlier choice." })).toBeVisible();
  await expect(undo).toHaveCount(0);

  // A checkbox ends the share's Undo too, once its write has landed.
  await enterCodeOpen(page, codeFor(paizoBooks));
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(shareUndo(page)).toBeVisible();
  await openBookGroups(page);
  const extra = paizoBooks.find((id) => !rulebooks.includes(id))!;
  await page.locator(`.book-directory-item[data-book-id="${extra}"]`).getByRole("checkbox").uncheck();
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length - 1));
  await expect(undo).toHaveCount(0);
});

installedTest("An Undo that was clicked while another change was saving does nothing", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");

  // A share Undo against a preset that is saving.
  await enterCodeOpen(page, codeFor(paizoBooks));
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));
  await expect(shareUndo(page)).toBeVisible();
  let hold = await holdWrites(page);
  await usePreset(page, "Paizo Rulebooks & More").click();
  await hold.held(1);
  // The preset is saving; the share's Undo is still on screen, and the reader clicks it.
  await expect(shareUndo(page)).toBeVisible();
  await shareUndo(page).click();
  await hold.release();
  await expect(countText(page)).toHaveText(activeLine(more.length));
  await expect(applied(page, "Paizo Rulebooks & More")).toBeVisible();
  await afterQueuedWrites(page);
  // The Undo was left out: it would have turned off the books that the preset kept on.
  expect(await hold.writes()).toBe(1);
  await expect(countText(page)).toHaveText(activeLine(more.length));
  await expect(shareUndo(page)).toHaveCount(0);
  await expect(page.getByRole("status").filter({ hasText: "Restored your earlier choice." })).toHaveCount(0);
  expect(await checkedBooks(page)).toEqual([...more].sort());

  // A preset Undo against a share that is saving. The preset line holds the Undo to Everything.
  await openShare(page);
  await expect(presetUndo(page)).toBeVisible();
  hold = await holdWrites(page);
  await enterCode(page, codeFor(rulebooks));
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await hold.held(1);
  await expect(presetUndo(page)).toBeVisible();
  await presetUndo(page).click();
  await hold.release();
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  await expect(applied(page, "Paizo Rulebooks")).toBeVisible();
  await afterQueuedWrites(page);
  expect(await hold.writes()).toBe(1);
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  await expect(presetUndo(page)).toHaveCount(0);
  expect(await checkedBooks(page)).toEqual([...rulebooks].sort());

  // Undo twice in a row, as a double tap does: the second one finds the set already restored.
  await usePreset(page, "Paizo Rulebooks & More").click();
  await expect(presetUndo(page)).toBeVisible();
  hold = await holdWrites(page);
  await presetUndo(page).click();
  await hold.held(1);
  await presetUndo(page).click();
  await hold.release();
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  await afterQueuedWrites(page);
  expect(await hold.writes()).toBe(1);
});

installedTest("A write that fails leaves the Undo that is on screen", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  await enterCodeOpen(page, codeFor(paizoBooks));
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));
  await expect(shareUndo(page)).toBeVisible();
  await openBookGroups(page);
  const extra = paizoBooks.find((id) => !rulebooks.includes(id))!;
  const box = page.locator(`.book-directory-item[data-book-id="${extra}"]`).getByRole("checkbox");
  const failed = page.getByRole("alert").filter({ hasText: "Could not save the book selection" });

  const store = await refuseWrites(page);
  // A checkbox that cannot be saved goes back, says why, and ends nothing.
  await box.click();
  await expect(failed).toBeVisible();
  await expect(box).toBeChecked();
  await expect(shareUndo(page)).toBeVisible();
  // So does a preset.
  await usePreset(page, "Paizo Rulebooks").click();
  await expect(failed).toBeVisible();
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));
  await expect(shareUndo(page)).toBeVisible();
  // And a share.
  await enterCodeOpen(page, codeFor(rulebooks));
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(page.getByTestId("share-notice")).toContainText("Could not save the book selection");
  await expect(shareUndo(page)).toBeVisible();
  // The Undo itself fails and stays.
  await shareUndo(page).click();
  await expect(page.getByTestId("share-notice")).toContainText("Could not save the book selection");
  await expect(shareUndo(page)).toBeVisible();

  // The store works again: the Undo that was kept restores the set.
  await store.allow();
  await shareUndo(page).click();
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  await expect(page.getByRole("status").filter({ hasText: "Restored your earlier choice." })).toBeVisible();
  await expect(shareUndo(page)).toHaveCount(0);
});

installedTest("Back and Forward take a share link's question with them, and Done follows the entry it leaves", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  const code = codeFor(paizoBooks);
  const question = page.getByRole("heading", { name: "Use this set of books?", exact: true });
  const use = page.getByRole("button", { name: "Use this set", exact: true });
  const manage = page.getByRole("link", { name: "Manage books", exact: true });

  // Books, then the Manage books link (the first entry), then a share link in the same tab.
  await visit(page, "#/books");
  await waitReady(page);
  await manage.click();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await page.goto(`#/books/manage?set=${code}`);
  await expect(question).toBeVisible();
  await expect(page).toHaveURL(/#\/books\/manage\?set=/);

  // Back to the entry before the link: the address is the plain one, and the question is gone
  // with the link. It cannot be answered from here.
  await page.goBack();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await expect(question).toHaveCount(0);
  await expect(use).toHaveCount(0);
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  // Forward to the link's entry asks again, because its address still carries the code.
  await page.goForward();
  await expect(page).toHaveURL(/#\/books\/manage\?set=/);
  await expect(question).toBeVisible();
  await expect(use).toBeVisible();
  await page.goBack();
  await expect(question).toHaveCount(0);

  // A bad link's error belongs to its entry as well.
  await page.goto(`#/books/manage?set=${code.slice(0, -1)}`);
  await expect(page.getByTestId("share-notice")).toBeVisible();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await page.goBack();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await expect(page.getByTestId("share-notice")).toHaveCount(0);

  // The link again, from the first entry, answered. Done leaves this entry for Books in place
  // of going Back to the first entry, which would show the same page again.
  await page.goto(`#/books/manage?set=${code}`);
  await expect(question).toBeVisible();
  await use.click();
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await page.getByRole("button", { name: "Done", exact: true }).click();
  await expect(page).toHaveURL(/#\/books$/);
  await expect(page.getByRole("heading", { level: 1, name: "Books", exact: true })).toBeVisible();
  // Focus is back on the link that Done's page was opened from.
  await expect(manage).toBeFocused();
  await expect(countText(page)).toHaveText(countLine(paizoBooks.length, "Paizo Everything"));
  // The entry before Books is the first Manage books entry: Done did not use it.
  await page.goBack();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await expect(page.getByRole("heading", { level: 1, name: "Manage books", exact: true })).toBeVisible();

  // On the first entry, which the Books link pushed, Done still goes Back to Books.
  await page.getByRole("button", { name: "Done", exact: true }).click();
  await expect(page).toHaveURL(/#\/books$/);
  await expect(manage).toBeFocused();
  await page.goForward();
  await expect(page).toHaveURL(/#\/books\/manage$/);
});

installedTest("Leaving Manage books while a share is saving breaks nothing and closes no other question", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  const question = page.getByRole("heading", { name: "Use this set of books?", exact: true });

  // The reader answers a link, then leaves the page before the write is done.
  await page.goto(`#/books/manage?set=${codeFor(paizoBooks)}`);
  await expect(question).toBeVisible();
  let hold = await holdWrites(page);
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await hold.held(1);
  await visit(page, "#/settings");
  await expect(page.getByRole("heading", { level: 1, name: "Settings", exact: true })).toBeVisible();
  await hold.release();
  // The books were saved, and the page that is on screen did not stop. The fixture also fails the
  // test on any error the page logged, which a panic would be.
  await visit(page, "#/books/manage");
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));
  await expect(page.getByRole("heading", { level: 1, name: "Manage books", exact: true })).toBeVisible();
  await expect(page).toHaveURL(/#\/books\/manage$/);

  // The same for Undo.
  await enterCodeOpen(page, codeFor(rulebooks));
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(shareUndo(page)).toBeVisible();
  hold = await holdWrites(page);
  await shareUndo(page).click();
  await hold.held(1);
  await visit(page, "#/settings");
  await expect(page.getByRole("heading", { level: 1, name: "Settings", exact: true })).toBeVisible();
  await hold.release();
  await visit(page, "#/books/manage");
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));

  // A write that finishes after the reader opened another link must not close that link's
  // question or take its code out of the address.
  await page.goto(`#/books/manage?set=${codeFor(rulebooks)}`);
  await expect(question).toBeVisible();
  hold = await holdWrites(page);
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await hold.held(1);
  const other = codeFor(more);
  await page.goto(`#/books/manage?set=${other}`);
  await expect(question).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`#/books/manage\\?set=${other}$`));
  await hold.release();
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  await expect(applied(page, "Paizo Rulebooks")).toBeVisible();
  await afterQueuedWrites(page);
  // The second link's question is still there, and so is its code.
  await expect(question).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`#/books/manage\\?set=${other}$`));
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(countText(page)).toHaveText(activeLine(more.length));
  await expect(page).toHaveURL(/#\/books\/manage$/);
});

installedTest("A modified click on Manage books marks no entry: Done from Settings goes to Books", async ({ page, context }) => {
  await installLibrary(page);
  await visit(page, "#/books");
  await waitReady(page);
  const manage = page.getByRole("link", { name: "Manage books", exact: true });
  // Ctrl-click (Cmd on a Mac) opens the page in another tab. This tab stays on Books and pushes
  // no entry, so nothing about it may reach the next time Manage books opens here.
  const opened = context.waitForEvent("page", { timeout: 15_000 });
  await manage.click({ modifiers: ["ControlOrMeta"] });
  await (await opened).close();
  await expect(page).toHaveURL(/#\/books$/);

  // From Settings the link pushes an entry whose Back is Settings: Done must not use it.
  await visit(page, "#/settings");
  await page.getByRole("link", { name: "Manage books", exact: true }).click();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await page.getByRole("button", { name: "Done", exact: true }).click();
  await expect(page).toHaveURL(/#\/books$/);
  await expect(page.getByRole("heading", { level: 1, name: "Books", exact: true })).toBeVisible();
  // Done replaced the entry: Back from Books is Settings, and Forward has nothing.
  await page.goBack();
  await expect(page).toHaveURL(/#\/settings$/);

  // The same for a share link opened from Settings: no stale request marks it either.
  await visit(page, "#/books");
  await waitReady(page);
  const again = context.waitForEvent("page", { timeout: 15_000 });
  await manage.click({ modifiers: ["ControlOrMeta"] });
  await (await again).close();
  await visit(page, "#/settings");
  await page.goto(`#/books/manage?set=${codeFor(more)}`);
  await expect(page.getByRole("heading", { name: "Use this set of books?", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await page.getByRole("button", { name: "Done", exact: true }).click();
  await expect(page).toHaveURL(/#\/books$/);
  await page.goBack();
  await expect(page).toHaveURL(/#\/settings$/);

  // A plain click still pushes an entry that Done goes Back from, and Books gets its focus back.
  // The mark is in the entry's history state, so a reload of Manage books keeps it.
  await visit(page, "#/books");
  await waitReady(page);
  await manage.click();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await page.reload();
  await waitReady(page);
  await page.getByRole("button", { name: "Done", exact: true }).click();
  await expect(page).toHaveURL(/#\/books$/);
  await expect(manage).toBeFocused();
  await page.goForward();
  await expect(page).toHaveURL(/#\/books\/manage$/);
});

installedTest("A save that finishes late closes only the review it answered", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  const question = page.getByRole("heading", { name: "Use this set of books?", exact: true });
  const use = page.getByRole("button", { name: "Use this set", exact: true });
  const field = page.getByLabel("Enter a code", { exact: true });

  // Code A is saving when the reader pastes code B: B's question and text stay when A lands.
  await enterCodeOpen(page, codeFor(paizoBooks));
  await expect(question).toBeVisible();
  const hold = await holdWrites(page);
  await use.click();
  await hold.held(1);
  const other = codeFor(more);
  await enterCode(page, other);
  await expect(question).toBeVisible();
  await expect(page.getByTestId("share-summary")).toBeVisible();
  await hold.release();
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));
  await expect(applied(page, "Paizo Everything")).toBeVisible();
  await afterQueuedWrites(page);
  expect(await hold.writes()).toBe(1);
  await expect(question).toBeVisible();
  await expect(field).toHaveValue(other);
  // B is answerable, and it is B that is answered.
  await use.click();
  await expect(countText(page)).toHaveText(activeLine(more.length));
  await expect(question).toHaveCount(0);
  await expect(field).toHaveValue("");
});

installedTest("A save that finishes late keeps a code the reader pasted and has not reviewed", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  const question = page.getByRole("heading", { name: "Use this set of books?", exact: true });
  const use = page.getByRole("button", { name: "Use this set", exact: true });
  const field = page.getByLabel("Enter a code", { exact: true });
  await enterCodeOpen(page, codeFor(paizoBooks));
  await expect(question).toBeVisible();
  const hold = await holdWrites(page);
  await use.click();
  await hold.held(1);
  // B is pasted, not submitted.
  const other = codeFor(more);
  await field.fill(other);
  await hold.release();
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));
  await afterQueuedWrites(page);
  await expect(question).toHaveCount(0);
  await expect(field).toHaveValue(other);
});

installedTest("A save that fails late reports nothing next to another review's question", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  const question = page.getByRole("heading", { name: "Use this set of books?", exact: true });
  const use = page.getByRole("button", { name: "Use this set", exact: true });

  // A fails after the reader opened another link on another entry: its error belongs to A's
  // review, not to B's question.
  await openShare(page);
  const second = await holdWrites(page);
  await enterCode(page, codeFor(paizoBooks));
  await use.click();
  await second.held(1);
  await page.goto(`#/books/manage?set=${codeFor(more)}`);
  await expect(question).toBeVisible();
  await expect(page).toHaveURL(/#\/books\/manage\?set=/);
  await second.release({ fail: true });
  await afterQueuedWrites(page);
  await expect(page.getByTestId("share-notice")).toHaveCount(0);
  await expect(question).toBeVisible();
  await expect(page).toHaveURL(/#\/books\/manage\?set=/);
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));

  // A failure of the review that is still on screen is reported, as before.
  const third = await holdWrites(page);
  await use.click();
  await third.held(1);
  await third.release({ fail: true });
  await expect(page.getByTestId("share-notice")).toContainText("Could not save the book selection");
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
});

installedTest("A save whose transaction aborts before its request answers is reported and leaves no page error", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  await openShare(page);
  await abortWrites(page);
  await enterCode(page, codeFor(paizoBooks));
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(page.getByTestId("share-notice")).toContainText("Could not save the book selection");
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  // The guard of the fixture fails the test on the page error that an unobserved transaction
  // promise used to leave behind ("pageerror: null").
});

installedTest("A save that finishes under Quick search rewrites no entry but the link's own", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  const question = page.getByRole("heading", { name: "Use this set of books?", exact: true });
  const use = page.getByRole("button", { name: "Use this set", exact: true });
  // An entry of another page, so that Back from the link's entry names it.
  await visit(page, "#/settings");
  await expect(page.getByRole("heading", { level: 1, name: "Settings", exact: true })).toBeVisible();
  await page.goto(`#/books/manage?set=${codeFor(paizoBooks)}`);
  await expect(question).toBeVisible();

  // An unanswered question stays with its entry through Quick search, and through Forward into
  // the entry that Quick search leaves behind after it closes.
  await openQuickSearch(page);
  await page.keyboard.press("Escape");
  await expect(quickSearch(page)).toBeHidden();
  await expect(question).toBeVisible();
  await expect(page).toHaveURL(/#\/books\/manage\?set=/);
  await page.goForward();
  await expect(question).toBeVisible();
  await page.goBack();
  await expect(question).toBeVisible();
  await expect(use).toBeVisible();

  // The reader answers, and opens Quick search while the books are still saving.
  const hold = await holdWrites(page);
  await use.click();
  await hold.held(1);
  await openQuickSearch(page);
  await hold.release();
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));
  await afterQueuedWrites(page);
  // Quick search's own entry is not the link's: the save left it, and Quick search, alone.
  await expect(quickSearch(page)).toBeVisible();
  await expect(page).toHaveURL(/#\/books\/manage\?set=/);
  await expect(question).toHaveCount(0);

  // Closing Quick search is Back to the link's entry, which takes the answered code out of its
  // address, and Back from there is the page before the link.
  await page.keyboard.press("Escape");
  await expect(quickSearch(page)).toBeHidden();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await expect(question).toHaveCount(0);
  await expect(applied(page, "Paizo Everything")).toBeVisible();
  await page.goBack();
  await expect(page).toHaveURL(/#\/settings$/);
  await page.goForward();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await expect(question).toHaveCount(0);

  // A plain page with no link: Quick search over it, and a save that lands under it, change nothing.
  await page.reload();
  await waitReady(page);
  await enterCodeOpen(page, codeFor(rulebooks));
  await expect(question).toBeVisible();
  const second = await holdWrites(page);
  await use.click();
  await second.held(1);
  await openQuickSearch(page);
  await second.release();
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  await afterQueuedWrites(page);
  await expect(quickSearch(page)).toBeVisible();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await page.keyboard.press("Escape");
  await expect(quickSearch(page)).toBeHidden();
  await expect(question).toHaveCount(0);
  await expect(page).toHaveURL(/#\/books\/manage$/);
});

installedTest("Forward into the entry Quick search left behind does not reopen an answered link", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  const question = page.getByRole("heading", { name: "Use this set of books?", exact: true });
  const use = page.getByRole("button", { name: "Use this set", exact: true });
  await visit(page, "#/settings");
  await expect(page.getByRole("heading", { level: 1, name: "Settings", exact: true })).toBeVisible();

  // Cancelled: the entry that Quick search left forward of the link still has the old address.
  await page.goto(`#/books/manage?set=${codeFor(paizoBooks)}`);
  await expect(question).toBeVisible();
  await openQuickSearch(page);
  await page.keyboard.press("Escape");
  await expect(quickSearch(page)).toBeHidden();
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(question).toHaveCount(0);
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await page.goForward();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await expect(question).toHaveCount(0);
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  await page.goBack();
  await page.goForward();
  await expect(question).toHaveCount(0);

  // A link opened again from outside is a new link and asks.
  await page.goto(`#/books/manage?set=${codeFor(paizoBooks)}`);
  await expect(question).toBeVisible();

  // Applied: the same entry path, after the save.
  await openQuickSearch(page);
  await page.keyboard.press("Escape");
  await expect(quickSearch(page)).toBeHidden();
  await use.click();
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));
  await expect(question).toHaveCount(0);
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await page.goForward();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await expect(question).toHaveCount(0);
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));
  await expect(applied(page, "Paizo Everything")).toBeVisible();
});

installedTest("Forward into the Quick search entry after Settings unmounted the panel does not reopen a cancelled link", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  const question = page.getByRole("heading", { name: "Use this set of books?", exact: true });
  await page.goto("#/settings");
  await expect(page.getByRole("heading", { level: 1, name: "Settings", exact: true })).toBeVisible();

  // Open the link in the same tab, put Quick search over it, close it, and cancel the review.
  await page.goto(`#/books/manage?set=${codeFor(paizoBooks)}`);
  await expect(question).toBeVisible();
  await openQuickSearch(page);
  await page.keyboard.press("Escape");
  await expect(quickSearch(page)).toBeHidden();
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(question).toHaveCount(0);
  await expect(page).toHaveURL(/#\/books\/manage$/);

  // Back to Settings unmounts the panel. The first Forward builds a new one; the second reaches
  // the entry that Quick search left behind, which still carries the link's address.
  await page.goBack();
  await expect(page.getByRole("heading", { level: 1, name: "Settings", exact: true })).toBeVisible();
  await page.goForward();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await page.goForward();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await expect(question).toHaveCount(0);
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
});

installedTest("Cancel leaves the books alone and drops the code from the address", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  await page.goto(`#/books/manage?set=${codeFor(paizoBooks)}`);
  await expect(page.getByRole("heading", { name: "Use this set of books?", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Use this set of books?", exact: true })).toHaveCount(0);
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  await expect(page.locator(".share-status").getByRole("button", { name: "Undo", exact: true })).toHaveCount(0);
});

installedTest("A typo is rejected with a clear message and changes nothing", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  const good = codeFor(paizoBooks);
  await openShare(page);
  const problem = page.getByTestId("share-problem");
  // Every kind of slip: one wrong character, two swapped, one missing, and text that is no code.
  const middle = Math.floor(good.length / 2);
  const wrong = good[middle] === "7" ? "8" : "7";
  const swapped = [...good];
  [swapped[3], swapped[4]] = [swapped[4], swapped[3]];
  const cases = [
    good.slice(0, middle) + wrong + good.slice(middle + 1),
    swapped.join(""),
    good.slice(0, middle) + good.slice(middle + 1),
    good.slice(0, -1),
  ].filter((text) => text !== good);
  for (const text of cases) {
    await enterCode(page, text);
    await expect(problem).toContainText("This code does not look right");
    await expect(problem).toContainText("Your books did not change");
    await expect(page.getByRole("heading", { name: "Use this set of books?", exact: true })).toHaveCount(0);
    await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  }
  await enterCode(page, "hello!");
  await expect(problem).toContainText("A code has no “!”");
  await enterCode(page, "   ");
  await expect(problem).toContainText("Type or paste a code first.");
  // Typing again clears the old message.
  await page.getByLabel("Enter a code", { exact: true }).fill("1");
  await expect(problem).toHaveCount(0);
  // A bad link says so, shows no question, and leaves the address clean.
  await page.goto(`#/books/manage?set=${cases[0]}`);
  await expect(page.getByTestId("share-notice")).toContainText(
    "This link's code does not look right. Check it for typos or a missing part. Your books did not change.",
  );
  await expect(page.getByRole("heading", { name: "Use this set of books?", exact: true })).toHaveCount(0);
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
});

installedTest("A bad link from another route explains itself while the disclosure is closed", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  const good = codeFor(paizoBooks);
  const typo = good.slice(0, 4) + (good[4] === "7" ? "8" : "7") + good.slice(5);
  for (const bad of [typo, "7".repeat(5000), "1"]) {
    const old = bad === typo;
    // Arrive from another route, so Manage books mounts with the link and the disclosure starts
    // closed. The first address is the link as it was made when the section sat on Books.
    await visit(page, "#/settings");
    await expect(page.getByRole("heading", { level: 1, name: "Settings", exact: true })).toBeVisible();
    await page.goto(`${old ? "#/books" : "#/books/manage"}?set=${bad}`);
    await waitReady(page);
    const notice = page.getByRole("alert").filter({ hasText: "This link's code" });
    await expect(notice).toBeVisible();
    await expect(notice).toContainText("Your books did not change");
    // One subject: the link's code, then the advice.
    await expect(notice).toContainText(
      "This link's code does not look right. Check it for typos or a missing part. Your books did not change.",
    );
    await expect(notice).not.toContainText("This link did not open");
    await expect(notice).not.toContainText("This code");
    await expect(page.getByTestId("share-notice")).toBeVisible();
    // The disclosure is closed, so the error is not inside it.
    expect(await disclosure(page).evaluate((element: HTMLDetailsElement) => element.open)).toBe(false);
    expect(await disclosure(page).getByTestId("share-notice").count()).toBe(0);
    await expect(page.getByRole("heading", { name: "Use this set of books?", exact: true })).toHaveCount(0);
    await expect(page).toHaveURL(/#\/books\/manage$/);
    await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  }
  // A later good link replaces the error with the question.
  await page.goto(`#/books/manage?set=${good}`);
  await expect(page.getByRole("heading", { name: "Use this set of books?", exact: true })).toBeVisible();
  await expect(page.getByTestId("share-notice")).toHaveCount(0);
});

installedTest("Undo stays through a rejected typo, a bad link and a cancelled review, until another change succeeds", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  const everything = codeFor(paizoBooks);
  const undo = page.getByRole("button", { name: "Undo", exact: true });
  await page.goto(`#/books/manage?set=${everything}`);
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));
  await expect(undo).toBeVisible();

  // A typo changes nothing, so Undo stays.
  await disclosure(page).locator("summary").click();
  await enterCode(page, everything.slice(0, -1));
  await expect(page.getByTestId("share-problem")).toBeVisible();
  await expect(undo).toBeVisible();
  // So does a bad link opened on this page.
  await page.goto(`#/books/manage?set=${everything.slice(0, -1)}`);
  await expect(page.getByTestId("share-notice")).toBeVisible();
  await expect(undo).toBeVisible();
  // Reviewing another set and cancelling changes nothing either.
  await enterCode(page, codeFor(rulebooks));
  await expect(page.getByRole("heading", { name: "Use this set of books?", exact: true })).toBeVisible();
  await expect(undo).toBeVisible();
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(undo).toBeVisible();
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));

  // A second set that is applied replaces Undo: it now restores the set before it.
  await enterCode(page, codeFor(more));
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(countText(page)).toHaveText(activeLine(more.length));
  await undo.click();
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length));
  await expect(undo).toHaveCount(0);
});

installedTest("Typing is forgiving: lower case, groups, look-alikes and a pasted link", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  const code = codeFor(paizoBooks);
  const grouped = code.match(/.{1,4}/g)!.join("-");
  await openShare(page);
  const heading = page.getByRole("heading", { name: "Use this set of books?", exact: true });
  for (const text of [
    code.toLowerCase(),
    grouped.toLowerCase(),
    code.replace(/1/g, "l").replace(/0/g, "o"),
    `https://example.test/cast-iron/#/books/manage?set=${code}`,
    `https://example.test/cast-iron/#/books?set=${code}`,
  ]) {
    await enterCode(page, text);
    await expect(heading).toBeVisible();
    await expect(page.getByTestId("share-summary")).toHaveText(
      `Turns on ${count(paizoBooks.length - rulebooks.length)} books.`,
    );
    await page.getByRole("button", { name: "Cancel", exact: true }).click();
    await expect(heading).toHaveCount(0);
  }
});

installedTest("The confirmation names a preset, and says when nothing changes", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks & More");
  await openShare(page);
  // The code of the same set: nothing to change.
  await enterCode(page, await shownCode(page));
  await expect(page.getByRole("heading", { name: "Same books", exact: true })).toBeVisible();
  await expect(page.getByTestId("share-summary")).toHaveText("These are the books you already have on. Nothing changes.");
  await expect(page.getByTestId("share-result")).toHaveText(
    `Afterwards: ${countLine(more.length, "Paizo Rulebooks & More")}`,
  );
  await expect(page.getByRole("button", { name: "Use this set", exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "Close", exact: true }).click();

  // The Rulebooks set turns books off, and the result is named.
  await enterCode(page, codeFor(rulebooks));
  await expect(page.getByTestId("share-result")).toHaveText(
    `Afterwards: ${countLine(rulebooks.length, "Paizo Rulebooks")}`,
  );
  await expect(page.getByTestId("share-summary")).toHaveText(/^Turns off [\d,]+ books?\.$/);
  // A set that is no preset reads Custom.
  await enterCode(page, codeFor(rulebooks.slice(1)));
  await expect(page.getByTestId("share-result")).toHaveText(
    `Afterwards: ${countLine(rulebooks.length - 1, "Custom")}`,
  );
});

installedTest("A code from a newer library skips the books this one lacks, and says how many", async ({ page }) => {
  await installLibrary(page);
  await applyPreset(page, "Paizo Rulebooks");
  // The sender had three more books, and two of them were on.
  const described = slotFile.length + 3;
  const on = new Set<number>([...rulebooks.map((id) => slotOf.get(id)!), slotFile.length, slotFile.length + 2]);
  await openShare(page);
  await enterCode(page, bitCode(described, on));
  await expect(page.getByRole("heading", { name: "Same books", exact: true })).toBeVisible();
  await expect(page.getByTestId("share-skipped")).toHaveText(
    "2 books in this code are not in your library, so they are skipped. A newer library may have them.",
  );
  await page.getByRole("button", { name: "Close", exact: true }).click();
  // With a book to change, the skipped line sits beside the change.
  await enterCode(page, bitCode(described, new Set([...on, slotOf.get(paizoBooks[paizoBooks.length - 1])!])));
  await expect(page.getByTestId("share-summary")).toHaveText("Turns on 1 book.");
  await expect(page.getByTestId("share-skipped")).toBeVisible();
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length + 1));
  // No preset matches, so the status gives the count.
  await expect(page.getByRole("status").filter({ hasText: `Now using ${count(rulebooks.length + 1)} books.` })).toBeVisible();
});

installedTest("A code made before a library gained books leaves those books as they are", async ({ page }) => {
  await installLibrary(page);
  // Every Paizo book is on. An older code describes only the first 28 slots, all on.
  await openShare(page);
  await enterCode(page, bitCode(28, new Set(Array.from({ length: 28 }, (_, slot) => slot))));
  await expect(page.getByTestId("share-summary")).toHaveText(
    "These are the books you already have on. Nothing changes.",
  );
  await page.getByRole("button", { name: "Close", exact: true }).click();
  // The same slots with the first one off turn that one book off and touch no other.
  const slots = new Set(Array.from({ length: 27 }, (_, slot) => slot + 1));
  await enterCode(page, bitCode(28, slots));
  await expect(page.getByTestId("share-summary")).toHaveText("Turns off 1 book.");
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(countText(page)).toHaveText(activeLine(paizoBooks.length - 1));
});

installedTest("A code with a later version says so instead of calling it a typo", async ({ page }) => {
  await installLibrary(page);
  await openShare(page);
  await enterCode(page, codeFor(paizoBooks, 2));
  await expect(page.getByTestId("share-problem")).toContainText("comes from a newer version of Cast Iron");
  await expect(page.getByTestId("share-problem")).toContainText("Your books did not change");
  await expect(page.getByRole("heading", { name: /Use this set|Same books/ })).toHaveCount(0);
});

installedTest("Sharing makes no network request beyond the app's own files", async ({ page, server }) => {
  await installLibrary(page);
  await openShare(page);
  await server.clearLog();
  await enterCode(page, codeFor(rulebooks));
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(countText(page)).toHaveText(activeLine(rulebooks.length));
  await page.getByRole("button", { name: "Undo", exact: true }).click();
  await expect(page.getByRole("status").filter({ hasText: "Restored your earlier choice." })).toBeVisible();
  // Opening, reviewing, applying and Undo are local: the server saw no request besides those (the
  // fixture's guard also fails the test on any request to a host that is not local).
  // The browser may re-check its service worker (`sw.js`) and the app's daily update check may
  // fetch the manifest. Neither is the share feature; anything else would be.
  const background = new Set(["sw.js", "content/manifest.json"]);
  expect((await server.requests()).filter((request) => !background.has(request))).toEqual([]);
});
