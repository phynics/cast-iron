import type { Page } from "@playwright/test";
import { installedTest as test, expect } from "./fixtures";
import { installLibrary } from "./helpers";

// Relationship panels are sections with label counts (#573). References and Referenced by fold
// below 64 rem, and a long relationship list shows 8 rows and then `Show all {N}` (#572).

const POWER_ATTACK = "#/entry/ref/feat/power-attack";
const LEADS = 'section[aria-labelledby="rel-leads"]';

async function open(page: Page, hash: string, width: number, height: number) {
  await page.setViewportSize({ width, height });
  await installLibrary(page);
  await page.goto(hash);
  await expect(page.getByTestId("entry-title")).toBeVisible();
}

test("Relationship panels on Power Attack are unboxed sections with Label counts", async ({ page }) => {
  await open(page, POWER_ATTACK, 1280, 900);
  await expect(page.locator(LEADS)).toBeVisible();
  const page_bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  const panels = await page.locator(".rel").evaluateAll((nodes) =>
    nodes.map((node) => {
      const style = getComputedStyle(node);
      return { border: style.borderTopWidth, background: style.backgroundColor };
    }),
  );
  expect(panels.length).toBeGreaterThan(0);
  for (const panel of panels) {
    expect(panel.border).toBe("0px");
    expect(panel.background).toBe(page_bg);
  }
  const counts = await page.locator(".rel-count").evaluateAll((nodes) =>
    nodes.map((node) => {
      const style = getComputedStyle(node);
      return { border: style.borderTopWidth, radius: style.borderTopLeftRadius, weight: style.fontWeight };
    }),
  );
  expect(counts.length).toBeGreaterThan(0);
  for (const count of counts) {
    expect(count.border).toBe("0px");
    expect(count.radius).not.toBe("999px");
    expect(count.weight).toBe("400");
  }
  // The count stays in the heading, so its accessible name keeps it.
  await expect(page.getByRole("heading", { level: 2, name: /^Leads to [\d,]+$/ })).toBeVisible();
  // The body prints the prerequisites and names no feat, so there is no Requirements panel.
  await expect(page.locator("#rel-prereq")).toHaveCount(0);
});

test("Class page fold counts have no border", async ({ page }) => {
  await open(page, "#/entry/ref/class/wizard", 1280, 900);
  const counts = page.locator(".ref-fold > summary .rel-count");
  await expect(counts.first()).toBeVisible();
  for (const count of await counts.all()) {
    await expect(count).toHaveCSS("border-top-width", "0px");
  }
});

test("Referenced by folds below 64 rem and Leads to shows 8 rows with Show all", async ({ page }) => {
  await open(page, POWER_ATTACK, 390, 844);
  const refs = page.locator("#entry-backlinks");
  await expect(refs).toBeVisible();
  expect(await refs.evaluate((node) => node.tagName)).toBe("DETAILS");
  await expect(refs).not.toHaveAttribute("open", "");
  await expect(refs.locator("> summary")).toHaveText(/Referenced by\s*[\d,]+/);
  for (const fold of await page.locator(".entry-refs details.ref-fold").all()) {
    await expect(fold).not.toHaveAttribute("open", "");
  }

  const leads = page.locator(LEADS);
  await expect(leads.locator("ul.rel-list > li")).toHaveCount(8);
  const more = leads.locator("details.backlink-more");
  await expect(more).toHaveCount(1);
  const summary = more.locator("> summary");
  const label = (await summary.textContent())!.trim();
  expect(label).toMatch(/^Show all [\d,]+$/);
  const total = Number(label.replace("Show all ", "").replace(/,/g, ""));
  // The closed row holds no rows.
  await expect(more.locator("li")).toHaveCount(0);
  await summary.click();
  await expect(more.locator("li")).toHaveCount(total - 8);
  await expect(summary).toBeFocused();

  // The same page without the open row stays short on a phone.
  await summary.click();
  const height = await page.evaluate(() => document.documentElement.scrollHeight);
  expect(height).toBeLessThan(4000);
});

test("Power Attack shows both lists open beside the body from 64 rem", async ({ page }) => {
  await open(page, POWER_ATTACK, 1280, 900);
  const panel = page.locator(".entry > .entry-refs");
  await expect(panel).toBeVisible();
  const folds = panel.locator("details.ref-fold");
  await expect(folds.filter({ hasText: /^Referenced by/ })).toHaveCount(1);
  for (const fold of await folds.all()) await expect(fold).toHaveAttribute("open", "");
  const body = (await page.locator(".entry > .rich").first().boundingBox())!;
  const box = (await panel.boundingBox())!;
  expect(box.x).toBeGreaterThan(body.x + body.width - 1);
  expect(Math.abs(box.y - body.y)).toBeLessThan(48);
  // Going narrow folds them; a reader's own toggle survives until the width crosses 64 rem.
  await page.setViewportSize({ width: 390, height: 844 });
  for (const fold of await folds.all()) await expect(fold).not.toHaveAttribute("open", "");
});

test("A race caps each long Racial traits group at 8 rows", async ({ page }) => {
  await open(page, "#/entry/ref/race/dwarf", 390, 844);
  const traits = page.locator('section[aria-labelledby="rel-traits"]');
  await expect(traits).toBeVisible();
  const groups = traits.locator("details.rel-group");
  const capped = groups.filter({ has: page.locator("details.backlink-more") });
  expect(await capped.count()).toBeGreaterThan(0);
  const group = capped.first();
  if (!(await group.evaluate((node) => (node as HTMLDetailsElement).open))) await group.locator("> summary").click();
  await expect(group.locator("> ul > li")).toHaveCount(8);
  const more = group.locator("details.backlink-more");
  await expect(more.locator("> summary")).toHaveText(/^Show all [\d,]+$/);
  await expect(more.locator("li")).toHaveCount(0);
});

test("A mythic feat that prints its prerequisites under a heading does not repeat them", async ({ page }) => {
  await open(page, "#/entry/ref/carried/mythic-adventures/feats/acrobatic/mythic", 1280, 900);
  await expect(page.getByRole("heading", { level: 2, name: "Prerequisites" })).toBeVisible();
  await expect(page.locator(".prereq-text")).toHaveCount(0);
});
