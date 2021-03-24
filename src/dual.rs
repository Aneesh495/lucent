use crate::{
    differentiate::inline,
    eval::Angle,
    graph::Context,
    interval::{Interval, Issue, Result},
    lang::{Expr, Kind, Op},
    mp::Unary,
    value::{Real, Truth},
};

pub struct Dual {
    pub value: Real,
    pub tangent: Real,
}

pub fn forward(ctx: &Context, expr: &Expr, point: f64) -> Result<Dual> {
    let expanded = inline(expr, ctx.program)?;
    sweep(ctx, &expanded, point)
}

fn sweep(ctx: &Context, expr: &Expr, point: f64) -> Result<Dual> {
    let bits = ctx.control.bits;
    let n = Real::integer;
    let binary = |a: &Real, op, b: &Real| a.arithmetic(op, b, bits);
    let apply = |a: &Real, op| a.unary(op, bits);
    let at = &[("x", [point; 2])];
    match &expr.kind {
        Kind::Literal(_) | Kind::Name(_) => Ok(Dual {
            value: ctx.real(expr, at)?,
            tangent: n(i64::from(
                matches!(&expr.kind, Kind::Name(name) if name == "x"),
            )),
        }),
        Kind::Negative(a) => {
            let v = sweep(ctx, a, point)?;
            Ok(Dual {
                value: v.value.neg()?,
                tangent: v.tangent.neg()?,
            })
        }
        Kind::Binary(op, a, b) => {
            let a = sweep(ctx, a, point)?;
            let b = sweep(ctx, b, point)?;
            let value = binary(&a.value, *op, &b.value)?;
            let tangent = match op {
                Op::Add | Op::Sub => binary(&a.tangent, *op, &b.tangent)?,
                Op::Mul | Op::Div => {
                    let x = binary(&a.tangent, Op::Mul, &b.value)?;
                    let y = binary(&a.value, Op::Mul, &b.tangent)?;
                    if *op == Op::Mul {
                        binary(&x, Op::Add, &y)?
                    } else {
                        binary(&binary(&x, Op::Sub, &y)?, Op::Div, &b.value.power(2, bits)?)?
                    }
                }
                Op::Pow if b.tangent.is_zero() => {
                    if b.value.is_zero() {
                        n(0)
                    } else {
                        let prior_power = binary(&b.value, Op::Sub, &n(1))?;
                        let prior = a.value.rational_power(&prior_power, bits)?;
                        binary(&binary(&b.value, Op::Mul, &prior)?, Op::Mul, &a.tangent)?
                    }
                }
                _ => {
                    return Err(Issue::Unsupported(
                        "Forward derivative is unsupported for this operator",
                    ));
                }
            };
            Ok(Dual { value, tangent })
        }
        Kind::Call(name, args) if name == "if" => {
            let [test, yes, no] = args.as_slice() else {
                return Err(Issue::Unsupported("Expected a predicate and two branches"));
            };
            let Kind::Binary(op, a, b) = &test.kind else {
                return Err(Issue::Unsupported("A comparison predicate is required"));
            };
            if !op.compares() {
                return Err(Issue::Unsupported("A comparison predicate is required"));
            }
            let left = ctx.real(a, at)?;
            let right = ctx.real(b, at)?;
            if left.compare(Op::Ne, &right, bits)? != Truth::True {
                return Err(Issue::Split("Piecewise derivative at a boundary"));
            }
            match left.compare(*op, &right, bits)? {
                Truth::True => sweep(ctx, yes, point),
                Truth::False => sweep(ctx, no, point),
                Truth::Unknown => Err(Issue::Split("Piecewise branch is undecidable")),
            }
        }
        Kind::Call(name, args) => {
            let [arg] = args.as_slice() else {
                return Err(Issue::Unsupported(
                    "Forward mode needs a supported scalar function",
                ));
            };
            let a = sweep(ctx, arg, point)?;
            let v = a.value;
            let angle_scale = || -> Result<Real> {
                match ctx.control.angle {
                    Angle::Radians => Ok(n(1)),
                    Angle::Degrees => binary(&Real::Bounded(Interval::pi(bits)?), Op::Div, &n(180)),
                }
            };
            let (value, factor) = match name.as_str() {
                "sin" | "cos" | "tan" => {
                    let scale = angle_scale()?;
                    let arg = binary(&v, Op::Mul, &scale)?;
                    let op = match name.as_str() {
                        "sin" => Unary::Sin,
                        "cos" => Unary::Cos,
                        _ => Unary::Tan,
                    };
                    let slope = match op {
                        Unary::Sin => apply(&arg, Unary::Cos)?,
                        Unary::Cos => apply(&arg, Unary::Sin)?.neg()?,
                        _ => binary(&n(1), Op::Div, &apply(&arg, Unary::Cos)?.power(2, bits)?)?,
                    };
                    (apply(&arg, op)?, binary(&slope, Op::Mul, &scale)?)
                }
                "exp" => {
                    let value = apply(&v, Unary::Exp)?;
                    (value.clone(), value)
                }
                "ln" | "log" => (apply(&v, Unary::Log)?, binary(&n(1), Op::Div, &v)?),
                "sqrt" => {
                    let value = apply(&v, Unary::Sqrt)?;
                    let denom = binary(&n(2), Op::Mul, &value)?;
                    (value, binary(&n(1), Op::Div, &denom)?)
                }
                "abs" => {
                    let slope = match v.sign() {
                        Some(std::cmp::Ordering::Greater) => n(1),
                        Some(std::cmp::Ordering::Less) => n(-1),
                        _ => return Err(Issue::Split("Absolute-value derivative at a cusp")),
                    };
                    (apply(&v, Unary::Abs)?, slope)
                }
                "atan" => {
                    let scale = angle_scale()?;
                    let den = binary(&n(1), Op::Add, &v.power(2, bits)?)?;
                    (
                        binary(&apply(&v, Unary::Atan)?, Op::Div, &scale)?,
                        binary(&n(1), Op::Div, &binary(&den, Op::Mul, &scale)?)?,
                    )
                }
                _ => {
                    return Err(Issue::Unsupported(
                        "Forward derivative is unsupported for this function",
                    ));
                }
            };
            Ok(Dual {
                value,
                tangent: binary(&factor, Op::Mul, &a.tangent)?,
            })
        }
        Kind::List(_) => Err(Issue::Unsupported("Forward mode requires a scalar")),
    }
}
