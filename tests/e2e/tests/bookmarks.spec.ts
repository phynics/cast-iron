import { test, expect, loadCorpus } from "./fixtures";
import { installLibrary, waitControlled, waitReady } from "./helpers";

const corpus = loadCorpus();
/** Ordinary entries the compiled library holds, so the test does not name one dataset's ids. */
const spells = corpus.catalog.entries.filter((entry) => entry.kind === "spell");
/** Distinct titles keep the Home list's link names unambiguous. */
const distinctSpells = spells.filter((entry, index) =>
  spells.findIndex((candidate) => candidate.title === entry.title) === index);
const legacyRows = [
  { entry_id: distinctSpells[0].id, title_snapshot: distinctSpells[0].title },
  { entry_id: distinctSpells[1].id, title_snapshot: distinctSpells[1].title },
  { entry_id: "missing-source/old-rule", title_snapshot: "Remembered Rule" },
];

const legacyProfile = async (page: import("./fixtures").Page) => {
  await page.evaluate((rows) => new Promise<void>((resolve, reject) => {
    const open = indexedDB.open("cast-iron", 1);
    open.onupgradeneeded = () => {
      const db = open.result;
      db.createObjectStore("meta");
      db.createObjectStore("bookmarks", { keyPath: "entry_id" });
      db.createObjectStore("collections", { keyPath: "id" });
    };
    open.onerror = () => reject(open.error);
    open.onsuccess = () => {
      const db = open.result;
      const tx = db.transaction(["bookmarks", "collections"], "readwrite");
      rows.forEach((row, i) => tx.objectStore("bookmarks").put({
        entry_id: row.entry_id, title_snapshot: row.title_snapshot,
        kind: row.entry_id.startsWith("missing-") ? "rule" : "spell", added_at: 10 + i,
        collection_ids: i < 2 ? ["old-collection"] : [],
      }));
      tx.objectStore("collections").put({ id: "old-collection", name: "Campaign notes", created_at: 1 });
      tx.oncomplete = () => { db.close(); resolve(); };
      tx.onerror = () => reject(tx.error);
    };
  }), legacyRows);
};

test("Home keeps legacy collection members as unique stars across offline restart", async ({ openProfile }) => {
  let context = await openProfile();
  let page = await context.newPage();
  await page.goto("./");
  // Finish the initial manifest request before the next navigation. WebKit reports an
  // aborted in-flight fetch as a page error even though this is only fixture setup.
  await expect(page.getByRole("button", { name: "Download", exact: true })).toBeVisible();
  await legacyProfile(page);
  await installLibrary(page);
  await waitControlled(page);
  await context.close();

  context = await openProfile({ offline: true });
  page = await context.newPage();
  await page.goto("#/bookmarks");
  await waitReady(page);
  const rows = page.locator(".home-bookmarks > li");
  await expect(rows).toHaveCount(3);
  await expect(rows).toContainText(["Remembered Rule", distinctSpells[1].title, distinctSpells[0].title]);
  await expect(rows.filter({ hasText: "Remembered Rule" })).toContainText("Not in this library");
  await expect(page.getByRole("link", { name: "Books", exact: true })).toBeVisible();

  // The missing item has a usable Unstar control; it must not navigate away from Home.
  await rows.filter({ hasText: "Remembered Rule" }).getByRole("button", { name: "Unstar Remembered Rule" }).click();
  await expect(rows).toHaveCount(2);
  await expect(page).toHaveURL(/#\/?$/);

  // Star state remains shared with the entry page.
  await page.goto(`#/entry/${distinctSpells[0].id}`);
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator(".bookmark-toggle")).toHaveAccessibleName(`Unstar ${distinctSpells[0].title}`);
});

test("More than six starred entries stay independently openable and removable", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  const distinct = distinctSpells.slice(0, 7);
  expect(distinct).toHaveLength(7);
  for (const stub of distinct) {
    await page.goto(`#/entry/${stub.id}`);
    await expect(page.locator(".bookmark-toggle")).toBeVisible();
    await page.locator(".bookmark-toggle").click();
    await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  }
  await page.goto("#/");
  const rows = page.locator(".home-bookmarks > li");
  await expect(rows).toHaveCount(5);
  const showAll = page.getByRole("button", { name: "Show all 7 starred entries" });
  await expect(showAll).toHaveAttribute("aria-expanded", "false");
  await expect(showAll).toHaveAttribute("aria-controls", "home-bookmarks-list");
  await showAll.click();
  await expect(rows).toHaveCount(7);
  const showFewer = page.getByRole("button", { name: "Show fewer" });
  await expect(showFewer).toHaveAttribute("aria-expanded", "true");
  await showFewer.click();
  await expect(rows).toHaveCount(5);
  await page.getByRole("button", { name: "Show all 7 starred entries" }).click();
  await expect(rows).toHaveCount(7);
  const first = rows.filter({ has: page.getByRole("link", { name: distinct[0].title, exact: true }) });
  await first.getByRole("button", { name: `Unstar ${distinct[0].title}` }).click();
  await expect(rows).toHaveCount(6);
  await expect(page.getByRole("heading", { name: "Your rules reference" })).toBeVisible();
  const last = distinct[6];
  await rows.filter({ has: page.getByRole("link", { name: last.title, exact: true }) })
    .getByRole("link", { name: last.title, exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: last.title, exact: true })).toBeVisible();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
});
