//! The world-map render primitive (spec §3.3, §4.1 render path).
//!
//! Draws every Deck Entity once as an inline-SVG `<path>` and highlights one by
//! **varying its `fill` attribute** — never by swapping SVG child nodes. That is
//! the deliberate workaround for Dioxus #2274 (reordering/replacing SVG children
//! corrupts the tree): the node list is fixed to the Deck's order with stable
//! `key`s, so changing the highlighted Card diffs two `fill` attributes and
//! touches no node structure.
//!
//! Projection is the pipeline's equirectangular `(lon, -lat)` degree space
//! (issue 01): x = longitude, y = negative latitude so north is up. The whole
//! world therefore spans x ∈ [-180, 180], y ∈ [-90, 90] — [`WORLD_VIEW_BOX`].
//! This ticket renders at world scale only; per-Card framing/zoom is issue 06.

use std::ops::Deref;
use std::rc::Rc;

use dioxus::prelude::*;

use crate::deck::Deck;

/// `viewBox` covering the whole equirectangular world: `min-x min-y width height`
/// for x ∈ [-180, 180], y ∈ [-90, 90] (see module docs for the projection).
pub const WORLD_VIEW_BOX: &str = "-180 -90 360 180";

/// Base fill for every non-highlighted Entity (prototype `--land`).
pub const BASE_FILL: &str = "#26323f";
/// Fill for the single highlighted Entity (prototype `--target`).
pub const HIGHLIGHT_FILL: &str = "#f5b301";

/// The `fill` attribute for one Entity: highlight colour when its `code` is the
/// highlighted one, base colour otherwise. An unknown highlight code (not a Deck
/// member) leaves every Entity at [`BASE_FILL`].
pub fn fill_for(code: &str, highlighted: &str) -> &'static str {
    if code == highlighted {
        HIGHLIGHT_FILL
    } else {
        BASE_FILL
    }
}

/// A shared, immutable [`Deck`] cheap enough to pass as a prop: an `Rc` compared
/// by pointer identity. The Deck is built once and never mutated, so identity
/// equality is the right prop-diff — it keeps [`WorldMap`] from re-rendering when
/// only an unrelated signal changed.
#[derive(Clone)]
pub struct SharedDeck(Rc<Deck>);

impl SharedDeck {
    pub fn new(deck: Deck) -> Self {
        Self(Rc::new(deck))
    }
}

impl PartialEq for SharedDeck {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Deref for SharedDeck {
    type Target = Deck;

    fn deref(&self) -> &Deck {
        &self.0
    }
}

/// The world map with one Entity highlighted. Fills its container (`h-full
/// w-full`); the caller sizes it (full-bleed on the Review screen).
///
/// Renders all Deck paths once in fixed intro order; only the highlighted Card's
/// `fill` differs. The boundary stroke is uniform and non-scaling, applied to
/// every child `<path>` via one Tailwind child-variant on the `<svg>`, so it
/// stays visible on every Entity in both states independent of `fill`.
#[component]
pub fn WorldMap(deck: SharedDeck, highlighted: ReadSignal<String>) -> Element {
    let highlighted = highlighted();
    rsx! {
        svg {
            class: "block h-full w-full \
                [&_path]:stroke-land-edge [&_path]:stroke-1 \
                [&_path]:[vector-effect:non-scaling-stroke] [&_path]:[stroke-linejoin:round]",
            view_box: WORLD_VIEW_BOX,
            preserve_aspect_ratio: "xMidYMid meet",
            for card in deck.cards() {
                path {
                    key: "{card.code}",
                    d: "{card.entity.d}",
                    fill: fill_for(&card.code, &highlighted),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_for_marks_only_the_matching_code() {
        assert_eq!(fill_for("FRA", "FRA"), HIGHLIGHT_FILL);
        assert_eq!(fill_for("DEU", "FRA"), BASE_FILL);
        // Unknown highlight code → nothing highlighted.
        assert_eq!(fill_for("FRA", "ZZZ"), BASE_FILL);
    }

    #[test]
    fn world_view_box_spans_the_full_projection() {
        let nums: Vec<f64> = WORLD_VIEW_BOX
            .split_whitespace()
            .map(|n| n.parse().unwrap())
            .collect();
        // min-x, min-y, width, height covering x∈[-180,180], y∈[-90,90].
        assert_eq!(nums, [-180.0, -90.0, 360.0, 180.0]);
    }

    #[test]
    fn exactly_one_entity_highlighted_over_the_real_deck() {
        let deck = Deck::load().unwrap();
        // The ticket's core claim: a valid code highlights exactly one Entity and
        // leaves the rest at base fill; a non-member highlights none.
        let highlit = |target: &str| {
            deck.cards()
                .iter()
                .filter(|c| fill_for(&c.code, target) == HIGHLIGHT_FILL)
                .count()
        };
        assert_eq!(highlit("FRA"), 1);
        assert_eq!(highlit("RUS"), 1);
        assert_eq!(highlit("ZZZ"), 0);
    }

    #[test]
    fn shared_deck_compares_by_identity() {
        let a = SharedDeck::new(Deck::load().unwrap());
        let b = a.clone();
        // Clones share the Rc → equal; an independently loaded Deck is not.
        // (`assert!` rather than `assert_ne!` so `SharedDeck` needn't impl Debug.)
        assert!(a == b);
        assert!(a != SharedDeck::new(Deck::load().unwrap()));
        // Deref reaches the Deck's own surface.
        assert_eq!(a.len(), 240);
    }
}
