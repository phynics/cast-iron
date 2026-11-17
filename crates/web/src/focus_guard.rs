//! Guards for a focus move that a route change schedules for the next animation frame (#419).
//!
//! A frame has no fixed deadline. Between the navigation and the frame, a newer navigation can
//! start, and the reader can focus a control (a fast keyboard user, switch control, a slow
//! device). The frame must not take focus from either. [`FocusTicket::take`] records the
//! navigation generation and the element that had focus when the frame was scheduled;
//! [`FocusTicket::verdict`] answers when the frame fires. The decision is pure ([`decide`]) and
//! has native tests; only the two reads of the document live here.

use std::cell::Cell;

thread_local! {
    static GENERATION: Cell<u64> = const { Cell::new(0) };
}

/// Starts a navigation: every ticket taken before this call is stale from now on.
pub fn begin_navigation() {
    GENERATION.with(|g| g.set(g.get().wrapping_add(1)));
}

fn generation() -> u64 {
    GENERATION.with(Cell::get)
}

/// What a scheduled frame may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// A newer navigation started. The frame does nothing, scroll included: the newer
    /// navigation's own frame scrolls.
    Stale,
    /// The reader focused another control. The frame may scroll but must leave focus alone.
    ScrollOnly,
    /// The frame may scroll and move focus.
    Proceed,
}

/// Decides what a frame may do. `at_schedule` and `now` are the focused element when the frame
/// was scheduled and when it fires; `None` means that nothing is focused (the document body).
/// Focus yields when it sits on an element other than the one it sat on at scheduling: the
/// reader moved it, or the new page took it. The element that had focus at scheduling (a link
/// that the reader just clicked, and which is still there) does not count as a move.
pub fn decide<T: PartialEq>(
    scheduled: u64,
    current: u64,
    at_schedule: Option<&T>,
    now: Option<&T>,
) -> Verdict {
    if scheduled != current {
        Verdict::Stale
    } else if now.is_some() && now != at_schedule {
        Verdict::ScrollOnly
    } else {
        Verdict::Proceed
    }
}

/// The state of a navigation when a frame was scheduled.
pub struct FocusTicket {
    generation: u64,
    focused: Option<web_sys::Element>,
}

impl FocusTicket {
    pub fn take() -> Self {
        Self {
            generation: generation(),
            focused: focused_element(),
        }
    }

    pub fn verdict(&self) -> Verdict {
        decide(
            self.generation,
            generation(),
            self.focused.as_ref(),
            focused_element().as_ref(),
        )
    }
}

/// The element that has focus, or `None` when nothing has (the body or the document element).
fn focused_element() -> Option<web_sys::Element> {
    let document = web_sys::window()?.document()?;
    let active = document.active_element()?;
    let tag = active.tag_name();
    (!tag.eq_ignore_ascii_case("body") && !tag.eq_ignore_ascii_case("html")).then_some(active)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_proceeds_when_nothing_moved_focus() {
        assert_eq!(decide::<&str>(3, 3, None, None), Verdict::Proceed);
        // The control that was focused at scheduling is still focused: the reader clicked it.
        assert_eq!(decide(3, 3, Some(&"nav"), Some(&"nav")), Verdict::Proceed);
        // The focused control left the page and focus fell to the body.
        assert_eq!(decide(3, 3, Some(&"link"), None), Verdict::Proceed);
    }

    #[test]
    fn a_control_focused_since_scheduling_keeps_focus() {
        assert_eq!(decide(3, 3, None, Some(&"filter")), Verdict::ScrollOnly);
        assert_eq!(
            decide(3, 3, Some(&"link"), Some(&"filter")),
            Verdict::ScrollOnly
        );
    }

    #[test]
    fn a_newer_navigation_cancels_the_frame() {
        assert_eq!(decide::<&str>(3, 4, None, None), Verdict::Stale);
        // Stale wins over a focus move: the newer navigation owns scroll and focus.
        assert_eq!(decide(3, 4, None, Some(&"filter")), Verdict::Stale);
    }

    #[test]
    fn begin_navigation_makes_earlier_generations_stale() {
        let before = generation();
        begin_navigation();
        assert_ne!(before, generation());
        assert_eq!(
            decide::<&str>(before, generation(), None, None),
            Verdict::Stale
        );
    }
}
