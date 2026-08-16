import { test, expect } from '../helpers/fixtures';

import { HIGHLIGHT_FILL, WORLD_VIEW_BOX, dueState, seed } from '../helpers/state';

/**
 * Representative-country render smoke. The regional-zoom framing *math* is
 * exhaustively unit-tested in `map.rs`; this only proves the SVG actually renders
 * in a real browser — one highlighted country, zoomed off the world view — for
 * the shapes past framing regressions targeted (large: Canada/Australia; split:
 * Indonesia; issues 10/14/15/21).
 */
for (const code of ['CAN', 'AUS', 'IDN']) {
  test(`the map renders a zoomed, highlighted frame for ${code}`, async ({ page }) => {
    await seed(page, dueState([code]));
    await page.getByRole('button', { name: 'Start · 1 cards' }).click();

    // Each of the three wrap copies draws the highlighted country once. The SVG
    // renders a beat after the route mounts and the copies can attach across
    // frames, so let the web-first count auto-retry until all three are present
    // before reading the viewBox — a plain querySelectorAll snapshot would race.
    const highlight = page.locator(`svg path[fill="${HIGHLIGHT_FILL}"]`);
    await expect(highlight).toHaveCount(3);

    const viewBox = await highlight
      .first()
      .evaluate((path) => path.closest('svg')?.getAttribute('viewBox') ?? null);

    expect(viewBox).not.toBeNull();
    expect(viewBox).not.toBe(WORLD_VIEW_BOX);
  });
}
