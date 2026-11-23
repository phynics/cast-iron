// Capture the conditional views of the built app: states that a reader meets only after
// something goes wrong or something is waiting. Dev-only; never shipped with the PWA (#448).
//
//   node tests/e2e/capture-conditional.mjs <out-dir> [--only=<group>[,<group>...]]
//
// Build the site first (`cargo xtask build-site --public-url /cast-iron/`). The script starts its
// own test server (port E2E_CAPTURE_PORT, default 4398), installs the library once per browser
// engine, and gives every capture a fresh copy of that profile. Phone (390x844) and desktop
// (1280x900), light and dark, one PNG each, plus `manifest.json` with the provenance of every
// capture: commit, shell and content version, corpus tag, engine and version, viewport, theme,
// settled state, and the masthead probe of `capture-lib.mjs`.
//
// Groups:
//   storage-failure   IndexedDB refuses to open, so the app stops before the reader starts.
//   shell-update      A newer app shell waits for a reload.
//   library-update    A newer rules library is offered; nothing is downloaded.
//   combined-updates  Both are waiting, so the header and Settings say "Updates".
//   ios-guide         The Add to Home Screen guide, in WebKit with an iPhone Safari user agent.
//                     That is an engine simulation. It is not iOS Safari and not a device (#423).
//
// Two states sit below the first screen: the iOS Settings install row and the phone
// combined-update reload row. Their `settings-install-row-scrolled` and `settings-reload-row`
// captures scroll the subject to the middle and check its bounds and text (#448).

import { chromium, webkit } from '@playwright/test';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { THEMES, VIEWPORTS, IOS_SAFARI_UA, buildProvenance, copyProfile, dist, here, installLibrary, pageFacts, shoot, waitControlled } from './capture-lib.mjs';
import { startServer } from './server.mjs';

const args = process.argv.slice(2);
const out = path.resolve(args.find(a => !a.startsWith('--')) ?? path.join(os.tmpdir(), 'cast-iron-conditional-captures'));
const only = args.find(a => a.startsWith('--only='))?.slice('--only='.length).split(',');
fs.mkdirSync(out, { recursive: true });

const port = Number(process.env.E2E_CAPTURE_PORT ?? 4398);
const origin = `http://127.0.0.1:${port}`;
const baseURL = `${origin}/cast-iron/`;
const build = buildProvenance();
const playwrightVersion = JSON.parse(fs.readFileSync(path.join(here, 'node_modules/@playwright/test/package.json'), 'utf8')).version;

const FIREBALL = '#/entry/ref/spell/fireball';
const IOS_SIMULATION = 'WebKit engine in Playwright with an iPhone Safari user agent, mobile viewport and touch emulation. '
  + 'Engine simulation only: not iOS Safari, not a device, no Home Screen. Physical-device acceptance is #423.';

/** Test-server controls (see `server.mjs`). */
const control = (route, body = {}) => fetch(`${origin}/__test/${route}`, { method: 'POST', body: JSON.stringify(body) });

/** A copy of the live manifest that names a newer content version, as `buildNewerManifest` in the specs does. */
function newerLibraryOverlay() {
  const old = JSON.parse(fs.readFileSync(path.join(dist, 'content/manifest.json'), 'utf8'));
  const version = 'vcapturenext0000000';
  const prefix = `${old.content_version}/`;
  const manifest = { ...old, content_version: version, files: old.files.map(file => ({ ...file, path: `${version}/${file.path.slice(prefix.length)}` })) };
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'cast-iron-capture-overlay-'));
  fs.mkdirSync(path.join(dir, 'content'), { recursive: true });
  fs.writeFileSync(path.join(dir, 'content/manifest.json'), JSON.stringify(manifest));
  return { dir, version, cleanup: () => fs.rmSync(dir, { recursive: true, force: true }) };
}

const header = page => page.locator('header.app-header');
const UPDATE_BUTTONS = {
  app: 'App update ready. Open Settings',
  library: 'Library update ready. Open Settings',
  both: 'Updates available. Open Settings',
};

/** Ask for an update check from Settings, as a reader does. */
async function checkNow(page) {
  await page.goto('#/settings');
  await page.getByRole('button', { name: 'Check now', exact: true }).first().click();
}

/**
 * Each group names its engine, whether it starts from the installed profile, the steps that make
 * the state (reproducible by hand), and a `run` that reaches the state and captures it.
 */
