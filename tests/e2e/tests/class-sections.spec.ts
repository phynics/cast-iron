import type { Locator, Page } from "@playwright/test";
import { installedTest as test, expect, loadCorpus } from "./fixtures";
import { installLibrary } from "./helpers";

// The section switcher of a class page (#170, part A): Overview · Features · Archetypes · Spells,
// the stops the class has, as a sticky bar of jump links. The page is one document, so every
// section is in it; a stop moves the page, records `?s=` in the address, and moves focus to the
// section's heading. Expectations come from the compiled corpus (docs/decisions.md, "Class pages:
// section switcher").

const corpus = loadCorpus();
const entryOf = (id: string) => corpus.entries.get(id)!;

const WIZARD = "ref/class/wizard";
const FIGHTER = "ref/class/fighter";
const ARCANIST = "ref/class/arcanist";
const CLASSES = [
  { id: WIZARD, stops: ["Overview", "Features", "Archetypes", "Spells"] },
  { id: FIGHTER, stops: ["Overview", "Features", "Archetypes"] },
  { id: ARCANIST, stops: ["Overview", "Features", "Archetypes", "Spells"] },
];
const WIDTHS = [390, 1280];

const switcher = (page: Page) => page.getByRole("navigation", { name: "Sections" });
const stop = (page: Page, name: string) => switcher(page).getByRole("link", { name, exact: true });
const section = (page: Page, slug: string) => page.locator(`[data-class-section="${slug}"]`);
const top = (locator: Locator) => locator.evaluate((el) => Math.round(el.getBoundingClientRect().top));
const scrollY = (page: Page) => page.evaluate(() => Math.round(window.scrollY));
const overflow = (page: Page) =>
  page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);

/** The text of an inline run, whatever the nesting. */
const inlineText = (items: any[]): string =>
  items.map((item) => item.v ?? inlineText(item.c ?? [])).join("");

/** The shallowest headings of a class body: where Features and Archetypes start. */
function topHeadings(id: string): string[] {
  const headings = entryOf(id).body.filter((block: any) => block.t === "heading");
  const shallowest = Math.min(...headings.map((block: any) => block.level));
  return headings.filter((block: any) => block.level === shallowest).map((block: any) => inlineText(block.content).trim());
}

async function openClass(page: Page, id: string, width: number, query = "") {
  await page.setViewportSize({ width, height: width === 390 ? 844 : 900 });
  await page.goto(`#/entry/${id}${query}`);
  await expect(page.getByRole("heading", { level: 1, name: entryOf(id).title })).toBeVisible();
  await expect(page.locator("section.class-card")).toBeVisible();
}

/** The bottom of the masthead: the line the switcher sticks to. */
const headerBottom = (page: Page) =>
  page.locator("header.app-header").evaluate((el) => Math.round(el.getBoundingClientRect().bottom));

/** Waits until a section's heading sits below the sticky bar and in the upper part of the view. */
async function expectAtTop(page: Page, slug: string, { exact = true } = {}) {
  const heading = section(page, slug);
  await expect(heading).toBeVisible();
  await expect
    .poll(async () => {
      const bar = await switcher(page).evaluate((el) => Math.round(el.getBoundingClientRect().bottom));
      const y = await top(heading);
      return y >= bar - 1 && (exact ? y < bar + 120 : y < (await page.evaluate(() => innerHeight)));
    })
    .toBe(true);
}

