/**
 * Suite-wide test fixtures.
 *
 * The one guard here fails any test whose page raised an uncaught exception. On
 * the wasm target a Rust `panic!` aborts and surfaces as a JS `pageerror` — the
 * runtime failure class these e2e tests exist to catch (a missing wasm backend
 * dep, a bad `unwrap` on the grade path; see AGENTS.md). App-level errors are
 * caught at the anyhow boundary and rendered as a `Failure` notice via a
 * `console.error` (`ui::log_and_display`), *not* an uncaught exception, so the
 * deliberately-corrupt error-path specs stay green under this guard.
 *
 * Specs import `test`/`expect` from here instead of `@playwright/test`; the guard
 * is `auto`, so every spec gets it without opting in.
 */
import { test as base, expect } from '@playwright/test';

export const test = base.extend<{ failOnPageError: void }>({
  failOnPageError: [
    async ({ page }, use) => {
      const errors: Error[] = [];
      page.on('pageerror', (err) => errors.push(err));
      await use();
      expect(
        errors,
        `uncaught page error (wasm panic?):\n${errors.map((e) => e.stack ?? e.message).join('\n')}`,
      ).toHaveLength(0);
    },
    { auto: true },
  ],
});

export { expect };
