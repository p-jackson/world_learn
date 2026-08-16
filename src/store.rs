//! Durable review-state store.
//!
//! One sparse serde-JSON file the app reads at launch and rewrites after every
//! grade. Loading a missing file yields defaults; saving is atomic (temp file in
//! the same directory → `rename`) so a mid-session quit never corrupts or loses
//! data. The iOS sandbox path is resolved separately (see [`app_support_dir`]);
//! the store itself is platform-agnostic and takes a directory, so the
//! serde/atomic-write logic is unit-testable off-device.

// The app shell now wires `Store` into launch, grading, and Settings, but a few
// reserved bits of its surface (e.g. `Store::path`, used only in tests) still
// read as dead code to a plain `cargo build`. Allowed module-wide rather than per
// item.
#![expect(
    dead_code,
    reason = "a few reserved items (e.g. Store::path) are only used in tests"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use jiff::civil::Date;
use serde::{Deserialize, Serialize};

/// Current on-disk schema. Migration path is load-all/save-all; only v1 exists.
pub const SCHEMA_VERSION: u32 = 1;

/// Default: new Cards introduced per day.
pub const DEFAULT_NEW_CARDS_PER_DAY: u32 = 10;

/// File name inside the store directory.
const STATE_FILE: &str = "review_state.json";

/// Sibling temp file `save` writes before renaming over [`STATE_FILE`].
const TEMP_FILE: &str = "review_state.json.tmp";

/// The whole persisted document. `cards` is sparse — it holds only
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

/// The only interactive setting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub new_cards_per_day: u32,
}

/// One seen Card's persisted state. FSRS memory fields are inline,
/// not nested; the app owns the three local-date fields. Dates are day-precision
/// local dates ([`jiff::civil::Date`]), which serialize as `YYYY-MM-DD` — the
/// type validates the calendar date at the serde boundary, so a malformed date
/// is a load error, not a landmine for the scheduling core.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardRecord {
    /// FSRS `MemoryState.stability`.
    pub stability: f32,
    /// FSRS `MemoryState.difficulty`.
    pub difficulty: f32,
    /// `≤ today` ⇒ due.
    pub due: Date,
    /// Date of the last grade.
    pub last_review: Date,
    /// Set once on first grade; drives the daily-new cap.
    pub introduced_on: Date,
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
///
/// A `Store` is just a resolved file path, so it is cheap to `Clone` and compares
/// by that path — [`crate::session::Session`] owns one by value (rather than a
/// borrow) so it can live in a Dioxus `Signal`, and it is passed as a UI prop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Store {
    path: PathBuf,
}

impl Store {
    /// Open the store in the iOS Application Support directory, creating it on
    /// first launch.
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

    /// Persist a new value for the daily new-Card cap (the only interactive
    /// setting), preserving every Card record. Loads the current
    /// document, replaces the one field, and saves atomically; returns the
    /// persisted state so the caller can reflect it without a reload.
    pub fn set_new_cards_per_day(&self, new_cards_per_day: u32) -> Result<ReviewState> {
        let mut state = self
            .load()
            .context("loading store to update new-cards-per-day")?;
        state.settings.new_cards_per_day = new_cards_per_day;
        self.save(&state)
            .context("persisting updated new-cards-per-day")?;
        Ok(state)
    }

