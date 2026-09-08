import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests',
  use: { baseURL: 'http://127.0.0.1:4325/omasend/', screenshot: 'only-on-failure', trace: 'retain-on-failure' },
  webServer: { command: 'bun scripts/preview.mjs', url: 'http://127.0.0.1:4325/omasend/', reuseExistingServer: !process.env.CI },
});
