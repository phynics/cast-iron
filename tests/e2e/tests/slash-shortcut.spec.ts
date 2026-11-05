import { test, expect } from "./fixtures";
import { installLibrary } from "./helpers";

test("slash focuses the search box from the page and types inside it", async ({ page }) => {
  await installLibrary(page);
  await expect(page.locator("#home-title")).toBeVisible();

  // Start from the document body, not a field, and prove the shortcut does not navigate.
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  const before = page.url();
  await page.keyboard.press("/");

  const search = page.getByRole("searchbox", { name: "Search the rules", exact: true });
  await expect(search).toBeFocused();
  expect(page.url()).toBe(before);

  // Inside the box, `/` is a character. It must not be swallowed as a shortcut.
  await page.keyboard.type("a/b");
  await expect(search).toHaveValue("a/b");
});
