use std::{cmp::Ordering, fmt};

use crate::mp::{
    Binary,
    Direction::{Down, Up},
    Float, RangeError, Unary,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Issue {
    Excluded(&'static str),
    Split(&'static str),
    Unsupported(&'static str),
    Budget,
    Range,
}

impl Issue {
    pub fn code(self) -> &'static str {
        match self {
            Self::Excluded(_) => "domain-excluded",
            Self::Split(_) => "domain-unresolved",
            Self::Unsupported(_) => "unsupported",
            Self::Budget => "budget",
            Self::Range => "numeric-range",
        }
    }
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Excluded(s) | Self::Split(s) | Self::Unsupported(s) => s,
            Self::Budget => "Computation reached its resource budget",
            Self::Range => "Numeric value exceeded the supported precision or output range",
        })
    }
}

impl From<RangeError> for Issue {
    fn from(_: RangeError) -> Self {
        Self::Range
    }
}

pub type Result<T> = std::result::Result<T, Issue>;

#[derive(Clone)]
pub struct Interval {
    low: Float,
    high: Float,
}

impl fmt::Debug for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Interval").field(&self.bounds()).finish()
    }
}

impl Interval {
    pub fn new(low: f64, high: f64, bits: u32) -> Result<Self> {
        if low > high {
            return Err(Issue::Range);
        }
        Ok(Self {
            low: Float::from_f64(low, bits, Down)?,
            high: Float::from_f64(high, bits, Up)?,
        })
    }

    pub fn point(x: f64, bits: u32) -> Result<Self> {
        Self::new(x, x, bits)
    }

    pub fn ratio(ratio: &str, bits: u32) -> Result<Self> {
        Ok(Self {
            low: Float::ratio(ratio, bits, Down)?,
            high: Float::ratio(ratio, bits, Up)?,
        })
    }

    pub fn pi(bits: u32) -> Result<Self> {
        Ok(Self {
            low: Float::pi(bits, Down)?,
            high: Float::pi(bits, Up)?,
        })
    }

    pub fn bits(&self) -> u32 {
        self.low.bits()
    }

    pub fn bounds(&self) -> Result<[f64; 2]> {
        let bounds = [self.low.to_f64(Down), self.high.to_f64(Up)];
        if bounds.iter().all(|x| x.is_finite()) {
            Ok(bounds)
        } else {
            Err(Issue::Range)
        }
    }

    pub fn sign(&self) -> Option<Ordering> {
        match (self.low.sign(), self.high.sign()) {
            (Ordering::Greater, _) => Some(Ordering::Greater),
            (_, Ordering::Less) => Some(Ordering::Less),
            (Ordering::Equal, Ordering::Equal) => Some(Ordering::Equal),
            _ => None,
        }
    }

    pub fn contains_zero(&self) -> bool {
        self.low.sign() != Ordering::Greater && self.high.sign() != Ordering::Less
    }

    pub fn is_point(&self) -> bool {
        self.low.compare(&self.high).is_eq()
    }

