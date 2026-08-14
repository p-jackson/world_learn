# Local persistence approach

Type: research
Status: resolved

## Question

How should the app persist FSRS review-state and user settings locally on iOS
in a Dioxus 0.7 app?

Compare the viable options — a JSON/RON file in the iOS app data directory,
SQLite (`rusqlite`), and any Dioxus-native storage crate (e.g. `dioxus-sdk`
storage) — on: iOS compatibility (the app runs natively in Rust behind a
WKWebView), ease of use, migration/robustness, and how well each holds ~250
per-card FSRS records plus settings.

Recommend one, with a fallback. Note where on-device the data lives and how the
app locates that path on iOS.

## Answer

**Decision: a single serde JSON (or RON) file in `Library/Application Support/<app>/`.**
SQLite (`rusqlite`, `features=["bundled"]`) is the fallback if the dataset ever
grows or formal migrations are wanted.

Key facts for the spec:

- **Dioxus mobile runs Rust natively** (it drives, but does not run inside, the
  WKWebView). So this is a native-filesystem problem, *not* browser storage —
  `localStorage`/IndexedDB are irrelevant.
- **~250 FSRS records + settings is tens of KB.** Hold it in a `Signal`/struct,
  persist on mutation. Write **atomically** (temp file in same dir → `rename`).
  Include a `schema_version: u32` field; migrate by load-all/save-all.
- **The one unavoidable platform-specific piece**: obtaining the iOS sandbox
  path. `dirs`/`directories`/`dioxus-sdk` all resolve iOS *wrong* (they fall
  back to Linux/XDG paths). Write a ~15-line `objc2` + `objc2-foundation` helper
  calling `NSSearchPathForDirectoriesInDomains(NSApplicationSupportDirectory,
  NSUserDomainMask, true)`; `create_dir_all` the dir on first launch. Dioxus 0.7
  ships no helper for this.
- Use **Application Support** (app-managed, backed up, hidden) — not Caches
  (purgeable) for review history.
- SQLite fallback builds fine under the iOS Xcode toolchain (native Apple
  compile, not the problematic Linux→Apple cross-compile); still needs the same
  objc2 path helper. It's over-engineered for 250 records.
