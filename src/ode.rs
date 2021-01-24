use serde::Serialize;

use crate::{
    differentiate::{derivative, inline},
    graph::{Context, View, middle, valid_domain},
    interval::{Interval, Issue, Result},
    lang::Expr,
};

pub type Vector = [f64; 2];

pub struct Field<'a> {
    pub dx: &'a Expr,
    pub dy: &'a Expr,
}

impl Field<'_> {
    fn bounds(
        &self,
        ctx: &Context,
        time: [f64; 2],
        x: [f64; 2],
        y: [f64; 2],
    ) -> Result<[Interval; 2]> {
        let coordinates = &[("t", time), ("x", x), ("y", y)];
        let f = |expr: &Expr| ctx.real(expr, coordinates)?.enclose(ctx.control.bits);
        Ok([f(self.dx)?, f(self.dy)?])
    }

    fn at(&self, ctx: &Context, t: f64, [x, y]: Vector) -> Result<Vector> {
        let [u, v] = self.bounds(ctx, [t; 2], [x; 2], [y; 2])?;
        Ok([middle(u.bounds()?), middle(v.bounds()?)])
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Sample {
    pub t: f64,
    pub state: Vector,
    pub local_error: f64,
}

#[derive(Debug, Serialize)]
pub struct Event {
    pub time_bracket: [f64; 2],
    pub endpoint_values: [f64; 2],
    pub sample: Sample,
}

#[derive(Debug, Serialize)]
pub struct LocalBox {
    pub start: Sample,
    pub time: [f64; 2],
    pub tube: [[f64; 2]; 2],
    pub endpoint: [[f64; 2]; 2],
    pub contraction_upper: f64,
}

#[derive(Debug, Serialize)]
pub struct Trajectory {
    pub samples: Vec<Sample>,
    pub event: Option<Event>,
    pub rejected_steps: usize,
    pub stiffness_warning: bool,
    pub complete: bool,
    pub reason: Option<&'static str>,
    pub local_box: Option<LocalBox>,
}

#[derive(Clone, Copy)]
pub struct InitialProblem<'a> {
    pub time: [f64; 2],
    pub state: Vector,
    pub tolerance: f64,
    pub event: Option<&'a Expr>,
    pub validated: bool,
}

pub fn integrate(ctx: &Context, field: &Field, problem: InitialProblem) -> Result<Trajectory> {
    let controls = ctx.control.check()?;
    if !valid_domain(problem.time)
        || !problem.state.iter().all(|x| x.is_finite())
        || !(1e-13..=0.05).contains(&problem.tolerance)
    {
        return Err(Issue::Unsupported("Invalid IVP time, state or tolerance"));
    }
    let [begin, end] = problem.time;
    let mut current = Sample {
        t: begin,
        state: problem.state,
        local_error: 0.0,
    };
    let mut h = (end - begin) / 32.0;
    let mut out = Trajectory {
        samples: vec![current],
        event: None,
        rejected_steps: 0,
        stiffness_warning: false,
        complete: false,
        reason: None,
        local_box: None,
    };
    let mut event_value = if let Some(expr) = problem.event {
        Some(event_at(ctx, expr, current)?)
    } else {
        None
    };
    let initial_box = [[begin; 2], [current.state[0]; 2], [current.state[1]; 2]];
    if field.at(ctx, begin, current.state).is_ok() {
        out.stiffness_warning = jacobian_bound(ctx, field, initial_box)
            .and_then(|norm| norm.bounds())
            .is_ok_and(|range| range[1] * (end - begin) > 100.0);
    }
    let mut rejection_streak = 0;
    let mut domain_rejections = 0;
    for _ in 0..controls.budget {
        h = h.min(end - current.t);
        if h <= 0.0 || current.t + h == current.t {
            out.reason = Some("Step size reached machine resolution");
            break;
        }
        let trial = step(ctx, field, current, h);
        let norm = trial.as_ref().ok().map_or(f64::INFINITY, |(next, error)| {
            (0..2)
                .map(|i| {
                    error[i].abs()
                        / (problem.tolerance
                            * (1.0 + current.state[i].abs().max(next.state[i].abs())))
                })
                .fold(0.0, f64::max)
        });
        if norm > 1.0 || !norm.is_finite() {
            if trial.is_ok() {
                rejection_streak += 1;
            } else {
                rejection_streak = 0;
                domain_rejections += 1;
            }
            out.rejected_steps += 1;
            out.stiffness_warning |= rejection_streak >= 5;
            h *= if norm.is_finite() {
                (0.9 * norm.powf(-0.2)).clamp(0.12, 0.5)
            } else {
                0.25
            };
            continue;
        }
        rejection_streak = 0;
        let (next, _) = trial?;
        if problem.validated && out.samples.len() == 1 {
            out.local_box = local_box(ctx, field, current, next).ok();
        }
        if let (Some(expr), Some(left)) = (problem.event, event_value) {
            let right = match event_at(ctx, expr, next) {
                Ok(value) => value,
                Err(_) => {
                    out.reason = Some("Event function is unresolved along the trajectory");
                    break;
                }
            };
            if left == 0.0 || right == 0.0 || left.is_sign_negative() != right.is_sign_negative() {
                if let Ok(event) = locate(
                    ctx,
                    field,
                    expr,
                    current,
                    next,
                    [left, right],
                    problem.tolerance,
                ) {
                    out.samples.push(event.sample);
                    out.event = Some(event);
                    out.complete = true;
                } else {
                    out.reason = Some("Event refinement reached an undefined state");
                }
                break;
            }
            event_value = Some(right);
        }
        out.samples.push(next);
        current = next;
        if current.t >= end {
            out.complete = true;
            break;
        }
        h *= if norm == 0.0 {
            3.0
        } else {
            (0.9 * norm.powf(-0.2)).clamp(0.2, 3.0)
        };
    }
    if !out.complete && out.reason.is_none() {
        out.reason = Some(
            if domain_rejections == out.rejected_steps && domain_rejections > 0 {
                "Stage evaluations remain undefined or outside the numeric range"
            } else {
                "Step budget reached"
            },
        );
    }
    Ok(out)
}

