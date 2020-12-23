use std::cmp::Ordering;

use num_traits::{One, Signed, ToPrimitive, Zero};
use serde::{Deserialize, Serialize};

use crate::{
    exact::{Rational, checked, decimal, ratio},
    interval::{Issue, Result},
    lang::{Expr, Kind, Op},
};

const MAX_DEGREE: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Polynomial(Vec<Rational>);

impl Polynomial {
    fn new(mut coefficients: Vec<Rational>) -> Self {
        while coefficients.last().is_some_and(Zero::is_zero) {
            coefficients.pop();
        }
        Self(coefficients)
    }

    fn constant(value: Rational) -> Self {
        Self::new(vec![value])
    }
    fn one() -> Self {
        Self::constant(Rational::one())
    }
    fn zero(&self) -> bool {
        self.0.is_empty()
    }
    fn unit(&self) -> bool {
        self.0.len() == 1 && self.0[0].is_one()
    }

    fn at(&self, x: &Rational) -> Result<Rational> {
        self.0
            .iter()
            .rev()
            .try_fold(Rational::zero(), |a, b| checked(checked(a * x)? + b))
    }

    fn derivative(&self) -> Result<Self> {
        let coefficients = self
            .0
            .iter()
            .enumerate()
            .skip(1)
            .map(|(power, q)| checked(q * Rational::from_integer(power.into())))
            .collect::<Result<_>>()?;
        Ok(Self::new(coefficients))
    }

    fn sum(&self, rhs: &Self, subtract: bool) -> Result<Self> {
        let mut result = self.0.clone();
        result.resize(self.0.len().max(rhs.0.len()), Rational::zero());
        for (a, b) in result.iter_mut().zip(&rhs.0) {
            *a = checked(if subtract { &*a - b } else { &*a + b })?;
        }
        Ok(Self::new(result))
    }

    fn scale(&self, scale: &Rational) -> Result<Self> {
        Ok(Self::new(
            self.0
                .iter()
                .map(|q| checked(q * scale))
                .collect::<Result<_>>()?,
        ))
    }

    fn product(&self, rhs: &Self) -> Result<Self> {
        if self.zero() || rhs.zero() {
            return Ok(Self(Vec::new()));
        }
        let terms = self.0.len() + rhs.0.len() - 1;
        if terms > MAX_DEGREE + 1 {
            return Err(Issue::Budget);
        }
        let mut result = vec![Rational::zero(); terms];
        for (i, a) in self.0.iter().enumerate() {
            for (j, b) in rhs.0.iter().enumerate() {
                result[i + j] = checked(&result[i + j] + checked(a * b)?)?;
            }
        }
        Ok(Self::new(result))
    }

    fn divide(&self, divisor: &Self) -> Result<(Self, Self)> {
        let lead = divisor
            .0
            .last()
            .ok_or(Issue::Excluded("Zero polynomial denominator"))?;
        let mut rem = self.0.clone();
        let mut quot = vec![Rational::zero(); (rem.len() + 1).saturating_sub(divisor.0.len())];
        while rem.len() >= divisor.0.len() {
            let shift = rem.len() - divisor.0.len();
            let factor = checked(rem.last().unwrap() / lead)?;
            quot[shift] = factor.clone();
            for (i, term) in divisor.0.iter().enumerate() {
                rem[shift + i] = checked(&rem[shift + i] - checked(&factor * term)?)?;
            }
            while rem.last().is_some_and(Zero::is_zero) {
                rem.pop();
            }
        }
        Ok((Self::new(quot), Self::new(rem)))
    }

    fn quotient(&self, divisor: &Self) -> Result<Self> {
        let (q, rem) = self.divide(divisor)?;
        if !rem.zero() {
            return Err(Issue::Unsupported("Nonzero exact-division remainder"));
        }
        Ok(q)
    }

    fn monic(&self) -> Result<Self> {
        if let Some(leading) = self.0.last() {
            self.scale(&leading.recip())
        } else {
            Ok(self.clone())
        }
    }
}

fn from_expr(expr: &Expr, axis: &str) -> Result<Polynomial> {
    match &expr.kind {
        Kind::Literal(s) => Ok(Polynomial::constant(decimal(s)?)),
        Kind::Name(name) if name == axis => Ok(Polynomial(vec![Rational::zero(), Rational::one()])),
        Kind::Negative(a) => from_expr(a, axis)?.scale(&(-Rational::one())),
        Kind::Binary(op, a, b) => {
            let a = from_expr(a, axis)?;
            let b = from_expr(b, axis)?;
            match op {
                Op::Add | Op::Sub => a.sum(&b, *op == Op::Sub),
                Op::Mul => a.product(&b),
                Op::Div if b.0.len() == 1 => a.scale(&b.0[0].recip()),
                Op::Pow => {
                    let exponent = if b.zero() {
                        0
                    } else {
                        b.0.first()
                            .filter(|q| b.0.len() == 1 && q.is_integer())
                            .and_then(|q| q.to_integer().to_u32())
                            .ok_or(Issue::Unsupported(
                                "Polynomial exponent must be a nonnegative integer",
                            ))?
                    };
                    if exponent as usize > MAX_DEGREE {
                        return Err(Issue::Budget);
                    }
                    let mut out = Polynomial::one();
                    for _ in 0..exponent {
                        out = out.product(&a)?;
                    }
                    Ok(out)
                }
                _ => Err(Issue::Unsupported(
                    "Expression is outside the rational polynomial subset",
                )),
            }
        }
        _ => Err(Issue::Unsupported(
            "Expression is outside the rational polynomial subset",
        )),
    }
}

fn gcd(mut a: Polynomial, mut b: Polynomial) -> Result<Polynomial> {
    while !b.zero() {
        let (_, remainder) = a.divide(&b)?;
        a = b;
        b = remainder;
    }
    a.monic()
}

