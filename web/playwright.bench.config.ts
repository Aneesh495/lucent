import { defineConfig } from '@playwright/test';
import base from './playwright.config';

export default defineConfig({
  ...base,
  testDir: 'benchmarks',
  timeout: 120_000,
  outputDir: 'test-results/performance',
  reporter: [['list'], ['json', { outputFile: 'test-results/performance.json' }]],
  use: { ...base.use, trace: 'on' },
});
