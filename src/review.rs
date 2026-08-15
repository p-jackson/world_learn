//! The Review route: the front/reveal presentation (issue 07) wired to the
//! scheduling core so grading drives the session (spec §4.1, §4.5), and the
//! Done-for-today route it lands on (spec §4.5, issue 09 routing).
//!
//! [`Review`] is the route component: it reads the shared [`SharedDeck`] and
//! [`Store`] from context, owns the transient [`Session`] in a signal, hands the
//! current Card to [`CardView`], and advances on each grade. When the queue drains
//! it navigates to [`Route::Done`], carrying the reviewed count.
//!
//! [`CardView`] renders two visual states of one Card, driven by a `revealed`
//! signal the driver owns (so it can reset to front for the next Card). **Front**:
//! the full-bleed regional-zoom map, a thin top status strip (`N left` · progress ·
//! `i/total`), and a single "Tap to reveal" pill; tapping the pill **or the map**
//! reveals. **Reveal**: the common name + formal long name, a 📍 dropped on the
//! entity centroid, and the four grade buttons — each fires `on_grade`, which the
//! driver routes into [`Session::grade`].

use dioxus::prelude::*;
use jiff::civil::Date;

use crate::deck::{Card, SharedDeck};
use crate::map::WorldMap;
use crate::scheduler;
use crate::session::{self, Session};
use crate::store::Store;
use crate::ui::{use_app_context, Failure};
use crate::Route;

/// The four FSRS self-grades, in button order (spec §4.1). A presentation-local
/// enum (labels + colours); [`From`] maps it to the scheduler's [`scheduler::Grade`]
/// when the driver grades the session, keeping the view independent of the core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Grade {
    Again,
    Hard,
    Good,
    Easy,
}

impl Grade {
    /// The four grades top-to-bottom in the stacked button column.
    const ALL: [Self; 4] = [Self::Again, Self::Hard, Self::Good, Self::Easy];

    const fn label(self) -> &'static str {
        match self {
            Self::Again => "Again",
            Self::Hard => "Hard",
            Self::Good => "Good",
            Self::Easy => "Easy",
        }
    }

    /// Tailwind background token per grade — red / orange / green / blue (spec §4.1).
    const fn bg(self) -> &'static str {
        match self {
            Self::Again => "bg-again",
            Self::Hard => "bg-hard",
            Self::Good => "bg-good",
            Self::Easy => "bg-easy",
        }
    }
}

impl From<Grade> for scheduler::Grade {
    /// Map the button-order presentation grade to the FSRS grade the scheduler
    /// advances on. One-to-one; the two enums are kept separate so the view layer
    /// doesn't depend on the scheduling core's type.
    fn from(grade: Grade) -> Self {
        match grade {
            Grade::Again => Self::Again,
            Grade::Hard => Self::Hard,
            Grade::Good => Self::Good,
            Grade::Easy => Self::Easy,
        }
    }
}

/// The current Card's position in the session queue — drives the status strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QueuePosition {
    /// 0-based index of the current Card (Cards passed so far).
    index: usize,
    /// Total distinct Cards in the session.
    total: usize,
}

impl QueuePosition {
    /// Cards remaining including the current one (`N left`).
    const fn left(self) -> usize {
        self.total.saturating_sub(self.index)
    }

    /// Progress-bar fill, whole percent of the session completed so far.
    const fn percent(self) -> usize {
        match (self.index * 100).checked_div(self.total) {
            Some(p) => p,
            None => 0,
        }
    }

    /// The `i/total` counter, 1-based.
    fn counter(self) -> String {
        format!("{}/{}", self.index + 1, self.total)
    }
}

/// Start a session, surfacing a store-load failure as a display string (logged
/// here, the app boundary, per AGENTS.md). [`Review`] renders the returned `Err`
/// as an inline failure message rather than launching into a broken loop.
fn start_session(deck: SharedDeck, store: Store, today: Date) -> Result<Session, String> {
    Session::start(deck, store, today).map_err(|e| {
        error!("{e:#}");
        format!("{e:#}")
    })
}

