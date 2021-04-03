import './style.css';
import { probability, regression } from './analysis-table';
import { Runner, type EngineResult, type EngineTask } from './engine';
import { GALLERY, loadExploration } from './gallery';
import { COLORS, HOME, LIMIT, MODES, History, cell, decode, domain, environment, load, save,
  type Angle, type Cell, type Mode, type Point, type View, type Workspace } from './model';
import { annotatePng } from './png';
import { SpatialCanvas, type Mesh, type SurfaceLayer } from './spatial-view';
import { Renderer, coordinate, type Geometry, type Layer, type Raster, type Arrow } from './renderer';

function id<T extends HTMLElement>(key: string): T {
  const element = document.getElementById(key);
  if (!element) throw Error('Missing UI element ' + key);
  return element as T;
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, text?: string, cls?: string): HTMLElementTagNameMap[K] {
  const element = document.createElement(tag);
  if (text !== undefined) element.textContent = text;
  if (cls) element.className = cls;
  return element;
}

const loaded = load(localStorage);
const history = new History(loaded.workspace);
const plots = new Runner();
const analysis = new Runner();
const tables = new Runner();
const renderer = new Renderer(id('grid'), id('geometry'), id('overlay'));
const spatial = new SpatialCanvas(id('spatial-canvas'), id('spatial-plot'));
const meshes = new Map<string, { layer: SurfaceLayer; request: Record<string, unknown>; response: EngineResult }>();
const layers = new Map<string, { layer: Layer; request: Record<string, unknown>; response: EngineResult }>();
const diagnostics = new Map<string, HTMLElement>();
const expressionInputs = new Map<string, HTMLInputElement>();
const plot = id('plot');
const plotStatus = id('compute-status');
const evidence = id('evidence-result');
const selected = id<HTMLSelectElement>('selected-cell');
const second = id<HTMLSelectElement>('comparison');
const angle = id<HTMLSelectElement>('angle');
const saveStatus = id('save-status');
const title = id<HTMLInputElement>('title');
const operation = id<HTMLSelectElement>('operation');
const lower = id<HTMLInputElement>('lower');
const upper = id<HTMLInputElement>('upper');
const setDisabled = (key: string, disabled: boolean) => { id<HTMLButtonElement>(key).disabled = disabled; };

let activeCell = history.current.cells[0]?.id ?? '';
let liveView = structuredClone(history.current.view);
let showUncertainty = true;
let selection: Point | null = null;
type Tab = 'graph' | 'table' | 'spatial';
let tab: Tab = 'graph';
let saveTimer = 0;
let computeTimer = 0;
let renderFrame = 0;
let revision = 0;
let settled = false;
let snapshot: { request: Record<string, unknown>; claim: EngineResult } | null = null;

saveStatus.textContent = loaded.message;

function status(state: string, message: string): void {
  plotStatus.dataset.state = state;
  plotStatus.replaceChildren(el('span', undefined, 'indicator'), document.createTextNode(message));
}

function dataMode(mode: Mode): boolean { return mode === 'regression' || mode === 'probability'; }

function surfaceMode(mode: Mode): boolean {
  return mode === 'surface' || mode === 'parametric_surface' || mode === 'isosurface';
}

function active(): Cell | undefined {
  return history.current.cells.find(c => c.id === activeCell);
}

function textResult(value: unknown): string {
  if (!value || typeof value !== 'object') return String(value ?? '—');
  const v = value as Record<string, unknown>;
  if (v.value !== undefined) return String(v.value);
  if (Array.isArray(v.bounds)) return '[' + v.bounds.map(x => Number(x).toPrecision(16)).join(', ') + ']';
  if (Array.isArray(v.items)) return '[' + v.items.map(textResult).join(', ') + ']';
  if (v.real) return textResult(v.real) + ' + (' + textResult(v.imaginary) + ')i';
  if (v.truth) return String(v.truth);
  return String(v.message ?? v.status ?? '—');
}

function badge(value: string, kind = ''): HTMLElement {
  return el('span', value.replaceAll('_', ' '), 'badge ' + kind);
}

function invalidateEvidence(): void {
  const paired = ['intersection', 'tangency', 'area_between'].includes(operation.value);
  second.hidden = !paired;
  id('comparison-label').hidden = !paired;
  analysis.cancel();
  snapshot = null;
  id('save-certificate').hidden = true;
  evidence.replaceChildren(badge('ready', 'neutral'), el('p', 'Calculate to inspect the current expression and assumptions.'));
  setDisabled('calculate', !active() || active()?.mode === 'definition');
}

