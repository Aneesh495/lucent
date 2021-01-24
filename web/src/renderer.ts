import type { Cell, Domain, Point, View } from './model';

export interface Tile {
  kind: 'segment' | 'crossing' | 'offscreen' | 'excluded' | 'unresolved' | 'empty' | 'inside';
  segment?: [Point, Point];
  domain?: Domain;
  x?: Domain;
  y?: Domain;
  error_px?: number;
  reason?: string;
}
export interface Geometry { cells: Tile[]; processed: number; dimension?: [number[], number[]] }
export interface Raster {
  size: [number, number]; rgba: number[]; view: View; undefined_samples: number;
}
export interface Arrow { origin: Point; vector: Point }
export interface Layer { cell: Cell; geometry?: Geometry; raster?: Raster; arrows?: Arrow[] }
interface Line { a: Point; b: Point; color: string; dashed: boolean }
interface Rect { x: number; y: number; w: number; h: number; fill: string; hatch: boolean }

function rgb(hex: string): [number, number, number] {
  return [1, 3, 5].map(i => Number.parseInt(hex.slice(i, i + 2), 16) / 255) as [number, number, number];
}

class Gpu {
  readonly gl: WebGL2RenderingContext;
  private readonly program: WebGLProgram;
  private readonly buffer: WebGLBuffer;
  private readonly array: WebGLVertexArrayObject;

  constructor(canvas: HTMLCanvasElement) {
    const gl = canvas.getContext('webgl2', { alpha: true, antialias: true, preserveDrawingBuffer: true });
    if (!gl) throw Error('WebGL2 unavailable');
    this.gl = gl;
    const shader = (type: number, source: string) => {
      const s = gl.createShader(type);
      if (!s) throw Error('Shader allocation failed');
      gl.shaderSource(s, source);
      gl.compileShader(s);
      if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw Error(gl.getShaderInfoLog(s) ?? 'Shader compile failed');
      return s;
    };
    const vs = shader(gl.VERTEX_SHADER, `#version 300 es
      layout(location=0) in vec2 position;
      layout(location=1) in vec3 color;
      out vec3 pigment;
      void main() { gl_Position = vec4(position, 0.0, 1.0); pigment = color; }
    `);
    const fs = shader(gl.FRAGMENT_SHADER, `#version 300 es
      precision mediump float;
      in vec3 pigment;
      out vec4 fragment;
      void main() { fragment = vec4(pigment, 1.0); }
    `);
    const program = gl.createProgram();
    const buffer = gl.createBuffer();
    const array = gl.createVertexArray();
    if (!program || !buffer || !array) throw Error('Plot buffer allocation failed');
    gl.attachShader(program, vs);
    gl.attachShader(program, fs);
    gl.linkProgram(program);
    gl.deleteShader(vs);
    gl.deleteShader(fs);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw Error(gl.getProgramInfoLog(program) ?? 'Shader link failed');
    this.program = program;
    this.buffer = buffer;
    this.array = array;
    gl.bindVertexArray(array);
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 20, 0);
    gl.enableVertexAttribArray(1);
    gl.vertexAttribPointer(1, 3, gl.FLOAT, false, 20, 8);
  }

  render(lines: Line[], width: number, height: number): void {
    const gl = this.gl;
    const vertices: number[] = [];
    const vertex = (p: Point, color: [number, number, number]) =>
      vertices.push(p[0] / width * 2 - 1, 1 - p[1] / height * 2, ...color);
    for (const line of lines) {
      const color = rgb(line.color);
      for (const [a, b] of dashes(line)) {
        const dx = b[0] - a[0];
        const dy = b[1] - a[1];
        const len = Math.hypot(dx, dy);
        if (len < 0.01) continue;
        const shift: Point = [-dy / len * 1.15, dx / len * 1.15];
        const offset = (p: Point, sign: number): Point => [p[0] + shift[0] * sign, p[1] + shift[1] * sign];
        const p = offset(a, -1), q = offset(a, 1), r = offset(b, -1), s = offset(b, 1);
        for (const corner of [p, q, r, q, s, r]) vertex(corner, color);
      }
    }
    gl.viewport(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.useProgram(this.program);
    gl.bindVertexArray(this.array);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(vertices), gl.STREAM_DRAW);
    gl.drawArrays(gl.TRIANGLES, 0, vertices.length / 5);
  }
}

