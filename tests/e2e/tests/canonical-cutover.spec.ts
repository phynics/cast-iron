import { test, expect, goOffline } from "./fixtures";
import { installLibrary, waitControlled, waitReady } from "./helpers";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

interface FixtureEntry {
  id: string;
  kind: string;
  title: string;
}

interface FixtureLibrary {
  dir: string;
  version: string;
  aliasesSha256: string | null;
  cleanup: () => void;
}

const sha = (bytes: Buffer) => crypto.createHash("sha256").update(bytes).digest("hex");

function entry({ id, kind, title }: FixtureEntry) {
  return {
    id,
    kind,
    title,
    source_ids: ["legacy-book"],
    section: [],
    body: [{ t: "paragraph", content: [{ t: "text", v: `${title} body.` }] }],
    fields: { kind: "none" },
    tags: [],
    links: [],
    provenance: [],
    license_refs: [],
    relations: [],
  };
}

/**
 * A small real content library for the cutover test: schema 3, four required files, one
 * entry shard, and (for the canonical side) a manifest-listed alias map. The production
 * cutover is not live, so the fixture pair is the only way to exercise the client rewrite.
 */
function buildFixtureLibrary(
  label: string,
  entries: FixtureEntry[],
  aliases?: object,
): FixtureLibrary {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), `cast-iron-${label}-`));
  const files = new Map<string, Buffer>();
  const shard = entries.map(entry);
  // The client binary-searches the catalog, which the compiler emits sorted by unique id.
  // An unsorted fixture is out of contract and makes every catalog lookup unreliable.
  const catalogEntries = [...entries].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  files.set(
    "catalog.json",
    Buffer.from(
      JSON.stringify({
        sources: [{ id: "legacy-book", title: "Legacy Book", entry_count: entries.length }],
        entries: catalogEntries.map((item) => ({
          id: item.id,
          kind: item.kind,
          title: item.title,
          source: "legacy-book",
          shard: "entries/all.json",
        })),
      }),
    ),
  );
  files.set(
    "legal.json",
    Buffer.from(
      JSON.stringify({
        licenses: [],
        product_identity: "",
        community_use_notice: "Fixture library.",
        section_15: [],
        datasets: [],
      }),
    ),
  );
  files.set("coverage.json", Buffer.from("{}"));
  files.set("search.bin", Buffer.from("fixture search index"));
  files.set("entries/all.json", Buffer.from(JSON.stringify(shard)));
  if (aliases) files.set("aliases.json", Buffer.from(JSON.stringify(aliases)));

  const sorted = [...files.keys()].sort();
  const version = `v${crypto
    .createHash("sha256")
    .update(sorted.map((name) => `${name}:${sha(files.get(name)!)}\n`).join(""))
    .digest("hex")
    .slice(0, 16)}`;
  const manifest = {
    schema_version: 3,
    content_version: version,
    total_bytes: sorted.reduce((total, name) => total + files.get(name)!.length, 0),
    files: sorted.map((name) => ({
      path: `${version}/${name}`,
      sha256: sha(files.get(name)!),
      bytes: files.get(name)!.length,
    })),
    entry_count: entries.length,
    datasets: [],
  };
  for (const name of sorted) {
    const target = path.join(dir, "content", version, name);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, files.get(name)!);
  }
  fs.writeFileSync(path.join(dir, "content/manifest.json"), JSON.stringify(manifest));
  return {
    dir,
    version,
    aliasesSha256: files.has("aliases.json") ? sha(files.get("aliases.json")!) : null,
    cleanup: () => fs.rmSync(dir, { recursive: true, force: true }),
  };
}

const OLD_ENTRIES: FixtureEntry[] = [
  { id: "legacy/direct", kind: "rule", title: "Old Direct" },
  { id: "legacy/shared-a", kind: "rule", title: "Old Shared A" },
  { id: "legacy/shared-b", kind: "spell", title: "Old Shared B" },
  { id: "legacy/trait-a", kind: "rule", title: "Old Trait A" },
  { id: "legacy/trait-b", kind: "rule", title: "Old Trait B" },
  { id: "legacy/retired", kind: "rule", title: "Retired Rule" },
];

const NEW_ENTRIES: FixtureEntry[] = [
  { id: "ref/rule/direct", kind: "rule", title: "Direct Rule" },
  { id: "ref/rule/shared", kind: "rule", title: "Shared Rule" },
  { id: "ref/race/trait", kind: "rule", title: "Dwarf Trait" },
];

const ALIASES = {
  format: 1,
  legacy: { count: OLD_ENTRIES.length, sha256: "0".repeat(64) },
  entries: [
    { old: "legacy/direct", target: "ref/rule/direct", basis: "direct" },
    { old: "legacy/shared-a", target: "ref/rule/shared", basis: "shared" },
    { old: "legacy/shared-b", target: "ref/rule/shared", basis: "shared" },
    { old: "legacy/trait-a", target: "ref/race/trait", basis: "derived" },
    { old: "legacy/trait-b", target: "ref/race/trait", basis: "shared" },
  ],
  retired: [{ old: "legacy/retired", reason: "no_source", note: "fixture retirement" }],
};