function explain(result: EngineResult): void {
  evidence.replaceChildren(badge(result.status, /error|unsupported|undefined|unresolved|partial/.test(result.status) ? 'problem' : ''));
  if (result.message) evidence.append(el('p', result.message));
  const p = result.result as Record<string, unknown> | undefined;
  if (p) {
    if (['exact', 'enclosure', 'complex', 'list', 'boolean'].includes(result.status)) {
      evidence.append(el('p', textResult(p)));
      if (Array.isArray(p.dimension) && p.dimension.some(Boolean)) evidence.append(el('p', 'SI unit powers ' + p.dimension.join(', ')));
    }
    if (Array.isArray(p.unique)) {
      for (const r of p.unique.slice(0, 10)) {
        evidence.append(el('p', 'Root ' + JSON.stringify(r.bracket) + ' · endpoint signs ' + r.endpoint_signs.join(', ') + ' · slope ' + JSON.stringify(r.derivative)));
      }
      if (!p.unique.length) evidence.append(el('p', 'No unique root proved on this domain.'));
    }
    if (p.distinct_count !== undefined) {
      evidence.append(el('p', p.distinct_count + ' distinct roots · total multiplicity ' + p.multiplicity_count));
      if (Array.isArray(p.roots)) for (const r of p.roots.slice(0, 10)) {
        evidence.append(el('p', r.low === r.high ? r.low + ' · multiplicity ' + r.multiplicity
          : '(' + r.low + ', ' + r.high + ') · multiplicity ' + r.multiplicity));
      }
    }
    if (Array.isArray(p.points)) for (const point of p.points.slice(0, 10)) {
      evidence.append(el('p', point.classification.replaceAll('_', ' ') + ' · ' + JSON.stringify(point.root.bracket)));
    }
    if (typeof p.estimate === 'number') evidence.append(el('p', 'Estimate ' + p.estimate + ' · estimated error ' + p.estimated_error));
    if (Array.isArray(p.enclosure)) evidence.append(el('p', textResult({ bounds: p.enclosure })));
    if (Array.isArray(p.panels)) evidence.append(el('p', p.panels.length + ' quadrature panels'));
    if (Array.isArray(p.unresolved) && p.unresolved.length) evidence.append(el('p', p.unresolved.length + ' unresolved subintervals'));
    if (Array.isArray(p.symbolic)) {
      evidence.append(el('p', 'Symbolic · ' + textResult({ bounds: p.symbolic })));
      if (Array.isArray(p.automatic)) evidence.append(el('p', 'Forward mode · ' + textResult({ bounds: p.automatic })));
      if (Array.isArray(p.condition)) evidence.append(el('p', 'Relative condition · ' + textResult({ bounds: p.condition })));
      evidence.append(el('p', 'Finite difference · ' + (p.finite_difference ?? 'unavailable')));
      if (p.step_disagreement !== null) evidence.append(el('p', 'Step disagreement · ' + p.step_disagreement));
    }
  }
  if (result.equation) evidence.append(el('code', result.equation));
  if (result.precision) evidence.append(el('p', result.precision + ' bit MPFR evaluation'));
  if (result.target_met === false) evidence.append(el('p', 'Requested width remained unresolved.'));
  if (result.assumptions?.length) {
    const list = el('ul');
    for (const item of result.assumptions) list.append(el('li', item));
    evidence.append(list);
  }
}

function selectCell(key: string, update = true): void {
  if (activeCell === key && selected.value === key) return;
  activeCell = key;
  selected.value = key;
  for (const article of id('expressions').querySelectorAll<HTMLElement>('.cell')) {
    article.classList.toggle('current', article.dataset.cell === key);
  }
  invalidateEvidence();
  if (update && tab === 'table') computeTable();
}

function persist(): void {
  clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    try {
      save(localStorage, history.current);
      saveStatus.textContent = 'Saved locally';
    } catch (error) {
      saveStatus.textContent = error instanceof Error ? error.message : 'Autosave unavailable';
    }
  }, 220);
}

function syncButtons(): void {
  setDisabled('undo', !history.canUndo);
  setDisabled('redo', !history.canRedo);
  setDisabled('add', history.current.cells.length >= 40);
  setDisabled('parameter-add', history.current.parameters.length >= 16);
}

function update(change: (doc: Workspace) => void, { rebuild = false, recompute = true } = {}): void {
  if (!history.edit(change)) return;
  syncButtons();
  persist();
  if (rebuild) renderDocument();
  if (recompute) schedule(true);
}

function patch(key: string, change: Partial<Cell>, rebuild = false, recompute = true): void {
  update(doc => Object.assign(doc.cells.find(c => c.id === key)!, change), { rebuild, recompute });
  if (!recompute) draw();
}

function inputFor(c: Cell, property: 'source' | 'horizontal' | 'event' | 'third', text: string): HTMLInputElement {
  const input = el('input', undefined, 'expression-input');
  input.value = c[property];
  input.maxLength = 8192;
  input.spellcheck = false;
  input.autocomplete = 'off';
  input.setAttribute('aria-label', text);
  input.addEventListener('focus', () => selectCell(c.id));
  input.addEventListener('input', () => {
    patch(c.id, { [property]: input.value });
    if (property === 'source') {
      const option = [...selected.options].find(o => o.value === c.id);
      if (option) option.text = input.value || 'Empty expression';
      const comparison = [...second.options].find(o => o.value === c.id);
      if (comparison) comparison.text = input.value || 'Empty expression';
    }
  });
  return input;
}

