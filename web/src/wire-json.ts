export function wireJson(value: unknown): string {
  if (typeof value === 'number' && Object.is(value, -0)) return '-0.0';
  if (Array.isArray(value)) return '[' + value.map(item => wireJson(item ?? null)).join(',') + ']';
  if (value && typeof value === 'object') {
    return '{' + Object.entries(value)
      .filter(([, field]) => field !== undefined)
      .map(([key, field]) => JSON.stringify(key) + ':' + wireJson(field))
      .join(',') + '}';
  }
  return JSON.stringify(value);
}
