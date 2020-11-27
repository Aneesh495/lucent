use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

use crate::interval::{Issue, Result};

pub type Rational = BigRational;
pub const MAX_BITS: u64 = 16_384;

pub fn decimal(source: &str) -> Result<Rational> {
    let (significand, exponent) = match source.split_once(['e', 'E']) {
        Some((m, e)) => (m, e.parse::<i32>().map_err(|_| Issue::Range)?),
        None => (source, 0),
    };
    let (left, right) = significand.split_once('.').unwrap_or((significand, ""));
    if left.len() + right.len() > 512 || exponent.unsigned_abs() > 4096 {
        return Err(Issue::Budget);
    }
    let digits = format!("{left}{right}");
    let mantissa = BigInt::parse_bytes(digits.as_bytes(), 10).ok_or(Issue::Range)?;
    let scale = exponent - right.len() as i32;
    let power = BigInt::from(10).pow(scale.unsigned_abs());
    let ratio = if scale >= 0 {
        Rational::from_integer(mantissa * power)
    } else {
        Rational::new(mantissa, power)
    };
    checked(ratio)
}

pub fn checked(number: Rational) -> Result<Rational> {
    if number.numer().bits().max(number.denom().bits()) > MAX_BITS {
        Err(Issue::Budget)
    } else {
        Ok(number)
    }
}

pub fn power(base: &Rational, exponent: i32) -> Result<Rational> {
    if exponent == 0 {
        return Ok(Rational::one());
    }
    if base.is_zero() && exponent < 0 {
        return Err(Issue::Excluded("Negative power of zero"));
    }
    if exponent.unsigned_abs() > 4096
        || base.numer().bits().max(base.denom().bits()) * u64::from(exponent.unsigned_abs())
            > MAX_BITS
    {
        return Err(Issue::Budget);
    }
    checked(base.pow(exponent))
}

pub fn ratio(text: &str) -> Result<Rational> {
    if text.len() > 10_000 {
        return Err(Issue::Budget);
    }
    if let Some((n, d)) = text.split_once('/') {
        let n = n.parse::<BigInt>().map_err(|_| Issue::Range)?;
        let d = d.parse::<BigInt>().map_err(|_| Issue::Range)?;
        if d.is_zero() {
            return Err(Issue::Excluded("Denominator is zero"));
        }
        checked(Rational::new(n, d))
    } else {
        decimal(text)
    }
}
