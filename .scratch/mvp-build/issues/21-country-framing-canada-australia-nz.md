# 21 — Country framing: Canada & Australia don't fit, NZ too tight

Status: needs-triage

**What's wrong:** Per-card map framing (`Frame::for_bbox` in `src/map.rs`) reads
wrong for several large/spread entities:

- **Canada** — doesn't fit the frame.
- **Australia** — doesn't fit the frame.
- **New Zealand** — should be more zoomed out.

Follows up the framing work in issues 06, 10, 14, 15. Issue 10's additive
`CONTEXT_MARGIN_DEG` reframe and antimeridian wrap fixed Russia/China/NZL/Fiji at
the time, but these cases still frame poorly.

## Task

- Reproduce Canada, Australia, NZ on the iOS simulator; confirm the mis-framing.
- Adjust the framing rule and/or per-entity bbox so each reads well — NZ wants a
  more zoomed-out frame specifically.
- Add/extend framing unit tests to lock the fixed cases.
- Eyeball on the simulator before closing; keep the standard gate green
  (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`,
  `cargo check --target aarch64-apple-ios`).
