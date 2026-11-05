import { test, expect } from "./fixtures";
import { installLibrary, waitControlled, waitReady } from "./helpers";

const legacyProfile = async (page: import("./fixtures").Page) => {
  await page.evaluate(() => new Promise<void>((resolve, reject) => {
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
      const entries = ["core-rulebook/spells/fireball", "core-rulebook/spells/magic-missile", "missing-source/old-rule"];
      entries.forEach((entry_id, i) => tx.objectStore("bookmarks").put({
        entry_id, title_snapshot: ["Fireball", "Magic Missile", "Remembered Rule"][i],
        kind: i === 2 ? "rule" : "spell", added_at: 10 + i,
        collection_ids: i < 2 ? ["old-collection"] : [],
      }));
      tx.objectStore("collections").put({ id: "old-collection", name: "Campaign notes", created_at: 1 });
      tx.oncomplete = () => { db.close(); resolve(); };
      tx.onerror = () => reject(tx.error);
    };
  }));
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
  await expect(rows).toContainText(["Remembered Rule", "Magic Missile", "Fireball"]);
  await expect(rows.filter({ hasText: "Remembered Rule" })).toContainText("Not in this library");
  await expect(page.getByRole("link", { name: "Books", exact: true })).toBeVisible();

  // The missing item has a usable Unstar control; it must not navigate away from Home.
  await rows.filter({ hasText: "Remembered Rule" }).getByRole("button", { name: "Unstar Remembered Rule" }).click();
  await expect(rows).toHaveCount(2);
  await expect(page).toHaveURL(/#\/?$/);

  // Star state remains shared with the entry page after the legacy-route redirect.
  await page.goto("#/entry/core-rulebook/spells/fireball");
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator(".bookmark-toggle")).toHaveAccessibleName("Unstar Fireball");
});

test("More than six starred entries stay independently openable and removable", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  const ids = [
    "fireball", "magic-missile", "acid-arrow", "burning-hands", "charm-person", "color-spray", "detect-magic",
  ];
  for (const slug of ids) {
    await page.goto(`#/entry/core-rulebook/spells/${slug}`);
    await expect(page.locator(".bookmark-toggle")).toBeVisible();
    await page.locator(".bookmark-toggle").click();
    await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  }
  await page.goto("#/");
  const rows = page.locator(".home-bookmarks > li");
  await expect(rows).toHaveCount(5);
  await page.getByRole("button", { name: "Show all 7 starred entries" }).click();
  await expect(rows).toHaveCount(7);
  const fireballRow = rows.filter({ has: page.getByRole("link", { name: "Fireball", exact: true }) });
  await fireballRow.getByRole("button", { name: "Unstar Fireball" }).click();
  await expect(rows).toHaveCount(6);
  await expect(page.getByRole("heading", { name: "Your rules reference" })).toBeVisible();
  await rows.filter({ has: page.getByRole("link", { name: "Detect Magic", exact: true }) })
    .getByRole("link", { name: "Detect Magic", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Detect Magic", level: 1, exact: true })).toBeVisible();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
});
