---
title: "A panic shows a Reload notice, and the stuck-router report stays open"
date: 2026-11-17
status: current
superseded_by: []
issues: [468]
prs: []
area: ui
---

# A panic shows a Reload notice, and the stuck-router report stays open, 2026-11-17

## Decision

`panic_notice::install` replaces `console_error_panic_hook::set_once`. The hook still writes the
panic to `console.error`, and adds a notice with a Reload button, built from typed DOM nodes
outside the reactive tree. The notice is a modal `<dialog>`: Quick search is a modal dialog too, anything outside the topmost modal is inert, and `z-index` cannot lift a node above the top layer, so a plain banner could not be clicked while Quick search was open. A modal opened later is the topmost one. The hook does not close the other dialog, because its close handler is reactive code.

## Why

Issue #468: once, after a burst of searches, the page stayed on Search while `location.hash` moved
on, and nothing answered a tap. A reload fixed it.

A panic in a reactive effect or a spawned task aborts that call and leaves the executor of
`js-sys` (`futures/task/singlethread.rs`) with a `RefCell` that stays borrowed. Every later task
then panics with `RefCell already borrowed`. The route signal still changes, but no effect runs,
so the page freezes in the state it had. A build with a deliberate panic in a screen showed exactly
that, and the second panic in the console. WebAssembly here aborts on panic and cannot catch it, so
the page cannot recover by itself. Reload is the recovery, and the notice says so instead of
leaving a reader with a page that ignores taps.

The panic that began it was not found. Scripted bursts of hash changes did not reproduce it:
0 ms to 2.5 s apart, a random walk of hashes, typing, Back and Forward and navigation clicks,
CPU throttled 6 to 20 times, 390 px with the Diagnostics probe on, and class section jumps.
`router-burst.spec.ts` keeps the burst covered. `ensure_index` read a signal of the Search screen
without checking that the screen was still there, which is the kind of read that panics, and now
uses the checked read.

## How to apply

If #468 shows again, the console holds the panic text and its source line (the notice only tells the reader to reload).
Do not remove the hook to silence a test: a test that sees `#panic-notice` found a panic.
