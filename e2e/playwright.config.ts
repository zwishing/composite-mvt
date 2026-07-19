import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: '.',
  timeout: 30_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  workers: 1,
  reporter: 'line',
  use: {
    baseURL: 'http://127.0.0.1:3101',
    browserName: 'chromium',
    screenshot: 'off',
    trace: 'off',
    video: 'off',
    ...devices['Desktop Chrome'],
  },
  webServer: {
    command: 'PORT=3101 cargo run --example maplibre_server --features gzip',
    port: 3101,
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
