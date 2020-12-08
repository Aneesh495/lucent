use num_traits::{One, Zero};

use crate::{
    exact::{Rational, checked},
    interval::{Issue, Result},
    lang::Op,
    value::{Real, Value},
};

pub fn call(name: &str, mut args: Vec<Value>, bits: u32) -> Result<Value> {
    match (name, args.len()) {
        ("dot", 2) => {
            let b = args.pop().unwrap().list()?;
            let a = args.pop().unwrap().list()?;
            if a.is_empty() || a.len() != b.len() {
                return Err(Issue::Unsupported(
                    "dot requires nonempty vectors of equal length",
                ));
            }
            let mut products = a
                .into_iter()
                .zip(b)
                .map(|(a, b)| a.arithmetic(Op::Mul, b, bits));
            let first = products.next().unwrap()?;
            products.try_fold(first, |a, b| a.arithmetic(Op::Add, b?, bits))
        }
        ("transpose", 1) => {
            let rows = rows(args.remove(0))?;
            let width = rows[0].len();
            let mut cols: Vec<_> = (0..width).map(|_| Vec::new()).collect();
            for row in rows {
                for (col, value) in cols.iter_mut().zip(row) {
                    col.push(value);
                }
            }
            Ok(Value::List(cols.into_iter().map(Value::List).collect()))
        }
        ("det", 1) => {
            let mut matrix = square(args.remove(0))?;
            let determinant = eliminate(&mut matrix, None)?;
            Ok(Value::real(Real::Exact(determinant)))
        }
        ("solve", 2) => {
            let mut rhs = args
                .pop()
                .unwrap()
                .list()?
                .into_iter()
                .map(rational)
                .collect::<Result<Vec<_>>>()?;
            let mut matrix = square(args.remove(0))?;
            if rhs.len() != matrix.len() {
                return Err(Issue::Unsupported("Right-hand vector has incorrect length"));
            }
            let det = eliminate(&mut matrix, Some(&mut rhs))?;
            if det.is_zero() {
                return Err(Issue::Unsupported("Singular matrix has no unique solution"));
            }
            for i in (0..matrix.len()).rev() {
                for j in i + 1..matrix.len() {
                    rhs[i] = checked(&rhs[i] - checked(&matrix[i][j] * &rhs[j])?)?;
                }
                rhs[i] = checked(&rhs[i] / &matrix[i][i])?;
            }
            Ok(Value::List(
                rhs.into_iter()
                    .map(|q| Value::real(Real::Exact(q)))
                    .collect(),
            ))
        }
        _ => Err(Issue::Unsupported("Invalid linear algebra argument count")),
    }
}

fn rows(input: Value) -> Result<Vec<Vec<Value>>> {
    let rows = input
        .list()?
        .into_iter()
        .map(Value::list)
        .collect::<Result<Vec<_>>>()?;
    if rows.is_empty() || rows[0].is_empty() || rows.iter().any(|row| row.len() != rows[0].len()) {
        return Err(Issue::Unsupported(
            "A nonempty rectangular matrix is required",
        ));
    }
    Ok(rows)
}

fn rational(value: Value) -> Result<Rational> {
    let q = value.quantity()?;
    if let Real::Exact(real) = q.dimensionless()? {
        Ok(real.clone())
    } else {
        Err(Issue::Unsupported(
            "Exact linear algebra requires rational entries",
        ))
    }
}

fn square(input: Value) -> Result<Vec<Vec<Rational>>> {
    let matrix = rows(input)?;
    if matrix.len() != matrix[0].len() || matrix.len() > 12 {
        return Err(Issue::Unsupported(
            "A square matrix of order 1 through 12 is required",
        ));
    }
    matrix
        .into_iter()
        .map(|row| row.into_iter().map(rational).collect())
        .collect()
}

fn eliminate(matrix: &mut [Vec<Rational>], mut rhs: Option<&mut [Rational]>) -> Result<Rational> {
    let n = matrix.len();
    let mut determinant = Rational::one();
    for col in 0..n {
        let Some(pivot) = (col..n).find(|row| !matrix[*row][col].is_zero()) else {
            return Ok(Rational::zero());
        };
        if col != pivot {
            matrix.swap(col, pivot);
            if let Some(rhs) = &mut rhs {
                rhs.swap(col, pivot);
            }
            determinant = -determinant;
        }
        determinant = checked(determinant * &matrix[col][col])?;
        for row in col + 1..n {
            if matrix[row][col].is_zero() {
                continue;
            }
            let scale = checked(&matrix[row][col] / &matrix[col][col])?;
            matrix[row][col] = Rational::zero();
            for j in col + 1..n {
                matrix[row][j] = checked(&matrix[row][j] - checked(&scale * &matrix[col][j])?)?;
            }
            if let Some(rhs) = &mut rhs {
                rhs[row] = checked(&rhs[row] - checked(&scale * &rhs[col])?)?;
            }
        }
    }
    Ok(determinant)
}
