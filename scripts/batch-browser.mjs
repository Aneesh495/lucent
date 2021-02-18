import { once } from 'node:events';
import { createServer } from 'node:http';
import { createReadStream, promises as fs } from 'node:fs';
import { resolve, extname, sep } from 'node:path';
import { createInterface } from 'node:readline';
import { chromium } from '../web/node_modules/playwright/index.mjs';

const root = resolve('web/dist');
const workers = (await fs.readdir(resolve(root, 'assets'))).filter(name => /^worker-.*\.js$/.test(name));
if (workers.length !== 1) throw new Error('Expected one production browser worker bundle');
const types = { '.mjs': 'text/javascript', '.js': 'text/javascript', '.wasm': 'application/wasm', '.html': 'text/html' };
const server = createServer(async (request, response) => {
  try {
    const url = new URL(request.url, 'http://localhost');
    if (url.pathname === '/_runner') {
      response.writeHead(200, { 'content-type': 'text/html' }).end('<!doctype html><title>Lucent worker diagnostic</title>');
      return;
    }
    const path = resolve(root, '.' + decodeURIComponent(url.pathname));
    if (!path.startsWith(root + sep) || !(await fs.stat(path)).isFile()) {
      response.writeHead(404).end();
      return;
    }
    response.writeHead(200, { 'content-type': types[extname(path)] ?? 'application/octet-stream' });
    const body = createReadStream(path);
    body.on('error', () => response.destroy());
    body.pipe(response);
  } catch {
    response.writeHead(404).end();
  }
});
server.listen(0, '127.0.0.1');
await once(server, 'listening');
let browser;
try {
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  await page.goto('http://127.0.0.1:' + server.address().port + '/_runner');
  await page.evaluate(url => {
    globalThis.corpusWorker = new Worker(url, { type: 'module' });
  }, '/assets/' + workers[0]);
  let ticket = 0;
  const run = async requests => {
    const result = await page.evaluate(({ requests, ticket }) => new Promise((resolve, reject) => {
      const worker = globalThis.corpusWorker;
      const raw = [];
      const timer = setTimeout(() => reject(Error('Production worker timed out')), 25_000);
      worker.onerror = event => {
        clearTimeout(timer);
        reject(Error(event.message || 'Production worker failed'));
      };
      const encode = value => {
        if (typeof value === 'number' && Object.is(value, -0)) return '-0.0';
        if (Array.isArray(value)) return '[' + value.map(encode).join(',') + ']';
        if (value && typeof value === 'object') {
          return '{' + Object.entries(value).map(([k, v]) => JSON.stringify(k) + ':' + encode(v)).join(',') + '}';
        }
        return JSON.stringify(value);
      };
      worker.onmessage = ({ data }) => {
        if (data.ticket !== ticket) return;
        raw[Number(data.key)] = encode(data.response);
        if (raw.filter(Boolean).length === requests.length) {
          clearTimeout(timer);
          resolve(raw.join('\n') + '\n');
        }
      };
      worker.postMessage({ ticket, tasks: requests.map((request, i) => ({ key: String(i), request })) });
    }), { requests, ticket: ++ticket });
    if (!process.stdout.write(result)) await once(process.stdout, 'drain');
  };
  let batch = [];
  for await (const line of createInterface({ input: process.stdin, crlfDelay: Infinity })) {
    if (!line) continue;
    batch.push(JSON.parse(line));
    if (batch.length === 48) {
      await run(batch);
      batch = [];
    }
  }
  if (batch.length) await run(batch);
  await page.evaluate(() => globalThis.corpusWorker.terminate());
} finally {
  await browser?.close();
  server.close();
}
