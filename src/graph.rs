use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use crate::{
    differentiate::{derivative, inline},
    eval::{Angle, Eval, Scope},
    interval::{Interval, Issue, Result},
    lang::Expr,
    program::Program,
    value::{Dimension, Real, Truth, Value},
};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct View {
    pub x: [f64; 2],
    pub y: [f64; 2],
    pub pixels: [u32; 2],
}

impl View {
    pub fn validate(self) -> Result<Self> {
        if valid_domain(self.x)
            && valid_domain(self.y)
            && self.pixels.iter().all(|p| (1..=4096).contains(p))
        {
            Ok(self)
        } else {
            Err(Issue::Unsupported(
                "Viewport needs finite increasing bounds and a size from 1 to 4096 pixels",
            ))
        }
    }

    pub fn scales(self) -> [f64; 2] {
        [
            f64::from(self.pixels[0]) / (self.x[1] - self.x[0]),
            f64::from(self.pixels[1]) / (self.y[1] - self.y[0]),
        ]
    }
}

pub fn valid_domain([a, b]: [f64; 2]) -> bool {
    a.is_finite() && b.is_finite() && a < b && (b - a).is_finite() && (b - a) >= 1e-280
}

#[derive(Clone, Copy, Debug)]
pub struct Controls {
    pub bits: u32,
    pub angle: Angle,
    pub budget: usize,
    pub tolerance: f64,
}

impl Controls {
    pub fn check(self) -> Result<Self> {
        if !(53..=512).contains(&self.bits)
            || !(1..=32_768).contains(&self.budget)
            || !(0.1..=8.0).contains(&self.tolerance)
        {
            return Err(Issue::Unsupported(
                "Precision, work budget or pixel tolerance outside the supported range",
            ));
        }
        Ok(self)
    }
}

pub struct Context<'a> {
    pub program: &'a Program,
    pub scope: &'a Scope,
    pub control: Controls,
}

