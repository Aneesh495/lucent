import { readdir } from 'node:fs/promises';
import { expect, test } from '@playwright/test';

test('preserves signed zero through the browser worker ABI', async ({ page }) => {
  const files = await readdir('dist/assets');
  const worker = files.find(file => /^worker-.*\.js$/.test(file));
  expect(worker).toBeTruthy();
  await page.goto('/');
  const result = await page.evaluate(async path => {
    const worker = new Worker(path, { type: 'module' });
    try {
      return await new Promise<{ negative: boolean; positive: boolean }>((resolve, reject) => {
        const timer = setTimeout(() => reject(Error('Worker did not answer')), 10000);
        worker.onmessage = ({ data }) => {
          clearTimeout(timer);
          const result = data.response.result;
          resolve({
            negative: Object.is(result[0].value.bounds[0], -0),
            positive: Object.is(result[1].value.bounds[0], 0),
          });
        };
        worker.onerror = reject;
        worker.postMessage({ ticket: 1, tasks: [{ key: 'zero', request: { op: 'table', expr: 'x', samples: [-0, 0] } }] });
      });
    } finally { worker.terminate(); }
  }, '/assets/' + worker);
  expect(result).toEqual({ negative: true, positive: true });
});