function* dashes(line: Line): Generator<[Point, Point]> {
  if (!line.dashed) { yield [line.a, line.b]; return; }
  const [a, b] = [line.a, line.b];
  const len = Math.hypot(b[0] - a[0], b[1] - a[1]);
  const position = (t: number): Point => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
  for (let x = 0; x < len; x += 10) {
    yield [position(x / len), position(Math.min(x + 6, len) / len)];
  }
}

function clipped(a: Point, b: Point, width: number, height: number): [Point, Point] | null {
  const delta: Point = [b[0] - a[0], b[1] - a[1]];
  if (![...a, ...b, ...delta].every(Number.isFinite)) return null;
  let t0 = 0, t1 = 1;
  for (const [p, q] of [
    [-delta[0], a[0] + 3], [delta[0], width + 3 - a[0]],
    [-delta[1], a[1] + 3], [delta[1], height + 3 - a[1]],
  ]) {
    if (p === 0) {
      if (q < 0) return null;
    } else if (p < 0) {
      t0 = Math.max(t0, q / p);
    } else {
      t1 = Math.min(t1, q / p);
    }
    if (t0 > t1) return null;
  }
  const at = (t: number): Point => [a[0] + t * delta[0], a[1] + t * delta[1]];
  return [at(t0), at(t1)];
}

export function ticks(range: Domain, target: number): number[] {
  const raw = (range[1] - range[0]) / target;
  const power = 10 ** Math.floor(Math.log10(raw));
  const factor = [1, 2, 5, 10].find(x => x * power >= raw) ?? 10;
  const step = factor * power;
  if (!Number.isFinite(step) || step === 0) return [];
  const start = Math.ceil(range[0] / step);
  const values = [];
  for (let i = 0; i < 100; i++) {
    const x = (start + i) * step;
    if (x > range[1] || (i && x <= values[i - 1])) break;
    values.push(x);
  }
  return values;
}

export function coordinate(value: number): string {
  if (value === 0) return '0';
  if (Math.abs(value) < 1e-4 || Math.abs(value) >= 1e6) return value.toExponential(3);
  return Number(value.toPrecision(7)).toString();
}

function axisUnit(units: number[]): string {
  const labels = ['m', 's', 'kg', 'rad'];
  return units.map((n, i) => n === 0 ? '' : labels[i] + (n === 1 ? '' : '^' + n)).filter(Boolean).join('·');
}

export class Renderer {
  width = 1;
  height = 1;
  private scale = 1;
  private readonly grid: CanvasRenderingContext2D;
  private readonly overlay: CanvasRenderingContext2D;
  private readonly fallback: CanvasRenderingContext2D | null;
  private gpu: Gpu | null;
  private lines: Line[] = [];
  private rects: Rect[] = [];
  private view!: View;
  private raster: Raster | null = null;
  private rasterCanvas: HTMLCanvasElement | null = null;
  private selection: Point | null = null;
  private units = ['', ''];
  readonly backend: string;

  constructor(private readonly gridCanvas: HTMLCanvasElement, private readonly geometryCanvas: HTMLCanvasElement,
    private readonly overlayCanvas: HTMLCanvasElement) {
    this.grid = gridCanvas.getContext('2d')!;
    this.overlay = overlayCanvas.getContext('2d')!;
    try { this.gpu = new Gpu(geometryCanvas); } catch { this.gpu = null; }
    this.fallback = this.gpu ? null : geometryCanvas.getContext('2d');
    this.backend = this.gpu ? 'WebGL2' : 'Canvas 2D';
  }

