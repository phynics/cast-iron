import { test, expect, corpusLabel, distManifest, loadCorpus } from "./fixtures";
import {
  activeLibrary,
  buildNewerManifest,
  buildUpdatedLibrary,
  cacheContents,
  contentCaches,
  corpusEntry,
  deleteMeta,
  firstBookPlacement,
  installLibrary,
  openBookGroups,
  storedMeta,
  waitControlled,
  waitHomeIdle,
  waitReady,
  writeMeta,
} from "./helpers";

const v1 = distManifest().content_version;
const corpus = loadCorpus();
const entry = corpusEntry("spell", "Fireball");
const placedIn = firstBookPlacement(entry.id)?.source;

/** A publication the reader shows, other than the one that owns the starred entry. */
const otherSource = corpus.catalog.sources.find((source) =>
  source.entry_count > 0 && source.id !== placedIn);

test("Content update keeps user records", async ({ page, context, server }) => {
  // The canonical library is about 220 MB decoded (45 MB stored); building the replacement copy and
  // verifying it take longer than the default test budget on a local disk.
  test.setTimeout(300_000);
  expect(otherSource, "a second publication in the compiled library").toBeTruthy();
  const marker = "UPDATED-LIBRARY-MARKER";
  const next = buildUpdatedLibrary(marker);
  try {
    expect(next.version).not.toBe(v1);
    await installLibrary(page);
    await waitControlled(page);

    // User records under v1: a star and a disabled publication.
    await page.goto(`#/entry/${entry.id}`);
    await page.locator(".bookmark-toggle").click();
    await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
    await page.goto("#/books/manage");
    await openBookGroups(page);
    await page.getByRole("checkbox", { name: `Active ${otherSource!.title}`, exact: true }).uncheck();
    await expect(page.getByRole("checkbox", { name: `Active ${otherSource!.title}`, exact: true })).toBeEnabled();

    // No update yet: the check says so.
    await page.goto("#/settings");
    await page.getByRole("button", { name: "Check now", exact: true }).click();
    await expect(page.getByTestId("update-status")).toContainText("up to date");

    // A second page opened under v1. It stays on the old library while a replacement downloads;
    // opening it first keeps the check independent of page-load time on a warm local server.
    const reader = await context.newPage();
    await reader.goto(`#/entry/${entry.id}`);
    await expect(reader.getByRole("heading", { level: 1, name: entry.title })).toBeVisible();
    await reader.goto("#/legal");
    await expect(reader.locator("#community-use + .legal-text")).not.toContainText(marker);
    expect((await activeLibrary(reader)).content_version).toBe(v1);

    // The server now offers v2. One of its files is slow, so the update is in flight while the
    // open second page is checked below.
    const slowFile = next.manifest.files.find((file) =>
      file.path.includes("/entries/") && /\.json(\.gz)?$/.test(file.path))!;
    const slow = slowFile.path;
    await server.overlay(next.dir);
    // Keep v2 visibly in flight long enough that every page agrees on v1. The server sends the
    // stored file in 16 KB chunks, `ms` apart. A stored shard is gzip since schema 8, a fifth of
    // its decoded size, so the delay is sized from the file to hold the download for about 40 s.
    const chunks = Math.ceil(slowFile.bytes / 16384);
    await server.hook(`content/${slow}`, "slow", { ms: Math.ceil(40_000 / chunks) });
    await page.getByRole("button", { name: "Check now", exact: true }).click();
    // The update row says each thing once (#584): one size, one button, one reassurance, and
    // nothing that belongs to the first install.
    const updates = page.locator("#set-updates");
    const offer = page.getByTestId("library-update");
    await expect(offer.getByRole("button", { name: "Download update" })).toHaveCount(1);
    await expect(offer.locator(".install-panel .facts")).toHaveCount(0);
    await expect(updates.getByText("On iPhone?")).toHaveCount(0);
    await expect(updates.getByTestId("download-size")).toHaveCount(0);
    // The section also names the installed library's size, so the offer is counted by itself.
    const text = (await offer.innerText()).replace(/\s+/g, " ");
    expect(text.match(/\d+(\.\d+)? [MG]B/g) ?? [], "the size appears once").toHaveLength(1);
    expect((await updates.innerText()).match(/Your stars/g) ?? [], "the reassurance appears once").toHaveLength(1);
    await expect(offer).toContainText("Your stars and book choices stay. The current library keeps working until the new one is verified.");
    await page.getByRole("button", { name: "Download update" }).click();
    await expect(page.getByTestId("install-status")).toContainText(/Downloading/);

    // While v2 downloads, the open page still reads v1 and shows the old legal text.
    expect((await activeLibrary(reader)).content_version).toBe(v1);
    await expect(reader.locator("#community-use + .legal-text")).not.toContainText(marker);
    await expect.poll(async () => contentCaches(await cacheContents(page)).sort()).toEqual([`cast-iron-content-${v1}`, `cast-iron-content-${next.version}`].sort());
    // A page opened now, with v2 still downloading, must also start on v1 (AGENTS.md rule 7:
    // a partly installed library never becomes active). The slow file keeps v2 in flight, and
    // the active pointer read on the fresh page is itself the proof: it would name v2 only
    // after activation. Do not add a later cache check here; activation may legitimately
    // finish while this page is being checked.
    const fresh = await context.newPage();
    await fresh.goto(`#/entry/${entry.id}`);
    await expect(fresh.getByRole("heading", { level: 1, name: entry.title })).toBeVisible();
    expect((await activeLibrary(fresh)).content_version).toBe(v1);
    await fresh.goto("#/legal");
    await expect(fresh.locator("#community-use + .legal-text")).not.toContainText(marker);
    await fresh.close();
    await reader.close();

    // Activation: the new version is active, the old cache is gone, the records are intact.
    // Wait for activation itself, then read the label.
    await expect
      .poll(async () => (await activeLibrary(page))?.content_version, { timeout: 120_000 })
      .toBe(next.version);
    await expect(page.getByTestId("library-version")).toHaveText(corpusLabel(next.manifest), { timeout: 120_000 });
    // The install panel's status line is gone with the old Reader; focus is on main, not the body.
    await expect.poll(() => page.evaluate(() => document.activeElement?.id)).toBe("main");
    await expect.poll(async () => contentCaches(await cacheContents(page))).toEqual([`cast-iron-content-${next.version}`]);
    const cached = new Set((await cacheContents(page))[`cast-iron-content-${next.version}`]);
    expect(next.manifest.files.every((f) => cached.has(`content/${f.path}`))).toBe(true);

    await page.goto("#/legal");
    await expect(page.locator("#community-use + .legal-text")).toContainText(marker);
    await page.goto("#/bookmarks");
    await expect(page.locator(".home-bookmarks")).toContainText(entry.title);
    await expect(page.locator(".home-bookmarks")).not.toContainText("not in this library");
    await page.goto("#/books/manage");
    await openBookGroups(page);
    await expect(page.getByRole("checkbox", { name: `Active ${otherSource!.title}`, exact: true })).not.toBeChecked();
    await page.goto(`#/entry/${entry.id}`);
    await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  } finally {
    next.cleanup();
  }
});

