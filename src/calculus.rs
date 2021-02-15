use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::{
    differentiate::{derivative, inline},
    graph::{Context, middle, valid_domain},
    interval::{Interval, Issue, Result},
    lang::Expr,
    mp::Unary,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Root {
    pub witness: [f64; 2],
    pub bracket: [f64; 2],
    pub endpoint_signs: [i8; 2],
    pub derivative: [f64; 2],
}

#[derive(Debug, Serialize)]
pub struct Roots {
    pub unique: Vec<Root>,
    pub unresolved: Vec<[f64; 2]>,
    pub excluded: Vec<[f64; 2]>,
    pub processed: usize,
}

fn interval(ctx: &Context, expr: &Expr, x: [f64; 2]) -> Result<Interval> {
    ctx.real(expr, &[("x", x)])?.enclose(ctx.control.bits)
}

pub fn first_derivative(ctx: &Context, expr: &Expr) -> Result<Expr> {
    derivative(&inline(expr, ctx.program)?, "x", ctx.control.angle)
}

fn signs(ctx: &Context, expr: &Expr, [lo, hi]: [f64; 2]) -> Option<[i8; 2]> {
    let sign = |x| -> Option<i8> {
        match interval(ctx, expr, [x, x]).ok()?.sign()? {
            Ordering::Less => Some(-1),
            Ordering::Equal => Some(0),
            Ordering::Greater => Some(1),
        }
    };
    Some([sign(lo)?, sign(hi)?])
}

fn is_bracket([a, b]: [i8; 2]) -> bool {
    a != b
}

pub fn root_witness(
    ctx: &Context,
    expr: &Expr,
    diff: &Expr,
    witness: [f64; 2],
) -> Result<([i8; 2], Interval)> {
    if !valid_domain(witness) {
        return Err(Issue::Range);
    }
    interval(ctx, expr, witness)?;
    let slope = interval(ctx, diff, witness)?;
    if slope.contains_zero() {
        return Err(Issue::Split(
            "Derivative may vanish on the witness interval",
        ));
    }
    let signs = signs(ctx, expr, witness)
        .filter(|s| is_bracket(*s))
        .ok_or(Issue::Split("Endpoints do not prove existence"))?;
    Ok((signs, slope))
}

pub fn roots(ctx: &Context, expr: &Expr, domain: [f64; 2], tolerance: f64) -> Result<Roots> {
    if !valid_domain(domain) || !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(Issue::Unsupported("Invalid root domain or tolerance"));
    }
    let controls = ctx.control.check()?;
    let symbolic = first_derivative(ctx, expr).ok();
    let mut work = vec![(domain, 0)];
    let mut out = Roots {
        unique: Vec::new(),
        unresolved: Vec::new(),
        excluded: Vec::new(),
        processed: 0,
    };
    while let Some((segment, depth)) = work.pop() {
        if out.processed == controls.budget {
            out.unresolved.push(segment);
            continue;
        }
        out.processed += 1;
        match interval(ctx, expr, segment) {
            Ok(value) if !value.contains_zero() => {
                out.excluded.push(segment);
                continue;
            }
            Err(Issue::Excluded(_)) => {
                out.excluded.push(segment);
                continue;
            }
            _ => {}
        }
        if let Some(diff) = &symbolic {
            if let Ok((s, slope)) = root_witness(ctx, expr, diff, segment) {
                let bracket = narrow(ctx, expr, diff, segment, s, tolerance);
                out.unique.push(Root {
                    witness: segment,
                    bracket,
                    endpoint_signs: s,
                    derivative: slope.bounds()?,
                });
                continue;
            }
        }
        let midpoint = middle(segment);
        if depth >= 52 || segment[1] - segment[0] <= tolerance || segment.contains(&midpoint) {
            out.unresolved.push(segment);
        } else {
            work.push(([midpoint, segment[1]], depth + 1));
            work.push(([segment[0], midpoint], depth + 1));
        }
    }
    Ok(out)
}

