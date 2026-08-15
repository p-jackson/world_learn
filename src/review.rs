//! The Review screen: the front/reveal presentation (issue 07) wired to the
//! scheduling core so grading drives the session (spec §4.1, §4.5, issue 08).
//!
//! [`ReviewSession`] is the entry point: it owns the transient [`Session`] in a
//! signal, hands the current Card to [`Review`], and advances on each grade. When
//! the session queue drains it swaps to [`DoneForToday`].
//!
//! [`Review`] renders two visual states of one Card, driven by a `revealed` signal
//! the driver owns (so it can reset to front for the next Card). **Front**: the
//! full-bleed regional-zoom map, a thin top status strip (`N left` · progress ·
//! `i/total`), and a single "Tap to reveal" pill; tapping the pill **or the map**
//! reveals. **Reveal**: the common name + formal long name, a 📍 dropped on the
//! entity centroid, and the four grade buttons — each fires `on_grade`, which the
//! driver routes into [`Session::grade`].

use dioxus::prelude::*;
use jiff::civil::Date;

use crate::deck::{Card, SharedDeck};
use crate::map::WorldMap;
use crate::scheduler;
use crate::session::Session;
use crate::store::Store;

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

/// The local device date (spec §5.4: the day boundary is local midnight). Captured
/// once per session mount so every grade in the session stamps the same day.
fn today_local() -> Date {
    jiff::Zoned::now().date()
}

/// Start a session, surfacing a store-load failure as a display string (logged
/// here, the app boundary, per AGENTS.md). Shared by the initial mount and the
/// restart path so both fail the same way — into the [`Screen::Failed`] state.
fn start_session(deck: SharedDeck, store: Store, today: Date) -> Result<Session, String> {
    Session::start(deck, store, today).map_err(|e| {
        error!("{e:#}");
        format!("{e:#}")
    })
}

/// What [`ReviewSession`] should render this frame, snapshotted from the session
/// signal so its read guard is dropped before any handler writes back.
enum Screen {
    /// The session failed to start (store load error) — surfaced, never silent.
    Failed(String),
    /// The queue is drained: done-for-today with this many Cards reviewed.
    Done(usize),
    /// A Card to review at this queue position.
    Card(Card, QueuePosition),
}

/// The Review loop: owns the [`Session`] and advances it on each grade (spec §4.1,
/// §4.5, §5.4). Renders the current Card via [`Review`] until the queue drains,
/// then [`DoneForToday`].
///
/// The session is built once from the store at mount and held in a signal; each
/// grade calls [`Session::grade`] (FSRS advance + atomic persist + requeue of an
/// **Again**) and resets the reveal to front for the next Card. Persistence is the
/// Session's own — a mid-session quit is safe because every grade has already
/// written through (an **Again**'s `due = today` re-drill survives, spec §5.4).
#[component]
pub fn ReviewSession(deck: SharedDeck, store: Store) -> Element {
    let today = use_hook(today_local);
    let mut session = use_signal({
        let deck = deck.clone();
        let store = store.clone();
        move || start_session(deck, store, today)
    });
    let mut revealed = use_signal(|| false);

    // Snapshot what to render, releasing the read guard before building rsx (whose
    // handlers write the same signal). Cloning the current Card out keeps nothing
    // borrowed from the guard.
    let screen = match &*session.read() {
        Err(e) => Screen::Failed(e.clone()),
        Ok(s) if s.is_done() => Screen::Done(s.reviewed()),
        Ok(s) => Screen::Card(
            s.current()
                .cloned()
                .expect("a non-done session always has a current Card"),
            QueuePosition {
                index: s.reviewed(),
                total: s.total(),
            },
        ),
    };

    match screen {
        Screen::Failed(err) => rsx! {
            div { class: "p-6 font-sans text-base text-danger", "Failed to start review: {err}" }
        },
        Screen::Done(reviewed) => rsx! {
            DoneForToday {
                reviewed,
                on_home: move |()| {
                    // No Home screen yet (issue 09 owns routing). Restart re-derives
                    // from the persisted store: after grading through, nothing is due
                    // and today's cap is spent, so it lands straight back on
                    // done-for-today — proof the passes persisted. A reload failure
                    // surfaces the same as at mount (the Failed screen).
                    session.set(start_session(deck.clone(), store.clone(), today));
                    revealed.set(false);
                },
            }
        },
        Screen::Card(card, position) => rsx! {
            Review {
                deck,
                card,
                position,
                revealed,
                on_grade: move |grade: Grade| {
                    let outcome = {
                        let mut guard = session.write();
                        // The grade buttons only render for an `Ok` session (a failed
                        // one shows `Screen::Failed`), so the `Err` arm is unreachable
                        // in practice — a no-op rather than a panic if it ever isn't.
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
/// `revealed` is owned by [`ReviewSession`] so it can reset to front after a grade;
/// tapping the map layer or the pill reveals. The four grade buttons fire
/// `on_grade`, which the driver routes into the scheduler.
#[component]
fn Review(
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

            // Bottom overlay: the reveal pill (front) or names + grade column (reveal).
            div {
                class: "absolute inset-x-0 bottom-0 z-[3] px-[18px] pt-5 \
                    pb-[calc(20px+env(safe-area-inset-bottom))] bg-[linear-gradient(transparent,#000c_28%)]",
                if revealed() {
                    div { class: "text-center text-[32px] font-[750] tracking-[-0.01em] [text-shadow:0_2px_10px_#000]",
                        "{card.entity.name}"
                    }
                    div { class: "mt-0.5 mb-4 text-center text-[14px] text-[#dfe8f0] [text-shadow:0_1px_6px_#000]",
                        "{card.entity.name_long}"
                    }
                    div { class: "flex flex-col gap-[9px]",
                        for grade in Grade::ALL {
                            button {
                                class: "rounded-[14px] px-[18px] py-[15px] text-left text-[16px] font-[650] text-ink-on-light {grade.bg()}",
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

/// The done-for-today end state (spec §4.5): a ✓, "N reviewed · next batch unlocks
/// tomorrow", and a Back-to-home button. `on_home` is wired by [`ReviewSession`]
/// (routing to a real Home arrives in issue 09).
#[component]
fn DoneForToday(reviewed: usize, on_home: EventHandler<()>) -> Element {
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
                    onclick: move |_| on_home.call(()),
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
