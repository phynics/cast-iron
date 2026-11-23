import { installedTest as test, expect, loadCorpus } from "./fixtures";
import { latestPrintingGroups, memberBooks, waitHomeIdle, waitReady, writeMeta } from "./helpers";
import type { Page } from "@playwright/test";

// The `Other printings:` line of an entry page (#650). Every fixture comes from the compiled
// catalog, so the spec follows the pinned corpus: an item that several books print, a revised
// feat, and a creature name that two books use for different stat blocks.

const corpus = loadCorpus();
type Stub = (typeof corpus.catalog.entries)[number];
const sources: any[] = corpus.catalog.sources;
const bookTitle = (stub: Stub) => sources.find((source) => source.id === stub.source)!.title as string;
const norm = (title: string) => title.trim().replace(/\s+/g, " ").toLowerCase();
const thirdParty = new Set<string>(sources.filter((source) => source.third_party).map((source) => source.id as string));

/**
 * Items of one name in several books of one product line, with distinct dates, newest first (the
 * order that search and the list folds give). Each printing is in one book alone, and the books
 * differ, so turning one off hides only that printing.
 */
const itemGroup = latestPrintingGroups(corpus, thirdParty, { kind: "item", sameText: false }).find(
  ({ stubs }) =>
    stubs.length >= 3 &&
    new Set(stubs.map((stub: Stub) => stub.source)).size === stubs.length &&
    stubs.every((stub: Stub) => memberBooks(stub).length === 1),
);

const line = (page: Page) => page.getByTestId("printings-line");
const open = async (page: Page, id: string) => {
  await page.goto(`#/entry/${id}`);
  await expect(page.getByTestId("entry-title")).toBeVisible();
  await expect(page.locator(".entry > .rich").first()).toBeVisible();
};
const hrefs = (page: Page) => line(page).locator("a").evaluateAll((anchors) => anchors.map((a) => a.getAttribute("href")));

test("An item printed in several books lists the others, newest first, under the credit lines", async ({ page }) => {
  expect(itemGroup, "the library keeps an item in three books of one product line, with distinct dates").toBeTruthy();
  const { stubs } = itemGroup!;
  await page.goto("#/");
  await waitReady(page);

  for (const viewed of [stubs[0], stubs[stubs.length - 1]]) {
    const others: Stub[] = stubs.filter((stub: Stub) => stub.id !== viewed.id);
    await open(page, viewed.id);
    await expect(line(page)).toHaveCount(1);
    await expect(line(page)).toHaveText(`Other printings: ${others.map(bookTitle).join("; ")}`);
    expect(await hrefs(page)).toEqual(others.map((stub) => `#/entry/${stub.id}`));
    // No `Latest` label and no year: the order alone says which printing is newest.
    await expect(line(page)).not.toContainText(/Latest|\b(19|20)\d\d\b/);
    // The line is its own row after the body and before the Archives of Nethys line.
    const order = await page.evaluate(() =>
      [...document.querySelectorAll(".entry > p")].map((p) => p.className).filter((c) => /sources-line|expanded-line|printings-line|aon-line/.test(c)),
    );
    expect(order.indexOf("printings-line")).toBeGreaterThanOrEqual(0);
    const aon = order.indexOf("aon-line");
    if (aon >= 0) expect(order.indexOf("printings-line")).toBeLessThan(aon);
    // The links open the other printing.
    await line(page).locator("a").first().click();
    await expect(page.getByTestId("entry-title")).toHaveText(others[0].title);
    await expect(page).toHaveURL(new RegExp(`#/entry/${others[0].id.replace(/[.*+?^${}()|[\]\\/]/g, "\\$&")}`));
  }
});

test("A printing in a book that is turned off stays in the line, dimmed and labelled Inactive", async ({ page }) => {
  expect(itemGroup).toBeTruthy();
  const { stubs } = itemGroup!;
  const viewed = stubs[0];
  const off = stubs[stubs.length - 1];
  await page.goto("#/");
  await waitHomeIdle(page);
  await writeMeta(page, "disabled_books", [off.source]);
  await page.reload();
  await waitReady(page);
  await waitHomeIdle(page);
  await open(page, viewed.id);
  const others: Stub[] = stubs.filter((stub: Stub) => stub.id !== viewed.id);
  await expect(line(page)).toHaveText(
    `Other printings: ${others.map((stub) => (stub.id === off.id ? `${bookTitle(stub)} · Inactive` : bookTitle(stub))).join("; ")}`,
  );
  const dimmed = line(page).locator(".printing[data-inactive='true']");
  await expect(dimmed).toHaveCount(1);
  await expect(dimmed.locator(".tag")).toHaveText("Inactive");
  await expect(dimmed.locator("a")).toHaveAttribute("href", `#/entry/${off.id}`);
});

test("A revised feat shows no line on the base page or the revision's page", async ({ page }) => {
  // A revision whose base is the only other entry of that kind and title: nothing but the
  // revision edge joins the two, so any line would repeat the revision notice.
  const byTitle = new Map<string, Stub[]>();
  for (const stub of corpus.catalog.entries) {
    const key = `${stub.kind}\u0000${norm(stub.title)}`;
    byTitle.set(key, [...(byTitle.get(key) ?? []), stub]);
  }
  const pair = [...corpus.entries.values()]
    .filter((entry) => entry.kind === "feat")
    .flatMap((entry) =>
      ((entry.relations ?? []) as any[])
        .filter((relation) => relation.kind === "revision")
        .map((relation) => ({ revision: entry.id as string, base: relation.target as string })),
    )
    .find(({ revision, base }) => {
      const stub = corpus.catalog.entries.find((candidate: Stub) => candidate.id === revision)!;
      const same = byTitle.get(`${stub.kind}\u0000${norm(stub.title)}`) ?? [];
      return same.length === 2 && same.some((candidate) => candidate.id === base);
    });
  expect(pair, "the library has a revised feat that no other book prints").toBeTruthy();
  // Every book on, so the revision's book is active and its notice shows on the base page.
  await page.goto("#/");
  await waitHomeIdle(page);
  await writeMeta(page, "disabled_books", []);
  await page.reload();
  await waitReady(page);
  await waitHomeIdle(page);
  await open(page, pair!.base);
  await expect(page.locator(".revision-notice")).toContainText("Revised by");
  await expect(line(page)).toHaveCount(0);
  await open(page, pair!.revision);
  await expect(line(page)).toHaveCount(0);
});

test("A creature that another book prints under the same name shows no line", async ({ page }) => {
  const byName = new Map<string, Stub[]>();
  for (const stub of corpus.catalog.entries) {
    if (stub.kind !== "creature") continue;
    const key = norm(stub.title);
    byName.set(key, [...(byName.get(key) ?? []), stub]);
  }
  const twins = [...byName.values()].find((stubs) => new Set(stubs.map((stub) => stub.source)).size > 1);
  expect(twins, "two books print a creature under one name").toBeTruthy();
  await page.goto("#/");
  await waitReady(page);
  for (const stub of twins!.slice(0, 2)) {
    await open(page, stub.id);
    await expect(line(page)).toHaveCount(0);
  }
});
