import { installedTest as test, expect } from "./fixtures";
import { installLibrary, quickField, quickSearch, waitResults } from "./helpers";

test("slash opens Quick search from the page and types inside it", async ({ page }) => {
  await installLibrary(page);
  await expect(page.locator("#home-title")).toBeVisible();

  // Start from the document body, not a field, and prove the shortcut does not navigate.
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  const before = page.url();
  await page.keyboard.press("/");

  const search = quickField(page);
  await expect(search).toBeFocused();
  expect(page.url()).toBe(before);

  // Inside the box, `/` is a character. It must not be swallowed as a shortcut.
  await page.keyboard.type("a/b");
  await expect(search).toHaveValue("a/b");
  await page.keyboard.press("Escape");
  await expect(quickSearch(page)).toBeHidden();
  expect(page.url()).toBe(before);
});

test("slash works when the layout needs Shift, and ignores Ctrl", async ({ page }) => {
  await installLibrary(page);
  await expect(page.locator("#home-title")).toBeVisible();
  const press = (init: KeyboardEventInit) =>
    page.evaluate((init) => {
      (document.activeElement as HTMLElement | null)?.blur();
      document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "/", bubbles: true, cancelable: true, ...init }));
    }, init);

  // Ctrl+/ is left to the browser and assistive tools.
  await press({ ctrlKey: true });
  await expect(quickSearch(page)).toBeHidden();

  // German and French layouts produce `/` with Shift held.
  await press({ shiftKey: true });
  await expect(quickSearch(page)).toBeVisible();
  await expect(quickField(page)).toBeFocused();
});

test("slash focuses the field of the results page instead of opening a second one", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/search?q=fire");
  await waitResults(page, "fire");
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  await page.keyboard.press("/");
  await expect(page.locator("#search-input")).toBeFocused();
  await expect(quickSearch(page)).toBeHidden();
});
