import { test, expect, loadCorpus, GENERATED } from "./fixtures";
import fs from "node:fs";
import path from "node:path";
import { installLibrary, waitControlled, waitReady } from "./helpers";

const corpus = loadCorpus();
const books = JSON.parse(fs.readFileSync(path.join(GENERATED, corpus.version, "books.json"), "utf8"));
const owners = new Map<string,string>();
const walk = (nodes: any[], source: string) => { for (const node of nodes) { if (node.entry) owners.set(node.entry, source); walk(node.children ?? [], source); } };
for (const book of books.books) walk(book.chapters, book.source);

test("Book visibility persists offline and filters discovery while keeping stars readable", async ({ openProfile }) => {
  let context = await openProfile();
  let page = await context.newPage();
  await installLibrary(page);
  await waitControlled(page);
  await page.goto("#/books");
  await waitReady(page);
  const core = page.locator('.book-directory-item[data-book-id="core-rulebook"]');
  const coreToggle = core.getByRole("checkbox", { name: "Active Core Rulebook" });
  await expect(coreToggle).toBeChecked();
  await page.goto("#/entry/core-rulebook/spells/fireball");
  await page.locator(".bookmark-toggle").click();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");

  await page.goto("#/books");
  await core.getByRole("checkbox", { name: "Active Core Rulebook" }).uncheck();
  await expect(core.getByRole("checkbox", { name: "Active Core Rulebook" })).not.toBeChecked();
  await expect(core.getByRole("checkbox", { name: "Active Core Rulebook" })).toBeEnabled();
  await context.close();

  context = await openProfile({ offline: true });
  page = await context.newPage();
  await page.goto("#/books");
  await waitReady(page);
  const restored = page.locator('.book-directory-item[data-book-id="core-rulebook"]');
  await expect(restored.getByRole("checkbox", { name: "Active Core Rulebook" })).not.toBeChecked();
  await page.goto("#/browse/kind/spell");
  const expected = corpus.catalog.entries.filter((entry) => entry.kind === "spell" && (owners.get(entry.id) ?? entry.source) !== "core-rulebook").length;
  await expect(page.locator('.entry-list a[href^="#/entry/"]')).toHaveCount(expected);
  await expect(page.getByRole("link", { name: "Fireball", exact: true })).toHaveCount(0);

  await page.goto("#/search?q=fire&kind=spell");
  await expect(page.locator(".results .hit-link").first()).toBeVisible();
  const hits = await page.locator(".results .hit-link").evaluateAll(links => links.map(link => (link as HTMLAnchorElement).hash.replace(/^#\/entry\//, "").split("?")[0]));
  expect(hits.length).toBeGreaterThan(0);
  for (const id of hits) expect(owners.get(id)).not.toBe("core-rulebook");
  await page.goto("#/");
  await page.locator(".home-bookmarks").getByRole("link", { name: "Fireball", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Fireball", level: 1, exact: true })).toBeVisible();
  await expect(page.locator(".disabled-book-notice")).toContainText("This book is disabled for search and browsing.");
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
});

test("No active books offers a direct recovery path from Home", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await page.goto("#/books");
  for (const toggle of await page.getByRole("checkbox", { name: /^Active / }).all()) { await toggle.uncheck(); await expect(toggle).toBeEnabled(); }
  await page.goto("#/");
  await expect(page.getByText("No books are active.")).toBeVisible();
  await page.getByRole("link", { name: "Open Books" }).click();
  await expect(page.getByRole("heading", { name: "Books", exact: true })).toBeVisible();
  const first = page.getByRole("checkbox", { name: /^Active / }).first();
  await first.check();
  await expect(first).toBeChecked();
});

test("Invalid book choices recover without removing the library or stars", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await page.goto("#/entry/core-rulebook/spells/fireball");
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
  await expect(page.locator(".home-bookmarks").getByRole("link", { name: "Fireball", exact: true })).toBeVisible();
  await page.goto("#/books");
  await expect(page.getByRole("checkbox", { name: "Active Core Rulebook", exact: true })).toBeChecked();
});
