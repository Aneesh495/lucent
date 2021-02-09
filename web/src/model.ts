export const COLORS = ['#255e50', '#3f5ca1', '#b05b36', '#875091', '#187c89', '#906b16'];
export const MODES = {
  explicit: 'Explicit y = f(x)', implicit: 'Implicit equation', region: 'Inequality region',
  parametric: 'Parametric curve', polar: 'Polar curve', scalar: 'Scalar calculation',
  regression: 'Regression data', probability: 'Normal probability data',
  surface: 'Height surface', parametric_surface: 'Parametric surface', isosurface: 'Isosurface',
  definition: 'Definition', complex: 'Complex coloring', field: 'Vector field', ode: 'ODE trajectory',
} as const;
export type Mode = keyof typeof MODES;
export type Point = [number, number];
export type Domain = [number, number];
export type Angle = 'radians' | 'degrees';

export interface View {
  x: Domain;
  y: Domain;
}

export interface Cell {
  id: string;
  mode: Mode;
  source: string;
  horizontal: string;
  domain: Domain;
  visible: boolean;
  initial: Point;
  event: string;
  third: string;
  domainV: Domain;
  degree: number;
  color: string;
  style: 'solid' | 'dashed';
  folder: string;
}

export interface Parameter {
  name: string;
  value: number;
  min: number;
  max: number;
  step: number;
}

export interface Workspace {
  version: 2;
  title: string;
  cells: Cell[];
  parameters: Parameter[];
  angle: Angle;
  view: View;
  z: Domain;
}

const STORAGE = 'lucent.workspace.v2';
const BACKUP = 'lucent.recovery.v2';
const OLD_STORAGE = 'lucent.workspace.v1';
export const HOME: View = { x: [-8, 8], y: [-5, 5] };
export const LIMIT = 196_608;

export function cell(source: string, mode: Mode = 'explicit', color = COLORS[0]): Cell {
  return {
    id: globalThis.crypto.randomUUID(), mode, source, horizontal: 'cos(t)', domain: [0, 2 * Math.PI],
    visible: true, initial: [1, 0], event: '', degree: 1, third: 'sin(v)', domainV: [0, 2 * Math.PI], color, style: 'solid', folder: '',
  };
}

export function initial(): Workspace {
  return {
    version: 2,
    title: 'Function study',
    cells: [
      cell('a sin(x)', 'explicit', COLORS[0]),
      cell('1 / (x - 1)', 'explicit', COLORS[1]),
      cell('x^2 / 8 - 2', 'explicit', COLORS[2]),
    ],
    parameters: [{ name: 'a', value: 1.5, min: 0.1, max: 3.5, step: 0.05 }],
    angle: 'radians',
    view: structuredClone(HOME),
    z: [-4, 4],
  };
}

function record(input: unknown): Record<string, unknown> {
  if (!input || typeof input !== 'object' || Array.isArray(input)) throw Error('Expected a workspace object');
  return input as Record<string, unknown>;
}

function string(input: unknown, max: number, description: string): string {
  if (typeof input !== 'string' || input.length > max) throw Error('Invalid ' + description);
  return input;
}

function pair(input: unknown): Point {
  if (!Array.isArray(input) || input.length !== 2 || !input.every(Number.isFinite)) throw Error('Invalid coordinate pair');
  return [input[0], input[1]];
}

export function domain(input: unknown): Domain {
  if (!Array.isArray(input) || input.length !== 2 || !input.every(Number.isFinite)
    || input[1] - input[0] < 1e-280 || !Number.isFinite(input[1] - input[0])) throw Error('Invalid coordinate range');
  return [input[0], input[1]];
}

