//! Durable review-state store (spec §5.1, §6).
//!
//! One sparse serde-JSON file the app reads at launch and rewrites after every
//! grade. Loading a missing file yields defaults; saving is atomic (temp file in
//! the same directory → `rename`) so a mid-session quit never corrupts or loses
//! data. The iOS sandbox path is resolved separately (see [`app_support_dir`]);
//! the store itself is platform-agnostic and takes a directory, so the
//! serde/atomic-write logic is unit-testable off-device.
//!
//! This module lands one issue ahead of its first caller: the app shell wires
//! `Store` into launch/grade later, so its public surface reads as dead code to
//! a plain `cargo build` until then. Allowed module-wide rather than per item;
//! drop the allow once the shell consumes it.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Current on-disk schema. Migration path is load-all/save-all; only v1 exists.
pub const SCHEMA_VERSION: u32 = 1;

/// Spec §4.4 default: new Cards introduced per day.
pub const DEFAULT_NEW_CARDS_PER_DAY: u32 = 10;

/// File name inside the store directory.
const STATE_FILE: &str = "review_state.json";

/// Sibling temp file `save` writes before renaming over [`STATE_FILE`].
const TEMP_FILE: &str = "review_state.json.tmp";

/// The whole persisted document (spec §5.1). `cards` is sparse — it holds only
/// Cards that have left "new"; an absent `ADM0_A3` key is a not-yet-introduced
/// Card. Lifecycle status is derived at runtime, never stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewState {
    pub schema_version: u32,
    pub settings: Settings,
    /// Keyed by `ADM0_A3`. `BTreeMap` keeps serialization order stable so the
    /// file diffs cleanly and round-trips deterministically.
    pub cards: BTreeMap<String, CardRecord>,
}

/// The only interactive setting (spec §4.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub new_cards_per_day: u32,
}

/// One seen Card's persisted state (spec §5.1). FSRS memory fields are inline,
/// not nested; the app owns the three local-date fields. Dates are
/// `YYYY-MM-DD` local-date strings (day precision) — parsing/arithmetic lives in
/// the scheduling core, not here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardRecord {
    /// FSRS `MemoryState.stability`.
    pub stability: f32,
    /// FSRS `MemoryState.difficulty`.
    pub difficulty: f32,
    /// `YYYY-MM-DD`; `≤ today` ⇒ due.
    pub due: String,
    /// `YYYY-MM-DD` of the last grade.
    pub last_review: String,
    /// `YYYY-MM-DD`, set once on first grade; drives the daily-new cap.
    pub introduced_on: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            new_cards_per_day: DEFAULT_NEW_CARDS_PER_DAY,
        }
    }
}

impl Default for ReviewState {
    /// First-launch state: current schema, default settings, no seen Cards.
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            settings: Settings::default(),
            cards: BTreeMap::new(),
        }
    }
}

/// The durable store: a directory holding [`STATE_FILE`]. Construct with
/// [`Store::open_default`] on device, or [`Store::open_in`] with an injected
/// directory in tests.
pub struct Store {
    path: PathBuf,
}

impl Store {
    /// Open the store in the iOS Application Support directory (spec §6),
    /// creating it on first launch. Wired into app launch by a later issue.
    pub fn open_default() -> Result<Self> {
        Self::open_in(app_support_dir()?)
    }

    /// Open the store in `dir`, creating the directory if needed. The seam the
    /// off-device tests inject a temp directory through.
    pub fn open_in(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)
            .with_context(|| format!("creating store directory {}", dir.display()))?;
        Ok(Self {
            path: dir.join(STATE_FILE),
        })
    }

    /// Path of the JSON file this store reads and writes.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load persisted state. A missing file yields [`ReviewState::default`]
    /// (first launch); a present-but-invalid file is an error, never silently
    /// discarded.
    pub fn load(&self) -> Result<ReviewState> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(ReviewState::default()),
            Err(e) => {
                return Err(e)
                    .with_context(|| format!("reading store file {}", self.path.display()));
            }
        };
        let state: ReviewState = serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing store file {}", self.path.display()))?;
        anyhow::ensure!(
            state.schema_version <= SCHEMA_VERSION,
            "unsupported schema version {}; this build supports up to {SCHEMA_VERSION}",
            state.schema_version
        );
        Ok(state)
    }

    /// Persist `state` atomically: serialize, write a sibling temp file, then
    /// `rename` it over the real file. `rename` on the same filesystem is
    /// atomic, so a crash mid-write leaves the previous file intact.
    pub fn save(&self, state: &ReviewState) -> Result<()> {
        let json = serde_json::to_vec_pretty(state).context("serializing review state")?;
        let tmp = self.path.with_file_name(TEMP_FILE);
        fs::write(&tmp, &json)
            .with_context(|| format!("writing temp store file {}", tmp.display()))?;
        fs::rename(&tmp, &self.path)
            .with_context(|| format!("renaming {} over {}", tmp.display(), self.path.display()))?;
        Ok(())
    }
}

