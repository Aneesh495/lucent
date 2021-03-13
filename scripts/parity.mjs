import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import createEngine from '../web/public/engine/lucent.mjs';

const engine = await createEngine();
const encoder = new TextEncoder();
const decoder = new TextDecoder();
const inputs = [
  { ratio: '1/3', bits: 128 },
  { ratio: '-1/3', bits: 128 },
  { ratio: '0', bits: 53 },
  { ratio: '-750', bits: 128 },
  { ratio: '750', bits: 128 },
  { ratio: '1/0', bits: 128 },
  { ratio: '1', bits: 999 },
].map(value => JSON.stringify(value));
inputs.push(
  '{"op":"table","expr":"x","samples":[-0.0,0.0]}',
  '{"op":"evaluate","expr":"1/0"}',
  '{"op":"table","expr":"x","samples":[1e400]}',
);

const native = spawnSync('./target/release/lucent', {
  input: inputs.join('\n') + '\n',
  encoding: 'utf8',
});
assert.equal(native.status, 0, native.stderr);
const expected = native.stdout.trim().split('\n');
for (const [index, input] of inputs.entries()) {
  const bytes = encoder.encode(input);
  const pointer = engine._lc_alloc(bytes.length);
  assert.ok(pointer);
  engine.HEAPU8.set(bytes, pointer);
  const result = engine._lc_run(pointer, bytes.length);
  const output = decoder.decode(engine.HEAPU8.subarray(result, result + engine._lc_output_len()));
  engine._lc_free(pointer, bytes.length);
  assert.equal(output, expected[index]);
}
const ordinary = JSON.parse(expected[0]);
assert.equal(ordinary.status, 'enclosure');
assert.ok(ordinary.bounds[0] < ordinary.bounds[1]);
assert.equal(JSON.parse(expected[2]).bounds[0], 1);
assert.deepEqual(JSON.parse(expected[3]).bounds, [0, Number.MIN_VALUE]);
assert.equal(JSON.parse(expected[4]).status, 'unresolved');
const zero = JSON.parse(expected[7]).result;
assert.ok(Object.is(zero[0].value.bounds[0], -0));
assert.ok(Object.is(zero[1].value.bounds[0], 0));
assert.equal(JSON.parse(expected[8]).status, 'undefined');
assert.equal(JSON.parse(expected[9]).status, 'error');
console.log(`Native/WASM parity: ${inputs.length} MPFR requests`);