fn combine(state: Vector, step: f64, slopes: &[Vector], coefficients: &[f64]) -> Vector {
    let mut result = state;
    for (slope, coeff) in slopes.iter().zip(coefficients) {
        for i in 0..2 {
            result[i] += step * coeff * slope[i];
        }
    }
    result
}

fn step(ctx: &Context, field: &Field, start: Sample, h: f64) -> Result<(Sample, Vector)> {
    // Dormand-Prince 5(4), sharing the seven stage evaluations.
    const C: [f64; 7] = [0.0, 1.0 / 5.0, 3.0 / 10.0, 4.0 / 5.0, 8.0 / 9.0, 1.0, 1.0];
    const A: [&[f64]; 7] = [
        &[],
        &[1.0 / 5.0],
        &[3.0 / 40.0, 9.0 / 40.0],
        &[44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0],
        &[
            19372.0 / 6561.0,
            -25360.0 / 2187.0,
            64448.0 / 6561.0,
            -212.0 / 729.0,
        ],
        &[
            9017.0 / 3168.0,
            -355.0 / 33.0,
            46732.0 / 5247.0,
            49.0 / 176.0,
            -5103.0 / 18656.0,
        ],
        &[
            35.0 / 384.0,
            0.0,
            500.0 / 1113.0,
            125.0 / 192.0,
            -2187.0 / 6784.0,
            11.0 / 84.0,
        ],
    ];
    const B4: [f64; 7] = [
        5179.0 / 57600.0,
        0.0,
        7571.0 / 16695.0,
        393.0 / 640.0,
        -92097.0 / 339200.0,
        187.0 / 2100.0,
        1.0 / 40.0,
    ];
    let mut slopes = [[0.0; 2]; 7];
    let mut next = start.state;
    for i in 0..7 {
        next = combine(start.state, h, &slopes, A[i]);
        slopes[i] = field.at(ctx, start.t + C[i] * h, next)?;
    }
    let alternate = combine(start.state, h, &slopes, &B4);
    let error = [next[0] - alternate[0], next[1] - alternate[1]];
    let t = start.t + h;
    if !t.is_finite() || !next.iter().all(|x| x.is_finite()) || !error.iter().all(|x| x.is_finite())
    {
        return Err(Issue::Range);
    }
    Ok((
        Sample {
            t,
            state: next,
            local_error: error[0].abs().max(error[1].abs()),
        },
        error,
    ))
}

fn event_at(ctx: &Context, expr: &Expr, sample: Sample) -> Result<f64> {
    let [x, y] = sample.state;
    Ok(middle(
        ctx.real(expr, &[("t", [sample.t; 2]), ("x", [x; 2]), ("y", [y; 2])])?
            .enclose(ctx.control.bits)?
            .bounds()?,
    ))
}

