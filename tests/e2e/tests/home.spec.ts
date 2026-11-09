import { installedTest as test, expect, distManifest, loadCorpus } from "./fixtures";
import { installLibrary, seedHome, storedRecentCount, waitHomeIdle, waitReady as waitReadyHome } from "./helpers";

const corpus = loadCorpus();
const spells = corpus.catalog.entries.filter((entry) => entry.kind === "spell");
/** Distinct titles keep the link names unambiguous. */
const distinct = spells.filter((entry, i) => spells.findIndex((c) => c.title === entry.title) === i);
const pick = (from: number, n: number) =>
  distinct.slice(from, from + n).map((e) => ({ id: e.id as string, title: e.title as string, kind: e.kind as string }));
const twins = (() => {
  for (const entry of corpus.catalog.entries) {
    // Twins from different books: the book is what tells them apart.
    const other = corpus.catalog.entries.find((c) => c.title === entry.title && c.id !== entry.id && c.kind === entry.kind && c.source !== entry.source);
    if (other) return [entry, other];
  }
  throw new Error("The compiled library holds no two entries with the same title");
})();

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

test("Home with no stars has no Starred section, one teaching line and Recently viewed first", async ({ page }) => {
  await seedHome(page, { recents: pick(0, 3), stars: [] });
  await expect(page.getByRole("heading", { name: "Starred" })).toHaveCount(0);
  await expect(page.locator(".home-bookmarks")).toHaveCount(0);
  await expect(page.locator(".home-hint")).toHaveText("Tap the star on an entry to keep it here.");
  await expect(page.locator(".home-recent > li")).toHaveCount(3);
  const recent = await page.locator("#home-recent").evaluate((el) => el.getBoundingClientRect().top);
  const hint = await page.locator(".home-hint").evaluate((el) => el.getBoundingClientRect().top);
  const categories = await page.locator("#by-kind").evaluate((el) => el.getBoundingClientRect().top);
  expect(recent).toBeLessThan(hint);
  expect(hint).toBeLessThan(categories);
  // No more than five rows: no Show more, and the old chips are gone.
  await expect(page.getByRole("button", { name: /^Show more/ })).toHaveCount(0);
  await expect(page.locator("section[aria-labelledby=home-recent] .jump")).toHaveCount(0);

  // The old Home sections are gone: the directory replaces the tiles and the books block.
  await expect(page.locator(".home-page .tiles")).toHaveCount(0);
  await expect(page.locator(".home-books")).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Browse by category" })).toHaveCount(0);
  await expect(page.locator(".cats-note")).toHaveText("Entries in your active books.");
});

