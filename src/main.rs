use dioxus::prelude::*;

mod deck;
mod home;
mod map;
mod review;
mod scheduler;
mod session;
mod settings;
mod store;
mod ui;

use deck::{Deck, SharedDeck};
use home::Home;
use review::{Done, Review};
use settings::Settings;
use store::Store;
use ui::{log_and_display, Failure};

const FAVICON: Asset = asset!("/assets/favicon.ico");
// Tailwind output, compiled from `tailwind.css` by `dx serve` (Dioxus 0.7
// automatic Tailwind). All styling is utility classes; there is no bespoke CSS.
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

/// The app's four screens and their transitions: Home → Review → Done → Home,
/// and Home ⇄ Settings. [`Shell`] is the outlet layout every screen renders
/// into. [`Done`] carries the reviewed count as a path segment, set when the Review
/// queue drains.
///
/// Every transition is a forward `push` — including the "back to Home" legs. There
/// is no back-gesture chrome on iOS full-screen, so `push` (which lands on the
/// right screen regardless of how the current one was reached) is simpler and safer
/// than `go_back` here: Done → Home must not pop back into the drained session.
#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[layout(Shell)]
    #[route("/")]
    Home {},
    #[route("/review")]
    Review {},
    #[route("/settings")]
    Settings {},
    #[route("/done/:reviewed")]
    Done { reviewed: usize },
}

fn main() {
    // Route tracing output to the platform log. On iOS the fmt subscriber writes
    // to stdout, which `dx serve --ios` and Xcode capture. `dioxus::launch` also
    // auto-inits this, but doing it here first covers any pre-launch logging and
    // makes the setup explicit.
    dioxus::logger::initialize_default();

    // Full-bleed presentation: paint the `body` the ocean colour so any exposed
    // safe-area band reads as dark ocean, not the default white. The colour
    // mirrors `--color-ocean` in assets/tailwind.css; kept as a literal since this
    // runs before that stylesheet loads.
    //
    // The `viewport-fit=cover` meta is aspirational: WKWebView still insets the
    // layout viewport by the bottom home-indicator safe area, so the map does not
    // actually render into that strip (the ocean `body` colour fills it instead).
    // Making the map truly reach the home indicator needs a native WKWebView tweak,
    // deferred for now.
    let cfg = dioxus::mobile::Config::new().with_custom_head(
        r#"<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<style>html, body { background-color: #0f1720; }</style>"#
            .to_string(),
    );
    dioxus::LaunchBuilder::new().with_cfg(cfg).launch(App);
}

/// Resolve the two long-lived handles every screen needs: the Deck (parsed from the
/// embedded geometry asset) and the store (the iOS Application Support file).
/// One-time, fallible setup — the app boundary logs the chain and surfaces it.
fn app_setup() -> anyhow::Result<(SharedDeck, Store)> {
    let deck = SharedDeck::new(Deck::load()?);
    let store = Store::open_default()?;
    Ok((deck, store))
}

#[component]
fn App() -> Element {
    // Resolve setup once. On failure, log the full chain and surface it rather than
    // launching into a broken loop.
    let setup = use_hook(|| app_setup().map_err(|e| log_and_display(&e)));

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Stylesheet { href: TAILWIND_CSS }
        div { class: "fixed inset-0",
            match setup {
                Ok((deck, store)) => rsx! { AppRouter { deck, store } },
                Err(err) => rsx! { Failure { message: format!("Failed to start: {err}") } },
            }
        }
    }
}

/// Provide the Deck and Store to every screen via context, then mount the router.
/// Context is provided above [`Router`] so the route components resolve it with
/// [`use_context`]; both handles (an `Rc` and a path) are cheap to share.
#[component]
fn AppRouter(deck: SharedDeck, store: Store) -> Element {
    use_context_provider(|| deck);
    use_context_provider(|| store);
    rsx! { Router::<Route> {} }
}

/// The router outlet layout: a full-size ground the per-screen content fills.
/// Kept minimal — each screen owns its own chrome.
#[component]
fn Shell() -> Element {
    rsx! {
        div { class: "h-full w-full bg-ocean", Outlet::<Route> {} }
    }
}