test("The daily check surfaces a newer library, and Settings puts Updates first", async ({ page, server }) => {
  const next = buildNewerManifest();
  try {
    await installLibrary(page);
    await waitControlled(page);
    // Forget the check the installation made, so the next start asks again at once.
    await deleteMeta(page, "update_check");
    await server.overlay(next.dir);
    await waitHomeIdle(page);
    await page.reload();
    await waitReady(page);

    // The start check finds the newer manifest, and the header says so without a tap.
    const action = page
      .locator("header.app-header")
      .getByRole("button", { name: "Library update ready. Open Settings" });
    await expect(action).toBeVisible();
    await action.click();

    // Settings → Updates, focused and first, with the library row before the app row.
    await expect(page.locator("#set-updates")).toBeFocused();
    const sections = page.locator("main section.settings-section");
    await expect(sections.first()).toHaveAttribute("id", "set-updates");
    const row = page.getByTestId("library-update");
    await expect(row.getByText("Update available", { exact: true })).toBeVisible();
    await expect(row).toContainText(next.manifest.corpus_tag ?? next.version);
    await expect(page.getByTestId("last-checked")).toContainText("Last checked:");
    await expect(page.getByRole("button", { name: "Check now", exact: true })).toBeVisible();

    // A check only reads the manifest: the active library and its cache are untouched.
    expect((await activeLibrary(page)).content_version).toBe(v1);
    expect(contentCaches(await cacheContents(page))).toEqual([`cast-iron-content-${v1}`]);
  } finally {
    next.cleanup();
  }
});

