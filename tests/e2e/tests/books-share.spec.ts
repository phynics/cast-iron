import fs from "node:fs";
import path from "node:path";
import { test, installedTest, expect, loadCorpus, ROOT } from "./fixtures";
import { count, expectPreinstalled, installLibrary, openBookGroups, waitReady } from "./helpers";

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
  await page.goto("#/books/manage");
  await waitReady(page);
  const details = disclosure(page);
  await expect(details).toBeVisible();
  if (!(await details.evaluate((element: HTMLDetailsElement) => element.open))) {
    await details.locator("summary").click();
  }
  await expect(details.getByTestId("share-code")).toBeVisible();
}

const shownCode = async (page: any) => (await page.getByTestId("share-code").innerText()).trim();

/** The ids that Manage books shows as checked. */
async function checkedBooks(page: any): Promise<string[]> {
  await page.goto("#/books/manage");
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
  await page.goto("#/books/manage");
  await waitReady(page);
  await page.getByRole("button", { name, exact: true }).click();
  await expect(page.getByRole("status").filter({ hasText: `Applied ${name}.` })).toBeVisible();
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
  await page.goto("#/books/manage");
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
  await page.goto("#/books/manage");
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
  await page.goto("#/settings");
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
  await page.goto("#/books");
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
  await expect(page.getByRole("status").filter({ hasText: "Applied Paizo Everything." })).toBeVisible();
  // A preset replaces the share's Undo: one Undo line at a time.
  await page.getByRole("button", { name: "Paizo Rulebooks", exact: true }).click();
  await expect(page.getByRole("status").filter({ hasText: "Applied Paizo Rulebooks." })).toBeVisible();
  await expect(undo).toHaveCount(1);
  await enterCodeOpen(page, codeFor(paizoBooks));
  await page.getByRole("button", { name: "Use this set", exact: true }).click();
  await expect(undo).toHaveCount(1);
  await expect(page.getByRole("status").filter({ hasText: "Applied Paizo Everything." })).toBeVisible();
  await expect(page.getByRole("status").filter({ hasText: "Applied Paizo Rulebooks." })).toHaveCount(0);
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
    await page.goto("#/settings");
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
