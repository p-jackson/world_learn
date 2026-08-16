/**
 * Typed store-state seeding — the deterministic-start seam.
 *
 * The web dev target persists to the `world_learn.review_state` `localStorage`
 * key (issue 24; `src/store.rs` `mod web`). Tests seed that key with a full
 * {@link ReviewState} before the app reads it, so a spec starts from a known
 * deck/schedule without grading through days of reviews. The types below mirror
 * `src/store.rs`'s serde types verbatim; a drift there fails these tests loudly
 * at the seam rather than silently mis-seeding.
 *
 * The app reads the real local clock (`session::today_local`), so due dates are
 * seeded relative to today via {@link isoDate}: a card due "yesterday" is due
 * whatever day the suite runs.
 */
import { type Locator, type Page } from '@playwright/test';

/** The single `localStorage` key the web backend reads/writes (`src/store.rs`). */
export const STORAGE_KEY = 'world_learn.review_state';

/** Current on-disk schema (`src/store.rs` `SCHEMA_VERSION`). */
export const SCHEMA_VERSION = 1;

/** The whole-world `viewBox` the map falls back to with no highlight (`map.rs`). */
export const WORLD_VIEW_BOX = '-180 -90 360 180';

/** Fill of the single highlighted country's `<path>` (`map.rs` `HIGHLIGHT_FILL`). */
export const HIGHLIGHT_FILL = '#f5b301';

/** One seen card's persisted state — mirrors `store::CardRecord`. */
export interface CardRecord {
  stability: number;
  difficulty: number;
  /** `YYYY-MM-DD`; `≤ today` ⇒ due. */
  due: string;
  last_review: string;
  introduced_on: string;
}

/** Mirrors `store::Settings`. */
export interface Settings {
  new_cards_per_day: number;
}

/** The whole persisted document — mirrors `store::ReviewState`. */
export interface ReviewState {
  schema_version: number;
  settings: Settings;
  /** Keyed by `ADM0_A3`. Sparse: an absent key is a not-yet-introduced card. */
  cards: Record<string, CardRecord>;
}

/** A local `YYYY-MM-DD` date `offsetDays` from today (negative = past). */
export function isoDate(offsetDays = 0): string {
  const d = new Date();
  d.setDate(d.getDate() + offsetDays);
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

/**
 * A record that is due now: `due` a day in the past so it is due regardless of
 * the run's local midnight, `introduced_on` far enough back that it never counts
 * against today's new-card allowance.
 */
export function dueRecord(): CardRecord {
  return {
    stability: 5,
    difficulty: 5,
    due: isoDate(-1),
    last_review: isoDate(-2),
    introduced_on: isoDate(-30),
  };
}

/** First-launch state: current schema, `new_cards_per_day` cap, no seen cards. */
export function emptyState(newCardsPerDay = 0): ReviewState {
  return {
    schema_version: SCHEMA_VERSION,
    settings: { new_cards_per_day: newCardsPerDay },
    cards: {},
  };
}

/**
 * A state where each `ADM0_A3` code is a seen card due now. With the default
 * `new_cards_per_day: 0` the session is exactly these cards — no new cards enter,
 * so the queue is deterministic and its order is the deck's intro order.
 */
export function dueState(codes: string[], newCardsPerDay = 0): ReviewState {
  const cards: Record<string, CardRecord> = {};
  for (const code of codes) cards[code] = dueRecord();
  return {
    schema_version: SCHEMA_VERSION,
    settings: { new_cards_per_day: newCardsPerDay },
    cards,
  };
}

/**
 * Seed the store and load the app from it. Establishes the origin, writes the
 * key, then reloads so launch reads the seeded state (the store reads
 * `localStorage` only at load). Pass a raw string to seed a deliberately corrupt
 * value for the error-path specs.
 */
export async function seed(page: Page, state: ReviewState | string): Promise<void> {
  const json = typeof state === 'string' ? state : JSON.stringify(state);
  await page.goto('/');
  await page.evaluate(
    ([key, value]) => window.localStorage.setItem(key, value),
    [STORAGE_KEY, json] as const,
  );
  await page.reload();
}

/** The persisted store as it stands, or `null` if the key is absent (cleared). */
export async function readState(page: Page): Promise<ReviewState | null> {
  const raw = await page.evaluate((key) => window.localStorage.getItem(key), STORAGE_KEY);
  return raw ? (JSON.parse(raw) as ReviewState) : null;
}

/**
 * The value `Locator` of a Home stat tile by its label (`Reviews due`,
 * `New today`). A tile is a big value div immediately followed by its label div,
 * so the value is the label's preceding sibling — assert on it with
 * `toHaveText`, which auto-waits for the Home re-mount to settle.
 */
export function statValue(page: Page, label: string): Locator {
  return page
    .getByText(label, { exact: true })
    .locator('xpath=preceding-sibling::div[1]');
}
