use num_traits::{One, Zero};
use serde::Serialize;

use crate::{
    exact::{self, Rational},
    graph::Context,
    interval::{Interval, Issue, Result},
    lang::{Expr, Kind, Op},
    mp::Unary,
    value::Real,
};

#[derive(Serialize, Debug)]
pub struct Tail {
    pub method: &'static str,
    pub enclosure: Option<[f64; 2]>,
    pub divergent: bool,
}

fn constant(expr: &Expr) -> Option<Rational> {
    match &expr.kind {
        Kind::Literal(s) => exact::decimal(s).ok(),
        Kind::Negative(a) => Some(-constant(a)?),
        Kind::Binary(Op::Div, a, b) => {
            let b = constant(b)?;
            if b.is_zero() {
                None
            } else {
                exact::checked(constant(a)? / b).ok()
            }
        }
        _ => None,
    }
}

fn axis(expr: &Expr) -> bool {
    matches!(&expr.kind, Kind::Name(name) if name == "x")
}

fn power(expr: &Expr) -> Option<Rational> {
    if axis(expr) {
        return Some(Rational::one());
    }
    if let Kind::Binary(Op::Pow, a, b) = &expr.kind {
        if axis(a) {
            return constant(b);
        }
    }
    None
}

fn family_power(expr: &Expr) -> Option<Rational> {
    if let Kind::Binary(Op::Div, a, b) = &expr.kind {
        if constant(a)?.is_one() {
            return Some(-power(b)?);
        }
    }
    power(expr)
}

fn linear(expr: &Expr) -> Option<Rational> {
    match &expr.kind {
        Kind::Name(name) if name == "x" => Some(Rational::one()),
        Kind::Negative(a) => Some(-linear(a)?),
        Kind::Binary(Op::Mul, a, b) if axis(a) => constant(b),
        Kind::Binary(Op::Mul, a, b) if axis(b) => constant(a),
        _ => None,
    }
}

pub fn integrate(ctx: &Context, expr: &Expr, lower: f64, upper: Option<f64>) -> Result<Tail> {
    let bits = ctx.control.bits;
    if let Some(upper) = upper {
        if lower != 0.0 || upper <= 0.0 || !upper.is_finite() {
            return Err(Issue::Unsupported(
                "Finite endpoint mode requires [0,b] with finite positive b",
            ));
        }
        let exponent = family_power(expr).ok_or(Issue::Split(
            "No supported finite endpoint proof for this expression",
        ))?;
        let p = exponent + Rational::one();
        let method = "Power test on [0,b]: b^(p+1)/(p+1) for x^p with p > -1";
        if p <= Rational::zero() {
            return Ok(Tail {
                method,
                enclosure: None,
                divergent: true,
            });
        }
        let exponent = Real::Exact(p);
        let value = Real::Bounded(Interval::point(upper, bits)?).rational_power(&exponent, bits)?;
        let enclosure = value
            .arithmetic(Op::Div, &exponent, bits)?
            .enclose(bits)?
            .bounds()?;
        return Ok(Tail {
            method,
            enclosure: Some(enclosure),
            divergent: false,
        });
    }
    let a = Interval::point(lower, bits)?;
    if let Some(exponent) = family_power(expr) {
        if lower <= 0.0 {
            return Err(Issue::Unsupported(
                "Power tail requires a positive lower endpoint",
            ));
        }
        let p = exponent + Rational::one();
        let method = "Power test on [a, +infinity): a^(p+1)/(-p-1) for x^p with p < -1";
        if p >= Rational::zero() {
            return Ok(Tail {
                method,
                enclosure: None,
                divergent: true,
            });
        }
        let exponent = Real::Exact(p);
        let value = Real::Bounded(a).rational_power(&exponent, bits)?;
        let enclosure = value
            .arithmetic(Op::Div, &exponent.neg()?, bits)?
            .enclose(bits)?
            .bounds()?;
        return Ok(Tail {
            method,
            enclosure: Some(enclosure),
            divergent: false,
        });
    }
    if let Kind::Call(name, args) = &expr.kind {
        if let [arg] = args.as_slice() {
            if name == "exp" {
                if let Some(rate) = linear(arg) {
                    let method = "Exponential tail on [a, +infinity): exp(k*a)/(-k) for k < 0";
                    if rate >= Rational::zero() {
                        return Ok(Tail {
                            method,
                            enclosure: None,
                            divergent: true,
                        });
                    }
                    let k = Interval::ratio(&rate.to_string(), bits)?;
                    let enclosure = k.mul(&a)?.unary(Unary::Exp)?.div(&k.neg()?)?.bounds()?;
                    return Ok(Tail {
                        method,
                        enclosure: Some(enclosure),
                        divergent: false,
                    });
                }
            }
        }
    }
    Err(Issue::Split("No supported tail proof for this expression"))
}
