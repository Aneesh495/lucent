use serde::Serialize;

use crate::interval::{Issue, Result};

type Matrix = Vec<Vec<f64>>;

fn dimensions(matrix: &[Vec<f64>]) -> Result<(usize, usize)> {
    let m = matrix.len();
    let n = matrix.first().map_or(0, Vec::len);
    if !(1..=512).contains(&m)
        || !(1..=32).contains(&n)
        || m < n
        || matrix
            .iter()
            .any(|r| r.len() != n || r.iter().any(|x| !x.is_finite() || x.abs() > 1e150))
    {
        return Err(Issue::Unsupported(
            "Expected a finite m by n matrix, with 1 <= n <= m, n <= 32 and m <= 512",
        ));
    }
    Ok((m, n))
}

fn norm(xs: impl IntoIterator<Item = f64>) -> f64 {
    xs.into_iter().fold(0.0, f64::hypot)
}

fn stable_dot(pairs: impl IntoIterator<Item = (f64, f64)>) -> f64 {
    let mut sum = 0.0;
    let mut compensation = 0.0;
    for (x, y) in pairs {
        let add = x * y - compensation;
        let updated = sum + add;
        compensation = (updated - sum) - add;
        sum = updated;
    }
    sum
}

struct Pivoted {
    data: Matrix,
    permutation: Vec<usize>,
    transformed: Vec<f64>,
    scale: f64,
    rank: usize,
}

fn factor(matrix: &[Vec<f64>], b: Option<&[f64]>) -> Result<Pivoted> {
    let (m, n) = dimensions(matrix)?;
    if b.is_some_and(|b| b.len() != m || b.iter().any(|x| !x.is_finite() || x.abs() > 1e150)) {
        return Err(Issue::Unsupported(
            "Right-hand side has invalid length or values",
        ));
    }
    let scale = matrix
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0.0, f64::max)
        .max(f64::MIN_POSITIVE);
    let mut data: Matrix = matrix
        .iter()
        .map(|row| row.iter().map(|x| x / scale).collect())
        .collect();
    let mut transformed = b.map_or_else(Vec::new, <[f64]>::to_vec);
    let mut permutation: Vec<_> = (0..n).collect();
    for k in 0..n {
        let Some((pivot, length)) = (k..n)
            .map(|j| (j, norm((k..m).map(|i| data[i][j]))))
            .max_by(|a, b| a.1.total_cmp(&b.1))
        else {
            unreachable!()
        };
        if length == 0.0 {
            break;
        }
        for row in &mut data {
            row.swap(k, pivot);
        }
        permutation.swap(k, pivot);
        let diagonal = -length.copysign(data[k][k]);
        let mut reflector: Vec<f64> = (k..m).map(|i| data[i][k]).collect();
        reflector[0] -= diagonal;
        let reflector_norm = norm(reflector.iter().copied());
        for x in &mut reflector {
            *x /= reflector_norm;
        }
        for j in k + 1..n {
            let factor =
                2.0 * stable_dot(reflector.iter().copied().zip((k..m).map(|i| data[i][j])));
            for (i, &x) in (k..m).zip(&reflector) {
                data[i][j] -= factor * x;
            }
        }
        if b.is_some() {
            let factor = 2.0
                * stable_dot(
                    reflector
                        .iter()
                        .copied()
                        .zip(transformed[k..].iter().copied()),
                );
            for (y, &x) in transformed[k..].iter_mut().zip(&reflector) {
                *y -= factor * x;
            }
        }
        data[k][k] = diagonal;
        for row in data.iter_mut().skip(k + 1) {
            row[k] = 0.0;
        }
    }
    let threshold = data[0][0].abs() * f64::EPSILON * m.max(n) as f64;
    let rank = (0..n).take_while(|&i| data[i][i].abs() > threshold).count();
    Ok(Pivoted {
        data,
        permutation,
        transformed,
        scale,
        rank,
    })
}

fn back_substitute(upper: &Matrix, rhs: &[f64], n: usize) -> Vec<f64> {
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let sum = stable_dot(
            upper[i][i + 1..n]
                .iter()
                .copied()
                .zip(x[i + 1..].iter().copied()),
        );
        x[i] = (rhs[i] - sum) / upper[i][i];
    }
    x
}

impl Pivoted {
    fn solution(&self) -> Option<Vec<f64>> {
        let n = self.permutation.len();
        if self.rank != n || self.transformed.is_empty() {
            return None;
        }
        let pivoted = back_substitute(&self.data, &self.transformed, n);
        let mut solution = vec![0.0; n];
        for (i, &j) in self.permutation.iter().enumerate() {
            solution[j] = pivoted[i] / self.scale;
        }
        Some(solution)
    }

