import { createInterface } from 'node:readline';
import { once } from 'node:events';
import createModule from '../web/public/engine/lucent.mjs';

const module = await createModule();
const encoder = new TextEncoder();
const decoder = new TextDecoder();
for await (const line of createInterface({ input: process.stdin, crlfDelay: Infinity })) {
  if (!line) continue;
  const bytes = encoder.encode(line);
  const pointer = module._lc_alloc(bytes.length);
  if (!pointer) throw new Error('Frozen case exceeds input budget');
  let response;
  try {
    module.HEAPU8.set(bytes, pointer);
    const address = module._lc_run(pointer, bytes.length);
    response = decoder.decode(module.HEAPU8.subarray(address, address + module._lc_output_len()));
  } finally {
    module._lc_free(pointer, bytes.length);
  }
  if (!process.stdout.write(response + '\n')) await once(process.stdout, 'drain');
}