async function storedIds(page: import("./fixtures").Page) {
  return page.evaluate(
    () =>
      new Promise<{ bookmarks: any[]; recent: any[]; marker: any }>((resolve, reject) => {
        const open = indexedDB.open("cast-iron");
        open.onerror = () => reject(open.error);
        open.onsuccess = () => {
          const db = open.result;
          const tx = db.transaction(["bookmarks", "meta"]);
          const bookmarks = tx.objectStore("bookmarks").getAll();
          const marker = tx.objectStore("meta").get("id_migration");
          const recent = tx.objectStore("meta").get("recent");
          tx.oncomplete = () => {
            db.close();
            resolve({
              bookmarks: bookmarks.result,
              recent: recent.result ?? [],
              marker: marker.result ?? null,
            });
          };
          tx.onerror = () => reject(tx.error);
        };
      }),
  );
}

test("Legacy bookmarks and recents migrate to canonical ids after a library cutover", async ({
  page,
  context,
  server,
  guard,
}) => {
  const old = buildFixtureLibrary("old", OLD_ENTRIES);
  const next = buildFixtureLibrary("new", NEW_ENTRIES, ALIASES);
  try {
    await server.overlay(old.dir);
    await page.goto("./");
    await installLibrary(page);
    await waitControlled(page);

    for (const id of OLD_ENTRIES.map((item) => item.id)) {
      await page.goto(`#/entry/${id}`);
      await expect(page.getByRole("heading", { level: 1 })).toContainText(
        OLD_ENTRIES.find((item) => item.id === id)!.title,
      );
      await page.locator(".bookmark-toggle").click();
      await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
    }

    await server.overlay(next.dir);
    await page.goto("#/settings");
    await page.getByRole("button", { name: "Check for updates", exact: true }).click();
    await expect(page.getByText("Newer library available")).toBeVisible();
    await page.getByRole("button", { name: "Download update" }).click();
    await expect(page.getByTestId("library-version")).toHaveText(next.version, {
      timeout: 60_000,
    });

    // The many:1 rows merged, the retired row kept its snapshot, and nothing was lost.
    await page.goto("#/");
    await waitReady(page);
    const rows = page.locator(".home-bookmarks > li");
    await expect(rows).toHaveCount(4);
    const titles = await rows.allInnerTexts();
    for (const title of ["Direct Rule", "Shared Rule", "Dwarf Trait", "Retired Rule"]) {
      expect(titles.some((text) => text.includes(title)), title).toBe(true);
    }
    await expect(rows.filter({ hasText: "Retired Rule" })).toContainText("Not in this library");

    const stored = await storedIds(page);
    const ids = stored.bookmarks.map((row) => row.entry_id).sort();
    expect(ids).toEqual([
      "legacy/retired",
      "ref/race/trait",
      "ref/rule/direct",
      "ref/rule/shared",
    ]);
    expect(stored.marker).toEqual({ aliases_sha256: next.aliasesSha256 });
    const recentIds = stored.recent.map((row: any) => row.id);
    expect(recentIds).toEqual([
      "legacy/retired",
      "ref/race/trait",
      "ref/rule/shared",
      "ref/rule/direct",
    ]);

    // A canonical page shows the star pressed, and an old URL replaces itself in place.
    await page.goto("#/entry/ref/rule/direct");
    await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
    await page.goto("#/entry/legacy/direct");
    await expect(page.getByRole("heading", { level: 1, name: "Direct Rule" })).toBeVisible();
    await expect(page).toHaveURL(/#\/entry\/ref\/rule\/direct$/);

    // A second load with the same alias hash changes nothing.
    const before = await storedIds(page);
    await page.reload();
    await waitReady(page);
    const after = await storedIds(page);
    expect(after.marker).toEqual(before.marker);
    expect(after.bookmarks).toEqual(before.bookmarks);
    expect(after.recent).toEqual(before.recent);

    // The rewrite and resolution are local: they still work with the network off. WebKit
    // surfaces the deliberately refused connection as a page error; that is the test behaving
    // as intended, not an app error. Its service-worker update check for sw.js is the request
    // that is refused, so both reported forms are allowed.
    await page.evaluate(() => navigator.serviceWorker.ready);
    guard.allow(/^pageerror: Load failed$/);
    guard.allow(/^pageerror: .*\/sw\.js due to access control checks\.$/);
    await goOffline(context, server);
    await page.goto("#/");
    await waitReady(page);
    await expect(page.locator(".home-bookmarks")).toContainText("Direct Rule");
    await page.goto("#/entry/ref/rule/shared");
    await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  } finally {
    old.cleanup();
    next.cleanup();
  }
});