function renderDocument(): void {
  const doc = history.current;
  title.value = doc.title;
  angle.value = doc.angle;
  id<HTMLInputElement>('zmin').value = String(doc.z[0]);
  id<HTMLInputElement>('zmax').value = String(doc.z[1]);
  liveView = structuredClone(doc.view);
  if (!active()) activeCell = doc.cells[0]?.id ?? '';
  diagnostics.clear();
  expressionInputs.clear();
  const prior = second.value;
  selected.replaceChildren();
  second.replaceChildren();
  const root = id('expressions');
  root.replaceChildren();
  const folders = [...new Set(doc.cells.map(c => c.folder))];
  for (const folder of folders) {
    if (folder) root.append(el('h3', folder, 'folder-heading'));
    for (const c of doc.cells.filter(item => item.folder === folder)) {
      const option = el('option', c.source || 'Empty expression');
      option.value = c.id;
      selected.append(option);
      if (c.mode === 'explicit') second.append(option.cloneNode(true));
      const node = el('article', undefined, 'cell' + (c.id === activeCell ? ' current' : ''));
      node.dataset.cell = c.id;
      node.style.setProperty('--cell-color', c.color);
      const heading = el('div', undefined, 'cell-heading');
      const color = el('input');
      color.type = 'color';
      color.value = c.color;
      color.setAttribute('aria-label', 'Plot color');
      color.addEventListener('change', () => patch(c.id, { color: color.value }, true));
      const mode = el('select');
      mode.setAttribute('aria-label', 'Expression mode');
      for (const [key, label] of Object.entries(MODES)) {
        const option = el('option', label);
        option.value = key;
        mode.append(option);
      }
      mode.value = c.mode;
      mode.addEventListener('change', () => patch(c.id, { mode: mode.value as Mode }, true));
      const remove = el('button', '×');
      remove.setAttribute('aria-label', 'Remove expression');
      remove.addEventListener('click', () => update(draft => {
        draft.cells = draft.cells.filter(item => item.id !== c.id);
      }, { rebuild: true }));
      heading.append(color, mode, remove);
      const input = inputFor(c, 'source', c.mode === 'parametric' ? 'Vertical expression' : c.mode === 'field' || c.mode === 'ode' ? 'dy/dt expression' : 'Expression');
      const meta = el('p', '', 'cell-meta');
      meta.id = 'diagnostic-' + c.id;
      input.setAttribute('aria-describedby', meta.id);
      node.append(heading, input);
      if (c.mode === 'parametric_surface') {
        node.append(inputFor(c, 'horizontal', 'Surface x(u,v)'));
        node.append(inputFor(c, 'third', 'Surface y(u,v)'));
      }
      if (c.mode === 'parametric') node.append(inputFor(c, 'horizontal', 'Horizontal expression'));
      if (c.mode === 'ode' || c.mode === 'field') node.append(inputFor(c, 'horizontal', 'dx/dt expression'));
      if (c.mode === 'parametric_surface') {
        for (const key of ['domain', 'domainV'] as const) {
          const range = el('div', undefined, 'cell-domain');
          for (const i of [0, 1] as const) {
            const label = el('label', (key === 'domain' ? 'u' : 'v') + (i === 0 ? ' min' : ' max'));
            const value = el('input');
            value.value = String(c[key][i]);
            value.addEventListener('change', () => {
              const next = [...history.current.cells.find(item => item.id === c.id)![key]];
              next[i] = Number(value.value);
              try { patch(c.id, { [key]: domain(next) }); } catch { value.value = String(c[key][i]); }
            });
            label.append(value);
            range.append(label);
          }
          node.append(range);
        }
      }
      if (c.mode === 'parametric' || c.mode === 'polar' || c.mode === 'ode') {
        const range = el('div', undefined, 'cell-domain');
        for (const index of [0, 1] as const) {
          const label = el('label', index === 0 ? 't min' : 't max');
          const value = el('input');
          value.value = String(c.domain[index]);
          value.inputMode = 'decimal';
          value.addEventListener('change', () => {
            const next = [...history.current.cells.find(item => item.id === c.id)!.domain];
            next[index] = Number(value.value);
            try { patch(c.id, { domain: domain(next) }); } catch { value.value = String(c.domain[index]); }
          });
          label.append(value);
          range.append(label);
        }
        node.append(range);
      }
      if (c.mode === 'ode') {
        const start = el('div', undefined, 'cell-domain');
        for (const index of [0, 1] as const) {
          const label = el('label', index === 0 ? 'x₀' : 'y₀');
          const value = el('input');
          value.value = String(c.initial[index]);
          value.inputMode = 'decimal';
          value.addEventListener('change', () => {
            const next = [...history.current.cells.find(item => item.id === c.id)!.initial] as Point;
            if (Number.isFinite(Number(value.value))) {
              next[index] = Number(value.value);
              patch(c.id, { initial: next });
            }
          });
          label.append(value);
          start.append(label);
        }
        node.append(start);
        const eventInput = inputFor(c, 'event', 'Event residual');
        eventInput.placeholder = 'Optional event, e.g. x - 1';
        node.append(eventInput);
      }
      if (c.mode === 'regression') {
        const label = el('label', 'Polynomial degree', 'cell-domain');
        const degree = el('input');
        degree.type = 'number';
        degree.min = '0';
        degree.max = '6';
        degree.value = String(c.degree);
        degree.onchange = () => {
          if (Number.isInteger(degree.valueAsNumber) && degree.valueAsNumber >= 0 && degree.valueAsNumber <= 6) {
            patch(c.id, { degree: degree.valueAsNumber });
          }
        };
        label.append(degree);
        node.append(label);
      }
      node.append(meta);
      const actions = el('div', undefined, 'cell-actions');
      const visible = el('input');
      visible.type = 'checkbox';
      visible.checked = c.visible;
      visible.addEventListener('change', () => patch(c.id, { visible: visible.checked }));
      const label = el('label', 'Show');
      label.prepend(visible);
      const style = el('button', c.style === 'solid' ? 'Solid' : 'Dashed');
      style.setAttribute('aria-label', 'Toggle line style');
      style.addEventListener('click', () => patch(c.id, { style: c.style === 'solid' ? 'dashed' : 'solid' }, true));
      const group = el('input');
      group.placeholder = 'Folder';
      group.value = c.folder;
      group.setAttribute('aria-label', 'Expression folder');
      group.size = 7;
      group.maxLength = 40;
      group.addEventListener('change', () => patch(c.id, { folder: group.value.trim() }, true));
      actions.append(label, style, group);
      node.append(actions);
      root.append(node);
      expressionInputs.set(c.id, input);
      diagnostics.set(c.id, meta);
    }
  }
  selected.value = activeCell;
  second.value = [...second.options].some(option => option.value === prior)
    ? prior : [...second.options].find(option => option.value !== activeCell)?.value ?? activeCell;
  invalidateEvidence();
  renderParameters();
  syncButtons();
}

