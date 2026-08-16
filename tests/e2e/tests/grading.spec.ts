import { type Page } from '@playwright/test';

import { test, expect } from '../helpers/fixtures';
import { dueState, emptyState, readState, seed, isoDate } from '../helpers/state';

/** Reveal the current card and grade it. */
async function grade(page: Page, button: 'Again' | 'Hard' | 'Good' | 'Easy'): Promise<void> {
  await page.getByRole('button', { name: 'Tap to reveal' }).click();
  await page.getByRole('button', { name: button }).click();
}

test('grading a due deck drains to the Done screen and persists each grade', async ({ page }) => {
  await seed(page, dueState(['FRA', 'DEU']));
  await page.getByRole('button', { name: 'Start · 2 cards' }).click();

  await grade(page, 'Good');
  // Still reviewing: the second card is now at the front.
  await expect(page.getByText('1 left')).toBeVisible();
  await grade(page, 'Good');

  // Session complete → the Done route, carrying the reviewed count.
  await expect(page).toHaveURL(/\/done\/2$/);
  await expect(page.getByText('2 reviewed · next batch unlocks tomorrow')).toBeVisible();

  // Every grade wrote through: both cards are now scheduled into the future.
  const state = await readState(page);
  expect(Object.keys(state!.cards).sort()).toEqual(['DEU', 'FRA']);
  for (const code of ['FRA', 'DEU']) {
    expect(state!.cards[code].due > isoDate(0)).toBe(true);
    expect(state!.cards[code].last_review).toBe(isoDate(0));
  }
});

test('a new-card session admits exactly the cap and stamps each introduction', async ({ page }) => {
  // No seen cards, cap 2: the session is 2 brand-new cards drawn from the deck.
  await seed(page, emptyState(2));
  await page.getByRole('button', { name: 'Start · 2 cards' }).click();

  await grade(page, 'Good');
  await grade(page, 'Good');

  await expect(page).toHaveURL(/\/done\/2$/);
  // Exactly the cap was introduced, each stamped as introduced today.
  const state = await readState(page);
  expect(Object.keys(state!.cards)).toHaveLength(2);
  for (const record of Object.values(state!.cards)) {
    expect(record.introduced_on).toBe(isoDate(0));
  }
});

test('Again re-drills the card instead of ending the session', async ({ page }) => {
  await seed(page, dueState(['FRA']));
  await page.getByRole('button', { name: 'Start · 1 cards' }).click();

  // Again requeues the only card: back to the front, not the Done screen.
  await grade(page, 'Again');
  await expect(page).toHaveURL(/\/review$/);
  await expect(page.getByRole('button', { name: 'Tap to reveal' })).toBeVisible();

  // A pass then drains it.
  await grade(page, 'Good');
  await expect(page).toHaveURL(/\/done\/1$/);
});

test('Back to home returns to Home from the Done screen (issue 22 guard)', async ({ page }) => {
  await seed(page, dueState(['FRA']));
  await page.getByRole('button', { name: 'Start · 1 cards' }).click();
  await grade(page, 'Good');

  await expect(page).toHaveURL(/\/done\/1$/);
  await page.getByRole('button', { name: 'Back to home' }).click();

  await expect(page).toHaveURL(/:\d+\/$/);
  await expect(page.getByRole('heading', { name: 'World Learn' })).toBeVisible();
  // The graded card is no longer due, so Home is back to nothing-to-do.
  await expect(page.getByRole('button', { name: /^Start/ })).toHaveCount(0);
});
