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
//! swap plus a group `transform` on each rendered copy — no re-projection, no path
//! re-render (issue 06, spec §3.2/§4.2). The map is drawn three times, the proper
//! map flanked by a `∓360°` wrap copy either side, so a frame near the ±180°
//! antimeridian never shows the map's edge (issue 10). With no highlight the map
//! falls back to [`WORLD_VIEW_BOX`].

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

/// Context added around the framing bbox, in projected degrees, on **each** side
/// (spec §4.2). Additive rather than a multiplier so the country's share of the
/// window grows with its size: a giant like Russia fills most of the frame while
/// still showing its neighbours, and a speck like Anguilla floats in a wide
/// context window — the size-dependent zoom the multiplier couldn't give (a
/// constant multiplier framed every country to the same fraction, blowing huge
/// countries out to a near-global squish, issue 10). Pinned so Japan (framing
/// span ≈ 8.8°) lands at ≈ 30° total, the reference that reads well. The 2×
/// margin also floors tiny entities at ≈ 21° with no separate minimum.
const CONTEXT_MARGIN_DEG: f64 = 10.6;
/// Floor on the cos(lat) x-scale — a degenerate guard for near-polar frame
/// centres so the transform never collapses the map to a vertical line. No
/// inhabited Entity's mainland reaches it.
const MIN_COS_SCALE: f64 = 0.1;
/// Reveal-pin glyph size as a fraction of the framed window's span. The square
/// `viewBox` always maps to the same rendered box under `meet`, so a span-relative
/// size keeps the pin's apparent size roughly constant across Cards.
const PIN_SIZE_FRACTION: f64 = 0.06;

/// The world is drawn three times — the map proper plus a wrap copy `∓360°`
/// either side — so a frame straddling the ±180° antimeridian never shows the
/// bare edge of the map (issue 10). `(stable key, raw-longitude shift)`; the keys
/// keep the three groups' node identity fixed across Card changes (Dioxus #2274).
/// A single frame spans < 180°, so at most one wrap copy is ever on-screen; the
/// other two sit outside the `viewBox` at no visible cost.
const WRAP_COPIES: [(&str, f64); 3] = [("wrap-w", -360.0), ("wrap-main", 0.0), ("wrap-e", 360.0)];