function renderParameters(): void {
  const parent = id('parameters');
  parent.replaceChildren();
  for (const p of history.current.parameters) {
    const item = el('div', undefined, 'parameter');
    const label = el('label', p.name);
    label.htmlFor = 'parameter-' + p.name;
    const current = el('output', coordinate(p.value));
    label.append(current);
    const range = el('input');
    range.type = 'range';
    range.id = label.htmlFor;
    range.min = String(p.min);
    range.max = String(p.max);
    range.step = String(p.step);
    range.value = String(p.value);
    range.addEventListener('input', () => {
      current.textContent = coordinate(range.valueAsNumber);
      update(doc => { doc.parameters.find(item => item.name === p.name)!.value = range.valueAsNumber; });
    });
    const remove = el('button', '×');
    remove.setAttribute('aria-label', 'Remove parameter ' + p.name);
    remove.addEventListener('click', () => update(doc => {
      doc.parameters = doc.parameters.filter(item => item.name !== p.name);
    }, { rebuild: true }));
    item.append(label, range, remove);
    parent.append(item);
  }
}

function diagnose(key: string, response: EngineResult): void {
  const node = diagnostics.get(key);
  if (!node) return;
  const kind = response.status;
  const failed = !!response.message;
  node.dataset.error = String(failed);
  expressionInputs.get(key)?.setAttribute('aria-invalid', String(failed));
  const position = response.span ? ' · bytes ' + response.span.start + '–' + response.span.end : '';
  if (response.message) node.textContent = response.message + position;
  else if (kind === 'approximate_geometry') {
    const geometry = response.result as Geometry;
    const uncertain = geometry.cells.filter(tile => tile.kind === 'unresolved' || tile.kind === 'crossing' && !tile.topology).length;
    node.textContent = uncertain ? uncertain + ' unresolved cells' : 'Approximate geometry';
  } else if (response.result && typeof response.result === 'object' && 'samples' in response.result) {
    const trajectory = response.result as Trajectory;
    node.textContent = trajectory.samples.length + ' samples · ' +
      (trajectory.stiffness_warning ? 'stiffness suspected' : trajectory.event ? 'event bracketed' : trajectory.reason ?? 'approximate trajectory') +
      (trajectory.local_box ? ' · validated first step' : '');
  } else if (kind === 'approximate_field') {
    const field = response.result as { arrows: Arrow[] };
    node.textContent = field.arrows.length + ' sampled vectors';
  } else if (kind === 'approximate_mesh') {
    const mesh = response.result as Mesh;
    node.textContent = mesh.faces.length + ' triangles · ' + mesh.gaps.length + ' excluded or unresolved cells';
  } else if (kind === 'approximate_coloring') {
    const raster = response.result as Raster;
    node.textContent = raster.size.join(' × ') + ' samples · ' + raster.undefined_samples + ' undefined';
  } else if (kind === 'syntax') node.textContent = 'Definition compiled';
  else node.textContent = kind.replaceAll('_', ' ') + ' · ' + textResult(response.result);
}

function draw(): void {
  if (renderFrame) return;
  renderFrame = requestAnimationFrame(() => {
    renderFrame = 0;
    renderer.draw(liveView, [...layers.values()].map(item => item.layer), showUncertainty, selection);
    id('view-range').textContent = 'x [' + liveView.x.map(coordinate).join(', ') + ']';
  });
}

interface Trajectory {
  samples: { state: Point }[];
  event?: unknown; reason?: string; stiffness_warning: boolean; local_box?: unknown;
}
function taskFor(c: Cell, preview: boolean): EngineTask {
  const doc = history.current;
  const shared = environment(doc);
  const base = { ...shared, expr: c.source };
  if (c.mode === 'definition') return { key: c.id, request: { ...base, op: 'inspect', expr: '0' } };
  if (c.mode === 'scalar') return { key: c.id, request: { ...base, op: 'evaluate' } };
  const view = { ...structuredClone(liveView), pixels: [Math.min(renderer.width, 4096), Math.min(renderer.height, 4096)] };
  if (c.mode === 'field') return { key: c.id, request: { ...shared, op: 'field', dx: c.horizontal, dy: c.source, view, grid: 13 } };
  if (c.mode === 'ode') return { key: c.id, request: { ...shared, op: 'ode', dx: c.horizontal, dy: c.source,
    time: c.domain, initial: c.initial, event: c.event.trim() || null, validated: true, tolerance: 1e-8, budget: 2000 } };
  return { key: c.id, request: {
    ...base, op: c.mode === 'complex' ? 'color' : 'graph', mode: c.mode, horizontal: c.horizontal,
    view,
    ...(c.mode === 'parametric' || c.mode === 'polar' ? { domain: c.domain } : {}),
    bits: preview ? 64 : 96,
    budget: preview ? 1200 : refinement,
    pixel_error: preview ? 1.5 : 0.45,
  } };
}

