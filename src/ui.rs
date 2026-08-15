//! Small shared pieces used across the route screens (spec §4.3–4.7).

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

/// A full-screen failure notice: a setup or store error surfaced rather than
/// hidden (AGENTS.md — errors are logged at the boundary and shown, never
/// swallowed). The caller passes the whole message; the danger styling lives here.
#[component]
pub fn Failure(message: String) -> Element {
    rsx! {
        div { class: "p-6 font-sans text-base text-danger", "{message}" }
    }
}
