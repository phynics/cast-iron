//! What a reader sees when the app panics (#468).
//!
//! A panic in WebAssembly aborts the call that ran it. When that call is a reactive effect (the
//! one that draws the page for the route), the page stops following the address bar for good:
//! the hash changes, the page stays, and nothing on screen says why. The console message of
//! `console_error_panic_hook` is the only trace, and a reader never opens the console.
//!
//! The hook below keeps that console message and adds a notice with a Reload button. The notice
//! is plain DOM, built from typed nodes (no `inner_html`) and outside the reactive tree, so it
//! still appears when that tree no longer updates.
//!
//! The notice is a modal `<dialog>`. Quick search is one too, and it can be open when the panic
//! comes: a node outside the topmost modal is inert, and no `z-index` lifts a node above the top
//! layer. A modal that opens later is the topmost one, so the Reload button takes focus and clicks
//! whatever is open. Nothing here closes the other dialog, because its `close` handler is
//! reactive code that may be the thing that is stuck.

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// The `id` of the notice. Tests and styles find it by this.
pub const NOTICE_ID: &str = "panic-notice";

/// Replaces the default panic hook. Call once, before anything else runs.
pub fn install() {
    std::panic::set_hook(Box::new(|info| {
        console_error_panic_hook::hook(info);
        show_notice();
    }));
}

/// Adds the notice to the page, once. Every step can fail quietly: a panic inside the panic hook
/// would hide the first one.
fn show_notice() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    if document.get_element_by_id(NOTICE_ID).is_some() {
        return;
    }
    let (Some(body), Ok(created), Ok(text), Ok(reload)) = (
        document.body(),
        document.create_element("dialog"),
        document.create_element("p"),
        document.create_element("button"),
    ) else {
        return;
    };
    let Ok(notice) = created.dyn_into::<web_sys::HtmlDialogElement>() else {
        return;
    };
    let _ = notice.set_attribute("id", NOTICE_ID);
    let _ = notice.set_attribute("role", "alertdialog");
    let _ = notice.set_attribute("aria-label", "Cast Iron hit an error");
    let _ = notice.set_attribute("class", "panic-notice");
    text.set_text_content(Some(
        "Cast Iron hit an error and may have stopped following your taps. \
         Your library and bookmarks are safe. Reload to carry on.",
    ));
    let _ = reload.set_attribute("type", "button");
    let _ = reload.set_attribute("class", "btn btn-primary");
    let _ = reload.set_attribute("autofocus", "");
    reload.set_text_content(Some("Reload"));
    let on_click = Closure::<dyn FnMut()>::new(|| {
        if let Some(window) = web_sys::window() {
            let _ = window.location().reload();
        }
    });
    let _ = reload.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref());
    // The page owns the listener until it goes away.
    on_click.forget();
    // Escape asks to close the dialog; the notice stays until the page reloads.
    let keep = Closure::<dyn FnMut(web_sys::Event)>::new(|event: web_sys::Event| {
        event.prevent_default();
    });
    let _ = notice.add_event_listener_with_callback("cancel", keep.as_ref().unchecked_ref());
    keep.forget();
    let _ = notice.append_child(&text);
    let _ = notice.append_child(&reload);
    let _ = body.append_child(&notice);
    let _ = notice.show_modal();
    if let Ok(button) = reload.dyn_into::<web_sys::HtmlElement>() {
        let _ = button.focus();
    }
}