let refinement = 4600;
function compute(expected: number, preview: boolean): void {
  if (expected !== revision || tab !== 'graph') return;
  const cells = history.current.cells.filter(c => c.visible && c.source.trim() && !surfaceMode(c.mode) && !dataMode(c.mode));
  const tasks = cells.map(c => taskFor(c, preview));
  const lookup = new Map(tasks.map(task => [task.key, task.request]));
  plots.run(tasks, ({ key, response }) => {
    if (expected !== revision) return;
    diagnose(key, response);
    if (response.status === 'approximate_geometry') {
      const c = cells.find(item => item.id === key)!;
      layers.set(key, { layer: { cell: c, geometry: response.result as Geometry }, request: lookup.get(key)!, response });
    } else if (response.status === 'approximate_coloring') {
      const c = cells.find(item => item.id === key)!;
      layers.set(key, { layer: { cell: c, raster: response.result as Raster }, request: lookup.get(key)!, response });
    } else if (response.status === 'approximate_field') {
      const c = cells.find(item => item.id === key)!;
      layers.set(key, { layer: { cell: c, arrows: (response.result as { arrows: Arrow[] }).arrows }, request: lookup.get(key)!, response });
    } else if (response.result && typeof response.result === 'object' && 'samples' in response.result) {
      const c = cells.find(item => item.id === key)!;
      const samples = (response.result as Trajectory).samples;
      const geometry: Geometry = { processed: samples.length, cells: samples.slice(1).map((sample, index) =>
        ({ kind: 'segment', segment: [samples[index].state, sample.state] })) };
      layers.set(key, { layer: { cell: c, geometry }, request: lookup.get(key)!, response });
    } else layers.delete(key);
    draw();
  }, () => {
    if (expected !== revision) return;
    if (preview && tasks.some(task => task.request.op === 'graph' || task.request.op === 'color')) {
      status('refining', 'Preview shown · refining');
      computeTimer = window.setTimeout(() => compute(expected, false), 55);
    } else {
      settled = true;
      const count = [...layers.values()].reduce((sum, item) => sum + (item.layer.geometry?.cells
        .filter(t => t.kind === 'unresolved' || t.kind === 'crossing' && !t.topology).length ?? item.layer.raster?.undefined_samples ?? 0), 0);
      status('settled', renderer.backend + (count ? ' · ' + count + ' unresolved' : ' · Current'));
    }
  }, message => status('error', message));
}

function schedule(clear: boolean, preview = true): void {
  ++revision;
  clearTimeout(computeTimer);
  plots.cancel();
  settled = false;
  if (clear) {
    layers.clear();
    meshes.clear();
    spatial.setLayers([], showUncertainty);
    selection = null;
    invalidateEvidence();
  }
  draw();
  if (tab === 'table') { computeTable(); return; }
  if (tab === 'spatial') {
    const current = revision;
    status('pending', 'Meshing current expressions');
    computeTimer = window.setTimeout(() => computeSpatial(current, preview), 22);
    return;
  }
  const expected = revision;
  status('pending', clear ? 'Computing current expressions' : 'Previous geometry · updating view');
  computeTimer = window.setTimeout(() => compute(expected, preview), 22);
}

function computeSpatial(expected: number, preview: boolean): void {
  if (expected !== revision || tab !== 'spatial') return;
  const doc = history.current;
  const cells = doc.cells.filter(c => c.visible && c.source.trim() && surfaceMode(c.mode));
  const space = { ...doc.view, z: doc.z };
  const tasks: EngineTask[] = cells.map(c => ({
    key: c.id, request: {
      ...environment(doc), op: c.mode === 'isosurface' ? 'isosurface' : 'surface',
      expr: c.source, space, depth: c.mode === 'isosurface' ? (preview ? 3 : 4) : (preview ? 4 : 6),
      budget: preview ? 1700 : Math.max(4600, refinement),
      ...(c.mode === 'parametric_surface' ? { x: c.horizontal, y: c.third, parameter_domain: [c.domain, c.domainV] } : {}),
    },
  }));
  plots.run(tasks, ({ key, response }) => {
    if (expected !== revision) return;
    diagnose(key, response);
    const c = cells.find(c => c.id === key)!;
    if (response.status === 'approximate_mesh') {
      meshes.set(key, { request: tasks.find(task => task.key === key)!.request, response,
        layer: { color: c.color, mesh: response.result as Mesh, mapGaps: c.mode === 'surface' } });
    } else meshes.delete(key);
    spatial.setLayers([...meshes.values()].map(item => item.layer), showUncertainty);
  }, () => {
    if (expected !== revision) return;
    if (preview && tasks.length) {
      status('refining', 'Mesh preview · refining');
      computeTimer = window.setTimeout(() => computeSpatial(expected, false), 55);
    } else {
      settled = true;
      const count = [...meshes.values()].reduce((sum, item) => sum + item.layer.mesh.faces.length, 0);
      status('settled', count ? count + ' triangles · approximate topology' : 'No mesh geometry');
    }
  }, message => status('error', message));
}

