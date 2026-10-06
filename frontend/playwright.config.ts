import { defineConfig } from '@playwright/test';

/**
 * End-to-end tests against the REAL API (memory store seeded with the
 * committed synthetic demo CSV) and the production build. Ports differ from
 * the dev servers (8003 / 5174) so both can run at once. Locally,
 * PW_CHROMIUM_PATH can point at an installed Chrome.
 *
 * `PW_VISUAL=1` enables the screenshot comparison in e2e/visual.spec.ts
 * (baselines are machine-specific and stay local in e2e/__visual__/).
 */
const API_PORT = 8093;
const WEB_PORT = 4184;

export default defineConfig({
  testDir: './e2e',
  timeout: 60_000,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: process.env.CI ? [['github'], ['list']] : 'list',
  snapshotPathTemplate: '{testDir}/__visual__/{arg}{ext}',
  use: {
    baseURL: `http://localhost:${WEB_PORT}`,
    trace: 'retain-on-failure',
    timezoneId: 'UTC',
    locale: 'en-US',
    launchOptions: {
      executablePath: process.env.PW_CHROMIUM_PATH || undefined,
    },
  },
  webServer: [
    {
      command: 'cargo run -q -p biomarker-api',
      cwd: '..',
      url: `http://localhost:${API_PORT}/health`,
      timeout: 300_000,
      reuseExistingServer: false,
      env: {
        APP_PORT: String(API_PORT),
        APP_STORE: 'memory',
        APP_SEED_DEMO: 'true',
        APP_DEMO_CSV: 'crates/api/tests/fixtures/demo_labs.csv',
      },
    },
    {
      command: `pnpm build && pnpm preview --port ${WEB_PORT} --strictPort`,
      url: `http://localhost:${WEB_PORT}`,
      timeout: 120_000,
      reuseExistingServer: false,
      env: { API_URL: `http://localhost:${API_PORT}` },
    },
  ],
});
