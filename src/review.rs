//! The Review screen's front/reveal presentation (spec §4.1, issue 07).
//!
//! Two visual states of one Card, driven by a local reveal toggle — no scheduler
//! wiring yet (issue 08). **Front**: the full-bleed regional-zoom map, a thin top
//! status strip (`N left` · progress · `i/total`), and a single "Tap to reveal"
//! pill; tapping the pill **or the map** reveals. **Reveal**: the common name +
//! formal long name, a 📍 dropped on the entity centroid, and the four grade
//! buttons. The grade buttons render and are styled but are **inert** this ticket
//! — issue 08 gives them an `on_grade` and wires the session queue.
//!
//! This is the greenfield component the loop will later drive; it lands ahead of
//! that wiring, so [`QueuePosition`] is a plain prop the demo hardcodes rather than
//! transient session state.

use dioxus::prelude::*;

use crate::deck::Card;
use crate::map::{SharedDeck, WorldMap};

/// The four FSRS self-grades, in button order (spec §4.1). Labels and colours only
/// this ticket — grading is wired to the scheduler in issue 08.
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

/// The current Card's position in the session queue — drives the status strip.
/// Transient session state that issue 08 owns; a plain prop here so the front/reveal
/// presentation is demoable without the scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueuePosition {
    /// 0-based index of the current Card in the session.
    pub index: usize,
    /// Total Cards in the session.
    pub total: usize,
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

/// The Review screen for a single Card: full-bleed map with a front⇄reveal toggle.
///
/// The reveal state is local component state; tapping the map layer or the pill
/// flips it. The grade buttons are inert this ticket (issue 08 wires grading).
#[component]
pub fn Review(deck: SharedDeck, card: Card, position: QueuePosition) -> Element {
    let mut revealed = use_signal(|| false);
    let code = card.code.clone();

    rsx! {
        div { class: "relative h-full w-full overflow-hidden bg-ocean font-sans text-ink",
            // Map layer, full-bleed behind the overlays. Tapping anywhere on it
            // toggles reveal — the front⇄reveal demo affordance while grading is
            // inert; issue 08 replaces it with advance-on-grade.
            div {
                class: "absolute inset-0",
                onclick: move |_| revealed.toggle(),
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