function computeTable(): void {
  const c = active();
  const parent = id('table-panel');
  status('pending', 'Computing current table');
  if (c && dataMode(c.mode)) {
    const op = c.mode === 'regression' ? 'regression' : 'probability_plot';
    const request = { ...environment(history.current), op, expr: c.source, degree: c.degree };
    parent.replaceChildren(el('p', 'Calculating current data…'));
    tables.run([{ key: c.id, request }], ({ response }) => {
      diagnose(c.id, response);
      if (response.result) {
        if (c.mode === 'regression') regression(parent, response.result);
        else probability(parent, response.result);
      } else parent.replaceChildren(el('p', response.message ?? response.status));
      status('settled', 'Data analysis · current');
    }, () => {}, message => { parent.replaceChildren(el('p', message)); status('error', message); });
    return;
  }
  if (!c || c.mode !== 'explicit') {
    tables.cancel();
    parent.replaceChildren(el('p', 'Select an explicit function or a data expression.'));
    status('settled', 'No expression selected for table');
    return;
  }
  parent.replaceChildren(el('p', 'Sampling current expression…'));
  const { x } = history.current.view;
  const samples = Array.from({ length: 17 }, (_, i) => x[0] + (x[1] - x[0]) * i / 16);
  const request = { ...environment(history.current), op: 'table', expr: c.source, samples };
  tables.run([{ key: c.id, request }], ({ response }) => {
    if (!Array.isArray(response.result)) {
      parent.replaceChildren(el('p', response.message ?? response.status));
      status('settled', 'Table request returned ' + response.status);
      return;
    }
    const table = el('table');
    const caption = el('caption', c.source);
    const header = el('tr');
    for (const name of ['x', 'Computed value']) {
      const label = el('th', name);
      label.scope = 'col';
      header.append(label);
    }
    const head = el('thead');
    head.append(header);
    const rows = el('tbody');
    for (const { x, value } of response.result) {
      const row = el('tr');
      row.append(el('td', coordinate(x)), el('td', textResult(value)));
      rows.append(row);
    }
    table.append(caption, head, rows);
    parent.replaceChildren(table);
    status('settled', 'Sample table · current');
  }, () => {}, message => { parent.replaceChildren(el('p', message)); status('error', message); });
}

function switchTab(next: Tab): void {
  tab = next;
  id('graph-panel').hidden = next !== 'graph';
  id('table-panel').hidden = next !== 'table';
  id('spatial-panel').hidden = next !== 'spatial';
  for (const key of ['graph', 'table', 'spatial'] as const) id(key + '-tab').setAttribute('aria-pressed', String(next === key));
  id('view-title').textContent = next === 'graph' ? 'Graph canvas' : next === 'table' ? 'Sample table' : 'Spatial canvas';
  id('navigation-help').textContent = next === 'spatial'
    ? 'Drag to rotate. Focus the mesh for arrow and + / − keys.'
    : 'Drag to pan. Scroll to zoom. Focus the graph for arrow and + / − keys.';
  tables.cancel();
  schedule(false);
}

function calculate(): void {
  const c = active();
  if (!c || c.mode === 'definition') return;
  const a = Number(lower.value), b = Number(upper.value);
  const op = operation.value;
  const fields = op === 'evaluate' ? { bindings: { x: lower.value } }
    : op === 'derivative' ? { at: a }
    : op === 'improper' ? { lower: a }
    : op === 'improper_zero' ? { lower: a, upper: b }
    : op === 'quantile' ? { probability: lower.value }
    : op === 'polynomial' ? { domain: [lower.value, upper.value] }
    : { domain: [a, b] };
  if (op !== 'polynomial' && op !== 'evaluate' && op !== 'quantile' && op !== 'simplify' && (!Number.isFinite(a) || op !== 'derivative' && op !== 'improper' && (!Number.isFinite(b) || b <= a))) {
    explain({ status: 'error', message: 'Enter a finite point or increasing interval.' });
    return;
  }
  const feature = ['intersection', 'extrema', 'tangency', 'area_between'].includes(op);
  const other = history.current.cells.find(item => item.id === second.value);
  const request = { ...environment(history.current), op: feature ? 'feature' : op === 'improper_zero' ? 'improper' : op, expr: c.source, ...fields, bits: 128, budget: 8192,
    ...(feature ? { kind: op, other: other?.source } : {}) };
  evidence.replaceChildren(badge('pending', 'neutral'), el('p', 'Computing in a worker…'));
  setDisabled('calculate', true);
  snapshot = null;
  id('save-certificate').hidden = true;
  analysis.run([{ key: c.id, request }], ({ response }) => {
    explain(response);
    if (!response.message) {
      snapshot = { request, claim: response };
      id('save-certificate').hidden = false;
    }
  }, () => setDisabled('calculate', false), message => {
    setDisabled('calculate', false);
    explain({ status: 'error', message });
  });
}

