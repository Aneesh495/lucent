import { cell, COLORS, HOME, type Mode, type Parameter, type View, type Workspace } from './model';

type Sketch = { source: string; mode?: Mode; horizontal?: string; domain?: [number, number] };
export interface Exploration {
  title: string;
  lesson: string;
  sketches: Sketch[];
  view?: View;
  parameter?: Parameter;
  investigation?: { op: string; bounds: [string, string] };
}

const one = (title: string, lesson: string, source: string, mode: Mode = 'explicit',
  view?: View, investigation?: Exploration['investigation']): Exploration => ({
  title, lesson, sketches: [{ source, mode }], view, investigation,
});
const view = (x: [number, number], y: [number, number]): View => ({ x, y });
const inspect = (op: string, a: string, b: string) => ({ op, bounds: [a, b] as [string, string] });
const slider = (value: number, min: number, max: number, step: number): Parameter => ({ name: 'a', value, min, max, step });

export const GALLERY: Exploration[] = [
  one('A pole smaller than a pixel', 'Zoom near zero and watch the unresolved strip track the denominator zero.', '1/(x - .000013)', 'explicit', view([-2, 2], [-8, 8])),
  one('A removable hole', 'The curve follows the diagonal away from one. The denominator still excludes x = 1.', '(x^2-1)/(x-1)', 'explicit', view([-3, 3], [-2, 5])),
  one('Tangent lattice', 'Refine around odd half multiples of pi. Compare with degree mode.', 'tan(x)', 'explicit', view([-8, 8], [-6, 6])),
  one('A jump at a closed endpoint', 'Compare the two branches and the unresolved band around the switch.', 'if(x <= 0, x + 1, x - 1)'),
  one('Logarithm boundary', 'Pan into x ≤ 0 and distinguish excluded cells from visible samples.', 'log(x)', 'explicit', view([-1, 5], [-5, 3])),
  one('Square root at zero', 'The function reaches zero while its derivative becomes unbounded.', 'sqrt(x)', 'explicit', view([-1, 5], [-1, 3]), inspect('derivative', '0', '1')),
  one('High-frequency envelope', 'Zoom until individual oscillations separate. Refine only the current viewport.', 'sin(140x)/(1+x^2)'),
  one('Chirp near zero', 'Subdivision eventually runs out of screen resolution around x = 0.', 'x*sin(1/x)', 'explicit', view([-0.8, 0.8], [-0.8, 0.8])),
  one('Absolute-value cusp', 'Try a derivative at zero and then on either side of the cusp.', 'abs(x)', 'explicit', view([-3, 3], [-1, 4]), inspect('derivative', '0', '1')),
  one('Well-conditioned root', 'Inspect the endpoint signs and a nonzero derivative on the witness.', 'x^2-2', 'explicit', view([0, 3], [-3, 6]), inspect('roots', '1', '2')),
  one('Repeated roots', 'Count multiplicities that a sign-change scan cannot detect.', '(x-1)^2*(x+1)^3', 'explicit', view([-2, 2], [-4, 4]), inspect('polynomial', '-1', '2')),
  { title: 'A split double root', lesson: 'Move a across zero and watch two real roots merge and disappear.',
    sketches: [{ source: '(x-1)^2-a' }], parameter: slider(0.02, -0.2, 0.2, 0.001), view: view([0.4, 1.6], [-0.25, 0.5]) },
  one('Near tangency', 'Zoom into the small gap between the two roots of the residual.', '(x-1)^2-1e-6', 'explicit', view([0.995, 1.005], [-0.000004, 0.00002]), inspect('roots', '0.995', '1.005')),
  one('Interval dependency', 'The interval evaluator tracks both appearances of x independently.', 'x-x', 'explicit', view([-2, 2], [-1, 1]), inspect('enclose', '1', '2')),
  one('Cancellation at a point', 'Precision escalation recovers a small remainder lost at low precision.', '((exp(1)+1e-40)-exp(1))/1e-40', 'scalar'),
  one('Exact decimals', 'Change a decimal digit and inspect the reduced rational result.', '0.1 + 0.2 - 0.3', 'scalar'),
  one('Rational power', 'Try an exact negative integer exponent and then a noninteger one.', '(2/7)^(-5)', 'scalar'),
  one('Complex rotation', 'Edit the exponent and compare the exact real and imaginary components.', '(1+i)^5', 'scalar'),
  one('Complex magnitude', 'Recover a rational length from a complex pair.', 'abs(complex(3/2, 2))', 'scalar'),
  one('Centimetres and metres', 'Add compatible lengths. Replace cm with s to see dimensional rejection.', '3m + 25cm', 'scalar'),
  one('Explicit angle', 'Switch the global convention. The explicit degree unit remains an angle.', 'sin(30deg)', 'scalar'),
  one('Finite sum', 'Change the list members and compare exact numerator and denominator.', 'sum([1/3,1/5,1/7,1/11])', 'scalar'),
  one('Finite product', 'Use a zero item to distinguish a zero product from an excluded item.', 'product([1/3,3/5,5/7,7/9])', 'scalar'),
  one('Two equations', 'Edit the right-hand side of an exact linear system.', 'solve([[3,2],[5,4]],[7,13])', 'scalar'),
  one('Determinant under perturbation', 'Make the two rows linearly dependent and inspect the exact zero.', 'det([[1,1],[1,1.00000000000001]])', 'scalar'),
  one('Stable right tail', 'Inspect the survival function where subtracting a CDF from one loses precision.', 'normal_sf(9)', 'scalar'),
  one('Normal CDF', 'Move across the median and compare symmetry with the survival curve.', 'normal_cdf(x)', 'explicit', view([-5, 5], [-0.12, 1.12])),
  one('Simpson exactly integrates a cubic', 'The fourth derivative vanishes. Inspect the panel sum.', 'x^3 - 2x + 1', 'explicit', view([-2, 2], [-3, 5]), inspect('integral', '0', '2')),
  one('Adaptive sine quadrature', 'Compare the width of the enclosure with the inspector precision.', 'sin(x)', 'explicit', HOME, inspect('integral', '0', '1')),
  one('A failed finite integral', 'The requested interval contains a pole. Coverage cannot become complete.', '1/x', 'explicit', HOME, inspect('integral', '-1', '1')),
  one('Derivative agreement', 'Compare the symbolic enclosure to finite differences at two step sizes.', 'exp(x^2)', 'explicit', view([-2, 2], [-0.5, 8]), inspect('derivative', '0.9', '1')),
  one('Circle by interval exclusion', 'Inspect the boundary pixels where sampled geometry leaves topology open.', 'x^2+y^2=4', 'implicit', view([-3, 3], [-3, 3])),
  one('Crossing branches', 'Refine the origin where four branches meet.', 'x^2-y^2=0', 'implicit', view([-3, 3], [-3, 3])),
  one('A small loop', 'Find a displaced loop inside a viewport much larger than its radius.', '(x-.43)^2+(y-.21)^2=.0144', 'implicit', view([-1, 1], [-1, 1])),
  one('Tangential contact', 'The two circle branches touch at the origin without a clean cell crossing there.', '((x-1)^2+y^2-1)*((x+1)^2+y^2-1)=0', 'implicit', view([-3, 3], [-2, 2])),
  one('Proved disk cells', 'Solid fill represents full-cell membership. The boundary band remains uncertain.', 'x^2+y^2<=4', 'region', view([-3, 3], [-3, 3])),
  one('Strict inequality', 'Switch < to ≤ and check how equality is treated at the boundary.', 'y < x^2-1', 'region', view([-3, 3], [-3, 3])),
  one('Three-leaf polar curve', 'Explore self-intersections created by negative radii.', '2cos(3t)', 'polar', view([-3, 3], [-3, 3])),
  one('Polar spiral', 'Set a shorter parameter range to inspect the first turn.', 't/3', 'polar', view([-3, 3], [-3, 3])),
  { title: 'Lissajous figure', lesson: 'Vary a to move between rational frequency ratios and near closure.',
    sketches: [{ source: 'sin(a*t)', horizontal: 'cos(2t)', mode: 'parametric' }], parameter: slider(3, 1, 6, 0.05), view: view([-1.25, 1.25], [-1.25, 1.25]) },
  { title: 'Parametric figure eight', lesson: 'Adjust the t domain and trace each traversal of the crossing.',
    sketches: [{ source: 'sin(2t)', horizontal: 'sin(t)', mode: 'parametric' }], view: view([-1.25, 1.25], [-1.25, 1.25]) },
  { title: 'Parametric singularity', lesson: 'The parameter can approach zero from both sides without joining across it.',
    sketches: [{ source: '1/t', horizontal: 't', mode: 'parametric', domain: [-3, 3] }], view: view([-3, 3], [-4, 4]) },
  { title: 'Shared function definition', lesson: 'Change the single definition to recompute both plots and the sample table.',
    sketches: [{ source: 'f(t) := exp(-t^2)', mode: 'definition' }, { source: 'f(x)' }, { source: 'f(x-2)' }], view: view([-4, 6], [-0.2, 1.25]) },
  { title: 'Competing growth rates', lesson: 'Compare a cubic and an exponential. Use a residual equation in the root inspector.',
    sketches: [{ source: 'exp(x)' }, { source: 'x^3' }], view: view([-1, 6], [-4, 100]) },
  one('A rational near a zero divisor', 'The quotient remains undefined when the exact rational denominator is zero.', '(1/3-0.333333333333333333)/(1/3-1/3)', 'scalar'),
];

export function loadExploration(entry: Exploration): Workspace {
  return {
    version: 2,
    title: entry.title,
    cells: entry.sketches.map((item, index) => {
      const c = cell(item.source, item.mode, COLORS[index % COLORS.length]);
      if (item.horizontal !== undefined) c.horizontal = item.horizontal;
      if (item.domain !== undefined) c.domain = [...item.domain];
      return c;
    }),
    parameters: entry.parameter ? [{ ...entry.parameter }] : [],
    angle: 'radians',
    view: structuredClone(entry.view ?? HOME),
  };
}
