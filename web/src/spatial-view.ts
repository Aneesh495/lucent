import type { Domain, Point, View } from './model';

export type Vec3 = [number, number, number];

export interface Space extends View { z: Domain }
export interface Face { vertices: [Vec3, Vec3, Vec3]; normal: Vec3 }
export interface Mesh {
  faces: Face[];
  gaps: { domain: Domain[]; reason: string; excluded: boolean }[];
  processed: number;
  interval_excluded: number;
  space: Space;
}
export interface SurfaceLayer { color: string; mesh: Mesh; mapGaps: boolean }

interface Projection { x: number; y: number; depth: number }

export class SpatialCanvas {
  yaw = -0.64;
  pitch = 0.62;
  magnification = 1;
  private width = 1;
  private height = 1;
  private readonly ctx: CanvasRenderingContext2D;
  private layers: SurfaceLayer[] = [];
  private queued = false;
  private revealGaps = true;
  private space: Space = { x: [-3, 3], y: [-3, 3], z: [-3, 3] };

  constructor(readonly canvas: HTMLCanvasElement, readonly host: HTMLElement) {
    this.ctx = canvas.getContext('2d')!;
    this.resize();
    new ResizeObserver(() => this.resize()).observe(host);
    let drag: { id: number; point: Point } | null = null;
    host.addEventListener('pointerdown', event => {
      if (event.button !== 0) return;
      drag = { id: event.pointerId, point: [event.clientX, event.clientY] };
      host.setPointerCapture(event.pointerId);
    });
    host.addEventListener('pointermove', event => {
      if (drag?.id !== event.pointerId) return;
      this.rotate((event.clientX - drag.point[0]) / 140, (event.clientY - drag.point[1]) / 140);
      drag.point = [event.clientX, event.clientY];
    });
    const stop = (event: PointerEvent) => { if (drag?.id === event.pointerId) drag = null; };
    host.addEventListener('pointerup', stop);
    host.addEventListener('pointercancel', stop);
    host.addEventListener('wheel', event => {
      event.preventDefault();
      this.zoom(Math.exp(-Math.max(-100, Math.min(100, event.deltaY)) * 0.004));
    }, { passive: false });
    host.addEventListener('keydown', event => {
      const moves: Record<string, Point> = { ArrowLeft: [-0.12, 0], ArrowRight: [0.12, 0],
        ArrowUp: [0, -0.12], ArrowDown: [0, 0.12] };
      if (moves[event.key]) this.rotate(...moves[event.key]);
      else if (event.key === '+' || event.key === '=') this.zoom(1.2);
      else if (event.key === '-') this.zoom(1 / 1.2);
      else if (event.key === 'Home') this.reset();
      else return;
      event.preventDefault();
    });
  }

  setLayers(layers: SurfaceLayer[], showUncertainty: boolean): void {
    this.layers = layers;
    if (layers.length) this.space = layers[0].mesh.space;
    this.revealGaps = showUncertainty;
    this.request();
  }

  camera() {
    return { yaw: this.yaw, pitch: this.pitch, magnification: this.magnification, space: this.space };
  }

  reset(): void {
    this.yaw = -0.64;
    this.pitch = 0.62;
    this.magnification = 1;
    this.request();
  }

  zoom(ratio: number): void {
    this.magnification = Math.max(0.15, Math.min(10, this.magnification * ratio));
    this.request();
  }

  private rotate(yaw: number, pitch: number): void {
    this.yaw = (this.yaw + yaw) % (2 * Math.PI);
    this.pitch = Math.max(-Math.PI / 2, Math.min(Math.PI / 2, this.pitch + pitch));
    this.request();
  }

  private resize(): void {
    if (this.host.hidden || !this.host.clientWidth || !this.host.clientHeight) return;
    this.width = this.host.clientWidth;
    this.height = this.host.clientHeight;
    const scale = Math.min(devicePixelRatio || 1, 2);
    this.canvas.width = Math.round(this.width * scale);
    this.canvas.height = Math.round(this.height * scale);
    this.ctx.setTransform(scale, 0, 0, scale, 0, 0);
    this.request();
  }

