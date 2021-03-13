import { wireJson } from './wire-json';
import type { EngineTask, EngineResult } from './engine';

interface Module {
  HEAPU8: Uint8Array;
  _lc_alloc(n: number): number;
  _lc_free(p: number, n: number): void;
  _lc_run(p: number, n: number): number;
  _lc_output_len(): number;
}

const base = import.meta.env.BASE_URL;
const moduleUrl = new URL(base + 'engine/lucent.mjs', self.location.origin).href;
const modulePromise: Promise<Module> = import(/* @vite-ignore */ moduleUrl).then(lib => lib.default());
const encoder = new TextEncoder();
const decoder = new TextDecoder();

function execute(module: Module, request: Record<string, unknown>): EngineResult {
  const bytes = encoder.encode(wireJson(request));
  const pointer = module._lc_alloc(bytes.length);
  if (!pointer) return { status: 'error', message: 'Request exceeds the engine input budget' };
  try {
    module.HEAPU8.set(bytes, pointer);
    const out = module._lc_run(pointer, bytes.length);
    const size = module._lc_output_len();
    if (!out || !size || size > 16_777_216) throw Error('Invalid engine response size');
    return JSON.parse(decoder.decode(module.HEAPU8.subarray(out, out + size)));
  } finally {
    module._lc_free(pointer, bytes.length);
  }
}

self.onmessage = async (event: MessageEvent<{ ticket: number; tasks: EngineTask[] }>) => {
  const { ticket, tasks } = event.data;
  try {
    const module = await modulePromise;
    for (const { key, request } of tasks) {
      self.postMessage({ ticket, key, response: execute(module, request) });
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    for (const { key } of tasks) self.postMessage({ ticket, key, response: { status: 'error', message } });
  }
};