test("With the automatic check off, no manifest request happens at start", async ({ page, server }) => {
  await installLibrary(page);
  await waitControlled(page);
  await page.goto("#/settings");
  const toggle = page.getByRole("checkbox", { name: "Check for updates automatically" });
  await expect(toggle).toBeChecked();
  await toggle.uncheck();
  await expect.poll(() => storedMeta(page, "auto_update_check")).toBe(false);

  // An enabled app would ask again as soon as the last check is forgotten.
  await deleteMeta(page, "update_check");
  await server.clearLog();
  await page.reload();
  await waitReady(page);
  // The switch renders from the stored value that the start path read before it would check.
  await expect(page.getByRole("checkbox", { name: "Check for updates automatically" })).not.toBeChecked();
  await expect.poll(() => storedMeta(page, "update_check")).toBeNull();
  expect((await server.requests()).filter((p) => p === "content/manifest.json")).toEqual([]);
});

test("A stored newer library mounts no installer until the reader asks", async ({ page, server }) => {
  await installLibrary(page);
  await waitControlled(page);
  const library = await activeLibrary(page);
  // A record from an earlier session: it knows the version and size, but holds no manifest.
  await writeMeta(page, "auto_update_check", false);
  await writeMeta(page, "update_check", {
    last_checked: Date.now(),
    known_version: "v0000000000000000",
    known_bytes: library.total_bytes,
  });
  await server.clearLog();
  await waitHomeIdle(page);
  await page.reload();
  await waitReady(page);

  // Opening Settings must not fetch the manifest just because a record names a newer library.
  await page.goto("#/settings");
  const row = page.getByTestId("library-update");
  await expect(row.getByText("Update available", { exact: true })).toBeVisible();
  await expect(row).toContainText("v0000000000000000");
  await expect(row.locator(".install-panel")).toHaveCount(0);
  await expect(row.getByRole("button", { name: "Check now", exact: true })).toBeVisible();
  await expect
    .poll(async () => (await server.requests()).filter((p) => p === "content/manifest.json"))
    .toEqual([]);
  // The stored record is untouched: no check ran and wrote a new one.
  await expect
    .poll(async () => (await storedMeta(page, "update_check"))?.known_version)
    .toBe("v0000000000000000");
});

test("Offline, the automatic check asks for nothing and Settings still works", async ({ openProfile, server }) => {
  let context = await openProfile();
  let page = await context.newPage();
  await installLibrary(page);
  await waitControlled(page);
  // The install session made its check, so the offline session has a time to show.
  await expect.poll(() => storedMeta(page, "update_check")).not.toBeNull();
  await context.close();

  // A new session with no network from its first navigation.
  await server.clearLog();
  context = await openProfile({ offline: true });
  page = await context.newPage();
  // Offline hides the request from the server, so record every attempt on the page as well.
  await page.addInitScript(() => {
    const original = window.fetch.bind(window);
    (window as any).__fetches = [];
    window.fetch = (input: RequestInfo | URL, init?: RequestInit) => {
      const url = typeof input === "string" ? input : input instanceof URL ? input.href : input.url;
      (window as any).__fetches.push(url);
      return original(input, init);
    };
  });
  await page.goto("./");
  await waitReady(page);
  await expect(page.locator("#home-title")).toBeVisible();
  await page.goto("#/settings");
  await expect(page.getByRole("heading", { name: "Settings", exact: true })).toBeVisible();
  await expect(page.getByTestId("last-checked")).toContainText("Last checked:");
  await expect
    .poll(() =>
      page.evaluate(() =>
        ((window as any).__fetches as string[]).filter((url) => url.includes("content/manifest.json")),
      ),
    )
    .toEqual([]);
  expect((await server.requests()).filter((p) => p === "content/manifest.json")).toEqual([]);
});

/** The `h2` texts of Settings, in page order (#584). */
const SECTIONS = ["Version & updates", "Appearance", "Books", "Library", "Install", "Storage", "Help"];

async function expectSettingsPage(page: import("@playwright/test").Page) {
  await expect(page.getByRole("heading", { level: 1, name: "Settings", exact: true })).toBeVisible();
  await expect(page.locator("main section.settings-section > h2")).toHaveText(SECTIONS);
  // Every section is named by its own h2.
  const unnamed = await page.locator("main section.settings-section").evaluateAll((sections) =>
    sections.filter((section) => {
      const id = section.getAttribute("aria-labelledby");
      return !id || document.getElementById(id)?.tagName !== "H2";
    }).length);
  expect(unnamed).toBe(0);
  await expect(page.locator("main section.settings-section").first()).toHaveAttribute("id", "set-updates");
  // No maintainer words: those live in Diagnostics.
  await expect(page.locator("main")).not.toContainText(/Content format|Corpus|manifest|schema/);
  // The two version rows keep their values; the rest of the page names no hash.
  await expect(page.getByTestId("library-version")).not.toHaveText("Checking…");
  await expect(page.getByTestId("shell-version")).not.toHaveText("Checking…");
}

