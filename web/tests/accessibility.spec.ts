import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

for (const size of [{ width: 1280, height: 720 }, { width: 390, height: 844 }]) {
  test(`checks accessibility rules at ${size.width} by ${size.height}`, async ({ page }, info) => {
    await page.setViewportSize(size);
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto('/');
    await expect(page.locator('#compute-status')).toHaveAttribute('data-state', 'settled');
    const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa']).analyze();
    await info.attach('axe.json', { body: JSON.stringify(results, null, 2), contentType: 'application/json' });
    expect(results.violations.map(v => ({ id: v.id, targets: v.nodes.map(n => n.target) }))).toEqual([]);
    await page.getByRole('button', { name: 'Explorations' }).focus();
    await page.keyboard.press('Enter');
    await expect(page.getByRole('dialog', { name: 'Explorations' })).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('button', { name: 'Explorations' })).toBeFocused();
  });
}
