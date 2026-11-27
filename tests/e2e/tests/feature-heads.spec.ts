import type { Page } from "@playwright/test";
import { test, installedTest, expect, loadCorpus } from "./fixtures";
import { buildLibraryWithFeatures, installLibrary, quickSearchFor, waitControlled } from "./helpers";

// The feature heads of an archetype in corpus format 4 (#540; decision
// `2026-11-24-feature-heads-and-search-aliases`): a listed feature renders as the head of a
// class-option power (name, dotted leader, `Level N`), keeps its anchor and its place in
// `On this page`, and a trade of the Replaces line links to the feature that states it. The pinned
// corpus types 81 archetypes of the Advanced Player's Guide, Two-Handed Fighter among them (v0.1.46);
// an overlay still replaces its body and `features`, so the spec does not depend on the printed prose.
// The head rule, the aliases and the link choice are the native tests' business.

const corpus = loadCorpus();
const TARGET = "ref/arc-fighter/two-handed-fighter";
// An archetype still in its import form (no `features`): the control that renders as before. #540
// converts the archetypes book by book (Lore Warden, the first control, converted in v0.1.56), so the
// control comes from the library. Once every archetype is converted, the step has nothing to compare.
const CONTROL: string | undefined = [...corpus.entries.values()]
  .filter((entry) => entry.kind === "archetype" && entry.id !== TARGET && !entry.fields?.features)
  .map((entry) => entry.id)
  .sort()[0];

const text = (v: string) => ({ t: "text", v });
const paragraph = (v: string) => ({ t: "paragraph", content: [text(v)] });
const heading = (anchor: string, v: string) => ({ t: "heading", level: 2, anchor, content: [text(v)] });
/** Enough text to push the later heads below the first screen, so a jump has to scroll. */
const filler = (n: number) => Array.from({ length: n }, (_, i) => paragraph(`Filler paragraph ${i + 1} of the opening text.`));

const body = [
  paragraph("Some fighters focus their efforts on the heaviest weapons."),
  heading("shattering-strike", "Shattering Strike (Ex)"),
  paragraph(
    "At 2nd level, a two-handed fighter gains a +1 bonus to CMB and CMD on sunder attempts. This ability replaces bravery.",
  ),
  ...filler(25),
  heading("piledriver", "Piledriver (Ex)"),
  paragraph("At 5th level, a two-handed fighter can make a bull rush as a standard action."),
  ...filler(25),
  heading("weapon-training", "Weapon Training"),
  paragraph("A two-handed fighter applies weapon training to two-handed weapons."),
  heading("notes", "Notes"),
  paragraph("A heading that the data does not list as a feature."),
];
const features = [
  { anchor: "shattering-strike", level: 2 },
  { anchor: "piledriver", level: 5 },
  { anchor: "weapon-training" },
];
const by = {
  Bravery: ["shattering-strike"],
  "Armor training 1": ["piledriver"],
  "Armor training 2": ["gone", "weapon-training"],
};

async function open(page: Page, id: string, anchor?: string) {
  await page.goto(`#/entry/${id}${anchor ? `?a=${anchor}` : ""}`);
  await expect(page.getByTestId("entry-title")).toHaveText(corpus.entries.get(id)!.title);
  await expect(page.locator(".entry-head")).toBeVisible();
}

const scrollY = (page: Page) => page.evaluate(() => window.scrollY);

/** The scroll offset once it stops moving: polls until two reads in a row are equal. */
async function settledScrollY(page: Page) {
  let previous = Number.NaN;
  await expect
    .poll(async () => {
      const now = await scrollY(page);
      const stable = now === previous;
      previous = now;
      return stable;
    })
    .toBe(true);
  return scrollY(page);
}

