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
//!
//! The highlighted Card's regional zoom is a pure per-Card [`Frame`]: a `viewBox`
//! swap plus one group `transform` on the single rendered map — no re-projection,
//! no path re-render (issue 06, spec §3.2/§4.2). With no highlight the map falls
//! back to [`WORLD_VIEW_BOX`].

use dioxus::prelude::*;

use crate::deck::SharedDeck;

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

/// Padding multiplier on the mainland bbox → the framed window (spec §4.2).
const FRAME_PADDING: f64 = 3.4;
/// Floor on the framed window's span, in projected degrees, so tiny/island
/// entities get a sane window rather than a pinpoint (§4.2: ~6° bbox → ~20°).
const MIN_WINDOW_DEG: f64 = 20.0;
/// Floor on the cos(lat) x-scale — a degenerate guard for near-polar frame
/// centres so the transform never collapses the map to a vertical line. No
/// inhabited Entity's mainland reaches it.
const MIN_COS_SCALE: f64 = 0.1;
/// Reveal-pin glyph size as a fraction of the framed window's span. The square
/// `viewBox` always maps to the same rendered box under `meet`, so a span-relative
/// size keeps the pin's apparent size roughly constant across Cards.
const PIN_SIZE_FRACTION: f64 = 0.06;

/// The regional-zoom framing for one Entity: a square `viewBox` plus a horizontal
/// cos(lat) correction, both derived from the Entity's **mainland** bbox — the
/// asset's `bbox`, already the largest polygon, so France frames on the European
/// mainland, not out across the Atlantic to French Guiana (spec §4.2).
///
/// A Card's zoom is applying this Frame to the one rendered map: [`Self::view_box`]
/// swaps the `viewBox`, [`Self::transform`] sets one group `transform`. Nothing
/// re-projects and no `<path>` changes.
///
/// Fields are the projected-degree geometry (`x = lon`, `y = -lat`); the string
/// builders format them as SVG attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// Frame-centre x (projected longitude) — the cos-correction pivot.
    center_x: f64,
    /// Frame-centre y (projected −latitude).
    center_y: f64,
    /// Side of the square window, in projected degrees.
    span: f64,
    /// Horizontal scale = cos(frame-centre latitude): undoes the equirectangular
    /// high-latitude horizontal stretch, applied about the frame centre.
    scale_x: f64,
}

impl Frame {
    /// Frame the mainland `bbox` = `[minx, miny, maxx, maxy]` (projected coords).
    ///
    /// The window is square and padded around the mainland, floored to a minimum
    /// span. The longitude extent is compressed by cos(lat) before choosing the
    /// square side, so the window frames the mainland's real-world footprint (the
    /// same correction the group transform then applies to the geometry) rather
    /// than its equirectangular-stretched one.
    pub fn for_bbox(bbox: [f64; 4]) -> Self {
        let [min_x, min_y, max_x, max_y] = bbox;
        let center_x = f64::midpoint(min_x, max_x);
        let center_y = f64::midpoint(min_y, max_y);
        // y = −lat, so the frame's midpoint latitude is −center_y.
        let scale_x = (-center_y).to_radians().cos().max(MIN_COS_SCALE);
        let corrected_width = (max_x - min_x) * scale_x;
        let height = max_y - min_y;
        let span = (corrected_width.max(height) * FRAME_PADDING).max(MIN_WINDOW_DEG);
        Self {
            center_x,
            center_y,
            span,
            scale_x,
        }
    }

    /// The square `viewBox` string `min-x min-y span span`, centred on the mainland.
    pub fn view_box(&self) -> String {
        let half = self.span / 2.0;
        format!(
            "{} {} {} {}",
            svg_num(self.center_x - half),
            svg_num(self.center_y - half),
            svg_num(self.span),
            svg_num(self.span),
        )
    }

