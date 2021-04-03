use serde::Serialize;

use crate::{
    graph::{Context, middle, valid_domain},
    interval::{Issue, Result},
    lang::Expr,
};

#[derive(Serialize, Debug)]
pub struct Heuristic {
    pub estimate: Option<f64>,
    pub estimated_error: Option<f64>,
    pub completed_panels: usize,
    pub processed: usize,
    pub unresolved: Vec<[f64; 2]>,
}

fn sample(ctx: &Context, expr: &Expr, x: f64) -> Result<f64> {
    Ok(middle(
        ctx.real(expr, &[("x", [x; 2])])?
            .enclose(ctx.control.bits)?
            .bounds()?,
    ))
}

fn simpson(a: f64, b: f64, [left, middle, right]: [f64; 3]) -> Option<f64> {
    let estimate = (b - a) * (left / 6.0 + middle / 1.5 + right / 6.0);
    estimate.is_finite().then_some(estimate)
}

#[derive(Clone, Copy)]
struct Tile {
    domain: [f64; 2],
    level: u8,
    samples: [Option<f64>; 3],
}

fn add_compensated(sum: &mut f64, correction: &mut f64, value: f64) {
    let adjusted = value - *correction;
    let total = *sum + adjusted;
    *correction = (total - *sum) - adjusted;
    *sum = total;
}

pub fn estimate(ctx: &Context, expr: &Expr, domain: [f64; 2], tolerance: f64) -> Result<Heuristic> {
    if !valid_domain(domain) || !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(Issue::Unsupported(
            "Invalid approximate integral domain or tolerance",
        ));
    }
    let budget = ctx.control.check()?.budget;
    let total_width = domain[1] - domain[0];
    let mut work = vec![Tile {
        domain,
        level: 0,
        samples: [None; 3],
    }];
    let mut out = Heuristic {
        estimate: None,
        estimated_error: None,
        completed_panels: 0,
        processed: 0,
        unresolved: Vec::new(),
    };
    let mut sum = 0.0;
    let mut error_sum = 0.0;
    let mut compensation = 0.0;
    let mut compensation_error = 0.0;
    while let Some(tile) = work.pop() {
        if out.processed >= budget {
            out.unresolved.push(tile.domain);
            continue;
        }
        out.processed += 1;
        let [a, b] = tile.domain;
        let c = middle(tile.domain);
        if tile.level >= 40 || !(a < c && c < b) {
            out.unresolved.push(tile.domain);
            continue;
        }
        let estimates = ctx.real(expr, &[("x", tile.domain)]);
        if matches!(estimates, Err(Issue::Excluded(_) | Issue::Unsupported(_))) {
            out.unresolved.push(tile.domain);
            continue;
        }
        let nodes = [a, a.midpoint(c), c, c.midpoint(b), b];
        let points: [Option<f64>; 5] = std::array::from_fn(|i| {
            let cached = if i % 2 == 0 {
                tile.samples[i / 2]
            } else {
                None
            };
            cached.or_else(|| sample(ctx, expr, nodes[i]).ok())
        });
        if let [Some(l), Some(lm), Some(m), Some(rm), Some(r)] = points {
            if estimates.is_ok() {
                if let (Some(whole), Some(left), Some(right)) = (
                    simpson(a, b, [l, m, r]),
                    simpson(a, c, [l, lm, m]),
                    simpson(c, b, [m, rm, r]),
                ) {
                    let delta = left + right - whole;
                    let error = delta.abs() / 15.0;
                    let estimate = left + right + delta / 15.0;
                    if error <= tolerance * ((b - a) / total_width) && estimate.is_finite() {
                        add_compensated(&mut sum, &mut compensation, estimate);
                        add_compensated(&mut error_sum, &mut compensation_error, error);
                        out.completed_panels += 1;
                        continue;
                    }
                }
            }
        }
        work.push(Tile {
            domain: [c, b],
            level: tile.level + 1,
            samples: [points[2], points[3], points[4]],
        });
        work.push(Tile {
            domain: [a, c],
            level: tile.level + 1,
            samples: [points[0], points[1], points[2]],
        });
    }
    if out.unresolved.is_empty() && sum.is_finite() && error_sum.is_finite() {
        out.estimate = Some(sum);
        out.estimated_error = Some(error_sum);
    }
    Ok(out)
}
