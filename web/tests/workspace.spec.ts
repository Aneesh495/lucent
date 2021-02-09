import { test, expect, type Page } from '@playwright/test';

async function settled(page: Page) {
  await expect(page.locator('#compute-status')).toHaveAttribute('data-state', 'settled');
}

test('runs the actual engine, edits a graph, and inspects a unique root', async ({ page }, info) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/');
  await settled(page);
  await expect(page.locator('#compute-status')).toContainText(/WebGL2|Canvas 2D/);
  await expect(page.locator('.cell-meta').nth(1)).toContainText('unresolved');
  await page.screenshot({ path: info.outputPath('workspace.png') });
  const expression = page.getByRole('textbox', { name: 'Expression', exact: true }).first();
  await expression.fill('x^2-2');
  await page.getByLabel('Operation').selectOption('roots');
  await page.getByLabel('Lower / x').fill('1');
  await page.getByLabel('Upper', { exact: true }).fill('2');
  await page.getByRole('button', { name: 'Calculate' }).click();
  await expect(page.locator('#evidence-result')).toContainText('root certificate');
  await expect(page.locator('#evidence-result')).toContainText('1.414213562373');
  const downloadPromise = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Download evidence' }).click();
  const download = await downloadPromise;
  await download.saveAs(info.outputPath(download.suggestedFilename()));
  await page.getByRole('button', { name: 'Table', exact: true }).click();
  await expect(page.getByRole('table')).toBeVisible();
  await expect(page.getByRole('row')).toHaveCount(18);
  expect(errors).toEqual([]);
});

test('cancels superseded work and restores saved edits across reload', async ({ page }) => {
  await page.goto('/');
  await settled(page);
  const input = page.getByRole('textbox', { name: 'Expression', exact: true }).first();
  await input.fill('sin(1000000*x)/(x^2-x^2)');
  await page.waitForTimeout(42);
  await input.fill('x^2+?');
  await expect(page.locator('.cell-meta').first()).toContainText('Unexpected character');
  await input.fill('sqrt(x)');
  await settled(page);
  await expect(page.locator('.cell-meta').first()).not.toContainText('Unexpected character');
  await expect(page.locator('#save-status')).toContainText('Saved locally');
  await page.reload();
  await settled(page);
  await expect(input).toHaveValue('sqrt(x)');
  const before = await page.locator('#view-range').textContent();
  await page.getByRole('region', { name: 'Interactive graph' }).focus();
  await page.keyboard.press('+');
  await expect(page.locator('#view-range')).not.toHaveText(before!);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(page.locator('#view-range')).toHaveText(before!);
});

test('loads a gallery example, keyboard edits, and avoids narrow overflow', async ({ page }, info) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await page.getByRole('button', { name: 'Explorations' }).click();
  await expect(page.locator('#gallery-list button')).toHaveCount(58);
  await page.getByRole('button', { name: /^Repeated roots/ }).click();
  await expect(page.getByRole('textbox', { name: 'Document title' })).toHaveValue('Repeated roots');
  await settled(page);
  await page.getByRole('button', { name: 'Calculate' }).click();
  await expect(page.locator('#evidence-result')).toContainText('total multiplicity 5');
  const input = page.getByRole('textbox', { name: 'Expression', exact: true }).first();
  await input.focus();
  await page.keyboard.press('End');
  await page.getByRole('button', { name: 'Insert ^', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(input).toBeFocused();
  await expect(input).toHaveValue(/\^$/);
  await expect(page.locator('.cell-meta').first()).toContainText('Expected an expression');
  const widths = await page.evaluate(() => [document.documentElement.scrollWidth, innerWidth]);
  expect(widths[0]).toBeLessThanOrEqual(widths[1]);
  await page.screenshot({ path: info.outputPath('narrow.png'), fullPage: true });
});

test('imports a legacy document and reports corruption without losing the workspace', async ({ page }) => {
  await page.goto('/');
  await page.locator('#file').setInputFiles({
    name: 'v1.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify({
      version: 1, title: 'Migrated example', cells: [{ expr: 'x/3' }],
    })),
  });
  const title = page.getByRole('textbox', { name: 'Document title' });
  await expect(title).toHaveValue('Migrated example');
  await page.locator('#file').setInputFiles({ name: 'broken.json', mimeType: 'application/json', buffer: Buffer.from('{') });
  await expect(page.locator('#save-status')).not.toHaveText('Saved locally');
  await expect(title).toHaveValue('Migrated example');
});

test('renders a spatial exploration and supports keyboard camera controls', async ({ page }, info) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Explorations' }).click();
  await page.getByRole('button', { name: /^Sphere isosurface/ }).click();
  await settled(page);
  await expect(page.getByRole('heading', { name: 'Spatial canvas' })).toBeVisible();
  await expect(page.locator('#compute-status')).toContainText('approximate topology');
  const host = page.getByRole('region', { name: 'Interactive 3D mesh' });
  await host.focus();
  const before = await page.locator('#spatial-canvas').evaluate((c: HTMLCanvasElement) => c.toDataURL());
  await page.keyboard.press('ArrowLeft');
  await expect.poll(() => page.locator('#spatial-canvas').evaluate((c: HTMLCanvasElement) => c.toDataURL())).not.toBe(before);
  await page.screenshot({ path: info.outputPath('spatial.png') });
  await page.getByRole('button', { name: 'Graph', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Graph canvas' })).toBeVisible();
});

test('computes real complex and ODE layers from editable gallery inputs', async ({ page }, info) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Explorations' }).click();
  await page.getByRole('button', { name: /^The logarithm cut/ }).click();
  await settled(page);
  await expect(page.locator('.cell-meta').first()).toContainText('samples');
  await page.screenshot({ path: info.outputPath('complex.png') });
  await page.getByRole('button', { name: 'Explorations' }).click();
  await page.getByRole('button', { name: /^Trajectory event/ }).click();
  await settled(page);
  await expect(page.locator('.cell-meta').first()).toContainText('event bracketed');
  await page.getByRole('textbox', { name: 'Event residual' }).fill('x');
  await settled(page);
  await expect(page.locator('.cell-meta').first()).toContainText('event bracketed');
});

test('links editable observations to regression residuals and uncertainty', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Explorations' }).click();
  await page.getByRole('button', { name: /^Residual pattern in a line fit/ }).click();
  await expect(page.getByRole('heading', { name: 'Polynomial fit' })).toBeVisible();
  await expect(page.getByRole('columnheader', { name: 'Standard error' })).toBeVisible();
  await expect(page.getByRole('img', { name: 'Residuals by input' })).toBeVisible();
  await page.getByRole('spinbutton', { name: 'Polynomial degree' }).fill('2');
  await page.getByRole('spinbutton', { name: 'Polynomial degree' }).press('Tab');
  await expect(page.getByRole('table', { name: 'Estimated coefficients' }).getByRole('row')).toHaveCount(4);
});
