import type { Locator, Page } from "@playwright/test";
import { installedTest as test, expect, loadCorpus } from "./fixtures";
import { installLibrary } from "./helpers";

// The section switcher of a class page (#170, part A): Overview · (Requirements) · Features · Archetypes · Spells,
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

// One address per test, per width (#476). The nine scenarios at two widths used to be one test of
// 18 real loads, about two minutes, at the 120 s limit. Each case below installs the library in
// its own clean profile, then does the real load it is about (`goto` and `reload`), so a failure
// names its width and scenario and each has its own time budget. The titles are plain strings so
// that the feature map check reaches every case.
async function useWidth(page: Page, width: number) {
  await page.setViewportSize({ width, height: width === 390 ? 844 : 900 });
}

/**
 * Reloads the page at an address it did not open with, as a bookmark or a typed address would.
 * The caller has let the page settle, so nothing is in flight.
 *
 * `goto` of a hash on the open page changes the hash, and the page then drops a section it lacks
 * with `replaceState`. A `reload()` right after raced that rewrite: it reloaded either address,
 * and in WebKit the old document's work then surfaced as `pageerror: Context is stopped` after
 * the console guard's navigation window had closed on the rewrite's `framenavigated` (#121).
 * `replaceState` fires no `hashchange`, so the open page does not react, and the reload is the
 * only new-document navigation. (`replaceState` still fires Playwright's same-document
 * `framenavigated` event on the open page, which the console guard's teardown window ignores.)
 */
async function coldLoad(page: Page, address: string) {
  await page.evaluate((hash) => history.replaceState(history.state, "", hash), address);
  await page.reload();
}

/** A section address opens at that section, from a real load of the address. */
async function expectOpensAtSection(page: Page, width: number, id: string, slug: string) {
  await installLibrary(page);
  await useWidth(page, width);
  const label = `${id} ${slug} at ${width}px`;
  // A real load of the address, not a hash change inside a page that is already open.
  await page.goto(`#/entry/${id}?s=${slug}`);
  await page.reload();
  await expect(page.getByRole("heading", { level: 1, name: entryOf(id).title }), label).toBeVisible();
  const name = slug[0].toUpperCase() + slug.slice(1);
  await expect(stop(page, name), label).toHaveAttribute("aria-current", "true");
  await expectAtTop(page, slug, { exact: slug !== "spells" });
  expect(await scrollY(page), label).toBeGreaterThan(300);
}

/** A class without Spells, an unknown value, `overview` and an empty value open at the top; no
 * link is current except Overview, and the page does not break. */
async function expectOpensAtTop(page: Page, width: number, query: string) {
  await installLibrary(page);
  await useWidth(page, width);
  const label = `${query} at ${width}px`;
  await coldLoad(page, `#/entry/${FIGHTER}${query}`);
  await expect(page.getByRole("heading", { level: 1, name: "Fighter" }), label).toBeVisible();
  await expect(stop(page, "Overview"), label).toHaveAttribute("aria-current", "true");
  await expect(switcher(page).locator('[aria-current="true"]'), label).toHaveCount(1);
  await expect.poll(() => scrollY(page), label).toBe(0);
}

/** A heading anchor still wins over a section, and the id in the address is the entry's own. */
async function expectAnchorWins(page: Page, width: number) {
  await installLibrary(page);
  await useWidth(page, width);
  const label = `anchor and section at ${width}px`;
  await page.goto(`#/entry/${WIZARD}?a=arcane-bond&s=archetypes`);
  await page.reload();
  await expect(page.getByRole("heading", { level: 1, name: "Wizard" }), label).toBeVisible();
  await expect(page.locator("#h-arcane-bond"), label).toBeInViewport();
}

test.describe("A section address opens at that section, and an unknown one opens at the top", () => {
  test("At 390 px the Wizard address opens at Archetypes", ({ page }) => expectOpensAtSection(page, 390, WIZARD, "archetypes"));
  test("At 390 px the Wizard address opens at Spells", ({ page }) => expectOpensAtSection(page, 390, WIZARD, "spells"));
  test("At 390 px the Fighter address opens at Features", ({ page }) => expectOpensAtSection(page, 390, FIGHTER, "features"));
  test("At 390 px the Arcanist address opens at Archetypes", ({ page }) => expectOpensAtSection(page, 390, ARCANIST, "archetypes"));
  test("At 390 px a Fighter address for Spells opens at the top", ({ page }) => expectOpensAtTop(page, 390, "?s=spells"));
  test("At 390 px an unknown section opens at the top", ({ page }) => expectOpensAtTop(page, 390, "?s=nope"));
  test("At 390 px the overview section opens at the top", ({ page }) => expectOpensAtTop(page, 390, "?s=overview"));
  test("At 390 px an empty section opens at the top", ({ page }) => expectOpensAtTop(page, 390, "?s="));
  test("At 390 px a heading anchor wins over a section", ({ page }) => expectAnchorWins(page, 390));

  test("At 1280 px the Wizard address opens at Archetypes", ({ page }) => expectOpensAtSection(page, 1280, WIZARD, "archetypes"));
  test("At 1280 px the Wizard address opens at Spells", ({ page }) => expectOpensAtSection(page, 1280, WIZARD, "spells"));
  test("At 1280 px the Fighter address opens at Features", ({ page }) => expectOpensAtSection(page, 1280, FIGHTER, "features"));
  test("At 1280 px the Arcanist address opens at Archetypes", ({ page }) => expectOpensAtSection(page, 1280, ARCANIST, "archetypes"));
  test("At 1280 px a Fighter address for Spells opens at the top", ({ page }) => expectOpensAtTop(page, 1280, "?s=spells"));
  test("At 1280 px an unknown section opens at the top", ({ page }) => expectOpensAtTop(page, 1280, "?s=nope"));
  test("At 1280 px the overview section opens at the top", ({ page }) => expectOpensAtTop(page, 1280, "?s=overview"));
  test("At 1280 px an empty section opens at the top", ({ page }) => expectOpensAtTop(page, 1280, "?s="));
  test("At 1280 px a heading anchor wins over a section", ({ page }) => expectAnchorWins(page, 1280));
});

