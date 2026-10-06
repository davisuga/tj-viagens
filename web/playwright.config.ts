import { defineConfig, devices } from '@playwright/test';

const API = 'http://localhost:3101';
const WEB = 'http://localhost:5174';

/**
 * End-to-end suite: the real Rust API, Postgres and the Vite app, driven through
 * Chromium. It runs on its own ports and database (tjviagens_e2e), so the dev
 * servers on :3001/:5173 and their data are never touched; every test reseeds.
 *
 * Run: cd web && bun run test:e2e   (first run compiles the API)
 */
export default defineConfig({
  testDir: './e2e/tests',
  // One database for the whole suite and every test reseeds it: no overlap.
  workers: 1,
  fullyParallel: false,
  timeout: 120_000,
  expect: { timeout: 15_000 },
  reporter: [['list'], ['html', { open: 'never', outputFolder: 'e2e/report' }]],
  outputDir: 'e2e/results',
  use: {
    baseURL: WEB,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'], viewport: { width: 1280, height: 800 } } },
  ],
  webServer: [
    {
      name: 'api',
      command: 'sh e2e/serve-api.sh',
      url: `${API}/health`,
      timeout: 600_000,
      reuseExistingServer: !process.env.CI,
    },
    {
      name: 'web',
      command: 'bunx vite --port 5174 --strictPort',
      url: WEB,
      env: { VITE_API_URL: API },
      timeout: 120_000,
      reuseExistingServer: !process.env.CI,
    },
  ],
});