  private request(): void {
    if (this.queued) return;
    this.queued = true;
    requestAnimationFrame(() => {
      this.queued = false;
      if (this.host.clientWidth && this.host.clientHeight) this.draw();
    });
  }

  private draw(): void {
    const ctx = this.ctx;
    const { width, height } = this;
    const bounds = [this.space.x, this.space.y, this.space.z];
    const center = bounds.map(range => range[0] + (range[1] - range[0]) / 2);
    const worldSpan = Math.max(...bounds.map(([a, b]) => b - a));
    const scale = Math.min(width, height) * 0.78 * this.magnification;
    const cy = Math.cos(this.yaw), sy = Math.sin(this.yaw);
    const cp = Math.cos(this.pitch), sp = Math.sin(this.pitch);
    const orient = ([x, y, z]: Vec3): Vec3 => {
      const across = cy * y + sy * x;
      return [cy * x - sy * y, cp * z - sp * across, sp * z + cp * across];
    };
    const project = (p: Vec3): Projection => {
      const [x, y, depth] = orient(p.map((v, i) => (v - center[i]) / worldSpan) as Vec3);
      return { x: width / 2 + scale * x, y: height / 2 - scale * y, depth };
    };
    ctx.fillStyle = '#fcfdf8';
    ctx.fillRect(0, 0, width, height);
    const corners: Vec3[] = Array.from({ length: 8 }, (_, i) =>
      bounds.map((b, axis) => b[(i >> axis) & 1]) as Vec3);
    const line = (a: Vec3, b: Vec3, color: string) => {
      const p = project(a), q = project(b);
      ctx.beginPath(); ctx.moveTo(p.x, p.y); ctx.lineTo(q.x, q.y);
      ctx.strokeStyle = color; ctx.stroke();
    };
    ctx.lineWidth = 1;
    for (let j = 0; j < 8; j++) {
      for (let axis = 0; axis < 3; axis++) {
        if ((j >> axis & 1) === 0) line(corners[j], corners[j | 1 << axis], '#dce6dc');
      }
    }
    const triangles = this.layers.flatMap(({ mesh, color }) => mesh.faces.map(face => {
      const p = face.vertices.map(project);
      const depth = p.reduce((a, b) => a + b.depth / 3, 0);
      const normal = orient(face.normal);
      const light = Math.max(0.23, Math.min(1, 0.50 + 0.40 * (normal[0] * -0.38 + normal[1] * 0.68 + normal[2] * 0.49)));
      return { p, depth, light, color };
    }));
    triangles.sort((a, b) => a.depth - b.depth);
    for (const { p, light, color } of triangles) {
      const rgb = [1, 3, 5].map(i => Number.parseInt(color.slice(i, i + 2), 16));
      ctx.fillStyle = `rgb(${rgb.map(v => Math.round(v * light + (1 - light) * 196)).join(' ')})`;
      ctx.strokeStyle = ctx.fillStyle;
      ctx.beginPath(); ctx.moveTo(p[0].x, p[0].y); ctx.lineTo(p[1].x, p[1].y); ctx.lineTo(p[2].x, p[2].y); ctx.closePath();
      ctx.fill(); ctx.stroke();
    }
    if (this.revealGaps) {
      for (const layer of this.layers) {
        for (const gap of layer.mesh.gaps) {
          if (gap.domain.length !== 3 && !layer.mapGaps) continue;
          const point = gap.domain.map(([a, b]) => (a + b) / 2);
          if (point.length === 2) point.push(this.space.z[0]);
          const p = project(point as Vec3);
          ctx.fillStyle = gap.excluded ? '#884f4455' : '#bc6b3a77';
          ctx.fillRect(p.x - 1, p.y - 1, 3, 3);
        }
      }
    }
    ctx.font = '11px ui-monospace, monospace';
    ctx.fillStyle = '#406255';
    for (let i = 0; i < 3; i++) {
      const p = project(corners[1 << i]);
      ctx.fillText(['x', 'y', 'z'][i], p.x + 6, p.y + 6);
    }
    if (!triangles.length) {
      ctx.font = '13px system-ui';
      ctx.fillText('Add a surface expression or open a spatial exploration.', 14, 24);
    }
  }
}
