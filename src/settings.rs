//! The Settings route: the new-cards/day stepper — the only interactive control —
//! plus two read-only rows (Scheduler, Deck) and a back link to Home.
//!
//! The stepper's value loads from the store once at mount; each `+`/`−` writes it
//! straight back through the store (the single source of truth), so Home re-reads
//! the new allowance when it re-mounts. The Deck and Store are read from context
//! (provided at the app root).

use dioxus::prelude::*;

use crate::store::DEFAULT_NEW_CARDS_PER_DAY;
use crate::ui::use_app_context;
use crate::Route;

/// Stepper ceiling. Floor is 0 (disables new-Card introductions); the ceiling keeps
/// the value a tidy two digits — well above any realistic daily pace, and an
/// over-large cap is harmless anyway (the backlog floors the actual intro count).
const MAX_NEW_CARDS_PER_DAY: u32 = 99;

/// The Settings route.
#[component]
pub fn Settings() -> Element {
    let (deck, store) = use_app_context();
    let nav = use_navigator();
    let deck_len = deck.len();

    // Load the persisted value once; a load failure falls back to the default
    // rather than blocking the screen (the store logs its own errors on write).
    let mut value = use_signal({
        let store = store.clone();
        move || {
            store
                .load()
                .map_or(DEFAULT_NEW_CARDS_PER_DAY, |s| s.settings.new_cards_per_day)
        }
    });

    // Set the value and persist it. Surfaces a write failure to the log but keeps
    // the UI responsive; the on-screen number always reflects the intent.
    // `Clone` (the Store is cheap to clone, the Signal is Copy) so each stepper
    // button gets its own handle.
    let commit = {
        let store = store.clone();
        move |n: u32| {
            value.set(n);
            if let Err(e) = store.set_new_cards_per_day(n) {
                crate::observability::report(&e);
            }
        }
    };

    // Destructive: erase all review history and settings. Guarded by a two-tap
    // confirm (the first tap arms it) so a stray tap can't wipe progress. On
    // clear, snap the stepper back to the default the fresh store will report.
    let mut confirm_clear = use_signal(|| false);
    let clear_memory = move |_| {
        if confirm_clear() {
            if let Err(e) = store.clear() {
                crate::observability::report(&e);
            }
            value.set(DEFAULT_NEW_CARDS_PER_DAY);
            confirm_clear.set(false);
        } else {
            confirm_clear.set(true);
        }
    };

    rsx! {
        div { class: "flex h-full w-full flex-col bg-ocean px-6 font-sans text-ink \
            pt-[calc(20px+env(safe-area-inset-top))] pb-[calc(24px+env(safe-area-inset-bottom))]",

            div { class: "flex items-center gap-3 pt-2",
                button {
                    class: "text-[26px] leading-none text-ink-dim",
                    onclick: move |_| { nav.push(Route::Home {}); },
                    "‹"
                }
                h1 { class: "text-[24px] font-[700]", "Settings" }
            }

            // The one interactive control: the new-cards/day stepper.
            div { class: "mt-8 flex items-center justify-between rounded-2xl bg-panel px-5 py-4",
                div {
                    div { class: "text-[16px] font-[600]", "New cards per day" }
                    div { class: "mt-0.5 text-[13px] text-ink-dim", "New places introduced each day" }
                }
                div { class: "flex items-center gap-4",
                    button {
                        class: "h-9 w-9 rounded-full bg-line text-[22px] font-bold leading-none text-ink",
                        onclick: {
                            let mut commit = commit.clone();
                            move |_| { commit(value().saturating_sub(1)); }
                        },
                        "−"
                    }
                    span { class: "w-8 text-center text-[20px] font-[700] tabular-nums", "{value}" }
                    button {
                        class: "h-9 w-9 rounded-full bg-line text-[22px] font-bold leading-none text-ink",
                        onclick: {
                            let mut commit = commit;
                            move |_| { commit((value() + 1).min(MAX_NEW_CARDS_PER_DAY)); }
                        },
                        "+"
                    }
                }
            }

            // Read-only rows.
            div { class: "mt-4 rounded-2xl bg-panel",
                ReadonlyRow { label: "Scheduler", value: "FSRS".to_string() }
                div { class: "mx-5 h-px bg-line" }
                ReadonlyRow { label: "Deck", value: "{deck_len}, incl. contested" }
            }

            // Destructive: erase all review history and settings.
            button {
                class: "mt-8 rounded-2xl bg-panel px-5 py-4 text-left text-[16px] font-[600] text-danger",
                onclick: clear_memory,
                if confirm_clear() {
                    "Tap again to erase all progress"
                } else {
                    "Clear all memory"
                }
            }
        }
    }
}

/// A non-interactive Settings row: a label with a dim value.
#[component]
fn ReadonlyRow(label: String, value: String) -> Element {
    rsx! {
        div { class: "flex items-center justify-between px-5 py-4",
            span { class: "text-[16px]", "{label}" }
            span { class: "text-[15px] text-ink-dim", "{value}" }
        }
    }
}
