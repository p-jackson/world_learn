import { test, expect } from '@playwright/test';

import { dueState, emptyState, readState, seed, statValue } from '../helpers/state';

/**
 * Settings — the one interactive control (new-cards/day stepper) and the
 * destructive Clear-all-memory reset — plus the new-card cap end-to-end and the
 * Settings→Home back navigation.
 */
test('the new-cards-per-day cap bounds the day’s new intake on Home', async ({ page }) => {
  // No due cards, cap 3: Home offers exactly 3 new and nothing due.
  await seed(page, emptyState(3));

  await expect(statValue(page, 'Reviews due')).toHaveText('0');
  await expect(statValue(page, 'New today')).toHaveText('3');
  await expect(page.getByRole('button', { name: 'Start · 3 cards' })).toBeVisible();
});

test('changing the stepper persists and Home re-reads the new allowance', async ({ page }) => {
  await seed(page, emptyState(5));
  await page.getByRole('button', { name: 'Settings' }).click();
  await expect(page).toHaveURL(/\/settings$/);

  // The stepper value is the span between the −/+ buttons; anchor on the stable
  // + button rather than a fixed DOM nesting depth.
  const stepperValue = page.getByRole('button', { name: '+' }).locator('xpath=preceding-sibling::span[1]');
  await expect(stepperValue).toHaveText('5');
  await page.getByRole('button', { name: '+' }).click();
  await expect(stepperValue).toHaveText('6');

  // Persisted immediately, and reflected on Home after the back nav.
  expect((await readState(page))!.settings.new_cards_per_day).toBe(6);
  await page.getByRole('button', { name: '‹' }).click();
  await expect(page).toHaveURL(/:\d+\/$/);
  await expect(statValue(page, 'New today')).toHaveText('6');
});

test('Clear all memory wipes the store behind a two-tap confirm', async ({ page }) => {
  await seed(page, dueState(['FRA', 'DEU'], 3));
  await page.getByRole('button', { name: 'Settings' }).click();

  // First tap arms the confirm; the store is untouched.
  await page.getByRole('button', { name: 'Clear all memory' }).click();
  await expect(page.getByRole('button', { name: 'Tap again to erase all progress' })).toBeVisible();
  expect(await readState(page)).not.toBeNull();

  // Second tap erases: the key is removed and Home is back to first-launch.
  await page.getByRole('button', { name: 'Tap again to erase all progress' }).click();
  expect(await readState(page)).toBeNull();

  // Home is back to first-launch: no seen cards are due (the whole deck was
  // wiped), and the store reports the default new-card cap again.
  await page.getByRole('button', { name: '‹' }).click();
  await expect(statValue(page, 'Reviews due')).toHaveText('0');
  await expect(statValue(page, 'New today')).toHaveText('10');
});
