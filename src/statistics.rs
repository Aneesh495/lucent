use num_traits::{One, Zero};
use serde::Serialize;

use crate::{
    exact::Rational,
    interval::{Interval, Issue, Result},
    mp::Unary,
};

#[derive(Debug, Serialize)]
pub struct Quantile {
    pub enclosure: Option<[f64; 2]>,
    pub limit: Option<&'static str>,
    pub precision: u32,
    pub target_met: bool,
    pub iterations: usize,
}

pub fn normal_quantile(
    probability: &Rational,
    mut bits: u32,
    max_bits: u32,
    tolerance: f64,
    budget: usize,
) -> Result<Quantile> {
    let one = Rational::one();
    let half = Rational::new(1.into(), 2.into());
    if probability < &Rational::zero()
        || probability > &one
        || !tolerance.is_finite()
        || tolerance <= 0.0
    {
        return Err(Issue::Unsupported(
            "Probability must be between zero and one",
        ));
    }
    if probability.is_zero() || probability.is_one() {
        return Ok(Quantile {
            enclosure: None,
            limit: Some(if probability.is_zero() {
                "-infinity"
            } else {
                "+infinity"
            }),
            precision: bits,
            target_met: true,
            iterations: 0,
        });
    }
    if probability == &half {
        return Ok(Quantile {
            enclosure: Some([0.0, 0.0]),
            limit: None,
            precision: bits,
            target_met: true,
            iterations: 0,
        });
    }
    let negative = probability < &half;
    let tail = if negative {
        probability.clone()
    } else {
        one - probability
    };
    let text = tail.to_string();
    let compare = |x, bits| -> Result<Option<std::cmp::Ordering>> {
        let two = Interval::point(2.0, bits)?;
        let scale = two.unary(Unary::Sqrt)?;
        let survival = Interval::point(x, bits)?
            .div(&scale)?
            .unary(Unary::Erfc)?
            .div(&two)?;
        Ok(survival.sub(&Interval::ratio(&text, bits)?)?.sign())
    };
    let mut lo = 0.0;
    let mut hi = 1.0;
    let mut iterations = 0;
    use std::cmp::Ordering::{Equal, Greater, Less};
    while compare(hi, bits)? != Some(Less) {
        hi *= 2.0;
        iterations += 1;
        if hi > 256.0 || iterations >= budget {
            return Err(Issue::Split(
                "A finite quantile bracket could not be established",
            ));
        }
    }
    while hi - lo > tolerance * hi.max(1.0) && iterations < budget.min(256) {
        let middle = lo.midpoint(hi);
        if middle == lo || middle == hi {
            break;
        }
        iterations += 1;
        match compare(middle, bits)? {
            Some(Less) => hi = middle,
            Some(Greater) => lo = middle,
            Some(Equal) => {
                lo = middle;
                hi = middle;
                break;
            }
            None if bits < max_bits => bits = (2 * bits).min(max_bits),
            None => break,
        }
    }
    Ok(Quantile {
        enclosure: Some(if negative { [-hi, -lo] } else { [lo, hi] }),
        limit: None,
        precision: bits,
        target_met: hi - lo <= tolerance * hi.max(1.0),
        iterations,
    })
}

#[derive(Serialize, Debug)]
pub struct ProbabilityPoint {
    pub expected: [f64; 2],
    pub observed: f64,
}

pub fn normal_plot(
    mut samples: Vec<f64>,
    bits: u32,
    max_bits: u32,
    tolerance: f64,
) -> Result<Vec<ProbabilityPoint>> {
    if !(3..=512).contains(&samples.len()) || samples.iter().any(|x| !x.is_finite()) {
        return Err(Issue::Unsupported(
            "Probability plot requires 3 through 512 finite samples",
        ));
    }
    samples.sort_by(f64::total_cmp);
    let denominator = 8 * samples.len() + 2;
    samples
        .into_iter()
        .enumerate()
        .map(|(i, observed)| {
            let numerator = 8 * (i + 1) - 3;
            let p = Rational::new(numerator.into(), denominator.into());
            let result = normal_quantile(&p, bits, max_bits, tolerance, 160)?;
            Ok(ProbabilityPoint {
                expected: result.enclosure.ok_or(Issue::Range)?,
                observed,
            })
        })
        .collect()
}
