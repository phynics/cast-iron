//! Small helpers shared by views.

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::future::Future;

/// Runs a future once and exposes its result: `None` while loading. Works for results that are
/// not `Send`, such as `Rc` values, because the signal is local to the browser thread.
pub fn local_resource<T: 'static>(
    future: impl Future<Output = T> + 'static,
) -> ReadSignal<Option<T>, LocalStorage> {
    let (read, write) = signal_local(None::<T>);
    spawn_local(async move { write.set(Some(future.await)) });
    read
}