    fn covariance_diagonal(&self) -> Option<Vec<f64>> {
        let n = self.permutation.len();
        if self.rank != n {
            return None;
        }
        let mut sum = vec![0.0_f64; n];
        let mut basis = vec![0.0; n];
        for j in 0..n {
            basis[j] = 1.0;
            let column = back_substitute(&self.data, &basis, n);
            for i in 0..n {
                sum[self.permutation[i]] = sum[self.permutation[i]].hypot(column[i] / self.scale);
            }
            basis[j] = 0.0;
        }
        Some(sum)
    }
}

#[derive(Debug, Serialize)]
pub struct Qr {
    pub upper: Matrix,
    pub column_permutation: Vec<usize>,
    pub rank: usize,
    pub solution: Option<Vec<f64>>,
    pub residual_norm: Option<f64>,
}

pub fn qr(matrix: &[Vec<f64>], rhs: Option<&[f64]>) -> Result<Qr> {
    let factor = factor(matrix, rhs)?;
    let n = factor.permutation.len();
    let solution = factor.solution();
    if solution.iter().flatten().any(|x| !x.is_finite()) {
        return Err(Issue::Range);
    }
    let residual_norm = rhs.zip(solution.as_ref()).map(|(b, x)| {
        norm(
            matrix
                .iter()
                .zip(b)
                .map(|(row, y)| stable_dot(row.iter().copied().zip(x.iter().copied())) - y),
        )
    });
    if residual_norm.is_some_and(|x| !x.is_finite()) {
        return Err(Issue::Range);
    }
    let upper = factor
        .data
        .iter()
        .take(n)
        .map(|r| r.iter().map(|x| x * factor.scale).collect())
        .collect();
    Ok(Qr {
        upper,
        column_permutation: factor.permutation,
        rank: factor.rank,
        solution,
        residual_norm,
    })
}

#[derive(Debug, Serialize)]
pub struct Svd {
    pub singular_values: Vec<f64>,
    pub right: Matrix,
    pub left: Matrix,
    pub rank: usize,
    pub condition: Option<f64>,
    pub relative_residual: f64,
    pub converged: bool,
    pub sweeps: usize,
}