/// The Review loop route (spec §4.1, §4.5, §4.7): owns the [`Session`] and advances
/// it on each grade. Renders the current Card via [`CardView`] until the queue
/// drains, then navigates to [`Route::Done`] with the reviewed count.
///
/// The Deck and Store come from context (provided at the app root), so the route
/// takes no props. The session is built once from the store at mount and held in a
/// signal; each grade calls [`Session::grade`] (FSRS advance + atomic persist +
/// requeue of an **Again**) and resets the reveal to front for the next Card.
/// Persistence is the Session's own — a mid-session quit is safe because every
/// grade has already written through (an **Again**'s `due = today` re-drill
/// survives, spec §5.4).
#[component]
pub fn Review() -> Element {
    let (deck, store) = use_app_context();
    let nav = use_navigator();
    let today = use_hook(session::today_local);
    let mut session = use_signal({
        let deck = deck.clone();
        move || start_session(deck, store, today)
    });
    let mut revealed = use_signal(|| false);

    // When the queue drains, leave for the Done route carrying the reviewed count
    // (also covers a defensively-empty start). Reading `session` subscribes this
    // effect, so it re-runs after each grade; once it pushes Done the route
    // unmounts, so it fires at most once.
    use_effect(move || {
        if let Ok(s) = &*session.read() {
            if s.is_done() {
                nav.push(Route::Done {
                    reviewed: s.reviewed(),
                });
            }
        }
    });

    // Snapshot what to render, releasing the read guard before building rsx (whose
    // handlers write the same signal). Cloning the current Card out keeps nothing
    // borrowed from the guard. A drained queue renders nothing — the effect above
    // is navigating away this frame.
    let screen = match &*session.read() {
        Err(e) => Err(e.clone()),
        Ok(s) => Ok(s.current().cloned().map(|card| {
            (
                card,
                QueuePosition {
                    index: s.reviewed(),
                    total: s.total(),
                },
            )
        })),
    };

    match screen {
        Err(err) => rsx! { Failure { message: format!("Failed to start review: {err}") } },
        Ok(None) => rsx! {},
        Ok(Some((card, position))) => rsx! {
            CardView {
                deck,
                card,
                position,
                revealed,
                on_grade: move |grade: Grade| {
                    let outcome = {
                        let mut guard = session.write();
                        // The grade buttons only render for an `Ok` session (a failed
                        // one shows the error), so the `Err` arm is unreachable in
                        // practice — a no-op rather than a panic if it ever isn't.
                        guard.as_mut().map_or_else(|_| Ok(()), |s| s.grade(grade.into()))
                    };
                    match outcome {
                        // Advance to the next Card at the front, back to front state.
                        Ok(()) => revealed.set(false),
                        Err(e) => error!("{e:#}"),
                    }
                },
            }
        },
    }
}

/// The Review screen for a single Card: full-bleed map with a front⇄reveal toggle.
///
/// `revealed` is owned by [`Review`] so it can reset to front after a grade;
/// tapping the map layer or the pill reveals. The four grade buttons fire
/// `on_grade`, which the driver routes into the scheduler.
#[component]
fn CardView(
    deck: SharedDeck,
    card: Card,
    position: QueuePosition,
    mut revealed: Signal<bool>,
    on_grade: EventHandler<Grade>,
) -> Element {
    let code = card.code.clone();

    rsx! {
        div { class: "relative h-full w-full overflow-hidden bg-ocean font-sans text-ink",
            // Map layer, full-bleed behind the overlays. Tapping anywhere on it
            // reveals (spec §4.1: tap the map or the pill to reveal).
            div {
                class: "absolute inset-0",
                onclick: move |_| revealed.set(true),
                WorldMap { deck, highlighted: code, pin: revealed() }
            }

            // Thin top status strip: `N left` · progress · `i/total`.
            div {
                class: "absolute inset-x-0 top-0 z-[3] flex items-center gap-[14px] px-[18px] \
                    pb-[14px] pt-[calc(14px+env(safe-area-inset-top))] text-[13px] tracking-[0.02em] \
                    bg-[linear-gradient(#000a,transparent)]",
                span { "{position.left()} left" }
                div { class: "h-1 flex-1 overflow-hidden rounded-[2px] bg-white/10",
                    div { class: "h-full bg-accent", style: "width:{position.percent()}%" }
                }
                span { class: "text-ink-dim", "{position.counter()}" }
            }

            // Bottom overlay: the reveal pill (front) or names + grade column
            // (reveal). The scrim is reveal-only — the front pill is already opaque
            // and self-legible, so the front state stays full-bleed instead of
            // painting a dark band over the home-indicator safe area (the "chin").
            div {
                class: "absolute inset-x-0 bottom-0 z-[3] px-[18px] pt-5 \
                    pb-[calc(20px+env(safe-area-inset-bottom))]",
                class: if revealed() { "bg-[linear-gradient(transparent,#000c_28%)]" },
                if revealed() {
                    div { class: "text-center text-[32px] font-[750] tracking-[-0.01em] [text-shadow:0_2px_10px_#000]",
                        "{card.entity.name}"
                    }
                    div { class: "mt-0.5 mb-4 text-center text-[14px] text-[#dfe8f0] [text-shadow:0_1px_6px_#000]",
                        "{card.entity.name_long}"
                    }
                    div { class: "flex flex-col gap-[6px]",
                        for grade in Grade::ALL {
                            button {
                                class: "flex h-10 items-center rounded-[14px] px-[18px] text-left text-[16px] font-[650] text-ink-on-light {grade.bg()}",
                                onclick: move |_| on_grade.call(grade),
                                "{grade.label()}"
                            }
                        }
                    }
                } else {
                    button {
                        class: "mx-auto block w-[74%] rounded-2xl bg-[#ffffffef] px-4 py-4 text-center \
                            text-[17px] font-bold text-ink-on-light shadow-[0_6px_24px_#0007]",
                        onclick: move |_| revealed.set(true),
                        "Tap to reveal"
                    }
                }
            }
        }
    }
}

