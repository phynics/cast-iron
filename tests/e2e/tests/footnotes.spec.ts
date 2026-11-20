import { installedTest as test, expect, loadCorpus } from "./fixtures";
import { installLibrary } from "./helpers";

// Footnotes (#507): the corpus writes a note as a level-six `footnote-` heading and one paragraph.
// The page shows the notes as one quiet "Notes" group after the rules text, smaller and in a
// secondary colour, with markers that link to a note and back. Expanded Summon Monster (Monster
// Summoner's Handbook, p. 28) has one printed note that the markers after some of the listed
// creatures cite; Aatheriexa has a printed stat-block note that two markers cite.

const corpus = loadCorpus();
const SUMMON_FEAT = "ref/feat/expanded-summon-monster";
const SUMMON_NOTE = "This creature is summoned with the celestial template";
const MONSTER = "ref/monster/aatheriexa";
const OTHER_MONSTER = "ref/monster/aether-elemental";

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
  test(`Expanded Summon Monster notes read as notes, not rules (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto(`#/entry/${SUMMON_FEAT}`);
    await expect(page.getByTestId("entry-title")).toHaveText("Expanded Summon Monster");
    const notes = page.getByRole("complementary", { name: "Notes" });
    await expect(notes).toBeVisible();
    await expect(notes.getByRole("heading", { level: 2, name: "Notes" })).toBeVisible();
    const items = notes.locator("li.note");
    await expect(items).toHaveCount(1);
    await expect(items.nth(0).locator(".note-label")).toHaveText("Note 1");
    const expected = noteTexts(SUMMON_FEAT);
    for (let i = 0; i < expected.length; i++) {
      await expect(items.nth(i).locator("p")).toHaveText(expected[i].replace(/\s+/g, " "));
    }
    await expect(items.nth(0)).toContainText(SUMMON_NOTE);

    // The group closes the body, and a note is not a section heading.
    await expect(page.locator(".rich > .notes")).toHaveCount(1);
    await expect(page.locator(".rich > *").last()).toHaveClass(/notes/);
    await expect(page.locator(".rich h6")).toHaveCount(0);

    // The markers sit in the lists of creatures: each one links to the note, and the note links
    // back to each of them.
    const markers = page.locator(".rich sup.fn-ref");
    const cited = await markers.count();
    expect(cited).toBeGreaterThan(1);
    await expect(markers.first().getByRole("link", { name: "Note 1" })).toHaveAttribute("href", /[?&]a=footnote-t$/);
    await expect(notes.locator(".note-back")).toHaveCount(cited);

    // Visibly secondary: smaller type and a different colour than the rules text.
    const rule = await style(page.locator(".rich > p").first());
    const note = await style(items.first().locator("p"));
    expect(note.size).toBeLessThan(rule.size);
    expect(note.color).not.toBe(rule.color);

    // The rules text is unchanged and none of it moved into the group.
    for (const text of rulesParagraphs(SUMMON_FEAT)) {
      await expect(page.locator(".rich > p", { hasText: text.replace(/\s+/g, " ").slice(0, 60) })).toHaveCount(1);
    }
    await expect(page.locator(".rich > p", { hasText: SUMMON_NOTE })).toHaveCount(0);
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
  await expect(markers.first().getByRole("link", { name: "Bonus feat, note B" })).toHaveAttribute("href", /[?&]a=footnote-b$/);
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

// De-emphasized markers (#517): the `B` after a bonus feat reads as a quiet annotation. It is
// smaller than the feat line, in the muted ink of the Notes group (not the accent), and still a
// link to its note with a name that says what it is.
for (const scheme of ["light", "dark"] as const) {
  for (const id of [MONSTER, OTHER_MONSTER]) {
    test(`The bonus-feat B is a quiet link to its note: ${id.split("/").pop()} (${scheme})`, async ({ page }) => {
      await page.emulateMedia({ colorScheme: scheme });
      await page.goto(`#/entry/${id}`);
      await expect(page.getByTestId("entry-title")).toBeVisible();
      const markers = page.locator(".rich p sup.fn-ref");
      expect(await markers.count()).toBeGreaterThan(0);

      // The colour the notes use, resolved for this theme.
      const muted = await page.evaluate(() => {
        const probe = document.createElement("span");
        probe.style.color = "var(--ink-muted)";
        document.body.append(probe);
        const color = getComputedStyle(probe).color;
        probe.remove();
        return color;
      });
      const accent = await page.evaluate(() => {
        const probe = document.createElement("span");
        probe.style.color = "var(--accent)";
        document.body.append(probe);
        const color = getComputedStyle(probe).color;
        probe.remove();
        return color;
      });
      const notesColor = (await style(page.locator("aside.notes li.note p").first())).color;
      expect(notesColor).toBe(muted);

      for (let i = 0; i < (await markers.count()); i++) {
        const marker = markers.nth(i);
        const link = marker.getByRole("link", { name: "Bonus feat, note B" });
        await expect(link).toHaveCount(1);
        await expect(link).toHaveText("B");
        // A monster with several stat blocks has one note per block (`footnote-b`, `footnote-b-2`).
        await expect(link).toHaveAttribute("href", /[?&]a=footnote-b(-\d+)?$/);

        // Against the feat text: the paragraph that holds the marker.
        const line = await style(marker.locator("xpath=ancestor::p[1]"));
        const mark = await style(link);
        expect(mark.size).toBeLessThan(line.size);
        expect(mark.color).toBe(muted);
        expect(mark.color).not.toBe(line.color);
        expect(mark.color).not.toBe(accent);
        const weight = await link.evaluate((el) => getComputedStyle(el).fontWeight);
        expect(Number(weight)).toBeLessThanOrEqual(400);
        const underline = await link.evaluate((el) => getComputedStyle(el).textDecorationLine);
        expect(underline).toBe("none");

        // The hit area is larger than the glyph and takes no space.
        const hit = await link.evaluate((el) => {
          const css = getComputedStyle(el, "::after");
          return { w: parseFloat(css.width), h: parseFloat(css.height), position: css.position };
        });
        expect(hit.position).toBe("absolute");
        expect(hit.h).toBeGreaterThanOrEqual(24);
        expect(hit.w).toBeGreaterThanOrEqual(16);
      }

      // Hover and focus bring the accent back, so the link stays discoverable.
      const first = markers.first().getByRole("link");
      await first.focus();
      await expect.poll(async () => (await style(first)).color).toBe(accent);

      // The marker still leads to the note.
      await first.click();
      await expect(page.locator("#h-footnote-b")).toBeInViewport();
    });
  }
}