test("An old entry id keeps the section of its address", async ({ page }) => {
  await installLibrary(page);
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: width === 390 ? 844 : 900 });
    // `core-rulebook/classes/wizard` is a retired id that resolves to the Wizard.
    await coldLoad(page, `#/entry/core-rulebook/classes/wizard?s=archetypes`);
    await expect(page.getByRole("heading", { level: 1, name: "Wizard" })).toBeVisible();
    await expect(page).toHaveURL(/#\/entry\/ref\/class\/wizard\?s=archetypes$/);
    await expect(stop(page, "Archetypes")).toHaveAttribute("aria-current", "true");
    await expectAtTop(page, "archetypes");
  }
});

test("A stop chosen from a heading address is a jump: the page stays and the heading takes focus", async ({ page }) => {
  await installLibrary(page);
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: width === 390 ? 844 : 900 });
    for (const [name, slug] of [["Archetypes", "archetypes"], ["Features", "features"], ["Spells", "spells"]]) {
      await page.goto(`#/entry/${WIZARD}?a=arcane-bond`);
      await page.reload();
      await expect(page.getByRole("heading", { level: 1, name: "Wizard" })).toBeVisible();
      await expect(page.locator("#h-arcane-bond")).toBeInViewport();
      // What the reader has open on the page, to show that it is not rebuilt.
      await page.locator("article.entry").evaluate((el) => el.setAttribute("data-kept", "1"));
      const skills = page.locator("details.class-skills");
      await skills.locator("summary").click();
      await expect(skills).toHaveJSProperty("open", true);

      await stop(page, name).click();
      await expect.poll(() => page.evaluate(() => location.hash), `${name} at ${width}px`).toBe(`#/entry/${WIZARD}?s=${slug}`);
      await expect(stop(page, name)).toHaveAttribute("aria-current", "true");
      await expect(section(page, slug)).toBeFocused();
      await expectAtTop(page, slug, { exact: slug !== "spells" });
      await expect(page.locator("article.entry")).toHaveAttribute("data-kept", "1");
      await expect(skills).toHaveJSProperty("open", true);
      await expect(page.locator("main#main")).not.toBeFocused();
    }
    // Overview from a heading address clears the anchor too, and keeps focus on the link.
    await page.goto(`#/entry/${WIZARD}?a=arcane-bond`);
    await page.reload();
    await expect(page.locator("#h-arcane-bond")).toBeInViewport();
    await page.locator("article.entry").evaluate((el) => el.setAttribute("data-kept", "1"));
    await stop(page, "Overview").click();
    await expect.poll(() => page.evaluate(() => location.hash)).toBe(`#/entry/${WIZARD}`);
    await expect.poll(() => scrollY(page)).toBe(0);
    await expect(page.locator("article.entry")).toHaveAttribute("data-kept", "1");
    await expect(stop(page, "Overview")).toBeFocused();
    await expect(stop(page, "Overview")).toHaveAttribute("aria-current", "true");
  }
});

