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
    if (!tasks.length) { complete(); return; }
    this.worker ??= new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
    const worker = this.worker;
    const ticket = this.ticket;
    this.active = true;
    let pending = tasks.length;
    const fail = (message: string) => {
      if (ticket !== this.ticket) return;
      this.cancel();
      failure(message);
    };
    this.timeout = window.setTimeout(() => fail('Worker exceeded its time budget'), 12_000);
    worker.onmessage = (event: MessageEvent<EngineReply>) => {
      if (ticket !== this.ticket || event.data.ticket !== ticket) return;
      receive(event.data);
      if (--pending === 0) {
        window.clearTimeout(this.timeout);
        this.active = false;
        complete();
      }
    };
    worker.onerror = event => fail(event.message || 'Could not load the numerical worker');
    worker.onmessageerror = () => fail('Could not decode the numerical worker response');
    worker.postMessage({ ticket, tasks });
  }

  dispose(): void {
    this.cancel();
    this.worker?.terminate();
    this.worker = null;
  }
}