    /// The group `transform`: scale x by cos(lat) about the frame centre so
    /// high-latitude entities aren't horizontally stretched (§3.2). This is
    /// `translate(cx) scale(k) translate(-cx)` collapsed to one translate + scale
    /// (`x ↦ k·x + cx·(1−k)`), leaving y untouched.
    pub fn transform(&self) -> String {
        let tx = self.center_x * (1.0 - self.scale_x);
        format!(
            "translate({} 0) scale({} 1)",
            svg_num(tx),
            svg_num(self.scale_x)
        )
    }

    /// Post-transform x for a raw projected x — the same cos(lat) correction the
    /// group `transform` applies to the geometry (`x ↦ k·x + cx·(1−k)`). The reveal
    /// pin is drawn *outside* that group (so its glyph isn't horizontally squished),
    /// so it must carry the correction itself to land over the entity's centroid.
    fn correct_x(&self, x: f64) -> f64 {
        self.scale_x
            .mul_add(x, self.center_x * (1.0 - self.scale_x))
    }

    /// Reveal-pin glyph size in projected-degree user units ([`PIN_SIZE_FRACTION`]
    /// of the framed span).
    fn pin_font_size(&self) -> f64 {
        self.span * PIN_SIZE_FRACTION
    }

    /// The horizontal cos(lat) correction factor — for tests to assert the
    /// correction directly rather than reverse-parsing [`Self::transform`].
    #[cfg(test)]
    pub const fn scale_x(&self) -> f64 {
        self.scale_x
    }
}

/// Format a number for an SVG attribute: rounded to 3 decimals, with trailing
/// zeros and any negative zero normalised away so attributes stay compact and
/// stable. `+ 0.0` collapses IEEE `-0.0` to `+0.0` (`-0.0 + 0.0 == +0.0`).
fn svg_num(v: f64) -> String {
    let rounded = (v * 1000.0).round() / 1000.0 + 0.0;
    let mut s = format!("{rounded:.3}");
    while s.contains('.') && (s.ends_with('0') || s.ends_with('.')) {
        s.pop();
    }
    s
}

