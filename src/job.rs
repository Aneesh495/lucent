use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};

use crate::{
    calculus,
    differentiate::inline,
    eval::{Angle, Eval, EvalError, Scope},
    exact,
    graph::{self, Context, Controls, CurveCell, CurveProblem, View},
    interval::{Interval, Issue},
    lang::{Diagnostic, Expr, Kind, Op, parse},
    mp, polynomial,
    program::Program,
    value::{Real, Truth, Value},
};

fn bits() -> u32 {
    96
}
fn max_bits() -> u32 {
    384
}
fn budget() -> usize {
    4096
}
fn tolerance() -> f64 {
    1e-12
}
fn pixels() -> f64 {
    0.45
}

#[derive(Deserialize, Serialize)]
pub struct Job {
    #[serde(default)]
    definitions: Vec<String>,
    #[serde(default)]
    parameters: BTreeMap<String, String>,
    #[serde(default)]
    angle: Angle,
    #[serde(default = "bits")]
    bits: u32,
    #[serde(default = "max_bits")]
    max_bits: u32,
    #[serde(default = "budget")]
    budget: usize,
    #[serde(default = "tolerance")]
    tolerance: f64,
    #[serde(default = "pixels")]
    pixel_error: f64,
    #[serde(flatten)]
    action: Action,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    #[default]
    Explicit,
    Parametric,
    Polar,
    Implicit,
    Region,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Action {
    Evaluate {
        expr: String,
        #[serde(default)]
        bindings: BTreeMap<String, String>,
    },
    Enclose {
        expr: String,
        domain: [f64; 2],
    },
    Graph {
        expr: String,
        #[serde(default)]
        mode: Mode,
        horizontal: Option<String>,
        domain: Option<[f64; 2]>,
        view: View,
    },
    Roots {
        expr: String,
        domain: [f64; 2],
    },
    Polynomial {
        expr: String,
        domain: [String; 2],
        #[serde(default = "exact_tolerance")]
        width: String,
    },
    Integral {
        expr: String,
        domain: [f64; 2],
    },
    Derivative {
        expr: String,
        at: f64,
    },
    Table {
        expr: String,
        samples: Vec<f64>,
    },
    Inspect {
        expr: String,
    },
}

fn exact_tolerance() -> String {
    "1/1000000000000".into()
}

enum Error {
    Syntax(Diagnostic),
    Eval(EvalError),
    Issue(Issue),
}

impl From<Diagnostic> for Error {
    fn from(error: Diagnostic) -> Self {
        Self::Syntax(error)
    }
}
impl From<EvalError> for Error {
    fn from(error: EvalError) -> Self {
        Self::Eval(error)
    }
}
impl From<Issue> for Error {
    fn from(error: Issue) -> Self {
        Self::Issue(error)
    }
}

impl Error {
    fn response(self) -> Json {
        match self {
            Self::Syntax(error) => {
                json!({"status":"error","code":"syntax-or-dependency","message":error.message,"span":error.span})
            }
            Self::Eval(error) => issue_response(error.issue, Some(error.span)),
            Self::Issue(error) => issue_response(error, None),
        }
    }
}

fn issue_response(issue: Issue, span: Option<crate::lang::Span>) -> Json {
    let status = match issue {
        Issue::Excluded(_) => "undefined",
        Issue::Unsupported(_) => "unsupported",
        _ => "unresolved",
    };
    json!({"status":status,"code":issue.code(),"message":issue.to_string(),"span":span})
}

type Result<T> = std::result::Result<T, Error>;

impl Job {
    fn context_data(&self) -> Result<(Program, Scope, Controls)> {
        if self.parameters.len() > 32
            || self.max_bits < self.bits
            || self.max_bits > 512
            || !(0.0 < self.tolerance && self.tolerance.is_finite())
        {
            return Err(
                Issue::Unsupported("Invalid parameter count, precision or tolerance").into(),
            );
        }
        let program = Program::compile(&self.definitions, self.parameters.keys())?;
        let mut scope = Scope::new();
        for (name, number) in &self.parameters {
            scope.insert(
                name.clone(),
                Value::real(Real::Exact(exact::ratio(number)?)),
            );
        }
        let controls = Controls {
            bits: self.bits,
            angle: self.angle,
            budget: self.budget,
            tolerance: self.pixel_error,
        }
        .check()?;
        Ok((program, scope, controls))
    }

