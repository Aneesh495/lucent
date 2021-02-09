import { coordinate } from './renderer';
import type { Point } from './model';

interface FitPoint { x: number; y: number; fitted: number; residual: number }
interface Fit {
  coefficients: number[] | null;
  coefficient_std_error: number[] | null;
  center: number;
  scale: number;
  rank: number;
  condition: number | null;
  residual_norm: number | null;
  residual_std: number | null;
  degrees_of_freedom: number;
  r_squared: number | null;
  points: FitPoint[];
}
interface ProbabilityPoint { expected: Point; observed: number }

function html<K extends keyof HTMLElementTagNameMap>(tag: K, text?: string): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  return node;
}

function number(value: number | null): string { return value === null ? 'unavailable' : coordinate(value); }

export function table(label: string, fields: string[], records: string[][]): HTMLTableElement {
  const node = html('table');
  node.createCaption().textContent = label;
  const row = node.createTHead().insertRow();
  for (const field of fields) {
    const heading = html('th', field);
    heading.scope = 'col';
    row.append(heading);
  }
  const body = node.createTBody();
  for (const record of records) {
    const row = body.insertRow();
    for (const value of record) row.insertCell().textContent = value;
  }
  return node;
}

export function regression(parent: HTMLElement, value: unknown): void {
  const fit = value as Fit;
  const heading = html('h2', 'Polynomial fit');
  const summary = html('p', 'Estimated rank ' + fit.rank + ' · condition ' + number(fit.condition));
  summary.className = 'analysis-summary';
  parent.replaceChildren(heading, summary);
  if (!fit.coefficients) {
    parent.append(html('p', 'A unique coefficient vector could not be determined at the numerical rank threshold.'));
    return;
  }
  const assumptions = html('p', 'Basis t = (x − ' + coordinate(fit.center) + ') / ' + coordinate(fit.scale) +
    '. Standard errors assume independent observations with equal residual variance.');
  assumptions.className = 'analysis-summary';
  const metrics = html('p', 'Residual norm ' + number(fit.residual_norm) +
    ' · residual standard deviation ' + number(fit.residual_std) + ' · R² ' + number(fit.r_squared) +
    ' · ' + fit.degrees_of_freedom + ' degrees of freedom');
  metrics.className = 'analysis-summary';
  const coefficientTable = table('Estimated coefficients', ['Term', 'Coefficient', 'Standard error'],
    fit.coefficients.map((coefficient, i) => ['t^' + i, number(coefficient), number(fit.coefficient_std_error?.[i] ?? null)]));
  const dataTable = table('Observations and residuals', ['x', 'Observed y', 'Fitted y', 'Residual'],
    fit.points.map(p => [p.x, p.y, p.fitted, p.residual].map(number)));
  const charts = html('div');
  charts.className = 'analysis-charts';
  const sorted = [...fit.points].sort((a,b) => a.x-b.x);
  charts.append(scatter('Observed values and fitted curve', sorted.map(p => [p.x, p.y]), sorted.map(p => [p.x, p.fitted])),
    scatter('Residuals by input', sorted.map(p => [p.x, p.residual]), [[sorted[0].x, 0], [sorted[sorted.length-1].x, 0]]));
  parent.append(assumptions, metrics, charts, coefficientTable, dataTable);
}

export function probability(parent: HTMLElement, data: unknown): void {
  const points = data as ProbabilityPoint[];
  const observations: Point[] = points.map(p => [p.expected[0] / 2 + p.expected[1] / 2, p.observed]);
  const summary = html('p', 'Each theoretical position is bracketed with directed normal-tail arithmetic.');
  summary.className = 'analysis-summary';
  parent.replaceChildren(html('h2', 'Normal probability plot'), summary,
    scatter('Theoretical normal quantile versus observed value', observations),
    table('Ordered observations', ['Theoretical interval', 'Observed value'],
      points.map(p => ['[' + p.expected.map(number).join(', ') + ']', number(p.observed)])));
}

function scatter(label: string, observations: Point[], line: Point[] = []): SVGSVGElement {
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('viewBox', '0 0 370 185');
  svg.setAttribute('role', 'img');
  svg.setAttribute('aria-label', label);
  svg.classList.add('analysis-chart');
  const points = [...observations, ...line];
  const extent = (axis: 0 | 1): Point => {
    const values = points.map(p => p[axis]);
    const a = Math.min(...values), b = Math.max(...values);
    const slack = a === b ? Math.max(1, Math.abs(a) * .1) : .06 * (b-a);
    return [a - slack, b + slack];
  };
  const x = extent(0), y = extent(1);
  const project = (p: Point): Point => [34 + (p[0]-x[0]) / (x[1]-x[0]) * 322, 153 - (p[1]-y[0]) / (y[1]-y[0]) * 123];
  const child = (tag: string, attrs: Record<string, string>, text?: string) => {
    const el = document.createElementNS('http://www.w3.org/2000/svg', tag);
    for (const [key, value] of Object.entries(attrs)) el.setAttribute(key, value);
    if (text !== undefined) el.textContent = text;
    svg.append(el);
  };
  child('text', { x: '11', y: '17', class: 'analysis-title' }, label);
  child('path', { d: 'M34 30V153H356', stroke: '#8aa297', fill: 'none' });
  for (let i = 0; i < 5; i++) {
    const tick = x[0] + (x[1]-x[0])*i/4;
    const [px] = project([tick, 0]);
    child('path', { d: `M${px} 30V153`, stroke: '#e0e8e1' });
    child('text', { x: String(px), y: '169', 'text-anchor': 'middle' }, number(tick));
  }
  if (line.length) {
    child('polyline', { points: line.map(p => project(p).join(',')).join(' '), fill: 'none', stroke: '#566bac', 'stroke-width': '1.6' });
  }
  for (const p of observations) {
    const [cx, cy] = project(p);
    child('circle', { cx: String(cx), cy: String(cy), r: '3', fill: '#2b7663' });
  }
  return svg;
}
