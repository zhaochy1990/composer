import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: '../../tests/e2e/tests',
  fullyParallel: false,
  workers: 1,
  retries: 1,
  reporter: 'html',
  timeout: 30_000,

  use: {
    baseURL: 'http://localhost:5173',
    screenshot: 'only-on-failure',
    trace: 'on-first-retry',
  },

  projects: [
    {
      name: 'chromium',
      use: { browserName: 'chromium' },
    },
  ],

  webServer: [
    {
      command: 'cargo run --bin composer-server',
      cwd: '../../',
      port: 3000,
      reuseExistingServer: !process.env.CI,
      env: {
        DATABASE_URL: `sqlite:${require('os').homedir().replace(/\\/g, '/')}/.composer/data/composer_test.db?mode=rwc`,
      },
      timeout: 120_000,
    },
    {
      command: 'pnpm run dev',
      port: 5173,
      reuseExistingServer: !process.env.CI,
      timeout: 30_000,
    },
  ],
});
