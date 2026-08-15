# 22 — Back-to-home button broken

Status: needs-triage

**What's wrong:** The "Back to home" button doesn't return to the Home screen.
It appears on the Done screen (issue 09, `Route::Done`) and/or Settings.

## Task

- Reproduce and identify the cause (routing / navigation wiring).
- Fix so the button navigates back to `Route::Home`.
- Eyeball on the simulator; keep the standard gate green (`cargo test`,
  `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`,
  `cargo check --target aarch64-apple-ios`).
