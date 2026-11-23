//! A view that is replaced, not patched, when it is rendered again.
//!
//! `view.into_any()` makes the compiler emit, for each view type, the code that patches an
//! existing DOM tree of that type in place (`Render::rebuild`). That code runs only when a
//! reactive closure produces the same view type twice in a row. For a view that is built once,
//! or whose page is new each time, it is dead weight: about 127 KB over 243 view types.
//!
//! [`IntoFresh::into_fresh`] erases a view like `into_any()`, but renders it again by building
//! the new tree, putting it where the old one is, and removing the old one. The patch code of
//! that view type is then never compiled.
//!
//! Use it only where one of these holds, and say which in the decision file
//! (`docs/decisions/2026-11-23-page-views-are-replaced-not-patched.md`):
//!
//! 1. The view is never rendered twice with the same type: it sits in a closure that reads a
//!    value that settles once (a `local_resource`, a "built" flag), or it is built once by a
//!    component whose own view meets this rule.
//! 2. The view is a whole page at the route level, where the frame moves focus and scroll for
//!    the new page anyway.
//!
//! Keep `into_any()` for a view that updates while the reader works in it: a list that follows a
//! filter or a query, a panel with an input, a disclosure the reader opened. Replacing such a
//! view drops focus, the caret, the open state and inner scroll.

use leptos::tachys::html::attribute::Attribute;
use leptos::tachys::html::attribute::any_attribute::AnyAttribute;
use leptos::tachys::hydration::Cursor;
use leptos::tachys::view::add_attr::AddAnyAttr;
use leptos::tachys::view::any_view::{AnyView, IntoAny};
use leptos::tachys::view::{Mountable, Position, PositionState, Render, RenderHtml};

/// See the module documentation. Made by [`IntoFresh::into_fresh`].
pub struct Fresh<T>(T);

impl<T: Render> Render for Fresh<T> {
    type State = T::State;

    fn build(self) -> Self::State {
        self.0.build()
    }

    /// The same steps that `AnyView` takes when the view type changed: build, insert before
    /// the old nodes, remove the old nodes.
    fn rebuild(self, state: &mut Self::State) {
        let mut new = self.0.build();
        state.insert_before_this(&mut new);
        state.unmount();
        *state = new;
    }
}

impl<T: AddAnyAttr> AddAnyAttr for Fresh<T> {
    type Output<SomeNewAttr: Attribute> = Fresh<T::Output<SomeNewAttr>>;

    fn add_any_attr<NewAttr: Attribute>(self, attr: NewAttr) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        Fresh(self.0.add_any_attr(attr))
    }
}

// The server-rendering and hydration methods only forward: the app renders in the browser.
impl<T: RenderHtml> RenderHtml for Fresh<T> {
    type AsyncOutput = Fresh<T::AsyncOutput>;
    type Owned = Fresh<T::Owned>;

    const MIN_LENGTH: usize = T::MIN_LENGTH;
    const EXISTS: bool = T::EXISTS;

    fn dry_resolve(&mut self) {
        self.0.dry_resolve();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        Fresh(self.0.resolve().await)
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Vec<AnyAttribute>,
    ) {
        self.0
            .to_html_with_buf(buf, position, escape, mark_branches, extra_attrs);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        self.0.hydrate::<FROM_SERVER>(cursor, position)
    }

    fn into_owned(self) -> Self::Owned {
        Fresh(self.0.into_owned())
    }
}

/// Erases a view so that rendering it again replaces it. See the module documentation for where
/// this is allowed.
pub trait IntoFresh {
    fn into_fresh(self) -> AnyView;
}

impl<T: RenderHtml + Send> IntoFresh for T {
    fn into_fresh(self) -> AnyView {
        Fresh(self).into_any()
    }
}
