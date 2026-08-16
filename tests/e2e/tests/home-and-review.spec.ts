import { test, expect } from '../helpers/fixtures';

import { dueState, seed, statValue } from '../helpers/state';

/**
 * Home launch surface + the front→reveal leg of the Review loop. These exercise
 * the wasm runtime, routing, and the localStorage read that unit tests can't: the
 * derivation math itself lives in `session.rs`/`map.rs` unit tests.
 */
test('Home reflects the seeded deck and offers Start', async ({ page }) => {
  await seed(page, dueState(['FRA', 'DEU']));

  await expect(page.getByRole('heading', { name: 'World Learn' })).toBeVisible();
  await expect(statValue(page, 'Reviews due')).toHaveText('2');
  await expect(statValue(page, 'New today')).toHaveText('0');
  await expect(page.getByRole('button', { name: 'Start · 2 cards' })).toBeVisible();
});

test('an empty deck shows 0/0 and omits Start', async ({ page }) => {
  await seed(page, dueState([]));

  await expect(statValue(page, 'Reviews due')).toHaveText('0');
  await expect(statValue(page, 'New today')).toHaveText('0');
  await expect(page.getByRole('button', { name: /^Start/ })).toHaveCount(0);
});

test('Start opens a card front, then reveal shows the name, pin, and grades', async ({ page }) => {
  // A single-country deck makes the queue's first (only) card deterministic.
  await seed(page, dueState(['FRA']));
  await page.getByRole('button', { name: 'Start · 1 cards' }).click();

  await expect(page).toHaveURL(/\/review$/);
  // Front: the reveal pill and the status strip; no name or grades yet.
  await expect(page.getByRole('button', { name: 'Tap to reveal' })).toBeVisible();
  await expect(page.getByText('1 left')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Good' })).toHaveCount(0);

  await page.getByRole('button', { name: 'Tap to reveal' }).click();

  // Reveal: the name and the four grade buttons; the front pill is gone.
  await expect(page.getByText('France').first()).toBeVisible();
  await expect(page.getByRole('button', { name: 'Tap to reveal' })).toHaveCount(0);
  for (const grade of ['Again', 'Hard', 'Good', 'Easy']) {
    await expect(page.getByRole('button', { name: grade })).toBeVisible();
  }
  // The pin drops on reveal (its entrance-animation class replaces `opacity-0`).
  await expect(page.getByText('📍')).toHaveClass(/animate-pin-drop/);
});
