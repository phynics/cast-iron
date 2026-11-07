import { test, expect, loadCorpus, loadBacklinks } from "./fixtures";
import { bodyLinks, installLibrary, waitControlled } from "./helpers";

const corpus = loadCorpus();
const backlinks = loadBacklinks();

/**
 * A source entry whose body links to a target, where the compiler kept a reverse row.
 * Prefer a target with a short list so the publication group opens by default; fall back
 * to any target and let the test open the groups.
 */
function choosePair() {
  const candidates = [...backlinks.entries()].filter(
    ([target, sources]) => corpus.entries.has(target) && sources.length > 0 && sources.length <= 3,
  );
  const pools = candidates.length > 0 ? [candidates, [...backlinks.entries()]] : [[...backlinks.entries()]];
  for (const pool of pools) {
    for (const [targetId, sources] of pool) {
      const target = corpus.entries.get(targetId);
      if (!target) continue;
      for (const row of sources) {
        const source = corpus.entries.get(row.source);
        if (!source || source.id === targetId) continue;
        if ((source.relations ?? []).some((relation) => relation.target === targetId)) continue;
        const link = bodyLinks(source.body).find(
          (candidate) => candidate.target === targetId && candidate.text.trim().length > 0,
        );
        if (link) return { target, source, link };
      }
    }
  }
  return undefined;
}

test("Show all builds its rows only when it opens", async ({ page }) => {
  const target = [...backlinks.entries()].find(
    ([targetId, sources]) => sources.length > 8 && corpus.entries.has(targetId),
  )?.[0];
  expect(target, "the corpus has an entry with a long backlink group").toBeTruthy();

  await installLibrary(page);
  await page.goto(`#/entry/${target}`);
  const section = page.locator("#entry-backlinks");
  await expect(section).toBeVisible();

  // A long group starts closed; open it so "Show all" is on screen.
  const group = section.locator("details.backlink-group:has(details.backlink-more)").first();
  await expect(group).toBeAttached();
  await group.locator("summary").first().click();
  await expect(group).toHaveAttribute("open", "");

  const more = group.locator("details.backlink-more").first();
  await expect(more).toBeVisible();
  await expect(more).not.toHaveAttribute("open", "");
  // The closed disclosure holds no rows: the preview above it is the only built list.
  await expect(more.locator("li")).toHaveCount(0);

  await more.locator("summary").click();
  await expect(more).toHaveAttribute("open", "");
  await expect(more.locator("li").first()).toBeVisible();
  expect(await more.locator("li").count()).toBeGreaterThan(0);
});

test("Referenced by lists a body link's source and returns to it offline", async ({
  page,
  context,
}) => {
  const pair = choosePair();
  expect(pair, "the compiled corpus has a body link with a reverse-index row").toBeTruthy();
  const { target, source } = pair!;

  await installLibrary(page);
  await waitControlled(page);
  await context.setOffline(true);

  // Follow the ordinary body link from the source.
  await page.goto(`#/entry/${source.id}`);
  await expect(page.getByRole("heading", { level: 1, name: source.title })).toBeVisible();
  const bodyLink = page.locator(`.rich a[href^="#/entry/${target.id}"]`).first();
  await expect(bodyLink, "the source body links to the target").toBeVisible();
  await bodyLink.click();
  await expect(page.getByRole("heading", { level: 1, name: target.title })).toBeVisible();

  // The target now lists the source under "Referenced by", read from the installed cache.
  const section = page.locator("#entry-backlinks");
  await expect(section.getByRole("heading", { level: 2, name: "Referenced by" })).toBeVisible();
  for (const summary of await section.locator("details.backlink-group:not([open]) > summary").all()) {
    await summary.click();
  }
  for (const summary of await section.locator("details.backlink-more:not([open]) > summary").all()) {
    await summary.click();
  }
  const back = section.locator(`a[href="#/entry/${source.id}"]`).first();
  await expect(back).toHaveText(source.title);

  // Keyboard, not just pointer: focus the row and activate it with Enter.
  await back.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { level: 1, name: source.title })).toBeVisible();
  await expect(page).toHaveURL(
    new RegExp(`#/entry/${source.id.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`),
  );
});