function download(name: string, content: Blob | string, type = 'application/json'): void {
  const blob = typeof content === 'string' ? new Blob([content], { type }) : content;
  const link = el('a');
  link.href = URL.createObjectURL(blob);
  link.download = name;
  link.click();
  window.setTimeout(() => URL.revokeObjectURL(link.href), 1_000);
}

function exportData() {
  if (tab === 'spatial') return { format: 'lucent-mesh-v1', document: history.current, camera: spatial.camera(),
    layers: [...meshes.values()].map(({ request, response }) => ({ request, claim: response })) };
  return { format: 'lucent-plot-v1', document: history.current, viewport: liveView, uncertainty: showUncertainty,
    pixels: [renderer.width, renderer.height], layers: [...layers.values()].map(({ request, response }) => ({ request, claim: response })) };
}

function viewport(next: View, commit: boolean): void {
  try { liveView = { x: domain(next.x), y: domain(next.y) }; } catch { return; }
  if (commit && history.edit(doc => { doc.view = structuredClone(liveView); })) {
    persist();
    syncButtons();
  }
  schedule(false);
}

function zoom(factor: number, anchor: Point = [renderer.width / 2, renderer.height / 2]): void {
  const [x, y] = renderer.world(anchor);
  viewport({ x: liveView.x.map(v => x + (v - x) * factor) as Point,
    y: liveView.y.map(v => y + (v - y) * factor) as Point }, true);
}

function pan(dx: number, dy: number, origin = liveView, commit = true): void {
  const x = dx / renderer.width * (origin.x[1] - origin.x[0]);
  const y = dy / renderer.height * (origin.y[1] - origin.y[0]);
  viewport({ x: [origin.x[0] - x, origin.x[1] - x], y: [origin.y[0] + y, origin.y[1] + y] }, commit);
}

function replace(doc: Workspace): void {
  history.edit(draft => Object.assign(draft, doc));
  activeCell = doc.cells.find(c => c.mode !== 'definition')?.id ?? '';
  persist();
  renderDocument();
  schedule(true);
}

function undo(redo: boolean): void {
  if (!(redo ? history.redo() : history.undo())) return;
  persist();
  renderDocument();
  schedule(true);
}

id('add').onclick = () => {
  const added = cell(tab === 'spatial' ? 'sin(x)*cos(y)' : 'sin(x)', tab === 'spatial' ? 'surface' : 'explicit', COLORS[history.current.cells.length % COLORS.length]);
  activeCell = added.id;
  update(doc => { doc.cells.push(added); }, { rebuild: true });
  expressionInputs.get(added.id)?.select();
};
id('undo').onclick = () => undo(false);
id('redo').onclick = () => undo(true);
title.onchange = () => update(doc => { doc.title = title.value; }, { recompute: false });
selected.onchange = () => selectCell(selected.value);
angle.onchange = () => update(doc => { doc.angle = angle.value as Angle; });
operation.onchange = second.onchange = lower.oninput = upper.oninput = invalidateEvidence;
id('calculate').onclick = calculate;
id('save-certificate').onclick = () => {
  if (snapshot) download('lucent-certificate.json', JSON.stringify({ op: 'verify', ...snapshot }, null, 2));
};
id('graph-tab').onclick = () => switchTab('graph');
id('table-tab').onclick = () => switchTab('table');
id('spatial-tab').onclick = () => switchTab('spatial');
id('spatial-in').onclick = () => spatial.zoom(1.2);
id('spatial-out').onclick = () => spatial.zoom(1 / 1.2);
id('spatial-reset').onclick = () => spatial.reset();
for (const key of ['zmin', 'zmax']) {
  id<HTMLInputElement>(key).onchange = () => {
    const a = id<HTMLInputElement>('zmin');
    const b = id<HTMLInputElement>('zmax');
    try {
      const next = domain([Number(a.value), Number(b.value)]);
      update(doc => { doc.z = next; });
    } catch {
      [a.value, b.value] = history.current.z.map(String);
    }
  };
}
id('zoom-in').onclick = () => zoom(0.75);
id('zoom-out').onclick = () => zoom(1 / 0.75);
id('reset-view').onclick = () => viewport(HOME, true);
id('refine').onclick = () => { refinement = Math.min(refinement * 2, 32768); schedule(false, false); };
id('uncertainty').onclick = () => {
  showUncertainty = !showUncertainty;
  id('uncertainty').setAttribute('aria-pressed', String(showUncertainty));
  draw();
  spatial.setLayers([...meshes.values()].map(item => item.layer), showUncertainty);
};
id('parameter-add').onclick = () => {
  const names = new Set(history.current.parameters.map(p => p.name));
  const name = ['a', 'b', 'c', 'd', 'h', 'k', 'p', 'q', 'r', 'w', 'aa', 'ab', 'ac', 'ad', 'ae', 'af', 'ag']
    .find(value => !names.has(value))!;
  update(doc => { doc.parameters.push({ name, value: 1, min: -5, max: 5, step: 0.05 }); }, { rebuild: true });
  id('parameter-' + name).focus();
};

for (const key of ['x', 't', 'π', '^', '(', ')', 'sin(', 'cos(', 'sqrt(', 'log(', '≤', '≥', ':=']) {
  const button = el('button', key);
  button.setAttribute('aria-label', 'Insert ' + key);
  button.onclick = () => {
    const input = expressionInputs.get(activeCell);
    if (!input) return;
    input.setRangeText(key, input.selectionStart ?? input.value.length, input.selectionEnd ?? input.value.length, 'end');
    input.dispatchEvent(new Event('input'));
    input.focus();
  };
  id('math-keys').append(button);
}