test("Settings has seven sections in a fixed order, Version & updates holds both versions, Help links out, and Verify writes a status", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await page.goto("#/settings");
  await expectSettingsPage(page);

  const help = page.locator("section[aria-labelledby=set-help]");
  await expect(help.getByRole("link", { name: "Diagnostics", exact: true })).toHaveAttribute("href", "#/settings/diagnostics");
  await expect(help.getByRole("link", { name: "Legal and attribution", exact: true })).toHaveAttribute("href", "#/legal");
  const report = help.getByRole("link", { name: "Report a problem on GitHub (opens in a new tab)" });
  await expect(report).toHaveAttribute("target", "_blank");
  await expect(report).toHaveText("Report a problem");
  await expect(page.locator("section[aria-labelledby=set-books]").getByRole("link", { name: "Manage books", exact: true })).toHaveAttribute("href", "#/books/manage");
  // Content format is a Diagnostics fact now.
  await expect(page.locator("main")).not.toContainText("Content format");

  await page.getByRole("button", { name: "Verify library", exact: true }).click();
  await expect(page.locator("section[aria-labelledby=set-library] [role=status]").filter({ hasText: "Library verified." })).toHaveCount(1, { timeout: 120_000 });
  await expect(page.getByTestId("verify-status")).toHaveAttribute("role", "status");
});

test("The sections keep their order while an update waits", async ({ page, server }) => {
  const next = buildNewerManifest();
  try {
    await installLibrary(page);
    await waitControlled(page);
    await server.overlay(next.dir);
    await page.goto("#/settings");
    await page.getByRole("button", { name: "Check now", exact: true }).click();
    await expect(page.getByTestId("library-update")).toBeVisible();
    await expectSettingsPage(page);
  } finally {
    next.cleanup();
  }
});

// #747: one Version & updates section holds both versions; the old homes of the rows keep none.
test("Version & updates holds the app and library versions, and Library and Help keep none", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await page.goto("#/settings");
  const section = page.locator("#set-updates");
  await expect(section.getByRole("heading", { level: 2, name: "Version & updates", exact: true })).toBeVisible();
  for (const id of ["shell-version", "library-version", "library-size", "library-books"]) {
    await expect(section.getByTestId(id)).toBeVisible();
  }
  await expect(section.getByTestId("library-version")).toHaveText(corpusLabel(distManifest()));
  await expect(section.getByTestId("shell-version")).not.toHaveText("Checking…");
  await expect(section.getByRole("checkbox", { name: "Check for updates automatically" })).toBeVisible();
  await expect(section.getByRole("button", { name: "Check now", exact: true })).toBeVisible();
  for (const id of ["shell-version", "library-version", "library-size"]) {
    await expect(page.locator(`section[aria-labelledby=set-library] [data-testid=${id}], section[aria-labelledby=set-help] [data-testid=${id}]`)).toHaveCount(0);
  }
  await expect(page.locator("section[aria-labelledby=set-library]")).not.toContainText("Rules library");
  await expect(page.locator("section[aria-labelledby=set-help]")).not.toContainText("App version");
});

