import { defineConfig, devices } from '@playwright/test';

/**
 * Drives the web **dev** target — `dx serve --web` on http://localhost:8080 (see
 * AGENTS.md). The suite seeds `localStorage` directly (the issue-24 web store
 * backend) to start each spec from a known deck/schedule; nothing here is a ship
 * build, so the seeding seam ships nowhere.
 *
 * `webServer` starts `dx serve --web` from the repo root and waits for the port.
 * Locally it reuses an already-running `dx serve`; in CI it always builds fresh.
 * The first wasm build is slow, hence the generous `timeout`.
 */
const PORT = 8080;

export default defineConfig({
  testDir: './tests',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  // One dev server backs the whole run; keep the concurrent page count modest.
  workers: 2,
  reporter: process.env.CI ? [['github'], ['html', { open: 'never' }]] : 'list',
  // dx serves the port before the wasm build finishes; this waits the build out
  // once so no spec races it (see helpers/global-setup.ts).
  globalSetup: './helpers/global-setup.ts',
  use: {
    baseURL: `http://localhost:${PORT}`,
    trace: 'on-first-retry',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: 'dx serve --web',
    cwd: '../..',
    url: `http://localhost:${PORT}`,
    reuseExistingServer: !process.env.CI,
    // The first wasm build from a cold cargo cache is minutes; give it room.
    timeout: 600_000,
    stdout: 'pipe',
    stderr: 'pipe',
  },
});
