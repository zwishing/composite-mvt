import { expect, test } from '@playwright/test';

test('loads the merged z0 tile and reports both source layers', async ({ page }) => {
  const pageErrors: Error[] = [];
  page.on('pageerror', (error) => pageErrors.push(error));

  const tileResponse = page.waitForResponse(
    (response) => response.url().endsWith('/tiles/0/0/0.pbf'),
  );

  await page.goto('/');
  const response = await tileResponse;

  expect(response.status()).toBe(200);
  expect(response.headers()['content-type']).toBe('application/vnd.mapbox-vector-tile');
  expect(response.headers()['content-encoding']).toBe('gzip');
  expect(response.headers()['cache-control']).toBe('no-store');
  await expect(page.locator('.maplibregl-canvas')).toBeVisible();
  const status = page.locator('#status');
  await expect(status).toHaveAttribute('role', 'status');
  await expect(status).toHaveAttribute('aria-live', 'polite');
  await expect(status).toHaveAttribute('data-state', 'ready');
  expect(Number(await status.getAttribute('data-roads'))).toBeGreaterThan(0);
  expect(Number(await status.getAttribute('data-buildings'))).toBeGreaterThan(0);
  await expect(pageErrors).toEqual([]);
});

test('shows an error state if tile requests fail', async ({ page }) => {
  await page.route('**/tiles/0/0/0.pbf', (route) => route.abort('failed'));

  await page.goto('/');
  await expect(page.locator('#status')).toHaveAttribute('data-state', 'error');
  await expect(page.locator('#status')).toHaveAttribute('data-roads', '0');
  await expect(page.locator('#status')).toHaveAttribute('data-buildings', '0');
});
