/**
 * The interval in which WebKit's "Context is stopped" page error is tolerated.
 *
 * WebKit raises that error while it tears down a document that a service worker controls
 * (docs/decisions/2026-11-17-webkit-context-stopped-at-teardown.md). In every recorded timeline it
 * arrives after the main frame's navigation request starts and before the new document commits
 * (`framenavigated`). So the window opens with a main-frame navigation request and closes at the
 * commit or at the failure of the request that is being tracked. The message does not say which
 * document it came from, so nothing outside the window is tolerated: not the replacement
 * document's own start-up, and not a page that survived a failed navigation.
 *
 * The window is a time interval for the whole page, not proof of where an error came from.
 * Playwright's `pageerror` does not name the frame either, so the same message from a child frame
 * during a main-frame navigation is also tolerated. The app has no iframes today.
 */
export class TeardownWindow {
  private tracked: object | null = null;
  private url = "";

  /** A main-frame navigation request started. A newer request, or a redirect target, replaces the tracked one. */
  begin(request: object, url: string) {
    this.tracked = request;
    this.url = url;
  }

  /** The main frame committed a document: the old document is gone, so the window closes. */
  commit() {
    this.tracked = null;
  }

  /** A request failed or was cancelled. Only the tracked request closes the window: an older request that a newer one superseded must not. */
  fail(request: object) {
    if (this.tracked === request) this.tracked = null;
  }

  /** The URL of the navigation in progress, or `null` outside the window. */
  get navigating(): string | null {
    return this.tracked === null ? null : this.url;
  }
}
