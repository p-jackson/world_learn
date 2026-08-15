# 18 — Animate the reveal pin drop

Status: done

**What's wrong:** The reveal pin (📍) in `src/map.rs` toggles visibility via a
plain `opacity-0` ⇄ visible class swap (`Map` component, `pin` prop, around
`src/map.rs:278`). It just appears — no motion, doesn't read as a pin
"dropping" onto the map.

## Task

- Animate the pin's entrance on reveal (e.g. drop-in translate + fade, or
  scale-in) instead of a bare opacity toggle. Keep it CSS-driven (Tailwind
  transition/animate utilities or a small custom keyframe) rather than
  hand-rolled JS/timers, per the front⇄reveal "only visibility toggles"
  architecture note at `src/map.rs:230`.
- Respect `prefers-reduced-motion` if other animations in the app already do;
  otherwise note as a follow-up.

## Acceptance

- [ ] Pin visibly drops/animates in on reveal, on-device
- [ ] Front→reveal→front cycle still only toggles visibility, no structural
      SVG changes (per existing architecture constraint)
- [ ] Gate green: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`
