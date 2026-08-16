import { chromium, type FullConfig } from '@playwright/test';

/**
 * Gate the suite behind a fully built dev server.
 *
 * `dx serve --web` binds the port and answers Playwright's `webServer` readiness
 * probe before the wasm build finishes (~2min cold), then holds every request
 * open until it does. Without this gate Playwright starts the specs mid-build and
 * the first tests per worker burn their 30s timeout on a hung `page.goto` — the
 * CI flake this exists to kill. Navigate once here with a build-sized budget and
 * wait for Home to actually render, so every spec starts against a ready server.
 */
export default async function globalSetup(config: FullConfig): Promise<void> {
  const baseURL = config.projects[0]?.use.baseURL;
  if (!baseURL) throw new Error('globalSetup: no baseURL configured');

  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.goto(baseURL, { waitUntil: 'load', timeout: 570_000 });
    // The heading only paints once the wasm app has mounted; wait it out in case
    // the port served a pre-build placeholder before the real app was ready.
    await page
      .getByRole('heading', { name: 'World Learn' })
      .waitFor({ state: 'visible', timeout: 120_000 });
  } finally {
    await browser.close();
  }
}
