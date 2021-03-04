import { expect, test } from '@playwright/test';
import { GALLERY } from '../src/gallery';

test('evaluates every gallery exploration through the production worker', async ({ page }, info) => {
  const results = [];
  await page.goto('/');
  for (const [index, example] of GALLERY.entries()) {
    await page.getByRole('button', { name: 'Explorations' }).click();
    await page.locator('#gallery-list button').nth(index).click();
    await expect(page.getByRole('textbox', { name: 'Document title' })).toHaveValue(example.title);
    await expect(page.locator('#compute-status')).toHaveAttribute('data-state', 'settled', { timeout: 12_000 });
    const details = await page.locator('.cell-meta').allTextContents();
    expect(details.length).toBe(example.sketches.length);
    expect(details.every(s => s.length > 0)).toBe(true);
    const errors = await page.locator('.cell-meta[data-error=true]').allTextContents();
    if (example.title === 'A rational near a zero divisor') expect(errors).toEqual([expect.stringContaining('Denominator is zero')]);
    else expect(errors, example.title).toEqual([]);
    results.push({ index, title: example.title, details, errors });
  }
  await info.attach('gallery-observations', { body: JSON.stringify(results, null, 2), contentType: 'application/json' });
});
