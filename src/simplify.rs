use num_traits::{Signed, ToPrimitive, Zero};
use serde::Serialize;

use crate::{
    exact::{self, Rational},
    lang::{Expr, Kind, Op, Span},
};

#[derive(Default, Debug, Serialize)]
pub struct Changes {
    pub exact_folds: usize,
    pub branch_folds: usize,
    pub guarded_identities: usize,
}

pub fn simplify(expr: &Expr) -> (Expr, Changes) {
    let mut changes = Changes::default();
    (visit(expr, &mut changes), changes)
}

fn visit(expr: &Expr, changes: &mut Changes) -> Expr {
    let span = expr.span;
    match &expr.kind {
        Kind::Negative(value) => {
            let value = visit(value, changes);
            if let Some(ratio) = constant(&value) {
                changes.exact_folds += 1;
                return exact_tree(-ratio, span);
            }
            Expr::negated(value, span)
        }
        Kind::Binary(op, a, b) => {
            let a = visit(a, changes);
            let b = visit(b, changes);
            if let (Some(a), Some(b)) = (constant(&a), constant(&b)) {
                if let Some(result) = arithmetic(*op, a, b) {
                    changes.exact_folds += 1;
                    return exact_tree(result, span);
                }
            }
            let same_axis = matches!((&a.kind, &b.kind), (Kind::Name(a), Kind::Name(b))
                if a == b && matches!(a.as_str(), "x" | "y" | "z" | "u" | "v" | "t"));
            if same_axis && matches!(op, Op::Div | Op::Sub) {
                let name = if *op == Op::Div {
                    "self_quotient"
                } else {
                    "self_difference"
                };
                changes.guarded_identities += 1;
                return Expr::new(Kind::Call(name.into(), vec![a]), span);
            }
            Expr::binary(*op, a, b, span)
        }
        Kind::Call(name, args) => {
            if let [test, yes, no] = args.as_slice() {
                if name == "if" {
                    if let Kind::Name(value) = &test.kind {
                        let selected = match value.as_str() {
                            "true" => Some(yes),
                            "false" => Some(no),
                            _ => None,
                        };
                        if let Some(selected) = selected {
                            changes.branch_folds += 1;
                            let mut result = visit(selected, changes);
                            result.span = span;
                            return result;
                        }
                    }
                }
            }
            Expr::new(
                Kind::Call(
                    name.clone(),
                    args.iter().map(|arg| visit(arg, changes)).collect(),
                ),
                span,
            )
        }
        Kind::List(items) => Expr::new(
            Kind::List(items.iter().map(|item| visit(item, changes)).collect()),
            span,
        ),
        _ => expr.clone(),
    }
}

fn constant(expr: &Expr) -> Option<Rational> {
    match &expr.kind {
        Kind::Literal(s) => exact::decimal(s).ok(),
        Kind::Negative(a) => Some(-constant(a)?),
        Kind::Binary(Op::Div, a, b) => arithmetic(Op::Div, constant(a)?, constant(b)?),
        _ => None,
    }
}

fn arithmetic(op: Op, a: Rational, b: Rational) -> Option<Rational> {
    let result = match op {
        Op::Add => a + b,
        Op::Sub => a - b,
        Op::Mul => a * b,
        Op::Div if !b.is_zero() => a / b,
        Op::Pow if b.is_integer() => exact::power(&a, b.to_integer().to_i32()?).ok()?,
        _ => return None,
    };
    // Emitted literals must fit the same decimal-token limits as user input.
    let checked = exact::checked(result).ok()?;
    if checked.numer().bits().max(checked.denom().bits()) > 1600 {
        return None;
    }
    Some(checked)
}

fn exact_tree(value: Rational, span: Span) -> Expr {
    let magnitude = Expr::literal(value.numer().abs().to_string(), span);
    let numerator = if value.is_negative() {
        Expr::negated(magnitude, span)
    } else {
        magnitude
    };
    if value.is_integer() {
        return numerator;
    }
    Expr::binary(
        Op::Div,
        numerator,
        Expr::literal(value.denom().to_string(), span),
        span,
    )
}
