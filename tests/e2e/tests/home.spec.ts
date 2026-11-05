import { test, expect } from "./fixtures";
import { installLibrary } from "./helpers";

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

test("Home shows starred entries above the category directory", async ({ page }) => {
  await page.goto("#/");
  await expect(page.locator("#home-title")).toBeVisible();
  await expect(page.locator(".cats-note")).toHaveText("Entries in your active books.");
  await expect(page.locator("section[aria-labelledby=by-kind] a.cat").first()).toBeVisible();

  const starred = await page.locator("#home-bookmarks").evaluate((el) => el.getBoundingClientRect().top);
  const categories = await page.locator("#by-kind").evaluate((el) => el.getBoundingClientRect().top);
  expect(starred).toBeLessThan(categories);
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

test("Home shows five starred rows until Show all is pressed", async ({ page }) => {
  const ids = ["fireball", "magic-missile", "acid-arrow", "burning-hands", "charm-person", "color-spray"];
  for (const slug of ids) {
    await page.goto(`#/entry/core-rulebook/spells/${slug}`);
    await expect(page.locator(".bookmark-toggle")).toBeVisible();
    await page.locator(".bookmark-toggle").click();
    await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  }

  await page.goto("#/");
  const rows = page.locator(".home-bookmarks > li");
  await expect(rows).toHaveCount(5);

  const showAll = page.getByRole("button", { name: "Show all 6 starred entries" });
  await expect(showAll).toHaveAttribute("aria-expanded", "false");
  await expect(showAll).toHaveAttribute("aria-controls", "home-bookmarks-list");
  await showAll.click();
  await expect(rows).toHaveCount(6);

  const showFewer = page.getByRole("button", { name: "Show fewer" });
  await expect(showFewer).toHaveAttribute("aria-expanded", "true");
  await showFewer.click();
  await expect(rows).toHaveCount(5);
});

test("Footer links to the source and the issue tracker", async ({ page }) => {
  await page.goto("#/settings");
  const footer = page.locator("footer.app-footer");
  await expect(footer).toContainText("not published, endorsed, or approved by Paizo Inc.");
  await expect(footer.getByRole("link", { name: "Source on GitHub" })).toHaveAttribute(
    "href",
    "https://github.com/phynics/cast-iron",
  );
  await expect(footer.getByRole("link", { name: "Send feedback or report a problem" })).toHaveAttribute(
    "href",
    "https://github.com/phynics/cast-iron/issues",
  );
});
