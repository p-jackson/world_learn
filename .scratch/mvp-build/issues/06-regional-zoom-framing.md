# 06 — Regional-zoom framing

**What to build:** Turn the world map into the Review's regional zoom. For the highlighted Entity, the viewport frames its mainland with padding so neighbours show as location cues, tiny islands aren't over-zoomed, and high-latitude horizontal stretch is corrected. After this ticket, showing a Card presents the medium regional framing the product calls for — a pure `viewBox` swap per Card, no re-projection.

Source spec: `.scratch/mvp-spec/spec.md` §3.2, §4.2.

**Blocked by:** 05 (SVG map component).

- [ ] `viewBox` = the entity's **mainland** bbox (from the asset) × ~3.4 padding, square, centred
- [ ] Minimum span enforced (~6° → ~20° window) so tiny/island entities aren't over-zoomed
- [ ] Per-Card `cos(lat_center)` horizontal correction applied via a group transform (latitude midpoint of the frame)
- [ ] Zoom is a pure per-Card `viewBox` swap on the single rendered map — no per-card re-projection, no re-render of paths
- [ ] Demoable: France frames on the European mainland (not the Atlantic out to French Guiana); a small island (e.g. Nauru/Niue) shows a sane window, not a pinpoint; a high-latitude entity is not horizontally stretched