    /// Erase all persisted state: remove [`STATE_FILE`] so the next
    /// [`Self::load`] yields [`ReviewState::default`] (the first-launch state).
    /// A missing file is already clear — not an error.
    pub fn clear(&self) -> Result<()> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => {
                Err(e).with_context(|| format!("clearing store file {}", self.path.display()))
            }
        }
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
/// first launch.
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
    use jiff::civil::date;

    fn sample_card() -> CardRecord {
        CardRecord {
            stability: 3.17,
            difficulty: 5.20,
            due: date(2026, 8, 16),
            last_review: date(2026, 8, 14),
            introduced_on: date(2026, 8, 10),
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
    fn parses_the_persisted_json_shape() {
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
    fn set_new_cards_per_day_persists_and_preserves_cards() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open_in(dir.path()).unwrap();
        let mut initial = ReviewState::default();
        initial.cards.insert("FRA".to_string(), sample_card());
        store.save(&initial).unwrap();

        let returned = store.set_new_cards_per_day(25).unwrap();
        assert_eq!(returned.settings.new_cards_per_day, 25);

        let reloaded = store.load().unwrap();
        assert_eq!(reloaded.settings.new_cards_per_day, 25);
        assert_eq!(
            reloaded.cards.get("FRA"),
            Some(&sample_card()),
            "records are preserved across a settings write"
        );
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
    fn clear_removes_state_and_reloads_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open_in(dir.path()).unwrap();
        let mut state = ReviewState::default();
        state.settings.new_cards_per_day = 25;
        state.cards.insert("FRA".to_string(), sample_card());
        store.save(&state).unwrap();

        store.clear().unwrap();

        assert!(!store.path().exists(), "clear removes the state file");
        // A missing file is not an error to clear again, and reloads as defaults.
        store.clear().unwrap();
        assert_eq!(store.load().unwrap(), ReviewState::default());
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
    fn malformed_date_is_a_load_error_not_a_silent_default() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open_in(dir.path()).unwrap();
        fs::write(
            store.path(),
            r#"{
                "schema_version": 1,
                "settings": { "new_cards_per_day": 10 },
                "cards": {
                    "FRA": {
                        "stability": 3.17,
                        "difficulty": 5.20,
                        "due": "2026-13-40",
                        "last_review": "2026-08-14",
                        "introduced_on": "2026-08-10"
                    }
                }
            }"#,
        )
        .unwrap();

        let err = store.load().unwrap_err();
        assert!(err.downcast_ref::<serde_json::Error>().is_some());
    }

    /// Tripwire for the store's one unstated invariant: every Card is keyed by
    /// its `ADM0_A3` code — the same string the geometry asset
    /// (`assets/geometry.json`) uses for its top-level keys. The serde types
    /// accept any `String`, so nothing else notices if a bump to the geometry
    /// data dependency changes the code format (2-letter ISO, lowercase,
    /// disputed-territory suffixes). Such a change would key freshly-built
    /// Cards differently from every record already written to a device,
    /// silently orphaning that history. Read the shipped asset and fail loudly
    /// if any key leaves the three-ASCII-uppercase-letter shape the store and
    /// scheduler assume.
    const GEOMETRY_ASSET: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/geometry.json");

    /// The set of `ADM0_A3` codes the shipped geometry asset ships with — the
    /// exact set of keys the store may ever see. Read live so the tests below
    /// guard the real asset, not a stale copy.
    fn geometry_asset_codes() -> std::collections::BTreeSet<String> {
        let bytes = fs::read(GEOMETRY_ASSET)
            .unwrap_or_else(|e| panic!("reading geometry asset {GEOMETRY_ASSET}: {e}"));
        let doc: serde_json::Value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|e| panic!("parsing geometry asset {GEOMETRY_ASSET}: {e}"));
        let keys = doc
            .as_object()
            .unwrap_or_else(|| panic!("geometry asset {GEOMETRY_ASSET} is not a JSON object"));
        keys.keys().cloned().collect()
    }

    /// [`GEOMETRY_ASSET`]'s tripwire: every key holds the three-ASCII-uppercase-letter shape.
    #[test]
    fn geometry_asset_keys_match_the_adm0_a3_shape() {
        let codes = geometry_asset_codes();
        assert!(
            !codes.is_empty(),
            "geometry asset {GEOMETRY_ASSET} has no country keys"
        );

        let malformed: Vec<&String> = codes
            .iter()
            .filter(|k| k.len() != 3 || !k.bytes().all(|b| b.is_ascii_uppercase()))
            .collect();
        assert!(
            malformed.is_empty(),
            "geometry keys must be ADM0_A3 codes (three ASCII uppercase letters); the store \
             keys review records by these, so a format change silently orphans on-device \
             history. Offending keys: {malformed:?}"
        );
    }

    /// Companion tripwire for *set*-membership drift, which the shape check
    /// above can't see: a geo-data bump that adds, drops, or renames a country
    /// keeps every code a valid `ADM0_A3` string yet still orphans any on-device
    /// history keyed by a removed or renamed code. Pin the whole set to a
    /// committed snapshot (`src/adm0_a3.snapshot`, one code per line) so any
    /// drift shows up as a reviewable +added / -removed diff and forces a
    /// deliberate decision. After deciding how existing data migrates,
    /// regenerate the snapshot with `UPDATE_ADM0_A3_SNAPSHOT=1 cargo test`.
    #[test]
    fn geometry_asset_key_set_matches_snapshot() {
        const SNAPSHOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/adm0_a3.snapshot");

        let current = geometry_asset_codes();

        if std::env::var_os("UPDATE_ADM0_A3_SNAPSHOT").is_some() {
            let mut contents: String = current.iter().flat_map(|c| [c.as_str(), "\n"]).collect();
            contents.pop(); // the trailing separator; a final newline is re-added
            contents.push('\n');
            fs::write(SNAPSHOT, contents)
                .unwrap_or_else(|e| panic!("writing snapshot {SNAPSHOT}: {e}"));
            return;
        }

        let snapshot: std::collections::BTreeSet<String> = fs::read_to_string(SNAPSHOT)
            .unwrap_or_else(|e| panic!("reading snapshot {SNAPSHOT}: {e}"))
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect();

        let added: Vec<&String> = current.difference(&snapshot).collect();
        let removed: Vec<&String> = snapshot.difference(&current).collect();
        assert!(
            added.is_empty() && removed.is_empty(),
            "the geometry ADM0_A3 set drifted from src/adm0_a3.snapshot. Added: {added:?}. \
             Removed: {removed:?}. A removed or renamed code orphans on-device review history \
             keyed by it — decide how that data migrates, then regenerate the snapshot with \
             UPDATE_ADM0_A3_SNAPSHOT=1 cargo test."
        );
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
