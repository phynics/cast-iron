import { test, expect, loadCorpus } from "./fixtures";
import { bookTrees, cacheContents, corpusEntry, firstEntry, hasBlock, hitTitles, installLibrary, waitControlled, waitHomeIdle, waitReady, waitResults } from "./helpers";

const corpus = loadCorpus();
const entries = [...corpus.entries.values()];
const spell = corpusEntry("spell", "Fireball");
const classEntry = firstEntry("a class", (entry) => entry.kind === "class");
const tableEntry = firstEntry("an entry with a table", (entry) => hasBlock(entry, "table"));
const source = bookTrees()[0].source;

test("Offline launch works after a new browser session", async ({ openProfile, server }) => {
  let context = await openProfile();
  let page = await context.newPage();
  await installLibrary(page);
  await waitControlled(page);
  await page.evaluate(() => navigator.serviceWorker.ready);
  expect(Object.keys(await cacheContents(page)).some((n) => n.startsWith("cast-iron-shell-"))).toBe(true);
  await context.close();

  // A new session with no network from its first navigation.
  await server.clearLog();
  context = await openProfile({ offline: true });
  page = await context.newPage();
  await page.goto("./");
  await waitReady(page);
  await expect(page.locator("#home-title")).toBeVisible();
  await page.goto("#/search");
  await page.locator("#search-input").fill(spell.title);
  await waitResults(page, spell.title);
  expect((await hitTitles(page))[0]).toBe(spell.title);
  await page.locator(".hit-link", { hasText: new RegExp(`^${spell.title}`) }).first().click();
  await expect(page.getByRole("heading", { level: 1, name: spell.title })).toBeVisible();
  // Chromium's offline mode does not cover the browser's own service worker script check; the
  // server refuses it. Nothing else may be requested.
  expect((await server.requests()).filter((p) => !p.startsWith("sw.js"))).toEqual([]);
});

test("No application feature requires a runtime server", async ({ page, context, server }) => {
  await installLibrary(page);
  await waitControlled(page);
  await page.evaluate(() => navigator.serviceWorker.ready);
  await waitHomeIdle(page);
  await page.reload();
  await waitReady(page);

  // Online, but every request that reaches the server from here on is a failure of the product.
  await server.clearLog();
  await page.goto(`#/search?q=${encodeURIComponent(spell.title)}`);
  await waitResults(page, spell.title);
  await page.locator(".hit-link", { hasText: new RegExp(`^${spell.title}`) }).first().click();
  await expect(page.getByRole("heading", { level: 1, name: spell.title })).toBeVisible();
  await page.locator(".bookmark-toggle").click();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  for (const hash of [
    "#/browse",
    "#/browse/kind/spell",
    "#/browse/kind/rule",
    `#/browse/source/${source}`,
    `#/entry/${classEntry.id}`,
    `#/entry/${tableEntry.id}`,
    "#/bookmarks",
    "#/legal",
  ]) {
    await page.goto(hash);
    await expect(page.locator("main h1").first()).toBeVisible();
  }
  await page.goto("#/settings");
  await page.getByRole("button", { name: "Verify library" }).click();
  await expect(page.getByTestId("verify-status")).toContainText("verified");
  // A new tab and a full reload are served from the service worker too.
  const other = await context.newPage();
  await other.goto(`#/entry/${spell.id}`);
  await expect(other.getByRole("heading", { level: 1, name: spell.title })).toBeVisible();
  await page.reload();
  await waitReady(page);

  // Only the browser's own check for a newer service worker script is allowed.
  const requests = (await server.requests()).filter((p) => p !== "sw.js");
  expect(requests).toEqual([]);
});
