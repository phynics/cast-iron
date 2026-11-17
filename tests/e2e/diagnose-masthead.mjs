// Reproduce the "missing masthead" capture discrepancy of #448 and record the evidence. Dev-only.
//
//   node tests/e2e/diagnose-masthead.mjs <out-dir>
//
// The audit saw Chromium frames without the top masthead although the DOM measurement before the
// shot said the header sat at y=0 with height 61. This script takes the same page in every capture
// mode Playwright offers (viewport, full page, element), at the top and scrolled down, and for each
// shot records the scroll position, focus, header bounds and computed style, what the browser hits
// at the header, and what the saved frame shows (read back from the PNG, see `probeFrame`).
//
// Finding (see docs/research/conditional-view-captures.md): the header is `position: sticky; top: 0`.
// A viewport shot always carries it. A full-page shot taken while the page is scrolled places the
// sticky header at the scroll offset inside the tall frame, so the top of the image has no masthead
// while `getBoundingClientRect()` still says y=0 (viewport coordinates). An element shot of <main>
// never contains the header. Neither is a product defect.

import { chromium } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { VIEWPORTS, buildProvenance, copyProfile, dist, installLibrary, probeFrame, probeMasthead, settle, waitControlled } from './capture-lib.mjs';
import { startServer } from './server.mjs';
import os from 'node:os';

const out = path.resolve(process.argv[2] ?? path.join(os.tmpdir(), 'cast-iron-masthead-diagnosis'));
fs.mkdirSync(out, { recursive: true });
const port = Number(process.env.E2E_CAPTURE_PORT ?? 4398);
const baseURL = `http://127.0.0.1:${port}/cast-iron/`;
const PAGES = [
  { name: 'search', hash: '#/search?q=grapple', ready: '.results .hit-link:visible' },
  { name: 'class', hash: '#/entry/core-rulebook/classes/fighter', ready: '.entry > .rich' },
];
const SCROLLS = [0, 1500];
const CROP = 300;
const ANALYSE = 4000;

const { server } = await startServer({ dist, port });
const installedDir = fs.mkdtempSync(path.join(os.tmpdir(), 'cast-iron-masthead-installed-'));
const rows = [];
try {
  const install = await chromium.launchPersistentContext(installedDir, { baseURL, viewport: VIEWPORTS.desktop, serviceWorkers: 'allow' });
  const installPage = install.pages()[0] ?? await install.newPage();
  await installLibrary(installPage);
  await waitControlled(installPage);
  await install.close();

  for (const [viewportName, size] of Object.entries(VIEWPORTS)) {
    const copy = copyProfile(installedDir);
    const context = await chromium.launchPersistentContext(copy.dir, { baseURL, viewport: size, serviceWorkers: 'allow', isMobile: viewportName === 'phone', hasTouch: viewportName === 'phone' });
    try {
      const page = context.pages()[0] ?? await context.newPage();
      const probePage = await context.newPage();
      await probePage.goto('about:blank');
      for (const target of PAGES) {
        await page.goto(target.hash);
        await settle(page, target.ready);
        for (const scrollTo of SCROLLS) {
          await page.evaluate(y => window.scrollTo(0, y), scrollTo);
          await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
          const masthead = await probeMasthead(page);
          const sticky = await page.evaluate(() => {
            const header = document.querySelector('header.app-header');
            const parent = header?.parentElement;
            return parent ? { parent: `${parent.tagName.toLowerCase()}${parent.className ? '.' + String(parent.className).trim().split(/\s+/).join('.') : ''}`, parent_height: parent.getBoundingClientRect().height } : null;
          });
          // Three capture modes. A full-page frame is read back over its first ANALYSE rows with a
          // scan: if the masthead is not where the DOM puts it, where is it? The saved image is the
          // top CROP rows of the same frame, which is what a reviewer sees at the top of the shot.
          const shots = [
            { mode: 'viewport', analyse: () => page.screenshot(), save: png => png, scan: false },
            {
              mode: 'full-page',
              analyse: () => page.screenshot({ fullPage: true, clip: { x: 0, y: 0, width: size.width, height: ANALYSE } }),
              save: () => page.screenshot({ fullPage: true, clip: { x: 0, y: 0, width: size.width, height: CROP } }),
              scan: true,
            },
            { mode: 'element <main>', analyse: () => page.locator('main').screenshot(), save: null, scan: false },
          ];
          for (const shot of shots) {
            const png = await shot.analyse();
            const frame = await probeFrame(probePage, png, masthead, { offsetY: 0, scan: shot.scan });
            const file = `${viewportName}-${target.name}-scroll${Math.round(masthead.scroll_y)}-${shot.mode.replace(/[^a-z0-9]+/gi, '-').replace(/-$/, '')}.png`;
            // Keep the evidence small: an element frame of a long page is read back but not saved.
            const saved = shot.save ? await shot.save(png) : null;
            if (saved) fs.writeFileSync(path.join(out, file), saved);
            rows.push({
              viewport: { name: viewportName, ...size },
              page: target.name,
              requested_scroll_y: scrollTo,
              scroll_y: masthead.scroll_y,
              document_height: masthead.document_height,
              focus: masthead.focus,
              header_bounds: masthead.header,
              header_style: masthead.header_style,
              header_sticky_parent: sticky,
              hit_at_header_centre: masthead.hit_at_header_centre,
              capture_mode: shot.mode,
              file: saved ? file : null,
              frame_note: shot.mode === 'full-page' ? `analysed over the first ${ANALYSE} rows; the saved image is the first ${CROP}` : undefined,
              frame,
              masthead_in_saved_frame: frame.masthead_in_frame,
            });
          }
        }
      }
    } finally {
      await context.close();
      copy.cleanup();
    }
  }
} finally {
  server.closeAllConnections();
  server.close();
  fs.rmSync(installedDir, { recursive: true, force: true });
}

const summary = new Map();
for (const row of rows) {
  const key = `${row.capture_mode} @ scroll ${row.scroll_y > 0 ? '>0' : '0'}`;
  const entry = summary.get(key) ?? { with: 0, without: 0, dom_says_y0: 0 };
  row.masthead_in_saved_frame ? entry.with++ : entry.without++;
  if (row.header_bounds?.y === 0) entry.dom_says_y0++;
  summary.set(key, entry);
}
const report = { script: 'tests/e2e/diagnose-masthead.mjs', build: buildProvenance(), summary: Object.fromEntries(summary), rows };
fs.writeFileSync(path.join(out, 'masthead-diagnosis.json'), JSON.stringify(report, null, 2));
console.log(JSON.stringify(report.summary, null, 2));
