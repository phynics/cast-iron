import { test, expect } from "./fixtures";
import { buildLibraryWithAbilityType, firstEntry, installLibrary, waitControlled } from "./helpers";

// The ability type of a single-power class option (owner ruling 63 of #540; decision
// `2026-11-27-class-option-ability-type`): the entry header shows it after the title, "Agony (Su)", in
// the muted italic of a feature head's tag. No pinned corpus types an option yet, so an overlay gives
// one `fields.ability_type`. The stored value has no parentheses; the page draws them.

const option = firstEntry("a class option", (entry) => entry.kind === "class_option");
const control = firstEntry("another class option", (entry) => entry.kind === "class_option" && entry.id !== option.id);

test("A class option with an ability type shows it after the title, in italics", async ({ page, server }) => {
  // The overlay and the first install of the library take longer than the default budget.
  test.setTimeout(300_000);
  const variant = buildLibraryWithAbilityType(option.id, "Ex or Sp");
  try {
    await server.overlay(variant.dir);
    await installLibrary(page);
    await waitControlled(page);

    await page.goto(`#/entry/${option.id}`);
    const title = page.getByTestId("entry-title");
    // The heading's name carries the tag with a space, as the printed name does.
    await expect(title).toHaveText(`${option.title} (Ex or Sp)`);
    await expect(page.getByRole("heading", { level: 1, name: `${option.title} (Ex or Sp)`, exact: true })).toBeVisible();
    // The tag is its own element after the title text: italic, muted, not bold.
    const tag = title.locator(".ability-tag");
    await expect(tag).toHaveText("(Ex or Sp)");
    const style = await tag.evaluate((el) => {
      const own = getComputedStyle(el);
      const heading = getComputedStyle(el.parentElement!);
      return { fontStyle: own.fontStyle, fontWeight: Number(own.fontWeight), color: own.color, headingColor: heading.color };
    });
    expect(style.fontStyle).toBe("italic");
    expect(style.fontWeight).toBeLessThan(600);
    expect(style.color).not.toBe(style.headingColor);
    // The title of the page, the star and the breadcrumb stay without the tag.
    await expect(page).toHaveTitle(new RegExp(`^${option.title.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`));
    await expect(page.locator(".entry-title-row .star-btn")).not.toHaveAttribute("aria-label", /\(Ex or Sp\)/);

    // An option without the key has the plain title.
    await page.goto(`#/entry/${control.id}`);
    await expect(page.getByTestId("entry-title")).toHaveText(control.title);
    await expect(page.locator(".entry-head .ability-tag")).toHaveCount(0);
  } finally {
    await server.overlay(null);
    variant.cleanup();
  }
});