test("Home with nothing recent and no stars shows only the teaching line", async ({ page }) => {
  await seedHome(page, {});
  await expect(page.getByRole("heading", { name: "Recently viewed" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Starred" })).toHaveCount(0);
  await expect(page.locator(".home-hint")).toBeVisible();
});

test("Home lists up to five stars above Recently viewed, and See all expands the rest", async ({ page }) => {
  const stars = pick(0, 7);
  await seedHome(page, { recents: pick(10, 2), stars });
  await expect(page.locator(".home-hint")).toHaveCount(0);
  const rows = page.locator(".home-bookmarks > li");
  await expect(rows).toHaveCount(5);
  const order = await page.evaluate(() => ["home-bookmarks", "home-recent", "by-kind"].map((id) => document.getElementById(id)!.getBoundingClientRect().top));
  expect(order[0]).toBeLessThan(order[1]);
  expect(order[1]).toBeLessThan(order[2]);
  const first = rows.first();
  await expect(first.locator(".home-row-meta")).toHaveText("Spell");
  expect((await first.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  const star = first.getByRole("button", { name: /^Unstar / });
  const box = (await star.boundingBox())!;
  expect(box.width).toBeGreaterThanOrEqual(44);
  expect(box.height).toBeGreaterThanOrEqual(44);
  const seeAll = page.getByRole("button", { name: "See all (7)" });
  await expect(seeAll).toHaveAttribute("aria-expanded", "false");
  await seeAll.click();
  await expect(rows).toHaveCount(7);
  await expect(page.getByRole("status").filter({ hasText: "Showing all 7 starred entries" })).toHaveCount(1);
  await page.getByRole("button", { name: "Show fewer" }).first().click();
  await expect(rows).toHaveCount(5);
});

test("Unstarring from Home is one tap and the last star brings back the teaching line", async ({ page }) => {
  await seedHome(page, { recents: pick(5, 1), stars: pick(0, 1) });
  await page.getByRole("button", { name: `Unstar ${pick(0, 1)[0].title}` }).click();
  await expect(page.locator(".home-bookmarks")).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Starred" })).toHaveCount(0);
  await expect(page.locator(".home-hint")).toBeVisible();
  // The section left the page with the star; focus must not fall to the body.
  await expect(page.locator("#home-title")).toBeFocused();
});

test("Recently viewed shows five rows, Show more reveals the stored history, Clear survives a reload", async ({ page }) => {
  await seedHome(page, { recents: pick(0, 12) });
  const rows = page.locator(".home-recent > li");
  await expect(rows).toHaveCount(5);
  const more = page.getByRole("button", { name: "Show more (7)" });
  await expect(more).toHaveAttribute("aria-expanded", "false");
  await expect(more).toHaveAttribute("aria-controls", "home-recent-list");
  await more.click();
  await expect(rows).toHaveCount(12);
  await expect(page.getByRole("status").filter({ hasText: "Showing all 12 recent entries" })).toHaveCount(1);
  const fewer = page.getByRole("button", { name: "Show fewer" });
  await expect(fewer).toHaveAttribute("aria-expanded", "true");
  await fewer.click();
  await expect(rows).toHaveCount(5);

  await page.getByRole("button", { name: "Clear", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Recently viewed" })).toHaveCount(0);
  // The click empties the list at once, so the section and its Clear button are gone before the
  // write lands; focus is already on the title, not lost to the body.
  await expect(page.locator("#home-title")).toBeFocused();
  // The list leaves the screen before its write finishes. Reloading under that write stops the
  // page mid-operation, which WebKit reports as "Context is stopped"; wait for it, and for Home's
  // index read, before reloading.
  await expect.poll(() => storedRecentCount(page)).toBe(0);
  await waitHomeIdle(page);
  await page.reload();
  await waitReadyHome(page);
  await expect(page.getByRole("heading", { name: "Recently viewed" })).toHaveCount(0);
});

test("Recent rows are plain one-line links; same-titled entries also name their book", async ({ page }) => {
  const [a, b] = twins;
  await seedHome(page, { recents: [pick(0, 1)[0], { id: a.id, title: a.title }, { id: b.id, title: b.title }] });
  const rows = page.locator(".home-recent > li");
  await expect(rows).toHaveCount(3);
  // A plain row, not a chip: no border or card radius on the link, a hairline under the row.
  const link = rows.first().locator("a");
  expect(await link.evaluate((el) => getComputedStyle(el).borderTopWidth)).toBe("0px");
  expect((await rows.first().boundingBox())!.height).toBeGreaterThanOrEqual(44);
  await expect(rows.first().locator(".home-row-meta")).toHaveText("Spell");
  for (const [i, entry] of [a, b].entries()) {
    const book = corpus.catalog.sources.find((s) => s.id === entry.source)!.title;
    await expect(rows.nth(i + 1).locator(".home-row-meta")).toContainText(book);
  }
  // Screen readers get the same disambiguation: the two links have different names.
  const names = [await rows.nth(1).locator("a").getAttribute("aria-label"), await rows.nth(2).locator("a").getAttribute("aria-label")];
  expect(names[0]).not.toBe(names[1]);
  for (const [i, entry] of [a, b].entries()) {
    const book = corpus.catalog.sources.find((s) => s.id === entry.source)!.title;
    expect(names[i]).toContain(entry.title);
    expect(names[i]).toContain(book);
    await expect(rows.nth(i + 1).getByRole("link", { name: names[i]!, exact: true })).toBeVisible();
  }
  // A row that is not twinned keeps its bare title as its name.
  await expect(rows.first().getByRole("link", { name: pick(0, 1)[0].title, exact: true })).toBeVisible();
  await rows.nth(1).locator("a").click();
  await expect(page).toHaveURL(new RegExp(`#/entry/${a.id}$`));
});

test("A recent entry the library no longer holds is a plain row, not a link", async ({ page }) => {
  await seedHome(page, { recents: [{ id: "missing-source/old", title: "Gone Rule" }, ...pick(0, 1)] });
  const gone = page.locator(".home-recent > li").first();
  await expect(gone.locator(".home-row-meta")).toHaveText("Not in this library");
  await expect(gone.getByRole("link")).toHaveCount(0);
  await expect(page.locator(".home-recent > li").nth(1).getByRole("link")).toHaveCount(1);
});

test("Row titles stay on one line, however long", async ({ page }) => {
  const long = "A very long entry title that goes on and on and on for far more than one line of text on a phone";
  await seedHome(page, {
    recents: [{ id: distinct[0].id, title: long }],
    stars: [{ id: "missing-source/old", title: long, kind: "rule" }],
  });
  for (const row of [page.locator(".home-recent > li").first(), page.locator(".home-bookmarks > li").first()]) {
    const title = row.locator(".home-row-title");
    const lineHeight = await title.evaluate((el) => parseFloat(getComputedStyle(el).lineHeight));
    expect((await title.boundingBox())!.height).toBeLessThanOrEqual(lineHeight + 1);
  }
});

test("#/browse opens Home and keeps Home current for category pages", async ({ page }) => {
  await page.goto("#/browse");
  await expect(page.locator("#home-title")).toBeVisible();

  const nav = page.getByRole("navigation", { name: "Primary" });
  await expect(nav.getByRole("link")).toHaveText(["Home", "Books", "Settings"]);
  await expect(nav.getByRole("link", { name: "Home", exact: true })).toHaveAttribute("aria-current", "page");

  const spells = page.locator("section[aria-labelledby=by-kind] a.cat").filter({
    has: page.locator(".cat-name", { hasText: /^Spells$/ }),
  });
  await spells.click();
  await expect(page).toHaveURL(/#\/browse\/kind\/spell$/);
  await expect(page.getByRole("heading", { level: 1, name: "Spells" })).toBeVisible();
  await expect(nav.getByRole("link", { name: "Home", exact: true })).toHaveAttribute("aria-current", "page");
  await expect(nav.getByRole("link", { name: "Books", exact: true })).not.toHaveAttribute("aria-current", "page");
});

test("Footer links to the source and one report control", async ({ page }) => {
  await page.goto("#/settings");
  const footer = page.locator("footer.app-footer");
  await expect(footer).toContainText("not published, endorsed, or approved by Paizo Inc.");
  await expect(footer.getByRole("link", { name: "Source on GitHub" })).toHaveAttribute(
    "href",
    "https://github.com/phynics/cast-iron",
  );
  const report = footer.getByRole("link", {
    name: "Report a problem on GitHub (opens in a new tab)",
  });
  await expect(report).toHaveCount(1);
  const url = new URL((await report.getAttribute("href"))!);
  expect(url.pathname).toBe("/phynics/cast-iron/issues/new");
  expect(url.searchParams.get("template")).toBe("app-problem.yml");
  expect(url.searchParams.get("route")).toBe("#/settings");
  expect(url.searchParams.get("content-version")).toBe(distManifest().content_version);
  expect(url.searchParams.get("schema-version")).toBe(String(distManifest().schema_version));
  await expect(footer.getByText("Send feedback or report a problem")).toHaveCount(0);
});