const GROUPS = [
  {
    id: 'storage-failure',
    engine: chromium,
    engineName: 'chromium',
    installed: false,
    steps: [
      'Open the app in a fresh profile.',
      'Before any app script runs, replace IDBFactory.prototype.open with a function that throws SecurityError ("The operation is insecure."). Firefox private browsing and some locked-down WebViews fail this way.',
      'Load ./ and wait for the failure screen. No library exists; the reader never starts.',
    ],
    async setup(context) {
      await context.addInitScript(() => {
        IDBFactory.prototype.open = function open() { throw new DOMException('The operation is insecure.', 'SecurityError'); };
      });
    },
    async run({ page, capture }) {
      await page.goto('./');
      await page.getByRole('heading', { name: "Cast Iron can't open its storage", exact: true }).waitFor();
      const text = await page.locator('main .error[role=alert]').innerText();
      if (!text.includes("won't let Cast Iron save its library")) throw new Error(`unexpected failure text: ${text}`);
      await capture('cannot-start', {
        description: 'Boot::Fatal screen: the installer masthead, heading "Cast Iron can\'t open its storage", the alert, the next step, Try again and the engine text under Details for a bug report.',
        ready: 'main h1:text("Cast Iron can\'t open its storage")',
        note: 'The brand-only installer masthead, no navigation: the reader has not started.',
      });
    },
  },
  {
    id: 'shell-update',
    engine: chromium,
    engineName: 'chromium',
    installed: true,
    steps: [
      'Install the library from the built site.',
      'POST /__test/shell {version} so the server hands out sw.js with a different SHELL_VERSION.',
      'Settings → Updates → Check now. The new shell installs and waits; nothing reloads.',
      'Open a reading page: the header shows "Update ready". Open Settings: the Updates section shows the reload row.',
    ],
    async run({ page, capture }) {
      await control('shell', { version: `capture-next-${build.shell_version}` });
      await checkNow(page);
      await header(page).getByRole('button', { name: UPDATE_BUTTONS.app }).waitFor();
      await page.goto(FIREBALL);
      await header(page).getByRole('button', { name: UPDATE_BUTTONS.app }).waitFor();
      await capture('reading-header', {
        description: 'Reading page (Fireball) with the header "Update ready" action for an app update.',
        ready: 'header.app-header [role=status]:text("App update ready.")',
      });
      await page.goto('#/settings');
      await page.getByTestId('app-update').waitFor();
      await capture('settings-updates', {
        description: 'Settings → Updates with the app-update row and its Reload button; the library is unchanged.',
        ready: '#set-updates [data-testid="app-update"]',
      });
    },
  },
  {
    id: 'library-update',
    engine: chromium,
    engineName: 'chromium',
    installed: true,
    steps: [
      'Install the library from the built site.',
      'POST /__test/overlay {dir}: the server answers content/manifest.json from a copy that names a newer content version (no content files; it is never downloaded).',
      'Settings → Updates → Check now. The newer library is offered with its size and "Download update"; the app does not download it.',
      'Open a reading page: the header shows "Update ready" for the library.',
    ],
    async run({ page, capture }) {
      const overlay = newerLibraryOverlay();
      try {
        await control('overlay', { dir: overlay.dir });
        await checkNow(page);
        await page.getByRole('button', { name: 'Download update' }).waitFor();
        await capture('settings-updates', {
          description: 'Settings → Updates with a newer rules library offered (version, size, Download update). Nothing downloads.',
          ready: '#set-updates [data-testid="library-update"] .install-panel .btn-primary',
        });
        await page.goto(FIREBALL);
        await header(page).getByRole('button', { name: UPDATE_BUTTONS.library }).waitFor();
        await capture('reading-header', {
          description: 'Reading page (Fireball) with the header "Update ready" action for a library update.',
          ready: 'header.app-header [role=status]:text("Library update ready.")',
        });
      } finally {
        overlay.cleanup();
      }
    },
  },
  {
    id: 'combined-updates',
    engine: chromium,
    engineName: 'chromium',
    installed: true,
    steps: [
      'Install the library from the built site.',
      'Do both of the shell-update and library-update fixtures: a different SHELL_VERSION and an overlay manifest with a newer content version.',
      'Settings → Updates → Check now. The header action and Settings both say that updates are waiting.',
      'On a phone the first screen ends above the reload row. Scroll the reload row to the middle for the scrolled frame; desktop shows both rows already.',
    ],
    async run({ page, capture, viewport }) {
      const overlay = newerLibraryOverlay();
      try {
        await control('shell', { version: `capture-next-${build.shell_version}` });
        await control('overlay', { dir: overlay.dir });
        await checkNow(page);
        await page.getByRole('button', { name: 'Download update' }).waitFor();
        await header(page).getByRole('button', { name: UPDATE_BUTTONS.both }).waitFor();
        await page.getByTestId('app-update').waitFor();
        await capture('settings-updates', {
          description: 'Settings → Updates with the app reload row and the newer library offered together.',
          ready: '#set-updates [data-testid="library-update"] .install-panel .btn-primary',
        });
        // The phone frame above stops above the reload row, so the combined state needs a scrolled
        // frame there. Desktop already shows both rows; it is not captured twice (#448).
        if (viewport === 'phone') await capture('settings-reload-row', {
          description: 'Settings → Updates scrolled so the app reload row ("Update ready" and Reload) and part of the library offer are in view.',
          ready: '[data-testid="app-update"]',
          scrollTo: { selector: '[data-testid="app-update"]', text: ['Update ready', 'Reload'] },
        });
        await page.goto(FIREBALL);
        await header(page).getByRole('button', { name: UPDATE_BUTTONS.both }).waitFor();
        await capture('reading-header', {
          description: 'Reading page (Fireball) with the combined header action ("Updates available").',
          ready: 'header.app-header [role=status]:text("Updates ready.")',
        });
      } finally {
        overlay.cleanup();
      }
    },
  },
  {
    id: 'ios-guide',
    engine: webkit,
    engineName: 'webkit',
    installed: true,
    userAgent: IOS_SAFARI_UA,
    simulation: IOS_SIMULATION,
    steps: [
      'Install the library in WebKit.',
      'Reopen with the iPhone Safari user agent, touch and mobile emulation (the app detects iOS Safari from that user agent).',
      'The header shows Install. Tap it: the Add to Home Screen guide opens as a modal dialog. Settings → Install app offers the same guide.',
      'Settings is taller than the viewport: the frame from the top shows no install row. Scroll the Install section to the middle for the scrolled frame, which needs the row fully in view with its sentence and button.',
    ],
    async run({ page, capture }) {
      await page.goto(FIREBALL);
      const install = header(page).getByRole('button', { name: 'Install Cast Iron as an app' });
      await install.waitFor();
      await capture('header-install-button', {
        description: 'Reading page (Fireball) in iOS-Safari emulation with the header Install button, before the guide opens.',
        ready: 'header.app-header button[aria-label="Install Cast Iron as an app"]',
      });
      await install.click();
      const dialog = page.getByRole('dialog', { name: 'Add Cast Iron to your Home Screen' });
      await dialog.waitFor();
      await capture('guide-dialog', {
        description: 'The Add to Home Screen guide (Share, Add to Home Screen, Add) over the reading page.',
        ready: 'dialog.install-sheet[open] .install-steps',
        mastheadExpected: false,
        note: 'A modal dialog: the backdrop dims the masthead by design, so the frame probe is not expected to find it.',
      });
      await dialog.getByRole('button', { name: 'Done' }).click();
      await page.goto('#/settings');
      await page.getByRole('button', { name: 'Install app' }).waitFor();
      await capture('settings-install-row', {
        description: 'Settings with the Install app row that opens the same guide.',
        ready: 'main button:text("Install app")',
      });
      // The Install section is below the first screen, so the frame above does not show the row it
      // names. This frame scrolls the row into the middle and checks its bounds and text (#448).
      await capture('settings-install-row-scrolled', {
        description: 'Settings scrolled to the Install section: the Install app row with its sentence and button in view.',
        ready: '[data-testid="app-install"]',
        scrollTo: { selector: '[data-testid="app-install"]', text: ['Install Cast Iron as an app', 'Install app'] },
      });
    },
  },
];

