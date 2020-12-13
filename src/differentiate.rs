use crate::{
    eval::Angle,
    interval::{Issue, Result},
    lang::{Expr, Kind, Op, Span},
    program::Program,
};

pub fn derivative(expr: &Expr, axis: &str, angle: Angle) -> Result<Expr> {
    let mut remaining = 3_000;
    derive(expr, axis, angle, &mut remaining)
}

pub fn inline(expr: &Expr, program: &Program) -> Result<Expr> {
    expand(expr, program, &[], 0, &mut 3_000)
}

fn expand(
    expr: &Expr,
    program: &Program,
    locals: &[(&str, Expr)],
    depth: usize,
    remaining: &mut usize,
) -> Result<Expr> {
    *remaining = remaining.checked_sub(1).ok_or(Issue::Budget)?;
    if depth > 80 {
        return Err(Issue::Budget);
    }
    let span = expr.span;
    let kind = match &expr.kind {
        Kind::Name(name) => {
            if let Some((_, value)) = locals.iter().find(|(key, _)| *key == name) {
                charge(value, remaining)?;
                return Ok(value.clone());
            }
            if let Some(def) = program.get(name) {
                if def.params.is_some() {
                    return Err(Issue::Unsupported("Function requires arguments"));
                }
                return expand(&def.body, program, &[], depth + 1, remaining);
            }
            expr.kind.clone()
        }
        Kind::Call(name, args) => {
            let args = args
                .iter()
                .map(|arg| expand(arg, program, locals, depth + 1, remaining))
                .collect::<Result<Vec<_>>>()?;
            if let Some(def) = program.get(name) {
                let parameters = def
                    .params
                    .as_ref()
                    .filter(|p| p.len() == args.len())
                    .ok_or(Issue::Unsupported("Invalid argument count"))?;
                let scope = parameters
                    .iter()
                    .map(String::as_str)
                    .zip(args)
                    .collect::<Vec<_>>();
                return expand(&def.body, program, &scope, depth + 1, remaining);
            }
            Kind::Call(name.clone(), args)
        }
        Kind::Binary(op, a, b) => Kind::Binary(
            *op,
            Box::new(expand(a, program, locals, depth + 1, remaining)?),
            Box::new(expand(b, program, locals, depth + 1, remaining)?),
        ),
        Kind::Negative(a) => {
            Kind::Negative(Box::new(expand(a, program, locals, depth + 1, remaining)?))
        }
        Kind::List(items) => Kind::List(
            items
                .iter()
                .map(|a| expand(a, program, locals, depth + 1, remaining))
                .collect::<Result<_>>()?,
        ),
        Kind::Literal(_) => expr.kind.clone(),
    };
    Ok(Expr::new(kind, span))
}

