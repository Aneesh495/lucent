use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};

use crate::{
    exact,
    interval::{Interval, Issue, Result as NumericResult},
    lang::{Expr, Kind, Op, Span},
    mp::Unary,
    program::Program,
    value::{Dimension, Quantity, Real, Truth, Value},
};

#[derive(Default, Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Angle {
    #[default]
    Radians,
    Degrees,
}

#[derive(Debug, Clone, Copy)]
pub struct EvalError {
    pub span: Span,
    pub issue: Issue,
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at bytes {}..{}",
            self.issue, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for EvalError {}

pub type Result<T> = std::result::Result<T, EvalError>;

pub type Scope = BTreeMap<String, Value>;

pub struct Eval<'a> {
    pub program: &'a Program,
    pub scope: &'a Scope,
    pub bits: u32,
    pub angle: Angle,
    left: usize,
}

impl<'a> Eval<'a> {
    pub fn new(program: &'a Program, scope: &'a Scope, bits: u32, angle: Angle) -> Self {
        Self {
            program,
            scope,
            bits,
            angle,
            left: 8_192,
        }
    }

    pub fn run(&mut self, expr: &Expr) -> Result<Value> {
        self.inner(expr, &Scope::new(), 0)
    }

    pub fn real(&mut self, expr: &Expr) -> Result<Real> {
        self.run(expr)?
            .quantity()
            .and_then(|q| q.dimensionless().cloned())
            .map_err(|issue| EvalError {
                span: expr.span,
                issue,
            })
    }

    fn inner(&mut self, expr: &Expr, locals: &Scope, depth: usize) -> Result<Value> {
        let error = |issue| EvalError {
            span: expr.span,
            issue,
        };
        if self.left == 0 || depth > 80 {
            return Err(error(Issue::Budget));
        }
        self.left -= 1;
        let out = match &expr.kind {
            Kind::Literal(s) => exact::decimal(s).map(|q| Value::real(Real::Exact(q))),
            Kind::Name(name) => {
                if let Some(value) = locals.get(name).or_else(|| self.scope.get(name)) {
                    return Ok(value.clone());
                }
                if let Some(def) = self.program.get(name) {
                    if def.params.is_some() {
                        return Err(error(Issue::Unsupported("Function needs arguments")));
                    }
                    return self.inner(&def.body, &Scope::new(), depth + 1);
                }
                self.constant(name)
            }
            Kind::Negative(expr) => self.inner(expr, locals, depth + 1)?.negated(),
            Kind::Binary(op, a, b) => {
                let a = self.inner(a, locals, depth + 1)?;
                let b = self.inner(b, locals, depth + 1)?;
                a.arithmetic(*op, b, self.bits)
            }
            Kind::List(items) => {
                if items.len() > 32 {
                    return Err(error(Issue::Budget));
                }
                return Ok(Value::List(self.arguments(items, locals, depth)?));
            }
            Kind::Call(name, args) => {
                if name == "if" {
                    let [condition, yes, no] = args.as_slice() else {
                        return Err(error(Issue::Unsupported(
                            "if takes a condition and two branches",
                        )));
                    };
                    let cond = self
                        .inner(condition, locals, depth + 1)?
                        .truth()
                        .map_err(error)?;
                    return match cond {
                        Truth::True => self.inner(yes, locals, depth + 1),
                        Truth::False => self.inner(no, locals, depth + 1),
                        Truth::Unknown => Err(error(Issue::Split(
                            "Piecewise predicate changes or is undecidable",
                        ))),
                    };
                }
                let values = self.arguments(args, locals, depth)?;
                if let Some(def) = self.program.get(name) {
                    let params = def
                        .params
                        .as_ref()
                        .filter(|p| p.len() == values.len())
                        .ok_or_else(|| {
                            error(Issue::Unsupported("Invalid function argument count"))
                        })?;
                    let scope = params.iter().cloned().zip(values).collect();
                    return self.inner(&def.body, &scope, depth + 1);
                }
                self.call(name, values)
            }
        }
        .map_err(error)?;
        self.charge_shape(&out).map_err(error)?;
        Ok(out)
    }

    fn arguments(&mut self, args: &[Expr], locals: &Scope, depth: usize) -> Result<Vec<Value>> {
        args.iter()
            .map(|arg| self.inner(arg, locals, depth + 1))
            .collect()
    }

    fn charge_shape(&mut self, value: &Value) -> NumericResult<()> {
        if let Value::List(items) = value {
            for item in items {
                self.left = self.left.checked_sub(1).ok_or(Issue::Budget)?;
                self.charge_shape(item)?;
            }
        }
        Ok(())
    }

