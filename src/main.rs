use dioxus::prelude::*;

mod deck;
mod map;
mod review;
mod scheduler;
mod session;
mod store;

use deck::Deck;
use map::SharedDeck;
use review::{QueuePosition, Review};

const FAVICON: Asset = asset!("/assets/favicon.ico");
// Tailwind output, compiled from `tailwind.css` by `dx serve` (Dioxus 0.7
// automatic Tailwind). All styling is utility classes; there is no bespoke CSS.
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    // Route tracing output to the platform log. On iOS the fmt subscriber writes
    // to stdout, which `dx serve --ios` and Xcode capture. `dioxus::launch` also
    // auto-inits this, but doing it here first covers any pre-launch logging and
    // makes the setup explicit. See AGENTS.md "Error handling" for how errors are
    // given context and logged at the app boundary.
    dioxus::logger::initialize_default();
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Load the Deck once and share it by cheap Rc clone. A load failure is
    // terminal for the map, so log the full chain and surface it rather than
    // rendering an empty world.
    let deck = use_hook(|| match Deck::load() {
        Ok(deck) => Ok(SharedDeck::new(deck)),
        Err(e) => {
            error!("{e:#}");
            Err(format!("{e:#}"))
        }
    });

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Stylesheet { href: TAILWIND_CSS }
        match deck {
            Ok(deck) => rsx! { ReviewDemo { deck } },
            Err(err) => rsx! {
                div { class: "p-6 font-sans text-base text-danger", "Failed to load map: {err}" }
            },
        }
    }
}

/// Throwaway scaffold for issue 07: renders the Review screen full-bleed on a
/// single hardcoded Card so front⇄reveal is demoable. Real navigation and the
/// session queue arrive in issues 08–09; this exists only to demo the presentation.
#[component]
fn ReviewDemo(deck: SharedDeck) -> Element {
    // France is a good demo Card: a mainland frame with visible neighbours.
    let card = deck
        .get("FRA")
        .or_else(|| deck.cards().first())
        .expect("the shipped Deck is never empty")
        .clone();
    rsx! {
        div { class: "fixed inset-0",
            Review { deck, card, position: QueuePosition { index: 0, total: 11 } }
        }
    }
}