fn narrow(
    ctx: &Context,
    expr: &Expr,
    diff: &Expr,
    mut bracket: [f64; 2],
    mut ends: [i8; 2],
    tolerance: f64,
) -> [f64; 2] {
    for _ in 0..80 {
        if ends[0] == 0 {
            return [bracket[0]; 2];
        }
        if ends[1] == 0 {
            return [bracket[1]; 2];
        }
        if bracket[1] - bracket[0] <= tolerance {
            break;
        }
        let midpoint = middle(bracket);
        if bracket.contains(&midpoint) {
            break;
        }
        let newton = newton_image(ctx, expr, diff, bracket, midpoint);
        if let Ok(image) = newton {
            if image[1] - image[0] < 0.7 * (bracket[1] - bracket[0]) {
                if image[0] == image[1] && signs(ctx, expr, image) == Some([0, 0]) {
                    return image;
                }
                if let Some(s) = signs(ctx, expr, image).filter(|s| is_bracket(*s)) {
                    bracket = image;
                    ends = s;
                    continue;
                }
            }
        }
        let Some([_, s]) = signs(ctx, expr, [bracket[0], midpoint]) else {
            break;
        };
        if s == 0 {
            return [midpoint; 2];
        }
        if s == ends[0] {
            bracket[0] = midpoint;
        } else {
            bracket[1] = midpoint;
        }
    }
    bracket
}

fn newton_image(
    ctx: &Context,
    expr: &Expr,
    diff: &Expr,
    domain: [f64; 2],
    midpoint: f64,
) -> Result<[f64; 2]> {
    let bits = ctx.control.bits;
    let slope = interval(ctx, diff, domain)?;
    let value = interval(ctx, expr, [midpoint, midpoint])?;
    let image = Interval::point(midpoint, bits)?.sub(&value.div(&slope)?)?;
    image
        .intersect(&Interval::new(domain[0], domain[1], bits)?)
        .ok_or(Issue::Split("Empty interval Newton image"))?
        .bounds()
}

pub fn verify_root(ctx: &Context, expr: &Expr, root: &Root) -> Result<()> {
    let diff = first_derivative(ctx, expr)?;
    let (signs_claimed, slope) = root_witness(ctx, expr, &diff, root.witness)?;
    let [dlo, dhi] = slope.bounds()?;
    if root.endpoint_signs != signs_claimed
        || root.derivative[0] > dlo
        || root.derivative[1] < dhi
        || root.derivative[0] <= 0.0 && root.derivative[1] >= 0.0
    {
        return Err(Issue::Unsupported("False uniqueness evidence"));
    }
    let [a, b] = root.bracket;
    if a < root.witness[0] || b > root.witness[1] || a > b {
        return Err(Issue::Unsupported("Root bracket is outside its witness"));
    }
    let proven = if a == b {
        interval(ctx, expr, [a, a])?.sign() == Some(Ordering::Equal)
    } else {
        signs(ctx, expr, [a, b]).is_some_and(is_bracket)
    };
    if !proven {
        return Err(Issue::Unsupported("Bracket does not establish existence"));
    }
    Ok(())
}

