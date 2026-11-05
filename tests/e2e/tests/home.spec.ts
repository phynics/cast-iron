import { installedTest as test, expect } from "./fixtures";
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

  // The old Home sections are gone: the directory replaces the tiles and the books block.
  await expect(page.locator(".home-page .tiles")).toHaveCount(0);
  await expect(page.locator(".home-books")).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Browse by category" })).toHaveCount(0);
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
  await expect(footer).toContainText("(needs a connection)");
});