test("A library update cut off by a reload shows as paused, waits for Resume, and completes from the cache", async ({ page, context, server, guard }) => {
  test.setTimeout(300_000);
  // The cut file is aborted on purpose.
  guard.allow(/Failed to load resource|net::ERR|NetworkError|Load failed/);
  const marker = "PAUSED-LIBRARY-MARKER";
  const next = buildUpdatedLibrary(marker);
  try {
    await installLibrary(page);
    await waitControlled(page);
    await server.overlay(next.dir);
    // One early file never arrives, so the download stops after the others that were in flight.
    const cut = next.manifest.files[3].path;
    await server.hook(`content/${cut}`, "abort");
    await page.goto("#/settings");
    await page.getByRole("button", { name: "Check now", exact: true }).click();
    await page.getByRole("button", { name: "Download update" }).click();
    await expect(page.getByTestId("library-update").locator(".error")).toBeVisible({ timeout: 60_000 });

    // The reader closes the app: the page forgets, the cache and the marker stay.
    const total = next.manifest.files.length;
    const cacheName = `cast-iron-content-${next.version}`;
    const cached = ((await cacheContents(page))[cacheName] ?? []).length;
    expect(cached).toBeGreaterThan(0);
    expect(cached).toBeLessThan(total);
    expect((await storedMeta(page, "pending_install")).content_version).toBe(next.version);
    await server.unhook(`content/${cut}`);
    await server.clearLog();
    await page.reload();
    await waitReady(page);

    // The header and Settings say paused, with the count of files in the cache.
    const action = page
      .locator("header.app-header")
      .getByRole("button", { name: "Library update paused. Open Settings" });
    await expect(action).toBeVisible();
    await expect(action).toContainText("Update paused");
    await action.click();
    await expect(page.locator("#set-updates")).toBeFocused();
    const row = page.getByTestId("library-update");
    await expect(row.getByTestId("update-paused")).toHaveText(`Update paused · ${cached} of ${total} files`);
    await expect(row.getByRole("button", { name: "Resume", exact: true })).toBeEnabled();
    await expect(row.locator(".install-panel")).toHaveCount(0);

    // Rule 1: nothing is fetched until Resume. Offline, Resume gives its reason and stays off.
    await context.setOffline(true);
    await expect(row.getByRole("button", { name: "Resume", exact: true })).toBeDisabled();
    await expect(row).toContainText("Connect to resume.");
    await context.setOffline(false);
    await expect(row.getByRole("button", { name: "Resume", exact: true })).toBeEnabled();
    expect((await server.requests()).filter((p) => p.startsWith("content/"))).toEqual([]);

    // Resume continues from the cache: it fetches only the files that are missing.
    await row.getByRole("button", { name: "Resume", exact: true }).click();
    await expect
      .poll(async () => (await activeLibrary(page))?.content_version, { timeout: 120_000 })
      .toBe(next.version);
    await expect(page.getByTestId("library-version")).toHaveText(corpusLabel(next.manifest), { timeout: 120_000 });
    const fetched = (await server.requests()).filter((p) => p.startsWith(`content/${next.version}/`));
    expect(fetched).toHaveLength(total - cached);
    expect(await storedMeta(page, "pending_install")).toBeNull();
    await expect.poll(async () => contentCaches(await cacheContents(page))).toEqual([cacheName]);
  } finally {
    next.cleanup();
  }
});

test("A paused update with a missing cache counts nothing, and a stale one is dropped", async ({ page, server }) => {
  const other = buildNewerManifest();
  try {
    await installLibrary(page);
    await waitControlled(page);
    const known = "v1111111111111111";
    // The marker names a version whose cache is gone.
    await writeMeta(page, "auto_update_check", false);
    await writeMeta(page, "pending_install", { content_version: known, started_at: Date.now(), files_total: 12 });
    await writeMeta(page, "update_check", { last_checked: Date.now(), known_version: known, known_bytes: 5000 });
    await waitHomeIdle(page);
    await server.clearLog();
    await page.reload();
    await waitReady(page);
    await page.goto("#/settings");
    const row = page.getByTestId("library-update");
    await expect(row.getByTestId("update-paused")).toHaveText("Update paused · 0 of 12 files");

    // Resume reads the manifest. It names another version, so the marker is stale: the paused
    // update goes, the newest is offered, and nothing downloads.
    await server.overlay(other.dir);
    await row.getByRole("button", { name: "Resume", exact: true }).click();
    await expect(row.getByRole("button", { name: "Download update" })).toBeVisible();
    await expect(row.getByTestId("update-paused")).toHaveCount(0);
    await expect(row).toContainText(other.manifest.corpus_tag ?? other.version);
    expect(await storedMeta(page, "pending_install")).toBeNull();
    expect((await server.requests()).filter((p) => p.startsWith(`content/${other.version}/`))).toEqual([]);

    // A marker that the last check already contradicts is dropped at start, with its partial cache.
    const stale = "v2222222222222222";
    await page.evaluate(async (name) => {
      const cache = await caches.open(name);
      await cache.put("content/x/file.json", new Response("{}"));
    }, `cast-iron-content-${stale}`);
    await writeMeta(page, "pending_install", { content_version: stale, started_at: Date.now(), files_total: 12 });
    await writeMeta(page, "update_check", { last_checked: Date.now(), known_version: other.version, known_bytes: 5000 });
    await page.reload();
    await waitReady(page);
    await expect.poll(() => storedMeta(page, "pending_install")).toBeNull();
    await expect.poll(async () => contentCaches(await cacheContents(page)).includes(`cast-iron-content-${stale}`)).toBe(false);
    await page.goto("#/settings");
    // The record names a version only, so the row shows it and asks for a check first.
    await expect(page.getByTestId("library-update")).toContainText(other.version);
    await expect(page.getByTestId("update-paused")).toHaveCount(0);
  } finally {
    other.cleanup();
  }
});
