use num_traits::{One, ToPrimitive, Zero};
use serde::Serialize;

use crate::{
    exact::{self, Rational},
    graph::Context,
    interval::{Interval, Issue, Result},
    lang::{Expr, Kind, Op},
    mp::Unary,
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

fn power(expr: &Expr) -> Option<i32> {
    if axis(expr) {
        return Some(1);
    }
    if let Kind::Binary(Op::Pow, a, b) = &expr.kind {
        if axis(a) {
            let n = constant(b)?;
            if n.is_integer() {
                return n.to_integer().to_i32();
            }
        }
    }
    None
}

fn family_power(expr: &Expr) -> Option<i32> {
    if let Kind::Binary(Op::Div, a, b) = &expr.kind {
        if constant(a)?.is_one() {
            return power(b)?.checked_neg();
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

pub fn integrate(ctx: &Context, expr: &Expr, lower: f64) -> Result<Tail> {
    let bits = ctx.control.bits;
    let a = Interval::point(lower, bits)?;
    if let Some(exponent) = family_power(expr) {
        if lower <= 0.0 {
            return Err(Issue::Unsupported(
                "Power tail requires a positive lower endpoint",
            ));
        }
        let method = "Power test on [a, +infinity): a^(1-p)/(p-1) for x^(-p), p > 1";
        if exponent >= -1 {
            return Ok(Tail {
                method,
                enclosure: None,
                divergent: true,
            });
        }
        let exponent = exponent.checked_add(1).ok_or(Issue::Budget)?;
        let divisor = Interval::point(-f64::from(exponent), bits)?;
        let enclosure = a.pow(exponent)?.div(&divisor)?.bounds()?;
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
