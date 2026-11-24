import type { Page } from "@playwright/test";
import { test, expect, loadCorpus } from "./fixtures";
import { buildLibraryWithFeatures, installLibrary, waitControlled } from "./helpers";

// The feature heads of an archetype in corpus format 4 (#540; decision
// `2026-11-24-feature-heads-and-search-aliases`): a listed feature renders as the head of a
// class-option power (name, dotted leader, `Level N`), keeps its anchor and its place in
// `On this page`, and a trade of the Replaces line links to the feature that states it. The pinned
// corpus types 81 archetypes of the Advanced Player's Guide, Two-Handed Fighter among them (v0.1.46);
// an overlay still replaces its body and `features`, so the spec does not depend on the printed prose.
// The head rule, the aliases and the link choice are the native tests' business.

const corpus = loadCorpus();
const TARGET = "ref/arc-fighter/two-handed-fighter";
const CONTROL = "ref/arc-fighter/lore-warden";

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

test("an archetype in format 4 shows its features as heads, a trade link and the contents", async ({ page, server }) => {
  // The overlay and the first install of the library take longer than the default budget.
  test.setTimeout(300_000);
  expect(corpus.entries.get(TARGET), "the compiled library has Two-Handed Fighter").toBeTruthy();
  expect(corpus.entries.get(CONTROL), "the compiled library has a second archetype").toBeTruthy();
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
      // A heading that the data does not list renders as before.
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
      const overflow = await page.evaluate(
        () => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth,
      );
      expect(overflow).toBeLessThanOrEqual(0);
    });

    await test.step("an archetype without features renders as before", async () => {
      await open(page, CONTROL);
      await expect(page.locator(".feature-head")).toHaveCount(0);
      await expect(page.locator(".entry-facts a")).toHaveCount(0);
    });
  } finally {
    variant.cleanup();
  }
});