/// The regional-zoom framing for one Entity: a square `viewBox` plus a horizontal
/// cos(lat) correction, both derived from the Entity's **framing** bbox — the
/// asset's `bbox`, its mainland (or an archipelago's major islands), so France
/// frames on the European mainland, not out across the Atlantic to French Guiana
/// (spec §4.2).
///
/// A Card's zoom is applying this Frame to the rendered map: [`Self::view_box`]
/// swaps the `viewBox`, [`Self::transform_shifted`] sets each copy's group
/// `transform`. Nothing re-projects and no `<path>` changes.
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
    /// The window is square and framed around the bbox with a fixed [context
    /// margin](CONTEXT_MARGIN_DEG) on each side. The longitude extent is
    /// compressed by cos(lat) before choosing the square side, so the window
    /// frames the region's real-world footprint (the same correction the group
    /// transform then applies to the geometry) rather than its
    /// equirectangular-stretched one.
    pub fn for_bbox(bbox: [f64; 4]) -> Self {
        let [min_x, min_y, max_x, max_y] = bbox;
        let center_x = f64::midpoint(min_x, max_x);
        let center_y = f64::midpoint(min_y, max_y);
        // y = −lat, so the frame's midpoint latitude is −center_y.
        let scale_x = (-center_y).to_radians().cos().max(MIN_COS_SCALE);
        let corrected_width = (max_x - min_x) * scale_x;
        let height = max_y - min_y;
        let span = 2.0f64.mul_add(CONTEXT_MARGIN_DEG, corrected_width.max(height));
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

    /// The group `transform` for the map proper — [`Self::transform_shifted`] with
    /// no wrap offset. Test-only shorthand; the component builds every copy
    /// (including the main one) through `transform_shifted`.
    #[cfg(test)]
    pub fn transform(&self) -> String {
        self.transform_shifted(0.0)
    }

    /// The group `transform` for a copy of the world shifted `shift` degrees in raw
    /// longitude: scale x by cos(lat) about the frame centre so high-latitude
    /// entities aren't horizontally stretched (§3.2), then offset by the wrap
    /// period. `translate(cx) scale(k) translate(-cx)` collapsed to one translate +
    /// scale (`x ↦ k·(x + shift) + cx·(1−k)`), leaving y untouched. A frame near the
    /// ±180° antimeridian would otherwise show the bare edge of the map with an
    /// ocean void beyond it; drawing the same paths at `shift = ±360` places wrap
    /// copies either side so context is continuous across the seam (issue 10).
    pub fn transform_shifted(&self, shift: f64) -> String {
        let tx = self
            .scale_x
            .mul_add(shift, self.center_x * (1.0 - self.scale_x));
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

/// The group `transform` for one wrap copy at raw-longitude `shift`. A framed
/// Card carries the cos-correction into every copy ([`Frame::transform_shifted`]);
/// with no highlight the map is the whole world at natural scale, so a copy is a
/// plain `∓360°` translate and the map proper (`shift == 0`) needs no transform.
fn wrap_transform(frame: Option<&Frame>, shift: f64) -> Option<String> {
    match frame {
        Some(f) => Some(f.transform_shifted(shift)),
        None if shift != 0.0 => Some(format!("translate({} 0)", svg_num(shift))),
        None => None,
    }
}

/// The world map with one Entity highlighted. Fills its container (`h-full
/// w-full`); the caller sizes it (full-bleed on the Review screen).
///
/// Renders all Deck paths in fixed intro order, three times over — the map plus a
/// wrap copy either side ([`WRAP_COPIES`]) so an antimeridian frame never shows
/// the map's edge — and only the highlighted Card's `fill` differs. The boundary
/// stroke is uniform and non-scaling, applied to
/// every child `<path>` via one Tailwind child-variant on the `<svg>`, so it
/// stays visible on every Entity in both states independent of `fill` and of the
/// group's cos-correction scale.
///
/// The highlighted Card's [`Frame`] drives the regional zoom: its `viewBox` swap
/// and each copy's group `transform`. With no valid highlight the map shows the
/// whole world ([`WORLD_VIEW_BOX`], wrap copies at a plain `∓360°`). All are
/// attributes on fixed nodes, so a Card change re-frames without touching the path
/// tree (Dioxus #2274).
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
            for (key, shift) in WRAP_COPIES {
                g {
                    key: "{key}",
                    transform: wrap_transform(frame.as_ref(), shift),
                    for card in deck.cards() {
                        path {
                            key: "{card.code}",
                            d: "{card.entity.d}",
                            fill: fill_for(&card.code, &highlighted),
                        }
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

    /// The two context margins a frame adds around the bbox — a local so the
    /// assertions stay readable and dodge clippy's `mul_add` rewrite of `a*b + c`.
    const MARGINS: f64 = 2.0 * CONTEXT_MARGIN_DEG;

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
        // centred and square, the larger dimension plus a context margin each side.
        let frame = Frame::for_bbox([-5.0, -5.0, 5.0, 5.0]);
        let [min_x, min_y, width, height] = parse_view_box(&frame.view_box());
        approx(width, 10.0 + MARGINS); // 10° span + margins
        approx(height, 10.0 + MARGINS);
        approx(min_x + width / 2.0, 0.0); // centred on the bbox
        approx(min_y + height / 2.0, 0.0);
    }

    #[test]
    fn frame_stays_square_for_a_wide_bbox() {
        // A wide, short mainland still frames square, sized off the larger side.
        let frame = Frame::for_bbox([-30.0, -2.0, 30.0, 2.0]);
        let [_, _, w, h] = parse_view_box(&frame.view_box());
        approx(w, h); // viewBox width equals height
        approx(w, 60.0 + MARGINS); // sized off the 60° width
    }

    #[test]
    fn frame_grows_the_country_share_of_the_window_with_its_size() {
        // The additive margin (not a multiplier) means a bigger country fills more
        // of its frame: the span/size ratio shrinks as the country grows.
        let small = Frame::for_bbox([-4.0, -4.0, 4.0, 4.0]); // 8° span
        let large = Frame::for_bbox([-40.0, -40.0, 40.0, 40.0]); // 80° span
        let share = |f: &Frame, size: f64| {
            let [_, _, w, _] = parse_view_box(&f.view_box());
            size / w
        };
        assert!(
            share(&large, 80.0) > share(&small, 8.0),
            "larger country should fill a bigger share of its window"
        );
    }

    #[test]
    fn frame_floors_tiny_entities_at_the_context_margin() {
        // A pinpoint bbox: with no extent, the window is exactly the two context
        // margins — a sane wide window, not a zoomed-in speck.
        let frame = Frame::for_bbox([166.93, 0.52, 166.93, 0.52]);
        let [_, _, w, h] = parse_view_box(&frame.view_box());
        approx(w, MARGINS);
        approx(h, MARGINS);
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
        approx(w, 20.0 + MARGINS);
    }

    #[test]
    fn wrap_copy_shifts_screen_x_by_the_scaled_period() {
        // High-latitude frame (k = cos60° = 0.5, base tx = 10): a wrap copy at
        // ±360° raw longitude lands 360·k = 180 screen units either side, so the
        // seam is continuous under the same cos-correction the map carries.
        let frame = Frame::for_bbox([10.0, -70.0, 30.0, -50.0]);
        assert_eq!(
            frame.transform_shifted(360.0),
            "translate(190 0) scale(0.5 1)"
        );
        assert_eq!(
            frame.transform_shifted(-360.0),
            "translate(-170 0) scale(0.5 1)"
        );
        // Offset 0 is exactly the un-shifted transform.
        assert_eq!(frame.transform_shifted(0.0), frame.transform());
    }

    #[test]
    fn wrap_transform_of_the_whole_world_is_a_plain_period_translate() {
        // No highlight → whole world at natural scale: the map proper needs no
        // transform, and each wrap copy is just a ∓360° translate.
        assert_eq!(wrap_transform(None, 0.0), None);
        assert_eq!(
            wrap_transform(None, -360.0),
            Some("translate(-360 0)".into())
        );
        assert_eq!(wrap_transform(None, 360.0), Some("translate(360 0)".into()));
    }

    #[test]
    fn wrap_copy_shifts_by_the_full_period_at_identity_scale() {
        // Equator frame (k = 1): the wrap copy is a plain ±360° translate.
        let frame = Frame::for_bbox([-5.0, -5.0, 5.0, 5.0]);
        assert_eq!(
            frame.transform_shifted(360.0),
            "translate(360 0) scale(1 1)"
        );
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
        let tiny = Frame::for_bbox([166.93, 0.52, 166.93, 0.52]); // floored to the margins
        approx(tiny.pin_font_size(), MARGINS * PIN_SIZE_FRACTION);
        let wide = Frame::for_bbox([-30.0, -2.0, 30.0, 2.0]);
        approx(wide.pin_font_size(), (60.0 + MARGINS) * PIN_SIZE_FRACTION);
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
        // Niue: a tiny island framed to a wide context window (≈ the two margins),
        // not a pinpoint.
        let deck = Deck::load().unwrap();
        let frame = frame_for(&deck, "NIU");
        let [_, _, w, _] = parse_view_box(&frame.view_box());
        assert!(
            (MARGINS..MARGINS + 1.0).contains(&w),
            "small island frames to ≈ the context margins (span {w})"
        );
    }

    #[test]
    fn real_deck_frames_russia_regionally_not_globally() {
        // Issue 10: the old multiplier blew Russia's frame out to ≈ 264° (a
        // near-global horizontal squish). The additive margin keeps it a regional
        // window well under a third of the globe.
        let deck = Deck::load().unwrap();
        let [_, _, w, _] = parse_view_box(&frame_for(&deck, "RUS").view_box());
        assert!(
            w < 110.0,
            "Russia frames regionally, not globally (span {w})"
        );
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
