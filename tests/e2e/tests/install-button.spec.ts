import { test, expect, type BrowserContext, type Page } from "./fixtures";
import { installLibrary, waitControlled, waitReady } from "./helpers";

// The header Install button and the Settings install option. Real browsers fire
// `beforeinstallprompt` on their own terms, so these tests dispatch a fake one that has the same
// shape: `prompt()` and `userChoice`. The app must hold such an event even when it arrives
// before the app has started.

const INSTALL_NAME = "Install Cast Iron as an app";
const UPDATE_NAME = "App update ready. Reload to apply";
const IOS_SAFARI =
  "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1";

/** Dispatches a fake `beforeinstallprompt` on the page. `window.__installPrompts` counts `prompt()` calls. */
function dispatchFakePrompt() {
  const w = window as any;
  w.__installPrompts ??= 0;
  const event: any = new Event("beforeinstallprompt", { cancelable: true });
  event.prompt = async () => {
    w.__installPrompts += 1;
  };
  event.userChoice = Promise.resolve({ outcome: "accepted", platform: "web" });
  window.dispatchEvent(event);
}

/** Script text that fires the fake event as soon as the document is parsed, before the app starts. */
const EARLY_PROMPT = { content: `document.addEventListener("DOMContentLoaded", ${dispatchFakePrompt.toString()});` };
const firePromptEarly = (target: BrowserContext | Page) => target.addInitScript(EARLY_PROMPT);

const headerButton = (page: Page) => page.locator("header.app-header").getByRole("button", { name: INSTALL_NAME });
const launches = (page: Page) => page.evaluate(() => localStorage.getItem("cast-iron.launches"));

test("Header Install button prompts the browser and then goes away", async ({ page, context }) => {
  await firePromptEarly(context);
  await installLibrary(page);

  const button = headerButton(page);
  await expect(button).toBeVisible();
  await expect(button).toContainText("Install");
  expect(await launches(page)).toBe("1");

  await button.click();
  await expect.poll(() => page.evaluate(() => (window as any).__installPrompts)).toBe(1);
  // The browser's prompt works once, so the button must not stay as a dead control.
  await expect(button).toHaveCount(0);
});

test("The header button is reachable with Tab and named for screen readers", async ({ page, context, browserName }) => {
  await firePromptEarly(context);
  await installLibrary(page);
  await expect(headerButton(page)).toBeVisible();

  // WebKit on macOS uses Option-Tab to include buttons in sequential navigation.
  const tabKey = browserName === "webkit" ? "Alt+Tab" : "Tab";
  await page.locator("body").click({ position: { x: 1, y: 1 } });
  let focused = "";
  for (let i = 0; i < 20 && focused !== INSTALL_NAME; i++) {
    await page.keyboard.press(tabKey);
    focused = await page.evaluate(() => document.activeElement?.getAttribute("aria-label") ?? "");
  }
  expect(focused).toBe(INSTALL_NAME);
  // It sits after the primary navigation in the tab order.
  const order = await page.evaluate(() => {
    const items = [...document.querySelectorAll<HTMLElement>("header.app-header a, header.app-header button, header.app-header input")];
    return items.map((el) => el.getAttribute("aria-label") ?? el.className);
  });
  expect(order.indexOf(INSTALL_NAME)).toBeGreaterThan(order.lastIndexOf("nav-link"));
  await page.keyboard.press("Enter");
  await expect.poll(() => page.evaluate(() => (window as any).__installPrompts)).toBe(1);
});

test("An installed app shows no header button and Settings says so", async ({ page, context }) => {
  await installLibrary(page);

  // Both ways of running as an installed app. A fresh page per way, in the same installed library.
  const ways: [string, () => void][] = [
    ["navigator.standalone", () => Object.defineProperty(navigator, "standalone", { value: true })],
    [
      "display-mode",
      () => {
        const real = window.matchMedia.bind(window);
        window.matchMedia = (query: string) =>
          /display-mode:\s*standalone/.test(query)
            ? ({ matches: true, media: query, onchange: null, addEventListener() {}, removeEventListener() {}, addListener() {}, removeListener() {}, dispatchEvent: () => false } as MediaQueryList)
            : real(query);
      },
    ],
  ];
  for (const [name, setup] of ways) {
    const installed = await context.newPage();
    await installed.addInitScript(setup);
    await firePromptEarly(installed);
    await installed.goto("#/settings");
    await waitReady(installed);
    await expect(installed.getByRole("heading", { name: "Settings" })).toBeVisible();
    await expect(installed.getByTestId("app-install"), name).toContainText("Cast Iron is running as an installed app.");
    await expect(installed.getByTestId("app-install").locator(".badge"), name).toHaveText("Installed");
    await expect(headerButton(installed), name).toHaveCount(0);
    await expect(installed.getByRole("button", { name: "Install app" }), name).toHaveCount(0);
    await installed.close();
  }
});