/** A profile that holds the installed library, one per engine, reused for every capture. */
async function installedProfile(engine) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'cast-iron-capture-installed-'));
  const context = await engine.launchPersistentContext(dir, { baseURL, viewport: VIEWPORTS.desktop, serviceWorkers: 'allow' });
  try {
    const page = context.pages()[0] ?? await context.newPage();
    await installLibrary(page);
    await waitControlled(page);
  } finally {
    await context.close();
  }
  return dir;
}

const captures = [];
const failures = [];
const { server } = await startServer({ dist, port });
const profiles = new Map();
/** A persistent context has no browser object, so ask a throwaway launch for each engine's version. */
const versions = new Map();
for (const [name, engine] of [['chromium', chromium], ['webkit', webkit]]) {
  if (only && !GROUPS.some(group => group.engineName === name && only.includes(group.id))) continue;
  const browser = await engine.launch();
  versions.set(name, browser.version());
  await browser.close();
}
try {
  for (const group of GROUPS) {
    if (only && !only.includes(group.id)) continue;
    if (group.installed && !profiles.has(group.engineName)) {
      console.log(`installing the library in ${group.engineName}…`);
      await control('reset'); // an earlier group's shell or overlay fixture must not reach the install
      profiles.set(group.engineName, await installedProfile(group.engine));
    }
    for (const [viewportName, size] of Object.entries(VIEWPORTS)) for (const theme of THEMES) {
      const tag = `${group.id}-%STATE%-${viewportName}-${theme}`;
      await control('reset');
      const copy = group.installed ? copyProfile(profiles.get(group.engineName)) : null;
      const scratch = copy ?? { dir: fs.mkdtempSync(path.join(os.tmpdir(), 'cast-iron-capture-fresh-')), cleanup() {} };
      const phoneLike = viewportName === 'phone';
      const context = await group.engine.launchPersistentContext(scratch.dir, {
        baseURL,
        viewport: size,
        colorScheme: theme,
        serviceWorkers: 'allow',
        ...(group.userAgent ? { userAgent: group.userAgent, isMobile: phoneLike, hasTouch: phoneLike } : {}),
      });
      const browser = { version: () => versions.get(group.engineName) };
      try {
        await group.setup?.(context);
        const page = context.pages()[0] ?? await context.newPage();
        const probePage = await context.newPage();
        await probePage.goto('about:blank');
        const errors = [];
        page.on('pageerror', error => errors.push(error.message));
        page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
        const capture = async (state, { description, ready, mastheadExpected = true, note, scrollTo }) => {
          const file = path.join(out, `${tag.replace('%STATE%', state)}.png`);
          // A click on Check now can leave the page scrolled; every state is shot from the top,
          // unless it names a subject that does not fit there (`scrollTo`).
          const record = await shoot(page, probePage, { file, readySelector: ready, resetScroll: true, scrollTo });
          if (!mastheadExpected) { delete record.warning; record.masthead_expected = false; }
          if (record.warning) console.warn(`${path.basename(file)}: ${record.warning}`);
          captures.push({
            group: group.id,
            state,
            description,
            ...(note ? { note } : {}),
            viewport: { name: viewportName, ...size },
            theme,
            evidence_class: group.simulation ? 'engine-simulation' : 'headless-chromium',
            ...(group.simulation ? { simulation: group.simulation } : {}),
            ...record,
            page: await pageFacts(page, browser, { engine: group.engineName }),
            console_errors_so_far: [...errors],
          });
        };
        await group.run({ page, capture, viewport: viewportName });
      } catch (error) {
        failures.push(`${tag.replace('%STATE%', '*')}: ${error.message.split('\n')[0]}`);
        console.error(`${tag.replace('%STATE%', '*')}: ${error.message}`);
      } finally {
        await context.close().catch(() => {});
        await control('reset');
        copy?.cleanup();
        if (!copy) fs.rmSync(scratch.dir, { recursive: true, force: true });
      }
    }
  }
} finally {
  await control('reset').catch(() => {});
  server.closeAllConnections();
  server.close();
  for (const dir of profiles.values()) fs.rmSync(dir, { recursive: true, force: true });
}