  resize(width: number, height: number): boolean {
    width = Math.max(1, Math.floor(width));
    height = Math.max(1, Math.floor(height));
    const scale = Math.min(window.devicePixelRatio || 1, 2);
    if (width === this.width && height === this.height && scale === this.scale) return false;
    this.width = width;
    this.height = height;
    this.scale = scale;
    for (const canvas of [this.gridCanvas, this.geometryCanvas, this.overlayCanvas]) {
      canvas.width = width * scale;
      canvas.height = height * scale;
    }
    for (const ctx of [this.grid, this.overlay, this.fallback]) ctx?.setTransform(scale, 0, 0, scale, 0, 0);
    return true;
  }

  project(point: Point): Point {
    const { x, y } = this.view;
    return [
      (point[0] - x[0]) / (x[1] - x[0]) * this.width,
      (y[1] - point[1]) / (y[1] - y[0]) * this.height,
    ];
  }

  world(p: Point): Point {
    const { x, y } = this.view;
    return [
      x[0] + p[0] / this.width * (x[1] - x[0]),
      y[1] - p[1] / this.height * (y[1] - y[0]),
    ];
  }

  draw(view: View, layers: Layer[], uncertainty: boolean, selection: Point | null): void {
    this.view = view;
    this.selection = selection;
    this.lines = [];
    this.rects = [];
    const raster = [...layers].reverse().find(layer => layer.raster)?.raster ?? null;
    if (this.raster !== raster) {
      this.raster = raster;
      this.rasterCanvas = raster ? document.createElement('canvas') : null;
      if (this.rasterCanvas && raster) {
        const [w, h] = raster.size;
        this.rasterCanvas.width = w;
        this.rasterCanvas.height = h;
        const image = new ImageData(new Uint8ClampedArray(raster.rgba), w, h);
        this.rasterCanvas.getContext('2d')?.putImageData(image, 0, 0);
      }
    }
    this.units = [0, 1].map(axis => {
      const set = new Set(layers.map(layer => axisUnit(layer.geometry?.dimension?.[axis] ?? [])));
      return set.size === 1 ? [...set][0] : '';
    });
    for (const { cell, geometry, arrows } of layers) {
      if (arrows) this.drawArrows(arrows, cell.color);
      if (!geometry) continue;
      for (const tile of geometry.cells) {
        if (tile.segment) {
          const segment = clipped(this.project(tile.segment[0]), this.project(tile.segment[1]), this.width, this.height);
          if (segment) this.lines.push({ a: segment[0], b: segment[1], color: cell.color, dashed: cell.style === 'dashed' });
        }
        if (tile.kind === 'inside' || uncertainty && ['unresolved', 'excluded', 'crossing'].includes(tile.kind)) {
          const x = tile.x ?? (cell.mode === 'explicit' ? tile.domain : undefined);
          if (!x) continue;
          const y = tile.y ?? view.y;
          const a = this.project([x[0], y[1]]), b = this.project([x[1], y[0]]);
          const left = Math.max(0, a[0]), top = Math.max(0, a[1]);
          const right = Math.min(this.width, b[0]), bottom = Math.min(this.height, b[1]);
          if (right < left || bottom < top) continue;
          const hatch = tile.kind === 'unresolved' || tile.kind === 'excluded';
          this.rects.push({ x: left, y: top, w: Math.max(1.5, right - left), h: bottom - top,
            fill: tile.kind === 'inside' ? cell.color + '20' : tile.kind === 'crossing' ? '#566cb011' : '#aa673628', hatch });
        }
      }
    }
    this.drawGrid();
    this.drawLines();
    this.drawSelection();
  }