    pub fn strictly_before(&self, other: &Self) -> bool {
        self.high.compare(&other.low).is_lt()
    }
    pub fn before_or_touches(&self, other: &Self) -> bool {
        !self.high.compare(&other.low).is_gt()
    }

    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let low = max(&self.low, &other.low);
        let high = min(&self.high, &other.high);
        if low.compare(high).is_gt() {
            None
        } else {
            Some(Self {
                low: low.clone(),
                high: high.clone(),
            })
        }
    }

    pub fn hull(&self, other: &Self) -> Self {
        Self {
            low: min(&self.low, &other.low).clone(),
            high: max(&self.high, &other.high).clone(),
        }
    }

    pub fn add(&self, b: &Self) -> Result<Self> {
        Ok(Self {
            low: self.low.binary(&b.low, Binary::Add, Down)?,
            high: self.high.binary(&b.high, Binary::Add, Up)?,
        })
    }

    pub fn sub(&self, b: &Self) -> Result<Self> {
        Ok(Self {
            low: self.low.binary(&b.high, Binary::Sub, Down)?,
            high: self.high.binary(&b.low, Binary::Sub, Up)?,
        })
    }

    pub fn neg(&self) -> Result<Self> {
        Ok(Self {
            low: self.high.unary(Unary::Neg, Down)?,
            high: self.low.unary(Unary::Neg, Up)?,
        })
    }

    pub fn mul(&self, b: &Self) -> Result<Self> {
        self.corners(b, Binary::Mul)
    }

    pub fn div(&self, b: &Self) -> Result<Self> {
        if b.sign() == Some(Ordering::Equal) {
            return Err(Issue::Excluded("Denominator is zero"));
        }
        if b.contains_zero() {
            return Err(Issue::Split("Denominator enclosure includes zero"));
        }
        self.corners(b, Binary::Div)
    }

    pub fn principal_argument(&self, real: &Self) -> Result<Self> {
        if self.sign() == Some(Ordering::Equal) && real.sign() == Some(Ordering::Less) {
            return Self::pi(self.bits());
        }
        if self.contains_zero() && real.contains_zero() {
            return Err(Issue::Split("Complex rectangle may contain the origin"));
        }
        if self.low.sign().is_lt() && !self.high.sign().is_lt() && real.low.sign().is_lt() {
            return Err(Issue::Split(
                "Complex rectangle crosses the principal branch cut",
            ));
        }
        let angle = self.corners(real, Binary::Atan2)?;
        if real.low.sign().is_lt() && self.contains_zero() {
            return Ok(angle.hull(&Self::pi(self.bits())?));
        }
        Ok(angle)
    }

    pub fn nonnegative_part(&self) -> Result<Self> {
        if self.high.sign().is_lt() {
            return Err(Issue::Range);
        }
        let zero = Float::from_f64(0.0, self.bits(), Down)?;
        Ok(Self {
            low: max(&zero, &self.low).clone(),
            high: self.high.clone(),
        })
    }

    fn corners(&self, other: &Self, op: Binary) -> Result<Self> {
        let mut low = self.low.binary(&other.low, op, Down)?;
        let mut high = self.low.binary(&other.low, op, Up)?;
        for (a, b) in [
            (&self.low, &other.high),
            (&self.high, &other.low),
            (&self.high, &other.high),
        ] {
            let down = a.binary(b, op, Down)?;
            let up = a.binary(b, op, Up)?;
            if down.compare(&low).is_lt() {
                low = down;
            }
            if up.compare(&high).is_gt() {
                high = up;
            }
        }
        Ok(Self { low, high })
    }

    pub fn pow(&self, exponent: i32) -> Result<Self> {
        if exponent.unsigned_abs() > 4096 {
            return Err(Issue::Budget);
        }
        if exponent < 0 {
            return Self::point(1.0, self.bits())?.div(&self.pow(-exponent)?);
        }
        if exponent == 0 {
            return Self::point(1.0, self.bits());
        }
        if exponent % 2 != 0 || !self.low.sign().is_lt() {
            return Ok(Self {
                low: self.low.pow(exponent, Down)?,
                high: self.high.pow(exponent, Up)?,
            });
        }
        if !self.high.sign().is_gt() {
            return Ok(Self {
                low: self.high.pow(exponent, Down)?,
                high: self.low.pow(exponent, Up)?,
            });
        }
        let a = self.low.pow(exponent, Up)?;
        let b = self.high.pow(exponent, Up)?;
        let high = if a.compare(&b).is_lt() { b } else { a };
        Ok(Self {
            low: Float::from_f64(0.0, self.bits(), Down)?,
            high,
        })
    }

    pub fn unary(&self, op: Unary) -> Result<Self> {
        match op {
            Unary::Neg => return self.neg(),
            Unary::Sqrt if self.high.sign().is_lt() => {
                return Err(Issue::Excluded("Square root needs a nonnegative argument"));
            }
            Unary::Sqrt if self.low.sign().is_lt() => {
                return Err(Issue::Split("Square-root argument may be negative"));
            }
            Unary::Log if !self.high.sign().is_gt() => {
                return Err(Issue::Excluded("Logarithm needs a positive argument"));
            }
            Unary::Log if !self.low.sign().is_gt() => {
                return Err(Issue::Split("Logarithm argument may be nonpositive"));
            }
            Unary::Sin | Unary::Cos => return self.periodic(op),
            Unary::Tan => {
                let normalized = self
                    .div(&Self::pi(self.bits())?)?
                    .sub(&Self::point(0.5, self.bits())?)?;
                if normalized.has_integer() {
                    return Err(Issue::Split("Tangent enclosure may include a pole"));
                }
            }
            Unary::Abs => {
                if self.low.sign().is_lt() {
                    if !self.high.sign().is_gt() {
                        return self.neg();
                    }
                    let negative = self.low.unary(Unary::Abs, Up)?;
                    let high = if negative.compare(&self.high).is_gt() {
                        negative
                    } else {
                        self.high.clone()
                    };
                    return Ok(Self {
                        low: Float::from_f64(0.0, self.bits(), Down)?,
                        high,
                    });
                }
            }
            Unary::Erfc => {
                return Ok(Self {
                    low: self.high.unary(op, Down)?,
                    high: self.low.unary(op, Up)?,
                });
            }
            _ => {}
        }
        Ok(Self {
            low: self.low.unary(op, Down)?,
            high: self.high.unary(op, Up)?,
        })
    }

    fn has_integer(&self) -> bool {
        self.low.has_integer_through(&self.high)
    }

    fn periodic(&self, op: Unary) -> Result<Self> {
        let bits = self.bits();
        let scaled = self.div(&Self::pi(bits)?)?;
        let offset = if matches!(op, Unary::Sin) { 0.5 } else { 0.0 };
        let phases = scaled
            .sub(&Self::point(offset, bits)?)?
            .div(&Self::point(2.0, bits)?)?;
        let contains_peak = phases.has_integer();
        let contains_trough = phases.sub(&Self::point(0.5, bits)?)?.has_integer();
        let low = if contains_trough {
            Float::from_f64(-1.0, bits, Down)?
        } else {
            let a = self.low.unary(op, Down)?;
            let b = self.high.unary(op, Down)?;
            if a.compare(&b).is_lt() { a } else { b }
        };
        let high = if contains_peak {
            Float::from_f64(1.0, bits, Up)?
        } else {
            let a = self.low.unary(op, Up)?;
            let b = self.high.unary(op, Up)?;
            if a.compare(&b).is_gt() { a } else { b }
        };
        Ok(Self { low, high })
    }
}

fn min<'a>(a: &'a Float, b: &'a Float) -> &'a Float {
    if a.compare(b).is_lt() { a } else { b }
}

fn max<'a>(a: &'a Float, b: &'a Float) -> &'a Float {
    if a.compare(b).is_gt() { a } else { b }
}