pub fn svd(matrix: &[Vec<f64>]) -> Result<Svd> {
    let (m, n) = dimensions(matrix)?;
    let scale = matrix
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0.0, f64::max)
        .max(f64::MIN_POSITIVE);
    let mut data: Matrix = matrix
        .iter()
        .map(|row| row.iter().map(|x| x / scale).collect())
        .collect();
    let mut right = identity(n);
    let mut converged = false;
    let mut sweeps = 0;
    for _ in 0..80 {
        sweeps += 1;
        let mut rotated = false;
        for p in 0..n {
            for q in p + 1..n {
                let p_norm = norm(data.iter().map(|r| r[p]));
                let q_norm = norm(data.iter().map(|r| r[q]));
                if p_norm <= f64::MIN_POSITIVE || q_norm <= f64::MIN_POSITIVE {
                    continue;
                }
                let overlap = stable_dot(data.iter().map(|r| (r[p] / p_norm, r[q] / q_norm)));
                if overlap.abs() <= 8.0 * f64::EPSILON {
                    continue;
                }
                let ratio = (p_norm / q_norm - q_norm / p_norm) / (2.0 * overlap);
                let tangent = -1.0_f64.copysign(ratio) / (ratio.abs() + ratio.hypot(1.0));
                if tangent == 0.0 {
                    continue;
                }
                let cosine = 1.0 / tangent.hypot(1.0);
                let sine = tangent * cosine;
                rotate(&mut data, p, q, cosine, sine);
                rotate(&mut right, p, q, cosine, sine);
                rotated = true;
            }
        }
        if !rotated {
            converged = true;
            break;
        }
    }
    let lengths: Vec<_> = (0..n)
        .map(|j| norm(data.iter().map(|row| row[j])))
        .collect();
    let mut order: Vec<_> = (0..n).collect();
    order.sort_by(|&a, &b| lengths[b].total_cmp(&lengths[a]));
    let singular_values: Vec<_> = order.iter().map(|&j| lengths[j] * scale).collect();
    let relative_threshold = f64::EPSILON * m.max(n) as f64;
    let max = lengths[order[0]];
    let rank = lengths
        .iter()
        .filter(|&&length| length > max * relative_threshold)
        .count();
    let condition = (rank == n && converged).then(|| max / lengths[order[n - 1]]);
    let left = data
        .iter()
        .map(|row| {
            order
                .iter()
                .map(|&j| {
                    if lengths[j] > 0.0 {
                        row[j] / lengths[j]
                    } else {
                        0.0
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Matrix>();
    let right = right
        .iter()
        .map(|row| order.iter().map(|&j| row[j]).collect::<Vec<_>>())
        .collect::<Matrix>();
    let mut error = 0.0_f64;
    let mut size = 0.0_f64;
    for (i, row) in matrix.iter().enumerate() {
        for (j, entry) in row.iter().enumerate() {
            let normalized = entry / scale;
            let rebuilt = stable_dot((0..n).map(|k| (left[i][k] * lengths[order[k]], right[j][k])));
            error = error.hypot(rebuilt - normalized);
            size = size.hypot(normalized);
        }
    }
    let relative_residual = if size > 0.0 { error / size } else { 0.0 };
    Ok(Svd {
        singular_values,
        right,
        left,
        rank,
        condition,
        relative_residual,
        converged,
        sweeps,
    })
}

fn identity(n: usize) -> Matrix {
    (0..n)
        .map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
        .collect()
}

fn rotate(data: &mut Matrix, p: usize, q: usize, cosine: f64, sine: f64) {
    for row in data {
        let (a, b) = (row[p], row[q]);
        row[p] = cosine * a - sine * b;
        row[q] = sine * a + cosine * b;
    }
}

#[derive(Debug, Serialize)]
pub struct FitPoint {
    pub x: f64,
    pub y: f64,
    pub fitted: f64,
    pub residual: f64,
}

#[derive(Debug, Serialize)]
pub struct Regression {
    pub coefficients: Option<Vec<f64>>,
    pub coefficient_std_error: Option<Vec<f64>>,
    pub center: f64,
    pub scale: f64,
    pub rank: usize,
    pub condition: Option<f64>,
    pub residual_norm: Option<f64>,
    pub residual_std: Option<f64>,
    pub degrees_of_freedom: usize,
    pub r_squared: Option<f64>,
    pub points: Vec<FitPoint>,
}

pub fn regress(data: &[[f64; 2]], degree: usize) -> Result<Regression> {
    if degree > 6
        || data.len() < degree + 1
        || data.len() > 512
        || data
            .iter()
            .flatten()
            .any(|x| !x.is_finite() || x.abs() > 1e100)
    {
        return Err(Issue::Unsupported(
            "Fit requires degree 0 through 6 and between degree+1 and 512 finite observations",
        ));
    }
    let min = data.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
    let max = data.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
    let center = min.midpoint(max);
    let scale = (max - min) / 2.0;
    let scale = if scale == 0.0 { 1.0 } else { scale };
    let mut matrix = Vec::with_capacity(data.len());
    for &[x, _] in data {
        let t = (x - center) / scale;
        let mut row = vec![1.0];
        for j in 0..degree {
            row.push(t * row[j]);
        }
        matrix.push(row);
    }
    let rhs: Vec<_> = data.iter().map(|p| p[1]).collect();
    let factor = factor(&matrix, Some(&rhs))?;
    let rank = factor.rank;
    let condition = svd(&matrix)?.condition;
    let dof = data.len() - rank;
    let mut report = Regression {
        coefficients: factor.solution(),
        coefficient_std_error: None,
        center,
        scale,
        rank,
        condition,
        residual_norm: None,
        residual_std: None,
        degrees_of_freedom: dof,
        r_squared: None,
        points: Vec::new(),
    };
    if let Some(coefficients) = &report.coefficients {
        for (&[x, y], row) in data.iter().zip(matrix) {
            let fitted = stable_dot(row.into_iter().zip(coefficients.iter().copied()));
            report.points.push(FitPoint {
                x,
                y,
                fitted,
                residual: y - fitted,
            });
        }
        let error = norm(report.points.iter().map(|p| p.residual));
        let average = rhs.iter().sum::<f64>() / rhs.len() as f64;
        let variation = norm(rhs.iter().map(|y| y - average));
        if !error.is_finite()
            || !variation.is_finite()
            || coefficients.iter().any(|x| !x.is_finite())
        {
            return Err(Issue::Range);
        }
        report.residual_norm = Some(error);
        if variation > 0.0 {
            report.r_squared = Some(1.0 - (error / variation).powi(2));
        }
        if dof > 0 {
            let standard_error = error / (dof as f64).sqrt();
            report.residual_std = Some(standard_error);
            let estimates = factor
                .covariance_diagonal()
                .ok_or(Issue::Range)?
                .into_iter()
                .map(|x| x * standard_error)
                .collect::<Vec<_>>();
            if estimates.iter().all(|x| x.is_finite()) {
                report.coefficient_std_error = Some(estimates);
            }
        }
    }
    Ok(report)
}
