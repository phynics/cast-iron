import { test, expect } from "./fixtures";
import { installLibrary, waitControlled } from "./helpers";

test("Application is installable where browser support allows", async ({ page, baseURL }) => {
  await installLibrary(page);
  await waitControlled(page);
  const scope = new URL(baseURL!).pathname; // /cast-iron/

  const href = await page.locator("link[rel=manifest]").getAttribute("href");
  expect(href).toBeTruthy();
  const manifestUrl = new URL(href!, page.url()).href;
  const response = await page.request.get(manifestUrl);
  expect(response.ok()).toBe(true);
  expect(response.headers()["content-type"]).toMatch(/manifest\+json|json/);
  const manifest = await response.json();
  expect(manifest.display).toBe("standalone");
  expect(manifest.name).toBeTruthy();

  // start_url, scope and id resolve inside /cast-iron/.
  for (const key of ["start_url", "scope", "id"] as const) {
    const resolved = new URL(manifest[key], manifestUrl);
    expect(resolved.pathname.startsWith(scope), `${key} -> ${resolved.pathname}`).toBe(true);
  }
  expect(new URL(manifest.scope, manifestUrl).pathname).toBe(scope);

  // 192 and 512 px icons exist and load as images of their declared size.
  for (const size of [192, 512]) {
    const icon = manifest.icons.find((i: any) => i.sizes === `${size}x${size}` && i.type === "image/png" && i.purpose === "any");
    expect(icon, `${size}px icon`).toBeTruthy();
    const url = new URL(icon.src, manifestUrl).href;
    const loaded = await page.evaluate(
      (src) =>
        new Promise<{ w: number; h: number }>((resolve, reject) => {
          const img = new Image();
          img.onload = () => resolve({ w: img.naturalWidth, h: img.naturalHeight });
          img.onerror = () => reject(new Error(`cannot load ${src}`));
          img.src = src;
        }),
      url,
    );
    expect(loaded).toEqual({ w: size, h: size });
  }
  expect(manifest.icons.some((i: any) => i.purpose === "maskable")).toBe(true);

  // The service worker controls the page, with a scope that matches the manifest.
  const worker = await page.evaluate(async () => {
    const registration = await navigator.serviceWorker.ready;
    return { scope: new URL(registration.scope).pathname, controlled: !!navigator.serviceWorker.controller, state: registration.active?.state };
  });
  expect(worker).toEqual({ scope, controlled: true, state: "activated" });
  expect(await page.evaluate(() => window.isSecureContext)).toBe(true);
});
