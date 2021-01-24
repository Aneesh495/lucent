use num_traits::{One, ToPrimitive, Zero};
use serde::Serialize;
use std::cmp::Ordering;

use crate::{
    exact::{self, Rational},
    interval::{Interval, Issue, Result},
    lang::Op,
    mp::Unary,
};

#[derive(Debug, Clone)]
pub enum Real {
    Exact(Rational),
    Bounded(Interval),
}

impl Real {
    pub fn integer(x: i64) -> Self {
        Self::Exact(Rational::from_integer(x.into()))
    }

    pub fn enclose(&self, bits: u32) -> Result<Interval> {
        match self {
            Self::Exact(x) => Interval::ratio(&x.to_string(), bits),
            Self::Bounded(i) => Ok(i.clone()),
        }
    }

    pub fn as_i32(&self) -> Result<i32> {
        match self {
            Self::Exact(q) if q.is_integer() => q.to_integer().to_i32().ok_or(Issue::Budget),
            _ => Err(Issue::Unsupported("An exact integer is required")),
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            Self::Exact(q) => q.is_zero(),
            Self::Bounded(i) => i.sign() == Some(Ordering::Equal),
        }
    }

    pub fn sign(&self) -> Option<Ordering> {
        match self {
            Self::Exact(q) => Some(q.cmp(&Rational::zero())),
            Self::Bounded(i) => i.sign(),
        }
    }

    pub fn arithmetic(&self, op: Op, other: &Self, bits: u32) -> Result<Self> {
        if op == Op::Pow {
            return self.power(other.as_i32()?, bits);
        }
        if let (Self::Exact(a), Self::Exact(b)) = (self, other) {
            let output = match op {
                Op::Add => a + b,
                Op::Sub => a - b,
                Op::Mul => a * b,
                Op::Div if b.is_zero() => return Err(Issue::Excluded("Denominator is zero")),
                Op::Div => a / b,
                _ => return Err(Issue::Unsupported("Invalid arithmetic operator")),
            };
            return Ok(Self::Exact(exact::checked(output)?));
        }
        let a = self.enclose(bits)?;
        let b = other.enclose(bits)?;
        Ok(Self::Bounded(match op {
            Op::Add => a.add(&b)?,
            Op::Sub => a.sub(&b)?,
            Op::Mul => a.mul(&b)?,
            Op::Div => a.div(&b)?,
            _ => return Err(Issue::Unsupported("Invalid arithmetic operator")),
        }))
    }

    pub fn power(&self, exponent: i32, _bits: u32) -> Result<Self> {
        match self {
            Self::Exact(q) => Ok(Self::Exact(exact::power(q, exponent)?)),
            Self::Bounded(i) => Ok(Self::Bounded(i.pow(exponent)?)),
        }
    }

    pub fn neg(&self) -> Result<Self> {
        match self {
            Self::Exact(q) => Ok(Self::Exact(-q)),
            Self::Bounded(i) => Ok(Self::Bounded(i.neg()?)),
        }
    }

    pub fn unary(&self, op: Unary, bits: u32) -> Result<Self> {
        if let Self::Exact(q) = self {
            match op {
                Unary::Abs => {
                    return if q < &Rational::zero() {
                        self.neg()
                    } else {
                        Ok(self.clone())
                    };
                }
                Unary::Sqrt if q >= &Rational::zero() => {
                    let n = q.numer().sqrt();
                    let d = q.denom().sqrt();
                    if &(&n * &n) == q.numer() && &(&d * &d) == q.denom() {
                        return Ok(Self::Exact(Rational::new(n, d)));
                    }
                }
                Unary::Sin | Unary::Tan | Unary::Atan | Unary::Erf if q.is_zero() => {
                    return Ok(Self::integer(0));
                }
                Unary::Cos | Unary::Exp | Unary::Erfc if q.is_zero() => {
                    return Ok(Self::integer(1));
                }
                Unary::Log if q.is_one() => return Ok(Self::integer(0)),
                _ => {}
            }
        }
        Ok(Self::Bounded(self.enclose(bits)?.unary(op)?))
    }

