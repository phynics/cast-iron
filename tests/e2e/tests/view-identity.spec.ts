import type { Page } from "@playwright/test";
import { installedTest as test, expect } from "./fixtures";
import { installLibrary } from "./helpers";

// Some views are replaced, not patched, when they render again (decision
// docs/decisions/2026-11-23-page-views-are-replaced-not-patched.md). That is allowed only for a view
// that renders once while its page is on screen. These tests hold a node of such a view, do what a
// reader does on the page, and then require the same node to be in the document still, with focus
// where the reader put it. A view that rendered again would have a new node.

const POWER_ATTACK = "#/entry/ref/feat/power-attack";
const WIZARD = "#/entry/ref/class/wizard";
const LEADS = 'section[aria-labelledby="rel-leads"]';

type Probes = Record<string, Element>;

/** Remembers the first node of each selector. */
async function hold(page: Page, selectors: string[]) {
  for (const selector of selectors) await expect(page.locator(selector).first()).toBeVisible();
  await page.evaluate((selectors) => {
    const probes: Probes = {};
    for (const selector of selectors) probes[selector] = document.querySelector(selector)!;
    (window as unknown as { probes: Probes }).probes = probes;
  }, selectors);
}

/** Every held node is still in the document and is still the first node of its selector. */
async function expectHeld(page: Page, step: string) {
  const lost = await page.evaluate(() =>
    Object.entries((window as unknown as { probes: Probes }).probes)
      .filter(([selector, node]) => !node.isConnected || document.querySelector(selector) !== node)
      .map(([selector]) => selector),
  );
  expect(lost, `nodes that were rendered again after: ${step}`).toEqual([]);
}

test("An entry's page, panels and header are not rendered again by the star or by Show all", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await installLibrary(page);
  await page.goto(POWER_ATTACK);
  await expect(page.getByTestId("entry-title")).toBeVisible();
  const star = page.locator(".bookmark-toggle");
  await expect(star).toBeEnabled();
  await hold(page, ["main article.entry", ".entry-rel", ".entry-meta", LEADS, ".bookmark-toggle"]);

  // The star is disabled while it saves, and a disabled button gives up focus in Chromium, on
  // main as well. So this step checks the nodes, the star's own included, and not focus.
  await star.click();
  await expect(star).toHaveAttribute("aria-pressed", "true");
  await expect(star).toBeEnabled();
  await expectHeld(page, "the star on");
  await star.click();
  await expect(star).toHaveAttribute("aria-pressed", "false");
  await expect(star).toBeEnabled();
  await expectHeld(page, "the star off");

  const more = page.locator(LEADS).locator("details.backlink-more");
  const summary = more.locator("> summary");
  const total = Number((await summary.textContent())!.replace(/\D/g, ""));
  for (const step of ["Show all opened", "Show all closed", "Show all opened again"]) {
    await summary.click();
    await expect(more.locator("li")).toHaveCount(step === "Show all closed" ? 0 : total - 8);
    await expect(summary).toBeFocused();
    await expectHeld(page, step);
  }
});

test("A class page is not rendered again by a section jump or by the archetype choices", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await installLibrary(page);
  await page.goto(WIZARD);
  await expect(page.getByRole("heading", { level: 1, name: "Wizard" })).toBeVisible();
  const switcher = page.getByRole("navigation", { name: "Sections" });
  await expect(switcher).toBeVisible();
  await hold(page, ["main article.entry", ".entry-rel", ".entry-meta"]);

  const stop = switcher.getByRole("link", { name: "Archetypes", exact: true });
  await stop.click();
  await expect.poll(() => page.evaluate(() => location.hash)).toContain("s=archetypes");
  await expect(page.locator('[data-class-section="archetypes"]')).toBeInViewport();
  await expectHeld(page, "a jump to Archetypes");

  // `?keep=`: tick a feature. The panel stays open and focus stays on the row.
  await page.locator("#filter-btn-keep").click();
  const keep = page.locator("#filter-panel-keep");
  const row = keep.getByRole("checkbox", { name: /^Keep Arcane Bond, \d+ archetypes?$/ });
  await row.check();
  await expect.poll(() => page.evaluate(() => location.hash)).toContain("keep=arcane-bond");
  await expect(row).toBeFocused();
  await expect(keep).toBeVisible();
  await expectHeld(page, "a Keep choice");
  await page.keyboard.press("Escape");
  await expect(keep).toBeHidden();

  // `?with=`: choose an archetype. The panel closes and focus returns to its button.
  const button = page.locator("#filter-btn-with");
  await button.click();
  const combine = page.locator("#filter-panel-with");
  // A row of an archetype is named `<title>, <count>`; the first row clears the choice.
  await combine.getByRole("link", { name: /, \d+$/ }).first().click();
  await expect.poll(() => page.evaluate(() => location.hash)).toContain("with=");
  await expect(combine).toBeHidden();
  await expect(button).toBeFocused();
  await expectHeld(page, "a Combine with choice");

  await switcher.getByRole("link", { name: "Overview", exact: true }).click();
  await expect.poll(() => page.evaluate(() => location.hash)).not.toContain("s=archetypes");
  await expectHeld(page, "a jump back to Overview");
});
