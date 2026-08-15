use dioxus::prelude::*;

mod deck;
mod map;
mod review;
mod scheduler;
mod session;
mod store;

use deck::{Deck, SharedDeck};
use review::ReviewSession;
use store::Store;

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

/// Resolve the two long-lived handles the Review loop needs: the Deck (parsed from
/// the embedded geometry asset) and the store (the iOS Application Support file).
/// One-time, fallible setup — the app boundary logs the chain and surfaces it.
fn app_setup() -> anyhow::Result<(SharedDeck, Store)> {
    let deck = SharedDeck::new(Deck::load()?);
    let store = Store::open_default()?;
    Ok((deck, store))
}

#[component]
fn App() -> Element {
    // Resolve setup once. On failure, log the full chain and surface it rather than
    // launching into a broken loop (AGENTS.md error handling).
    let setup = use_hook(|| {
        app_setup().map_err(|e| {
            error!("{e:#}");
            format!("{e:#}")
        })
    });

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Stylesheet { href: TAILWIND_CSS }
        div { class: "fixed inset-0",
            match setup {
                Ok((deck, store)) => rsx! { ReviewSession { deck, store } },
                Err(err) => rsx! {
                    div { class: "p-6 font-sans text-base text-danger", "Failed to start: {err}" }
                },
            }
        }
    }
}