test("The switcher lists the stops a class has and every section stays in the page", async ({ page }) => {
  await installLibrary(page);
  for (const { id, stops } of CLASSES) {
    for (const width of WIDTHS) {
      await openClass(page, id, width);
      const label = `${id} at ${width}px`;
      await expect(switcher(page).getByRole("link"), label).toHaveText(stops);
      // It replaces `On this page`; the page keeps no second contents list.
      await expect(page.locator(".article-contents"), label).toHaveCount(0);
      // Overview is where the page opens.
      await expect(stop(page, "Overview"), label).toHaveAttribute("aria-current", "true");
      await expect(switcher(page).locator('[aria-current="true"]'), label).toHaveCount(1);
      // Each stop is a real link with the entry id untouched (rule 3).
      for (const name of stops.slice(1)) {
        await expect(stop(page, name), label).toHaveAttribute("href", `#/entry/${id}?s=${name.toLowerCase()}`);
      }
      await expect(stop(page, "Overview"), label).toHaveAttribute("href", `#/entry/${id}`);
      // Every section is in the document and visible, not only the chosen one, so Find in page
      // and a screen reader's heading list reach all of them.
      const headings = topHeadings(id);
      const features = headings.find((title) => title.toLowerCase().startsWith("class features"))!;
      await expect(section(page, "features"), label).toHaveText(features);
      await expect(section(page, "features"), label).toBeAttached();
      await expect(section(page, "archetypes"), label).toHaveText("Archetypes");
      await expect(section(page, "spells"), label).toHaveCount(stops.includes("Spells") ? 1 : 0);
      for (const slug of ["features", "archetypes", "spells"]) {
        if (await section(page, slug).count()) expect(await section(page, slug).isHidden(), `${label} ${slug}`).toBe(false);
      }
      expect(await overflow(page), label).toBeLessThanOrEqual(0);
    }
  }
});

test("At 390 px the card and the switcher are in the first viewport with the start of the text", async ({ page }) => {
  await installLibrary(page);
  for (const { id, stops } of CLASSES) {
    await openClass(page, id, 390);
    const card = (await page.locator("section.class-card").boundingBox())!;
    const bar = (await switcher(page).boundingBox())!;
    const firstText = (await page.locator(".entry > .rich").first().boundingBox())!;
    expect(card.y + card.height, `${id} card`).toBeLessThan(bar.y);
    expect(bar.y + bar.height, `${id} bar`).toBeLessThan(844);
    // The text under the bar begins on screen too.
    expect(firstText.y, `${id} text`).toBeLessThan(844 - 24);
    // A stop is a touch target of at least 44 px, and the bar fits the page width.
    for (const name of stops) {
      const link = (await stop(page, name).boundingBox())!;
      expect(link.height, `${id} ${name}`).toBeGreaterThanOrEqual(44);
      expect(link.width, `${id} ${name}`).toBeGreaterThanOrEqual(44);
      expect(link.x + link.width, `${id} ${name}`).toBeLessThanOrEqual(390);
    }
    expect(await overflow(page), id).toBeLessThanOrEqual(0);
  }
});

test("A stop moves the page to its section, sticks under the header, and records its address", async ({ page }) => {
  await installLibrary(page);
  for (const { id, stops } of CLASSES) {
    for (const width of WIDTHS) {
      await openClass(page, id, width);
      const label = `${id} at ${width}px`;
      // A jump is not a rebuild: the page that is on screen stays, with what the reader opened.
      await page.locator("article.entry").evaluate((el) => el.setAttribute("data-kept", "1"));
      for (const name of stops.slice(1)) {
        await stop(page, name).click();
        const slug = name.toLowerCase();
        await expect.poll(() => page.evaluate(() => location.hash), label).toBe(`#/entry/${id}?s=${slug}`);
        await expect(stop(page, name), label).toHaveAttribute("aria-current", "true");
        await expect(switcher(page).locator('[aria-current="true"]'), label).toHaveCount(1);
        // The section's heading takes focus, so the next Tab enters it.
        await expect(section(page, slug), label).toBeFocused();
        // The Spell list panel is near the end of the page and may not reach the top of the view.
        await expectAtTop(page, slug, { exact: slug !== "spells" });
        // The bar stays pinned under the header.
        expect(Math.abs((await top(switcher(page))) - (await headerBottom(page))), `${label} ${name}`).toBeLessThanOrEqual(1);
        await expect(page.locator("article.entry"), label).toHaveAttribute("data-kept", "1");
      }
      // Overview returns to the top of the page and clears the section.
      await stop(page, "Overview").click();
      await expect.poll(() => page.evaluate(() => location.hash), label).toBe(`#/entry/${id}`);
      await expect(stop(page, "Overview"), label).toHaveAttribute("aria-current", "true");
      await expect.poll(() => scrollY(page), label).toBe(0);
      expect(await overflow(page), label).toBeLessThanOrEqual(0);
    }
  }
});

