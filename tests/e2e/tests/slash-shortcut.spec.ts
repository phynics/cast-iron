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

test("slash works when the layout needs Shift, and ignores Ctrl", async ({ page }) => {
  await installLibrary(page);
  await expect(page.locator("#home-title")).toBeVisible();
  const search = page.getByRole("searchbox", { name: "Search the rules", exact: true });
  const press = (init: KeyboardEventInit) =>
    page.evaluate((init) => {
      (document.activeElement as HTMLElement | null)?.blur();
      document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "/", bubbles: true, cancelable: true, ...init }));
    }, init);

  // Ctrl+/ is left to the browser and assistive tools.
  await press({ ctrlKey: true });
  await expect(search).not.toBeFocused();

  // German and French layouts produce `/` with Shift held.
  await press({ shiftKey: true });
  await expect(search).toBeFocused();
});