/// The done-for-today route (spec §4.5, §4.7): a ✓, "N reviewed · next batch
/// unlocks tomorrow", and a Back-to-home button that navigates to [`Route::Home`].
/// `reviewed` rides in as the route's path segment, set when [`Review`] drains.
#[component]
pub fn Done(reviewed: usize) -> Element {
    let nav = use_navigator();
    // Centred vertically by flanking flex-1 spacers (avoids a justify-center
    // utility the compiled Tailwind doesn't ship).
    rsx! {
        div { class: "flex h-full w-full flex-col items-center bg-ocean px-[18px] text-center font-sans text-ink",
            div { class: "flex-1" }
            div { class: "flex flex-col items-center gap-[14px]",
                div { class: "text-[32px] [text-shadow:0_2px_10px_#000]", "✓" }
                div { class: "text-[17px] text-ink-dim",
                    "{reviewed} reviewed · next batch unlocks tomorrow"
                }
                button {
                    class: "rounded-2xl bg-[#ffffffef] px-6 py-4 text-[16px] font-bold \
                        text-ink-on-light shadow-[0_6px_24px_#0007]",
                    onclick: move |_| { nav.push(Route::Home {}); },
                    "Back to home"
                }
            }
            div { class: "flex-1" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grades_are_the_four_fsrs_labels_in_order() {
        let labels: Vec<&str> = Grade::ALL.iter().map(|g| g.label()).collect();
        assert_eq!(labels, ["Again", "Hard", "Good", "Easy"]);
    }

    #[test]
    fn grade_colours_are_red_orange_green_blue() {
        // The spec's colour order maps to the palette tokens (§4.1).
        assert_eq!(Grade::Again.bg(), "bg-again");
        assert_eq!(Grade::Hard.bg(), "bg-hard");
        assert_eq!(Grade::Good.bg(), "bg-good");
        assert_eq!(Grade::Easy.bg(), "bg-easy");
    }

    #[test]
    fn each_button_grade_maps_to_its_scheduler_grade() {
        // The wiring issue 08 adds: a tapped button must advance FSRS on the same
        // grade it shows. A swapped arm here would silently mis-schedule.
        assert_eq!(
            scheduler::Grade::from(Grade::Again),
            scheduler::Grade::Again
        );
        assert_eq!(scheduler::Grade::from(Grade::Hard), scheduler::Grade::Hard);
        assert_eq!(scheduler::Grade::from(Grade::Good), scheduler::Grade::Good);
        assert_eq!(scheduler::Grade::from(Grade::Easy), scheduler::Grade::Easy);
    }

    #[test]
    fn queue_position_drives_the_status_strip() {
        // First of eleven: eleven left, no progress yet, counter reads 1/11.
        let start = QueuePosition {
            index: 0,
            total: 11,
        };
        assert_eq!(start.left(), 11);
        assert_eq!(start.percent(), 0);
        assert_eq!(start.counter(), "1/11");

        // Midway: fewer left, partial bar, 1-based counter.
        let mid = QueuePosition {
            index: 5,
            total: 11,
        };
        assert_eq!(mid.left(), 6);
        assert_eq!(mid.percent(), 45); // 5/11 → 45%
        assert_eq!(mid.counter(), "6/11");

        // Last card: one left, bar nearly full, counter reads total/total.
        let last = QueuePosition {
            index: 10,
            total: 11,
        };
        assert_eq!(last.left(), 1);
        assert_eq!(last.counter(), "11/11");
    }

    #[test]
    fn empty_session_never_divides_by_zero() {
        let empty = QueuePosition { index: 0, total: 0 };
        assert_eq!(empty.left(), 0);
        assert_eq!(empty.percent(), 0);
    }
}