    fn constant(&self, name: &str) -> NumericResult<Value> {
        let bits = self.bits;
        let units = |value: Real, dimension| Ok(Value::Quantity(Quantity::new(value, dimension)));
        let ratio = |text| exact::ratio(text).map(Real::Exact);
        match name {
            "pi" => Ok(Value::real(Real::Bounded(Interval::pi(bits)?))),
            "e" => Ok(Value::real(Real::integer(1).unary(Unary::Exp, bits)?)),
            "i" => Ok(Value::Complex(Real::integer(0), Real::integer(1))),
            "true" => Ok(Value::Bool(Truth::True)),
            "false" => Ok(Value::Bool(Truth::False)),
            "m" => units(Real::integer(1), Dimension::LENGTH),
            "cm" => units(ratio("1/100")?, Dimension::LENGTH),
            "km" => units(Real::integer(1000), Dimension::LENGTH),
            "s" => units(Real::integer(1), Dimension::TIME),
            "ms" => units(ratio("1/1000")?, Dimension::TIME),
            "kg" => units(Real::integer(1), Dimension::MASS),
            "rad" => units(Real::integer(1), Dimension::ANGLE),
            "deg" => units(self.degrees(&Real::integer(1))?, Dimension::ANGLE),
            _ => Err(Issue::Unsupported("Unknown name")),
        }
    }

    fn call(&self, name: &str, mut args: Vec<Value>) -> NumericResult<Value> {
        let bits = self.bits;
        match name {
            "self_quotient" | "self_difference" if args.len() == 1 => {
                let value = args.remove(0);
                let quotient = name == "self_quotient";
                let nonzero = |value: &Real| value.sign().is_some_and(|s| !s.is_eq());
                match value {
                    Value::Quantity(q) => {
                        if !quotient {
                            return Ok(Value::Quantity(Quantity::new(
                                Real::integer(0),
                                q.dimension,
                            )));
                        }
                        if q.real.is_zero() {
                            return Err(Issue::Excluded("Denominator is zero"));
                        }
                        if !nonzero(&q.real) {
                            return Err(Issue::Split("Denominator enclosure includes zero"));
                        }
                        Ok(Value::real(Real::integer(1)))
                    }
                    Value::Complex(re, im) => {
                        if quotient && !nonzero(&re) && !nonzero(&im) {
                            return if re.is_zero() && im.is_zero() {
                                Err(Issue::Excluded("Complex denominator is zero"))
                            } else {
                                Err(Issue::Split(
                                    "Complex denominator rectangle may contain zero",
                                ))
                            };
                        }
                        Ok(Value::Complex(
                            Real::integer(i64::from(quotient)),
                            Real::integer(0),
                        ))
                    }
                    _ => Err(Issue::Unsupported(
                        "A scalar is required for this guarded identity",
                    )),
                }
            }
            "sum" | "product" if args.len() == 1 => {
                let (op, initial) = if name == "sum" {
                    (Op::Add, 0)
                } else {
                    (Op::Mul, 1)
                };
                let mut items = args.remove(0).list()?.into_iter();
                let first = items
                    .next()
                    .unwrap_or_else(|| Value::real(Real::integer(initial)));
                items.try_fold(first, |a, b| a.arithmetic(op, b, bits))
            }
            "complex" | "interval" if args.len() == 2 => {
                let b = args.pop().unwrap().quantity()?;
                let a = args.pop().unwrap().quantity()?;
                if name == "complex" {
                    a.dimensionless()?;
                    b.dimensionless()?;
                    Ok(Value::Complex(a.real, b.real))
                } else {
                    a.same_units(&b)?;
                    if a.real.compare(Op::Le, &b.real, bits)? != Truth::True {
                        return Err(Issue::Split("Interval endpoints must be proved ordered"));
                    }
                    let enclosure = a.real.enclose(bits)?.hull(&b.real.enclose(bits)?);
                    Ok(Value::Quantity(Quantity::new(
                        Real::Bounded(enclosure),
                        a.dimension,
                    )))
                }
            }
            "re" | "im" | "conj" if args.len() == 1 => {
                let (re, im) = args.remove(0).complex()?;
                match name {
                    "re" => Ok(Value::real(re)),
                    "im" => Ok(Value::real(im)),
                    _ => Ok(Value::Complex(re, im.neg()?)),
                }
            }
            "dot" | "transpose" | "det" | "solve" => crate::linear::call(name, args, bits),
            _ if args.len() == 1 => self.unary(name, args.remove(0)),
            _ => Err(Issue::Unsupported(
                "Unknown function or invalid argument count",
            )),
        }
    }

