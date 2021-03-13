import { wireJson } from './wire-json';

export interface EngineResult {
  status: string;
  result?: unknown;
  message?: string;
  precision?: number;
  assumptions?: string[];
  equation?: string;
  span?: { start: number; end: number };
  target_met?: boolean;
}

export interface EngineTask { key: string; request: Record<string, unknown> }
export interface EngineReply { ticket: number; key: string; response: EngineResult }

export class Runner {
  private worker: Worker | null = null;
  private ticket = 0;
  private active = false;
  private timeout: number | undefined;
  private cacheBytes = 0;
  private readonly cache = new Map<string, { response: EngineResult; bytes: number }>();

  cancel(): void {
    ++this.ticket;
    window.clearTimeout(this.timeout);
    if (!this.active) return;
    this.worker?.terminate();
    this.worker = null;
    this.active = false;
  }

  run(tasks: EngineTask[], receive: (reply: EngineReply) => void, complete: () => void, failure: (message: string) => void): void {
    this.cancel();
    const keys = new Map<string, string>();
    const cold: EngineTask[] = [];
    for (const task of tasks) {
      const cacheable = ['graph', 'evaluate', 'inspect', 'color', 'field'].includes(String(task.request.op));
      const signature = cacheable ? wireJson(task.request) : null;
      const hit = signature ? this.cache.get(signature) : undefined;
      if (hit && signature) {
        this.cache.delete(signature);
        this.cache.set(signature, hit);
        receive({ ticket: this.ticket, key: task.key, response: hit.response });
      } else {
        if (signature) keys.set(task.key, signature);
        cold.push(task);
      }
    }
    if (!cold.length) { complete(); return; }
    this.worker ??= new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
    const worker = this.worker;
    const ticket = this.ticket;
    this.active = true;
    let pending = cold.length;
    const fail = (message: string) => {
      if (ticket !== this.ticket) return;
      this.cancel();
      failure(message);
    };
    this.timeout = window.setTimeout(() => fail('Worker exceeded its time budget'), 12_000);
    worker.onmessage = (event: MessageEvent<EngineReply>) => {
      if (ticket !== this.ticket || event.data.ticket !== ticket) return;
      const signature = keys.get(event.data.key);
      if (signature && event.data.response.status !== 'error') this.remember(signature, event.data.response);
      receive(event.data);
      if (--pending === 0) {
        window.clearTimeout(this.timeout);
        this.active = false;
        complete();
      }
    };
    worker.onerror = event => fail(event.message || 'Could not load the numerical worker');
    worker.onmessageerror = () => fail('Could not decode the numerical worker response');
    worker.postMessage({ ticket, tasks: cold });
  }

  private remember(key: string, response: EngineResult): void {
    const bytes = 2 * (key.length + JSON.stringify(response).length);
    if (bytes > 300_000 || this.cache.has(key)) return;
    const budget = 4_000_000;
    while (this.cache.size >= 64 || this.cacheBytes + bytes > budget) {
      const oldest = this.cache.keys().next().value;
      if (oldest === undefined) break;
      this.cacheBytes -= this.cache.get(oldest)!.bytes;
      this.cache.delete(oldest);
    }
    this.cache.set(key, { response, bytes });
    this.cacheBytes += bytes;
  }

  dispose(): void {
    this.cancel();
    this.worker?.terminate();
    this.worker = null;
    this.cache.clear();
    this.cacheBytes = 0;
  }
}