  private drawArrows(arrows: Arrow[], color: string): void {
    const xscale = this.width / (this.view.x[1] - this.view.x[0]);
    const yscale = this.height / (this.view.y[1] - this.view.y[0]);
    for (const { origin, vector } of arrows) {
      const p = this.project(origin);
      if (p[0] < 0 || p[0] > this.width || p[1] < 0 || p[1] > this.height) continue;
      const dx = vector[0] * xscale, dy = -vector[1] * yscale;
      const norm = Math.hypot(dx, dy);
      if (!(norm > 0) || !Number.isFinite(norm)) continue;
      const ux = dx / norm, uy = dy / norm;
      const span = Math.min(18, 4 + Math.log1p(norm) * 2.3);
      const tip: Point = [p[0] + ux * span, p[1] + uy * span];
      const sides: Point[] = [
        [tip[0] - ux * 4 + uy * 3, tip[1] - uy * 4 - ux * 3],
        [tip[0] - ux * 4 - uy * 3, tip[1] - uy * 4 + ux * 3],
      ];
      this.lines.push({ a: p, b: tip, color, dashed: false });
      for (const side of sides) this.lines.push({ a: side, b: tip, color, dashed: false });
    }
  }

  private labels(): { x: number; y: number; text: string; vertical: boolean; zero: boolean }[] {
    const horizontal = ticks(this.view.x, Math.max(2, this.width / 78));
    const vertical = ticks(this.view.y, Math.max(2, this.height / 59));
    return [
      ...horizontal.map(x => ({ x: this.project([x, this.view.y[0]])[0], y: 0, text: coordinate(x), vertical: true, zero: x === 0 })),
      ...vertical.map(y => ({ x: 0, y: this.project([this.view.x[0], y])[1], text: coordinate(y), vertical: false, zero: y === 0 })),
    ];
  }

  private drawGrid(): void {
    const ctx = this.grid;
    const { width, height } = this;
    ctx.fillStyle = '#fcfdf8';
    ctx.fillRect(0, 0, width, height);
    const picture = this.rasterRectangle();
    if (picture && this.rasterCanvas) ctx.drawImage(this.rasterCanvas, ...picture);
    for (const r of this.rects) {
      ctx.fillStyle = r.fill;
      ctx.fillRect(r.x, r.y, r.w, r.h);
      if (r.hatch && r.w > 2 && r.h > 2) {
        ctx.save();
        ctx.beginPath(); ctx.rect(r.x, r.y, r.w, r.h); ctx.clip();
        ctx.strokeStyle = '#95603633';
        ctx.beginPath();
        for (let s = -r.h; s < r.w; s += 8) {
          ctx.moveTo(r.x + s, r.y + r.h); ctx.lineTo(r.x + s + r.h, r.y);
        }
        ctx.stroke();
        ctx.restore();
      }
    }
    ctx.font = '10px ui-monospace, monospace';
    const axes = this.project([0, 0]);
    const textX = Math.max(7, Math.min(width - 49, axes[0] + 5));
    const textY = Math.max(13, Math.min(height - 28, axes[1] + 14));
    for (const tick of this.labels()) {
      const x = Math.floor(tick.x) + 0.5, y = Math.floor(tick.y) + 0.5;
      ctx.strokeStyle = tick.zero ? '#849b92' : '#e0e7de';
      ctx.lineWidth = 1;
      ctx.beginPath();
      if (tick.vertical) { ctx.moveTo(x, 0); ctx.lineTo(x, height); }
      else { ctx.moveTo(0, y); ctx.lineTo(width, y); }
      ctx.stroke();
      ctx.fillStyle = '#54655e';
      if (tick.vertical && x > 4 && x < width - 31) ctx.fillText(tick.text, x + 4, textY);
      if (!tick.vertical && y > 13 && y < height - 10 && !tick.zero) ctx.fillText(tick.text, textX, y - 4);
    }
    ctx.fillStyle = '#3c5b53';
    ctx.fillText('x' + (this.units[0] ? ' (' + this.units[0] + ')' : ''), width - 48, textY);
    ctx.fillText('y' + (this.units[1] ? ' (' + this.units[1] + ')' : ''), textX, 14);
  }

  private drawLines(): void {
    if (this.gpu && !this.gpu.gl.isContextLost()) {
      this.gpu.render(this.lines, this.width, this.height);
      return;
    }
    const ctx = this.fallback ?? this.overlay;
    if (this.fallback) ctx.clearRect(0, 0, this.width, this.height);
    ctx.lineWidth = 2.2;
    ctx.lineCap = 'round';
    for (const line of this.lines) {
      ctx.strokeStyle = line.color;
      ctx.beginPath();
      for (const [a, b] of dashes(line)) { ctx.moveTo(...a); ctx.lineTo(...b); }
      ctx.stroke();
    }
  }