    fn execute(&self) -> Result<Json> {
        let (program, scope, controls) = self.context_data()?;
        let ctx = Context {
            program: &program,
            scope: &scope,
            control: controls,
        };
        match &self.action {
            Action::Evaluate { expr, bindings } => {
                if bindings.len() > 6 {
                    return Err(Issue::Budget.into());
                }
                let mut scope = scope.clone();
                for (key, value) in bindings {
                    if !matches!(key.as_str(), "x" | "y" | "z" | "t" | "u" | "v") {
                        return Err(Issue::Unsupported(
                            "Bindings may only set x, y, z, t, u, or v",
                        )
                        .into());
                    }
                    scope.insert(key.clone(), Value::real(Real::Exact(exact::ratio(value)?)));
                }
                self.scalar(&expression(expr)?, &program, &scope, None)
            }
            Action::Enclose { expr, domain } => {
                if !graph::valid_domain(*domain) {
                    return Err(Issue::Range.into());
                }
                self.scalar(&expression(expr)?, &program, &scope, Some(*domain))
            }
            Action::Graph {
                expr,
                mode,
                horizontal,
                domain,
                view,
            } => {
                let vertical = expression(expr)?;
                let view = view.validate()?;
                let result = match mode {
                    Mode::Implicit | Mode::Region => {
                        let residual = if matches!(mode, Mode::Implicit) {
                            if let Kind::Binary(Op::Eq, a, b) = &vertical.kind {
                                Expr::binary(Op::Sub, *a.clone(), *b.clone(), vertical.span)
                            } else {
                                vertical
                            }
                        } else {
                            vertical
                        };
                        json!(graph::area(
                            &ctx,
                            &residual,
                            view,
                            matches!(mode, Mode::Region)
                        )?)
                    }
                    _ => {
                        let (horizontal, vertical, axis) =
                            curve_axes(*mode, horizontal.as_deref(), vertical)?;
                        let problem = CurveProblem {
                            horizontal: horizontal.as_ref(),
                            vertical: &vertical,
                            axis,
                            domain: domain.unwrap_or(view.x),
                            view,
                        };
                        json!(graph::curve(&ctx, &problem)?)
                    }
                };
                Ok(
                    json!({"status":"approximate_geometry","precision":self.bits,"result":result,
                    "assumptions":["Closed-domain interval checks","Geometry has a separate numerical error model","Unresolved cells remain visible"]}),
                )
            }
            Action::Roots { expr, domain } => {
                let expr = equation(expr)?;
                let result = calculus::roots(&ctx, &expr, *domain, self.tolerance)?;
                Ok(
                    json!({"status": if result.unresolved.is_empty() { "root_certificate" } else { "partial" },
                    "precision":self.bits,"equation":expr.to_string(),"result":result,
                    "assumptions":["Proved continuity on each root witness","Nonzero interval derivative for uniqueness","Endpoint sign or exact-zero evidence"]}),
                )
            }
            Action::Polynomial {
                expr,
                domain,
                width,
            } => {
                let expr = inline(&equation(expr)?, &program)?;
                let result =
                    polynomial::isolate(&expr, [&domain[0], &domain[1]], width, self.budget)?;
                Ok(
                    json!({"status":if result.unresolved.is_empty() { "exact_root_count" } else { "partial" },
                    "equation":expr.to_string(),"result":result,
                    "assumptions":["Rational coefficients","Closed requested domain","Nonpoint isolators are open","Multiplicity from square-free factors"]}),
                )
            }
            Action::Integral { expr, domain } => {
                let expr = expression(expr)?;
                let result = calculus::integrate(&ctx, &expr, *domain, self.tolerance)?;
                Ok(
                    json!({"status":if result.enclosure.is_some() { "integral_certificate" } else { "unresolved" },
                    "precision":self.bits,"result":result,
                    "assumptions":["Full finite interval coverage required","Simpson panels use an enclosed fourth derivative","Range panels use an interval range times panel width"]}),
                )
            }
            Action::Derivative { expr, at } => {
                let expr = expression(expr)?;
                let derivative = calculus::first_derivative(&ctx, &expr)?;
                ctx.real(&expr, &[("x", [*at; 2])])?;
                let real = ctx.real(&derivative, &[("x", [*at; 2])])?;
                let bound = real.enclose(self.bits)?.bounds()?;
                let h = f64::EPSILON.cbrt() * at.abs().max(1.0);
                let sample = |x| -> Option<f64> {
                    Some(graph::middle(
                        ctx.real(&expr, &[("x", [x; 2])])
                            .ok()?
                            .enclose(self.bits)
                            .ok()?
                            .bounds()
                            .ok()?,
                    ))
                };
                let estimate =
                    |h| -> Option<f64> { Some((sample(at + h)? - sample(at - h)?) / (2.0 * h)) };
                let finite_difference = estimate(h);
                let comparison = finite_difference
                    .zip(estimate(h * 0.5))
                    .map(|(a, b)| (a - b).abs());
                Ok(
                    json!({"status":"derivative","precision":self.bits,"equation":derivative.to_string(),
                    "result":{"symbolic":bound,"finite_difference":finite_difference,"step":h,"step_disagreement":comparison}}),
                )
            }
            Action::Table { expr, samples } => {
                if samples.is_empty() || samples.len() > 512 {
                    return Err(Issue::Budget.into());
                }
                let expr = expression(expr)?;
                let values: Vec<_> = samples.iter().map(|x| {
                    let result = ctx.evaluate(&expr, &[("x", [*x; 2])]).and_then(|v| render_value(&v, self.bits));
                    json!({"x":x,"value":result.unwrap_or_else(|error| issue_response(error, None))})
                }).collect();
                Ok(json!({"status":"table","precision":self.bits,"result":values}))
            }
            Action::Inspect { expr } => Ok(
                json!({"status":"syntax","canonical":expression(expr)?.to_string(),
                "dependencies":program.dependencies(),"order":program.order()}),
            ),
        }
    }

