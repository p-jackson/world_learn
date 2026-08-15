//! The Home route (spec §4.3, §4.7): the launch surface. Title + tagline, two stat
//! tiles (reviews-due and new-today, derived from the store), and — when there is
//! anything to review — the primary `Start · N cards` action. A Settings link sits
//! below it.
//!
//! Counts come from [`session::counts`] over the persisted store and today's date;
//! the Deck and Store are read from context (provided at the app root). When
//! nothing is due the tiles read 0/0 and Start is simply absent — no special
//! re-entry screen (spec §4.3). The route re-mounts on every navigation back, so
//! the counts always reflect the store as it stands.

use dioxus::prelude::*;

use crate::session::{self, SessionCounts};
use crate::ui::{log_and_display, use_app_context, Failure};
use crate::Route;

/// The Home route. Derives the launch counts once at mount; a store-load failure
/// surfaces rather than silently showing 0/0 (AGENTS.md error handling).
#[component]
pub fn Home() -> Element {
    let (deck, store) = use_app_context();

    let counts = use_hook(move || {
        let today = session::today_local();
        store
            .load()
            .map(|state| session::counts(&state, &deck, today))
            .map_err(|e| log_and_display(&e))
    });

    match counts {
        Err(err) => rsx! { Failure { message: format!("Failed to load: {err}") } },
        Ok(counts) => rsx! { HomeView { counts } },
    }
}

/// The Home presentation — pure over the derived [`SessionCounts`], so the context
/// and store I/O stay in [`Home`]. Start is present only when there is work to do.
#[component]
fn HomeView(counts: SessionCounts) -> Element {
    let nav = use_navigator();
    let total = counts.total();

    rsx! {
        div { class: "flex h-full w-full flex-col bg-ocean px-6 font-sans text-ink \
            pt-[calc(28px+env(safe-area-inset-top))] pb-[calc(24px+env(safe-area-inset-bottom))]",

            div { class: "pt-6",
                h1 { class: "text-[34px] font-[750] tracking-[-0.02em]", "World Learn" }
                p { class: "mt-1 text-[15px] text-ink-dim", "Learn where every country is." }
            }

            // Two stat tiles: reviews-due and new-today (spec §4.3).
            div { class: "mt-8 flex gap-3",
                StatTile { value: counts.due, label: "Reviews due" }
                StatTile { value: counts.new_today, label: "New today" }
            }

            div { class: "flex-1" }

            // Start is present only when something is due or new (spec §4.3).
            if total > 0 {
                button {
                    class: "w-full rounded-2xl bg-accent px-6 py-4 text-center text-[17px] \
                        font-bold text-ink-on-light shadow-[0_6px_24px_#0007]",
                    onclick: move |_| { nav.push(Route::Review {}); },
                    "Start · {total} cards"
                }
            }

            button {
                class: "mt-3 w-full py-3 text-center text-[15px] text-ink-dim",
                onclick: move |_| { nav.push(Route::Settings {}); },
                "Settings"
            }
        }
    }
}

/// One Home stat tile: a big count over a dim label (spec §4.3).
#[component]
fn StatTile(value: usize, label: String) -> Element {
    rsx! {
        div { class: "flex-1 rounded-2xl bg-panel px-4 py-5 text-center",
            div { class: "text-[40px] font-[750] leading-none", "{value}" }
            div { class: "mt-2 text-[13px] text-ink-dim", "{label}" }
        }
    }
}
