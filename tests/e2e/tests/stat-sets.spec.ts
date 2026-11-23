import { installedTest as test, expect, loadCorpus } from "./fixtures";
import { installLibrary } from "./helpers";

// A creature that prints several stat sets lists them on its entry page (#166). The pinned corpus
// has real sets (corpus v0.1.41: Air Elemental and the other elementals, Waldgeist, Benaioh and
// the goblin vulture pilot), so the spec reads the compiled library with no overlay. The
// derivation, the drift rule and the CR facet are the native tests' business
// (`cast_iron_schema::stat_sets`, the compiler's corpus tests).

const corpus = loadCorpus();
const TARGET = "ref/monster/air-elemental";
const SECOND = "ref/monster/waldgeist";
const CONTROL = [...corpus.entries.values()].find(
  (entry) => entry.kind === "creature" && entry.id !== TARGET && !((entry as any).stat_sets ?? []).length,
);

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

test("a creature with several stat sets lists them with their rating and jumps to a set", async ({ page }) => {
  const sets: any[] = (corpus.entries.get(TARGET) as any)?.stat_sets ?? [];
  expect(sets.map((set) => set.anchor), "Air Elemental has its six sets in the compiled library").toEqual([
    "stat-small",
    "stat-medium",
    "stat-large",
    "stat-huge",
    "stat-greater",
    "stat-elder",
  ]);

  const list = page.getByRole("navigation", { name: "Stat sets" });

  await test.step("the sets are listed above the body, in order, with CR and size, named as one", async () => {
    await page.goto(`#/entry/${TARGET}`);
    await expect(page.getByTestId("entry-title")).toBeVisible();
    await expect(list).toHaveCount(1);
    const links = list.getByRole("link");
    await expect(links).toHaveCount(6);
    await expect(links.nth(0)).toHaveAccessibleName("Small Air Elemental, CR 1 · Small");
    await expect(links.nth(1)).toHaveAccessibleName("Medium Air Elemental, CR 3 · Medium");
    await expect(links.nth(2)).toHaveAccessibleName("Large Air Elemental, CR 5 · Large");
    await expect(links.nth(3)).toHaveAccessibleName("Huge Air Elemental, CR 7 · Huge");
    await expect(links.nth(4)).toHaveAccessibleName("Greater Air Elemental, CR 9 · Huge");
    await expect(links.nth(5)).toHaveAccessibleName("Elder Air Elemental, CR 11 · Huge");
    await expect(links.nth(5)).toHaveAttribute("href", `#/entry/${TARGET}?a=stat-elder`);
    // The list comes before the body it points into.
    const listTop = (await list.boundingBox())!.y;
    const bodyTop = (await page.locator(".rich").first().boundingBox())!.y;
    expect(listTop).toBeLessThan(bodyTop);
  });

  await test.step("a link scrolls to its heading and the list keeps working after it", async () => {
    await list.getByRole("link", { name: /Elder Air Elemental/ }).click();
    await expect(page).toHaveURL(new RegExp(`#/entry/${TARGET}\\?a=stat-elder$`));
    await expect(page.locator("#h-stat-elder")).toBeInViewport();
    await list.getByRole("link", { name: /Large Air Elemental/ }).click();
    await expect(page.locator("#h-stat-large")).toBeInViewport();
  });

  await test.step("a set with a rating and no size is named by its rating alone", async () => {
    await page.goto(`#/entry/${SECOND}`);
    await expect(page.getByTestId("entry-title")).toBeVisible();
    const links = list.getByRole("link");
    await expect(links).toHaveCount(2);
    await expect(links.nth(0)).toHaveAccessibleName("Waldgeist, CR 8 · Small");
    await expect(links.nth(1)).toHaveAccessibleName("Possessed Tree, CR 5");
  });

  await test.step("a creature with no stat sets shows no list", async () => {
    expect(CONTROL, "the compiled library has a creature without derived stat sets").toBeTruthy();
    await page.goto(`#/entry/${CONTROL!.id}`);
    await expect(page.getByTestId("entry-title")).toBeVisible();
    await expect(page.getByRole("navigation", { name: "Stat sets" })).toHaveCount(0);
  });
});
