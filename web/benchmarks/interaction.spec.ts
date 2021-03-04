import { test, expect, type Page } from '@playwright/test';
import { writeFile } from 'node:fs/promises';

interface Mark { kind: 'post' | 'terminate'; time: number; source?: string }
interface InstrumentedWindow extends Window { workerMarks: Mark[]; onWorkerPost?: (source: string | undefined) => void }
const quantile = (values: number[], p: number) => [...values].sort((a,b) => a-b)[Math.ceil(p * values.length)-1];

async function ready(page: Page) {
  await expect(page.locator('#compute-status')).toHaveAttribute('data-state', 'settled');
}

test('records interaction, preview, refinement and cancellation distributions', async ({ page, browser }, info) => {
  await page.addInitScript(() => {
    const scope = window as unknown as InstrumentedWindow;
    scope.workerMarks = [];
    const Original = window.Worker;
    window.Worker = class extends Original {
      postMessage(value: { tasks?: {request?: {expr?: string}}[] }, transfer?: Transferable[]) {
        const source = value.tasks?.[0]?.request?.expr;
        scope.workerMarks.push({ kind: 'post', time: performance.now(), source });
        super.postMessage(value, transfer ?? []);
        scope.onWorkerPost?.(source);
      }
      terminate() {
        scope.workerMarks.push({ kind: 'terminate', time: performance.now() });
        super.terminate();
      }
    };
  });
  await page.goto('/');
  await ready(page);
  const engine = await page.evaluate(() => {
    const context = document.createElement('canvas').getContext('webgl2');
    const ext = context?.getExtension('WEBGL_debug_renderer_info');
    return { user_agent: navigator.userAgent, webgl: ext ? context!.getParameter(ext.UNMASKED_RENDERER_WEBGL) : 'unavailable',
      hardware_threads: navigator.hardwareConcurrency, pixels: [innerWidth, innerHeight] };
  });
  const preview: number[] = [];
  const refinement: number[] = [];
  for (let i = 0; i < 24; i++) {
    const timing = await page.evaluate(value => new Promise<{ preview: number; refinement: number }>((resolve, reject) => {
      const status = document.querySelector<HTMLElement>('#compute-status')!;
      const slider = document.querySelector<HTMLInputElement>('#parameter-a')!;
      const start = performance.now();
      let first: number | undefined;
      const watch = new MutationObserver(() => {
        if (status.dataset.state === 'refining') first ??= performance.now() - start;
        if (status.dataset.state === 'settled' && first !== undefined) {
          clearTimeout(timer);
          watch.disconnect();
          resolve({ preview: first, refinement: performance.now() - start });
        }
      });
      const timer = setTimeout(() => { watch.disconnect(); reject(Error('Missing preview or refinement transition')); }, 7000);
      watch.observe(status, { attributes: true, attributeFilter: ['data-state'] });
      slider.value = String(value);
      slider.dispatchEvent(new Event('input', { bubbles: true }));
    }), 0.35 + 0.05 * i);
    preview.push(timing.preview);
    refinement.push(timing.refinement);
  }

  const cancellation: number[] = [];
  const expression = page.getByRole('textbox', { name: 'Expression', exact: true }).first();
  for (let i = 0; i < 24; i++) {
    const pathological = `sin(${100001+i}x)/(x^2-x^2)`;
    const elapsed = await page.evaluate(({ pathological, i }) => new Promise<number>((resolve, reject) => {
      const scope = window as unknown as InstrumentedWindow;
      const source = document.querySelector<HTMLInputElement>('.expression-input')!;
      const timer = setTimeout(() => reject(Error('Pathological job did not dispatch')), 3000);
      scope.onWorkerPost = posted => {
        if (posted !== pathological) return;
        scope.onWorkerPost = undefined;
        clearTimeout(timer);
        queueMicrotask(() => {
          const edit = performance.now();
          source.value = `sin(x)+${i}/1000`;
          source.dispatchEvent(new Event('input', { bubbles: true }));
          const last = scope.workerMarks.find(mark => mark.kind === 'terminate' && mark.time >= edit);
          if (!last) { reject(Error('Superseding edit failed to terminate an active worker')); return; }
          resolve(last.time - edit);
        });
      };
      source.value = pathological;
      source.dispatchEvent(new Event('input', { bubbles: true }));
    }), { pathological, i });
    cancellation.push(elapsed);
    await ready(page);
    await expect(expression).toHaveValue(`sin(x)+${i}/1000`);
  }
  const times = await page.evaluate(() => new Promise<number[]>(resolve => {
    const canvas = document.querySelector<HTMLElement>('#plot')!;
    const rect = canvas.getBoundingClientRect();
    const rows: number[] = [];
    let previous = 0;
    const frame = (stamp: number) => {
      if (previous) rows.push(stamp - previous);
      previous = stamp;
      canvas.dispatchEvent(new WheelEvent('wheel', {
        clientX: rect.left + rect.width * .6, clientY: rect.top + rect.height * .45,
        deltaY: rows.length % 2 ? 8 : -8, cancelable: true,
      }));
      if (rows.length === 150) resolve(rows);
      else requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
  }));
  await ready(page);
  const medianFrame = quantile(times, .5);
  const metrics = {
    median_fps: 1000 / medianFrame,
    frame_p95_ms: quantile(times, .95),
    preview_p95_ms: quantile(preview, .95),
    refinement_p95_ms: quantile(refinement, .95),
    cancel_p95_ms: quantile(cancellation, .95),
  };
  const report = {
    engine: { ...engine, chromium: browser.version() }, metrics,
    raw: { frames: times, preview, refinement, cancellation },
    thresholds: { median_fps: 60, frame_p95_ms: 1000/30, preview_p95_ms: 100, refinement_p95_ms: 1000, cancel_p95_ms: 100 },
    passed: { median_fps: metrics.median_fps >= 60, frame_p95_ms: metrics.frame_p95_ms <= 1000/30,
      preview_p95_ms: metrics.preview_p95_ms <= 100, refinement_p95_ms: metrics.refinement_p95_ms <= 1000, cancel_p95_ms: metrics.cancel_p95_ms <= 100 },
  };
  await writeFile(info.outputPath('observations.json'), JSON.stringify(report, null, 2));
  console.log(JSON.stringify({ ...report, raw: undefined }));
  expect(times).toHaveLength(150);
  expect(preview).toHaveLength(24);
  expect(cancellation).toHaveLength(24);
});