    fn scalar(
        &self,
        expr: &Expr,
        program: &Program,
        scope: &Scope,
        domain: Option<[f64; 2]>,
    ) -> Result<Json> {
        let mut bits = self.bits;
        loop {
            let mut scoped = scope.clone();
            if let Some([a, b]) = domain {
                scoped.insert(
                    "x".into(),
                    Value::real(Real::Bounded(Interval::new(a, b, bits)?)),
                );
            }
            let evaluated = Eval::new(program, &scoped, bits, self.angle).run(expr);
            let output = match evaluated {
                Ok(value) => render_value(&value, bits)?,
                Err(error)
                    if matches!(error.issue, Issue::Split(_) | Issue::Range)
                        && bits < self.max_bits =>
                {
                    bits = (bits * 2).min(self.max_bits);
                    continue;
                }
                Err(error) => return Err(error.into()),
            };
            let mut target_met = None;
            if let Some([lo, hi]) = output
                .get("bounds")
                .and_then(|b| serde_json::from_value::<[f64; 2]>(b.clone()).ok())
            {
                let target = hi - lo <= self.tolerance * hi.abs().max(lo.abs()).max(1.0);
                target_met = Some(target);
                if !target && domain.is_none() && bits < self.max_bits {
                    bits = (bits * 2).min(self.max_bits);
                    continue;
                }
            }
            let status = output["status"].clone();
            return Ok(
                json!({"status":status,"precision":bits,"target_met":target_met,
                "canonical":expr.to_string(),"result":output,"mpfr":mp::version()}),
            );
        }
    }