pub fn verify_root_coverage(
    ctx: &Context,
    expr: &Expr,
    domain: [f64; 2],
    roots: &[Root],
    excluded: &[[f64; 2]],
    unresolved: &[[f64; 2]],
) -> Result<()> {
    if !valid_domain(domain)
        || roots.len() + excluded.len() + unresolved.len() > ctx.control.budget + 64
    {
        return Err(Issue::Unsupported("Invalid root coverage or budget"));
    }
    for root in roots {
        verify_root(ctx, expr, root)?;
    }
    for &cell in excluded {
        let proven = match interval(ctx, expr, cell) {
            Ok(bound) => !bound.contains_zero(),
            Err(Issue::Excluded(_)) => true,
            _ => false,
        };
        if !proven {
            return Err(Issue::Unsupported(
                "A claimed excluded root cell may contain a zero",
            ));
        }
    }
    let mut cover: Vec<_> = roots
        .iter()
        .map(|r| r.witness)
        .chain(excluded.iter().copied())
        .chain(unresolved.iter().copied())
        .collect();
    cover.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut cursor = domain[0];
    for cell in cover {
        if cell[0] != cursor || cell[0] >= cell[1] {
            return Err(Issue::Unsupported(
                "Root coverage has a gap, overlap or reversed interval",
            ));
        }
        cursor = cell[1];
    }
    if cursor != domain[1] {
        return Err(Issue::Unsupported("Root coverage is incomplete"));
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct StationaryPoint {
    pub root: Root,
    pub classification: &'static str,
    pub residual: Option<[f64; 2]>,
}

#[derive(Debug, Serialize)]
pub struct Stationary {
    pub points: Vec<StationaryPoint>,
    pub unresolved: Vec<[f64; 2]>,
}

pub fn stationary(
    ctx: &Context,
    expr: &Expr,
    domain: [f64; 2],
    tolerance: f64,
    tangent: bool,
) -> Result<(Expr, Stationary)> {
    let differential = first_derivative(ctx, expr)?;
    let candidates = roots(ctx, &differential, domain, tolerance)?;
    let mut result = Stationary {
        points: Vec::new(),
        unresolved: candidates.unresolved,
    };
    for root in candidates.unique {
        if root.bracket[0] == root.bracket[1] && domain.contains(&root.bracket[0]) {
            continue;
        }
        let original = interval(ctx, expr, root.witness);
        let residual = interval(ctx, expr, root.bracket);
        if original.is_err() || residual.is_err() {
            result.unresolved.push(root.bracket);
            continue;
        }
        let residual = residual?;
        if tangent {
            if residual.sign() == Some(Ordering::Equal) {
                result.points.push(StationaryPoint {
                    root,
                    classification: "tangency",
                    residual: Some(residual.bounds()?),
                });
            } else if residual.contains_zero() {
                result.unresolved.push(root.bracket);
            }
        } else {
            let classification = if root.derivative[0] > 0.0 {
                "local_minimum"
            } else {
                "local_maximum"
            };
            result.points.push(StationaryPoint {
                root,
                classification,
                residual: Some(residual.bounds()?),
            });
        }
    }
    Ok((differential, result))
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    Simpson,
    Range,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Panel {
    pub domain: [f64; 2],
    pub contribution: [f64; 2],
    pub rule: Rule,
}

#[derive(Debug, Serialize)]
pub struct Integral {
    pub enclosure: Option<[f64; 2]>,
    pub target_met: bool,
    pub panels: Vec<Panel>,
    pub unresolved: Vec<[f64; 2]>,
    pub processed: usize,
}

fn fourth(ctx: &Context, expr: &Expr) -> Option<Expr> {
    let mut derivative_expr = inline(expr, ctx.program).ok()?;
    for _ in 0..4 {
        derivative_expr = derivative(&derivative_expr, "x", ctx.control.angle).ok()?;
    }
    Some(derivative_expr)
}

fn panel_bound(
    ctx: &Context,
    expr: &Expr,
    derivative: Option<&Expr>,
    domain: [f64; 2],
    rule: Rule,
) -> Result<Interval> {
    let [a, b] = domain;
    let bits = ctx.control.bits;
    let pa = Interval::point(a, bits)?;
    let pb = Interval::point(b, bits)?;
    let width = pb.sub(&pa)?;
    let range = interval(ctx, expr, domain)?;
    match rule {
        Rule::Range => width.mul(&range),
        Rule::Simpson => {
            let d4 = derivative.ok_or(Issue::Unsupported("Fourth derivative unavailable"))?;
            let c = pa.add(&pb)?.div(&Interval::point(2.0, bits)?)?;
            let fc = interval(ctx, expr, c.bounds()?)?;
            let sum = interval(ctx, expr, [a, a])?
                .add(&Interval::point(4.0, bits)?.mul(&fc)?)?
                .add(&interval(ctx, expr, [b, b])?)?;
            let estimate = sum.mul(&width)?.div(&Interval::point(6.0, bits)?)?;
            let cap = interval(ctx, d4, domain)?.unary(Unary::Abs)?;
            let error = cap
                .mul(&width.pow(5)?)?
                .div(&Interval::point(2880.0, bits)?)?;
            let symmetric = error.neg()?.hull(&error);
            estimate.add(&symmetric)
        }
    }
}

pub fn integrate(ctx: &Context, expr: &Expr, domain: [f64; 2], tolerance: f64) -> Result<Integral> {
    if !valid_domain(domain) || !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(Issue::Unsupported(
            "Invalid integration domain or tolerance",
        ));
    }
    let controls = ctx.control.check()?;
    let d4 = fourth(ctx, expr);
    let mut work = vec![(domain, 0)];
    let mut out = Integral {
        enclosure: None,
        target_met: false,
        panels: Vec::new(),
        unresolved: Vec::new(),
        processed: 0,
    };
    let mut total = Interval::point(0.0, controls.bits)?;
    let width = domain[1] - domain[0];
    while let Some((cell, depth)) = work.pop() {
        if out.processed >= controls.budget {
            out.unresolved.push(cell);
            continue;
        }
        out.processed += 1;
        let candidate = panel_bound(ctx, expr, d4.as_ref(), cell, Rule::Simpson)
            .map(|bound| (Rule::Simpson, bound))
            .or_else(|_| {
                panel_bound(ctx, expr, None, cell, Rule::Range).map(|bound| (Rule::Range, bound))
            });
        let m = middle(cell);
        let terminal = depth >= 24 || cell.contains(&m);
        let local = tolerance * ((cell[1] - cell[0]) / width);
        match candidate.and_then(|(rule, bound)| Ok((rule, bound.bounds()?))) {
            Ok((rule, contribution)) if terminal || contribution[1] - contribution[0] <= local => {
                total = total.add(&Interval::new(
                    contribution[0],
                    contribution[1],
                    controls.bits,
                )?)?;
                out.panels.push(Panel {
                    domain: cell,
                    contribution,
                    rule,
                });
            }
            _ if terminal => out.unresolved.push(cell),
            _ => {
                work.push(([m, cell[1]], depth + 1));
                work.push(([cell[0], m], depth + 1));
            }
        }
    }
    if out.unresolved.is_empty() {
        let enclosure = total.bounds()?;
        out.target_met = enclosure[1] - enclosure[0] <= tolerance;
        out.enclosure = Some(enclosure);
    }
    Ok(out)
}

pub fn verify_integral(
    ctx: &Context,
    expr: &Expr,
    domain: [f64; 2],
    enclosure: [f64; 2],
    panels: &[Panel],
) -> Result<()> {
    if panels.is_empty() || panels.len() > ctx.control.budget {
        return Err(Issue::Unsupported("Missing or excessive quadrature panels"));
    }
    let d4 = fourth(ctx, expr);
    let mut end = domain[0];
    let mut total = Interval::point(0.0, ctx.control.bits)?;
    for panel in panels {
        if end != panel.domain[0] || !valid_domain(panel.domain) {
            return Err(Issue::Unsupported(
                "Panels do not tile the requested domain",
            ));
        }
        end = panel.domain[1];
        let [lo, hi] = panel_bound(ctx, expr, d4.as_ref(), panel.domain, panel.rule)?.bounds()?;
        if panel.contribution[0] > lo || panel.contribution[1] < hi {
            return Err(Issue::Unsupported(
                "Panel excludes recomputed quadrature bounds",
            ));
        }
        total = total.add(&Interval::new(
            panel.contribution[0],
            panel.contribution[1],
            ctx.control.bits,
        )?)?;
    }
    let [lo, hi] = total.bounds()?;
    if end != domain[1] || enclosure[0] > lo || enclosure[1] < hi {
        return Err(Issue::Unsupported("Integral excludes covered panel sum"));
    }
    Ok(())
}