  private drawSelection(): void {
    const ctx = this.overlay;
    if (!this.gpu?.gl.isContextLost()) ctx.clearRect(0, 0, this.width, this.height);
    if (!this.selection) return;
    const [x, y] = this.project(this.selection);
    if (x < 0 || x > this.width || y < 0 || y > this.height) return;
    ctx.strokeStyle = '#37655f';
    ctx.setLineDash([3, 3]);
    ctx.beginPath();
    ctx.moveTo(x, 0); ctx.lineTo(x, this.height); ctx.moveTo(0, y); ctx.lineTo(this.width, y);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.beginPath();
    ctx.arc(x, y, 4.5, 0, 2 * Math.PI);
    ctx.fillStyle = '#fff';
    ctx.fill();
    ctx.stroke();
  }

  snap(p: Point): Point | null {
    let closest: Point | null = null;
    let distance = 14;
    for (const { a, b } of this.lines) {
      const dx = b[0] - a[0], dy = b[1] - a[1];
      const norm = dx * dx + dy * dy;
      const t = norm ? Math.max(0, Math.min(1, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / norm)) : 0;
      const q: Point = [a[0] + dx * t, a[1] + dy * t];
      const candidate = Math.hypot(p[0] - q[0], p[1] - q[1]);
      if (candidate < distance) { distance = candidate; closest = q; }
    }
    return closest ? this.world(closest) : null;
  }

  private rasterRectangle(): [number, number, number, number] | null {
    if (!this.raster) return null;
    const { x, y } = this.raster.view;
    const a = this.project([x[0], y[1]]);
    const b = this.project([x[1], y[0]]);
    const rect: [number, number, number, number] = [a[0], a[1], b[0] - a[0], b[1] - a[1]];
    return rect.every(Number.isFinite) ? rect : null;
  }

  svg(metadata: unknown): string {
    const esc = (value: string) => value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
    const { width, height } = this;
    const picture = this.rasterRectangle();
    const raster = picture && this.rasterCanvas
      ? `<image x="${picture[0]}" y="${picture[1]}" width="${picture[2]}" height="${picture[3]}" preserveAspectRatio="none" href="${this.rasterCanvas.toDataURL()}"/>`
      : '';
    const rects = this.rects.map(r => `<rect x="${r.x}" y="${r.y}" width="${r.w}" height="${r.h}" fill="${r.fill}"/>`);
    const axes = this.labels().map(t => t.vertical
      ? `<path d="M${t.x} 0V${height}"/><text x="${t.x + 3}" y="${height - 22}">${t.text}</text>`
      : `<path d="M0 ${t.y}H${width}"/><text x="5" y="${t.y - 3}">${t.text}</text>`);
    const lines = this.lines.map(l => `<path d="M${l.a.join(' ')}L${l.b.join(' ')}" fill="none" stroke="${l.color}" stroke-width="2"${l.dashed ? ' stroke-dasharray="6 4"' : ''}/>`);
    return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" width="${width}" height="${height}">
      <title>Lucent plot</title><metadata>${esc(JSON.stringify(metadata))}</metadata>
      <rect width="100%" height="100%" fill="#fcfdf8"/>${raster}${rects.join('')}
      <g stroke="#dde5dc" font-size="10" font-family="monospace">${axes.join('')}</g>${lines.join('')}</svg>`;
  }

  png(): Promise<Blob | null> {
    const canvas = document.createElement('canvas');
    canvas.width = this.gridCanvas.width;
    canvas.height = this.gridCanvas.height;
    const ctx = canvas.getContext('2d')!;
    for (const layer of [this.gridCanvas, this.geometryCanvas, this.overlayCanvas]) ctx.drawImage(layer, 0, 0);
    return new Promise(resolve => canvas.toBlob(resolve));
  }
}