fn locate(
    ctx: &Context,
    field: &Field,
    expr: &Expr,
    initial: Sample,
    final_sample: Sample,
    mut values: [f64; 2],
    tolerance: f64,
) -> Result<Event> {
    let mut a = initial;
    let mut b = final_sample;
    for _ in 0..48 {
        if values[0] == 0.0 {
            b = a;
            values[1] = values[0];
            break;
        }
        if values[1] == 0.0 {
            a = b;
            values[0] = values[1];
            break;
        }
        let midpoint = a.t.midpoint(b.t);
        if b.t - a.t <= tolerance || midpoint <= a.t || midpoint >= b.t {
            break;
        }
        let (m, _) = step(ctx, field, initial, midpoint - initial.t)?;
        let value = event_at(ctx, expr, m)?;
        if value.is_sign_negative() == values[0].is_sign_negative() {
            a = m;
            values[0] = value;
        } else {
            b = m;
            values[1] = value;
        }
    }
    let sample = if values[0].abs() < values[1].abs() {
        a
    } else {
        b
    };
    Ok(Event {
        time_bracket: [a.t, b.t],
        endpoint_values: values,
        sample,
    })
}

fn local_box(ctx: &Context, field: &Field, start: Sample, stop: Sample) -> Result<LocalBox> {
    let bits = ctx.control.bits;
    let time = [start.t, stop.t];
    let step = Interval::point(stop.t, bits)?.sub(&Interval::point(start.t, bits)?)?;
    let dt = Interval::point(0.0, bits)?.hull(&step);
    let mut tube: [[f64; 2]; 2] = std::array::from_fn(|i| {
        let a = start.state[i];
        let b = stop.state[i];
        pad(a.min(b), a.max(b))
    });
    let origins = [
        Interval::point(start.state[0], bits)?,
        Interval::point(start.state[1], bits)?,
    ];
    let mut proof = None;
    for _ in 0..8 {
        let slope = field.bounds(ctx, time, tube[0], tube[1])?;
        let mut encloses = true;
        for i in 0..2 {
            let [lo, hi] = origins[i].add(&dt.mul(&slope[i])?)?.bounds()?;
            if lo <= tube[i][0] || hi >= tube[i][1] {
                tube[i] = pad(lo.min(tube[i][0]), hi.max(tube[i][1]));
                encloses = false;
            }
        }
        if encloses {
            proof = Some(slope);
            break;
        }
    }
    let slope = proof.ok_or(Issue::Split("Picard image escapes the proposed tube"))?;
    let mut endpoint = [[0.0; 2]; 2];
    for i in 0..2 {
        endpoint[i] = origins[i].add(&step.mul(&slope[i])?)?.bounds()?;
    }
    let row_norm = jacobian_bound(ctx, field, [time, tube[0], tube[1]])?;
    let contraction = row_norm.mul(&step)?.bounds()?[1];
    if contraction >= 1.0 {
        return Err(Issue::Split("Picard operator is not proved contractive"));
    }
    Ok(LocalBox {
        start,
        time,
        tube,
        endpoint,
        contraction_upper: contraction,
    })
}

fn pad(lo: f64, hi: f64) -> [f64; 2] {
    let slack = 0.25 * (hi - lo) + 1e-9 * (1.0 + lo.abs().max(hi.abs()));
    [lo - slack, hi + slack]
}

fn jacobian_bound(ctx: &Context, field: &Field, [time, x, y]: [[f64; 2]; 3]) -> Result<Interval> {
    let bits = ctx.control.bits;
    let mut norm = Interval::point(0.0, bits)?;
    for expr in [field.dx, field.dy] {
        let expanded = inline(expr, ctx.program)?;
        let mut row = Interval::point(0.0, bits)?;
        for axis in ["x", "y"] {
            let diff = derivative(&expanded, axis, ctx.control.angle)?;
            let entry = ctx
                .real(&diff, &[("t", time), ("x", x), ("y", y)])?
                .enclose(bits)?
                .unary(crate::mp::Unary::Abs)?;
            row = row.add(&entry)?;
        }
        norm = norm.hull(&row);
    }
    Ok(norm)
}

#[derive(Debug, Serialize)]
pub struct Arrow {
    pub origin: Vector,
    pub vector: Vector,
}

pub fn sample_field(ctx: &Context, field: &Field, view: View, grid: usize) -> Result<Vec<Arrow>> {
    let view = view.validate()?;
    if !(3..=41).contains(&grid) || grid * grid > ctx.control.budget {
        return Err(Issue::Budget);
    }
    let mut arrows = Vec::new();
    for j in 0..grid {
        let y = view.y[0] + (j as f64 + 0.5) / grid as f64 * (view.y[1] - view.y[0]);
        for i in 0..grid {
            let x = view.x[0] + (i as f64 + 0.5) / grid as f64 * (view.x[1] - view.x[0]);
            if let Ok(vector) = field.at(ctx, 0.0, [x, y]) {
                arrows.push(Arrow {
                    origin: [x, y],
                    vector,
                });
            }
        }
    }
    Ok(arrows)
}