test("The bar stays pinned while the page scrolls, and a second tap on the same stop scrolls back", async ({ page }) => {
  await installLibrary(page);
  for (const width of WIDTHS) {
    await openClass(page, WIZARD, width);
    await page.mouse.wheel(0, 3000);
    await expect.poll(() => scrollY(page)).toBeGreaterThan(2000);
    expect(Math.abs((await top(switcher(page))) - (await headerBottom(page)))).toBeLessThanOrEqual(1);
    // The bar is opaque: its background is the page's, not transparent.
    const background = await switcher(page).evaluate((el) => getComputedStyle(el, "::before").backgroundColor);
    expect(background).not.toBe("rgba(0, 0, 0, 0)");
    // Tapping the stop the address already names still returns to it.
    await stop(page, "Archetypes").click();
    await expectAtTop(page, "archetypes");
    await page.mouse.wheel(0, 2500);
    await expect.poll(async () => (await top(section(page, "archetypes")))).toBeLessThan(0);
    await stop(page, "Archetypes").click();
    await expectAtTop(page, "archetypes");
    // Overview scrolls up even though the address was already plain after the jump above.
    await stop(page, "Overview").click();
    await expect.poll(() => scrollY(page)).toBe(0);
    await page.mouse.wheel(0, 1500);
    await stop(page, "Overview").click();
    await expect.poll(() => scrollY(page)).toBe(0);
  }
});

test("A section address opens at that section, and an unknown one opens at the top", async ({ page }) => {
  await installLibrary(page);
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: width === 390 ? 844 : 900 });
    for (const [id, slug] of [[WIZARD, "archetypes"], [WIZARD, "spells"], [FIGHTER, "features"], [ARCANIST, "archetypes"]]) {
      // A real load of the address, not a hash change inside a page that is already open.
      await page.goto(`#/entry/${id}?s=${slug}`);
      await page.reload();
      await expect(page.getByRole("heading", { level: 1, name: entryOf(id).title })).toBeVisible();
      const name = slug[0].toUpperCase() + slug.slice(1);
      await expect(stop(page, name), `${id} ${slug}`).toHaveAttribute("aria-current", "true");
      await expectAtTop(page, slug, { exact: slug !== "spells" });
      expect(await scrollY(page)).toBeGreaterThan(300);
    }
    // A class without Spells, an unknown value and `overview` open at the top; no link is current
    // except Overview, and the page does not break.
    for (const query of ["?s=spells", "?s=nope", "?s=overview", "?s="]) {
      await page.goto(`#/entry/${FIGHTER}${query}`);
      await page.reload();
      await expect(page.getByRole("heading", { level: 1, name: "Fighter" })).toBeVisible();
      await expect(stop(page, "Overview"), query).toHaveAttribute("aria-current", "true");
      await expect(switcher(page).locator('[aria-current="true"]'), query).toHaveCount(1);
      await expect.poll(() => scrollY(page), query).toBe(0);
    }
    // A heading anchor still wins over a section, and the id in the address is the entry's own.
    await page.goto(`#/entry/${WIZARD}?a=arcane-bond&s=archetypes`);
    await page.reload();
    await expect(page.getByRole("heading", { level: 1, name: "Wizard" })).toBeVisible();
    await expect(page.locator("#h-arcane-bond")).toBeInViewport();
  }
});

test("Back returns to the section before", async ({ page }) => {
  await installLibrary(page);
  for (const width of WIDTHS) {
    await openClass(page, WIZARD, width);
    await stop(page, "Features").click();
    await expectAtTop(page, "features");
    await stop(page, "Archetypes").click();
    await expectAtTop(page, "archetypes");
    await stop(page, "Spells").click();
    await expectAtTop(page, "spells", { exact: false });

    await page.goBack();
    await expect.poll(() => page.evaluate(() => location.hash)).toBe(`#/entry/${WIZARD}?s=archetypes`);
    await expect(stop(page, "Archetypes")).toHaveAttribute("aria-current", "true");
    await expectAtTop(page, "archetypes");

    await page.goBack();
    await expect.poll(() => page.evaluate(() => location.hash)).toBe(`#/entry/${WIZARD}?s=features`);
    await expect(stop(page, "Features")).toHaveAttribute("aria-current", "true");
    await expectAtTop(page, "features");

    // The plain address is Overview, at the top where the reader started.
    await page.goBack();
    await expect.poll(() => page.evaluate(() => location.hash)).toBe(`#/entry/${WIZARD}`);
    await expect(stop(page, "Overview")).toHaveAttribute("aria-current", "true");
    await expect.poll(() => scrollY(page)).toBe(0);

    // Forward walks the same way.
    await page.goForward();
    await expect(stop(page, "Features")).toHaveAttribute("aria-current", "true");
    await expectAtTop(page, "features");
    await expect(page.locator("article.entry")).toBeVisible();
  }
});