fn charge(expr: &Expr, remaining: &mut usize) -> Result<()> {
    *remaining = remaining.checked_sub(1).ok_or(Issue::Budget)?;
    match &expr.kind {
        Kind::Binary(_, a, b) => {
            charge(a, remaining)?;
            charge(b, remaining)?;
        }
        Kind::Negative(a) => charge(a, remaining)?,
        Kind::Call(_, args) | Kind::List(args) => {
            for arg in args {
                charge(arg, remaining)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn derive(expr: &Expr, axis: &str, angle: Angle, remaining: &mut usize) -> Result<Expr> {
    *remaining = remaining.checked_sub(1).ok_or(Issue::Budget)?;
    let span = expr.span;
    let number = |x: i32| literal(x, span);
    let binary = |op, a, b| simplified(op, a, b, span);
    let apply = |name: &str, input| Expr::new(Kind::Call(name.to_owned(), vec![input]), span);
    let named = |name: &str| Expr::new(Kind::Name(name.to_owned()), span);
    let trigonometric_scale = |factor| match angle {
        Angle::Radians => factor,
        Angle::Degrees => binary(Op::Mul, factor, binary(Op::Div, named("pi"), number(180))),
    };
    let copy = |value: &Expr, remaining: &mut usize| -> Result<Expr> {
        charge(value, remaining)?;
        Ok(value.clone())
    };
    Ok(match &expr.kind {
        Kind::Literal(_) => number(0),
        Kind::Name(name) => number(i32::from(name == axis)),
        Kind::Negative(a) => Expr::negated(derive(a, axis, angle, remaining)?, span),
        Kind::Binary(Op::Pow, a, b) => {
            let n = literal_integer(b).ok_or(Issue::Unsupported(
                "Derivative needs a literal integer exponent",
            ))?;
            if n == 0 {
                return Ok(number(0));
            }
            if n.unsigned_abs() > 4095 {
                return Err(Issue::Budget);
            }
            let power = binary(Op::Pow, copy(a, remaining)?, number(n - 1));
            let factor = binary(Op::Mul, number(n), power);
            binary(Op::Mul, factor, derive(a, axis, angle, remaining)?)
        }
        Kind::Binary(op, a, b) => {
            let da = derive(a, axis, angle, remaining)?;
            let db = derive(b, axis, angle, remaining)?;
            match op {
                Op::Add | Op::Sub => binary(*op, da, db),
                Op::Mul | Op::Div => {
                    let first = binary(Op::Mul, da, copy(b, remaining)?);
                    let second = binary(Op::Mul, copy(a, remaining)?, db);
                    if *op == Op::Mul {
                        binary(Op::Add, first, second)
                    } else {
                        binary(
                            Op::Div,
                            binary(Op::Sub, first, second),
                            binary(Op::Pow, copy(b, remaining)?, number(2)),
                        )
                    }
                }
                _ => return Err(Issue::Unsupported("Comparison has no scalar derivative")),
            }
        }
        Kind::Call(name, args) if name == "if" => {
            let [condition, a, b] = args.as_slice() else {
                return Err(Issue::Unsupported("Invalid piecewise expression"));
            };
            // A derivative within a branch only makes sense away from the switch.
            let Kind::Binary(op, left, right) = &condition.kind else {
                return Err(Issue::Unsupported(
                    "Piecewise derivative needs a comparison predicate",
                ));
            };
            if !op.compares() {
                return Err(Issue::Unsupported(
                    "Piecewise derivative needs a comparison predicate",
                ));
            }
            let strict = binary(Op::Ne, copy(left, remaining)?, copy(right, remaining)?);
            let inside = Expr::new(
                Kind::Call(
                    "if".into(),
                    vec![
                        copy(condition, remaining)?,
                        derive(a, axis, angle, remaining)?,
                        derive(b, axis, angle, remaining)?,
                    ],
                ),
                span,
            );
            Expr::new(
                Kind::Call(
                    "if".into(),
                    vec![strict, inside, binary(Op::Div, number(1), number(0))],
                ),
                span,
            )
        }
        Kind::Call(name, args) => {
            let [a] = args.as_slice() else {
                return Err(Issue::Unsupported("Unsupported symbolic derivative"));
            };
            let da = derive(a, axis, angle, remaining)?;
            let arg = copy(a, remaining)?;
            let factor = match name.as_str() {
                "sin" => trigonometric_scale(apply("cos", arg)),
                "cos" => trigonometric_scale(Expr::negated(apply("sin", arg), span)),
                "tan" => trigonometric_scale(binary(
                    Op::Div,
                    number(1),
                    binary(Op::Pow, apply("cos", arg), number(2)),
                )),
                "exp" => apply("exp", arg),
                "log" | "ln" => binary(Op::Div, number(1), arg),
                "sqrt" => binary(
                    Op::Div,
                    number(1),
                    binary(Op::Mul, number(2), apply("sqrt", arg)),
                ),
                "atan" => {
                    let base = binary(
                        Op::Div,
                        number(1),
                        binary(Op::Add, number(1), binary(Op::Pow, arg, number(2))),
                    );
                    match angle {
                        Angle::Radians => base,
                        Angle::Degrees => {
                            binary(Op::Mul, base, binary(Op::Div, number(180), named("pi")))
                        }
                    }
                }
                "abs" => binary(Op::Div, arg, apply("abs", copy(a, remaining)?)),
                _ => return Err(Issue::Unsupported("Unsupported symbolic derivative")),
            };
            binary(Op::Mul, factor, da)
        }
        Kind::List(_) => return Err(Issue::Unsupported("Lists have no scalar derivative")),
    })
}

fn literal(value: i32, span: Span) -> Expr {
    if value < 0 {
        Expr::negated(Expr::literal(value.unsigned_abs().to_string(), span), span)
    } else {
        Expr::literal(value.to_string(), span)
    }
}

fn literal_integer(expr: &Expr) -> Option<i32> {
    match &expr.kind {
        Kind::Literal(text) => text.parse().ok(),
        Kind::Negative(a) => literal_integer(a)?.checked_neg(),
        _ => None,
    }
}

fn simplified(op: Op, a: Expr, b: Expr, span: Span) -> Expr {
    let n = literal_integer(&a);
    let m = literal_integer(&b);
    if let (Some(n), Some(m)) = (n, m) {
        let value = match op {
            Op::Add => n.checked_add(m),
            Op::Sub => n.checked_sub(m),
            Op::Mul => n.checked_mul(m),
            _ => None,
        };
        if let Some(value) = value {
            return literal(value, span);
        }
    }
    match (op, n, m) {
        (Op::Add, Some(0), _) | (Op::Mul, Some(1), _) => b,
        (Op::Add | Op::Sub, _, Some(0)) | (Op::Mul | Op::Div | Op::Pow, _, Some(1)) => a,
        _ => Expr::binary(op, a, b, span),
    }
}