impl Context<'_> {
    pub fn evaluate(&self, expr: &Expr, coordinates: &[(&str, [f64; 2])]) -> Result<Value> {
        let mut scope = self.scope.clone();
        for (axis, [lo, hi]) in coordinates {
            let interval = Interval::new(*lo, *hi, self.control.bits)?;
            scope.insert((*axis).to_string(), Value::real(Real::Bounded(interval)));
        }
        Eval::new(self.program, &scope, self.control.bits, self.control.angle)
            .run(expr)
            .map_err(|error| error.issue)
    }

    pub fn real(&self, expr: &Expr, coordinates: &[(&str, [f64; 2])]) -> Result<Real> {
        let q = self.evaluate(expr, coordinates)?.quantity()?;
        Ok(q.dimensionless()?.clone())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CellKind {
    Segment,
    Offscreen,
    Excluded,
    Unresolved,
    Empty,
    Inside,
    Crossing,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CurveCell {
    pub domain: [f64; 2],
    pub kind: CellKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment: Option<[[f64; 2]; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_px: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Curve {
    pub cells: Vec<CurveCell>,
    pub processed: usize,
    pub dimension: [Dimension; 2],
}

pub struct CurveProblem<'a> {
    pub horizontal: Option<&'a Expr>,
    pub vertical: &'a Expr,
    pub axis: &'a str,
    pub domain: [f64; 2],
    pub view: View,
}

type Range = ([f64; 2], Dimension);

impl CurveProblem<'_> {
    fn evaluate(&self, ctx: &Context, domain: [f64; 2]) -> Result<[Range; 2]> {
        let interval = &[(self.axis, domain)];
        let range = |expr: &Expr| -> Result<Range> {
            let value = ctx.evaluate(expr, interval)?.quantity()?;
            Ok((
                value.real.enclose(ctx.control.bits)?.bounds()?,
                value.dimension,
            ))
        };
        let x = match self.horizontal {
            Some(expr) => range(expr)?,
            None => (domain, Dimension::NONE),
        };
        Ok([x, range(self.vertical)?])
    }

    pub fn domain_valid(&self, ctx: &Context, domain: [f64; 2]) -> bool {
        self.evaluate(ctx, domain).is_ok()
    }

    pub fn segment_endpoints(&self, ctx: &Context, [a, b]: [f64; 2]) -> Result<[[f64; 2]; 2]> {
        let point = |x| -> Result<[f64; 2]> {
            let [horizontal, vertical] = self.evaluate(ctx, [x, x])?;
            Ok([middle(horizontal.0), middle(vertical.0)])
        };
        Ok([point(a)?, point(b)?])
    }
}

pub fn curve(ctx: &Context, problem: &CurveProblem) -> Result<Curve> {
    let view = problem.view.validate()?;
    if !valid_domain(problem.domain) {
        return Err(Issue::Unsupported("Invalid parameter range"));
    }
    let controls = ctx.control.check()?;
    let second = |expr| -> Option<Expr> {
        let expanded = inline(expr, ctx.program).ok()?;
        let first = derivative(&expanded, problem.axis, controls.angle).ok()?;
        derivative(&first, problem.axis, controls.angle).ok()
    };
    let second_x = problem.horizontal.and_then(second);
    let second_y = second(problem.vertical);
    let scales = view.scales();
    let mut stack = vec![(problem.domain, 0)];
    let mut cells = Vec::new();
    let mut processed = 0;
    let mut dimensions = None;
    while let Some((domain, depth)) = stack.pop() {
        let [a, b] = domain;
        let m = middle(domain);
        let terminal = m == a
            || m == b
            || depth >= 38
            || (problem.horizontal.is_none() && (b - a) * scales[0] < 0.18);
        let mut cell = CurveCell {
            domain,
            kind: CellKind::Unresolved,
            segment: None,
            error_px: None,
            reason: None,
        };
        if processed == controls.budget {
            cell.reason = Some("Subdivision budget reached".into());
            cells.push(cell);
            continue;
        }
        processed += 1;
        match problem.evaluate(ctx, domain) {
            Ok([x, y]) => {
                let current = [x.1, y.1];
                if *dimensions.get_or_insert(current) != current {
                    cell.reason = Some("Output dimensions change across the curve".into());
                } else if outside(x.0, view.x) || outside(y.0, view.y) {
                    cell.kind = CellKind::Offscreen;
                } else {
                    match problem.segment_endpoints(ctx, domain) {
                        Ok(segment) => {
                            let err = |range: [f64; 2], column: usize, second: Option<&Expr>| {
                                let chord = [segment[0][column], segment[1][column]];
                                let band = (range[1] - chord[0].min(chord[1]))
                                    .max(chord[0].max(chord[1]) - range[0]);
                                if column == 0 && problem.horizontal.is_none() {
                                    return 0.0;
                                }
                                let derivative = second
                                    .and_then(|expr| ctx.real(expr, &[(problem.axis, domain)]).ok())
                                    .and_then(|r| r.enclose(controls.bits).ok()?.bounds().ok())
                                    .map(|[lo, hi]| lo.abs().max(hi.abs()));
                                let slack = chord.iter().fold(0.0_f64, |a, b| a.max(b.abs()))
                                    * f64::EPSILON
                                    * 4.0;
                                let width = b - a;
                                let deviation = derivative
                                    .map_or(band, |d| band.min(d * width * width / 8.0 + slack));
                                deviation * scales[column]
                            };
                            let error = err(x.0, 0, second_x.as_ref()).hypot(err(
                                y.0,
                                1,
                                second_y.as_ref(),
                            ));
                            if error <= controls.tolerance {
                                cell.kind = CellKind::Segment;
                                cell.error_px = Some(error);
                                cell.segment = Some(segment);
                            } else {
                                cell.reason = Some("Screen-space tolerance unresolved".into());
                            }
                        }
                        Err(issue) => cell.reason = Some(issue.to_string()),
                    }
                }
            }
            Err(issue @ Issue::Excluded(_)) => {
                cell.kind = CellKind::Excluded;
                cell.reason = Some(issue.to_string());
            }
            Err(issue) => cell.reason = Some(issue.to_string()),
        }
        if cell.kind == CellKind::Unresolved && !terminal {
            stack.push(([m, b], depth + 1));
            stack.push(([a, m], depth + 1));
        } else {
            cells.push(cell);
        }
    }
    Ok(Curve {
        cells,
        processed,
        dimension: dimensions.unwrap_or_default(),
    })
}

pub fn middle([a, b]: [f64; 2]) -> f64 {
    a.midpoint(b)
}

fn outside(range: [f64; 2], domain: [f64; 2]) -> bool {
    range[0] > domain[1] || range[1] < domain[0]
}

#[derive(Debug, Serialize)]
pub struct AreaCell {
    pub x: [f64; 2],
    pub y: [f64; 2],
    pub kind: CellKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment: Option<[[f64; 2]; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Area {
    pub cells: Vec<AreaCell>,
    pub processed: usize,
}

pub fn area(ctx: &Context, expr: &Expr, view: View, inequality: bool) -> Result<Area> {
    let controls = ctx.control.check()?;
    let view = view.validate()?;
    let scale = view.scales();
    let mut work = vec![(view.x, view.y, 0)];
    let mut cells = Vec::new();
    let mut processed = 0;
    while let Some((x, y, depth)) = work.pop() {
        let mut cell = AreaCell {
            x,
            y,
            kind: CellKind::Unresolved,
            segment: None,
            reason: None,
        };
        if processed == controls.budget {
            cell.reason = Some("Subdivision budget reached".into());
            cells.push(cell);
            continue;
        }
        processed += 1;
        let span = [(x[1] - x[0]) * scale[0], (y[1] - y[0]) * scale[1]];
        let eval = ctx.evaluate(expr, &[("x", x), ("y", y)]);
        let mut potentially_crossing = false;
        match eval {
            Ok(Value::Bool(Truth::True)) if inequality => cell.kind = CellKind::Inside,
            Ok(Value::Bool(Truth::False)) if inequality => cell.kind = CellKind::Empty,
            Ok(Value::Bool(Truth::Unknown)) if inequality => {
                cell.reason = Some("Boundary membership unresolved".into())
            }
            Ok(Value::Quantity(quantity)) if !inequality => {
                if !quantity.real.enclose(controls.bits)?.contains_zero() {
                    cell.kind = CellKind::Empty;
                } else {
                    potentially_crossing = true;
                    cell.reason =
                        Some("Possible multiple components, tangency or singularity".into());
                }
            }
            Ok(_) => {
                return Err(Issue::Unsupported(
                    "Implicit mode needs a real residual; region mode needs a comparison",
                ));
            }
            Err(issue @ Issue::Excluded(_)) => {
                cell.kind = CellKind::Excluded;
                cell.reason = Some(issue.to_string());
            }
            Err(issue) => cell.reason = Some(issue.to_string()),
        }
        if cell.kind == CellKind::Unresolved
            && span[0].max(span[1]) > controls.tolerance.max(2.0)
            && depth < 28
        {
            let split_x = span[0] >= span[1];
            let [lo, hi] = if split_x { x } else { y };
            let m = lo.midpoint(hi);
            if m != lo && m != hi {
                if split_x {
                    work.push(([m, hi], y, depth + 1));
                    work.push(([lo, m], y, depth + 1));
                } else {
                    work.push((x, [m, hi], depth + 1));
                    work.push((x, [lo, m], depth + 1));
                }
                continue;
            }
        }
        if potentially_crossing {
            if let Some(segment) = crossing(ctx, expr, x, y) {
                cell.kind = CellKind::Crossing;
                cell.segment = Some(segment);
                cell.reason = Some("Sampled edge crossing; topology unresolved".into());
            }
        }
        cells.push(cell);
    }
    Ok(Area { cells, processed })
}

fn crossing(ctx: &Context, expr: &Expr, x: [f64; 2], y: [f64; 2]) -> Option<[[f64; 2]; 2]> {
    let corners = [[x[0], y[0]], [x[1], y[0]], [x[1], y[1]], [x[0], y[1]]];
    let mut signs = [Ordering::Equal; 4];
    let mut values = [0.0; 4];
    for (i, &[a, b]) in corners.iter().enumerate() {
        let real = ctx.real(expr, &[("x", [a, a]), ("y", [b, b])]).ok()?;
        signs[i] = real.sign()?;
        values[i] = middle(real.enclose(ctx.control.bits).ok()?.bounds().ok()?);
    }
    let mut hits = Vec::new();
    for i in 0..4 {
        let j = (i + 1) % 4;
        if signs[i] == Ordering::Equal {
            hits.push(corners[i]);
        } else if signs[j] != Ordering::Equal && signs[i] != signs[j] {
            let weight = values[i].abs() / (values[i].abs() + values[j].abs());
            if !weight.is_finite() {
                return None;
            }
            hits.push([
                corners[i][0] + (corners[j][0] - corners[i][0]) * weight,
                corners[i][1] + (corners[j][1] - corners[i][1]) * weight,
            ]);
        }
    }
    if let [a, b] = hits.as_slice() {
        Some([*a, *b])
    } else {
        None
    }
}