/// Resolve the iOS Application Support directory for this app, creating it on
/// first launch (spec §6).
///
/// This is the one unavoidable platform-specific piece: the `dirs` /
/// `directories` / `dioxus-sdk` crates resolve iOS wrong (they fall back to
/// Linux/XDG), so we call `NSSearchPathForDirectoriesInDomains` directly.
/// Application Support (app-managed, backed up, hidden) — not Caches (purgeable)
/// — holds review history.
#[cfg(target_vendor = "apple")]
pub fn app_support_dir() -> Result<PathBuf> {
    use objc2_foundation::{
        NSSearchPathDirectory, NSSearchPathDomainMask, NSSearchPathForDirectoriesInDomains,
    };

    let paths = NSSearchPathForDirectoriesInDomains(
        NSSearchPathDirectory::ApplicationSupportDirectory,
        NSSearchPathDomainMask::UserDomainMask,
        true,
    );
    let base = paths
        .firstObject()
        .context("no Application Support directory in the user domain search path")?;
    let dir = PathBuf::from(base.to_string()).join(APP_SUBDIR);
    fs::create_dir_all(&dir)
        .with_context(|| format!("creating app support directory {}", dir.display()))?;
    Ok(dir)
}

/// Non-Apple builds (host CI, `cargo test` on Linux) have no iOS sandbox. The
/// store's serde/atomic-write logic is exercised through [`Store::open_in`]
/// with an injected directory, so this only needs to keep the crate compiling.
#[cfg(not(target_vendor = "apple"))]
pub fn app_support_dir() -> Result<PathBuf> {
    anyhow::bail!("Application Support directory is only resolvable on Apple platforms")
}

/// App-owned subdirectory under Application Support.
#[cfg(target_vendor = "apple")]
const APP_SUBDIR: &str = "WorldLearn";

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_card() -> CardRecord {
        CardRecord {
            stability: 3.17,
            difficulty: 5.20,
            due: "2026-08-16".to_string(),
            last_review: "2026-08-14".to_string(),
            introduced_on: "2026-08-10".to_string(),
        }
    }

    #[test]
    fn first_launch_yields_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open_in(dir.path()).unwrap();

        let state = store.load().unwrap();

        assert_eq!(state.schema_version, SCHEMA_VERSION);
        assert_eq!(state.settings.new_cards_per_day, DEFAULT_NEW_CARDS_PER_DAY);
        assert!(state.cards.is_empty());
        // Loading absent state must not create the file.
        assert!(!store.path().exists());
    }

    #[test]
    fn open_in_creates_missing_directory() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a/b/c");
        let store = Store::open_in(&nested).unwrap();
        assert!(nested.is_dir());
        assert_eq!(store.path().parent().unwrap(), nested);
    }

    #[test]
    fn round_trip_load_mutate_save_reload() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open_in(dir.path()).unwrap();

        let mut state = store.load().unwrap();
        state.settings.new_cards_per_day = 25;
        state.cards.insert("FRA".to_string(), sample_card());
        store.save(&state).unwrap();

        let reloaded = store.load().unwrap();
        assert_eq!(reloaded, state);
    }

    #[test]
    fn parses_the_spec_5_1_shape() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open_in(dir.path()).unwrap();
        let json = r#"{
            "schema_version": 1,
            "settings": { "new_cards_per_day": 10 },
            "cards": {
                "FRA": {
                    "stability": 3.17,
                    "difficulty": 5.20,
                    "due": "2026-08-16",
                    "last_review": "2026-08-14",
                    "introduced_on": "2026-08-10"
                }
            }
        }"#;
        fs::write(store.path(), json).unwrap();

        let state = store.load().unwrap();

        assert_eq!(state.schema_version, 1);
        assert_eq!(state.settings.new_cards_per_day, 10);
        assert_eq!(state.cards.len(), 1);
        assert_eq!(state.cards["FRA"], sample_card());
    }

    #[test]
    fn save_is_atomic_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open_in(dir.path()).unwrap();

        store.save(&ReviewState::default()).unwrap();

        assert!(store.path().exists());
        assert!(!dir.path().join(TEMP_FILE).exists());
    }

    #[test]
    fn overwriting_save_preserves_prior_file_on_reload() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open_in(dir.path()).unwrap();

        let mut first = ReviewState::default();
        first.cards.insert("USA".to_string(), sample_card());
        store.save(&first).unwrap();

        let mut second = store.load().unwrap();
        second.cards.insert("DEU".to_string(), sample_card());
        store.save(&second).unwrap();

        let reloaded = store.load().unwrap();
        assert_eq!(reloaded.cards.len(), 2);
        assert!(reloaded.cards.contains_key("USA"));
        assert!(reloaded.cards.contains_key("DEU"));
    }

    #[test]
    fn invalid_json_is_an_error_not_a_silent_default() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open_in(dir.path()).unwrap();
        fs::write(store.path(), "{ not valid json").unwrap();

        let err = store.load().unwrap_err();
        // anyhow preserves the underlying serde_json error as the source.
        assert!(err.downcast_ref::<serde_json::Error>().is_some());
    }

    #[test]
    fn newer_schema_version_fails_loudly() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open_in(dir.path()).unwrap();
        fs::write(
            store.path(),
            r#"{ "schema_version": 2, "settings": { "new_cards_per_day": 10 }, "cards": {} }"#,
        )
        .unwrap();

        let err = store.load().unwrap_err();
        assert!(err.to_string().contains("unsupported schema version 2"));
    }
}
