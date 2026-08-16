//! Small shared pieces used across the route screens.

use dioxus::prelude::*;

use crate::deck::SharedDeck;
use crate::store::Store;

/// Read the Deck and Store the app provides to every screen (see [`crate::App`]'s
/// `AppRouter`). The two are always provided together, so screens take them as a
/// pair rather than reaching for each context separately.
#[must_use]
pub fn use_app_context() -> (SharedDeck, Store) {
    (use_context::<SharedDeck>(), use_context::<Store>())
}

/// Log an [`anyhow::Error`]'s full chain at the app boundary and report it, then
/// flatten it to a display string. Modules stay logging-free; screens funnel
/// through here on the way to a [`Failure`]. Reporting lives in
/// [`crate::observability::report`] so the log-and-report action is one thing.
pub fn log_and_display(e: &anyhow::Error) -> String {
    crate::observability::report(e);
    format!("{e:#}")
}

/// A full-screen failure notice: a setup or store error surfaced rather than
/// hidden — errors are logged at the boundary and shown, never swallowed. The
/// caller passes the whole message; the danger styling lives here.
#[component]
pub fn Failure(message: String) -> Element {
    rsx! {
        div { class: "p-6 font-sans text-base text-danger", "{message}" }
    }
}