const gallery = id<HTMLDialogElement>('gallery-dialog');
id('gallery-button').onclick = () => gallery.showModal();
for (const item of GALLERY) {
  const button = el('button', item.title);
  button.append(el('small', item.lesson));
  button.onclick = () => {
    replace(loadExploration(item));
    switchTab(item.sketches.some(sketch => sketch.mode && surfaceMode(sketch.mode)) ? 'spatial'
      : item.sketches.some(sketch => sketch.mode && dataMode(sketch.mode)) ? 'table' : 'graph');
    if (item.investigation) {
      operation.value = item.investigation.op;
      [lower.value, upper.value] = item.investigation.bounds;
    }
    gallery.close();
  };
  id('gallery-list').append(button);
}

const file = id<HTMLInputElement>('file');
id('import-button').onclick = () => file.click();
file.onchange = async () => {
  const input = file.files?.[0];
  if (!input) return;
  try {
    if (input.size > LIMIT) throw Error('Document exceeds 192 KiB');
    replace(decode(await input.text()));
  } catch (error) {
    saveStatus.textContent = error instanceof Error ? error.message : 'Document could not be read';
  } finally { file.value = ''; }
};

const exports = id<HTMLDialogElement>('export-dialog');
id('export-button').onclick = () => {
  for (const button of exports.querySelectorAll<HTMLButtonElement>('[data-export]')) {
    const kind = button.dataset.export;
    button.disabled = kind !== 'document' && (!settled || tab === 'table'
      || (tab === 'spatial' ? !meshes.size || kind === 'svg' : !layers.size));
  }
  exports.showModal();
};
for (const button of exports.querySelectorAll<HTMLButtonElement>('[data-export]')) {
  button.onclick = async () => {
    if (button.dataset.export === 'document') download('lucent-workspace.json', JSON.stringify(history.current, null, 2));
    else if (button.dataset.export === 'plot') download('lucent-plot.json', JSON.stringify(exportData()));
    else if (button.dataset.export === 'svg') download('lucent-plot.svg', renderer.svg(exportData()), 'image/svg+xml');
    else {
      const png = tab === 'spatial' ? await new Promise<Blob | null>(resolve => spatial.canvas.toBlob(resolve)) : await renderer.png();
      if (png) {
        try { download('lucent-plot.png', await annotatePng(png, exportData())); }
        catch (error) { saveStatus.textContent = error instanceof Error ? error.message : 'PNG export failed'; }
      }
    }
    exports.close();
  };
}

const pointer = (event: PointerEvent | WheelEvent): Point => {
  const rect = plot.getBoundingClientRect();
  return [event.clientX - rect.left, event.clientY - rect.top];
};
let drag: { id: number; point: Point; view: View } | null = null;
plot.addEventListener('pointerdown', event => {
  if (event.button !== 0) return;
  drag = { id: event.pointerId, point: pointer(event), view: structuredClone(liveView) };
  plot.setPointerCapture(event.pointerId);
});
plot.addEventListener('pointermove', event => {
  const point = pointer(event);
  const world = renderer.world(point);
  id('coordinate').textContent = world.map(coordinate).join(', ');
  if (drag?.id === event.pointerId) pan(point[0] - drag.point[0], point[1] - drag.point[1], drag.view, false);
});
function release(event: PointerEvent): void {
  if (drag?.id !== event.pointerId) return;
  const point = pointer(event);
  const moved = Math.hypot(point[0] - drag.point[0], point[1] - drag.point[1]) > 3;
  drag = null;
  if (moved) viewport(liveView, true);
  else { selection = renderer.snap(point); draw(); }
}
plot.addEventListener('pointerup', release);
plot.addEventListener('pointercancel', release);
plot.addEventListener('wheel', event => {
  event.preventDefault();
  zoom(Math.exp(Math.max(-100, Math.min(100, event.deltaY)) * 0.003), pointer(event));
}, { passive: false });
plot.addEventListener('keydown', event => {
  const moves: Record<string, Point> = { ArrowLeft: [45, 0], ArrowRight: [-45, 0], ArrowUp: [0, 45], ArrowDown: [0, -45] };
  if (moves[event.key]) pan(...moves[event.key]);
  else if (event.key === '+' || event.key === '=') zoom(0.75);
  else if (event.key === '-') zoom(1 / 0.75);
  else if (event.key === 'Home') viewport(HOME, true);
  else return;
  event.preventDefault();
});
document.addEventListener('keydown', event => {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'z') {
    if (event.target instanceof HTMLInputElement && event.target.type === 'text') return;
    event.preventDefault();
    undo(event.shiftKey);
  }
});

renderDocument();
renderer.resize(plot.clientWidth, plot.clientHeight);
renderer.draw(liveView, [], showUncertainty, null);
new ResizeObserver(() => {
  if (tab === 'graph' && renderer.resize(plot.clientWidth, plot.clientHeight)) schedule(false);
}).observe(plot);
schedule(true);
window.addEventListener('pagehide', () => {
  try { save(localStorage, history.current); } catch { /* the next load handles recovery */ }
});
