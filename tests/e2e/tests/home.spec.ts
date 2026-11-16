import { installedTest as test, expect, distManifest, loadCorpus } from "./fixtures";
import { installLibrary, seedHome } from "./helpers";

const corpus = loadCorpus();
const spells = corpus.catalog.entries.filter((entry) => entry.kind === "spell");
/** Distinct titles keep the link names unambiguous. */
const distinct = spells.filter((entry, i) => spells.findIndex((c) => c.title === entry.title) === i);
const pick = (from: number, n: number) =>
  distinct.slice(from, from + n).map((e) => ({ id: e.id as string, title: e.title as string, kind: e.kind as string }));

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

test("Home with no stars has no Starred section and one teaching line, and no Recently viewed", async ({ page }) => {
  await seedHome(page, { recents: pick(0, 3), stars: [] });
  await expect(page.getByRole("heading", { name: "Starred" })).toHaveCount(0);
  await expect(page.locator(".home-bookmarks")).toHaveCount(0);
  await expect(page.locator(".home-hint")).toHaveText("Tap the star on an entry to keep it here.");
  // Recently viewed moved to Quick search (#356); Home holds only starred entries and categories.
  await expect(page.getByRole("heading", { name: "Recently viewed" })).toHaveCount(0);
  await expect(page.locator(".home-recent")).toHaveCount(0);
  const hint = await page.locator(".home-hint").evaluate((el) => el.getBoundingClientRect().top);
  const categories = await page.locator("#by-kind").evaluate((el) => el.getBoundingClientRect().top);
  expect(hint).toBeLessThan(categories);

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

test("Home lists up to five stars above the category directory, and See all expands the rest", async ({ page }) => {
  const stars = pick(0, 7);
  await seedHome(page, { recents: pick(10, 2), stars });
  await expect(page.locator(".home-hint")).toHaveCount(0);
  const rows = page.locator(".home-bookmarks > li");
  await expect(rows).toHaveCount(5);
  const order = await page.evaluate(() => ["home-bookmarks", "by-kind"].map((id) => document.getElementById(id)!.getBoundingClientRect().top));
  expect(order[0]).toBeLessThan(order[1]);
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

test("Row titles stay on one line, however long", async ({ page }) => {
  const long = "A very long entry title that goes on and on and on for far more than one line of text on a phone";
  await seedHome(page, {
    stars: [{ id: "missing-source/old", title: long, kind: "rule" }],
  });
  const title = page.locator(".home-bookmarks > li").first().locator(".home-row-title");
  const lineHeight = await title.evaluate((el) => parseFloat(getComputedStyle(el).lineHeight));
  expect((await title.boundingBox())!.height).toBeLessThanOrEqual(lineHeight + 1);
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

test("From 48rem up the header content shares the wide column with Home", async ({ page }) => {
  await seedHome(page, {});
  for (const width of [1024, 1280, 1440, 1920]) {
    await page.setViewportSize({ width, height: 900 });
    const edges = await page.evaluate(() => {
      const rect = (selector: string) => document.querySelector(selector)!.getBoundingClientRect();
      const header = document.querySelector("header.app-header")!;
      // The last control is the right-most visible direct child; the screen-reader status span is not one.
      const last = [...header.children]
        .filter((el) => !el.classList.contains("sr-only"))
        .map((el) => el.getBoundingClientRect())
        .filter((r) => r.width > 0)
        .pop()!;
      return {
        brand: rect("header.app-header .brand").left,
        title: rect("main h1").left,
        last: last.right,
        grid: rect("main ul.cats").right,
        header: header.getBoundingClientRect(),
        viewport: document.documentElement.clientWidth,
      };
    });
    expect(Math.abs(edges.brand - edges.title), `brand vs h1 at ${width}`).toBeLessThanOrEqual(1);
    expect(Math.abs(edges.last - edges.grid), `last control vs grid at ${width}`).toBeLessThanOrEqual(1);
    // The background and border stay full-bleed.
    expect(edges.header.left).toBe(0);
    expect(edges.header.width).toBe(edges.viewport);
  }

  // Below 48rem the header keeps its own 16 px padding: the brand at 16 px, the last control 16 px
  // short of the right edge.
  await page.setViewportSize({ width: 390, height: 844 });
  const narrow = await page.evaluate(() => {
    const header = document.querySelector("header.app-header")!;
    const last = [...header.children]
      .filter((el) => !el.classList.contains("sr-only"))
      .map((el) => el.getBoundingClientRect())
      .filter((r) => r.width > 0)
      .pop()!;
    return { brand: document.querySelector("header.app-header .brand")!.getBoundingClientRect().left, last: last.right };
  });
  expect(Math.abs(narrow.brand - 16), "brand left at 390").toBeLessThanOrEqual(1);
  expect(Math.abs(narrow.last - 374), "last control right at 390").toBeLessThanOrEqual(1);
});

test("From 48rem up Settings, Legal and the footer start at the brand's left edge", async ({ page }) => {
  const remPx = 16;
  for (const hash of ["#/settings", "#/legal"]) {
    await page.goto(hash);
    await expect(page.locator("main h1")).toBeVisible();
    for (const width of [1024, 1280, 1920]) {
      await page.setViewportSize({ width, height: 900 });
      const edges = await page.evaluate(() => {
        const rect = (selector: string) => document.querySelector(selector)!.getBoundingClientRect();
        const main = document.querySelector("main")!;
        const pad = parseFloat(getComputedStyle(main).paddingLeft) + parseFloat(getComputedStyle(main).paddingRight);
        const footerText = [...document.querySelectorAll("footer.app-footer p")].map((p) => {
          const r = p.getBoundingClientRect();
          return { left: r.left, width: r.width };
        });
        return {
          brand: rect("header.app-header .brand").left,
          title: rect("main h1").left,
          content: main.getBoundingClientRect().width - pad,
          footer: footerText,
        };
      });
      expect(Math.abs(edges.brand - edges.title), `${hash} h1 vs brand at ${width}`).toBeLessThanOrEqual(1);
      expect(edges.content, `${hash} content width at ${width}`).toBeLessThanOrEqual(44 * remPx + 1);
      for (const line of edges.footer) {
        expect(Math.abs(edges.brand - line.left), `${hash} footer vs brand at ${width}`).toBeLessThanOrEqual(1);
        expect(line.width, `${hash} footer line length at ${width}`).toBeLessThanOrEqual(44 * remPx + 1);
      }
    }
  }

  // The phone layout keeps its centred measure column and its 16 px footer padding.
  await page.setViewportSize({ width: 390, height: 844 });
  const narrow = await page.evaluate(() => ({
    h1: document.querySelector("main h1")!.getBoundingClientRect().left,
    footer: document.querySelector("footer.app-footer p")!.getBoundingClientRect().left,
  }));
  expect(narrow.footer, "footer left at 390").toBeCloseTo(16, 0);
  expect(narrow.h1, "h1 left at 390").toBeLessThanOrEqual(24);
});