test("A section jump does not move focus to the page, and the switcher works by keyboard", async ({ page }) => {
  await installLibrary(page);
  for (const width of WIDTHS) {
    await openClass(page, WIZARD, width);
    await stop(page, "Overview").focus();
    await page.keyboard.press("Tab");
    await expect(stop(page, "Features")).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(stop(page, "Archetypes")).toBeFocused();
    await page.keyboard.press("Enter");
    await expect.poll(() => page.evaluate(() => location.hash)).toBe(`#/entry/${WIZARD}?s=archetypes`);
    await expect(stop(page, "Archetypes")).toHaveAttribute("aria-current", "true");
    // Focus is on the heading that the jump reached, not on <main> and not left behind the page.
    await expect(section(page, "archetypes")).toBeFocused();
    await expectAtTop(page, "archetypes");
    // The next Tab enters the section: it lands after the heading, inside the page text.
    await page.keyboard.press("Tab");
    const next = await page.evaluate(() => {
      const heading = document.querySelector('[data-class-section="archetypes"]')!;
      const active = document.activeElement!;
      return {
        after: !!(heading.compareDocumentPosition(active) & Node.DOCUMENT_POSITION_FOLLOWING),
        inBody: !!active.closest(".entry"),
        inBar: !!active.closest(".section-switcher"),
      };
    });
    expect(next).toEqual({ after: true, inBody: true, inBar: false });
    // Back at the bar, Overview by keyboard returns to the top and keeps focus on the link.
    await stop(page, "Overview").focus();
    await page.keyboard.press("Enter");
    await expect.poll(() => page.evaluate(() => location.hash)).toBe(`#/entry/${WIZARD}`);
    await expect.poll(() => scrollY(page)).toBe(0);
    await expect(stop(page, "Overview")).toBeFocused();
    // The focus ring on a stop is visible to a keyboard user.
    const outline = await stop(page, "Overview").evaluate((el) => getComputedStyle(el).outlineStyle);
    expect(outline).not.toBe("none");
  }
});

test("A prestige class has no switcher and keeps the contents disclosure where it has one", async ({ page }) => {
  await installLibrary(page);
  const headingCount = (entry: any) => entry.body.filter((block: any) => block.t === "heading").length;
  const archer = entryOf("ref/pclass/arcane-archer");
  // A prestige class has only Overview and Features, so the bar would add nothing: no switcher
  // and no section targets. (#443 tracks its missing route to Requirements.)
  const withContents = [...corpus.entries.values()].find(
    (entry) => entry.kind === "prestige_class" && headingCount(entry) >= 4,
  )!;
  expect(withContents, "a prestige class with four headings").toBeTruthy();
  for (const width of WIDTHS) {
    for (const entry of [archer, withContents]) {
      await openClass(page, entry.id, width);
      await expect(switcher(page), entry.id).toHaveCount(0);
      await expect(page.locator("[data-class-section]"), entry.id).toHaveCount(0);
      await expect(page.locator(".article-contents"), entry.id).toHaveCount(entry === archer ? 0 : 1);
    }
  }
});

test("An NPC class has the stops it has, and a casting one gets Spells", async ({ page }) => {
  await installLibrary(page);
  for (const width of WIDTHS) {
    await openClass(page, "ref/npcclass/adept", width);
    await expect(switcher(page).getByRole("link")).toHaveText(["Overview", "Features", "Spells"]);
    await openClass(page, "ref/npcclass/warrior", width);
    await expect(switcher(page)).toHaveCount(0);
  }
});