test("an archetype in format 4 shows its features as heads, a trade link and the contents", async ({ page, server }) => {
  // The overlay and the first install of the library take longer than the default budget.
  test.setTimeout(300_000);
  expect(corpus.entries.get(TARGET), "the compiled library has Two-Handed Fighter").toBeTruthy();
  const variant = buildLibraryWithFeatures(TARGET, body, features, by);
  try {
    await server.overlay(variant.dir);
    await installLibrary(page);
    await waitControlled(page);
    await page.setViewportSize({ width: 1280, height: 900 });

    await test.step("a listed feature is a head with a leader and its level; a feature without one has neither", async () => {
      await open(page, TARGET);
      const heads = page.locator(".rich > .power-head.feature-head");
      await expect(heads).toHaveCount(3);
      const strike = heads.filter({ has: page.getByRole("heading", { name: "Shattering Strike (Ex)", exact: true }) });
      await expect(strike.getByRole("heading", { level: 2 })).toHaveClass(/power-name/);
      await expect(strike.locator(".power-level")).toHaveText("Gained at Level 2");
      await expect(strike.locator(".power-leader")).toHaveAttribute("aria-hidden", "true");
      await expect(heads.nth(1).locator(".power-level")).toContainText("Level 5");
      // Weapon Training has no level: the name alone, with no leader and no right side.
      const training = heads.nth(2);
      await expect(training.getByRole("heading", { name: "Weapon Training", exact: true })).toBeVisible();
      await expect(training.locator(".power-level")).toHaveCount(0);
      await expect(training.locator(".power-leader")).toHaveCount(0);
      // The level sits on the right of the name, on one line.
      const [name, level] = await Promise.all([
        strike.getByRole("heading").boundingBox(),
        strike.locator(".power-level").boundingBox(),
      ]);
      expect(level!.x).toBeGreaterThan(name!.x + name!.width);
      expect(Math.abs(level!.y - name!.y)).toBeLessThan(name!.height);
      // The tag is its own element after the name: italic, muted, not bold, and not inside the name.
      const tag = strike.locator(".ability-tag");
      await expect(tag).toHaveText("(Ex)");
      await expect(strike.locator(".feature-name")).toHaveText("Shattering Strike");
      await expect(strike.locator(".feature-name .ability-tag")).toHaveCount(0);
      const tagStyle = await tag.evaluate((el) => {
        const style = getComputedStyle(el);
        return { fontStyle: style.fontStyle, fontWeight: style.fontWeight, color: style.color, size: style.fontSize };
      });
      const nameStyle = await strike.locator(".feature-name").evaluate((el) => {
        const style = getComputedStyle(el);
        return { fontWeight: style.fontWeight, color: style.color };
      });
      const bodySize = await page.locator(".rich > p").first().evaluate((el) => getComputedStyle(el).fontSize);
      expect(tagStyle.fontStyle).toBe("italic");
      expect(Number(tagStyle.fontWeight)).toBeLessThan(600);
      expect(Number(nameStyle.fontWeight)).toBeGreaterThanOrEqual(600);
      expect(tagStyle.color).not.toBe(nameStyle.color);
      expect(tagStyle.size).toBe(bodySize);
      // Piledriver has a tag, Weapon Training none: no tag element, so no stray space.
      await expect(heads.nth(1).locator(".ability-tag")).toHaveText("(Ex)");
      await expect(training.locator(".ability-tag")).toHaveCount(0);
      // The printed heading text and its accessible name are unchanged.
      await expect(strike.getByRole("heading")).toHaveText("Shattering Strike (Ex)");
      // A hairline rule sits between one listed feature and the next, and not above the first.
      const rule = (head: ReturnType<typeof heads.nth>) =>
        head.evaluate((el) => {
          const style = getComputedStyle(el);
          return { width: style.borderTopWidth, style: style.borderTopStyle };
        });
      expect(await rule(heads.nth(0))).toEqual({ width: "0px", style: "none" });
      expect(await rule(heads.nth(1))).toEqual({ width: "1px", style: "solid" });
      expect(await rule(heads.nth(2))).toEqual({ width: "1px", style: "solid" });
      // The rule is above the head, with the name below it, not on it.
      const [piledriver, piledriverName] = await Promise.all([
        heads.nth(1).boundingBox(),
        heads.nth(1).getByRole("heading").boundingBox(),
      ]);
      expect(piledriverName!.y).toBeGreaterThan(piledriver!.y + 1);
      // A heading that the data does not list renders as before, with no hairline of this kind.
      const notes = page.getByRole("heading", { name: "Notes", exact: true });
      await expect(notes).not.toHaveClass(/power-name/);
      await expect(page.locator(".feature-head").getByRole("heading", { name: "Notes" })).toHaveCount(0);
    });

    await test.step("the anchor of a feature is a deep link that scrolls to its head", async () => {
      await open(page, TARGET, "weapon-training");
      await expect(page.locator("#h-weapon-training")).toBeInViewport();
      expect(await scrollY(page)).toBeGreaterThan(300);
    });

    await test.step("On this page lists the features with the other headings, in page order", async () => {
      await open(page, TARGET);
      const contents = page.locator("details.article-contents");
      await expect(contents).toHaveCount(1);
      await contents.locator("summary").click();
      const links = contents.getByRole("link");
      await expect(links).toHaveText(["Shattering Strike (Ex)", "Piledriver (Ex)", "Weapon Training", "Notes"]);
      await expect(links.nth(1)).toHaveAttribute("href", `#/entry/${TARGET}?a=piledriver`);
      await links.nth(1).click();
      await expect(page.locator("#h-piledriver")).toBeInViewport();
    });

    await test.step("a trade that names a feature links to it; one that names none stays text", async () => {
      await open(page, TARGET);
      const facts = page.locator(".entry-facts");
      const bravery = facts.getByRole("link", { name: "Bravery (stated under Shattering Strike)" });
      await expect(bravery).toHaveAttribute("href", `#/entry/${TARGET}?a=shattering-strike`);
      // The first `by` anchor that has a head on the page: `gone` has none.
      await expect(facts.getByRole("link", { name: "Armor training 2 (stated under Weapon Training)" })).toHaveAttribute(
        "href",
        `#/entry/${TARGET}?a=weapon-training`,
      );
      await expect(facts.getByRole("link", { name: /^Armor training 3/ })).toHaveCount(0);
      await expect(facts).toContainText("Armor mastery");
      await expect(facts.getByRole("link", { name: /^Armor mastery/ })).toHaveCount(0);
      // Bravery, Armor training 1 and Armor training 2 name a feature; the rest stay text.
      await expect(facts.getByRole("link")).toHaveCount(3);
      // The names keep the order and the commas of the plain line.
      await expect(facts).toContainText("Replaces Bravery");
      await facts.getByRole("link", { name: /^Armor training 1/ }).click();
      await expect(page).toHaveURL(new RegExp(`#/entry/${TARGET}\\?a=piledriver$`));
      await expect(page.locator("#h-piledriver")).toBeInViewport();
    });

    await test.step("at phone width the head keeps the level on its line and the page does not overflow", async () => {
      await page.setViewportSize({ width: 390, height: 844 });
      await open(page, TARGET);
      const strike = page.locator(".feature-head").first();
      await expect(strike.locator(".power-level")).toBeVisible();
      // The name wraps or fits, and the tag and the level stay readable beside it without overlap.
      const [nameBox, levelBox] = await Promise.all([
        strike.getByRole("heading").boundingBox(),
        strike.locator(".power-level").boundingBox(),
      ]);
      expect(levelBox!.x).toBeGreaterThanOrEqual(nameBox!.x + nameBox!.width - 1);
      await expect(strike.locator(".ability-tag")).toBeVisible();
      const overflow = await page.evaluate(
        () => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth,
      );
      expect(overflow).toBeLessThanOrEqual(0);
    });

    await test.step("an archetype without features renders as before", async () => {
      // `test.skip` here would skip the whole test, so the step returns instead.
      if (CONTROL === undefined) return;
      await open(page, CONTROL);
      await expect(page.locator(".feature-head")).toHaveCount(0);
      await expect(page.locator(".entry-facts a")).toHaveCount(0);
    });
  } finally {
    variant.cleanup();
  }
});