export function decode(text: string): Workspace {
  if (new TextEncoder().encode(text).length > LIMIT) throw Error('Document exceeds 192 KiB');
  const raw = record(JSON.parse(text));
  if (raw.version !== 1 && raw.version !== 2) throw Error('Unsupported document version');
  const migrated = raw.version === 1;
  const cells = raw.cells;
  const params = raw.parameters ?? [];
  if (!Array.isArray(cells) || cells.length > 40 || !Array.isArray(params) || params.length > 16) {
    throw Error('Document exceeds expression or parameter limits');
  }
  const ids = new Set<string>();
  const normalized = cells.map((entry, index): Cell => {
    const c = record(entry);
    const id = string(c.id ?? (migrated ? 'migrated-' + index : undefined), 64, 'cell identity');
    if (!id || ids.has(id)) throw Error('Missing or duplicate cell identity');
    ids.add(id);
    const mode = c.mode ?? 'explicit';
    if (typeof mode !== 'string' || !Object.hasOwn(MODES, mode)) throw Error('Unknown cell mode');
    const color = c.color ?? COLORS[index % COLORS.length];
    if (typeof color !== 'string' || !/^#[0-9a-f]{6}$/i.test(color)) throw Error('Invalid plot color');
    const style = c.style ?? 'solid';
    if (style !== 'solid' && style !== 'dashed') throw Error('Invalid line style');
    if (c.visible !== undefined && typeof c.visible !== 'boolean') throw Error('Invalid plot visibility');
    return {
      id, mode: mode as Mode, color, style,
      source: string(c.source ?? c.expr, 8192, 'expression text'),
      horizontal: string(c.horizontal ?? 'cos(t)', 8192, 'horizontal expression'),
      domain: domain(c.domain ?? [0, 2 * Math.PI]),
      initial: pair(c.initial ?? [1, 0]),
      event: string(c.event ?? '', 8192, 'event expression'),
      third: string(c.third ?? 'sin(v)', 8192, 'third coordinate'),
      domainV: domain(c.domainV ?? [0, 2 * Math.PI]),
      degree: typeof c.degree === 'number' && Number.isInteger(c.degree) && c.degree >= 0 && c.degree <= 6 ? c.degree : 1,
      folder: string(c.folder ?? '', 40, 'folder'),
      visible: c.visible !== false,
    };
  });
  const names = new Set<string>();
  const parameters = params.map((entry): Parameter => {
    const p = record(entry);
    const name = string(p.name, 32, 'parameter name');
    if (!/^[A-Za-z_][A-Za-z_0-9]*$/.test(name) || names.has(name)) throw Error('Invalid or duplicate parameter');
    names.add(name);
    const range = domain([p.min, p.max]);
    const value = p.value;
    const step = p.step;
    if (typeof value !== 'number' || value < range[0] || value > range[1] || !Number.isFinite(value)
      || typeof step !== 'number' || !Number.isFinite(step) || step <= 0) throw Error('Invalid parameter range or step');
    return { name, value, min: range[0], max: range[1], step };
  });
  const angle = raw.angle ?? 'radians';
  if (angle !== 'radians' && angle !== 'degrees') throw Error('Unknown angle convention');
  const view = record(raw.view ?? HOME);
  return { version: 2, title: string(raw.title ?? 'Untitled', 80, 'title'), cells: normalized, parameters, angle,
    view: { x: domain(view.x), y: domain(view.y) }, z: domain(raw.z ?? [-4, 4]) };
}

export function load(storage: Storage): { workspace: Workspace; message: string } {
  try {
    const primary = storage.getItem(STORAGE) ?? storage.getItem(OLD_STORAGE);
    if (!primary) return { workspace: initial(), message: 'Local document' };
    try {
      return { workspace: decode(primary), message: 'Restored local document' };
    } catch {
      const backup = storage.getItem(BACKUP);
      if (backup) return { workspace: decode(backup), message: 'Recovered the last valid autosave' };
    }
    return { workspace: initial(), message: 'Autosave unreadable. Opened a new document.' };
  } catch {
    return { workspace: initial(), message: 'Local storage unavailable' };
  }
}

export function save(storage: Storage, workspace: Workspace): void {
  const serialized = JSON.stringify(workspace);
  decode(serialized);
  const previous = storage.getItem(STORAGE);
  if (previous !== null) {
    let valid = false;
    try { decode(previous); valid = true; } catch { /* keep the last valid backup */ }
    if (valid) storage.setItem(BACKUP, previous);
  }
  storage.setItem(STORAGE, serialized);
}

export class History {
  private undoStack: Workspace[] = [];
  private redoStack: Workspace[] = [];

  constructor(public current: Workspace) {}

  get canUndo(): boolean { return this.undoStack.length > 0; }
  get canRedo(): boolean { return this.redoStack.length > 0; }

  edit(change: (draft: Workspace) => void, before = this.current): boolean {
    const next = structuredClone(this.current);
    change(next);
    if (JSON.stringify(next) === JSON.stringify(before)) return false;
    this.undoStack.push(before);
    if (this.undoStack.length > 100) this.undoStack.shift();
    this.current = next;
    this.redoStack.length = 0;
    return true;
  }

  undo(): boolean {
    const previous = this.undoStack.pop();
    if (!previous) return false;
    this.redoStack.push(this.current);
    this.current = previous;
    return true;
  }

  redo(): boolean {
    const next = this.redoStack.pop();
    if (!next) return false;
    this.undoStack.push(this.current);
    this.current = next;
    return true;
  }
}

export function environment(doc: Workspace): Record<string, unknown> {
  return {
    angle: doc.angle,
    definitions: doc.cells.filter(c => c.mode === 'definition' && c.source.trim()).map(c => c.source),
    parameters: Object.fromEntries(doc.parameters.map(p => [p.name, p.value.toString()])),
  };
}