/// The world map with one Entity highlighted. Fills its container (`h-full
/// w-full`); the caller sizes it (full-bleed on the Review screen).
///
/// Renders all Deck paths once in fixed intro order; only the highlighted Card's
/// `fill` differs. The boundary stroke is uniform and non-scaling, applied to
/// every child `<path>` via one Tailwind child-variant on the `<svg>`, so it
/// stays visible on every Entity in both states independent of `fill` and of the
/// group's cos-correction scale.
///
/// The highlighted Card's [`Frame`] drives the regional zoom: its `viewBox` swap
/// and one group `transform`. With no valid highlight the map shows the whole
/// world ([`WORLD_VIEW_BOX`], no transform). Both are attributes on fixed nodes,
/// so a Card change re-frames without touching the path tree (Dioxus #2274).
///
/// When `pin` is set, a 📍 marks the highlighted Card's centroid (the reveal
/// state, spec §4.1). It renders as a `<text>` *outside* the cos-correction group
/// — so its glyph isn't horizontally squished — carrying the correction on its own
/// x ([`Frame::correct_x`]). The node is always present for a highlighted Card and
/// only its visibility toggles with `pin`, so front⇄reveal never restructures the
/// SVG children (Dioxus #2274).
#[component]
pub fn WorldMap(
    deck: SharedDeck,
    highlighted: ReadSignal<String>,
    #[props(default)] pin: bool,
) -> Element {
    let highlighted = highlighted();
    let highlighted_card = deck.get(&highlighted);
    let frame = highlighted_card.map(|card| Frame::for_bbox(card.entity.bbox));
    let view_box = frame
        .as_ref()
        .map_or_else(|| WORLD_VIEW_BOX.to_string(), Frame::view_box);
    let transform = frame.as_ref().map(Frame::transform);
    let pin_marker = frame.as_ref().zip(highlighted_card).map(|(f, card)| {
        let [cx, cy] = card.entity.centroid;
        (
            svg_num(f.correct_x(cx)),
            svg_num(cy),
            svg_num(f.pin_font_size()),
        )
    });
    rsx! {
        svg {
            class: "block h-full w-full \
                [&_path]:stroke-land-edge [&_path]:stroke-1 \
                [&_path]:[vector-effect:non-scaling-stroke] [&_path]:[stroke-linejoin:round]",
            view_box: "{view_box}",
            preserve_aspect_ratio: "xMidYMid meet",
            g {
                transform,
                for card in deck.cards() {
                    path {
                        key: "{card.code}",
                        d: "{card.entity.d}",
                        fill: fill_for(&card.code, &highlighted),
                    }
                }
            }
            if let Some((px, py, size)) = pin_marker {
                text {
                    x: "{px}",
                    y: "{py}",
                    "font-size": "{size}",
                    "text-anchor": "middle",
                    class: if pin { "[filter:drop-shadow(0_1px_2px_#000a)]" } else { "opacity-0" },
                    "📍"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deck::Deck;

    #[test]
    fn fill_for_marks_only_the_matching_code() {
        assert_eq!(fill_for("FRA", "FRA"), HIGHLIGHT_FILL);
        assert_eq!(fill_for("DEU", "FRA"), BASE_FILL);
        // Unknown highlight code → nothing highlighted.
        assert_eq!(fill_for("FRA", "ZZZ"), BASE_FILL);
    }

    /// Parse a `viewBox` string into `[min-x, min-y, width, height]`.
    fn parse_view_box(vb: &str) -> [f64; 4] {
        let nums: Vec<f64> = vb.split_whitespace().map(|n| n.parse().unwrap()).collect();
        [nums[0], nums[1], nums[2], nums[3]]
    }

    /// Assert two projected-degree quantities are equal within rounding noise.
    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }

    /// The [`Frame`] for a real-deck Entity by code.
    fn frame_for(deck: &Deck, code: &str) -> Frame {
        Frame::for_bbox(deck.get(code).unwrap().entity.bbox)
    }

    #[test]
    fn frame_centres_a_square_window_on_the_bbox() {
        // Symmetric bbox on the equator (lat 0 → no cos correction): the window is
        // centred and square, padded by FRAME_PADDING off the larger dimension.
        let frame = Frame::for_bbox([-5.0, -5.0, 5.0, 5.0]);
        let [min_x, min_y, width, height] = parse_view_box(&frame.view_box());
        approx(width, 34.0); // square, 10° × 3.4 padding
        approx(height, 34.0);
        approx(min_x + width / 2.0, 0.0); // centred on the bbox
        approx(min_y + height / 2.0, 0.0);
    }

    #[test]
    fn frame_stays_square_for_a_wide_bbox() {
        // A wide, short mainland still frames square, sized off the larger side.
        let frame = Frame::for_bbox([-30.0, -2.0, 30.0, 2.0]);
        let [_, _, w, h] = parse_view_box(&frame.view_box());
        approx(w, h); // viewBox width equals height
        approx(w, 60.0 * FRAME_PADDING); // sized off the 60° width
    }

    #[test]
    fn frame_enforces_the_minimum_window_for_tiny_entities() {
        // Nauru-scale bbox (~0.05°): padding alone gives a pinpoint, so the floor
        // takes over and the window is exactly the minimum span.
        let frame = Frame::for_bbox([166.91, 0.49, 166.96, 0.55]);
        let [_, _, w, h] = parse_view_box(&frame.view_box());
        approx(w, MIN_WINDOW_DEG);
        approx(h, MIN_WINDOW_DEG);
    }

    #[test]
    fn frame_is_identity_scale_on_the_equator() {
        // lat 0 → cos = 1 → no horizontal compression.
        let frame = Frame::for_bbox([-5.0, -5.0, 5.0, 5.0]);
        assert_eq!(frame.transform(), "translate(0 0) scale(1 1)");
    }

    #[test]
    fn frame_compresses_x_at_high_latitude() {
        // Frame centred at lat 60 (center_y = −60) → cos 60° = 0.5: the group
        // transform halves x about the centre so the entity isn't stretched.
        let frame = Frame::for_bbox([10.0, -70.0, 30.0, -50.0]);
        // center_x = 20, k = 0.5 → tx = 20·(1−0.5) = 10.
        assert_eq!(frame.transform(), "translate(10 0) scale(0.5 1)");
        // Corrected width 20·0.5 = 10 < height 20 → span off the height.
        let [_, _, w, _] = parse_view_box(&frame.view_box());
        approx(w, 20.0 * FRAME_PADDING);
    }

    #[test]
    fn pin_x_is_the_fixed_point_at_the_frame_centre() {
        // The cos-correction pivots about the frame centre, so a centroid on the
        // centre-x isn't shifted regardless of latitude.
        let frame = Frame::for_bbox([10.0, -70.0, 30.0, -50.0]); // center_x = 20
        approx(frame.correct_x(20.0), 20.0);
    }

    #[test]
    fn pin_x_compresses_toward_the_centre_at_high_latitude() {
        // center_x = 20, k = cos60° = 0.5: a centroid 10° east of centre is pulled
        // to half that offset (x ↦ k·x + cx·(1−k)), matching the geometry the group
        // transform draws so the pin lands on the entity, not beside it.
        let frame = Frame::for_bbox([10.0, -70.0, 30.0, -50.0]);
        approx(frame.correct_x(30.0), 25.0); // 20 + 0.5·(30−20)
    }

    #[test]
    fn pin_font_size_scales_with_the_framed_span() {
        // Sizing off the span (not a fixed user-unit) keeps the pin's apparent size
        // constant under `meet`: a tiny island's floored window and a wide mainland
        // each get a pin proportional to their own window.
        let tiny = Frame::for_bbox([166.91, 0.49, 166.96, 0.55]); // floored to MIN_WINDOW_DEG
        approx(tiny.pin_font_size(), MIN_WINDOW_DEG * PIN_SIZE_FRACTION);
        let wide = Frame::for_bbox([-30.0, -2.0, 30.0, 2.0]);
        approx(
            wide.pin_font_size(),
            60.0 * FRAME_PADDING * PIN_SIZE_FRACTION,
        );
    }

    #[test]
    fn real_deck_frames_france_on_the_european_mainland() {
        // Mainland bbox (asset uses the largest polygon), so the window sits over
        // continental France — a modest span, not blown out across the Atlantic to
        // French Guiana.
        let deck = Deck::load().unwrap();
        let frame = frame_for(&deck, "FRA");
        let [min_x, min_y, width, height] = parse_view_box(&frame.view_box());
        assert!(
            width < 40.0,
            "framed on Europe, not the Atlantic (span {width})"
        );
        let (cx, cy) = (min_x + width / 2.0, min_y + height / 2.0);
        assert!(
            (0.0..4.0).contains(&cx),
            "centre longitude in France ({cx})"
        );
        assert!(
            (-52.0..-42.0).contains(&cy),
            "centre latitude in France ({cy})"
        );
    }

    #[test]
    fn real_deck_gives_a_small_island_a_sane_window() {
        // Niue: a tiny island floored to the minimum window, not a pinpoint.
        let deck = Deck::load().unwrap();
        let frame = frame_for(&deck, "NIU");
        let [_, _, w, _] = parse_view_box(&frame.view_box());
        approx(w, MIN_WINDOW_DEG);
    }

    #[test]
    fn real_deck_corrects_a_high_latitude_entity() {
        // Iceland (~65°N): the x-scale is well below 1, so the render is
        // horizontally corrected rather than stretched.
        let deck = Deck::load().unwrap();
        let scale = frame_for(&deck, "ISL").scale_x();
        assert!(
            scale < 0.5,
            "high-latitude x-scale should be < 0.5 (got {scale})"
        );
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
}
