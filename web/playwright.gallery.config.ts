import { defineConfig } from '@playwright/test';
import base from './playwright.config';

export default defineConfig({
  ...base,
  testDir: 'gallery-tests',
  timeout: 110_000,
  reporter: [['list'], ['json', { outputFile: 'test-results/gallery.json' }]],
});