const manifest = {
  script: 'tests/e2e/capture-conditional.mjs',
  issue: 'https://github.com/phynics/cast-iron/issues/448',
  build,
  // Playwright injects an inline <style> into WebKit pages of persistent contexts, which the page's
  // Content Security Policy refuses. It is a test-harness artifact (see playwright.config.ts), not an app error.
  known_console_artifacts: ['WebKit: "Refused to apply a stylesheet because its hash, its nonce, or \'unsafe-inline\' does not appear in the style-src directive"'],
  fixtures: {
    reading_page: FIREBALL,
    library_update: 'synthetic: a copy of the live manifest with content_version "vcapturenext0000000"; its files are never fetched',
    shell_update: 'the test server serves sw.js with SHELL_VERSION "capture-next-<shell version>"',
  },
  tools: { playwright: playwrightVersion, node: process.version, platform: `${process.platform}-${process.arch}` },
  groups: GROUPS.filter(group => !only || only.includes(group.id)).map(group => ({
    id: group.id, engine: group.engineName, steps: group.steps, ...(group.simulation ? { simulation: group.simulation } : {}),
  })),
  captures,
  failures,
};
fs.writeFileSync(path.join(out, 'manifest.json'), JSON.stringify(manifest, null, 2));
const withoutMasthead = captures.filter(c => c.warning);
console.log(JSON.stringify({ out, captures: captures.length, failures, mastheadMissing: withoutMasthead.map(c => c.file) }, null, 2));
if (failures.length || withoutMasthead.length) process.exitCode = 1;