// A base class of the Advanced Player's Guide in the pinned corpus (v0.1.65, #373) lists its class
// features: the Inquisitor. The expectations come from the compiled entry, so a later edit of the
// printed prose does not break the test.
const INQUISITOR = "ref/class/inquisitor";
const FEATURE = { anchor: "monster-lore", name: "Monster Lore" };

installedTest("a base class in format 4 shows its features as heads and deep links, and search finds it at one", async ({ page }) => {
  installedTest.setTimeout(300_000);
  const entry = corpus.entries.get(INQUISITOR)!;
  expect(entry, "the compiled library has the Inquisitor").toBeTruthy();
  const listed: { anchor: string; level?: number }[] = entry.fields.features;
  expect(listed.length, "the Inquisitor lists class features").toBeGreaterThan(10);
  const heads = new Map<string, string>();
  const walk = (blocks: any[]) => {
    for (const block of blocks) {
      if (block.t === "heading") heads.set(block.anchor, block.content.map((part: any) => part.v ?? "").join(""));
    }
  };
  walk(entry.body);
  const shown = listed.filter((feature) => heads.has(feature.anchor));
  expect(shown.length, "every listed feature has a heading").toBe(listed.length);
  const monster = listed.find((feature) => feature.anchor === FEATURE.anchor)!;
  expect(monster.level, "Monster Lore is gained at level 1").toBe(1);

  await installLibrary(page);
  await waitControlled(page);
  await page.setViewportSize({ width: 1280, height: 900 });

  await test.step("each listed feature is a head, and one with a level shows it", async () => {
    await page.goto(`#/entry/${INQUISITOR}`);
    await expect(page.getByRole("heading", { level: 1, name: entry.title })).toBeVisible();
    const feature = page.locator(".feature-head");
    await expect(feature).toHaveCount(listed.length);
    // Each anchor of the corpus is one head in the DOM, with the printed text and the level.
    for (const item of listed) {
      const head = feature.filter({ has: page.locator(`#h-${item.anchor}`) });
      await expect(head, item.anchor).toHaveCount(1);
      await expect(head.getByRole("heading"), item.anchor).toHaveText(heads.get(item.anchor)!);
      if (item.level === undefined) await expect(head.locator(".power-level"), item.anchor).toHaveCount(0);
      else await expect(head.locator(".power-level"), item.anchor).toHaveText(`Gained at Level ${item.level}`);
    }
    // Monster Lore is the one the rest of the test follows: exactly `Level 1`.
    const lore = feature.filter({ has: page.locator(`#h-${FEATURE.anchor}`) });
    await expect(lore.locator(".power-level")).toHaveText("Gained at Level 1");
    await expect(lore.locator(".feature-name")).toHaveText(FEATURE.name);
  });

  await test.step("the Features stop and the address of a feature reach its head; the switcher replaces On this page", async () => {
    // A class page has the section switcher in place of the contents list (class-sections.spec.ts).
    await page.goto(`#/entry/${INQUISITOR}`);
    await expect(page.locator("details.article-contents")).toHaveCount(0);
    const stops = page.getByRole("navigation", { name: "Sections" });
    await expect(stops.getByRole("link", { name: "Features", exact: true })).toBeVisible();
    await page.goto(`#/entry/${INQUISITOR}?a=${FEATURE.anchor}`);
    await expect(page.locator(`#h-${FEATURE.anchor}`)).toHaveText(heads.get(FEATURE.anchor)!);
    await expect.poll(() => page.evaluate(() => location.hash)).toBe(`#/entry/${INQUISITOR}?a=${FEATURE.anchor}`);
    await expect(page.locator(`#h-${FEATURE.anchor}`)).toBeInViewport();
    expect(await settledScrollY(page)).toBeGreaterThan(300);
  });

  await test.step("a search for a feature name finds the class at that feature", async () => {
    await quickSearchFor(page, FEATURE.name);
    const hit = page.locator(".results .hit-link").filter({ has: page.locator(".hit-title", { hasText: /^Inquisitor$/ }) });
    await expect(hit).toHaveCount(1);
    // The result carries the matched feature in the anchor parameter: its printed name or its anchor.
    const href = (await hit.getAttribute("href"))!;
    const [path, query] = href.split("?");
    expect(path).toBe(`#/entry/${INQUISITOR}`);
    expect([FEATURE.name, FEATURE.anchor]).toContain(new URLSearchParams(query).get("a"));
    await hit.click();
    await expect(page.getByRole("heading", { level: 1, name: entry.title })).toBeVisible();
    await expect(page.locator(`#h-${FEATURE.anchor}`)).toBeInViewport();
    expect(await settledScrollY(page)).toBeGreaterThan(300);
  });
});
