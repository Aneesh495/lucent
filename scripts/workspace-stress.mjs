import assert from 'node:assert/strict';
import { decode, History, load, save } from '../web/src/model.ts';

class Storage {
  map = new Map();
  getItem(key) { return this.map.get(key) ?? null; }
  setItem(key, value) { this.map.set(key, value); }
}

const base = decode(JSON.stringify({
  version: 1, title: 'Stress workspace', cells: [{ id: 'a', expr: 'sin(x)' }],
  parameters: [{ name: 'k', value: 0, min: -50, max: 50, step: 1 }],
}));
const storage = new Storage();
const history = new History(base);
const past = [];
const future = [];
let expected = JSON.stringify(base);
let state = 0x8bf8c4aa;
function random(n) {
  state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
  return state % n;
}

const totals = { edit: 0, undo: 0, redo: 0, reload: 0, recovery: 0 };
for (let i = 0; i < 5500; i++) {
  const op = random(127) % 10;
  if (op < 5) {
    const number = random(101) - 50;
    const next = JSON.parse(expected);
    next.cells[0].source = `sin(${number}x)+${i}/17`;
    next.parameters[0].value = number;
    next.title = 'Revision ' + i;
    next.view.x = [-8 + number, 8 + number];
    past.push(expected);
    if (past.length > 100) past.shift();
    expected = JSON.stringify(next);
    future.length = 0;
    assert.equal(history.edit(draft => {
      draft.cells[0].source = next.cells[0].source;
      draft.parameters[0].value = number;
      draft.title = next.title;
      draft.view.x = next.view.x;
    }), true);
    totals.edit++;
  } else if (op === 5) {
    const found = past.pop();
    assert.equal(history.undo(), found !== undefined);
    if (found !== undefined) { future.push(expected); expected = found; }
    totals.undo++;
  } else if (op === 6) {
    const found = future.pop();
    assert.equal(history.redo(), found !== undefined);
    if (found !== undefined) { past.push(expected); expected = found; }
    totals.redo++;
  } else {
    save(storage, history.current);
    assert.equal(JSON.stringify(load(storage).workspace), expected);
    totals.reload++;
    if (op === 9) {
      save(storage, history.current);
      storage.setItem('lucent.workspace.v2', '{');
      const result = load(storage);
      assert.equal(JSON.stringify(result.workspace), expected);
      assert.equal(result.message, 'Recovered the last valid autosave');
      save(storage, result.workspace);
      totals.recovery++;
    }
  }
  assert.equal(JSON.stringify(history.current), expected);
}
assert.throws(() => decode(JSON.stringify({ version: 4 })));
assert.throws(() => decode(JSON.stringify({ version: 2, cells: Array(41).fill({}) })));
assert.throws(() => decode(JSON.stringify({ version: 2, cells: [{ id: 'same', source: 'x' }, { id: 'same', source: 'y' }] })));
assert.equal(base.cells[0].mode, 'explicit');
console.log(JSON.stringify({ seed: '8bf8c4aa', operations: 5500, ...totals }));
