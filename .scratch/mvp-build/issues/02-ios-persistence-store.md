# 02 — iOS persistence store

Status: done

**What to build:** The durable store the app reads at launch and writes after every grade. It resolves the correct iOS sandbox directory, loads the review-state JSON on start, and saves it back atomically so a mid-session quit never corrupts or loses data. Everything downstream that persists a Card's schedule goes through this.

Source spec: `.scratch/mvp-spec/spec.md` §5.1, §6.

**Blocked by:** None — can start immediately.

- [x] ~15-line `objc2` + `objc2-foundation` helper resolving `NSApplicationSupportDirectory` / `NSUserDomainMask` (the `dirs`/`directories`/`dioxus-sdk` crates resolve iOS wrong — do not use them); `create_dir_all` on first launch
- [x] Uses Application Support (backed up, hidden), not Caches
- [x] serde load of the §5.1 shape: `schema_version`, `settings.new_cards_per_day`, sparse `cards` map keyed by `ADM0_A3` with flat inline memory fields (`stability`, `difficulty` f32; `due`/`last_review`/`introduced_on` as `YYYY-MM-DD` local-date strings)
- [x] First launch (no file) yields sensible defaults (`new_cards_per_day: 10`, empty `cards`)
- [x] Save is **atomic**: write temp file in the same dir → `rename`
- [x] `schema_version: u32` present; migration path is load-all/save-all (only v1 needed now)
- [x] Round-trip verified (load → mutate → save → reload equals mutation); off-device unit coverage for the serde/atomic-write logic where the iOS path can be injected
