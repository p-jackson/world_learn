use dioxus::prelude::*;

mod deck;
mod map;
mod scheduler;
mod session;
mod store;

use deck::Deck;
use map::{SharedDeck, WorldMap};

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
            Ok(deck) => rsx! { MapScreen { deck } },
            Err(err) => rsx! {
                div { class: "p-6 font-sans text-base text-danger", "Failed to load map: {err}" }
            },
        }
    }
}

/// Throwaway scaffold for issue 05: shows the map full-bleed with a dev bar to
/// pick which `ADM0_A3` is highlighted (type a code, or step through the Deck).
/// Real navigation is issue 09; this exists only to demo the render primitive.
#[component]
fn MapScreen(deck: SharedDeck) -> Element {
    let len = deck.len();
    let mut idx = use_signal(|| 0usize);
    let card = deck.cards()[idx()].clone();

    let jump_deck = deck.clone();
    let step = "rounded-[10px] bg-land px-[18px] py-1.5 text-[22px] leading-none text-ink";
    rsx! {
        div { class: "fixed inset-0 flex flex-col bg-ocean",
            div { class: "min-h-0 flex-1",
                WorldMap { deck, highlighted: card.code.clone() }
            }
            div { class: "flex items-center gap-3 bg-panel px-4 py-3 font-sans text-[15px] text-ink",
                button {
                    class: step,
                    onclick: move |_| idx.set((idx() + len - 1) % len),
                    "‹"
                }
                input {
                    class: "w-[4.5em] rounded-[10px] border border-line bg-ocean px-2 py-1.5 uppercase tracking-[0.08em] text-ink",
                    value: "{card.code}",
                    maxlength: 3,
                    autocapitalize: "characters",
                    oninput: move |e| {
                        let code = e.value().to_uppercase();
                        if let Some(i) = jump_deck.cards().iter().position(|c| c.code == code) {
                            idx.set(i);
                        }
                    },
                }
                span { class: "flex-1 text-ink-dim", "{card.entity.name}" }
                button {
                    class: step,
                    onclick: move |_| idx.set((idx() + 1) % len),
                    "›"
                }
            }
        }
    }
}