fn sturm(polynomial: &Polynomial) -> Result<Vec<Polynomial>> {
    let mut sequence = vec![polynomial.clone()];
    let mut previous = polynomial.clone();
    let mut current = polynomial.derivative()?;
    while !current.zero() {
        let (_, remainder) = previous.divide(&current)?;
        sequence.push(current.clone());
        previous = current;
        current = if let Some(lead) = remainder.0.last() {
            remainder.scale(&(-lead.abs().recip()))?
        } else {
            remainder
        };
    }
    Ok(sequence)
}

fn variation(sequence: &[Polynomial], x: &Rational) -> Result<usize> {
    let mut previous = Ordering::Equal;
    let mut count = 0;
    for p in sequence {
        let sign = p.at(x)?.cmp(&Rational::zero());
        if sign == Ordering::Equal {
            continue;
        }
        if previous != Ordering::Equal && previous != sign {
            count += 1;
        }
        previous = sign;
    }
    Ok(count)
}

fn open_count(sequence: &[Polynomial], a: &Rational, b: &Rational) -> Result<usize> {
    let left = variation(sequence, a)?;
    let right = variation(sequence, b)? + usize::from(sequence[0].at(b)?.is_zero());
    left.checked_sub(right).ok_or(Issue::Range)
}

fn square_free(poly: &Polynomial) -> Result<(Polynomial, Vec<(usize, Vec<Polynomial>)>)> {
    let p = poly.monic()?;
    let mut c = gcd(p.clone(), p.derivative()?)?;
    let distinct = p.quotient(&c)?;
    let mut w = distinct.clone();
    let mut multiplicity = 1;
    let mut factors = Vec::new();
    while !w.unit() {
        let y = gcd(w.clone(), c.clone())?;
        let f = w.quotient(&y)?;
        if !f.unit() {
            factors.push((multiplicity, sturm(&f)?));
        }
        c = c.quotient(&y)?;
        w = y;
        multiplicity += 1;
    }
    Ok((distinct, factors))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExactRoot {
    pub low: String,
    pub high: String,
    pub multiplicity: usize,
}

#[derive(Debug, Serialize)]
pub struct Isolation {
    pub distinct_count: usize,
    pub multiplicity_count: usize,
    pub roots: Vec<ExactRoot>,
    pub unresolved: Vec<[String; 2]>,
    pub degree: usize,
    pub processed: usize,
}

type Factors = [(usize, Vec<Polynomial>)];

fn multiplicity(factors: &Factors, a: &Rational, b: &Rational) -> Result<usize> {
    for (m, sequence) in factors {
        let count = if a == b {
            usize::from(sequence[0].at(a)?.is_zero())
        } else {
            open_count(sequence, a, b)?
        };
        if count == 1 {
            return Ok(*m);
        }
    }
    Err(Issue::Range)
}

fn certificate(factors: &Factors, a: &Rational, b: &Rational) -> Result<ExactRoot> {
    Ok(ExactRoot {
        low: a.to_string(),
        high: b.to_string(),
        multiplicity: multiplicity(factors, a, b)?,
    })
}

pub fn isolate(
    expr: &Expr,
    domain: [&str; 2],
    tolerance: &str,
    budget: usize,
) -> Result<Isolation> {
    let a = ratio(domain[0])?;
    let b = ratio(domain[1])?;
    let tolerance = ratio(tolerance)?;
    if a >= b || tolerance <= Rational::zero() || budget == 0 || budget > 32_768 {
        return Err(Issue::Unsupported("Invalid exact isolation limits"));
    }
    let poly = from_expr(expr, "x")?;
    if poly.zero() {
        return Err(Issue::Unsupported(
            "Identically zero polynomial has infinitely many roots",
        ));
    }
    let (distinct, factors) = square_free(&poly)?;
    let sequence = sturm(&distinct)?;
    let count = open_count(&sequence, &a, &b)?
        + usize::from(distinct.at(&a)?.is_zero())
        + usize::from(distinct.at(&b)?.is_zero());
    let mut weighted = 0;
    for (m, sequence) in &factors {
        weighted += m
            * (open_count(sequence, &a, &b)?
                + usize::from(sequence[0].at(&a)?.is_zero())
                + usize::from(sequence[0].at(&b)?.is_zero()));
    }
    let mut out = Isolation {
        distinct_count: count,
        multiplicity_count: weighted,
        degree: poly.0.len() - 1,
        roots: Vec::new(),
        unresolved: Vec::new(),
        processed: 0,
    };
    for point in [&a, &b] {
        if distinct.at(point)?.is_zero() {
            out.roots.push(certificate(&factors, point, point)?);
        }
    }
    let mut pending = vec![(a, b, 0)];
    while let Some((a, b, depth)) = pending.pop() {
        let count = open_count(&sequence, &a, &b)?;
        if count == 0 {
            continue;
        }
        if out.processed == budget || depth > 128 {
            out.unresolved.push([a.to_string(), b.to_string()]);
            continue;
        }
        out.processed += 1;
        if count == 1 && &b - &a <= tolerance {
            out.roots.push(certificate(&factors, &a, &b)?);
            continue;
        }
        let mid = checked((&a + &b) / Rational::from_integer(2.into()))?;
        if distinct.at(&mid)?.is_zero() {
            out.roots.push(certificate(&factors, &mid, &mid)?);
        }
        pending.push((mid.clone(), b, depth + 1));
        pending.push((a, mid, depth + 1));
    }
    out.roots
        .sort_by_cached_key(|root| (ratio(&root.low).unwrap(), ratio(&root.high).unwrap()));
    Ok(out)
}
