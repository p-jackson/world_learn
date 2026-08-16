import { test, expect } from '@playwright/test';

import { SCHEMA_VERSION, seed } from '../helpers/state';

/**
 * Store-load error paths surfaced through the UI boundary (`ui::Failure` /
 * `log_and_display`, ADR-0001). The point is that a bad stored document renders
 * a visible failure notice rather than a blank screen or a panic — the parse and
 * schema rules themselves are unit-tested in `store.rs`.
 *
 * Not covered here, deliberately:
 * - Missing/failed geometry asset: embedded via `include_str!`, so it cannot
 *   fail at runtime — a compile-time + `deck.rs` unit-test concern.
 * - Grade-path save error (the non-throwing `error!("{e:#}")` path): a
 *   `localStorage.setItem` failure isn't deterministically inducible headless;
 *   left to `session.rs` reasoning and unit coverage.
 */
test('a corrupt stored document renders the failure notice, not a blank screen', async ({ page }) => {
  await seed(page, '{ not valid json');

  await expect(page.getByText(/Failed to load/)).toBeVisible();
  await expect(page.getByRole('heading', { name: 'World Learn' })).toHaveCount(0);
});

test('an unsupported schema version fails loudly', async ({ page }) => {
  await seed(page, JSON.stringify({
    schema_version: SCHEMA_VERSION + 1,
    settings: { new_cards_per_day: 10 },
    cards: {},
  }));

  await expect(page.getByText(/unsupported schema version 2/)).toBeVisible();
});