    pub fn compare(&self, op: Op, other: &Self, bits: u32) -> Result<Truth> {
        let truth = |ordering: Ordering| match op {
            Op::Eq => ordering.is_eq(),
            Op::Ne => !ordering.is_eq(),
            Op::Lt => ordering.is_lt(),
            Op::Le => !ordering.is_gt(),
            Op::Gt => ordering.is_gt(),
            Op::Ge => !ordering.is_lt(),
            _ => unreachable!("comparison opcode"),
        };
        if let (Self::Exact(a), Self::Exact(b)) = (self, other) {
            return Ok(truth(a.cmp(b)).into());
        }
        let a = self.enclose(bits)?;
        let b = other.enclose(bits)?;
        if a.strictly_before(&b) {
            return Ok(truth(Ordering::Less).into());
        }
        if b.strictly_before(&a) {
            return Ok(truth(Ordering::Greater).into());
        }
        if a.is_point() && b.is_point() {
            return Ok(truth(Ordering::Equal).into());
        }
        if a.before_or_touches(&b) && truth(Ordering::Less) == truth(Ordering::Equal) {
            return Ok(truth(Ordering::Less).into());
        }
        if b.before_or_touches(&a) && truth(Ordering::Greater) == truth(Ordering::Equal) {
            return Ok(truth(Ordering::Greater).into());
        }
        Ok(Truth::Unknown)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Truth {
    True,
    False,
    Unknown,
}

impl From<bool> for Truth {
    fn from(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize)]
pub struct Dimension(pub [i8; 4]);

impl Dimension {
    pub const NONE: Self = Self([0, 0, 0, 0]);
    pub const LENGTH: Self = Self([1, 0, 0, 0]);
    pub const TIME: Self = Self([0, 1, 0, 0]);
    pub const MASS: Self = Self([0, 0, 1, 0]);
    pub const ANGLE: Self = Self([0, 0, 0, 1]);

    fn transform(self, mut f: impl FnMut(usize, i8) -> Option<i8>) -> Result<Self> {
        let mut units = self.0;
        for (index, unit) in units.iter_mut().enumerate() {
            *unit = f(index, *unit).ok_or(Issue::Budget)?;
        }
        Ok(Self(units))
    }

    pub fn combine(self, other: Self, sign: i8) -> Result<Self> {
        self.transform(|i, x| x.checked_add(other.0[i].checked_mul(sign)?))
    }

    pub fn scale(self, exponent: i32) -> Result<Self> {
        self.transform(|_, x| i8::try_from(i32::from(x).checked_mul(exponent)?).ok())
    }

    pub fn sqrt(self) -> Result<Self> {
        if self.0.iter().any(|x| x % 2 != 0) {
            return Err(Issue::Unsupported("Square root needs even unit powers"));
        }
        self.transform(|_, x| Some(x / 2))
    }
}

#[derive(Debug, Clone)]
pub struct Quantity {
    pub real: Real,
    pub dimension: Dimension,
}

impl Quantity {
    pub fn real(real: Real) -> Self {
        Self {
            real,
            dimension: Dimension::NONE,
        }
    }
    pub fn new(real: Real, dimension: Dimension) -> Self {
        Self { real, dimension }
    }

    pub fn arithmetic(&self, op: Op, other: &Self, bits: u32) -> Result<Self> {
        let dimension = match op {
            Op::Add | Op::Sub => {
                self.same_units(other)?;
                self.dimension
            }
            Op::Mul => self.dimension.combine(other.dimension, 1)?,
            Op::Div => self.dimension.combine(other.dimension, -1)?,
            Op::Pow => {
                other.dimensionless()?;
                self.dimension.scale(other.real.as_i32()?)?
            }
            _ => return Err(Issue::Unsupported("Invalid arithmetic operator")),
        };
        Ok(Self {
            real: self.real.arithmetic(op, &other.real, bits)?,
            dimension,
        })
    }

    pub fn same_units(&self, other: &Self) -> Result<()> {
        if self.dimension != other.dimension {
            return Err(Issue::Unsupported("Operands have different dimensions"));
        }
        Ok(())
    }

    pub fn dimensionless(&self) -> Result<&Real> {
        if self.dimension != Dimension::NONE {
            return Err(Issue::Unsupported("A dimensionless argument is required"));
        }
        Ok(&self.real)
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Quantity(Quantity),
    Complex(Real, Real),
    Bool(Truth),
    List(Vec<Self>),
}

impl Value {
    pub fn real(number: Real) -> Self {
        Self::Quantity(Quantity::real(number))
    }

    pub fn quantity(self) -> Result<Quantity> {
        match self {
            Self::Quantity(value) => Ok(value),
            _ => Err(Issue::Unsupported("A real quantity is required")),
        }
    }

    pub fn truth(self) -> Result<Truth> {
        match self {
            Self::Bool(value) => Ok(value),
            _ => Err(Issue::Unsupported("A Boolean is required")),
        }
    }

    pub fn list(self) -> Result<Vec<Self>> {
        match self {
            Self::List(values) => Ok(values),
            _ => Err(Issue::Unsupported("A list is required")),
        }
    }

    pub fn complex(self) -> Result<(Real, Real)> {
        match self {
            Self::Complex(re, im) => Ok((re, im)),
            Self::Quantity(q) => Ok((q.dimensionless()?.clone(), Real::integer(0))),
            _ => Err(Issue::Unsupported("A dimensionless scalar is required")),
        }
    }

    pub fn negated(self) -> Result<Self> {
        match self {
            Self::Quantity(q) => Ok(Self::Quantity(Quantity::new(q.real.neg()?, q.dimension))),
            Self::Complex(re, im) => Ok(Self::Complex(re.neg()?, im.neg()?)),
            Self::List(items) => Ok(Self::List(
                items
                    .into_iter()
                    .map(Self::negated)
                    .collect::<Result<_>>()?,
            )),
            _ => Err(Issue::Unsupported("Negation needs numbers")),
        }
    }

    pub fn arithmetic(self, op: Op, rhs: Self, bits: u32) -> Result<Self> {
        match (self, rhs) {
            (Self::Quantity(a), Self::Quantity(b)) if op.compares() => {
                a.same_units(&b)?;
                Ok(Self::Bool(a.real.compare(op, &b.real, bits)?))
            }
            (Self::Quantity(a), Self::Quantity(b)) => {
                Ok(Self::Quantity(a.arithmetic(op, &b, bits)?))
            }
            (Self::List(a), Self::List(b))
                if matches!(op, Op::Add | Op::Sub) && a.len() == b.len() =>
            {
                Ok(Self::List(
                    a.into_iter()
                        .zip(b)
                        .map(|(x, y)| x.arithmetic(op, y, bits))
                        .collect::<Result<_>>()?,
                ))
            }
            (Self::List(items), scalar @ (Self::Quantity(_) | Self::Complex(..)))
                if matches!(op, Op::Mul | Op::Div) =>
            {
                Ok(Self::List(
                    items
                        .into_iter()
                        .map(|x| x.arithmetic(op, scalar.clone(), bits))
                        .collect::<Result<_>>()?,
                ))
            }
            (scalar @ (Self::Quantity(_) | Self::Complex(..)), Self::List(items))
                if op == Op::Mul =>
            {
                Ok(Self::List(
                    items
                        .into_iter()
                        .map(|x| scalar.clone().arithmetic(op, x, bits))
                        .collect::<Result<_>>()?,
                ))
            }
            (a @ Self::Complex(..), b) | (a, b @ Self::Complex(..)) => {
                complex_arithmetic(a, op, b, bits)
            }
            _ => Err(Issue::Unsupported(
                "Operator does not support these operand shapes",
            )),
        }
    }
}

fn complex_arithmetic(a: Value, op: Op, b: Value, bits: u32) -> Result<Value> {
    let (ar, ai) = a.complex()?;
    let (br, bi) = b.complex()?;
    let combine = |a: &Real, op, b: &Real| a.arithmetic(op, b, bits);
    let (re, im) = match op {
        Op::Add | Op::Sub => (combine(&ar, op, &br)?, combine(&ai, op, &bi)?),
        Op::Mul => (
            combine(
                &combine(&ar, Op::Mul, &br)?,
                Op::Sub,
                &combine(&ai, Op::Mul, &bi)?,
            )?,
            combine(
                &combine(&ar, Op::Mul, &bi)?,
                Op::Add,
                &combine(&ai, Op::Mul, &br)?,
            )?,
        ),
        Op::Div => {
            let denominator = combine(&br.power(2, bits)?, Op::Add, &bi.power(2, bits)?)?;
            let real = combine(
                &combine(&ar, Op::Mul, &br)?,
                Op::Add,
                &combine(&ai, Op::Mul, &bi)?,
            )?;
            let imag = combine(
                &combine(&ai, Op::Mul, &br)?,
                Op::Sub,
                &combine(&ar, Op::Mul, &bi)?,
            )?;
            (
                combine(&real, Op::Div, &denominator)?,
                combine(&imag, Op::Div, &denominator)?,
            )
        }
        Op::Pow => {
            if !bi.is_zero() {
                return Err(Issue::Unsupported(
                    "Complex exponent must be an exact real integer",
                ));
            }
            let power = br.as_i32()?;
            if power.unsigned_abs() > 4096 {
                return Err(Issue::Budget);
            }
            let mut out = Value::real(Real::integer(1));
            let mut base = Value::Complex(ar, ai);
            let mut n = power.unsigned_abs();
            while n > 0 {
                if n % 2 != 0 {
                    out = out.arithmetic(Op::Mul, base.clone(), bits)?;
                }
                n /= 2;
                if n > 0 {
                    base = base.clone().arithmetic(Op::Mul, base, bits)?;
                }
            }
            return if power < 0 {
                Value::real(Real::integer(1)).arithmetic(Op::Div, out, bits)
            } else {
                Ok(out)
            };
        }
        _ => {
            return Err(Issue::Unsupported(
                "Comparison of complex values is unsupported",
            ));
        }
    };
    Ok(Value::Complex(re, im))
}
