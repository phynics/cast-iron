import { installedTest as test, expect, loadCorpus } from "./fixtures";
import { installLibrary } from "./helpers";

// Footnotes (#507): the corpus writes a note as a level-six `footnote-` heading and one paragraph.
// The page shows the notes as one quiet "Notes" group after the rules text, smaller and in a
// secondary colour, with markers that link to a note and back. Summon Monster II has two editorial
// notes whose markers are missing from the text; Aatheriexa has a printed stat-block note that two
// markers cite.

const corpus = loadCorpus();
const SUMMON = "ref/spell/summon-monster-2";
const MONSTER = "ref/monster/aatheriexa";

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

const inlineText = (items: any[] = []): string =>
  items.map((item) => (item.t === "text" ? item.v : inlineText(item.c ?? []))).join("");
const isNote = (block: any) => block.t === "heading" && block.level === 6 && String(block.anchor ?? "").startsWith("footnote-");

/** The paragraph texts of an entry body that are not a note: heading and the paragraph after it. */
function rulesParagraphs(id: string): string[] {
  const body: any[] = corpus.entries.get(id)!.body;
  const out: string[] = [];
  for (let i = 0; i < body.length; i++) {
    if (isNote(body[i])) {
      i += 1;
      continue;
    }
    if (body[i].t === "paragraph") out.push(inlineText(body[i].content).trim());
  }
  return out;
}

function noteTexts(id: string): string[] {
  const body: any[] = corpus.entries.get(id)!.body;
  return body.flatMap((block, i) => (isNote(block) ? [inlineText(body[i + 1].content).trim()] : []));
}

const style = (locator: ReturnType<import("@playwright/test").Page["locator"]>) =>
  locator.evaluate((el) => {
    const css = getComputedStyle(el);
    return { size: parseFloat(css.fontSize), color: css.color };
  });

for (const scheme of ["light", "dark"] as const) {
  test(`Summon Monster II notes read as notes, not rules (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto(`#/entry/${SUMMON}`);
    await expect(page.getByTestId("entry-title")).toHaveText("Summon Monster II");
    const notes = page.getByRole("complementary", { name: "Notes" });
    await expect(notes).toBeVisible();
    await expect(notes.getByRole("heading", { level: 2, name: "Notes" })).toBeVisible();
    const items = notes.locator("li.note");
    await expect(items).toHaveCount(2);
    await expect(items.nth(0).locator(".note-label")).toHaveText("Note 1");
    await expect(items.nth(1).locator(".note-label")).toHaveText("Note 2");
    const expected = noteTexts(SUMMON);
    for (let i = 0; i < expected.length; i++) {
      await expect(items.nth(i).locator("p")).toHaveText(expected[i].replace(/\s+/g, " "));
    }
    await expect(items.nth(0)).toContainText("Consult your GM");

    // The group closes the body, and a note is not a section heading.
    await expect(page.locator(".rich > .notes")).toHaveCount(1);
    await expect(page.locator(".rich > *").last()).toHaveClass(/notes/);
    await expect(page.locator(".rich h6")).toHaveCount(0);

    // Markers are missing from the text: the notes still render and have nothing to link back to.
    await expect(page.locator(".rich sup.fn-ref")).toHaveCount(0);
    await expect(notes.locator(".note-back")).toHaveCount(0);

    // Visibly secondary: smaller type and a different colour than the rules text.
    const rule = await style(page.locator(".rich > p").first());
    const note = await style(items.first().locator("p"));
    expect(note.size).toBeLessThan(rule.size);
    expect(note.color).not.toBe(rule.color);

    // The rules text is unchanged and none of it moved into the group.
    for (const text of rulesParagraphs(SUMMON)) {
      await expect(page.locator(".rich > p", { hasText: text.replace(/\s+/g, " ").slice(0, 60) })).toHaveCount(1);
    }
    await expect(page.locator(".rich > p", { hasText: "Consult your GM" })).toHaveCount(0);
  });
}

test("A printed stat-block note moves to Notes, and its markers link to it and back", async ({ page }) => {
  await page.goto(`#/entry/${MONSTER}`);
  await expect(page.getByTestId("entry-title")).toBeVisible();
  const notes = page.getByRole("complementary", { name: "Notes" });
  await expect(notes.locator("li.note")).toHaveCount(1);
  const [text] = noteTexts(MONSTER);
  await expect(notes.locator("li.note p")).toHaveText(text);
  await expect(notes.locator("li.note .note-label")).toHaveText("Note B");

  // Only the note's paragraph moved: the stat block text after it is still in the body.
  const rules = rulesParagraphs(MONSTER);
  expect(rules.some((paragraph) => paragraph.includes("Environment:"))).toBe(true);
  await expect(page.locator(".rich > p", { hasText: "Environment:" })).toHaveCount(1);
  await expect(notes).not.toContainText("Environment:");
  await expect(page.locator(".rich h6")).toHaveCount(0);

  // Two markers cite the note, in the text of the Feats line; each links to the note.
  const markers = page.locator(".rich sup.fn-ref");
  await expect(markers).toHaveCount(2);
  await expect(markers.first().getByRole("link", { name: "Note B" })).toHaveAttribute("href", /[?&]a=footnote-b$/);
  const backs = notes.locator(".note-back");
  await expect(backs).toHaveCount(2);

  // Quiet: smaller type than the paragraph that cites it, in a different colour.
  const rule = await style(page.locator(".rich > p", { hasText: "Environment:" }));
  const note = await style(notes.locator("li.note p"));
  expect(note.size).toBeLessThan(rule.size);
  expect(note.color).not.toBe(rule.color);

  await markers.first().getByRole("link").click();
  await expect(page.locator("#h-footnote-b")).toBeInViewport();
  // The page keeps the notes after the jump, and the way back lands on the marker.
  await page.locator("#h-footnote-b .note-back").first().click();
  await expect(page.locator("sup.fn-ref").first()).toBeInViewport();
  await expect(page.locator("#h-footnote-b")).toHaveCount(1);
});
