import type { Locator } from "@playwright/test";
import { installedTest as test, expect, loadCorpus } from "./fixtures";
import { firstEntry, installLibrary, waitResults } from "./helpers";

// The entry header and the four chip roles in DESIGN.md: metadata is a text line, filters are the
// only pills, navigation is a rectangle, labels carry no border.
const corpus = loadCorpus();
const single = firstEntry("an entry in one book", (entry) => entry.kind === "feat" && entry.source_ids.length === 1);
const multi = firstEntry("an entry in several books", (entry) => entry.kind === "spell" && entry.source_ids.length > 2);

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

/** One line of text: shorter than 1.6 line heights, whatever the corpus titles are. */
async function expectOneLine(meta: Locator) {
  const lineHeight = await meta.evaluate((el) => parseFloat(getComputedStyle(el).lineHeight));
  expect((await meta.boundingBox())!.height).toBeLessThan(lineHeight * 1.6);
}

test("The metadata line is one line of links, and extra books open on demand", async ({ page }) => {
  await page.goto(`#/entry/${single.id}`);
  const meta = page.locator(".entry-meta");
  await expect(meta.getByRole("link")).toHaveCount(2);
  await expect(meta.locator(".meta-more")).toHaveCount(0);
  await expectOneLine(meta);
  await expect(page.getByRole("link", { name: "Related entries" })).toHaveCount(0);

  await page.goto(`#/entry/${multi.id}`);
  const extra = multi.source_ids.length - 1;
  const more = meta.locator(".meta-more");
  await expect(more.locator("summary")).toHaveText(`+${extra} more`);
  // Kind, first book, and the disclosure share one line.
  await expectOneLine(meta);
  // Closed, the extra books are out of the accessibility tree, so count anchors, not roles.
  await expect(more.locator("a")).toHaveCount(extra);
  await expect(more.locator("a").first()).toBeHidden();
  const summaryTop = (await more.locator("summary").boundingBox())!.y;
  await more.locator("summary").click();
  await expect(more.getByRole("link").first()).toBeVisible();
  // Opening the list keeps the summary on the first line; only the extra books expand below.
  expect(Math.abs((await more.locator("summary").boundingBox())!.y - summaryTop)).toBeLessThan(1);
  expect((await more.getByRole("link").first().boundingBox())!.y).toBeGreaterThan(summaryTop);
});

test("The star is an icon-only 44 by 44 toggle in the title row", async ({ page }) => {
  await page.goto(`#/entry/${multi.id}`);
  const star = page.locator(".bookmark-toggle");
  await expect(star).toBeEnabled();
  await expect(star).toHaveAccessibleName(`Star ${multi.title}`);
  await expect(star).toHaveAttribute("aria-pressed", "false");
  expect((await star.innerText()).trim()).toBe("");
  const box = (await star.boundingBox())!;
  expect(Math.round(box.width)).toBe(44);
  expect(Math.round(box.height)).toBe(44);
  const title = (await page.getByTestId("entry-title").boundingBox())!;
  expect(box.y).toBeLessThan(title.y + title.height);

  await star.click();
  await expect(star).toHaveAttribute("aria-pressed", "true");
  await expect(star).toHaveAccessibleName(`Unstar ${multi.title}`);
});

test("Keyboard focus is visible on the star and on metadata links", async ({ page }) => {
  await page.goto(`#/entry/${multi.id}`);
  await expect(page.locator(".bookmark-toggle")).toBeEnabled();
  const seen = new Set<string>();
  for (let i = 0; i < 40 && seen.size < 2; i++) {
    await page.keyboard.press("Tab");
    const state = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null;
      if (!el) return null;
      const inHead = el.closest(".entry-head");
      if (!inHead) return null;
      const style = getComputedStyle(el);
      return {
        what: el.classList.contains("bookmark-toggle") ? "star" : "meta",
        ok: style.outlineStyle !== "none" && parseFloat(style.outlineWidth) >= 2,
      };
    });
    if (state) {
      expect(state.ok, `${state.what} focus ring`).toBe(true);
      seen.add(state.what);
    }
  }
  expect([...seen].sort()).toEqual(["meta", "star"]);
});

test("Chips keep one form per role", async ({ page }) => {
  // Navigation: Home's recently viewed entries are plain link rows, not pills or boxes.
  await page.goto(`#/entry/${single.id}`);
  await expect(page.getByTestId("entry-title")).toBeVisible();
  await page.goto("#/");
  const recent = page.locator(".home-recent .home-row-link").first();
  await expect(recent).toBeVisible();
  expect(await recent.evaluate((el) => getComputedStyle(el).borderTopWidth)).toBe("0px");
  expect(await recent.evaluate((el) => getComputedStyle(el).borderRadius)).toBe("0px");

  // Filters: the only pill; a selected one is filled and carries a check, and keeps its count.
  await page.goto("#/search?q=fire");
  await waitResults(page, "fire");
  const chip = page.locator(".filters .chip", { hasText: /^Spells/ });
  const pill = chip.locator("span").first();
  expect(await pill.evaluate((el) => getComputedStyle(el).borderRadius)).toBe("999px");
  expect(await pill.evaluate((el) => getComputedStyle(el, "::before").content)).toBe("none");
  await chip.click();
  await expect(chip.locator(".chip-count")).toHaveText(/^[\d,]+$/);
  expect(await pill.evaluate((el) => getComputedStyle(el, "::before").content)).toBe('""');
  const selected = await pill.evaluate((el) => {
    const s = getComputedStyle(el);
    return { background: s.backgroundColor, accent: getComputedStyle(document.documentElement).getPropertyValue("--accent") };
  });
  expect(selected.background).not.toBe("rgba(0, 0, 0, 0)");

  // Browse facets are pills too.
  await page.goto("#/browse/kind/spell");
  const lens = page.locator(".lens-chip").first();
  await expect(lens).toBeVisible();
  expect(await lens.evaluate((el) => getComputedStyle(el).borderRadius)).toBe("999px");
  expect(corpus.catalog.entries.length).toBeGreaterThan(0);
});

test("A label has no border and cannot be mistaken for a link", async ({ page }) => {
  // The feat chain screen marks repeating links with a label.
  await page.goto("#/feats/chains");
  const label = page.locator(".tag").first();
  if ((await label.count()) === 0) test.skip(true, "This library has no repeating chain links");
  await expect(label).toBeVisible();
  const style = await label.evaluate((el) => {
    const s = getComputedStyle(el);
    return { border: s.borderTopWidth, radius: s.borderRadius, cursor: s.cursor, link: !!el.closest("a") };
  });
  expect(style.border).toBe("0px");
  expect(style.radius).toBe("0px");
  expect(style.cursor).not.toBe("pointer");
});