test("The header button lasts five launches, and Settings keeps offering install", async ({ page, context }) => {
  await firePromptEarly(context);
  await installLibrary(page);
  expect(await launches(page)).toBe("1");

  // A reload is the same session, so it does not count again.
  await page.reload();
  await waitReady(page);
  expect(await launches(page)).toBe("1");
  await page.goto("#/settings");
  await page.goto("#/");
  expect(await launches(page)).toBe("1");
  await expect(headerButton(page)).toBeVisible();

  // Launches 2 to 6 are fresh sessions: a new page has an empty sessionStorage.
  for (let launch = 2; launch <= 6; launch++) {
    const next = await context.newPage();
    await next.goto("./");
    await waitReady(next);
    expect(await launches(next), `launch ${launch}`).toBe(String(launch));
    if (launch <= 5) {
      await expect(headerButton(next), `launch ${launch}`).toBeVisible();
      await next.close();
      continue;
    }
    // The sixth launch has no header button, but Settings still offers installing.
    await expect(headerButton(next)).toHaveCount(0);
    await next.goto("#/settings");
    await expect(next.getByText("for full-screen reading and an icon on your device")).toBeVisible();
    await next.getByRole("button", { name: "Install app" }).click();
    await expect.poll(() => next.evaluate(() => (window as any).__installPrompts)).toBe(1);
  }
});

test("A browser without an install option gets only a hint in Settings", async ({ page }) => {
  await installLibrary(page);
  await expect(headerButton(page)).toHaveCount(0);
  await page.goto("#/settings");
  await expect(page.getByTestId("app-install")).toHaveText("Your browser's menu may offer Install or Add to Home Screen.");
  await expect(page.getByRole("button", { name: "Install app" })).toHaveCount(0);
});

test("When an app update is ready, only the Update button shows", async ({ page, context, server }) => {
  await firePromptEarly(context);
  await installLibrary(page);
  await waitControlled(page);
  await expect(headerButton(page)).toBeVisible();

  await page.goto("#/settings");
  const shell = page.getByTestId("shell-version");
  await expect(shell).not.toHaveText(/Checking|Not available/);
  const nextShell = `e2e-both-${(await shell.innerText()).trim()}`;
  await server.shellVersion(nextShell);
  await page.getByRole("button", { name: "Check for updates", exact: true }).click();

  // Both actions apply, but the header has one slot and the update wins.
  const update = page.locator("header.app-header").getByRole("button", { name: UPDATE_NAME });
  await expect(update).toBeVisible();
  await expect(headerButton(page)).toHaveCount(0);
  // Settings keeps offering install, so the update does not hide that option.
  await expect(page.getByTestId("app-install")).toContainText("for full-screen reading and an icon on your device");
});

test("Unavailable storage hides the header button without breaking the app", async ({ page, context }) => {
  await firePromptEarly(context);
  await context.addInitScript(() => {
    const refuse = () => {
      throw new DOMException("denied", "SecurityError");
    };
    Storage.prototype.setItem = refuse;
    Storage.prototype.getItem = refuse;
  });
  await installLibrary(page);
  await expect(headerButton(page)).toHaveCount(0);
});

test.describe("iOS Safari", () => {
  test.use({ userAgent: IOS_SAFARI });

  test("The guidance dialog opens, Esc closes it, and focus returns", async ({ page }) => {
    await installLibrary(page);
    const button = headerButton(page);
    await expect(button).toBeVisible();

    // By keyboard.
    await button.focus();
    await page.keyboard.press("Enter");
    const dialog = page.getByRole("dialog", { name: "Add Cast Iron to your Home Screen" });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole("listitem")).toHaveText([
      "Tap Share in Safari's toolbar.",
      "Choose Add to Home Screen.",
      "Tap Add to finish.",
    ]);
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    await expect(button).toBeFocused();

    // By pointer, closed with Done.
    await button.click();
    await expect(dialog).toBeVisible();
    await dialog.getByRole("button", { name: "Done" }).click();
    await expect(dialog).toBeHidden();
    await expect(button).toBeFocused();
  });

  test("Settings offers the same guidance and returns focus to its button", async ({ page }) => {
    await installLibrary(page);
    await page.goto("#/settings");
    const install = page.getByRole("button", { name: "Install app" });
    await expect(install).toBeVisible();
    await install.focus();
    await page.keyboard.press("Enter");
    const dialog = page.getByRole("dialog", { name: "Add Cast Iron to your Home Screen" });
    await expect(dialog).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    await expect(install).toBeFocused();
  });
});