    fn verify(&self, claim: Json) -> Result<()> {
        let (program, scope, mut control) = self.context_data()?;
        let precision = claim
            .get("precision")
            .and_then(Json::as_u64)
            .unwrap_or(u64::from(self.bits));
        control.bits = u32::try_from(precision)
            .ok()
            .filter(|p| *p >= self.bits && *p <= self.max_bits)
            .ok_or(Issue::Unsupported("Claim precision outside request"))?;
        let ctx = Context {
            program: &program,
            scope: &scope,
            control,
        };
        let result = claim
            .get("result")
            .ok_or(Issue::Unsupported("Missing claim result"))?;
        match &self.action {
            Action::Evaluate { .. } | Action::Enclose { .. } => {
                let fresh = self.execute()?;
                compare_scalar(result, &fresh["result"])?;
            }
            Action::Roots { expr, domain } => {
                let roots = serde_json::from_value::<Vec<calculus::Root>>(result["unique"].clone())
                    .map_err(|_| Issue::Unsupported("Invalid root claim"))?;
                if roots.is_empty() || roots.len() > control.budget {
                    return Err(Issue::Unsupported("Missing or excessive root witnesses").into());
                }
                let expr = equation(expr)?;
                for root in &roots {
                    if root.witness[0] < domain[0] || root.witness[1] > domain[1] {
                        return Err(
                            Issue::Unsupported("Root witness outside requested domain").into()
                        );
                    }
                    calculus::verify_root(&ctx, &expr, root)?;
                }
            }
            Action::Integral { expr, domain } => {
                let enclosure = serde_json::from_value::<[f64; 2]>(result["enclosure"].clone())
                    .map_err(|_| Issue::Unsupported("Missing integral enclosure"))?;
                let panels =
                    serde_json::from_value::<Vec<calculus::Panel>>(result["panels"].clone())
                        .map_err(|_| Issue::Unsupported("Missing quadrature panel evidence"))?;
                calculus::verify_integral(&ctx, &expression(expr)?, *domain, enclosure, &panels)?;
            }
            Action::Graph {
                expr,
                mode,
                horizontal,
                domain,
                view,
            } if !matches!(mode, Mode::Implicit | Mode::Region) => {
                let (horizontal, vertical, axis) =
                    curve_axes(*mode, horizontal.as_deref(), expression(expr)?)?;
                let problem = CurveProblem {
                    horizontal: horizontal.as_ref(),
                    vertical: &vertical,
                    axis,
                    domain: domain.unwrap_or(view.x),
                    view: *view,
                };
                let cells = serde_json::from_value::<Vec<CurveCell>>(result["cells"].clone())
                    .map_err(|_| Issue::Unsupported("Malformed curve coverage"))?;
                verify_curve(&ctx, &problem, &cells)?;
            }
            _ => {
                return Err(
                    Issue::Unsupported("This claim type has no semantic verifier yet").into(),
                );
            }
        }
        Ok(())
    }
}

fn expression(source: &str) -> Result<Expr> {
    Ok(parse(source)?.expression()?)
}

fn equation(source: &str) -> Result<Expr> {
    let expression = expression(source)?;
    if let Kind::Binary(Op::Eq, a, b) = expression.kind {
        return Ok(Expr::binary(Op::Sub, *a, *b, expression.span));
    }
    Ok(expression)
}

fn curve_axes(
    mode: Mode,
    horizontal: Option<&str>,
    expr: Expr,
) -> Result<(Option<Expr>, Expr, &'static str)> {
    match mode {
        Mode::Explicit => Ok((None, expr, "x")),
        Mode::Parametric => Ok((
            Some(expression(horizontal.ok_or(Issue::Unsupported(
                "Parametric plots need a horizontal expression",
            ))?)?),
            expr,
            "t",
        )),
        Mode::Polar => {
            let span = expr.span;
            let t = Expr::new(Kind::Name("t".into()), span);
            let radial = |name: &str, radius: Expr| {
                Expr::binary(
                    Op::Mul,
                    radius,
                    Expr::new(Kind::Call(name.into(), vec![t.clone()]), span),
                    span,
                )
            };
            Ok((Some(radial("cos", expr.clone())), radial("sin", expr), "t"))
        }
        _ => Err(Issue::Unsupported("Mode is not a one-dimensional curve").into()),
    }
}

fn render_value(value: &Value, bits: u32) -> std::result::Result<Json, Issue> {
    fn real(value: &Real, bits: u32) -> std::result::Result<Json, Issue> {
        Ok(match value {
            Real::Exact(q) => json!({"status":"exact","value":q.to_string()}),
            Real::Bounded(i) => json!({"status":"enclosure","bounds":i.bounds()?,"precision":bits}),
        })
    }
    Ok(match value {
        Value::Quantity(q) => {
            let mut number = real(&q.real, bits)?;
            number["dimension"] = json!(q.dimension);
            number
        }
        Value::Complex(re, im) => {
            json!({"status":"complex","real":real(re,bits)?,"imaginary":real(im,bits)?})
        }
        Value::Bool(Truth::Unknown) => json!({"status":"unresolved","truth":"unknown"}),
        Value::Bool(truth) => json!({"status":"boolean","truth":truth}),
        Value::List(items) => {
            json!({"status":"list","items":items.iter().map(|item|render_value(item,bits)).collect::<std::result::Result<Vec<_>,_>>()?})
        }
    })
}

fn compare_scalar(claim: &Json, fresh: &Json) -> Result<()> {
    if claim["status"] != fresh["status"] || claim["dimension"] != fresh["dimension"] {
        return Err(Issue::Unsupported("Claim classification or units differ").into());
    }
    match fresh["status"].as_str() {
        Some("enclosure") => {
            let get = |v: &Json| {
                serde_json::from_value::<[f64; 2]>(v["bounds"].clone())
                    .map_err(|_| Issue::Unsupported("Invalid finite enclosure"))
            };
            let [lo, hi] = get(claim)?;
            let [a, b] = get(fresh)?;
            if lo > a || hi < b {
                return Err(Issue::Unsupported("Claim excludes the recomputed enclosure").into());
            }
        }
        Some("exact" | "boolean" | "complex" | "list") if claim == fresh => {}
        _ => return Err(Issue::Unsupported("Claim is not a verified scalar value").into()),
    }
    Ok(())
}

fn verify_curve(ctx: &Context, problem: &CurveProblem, cells: &[CurveCell]) -> Result<()> {
    if cells.is_empty() || cells.len() > ctx.control.budget + 64 {
        return Err(Issue::Unsupported("Missing or excessive curve cells").into());
    }
    let mut end = problem.domain[0];
    let mut segments = 0;
    for cell in cells {
        if cell.domain[0] != end || !graph::valid_domain(cell.domain) {
            return Err(Issue::Unsupported("Curve coverage has a gap or overlap").into());
        }
        end = cell.domain[1];
        if let Some(segment) = cell.segment {
            if cell.kind != graph::CellKind::Segment || !problem.domain_valid(ctx, cell.domain) {
                return Err(
                    Issue::Unsupported("Segment bridges an unproved or excluded domain").into(),
                );
            }
            if segment != problem.segment_endpoints(ctx, cell.domain)? {
                return Err(Issue::Unsupported(
                    "Stale geometry does not match the requested expression",
                )
                .into());
            }
            segments += 1;
        } else if cell.kind == graph::CellKind::Segment {
            return Err(Issue::Unsupported("Missing segment payload").into());
        }
    }
    if end != problem.domain[1] || segments == 0 {
        return Err(Issue::Unsupported("Coverage is incomplete or has no segments").into());
    }
    Ok(())
}

pub fn execute(value: Json) -> Json {
    let error = |message: String| json!({"status":"error","message":message});
    if value["op"] == "verify" {
        let Some(request) = value.get("request") else {
            return error("Missing verification request".into());
        };
        let Some(claim) = value.get("claim") else {
            return error("Missing claim".into());
        };
        return match serde_json::from_value::<Job>(request.clone()) {
            Ok(job) => match job.verify(claim.clone()) {
                Ok(()) => json!({"status":"verified"}),
                Err(reason) => json!({"status":"rejected","reason":reason.response()}),
            },
            Err(err) => error(err.to_string()),
        };
    }
    match serde_json::from_value::<Job>(value) {
        Ok(job) => job.execute().unwrap_or_else(Error::response),
        Err(err) => error(err.to_string()),
    }
}
