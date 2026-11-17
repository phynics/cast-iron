import { installedTest, expect, type Page } from "./fixtures";
import { installLibrary } from "./helpers";

/**
 * The router follows `location.hash` through a burst of changes (#468), also while a heavy
 * search is still rendering and while the Diagnostics page probe is on. The fixture guard fails
 * a test on any console error, which is where a panic reports (`console_error_panic_hook`).
 *
 * The one report of the bug could not be reproduced: the specs here pass without the fix of
 * #468 too, and keep the burst covered. The panic notice itself has no spec, because nothing in
 * the app panics on purpose.
 */

const BURST: { hash: string; check: (page: Page) => Promise<void> }[] = [
  ...["fireball", "magic missle", "cure light", "grapple", "ma", "power attack"].map((q) => ({
    hash: `#/search?q=${encodeURIComponent(q)}`,
    check: async (page: Page) => {
      await expect(page.locator("#search-input")).toHaveValue(q);
    },
  })),
  {
    hash: "#/entry/ref/class/wizard",
    check: async (page) => {
      await expect(page.getByTestId("entry-title")).toHaveText("Wizard");
    },
  },
  {
    hash: "#/entry/ref/class/fighter",
    check: async (page) => {
      await expect(page.getByTestId("entry-title")).toHaveText("Fighter");
    },
  },
  {
    hash: "#/settings/diagnostics",
    check: async (page) => {
      await expect(page.getByRole("heading", { level: 1, name: "Diagnostics" })).toBeVisible();
    },
  },
];

async function setHash(page: Page, hash: string) {
  await page.evaluate((hash) => {
    location.hash = hash;
  }, hash);
}

for (const gap of [2500, 250, 0]) {
  installedTest("The page follows nine hash changes " + gap + " ms apart, from Diagnostics at phone width", async ({ page }) => {
    const problems: string[] = [];
    page.on("pageerror", (error) => problems.push(`pageerror: ${error.message}`));
    await page.setViewportSize({ width: 390, height: 844 });
    await installLibrary(page);
    // Opening Diagnostics switches the page probe on for this launch.
    await page.goto("#/settings/diagnostics");
    await expect(page.getByRole("heading", { level: 1, name: "Diagnostics" })).toBeVisible();

    for (const step of BURST) {
      await setHash(page, step.hash);
      if (gap > 0) await page.waitForTimeout(gap);
    }
    // The burst ends on Diagnostics, where it began, so a screen stuck since the start would pass
    // for it. One more hash, to another screen, proves that routing went on.
    await setHash(page, "#/settings");
    await expect(page.getByRole("heading", { level: 1, name: "Settings" })).toBeVisible();
    expect(await page.evaluate(() => location.hash)).toBe("#/settings");

    // Every step of a second, slower pass lands on its own page.
    for (const step of BURST) {
      await setHash(page, step.hash);
      await step.check(page);
    }

    // The navigation and the search box still work.
    await page.getByRole("navigation", { name: "Primary" }).getByRole("link", { name: "Settings" }).click();
    await expect(page.getByRole("heading", { level: 1, name: "Settings" })).toBeVisible();
    await setHash(page, "#/search?q=ma");
    await expect(page.locator("#search-input")).toHaveValue("ma");
    await page.locator("#search-input").fill("grapple");
    await expect(page.locator("#search-input")).toHaveValue("grapple");
    await expect(page).toHaveURL(/#\/search\?q=grapple/);
    // A panic leaves the page stuck; the app says so with this notice (`panic_notice.rs`).
    await expect(page.locator("#panic-notice")).toHaveCount(0);
    expect(problems).toEqual([]);
  });
}

installedTest("A class page follows section jumps made in a burst", async ({ page }) => {
  const problems: string[] = [];
  page.on("pageerror", (error) => problems.push(`pageerror: ${error.message}`));
  await installLibrary(page);
  await page.goto("#/settings/diagnostics");
  await expect(page.getByRole("heading", { level: 1, name: "Diagnostics" })).toBeVisible();
  await page.goto("#/entry/ref/class/wizard");
  await expect(page.getByTestId("entry-title")).toHaveText("Wizard");
  for (let round = 0; round < 5; round++) {
    for (const section of ["features", "spells", "archetypes", ""]) {
      await setHash(page, section ? `#/entry/ref/class/wizard?s=${section}` : "#/entry/ref/class/wizard");
    }
    await setHash(page, "#/search?q=ma");
    await setHash(page, "#/entry/ref/class/fighter?s=features");
  }
  await expect(page.getByTestId("entry-title")).toHaveText("Fighter");
  await setHash(page, "#/settings");
  await expect(page.getByRole("heading", { level: 1, name: "Settings" })).toBeVisible();
  await expect(page.locator("#panic-notice")).toHaveCount(0);
  expect(problems).toEqual([]);
});