    fn unary(&self, name: &str, value: Value) -> NumericResult<Value> {
        let bits = self.bits;
        if let Value::Complex(re, im) = value {
            return self.complex_unary(name, re, im);
        }
        let mut quantity = value.quantity()?;
        let op = match name {
            "sqrt" => {
                quantity.dimension = quantity.dimension.sqrt()?;
                Unary::Sqrt
            }
            "abs" => Unary::Abs,
            "sin" | "cos" | "tan" => {
                match quantity.dimension {
                    Dimension::NONE if matches!(self.angle, Angle::Degrees) => {
                        quantity.real = self.degrees(&quantity.real)?
                    }
                    Dimension::ANGLE => quantity.dimension = Dimension::NONE,
                    Dimension::NONE => {}
                    _ => {
                        return Err(Issue::Unsupported(
                            "Trigonometry needs an angle or dimensionless number",
                        ));
                    }
                }
                match name {
                    "sin" => Unary::Sin,
                    "cos" => Unary::Cos,
                    _ => Unary::Tan,
                }
            }
            "normal_cdf" | "normal_sf" => {
                let q = quantity.dimensionless()?;
                let scale = Real::integer(2).unary(Unary::Sqrt, bits)?;
                let x = if name == "normal_cdf" {
                    q.neg()?
                } else {
                    q.clone()
                };
                let z = x
                    .arithmetic(Op::Div, &scale, bits)?
                    .unary(Unary::Erfc, bits)?;
                return Ok(Value::real(z.arithmetic(
                    Op::Div,
                    &Real::integer(2),
                    bits,
                )?));
            }
            _ => {
                quantity.dimensionless()?;
                match name {
                    "exp" => Unary::Exp,
                    "log" | "ln" => Unary::Log,
                    "atan" => Unary::Atan,
                    "erf" => Unary::Erf,
                    "erfc" => Unary::Erfc,
                    _ => return Err(Issue::Unsupported("Unknown unary function")),
                }
            }
        };
        quantity.real = quantity.real.unary(op, bits)?;
        if name == "atan" && matches!(self.angle, Angle::Degrees) {
            quantity.real = quantity
                .real
                .arithmetic(Op::Mul, &Real::integer(180), bits)?
                .arithmetic(Op::Div, &Real::Bounded(Interval::pi(bits)?), bits)?;
        }
        Ok(Value::Quantity(quantity))
    }

    fn degrees(&self, input: &Real) -> NumericResult<Real> {
        input
            .arithmetic(Op::Mul, &Real::Bounded(Interval::pi(self.bits)?), self.bits)?
            .arithmetic(Op::Div, &Real::integer(180), self.bits)
    }

    fn complex_unary(&self, name: &str, re: Real, im: Real) -> NumericResult<Value> {
        let bits = self.bits;
        let radius_squared = || {
            re.power(2, bits)?
                .arithmetic(Op::Add, &im.power(2, bits)?, bits)
        };
        match name {
            "abs" => Ok(Value::real(radius_squared()?.unary(Unary::Sqrt, bits)?)),
            "exp" => {
                let scale = re.unary(Unary::Exp, bits)?;
                let real = scale.arithmetic(Op::Mul, &im.unary(Unary::Cos, bits)?, bits)?;
                let imag = scale.arithmetic(Op::Mul, &im.unary(Unary::Sin, bits)?, bits)?;
                Ok(Value::Complex(real, imag))
            }
            "ln" | "log" => {
                let magnitude = radius_squared()?.unary(Unary::Log, bits)?.arithmetic(
                    Op::Div,
                    &Real::integer(2),
                    bits,
                )?;
                let angle = im.enclose(bits)?.principal_argument(&re.enclose(bits)?)?;
                Ok(Value::Complex(magnitude, Real::Bounded(angle)))
            }
            "sqrt" => {
                if im.is_zero() {
                    if re.sign() == Some(std::cmp::Ordering::Less) {
                        return Ok(Value::Complex(
                            Real::integer(0),
                            re.neg()?.unary(Unary::Sqrt, bits)?,
                        ));
                    }
                    if re.sign().is_some() {
                        return Ok(Value::Complex(
                            re.unary(Unary::Sqrt, bits)?,
                            Real::integer(0),
                        ));
                    }
                }
                let r = radius_squared()?.unary(Unary::Sqrt, bits)?;
                let two = Real::integer(2);
                let component = |op| -> NumericResult<Real> {
                    let part = r
                        .arithmetic(op, &re, bits)?
                        .arithmetic(Op::Div, &two, bits)?;
                    Ok(Real::Bounded(
                        part.enclose(bits)?.nonnegative_part()?.unary(Unary::Sqrt)?,
                    ))
                };
                let real = component(Op::Add)?;
                let imaginary = match im.sign() {
                    Some(std::cmp::Ordering::Less) => component(Op::Sub)?.neg()?,
                    Some(std::cmp::Ordering::Greater) => component(Op::Sub)?,
                    _ if re.sign() == Some(std::cmp::Ordering::Greater) => {
                        im.arithmetic(Op::Div, &two.arithmetic(Op::Mul, &real, bits)?, bits)?
                    }
                    _ => return Err(Issue::Split("Principal square root crosses its branch cut")),
                };
                Ok(Value::Complex(real, imaginary))
            }
            _ => Err(Issue::Unsupported(
                "Complex function is outside the supported branch contract",
            )),
        }
    }
}
