import { test, expect } from '@playwright/test';

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
    // The SVG renders a beat after the route mounts; read it only once it has.
    const highlight = `svg path[fill="${HIGHLIGHT_FILL}"]`;
    await page.locator(highlight).first().waitFor();

    const framing = await page.evaluate((sel) => {
      const mapSvg = document.querySelector(sel)?.closest('svg');
      return {
        viewBox: mapSvg?.getAttribute('viewBox') ?? null,
        // Each of the three wrap copies draws the highlighted country once.
        highlightedCount: document.querySelectorAll(sel).length,
      };
    }, highlight);

    expect(framing.highlightedCount).toBe(3);
    expect(framing.viewBox).not.toBeNull();
    expect(framing.viewBox).not.toBe(WORLD_VIEW_BOX);
  });
}