test("An address that names a section the page lacks drops it and opens at the top", async ({ page }) => {
  await installLibrary(page);
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: width === 390 ? 844 : 900 });
    // A class without a spell list, a prestige class (no Spells or Archetypes) and an entry that is no class.
    for (const [id, query] of [[FIGHTER, "?s=spells"], ["ref/pclass/arcane-archer", "?s=archetypes"], ["ref/spell/fireball", "?s=archetypes"]]) {
      await coldLoad(page, `#/entry/${id}${query}`);
      await expect(page.getByRole("heading", { level: 1, name: entryOf(id).title })).toBeVisible();
      await expect(page, `${id}${query}`).toHaveURL(new RegExp(`#/entry/${id}$`));
      await expect.poll(() => scrollY(page)).toBe(0);
    }
    await expect(page.getByRole("heading", { level: 1, name: "Fireball" })).toBeVisible();
    // An address typed into the page that is already open is dropped too, with no reload.
    await page.goto(`#/entry/${FIGHTER}`);
    await expect(page.getByRole("heading", { level: 1, name: "Fighter" })).toBeVisible();
    await page.evaluate((hash) => { location.hash = hash; }, `#/entry/${FIGHTER}?s=spells`);
    await expect(page).toHaveURL(new RegExp(`#/entry/${FIGHTER}$`));
    await expect(stop(page, "Overview")).toHaveAttribute("aria-current", "true");
    // A heading anchor in the same address stays.
    await coldLoad(page, `#/entry/${FIGHTER}?a=bravery&s=spells`);
    await expect(page.getByRole("heading", { level: 1, name: "Fighter" })).toBeVisible();
    await expect(page).toHaveURL(new RegExp(`#/entry/${FIGHTER}\\?a=bravery$`));
    await expect(page.locator("#h-bravery")).toBeInViewport();
    await expect(stop(page, "Overview")).toHaveAttribute("aria-current", "true");
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

test("A prestige class has a Requirements stop before the long introduction and keeps its contents", async ({ page }) => {
  await installLibrary(page);
  const headingCount = (entry: any) => entry.body.filter((block: any) => block.t === "heading").length;
  const archer = entryOf("ref/pclass/arcane-archer");
  const withContents = [...corpus.entries.values()].find(
    (entry) => entry.kind === "prestige_class" && headingCount(entry) >= 4,
  )!;
  expect(withContents, "a prestige class with four headings").toBeTruthy();
  const stops = ["Overview", "Requirements", "Features"];
  for (const width of WIDTHS) {
    for (const entry of [archer, withContents]) {
      const id = entry.id;
      const label = `${id} at ${width}px`;
      await openClass(page, id, width);
      await expect(switcher(page).getByRole("link"), label).toHaveText(stops);
      for (const name of stops.slice(1)) {
        await expect(stop(page, name), label).toHaveAttribute("href", `#/entry/${id}?s=${name.toLowerCase()}`);
      }
      // The bar is in the first viewport; the Requirements heading is below it before the jump
      // (Arcane Archer at 390 px) and the page keeps every section.
      const bar = (await switcher(page).boundingBox())!;
      expect(bar.y + bar.height, label).toBeLessThanOrEqual(page.viewportSize()!.height);
      expect(bar.height, label).toBeGreaterThanOrEqual(44);
      await expect(section(page, "requirements"), label).toHaveText("Requirements");
      await expect(section(page, "features"), label).toHaveText(
        topHeadings(id).find((title) => title.toLowerCase().startsWith("class features"))!,
      );
      // The contents disclosure stays where the page has one (four headings or more).
      await expect(page.locator(".article-contents"), label).toHaveCount(id === archer.id ? 0 : 1);
      // The full introduction is still in the page, ahead of Requirements.
      const intro = entry.body.find((block: any) => block.t === "paragraph");
      if (intro) {
        const words = inlineText(intro.content).trim().split(/\s+/).slice(0, 6).join(" ");
        await expect(page.locator(".rich").getByText(words, { exact: false }).first(), label).toBeAttached();
      }
      await page.locator("article.entry").evaluate((el) => el.setAttribute("data-kept", "1"));
      for (const name of stops.slice(1)) {
        await stop(page, name).click();
        const slug = name.toLowerCase();
        await expect.poll(() => page.evaluate(() => location.hash), label).toBe(`#/entry/${id}?s=${slug}`);
        await expect(stop(page, name), label).toHaveAttribute("aria-current", "true");
        await expect(section(page, slug), label).toBeFocused();
        await expectAtTop(page, slug, { exact: false });
        expect(Math.abs((await top(switcher(page))) - (await headerBottom(page))), `${label} ${name}`).toBeLessThanOrEqual(1);
        await expect(page.locator("article.entry"), label).toHaveAttribute("data-kept", "1");
      }
      // Back returns to the stop before it.
      await page.goBack();
      await expect.poll(() => page.evaluate(() => location.hash), label).toBe(`#/entry/${id}?s=requirements`);
      await expect(stop(page, "Requirements"), label).toHaveAttribute("aria-current", "true");
      expect(await overflow(page), label).toBeLessThanOrEqual(0);
    }
  }
  // A shared link opens at the Requirements, and a heading link into the page still works.
  await coldLoad(page, `#/entry/${archer.id}?s=requirements`);
  await expect(page.getByRole("heading", { level: 1, name: archer.title })).toBeVisible();
  await expectAtTop(page, "requirements", { exact: false });
  await expect(stop(page, "Requirements")).toHaveAttribute("aria-current", "true");
  const featuresId = (await section(page, "features").getAttribute("id"))!;
  expect(featuresId).toMatch(/^h-/);
  await coldLoad(page, `#/entry/${archer.id}?a=${featuresId.slice(2)}`);
  await expect(page.getByRole("heading", { level: 1, name: archer.title })).toBeVisible();
  await expect(page.locator(`#${featuresId}`)).toBeInViewport();
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
