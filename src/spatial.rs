use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::{
    graph::{Context, middle, valid_domain},
    interval::{Interval, Issue, Result},
    lang::Expr,
};

pub type Vec3 = [f64; 3];
pub type Range = [f64; 2];

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Space {
    pub x: Range,
    pub y: Range,
    pub z: Range,
}

impl Space {
    fn bounds(self) -> [Range; 3] {
        [self.x, self.y, self.z]
    }

    fn validate(self) -> Result<Self> {
        if self.bounds().iter().copied().all(valid_domain)
            && self.bounds().iter().flatten().all(|x| x.abs() <= 1e100)
        {
            Ok(self)
        } else {
            Err(Issue::Unsupported(
                "Spatial bounds must be finite, increasing and at most 1e100 in magnitude",
            ))
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Face {
    pub vertices: [Vec3; 3],
    pub normal: Vec3,
}

#[derive(Debug, Serialize)]
pub struct Gap {
    pub domain: Vec<Range>,
    pub reason: String,
    pub excluded: bool,
}

#[derive(Debug, Serialize)]
pub struct Mesh {
    pub faces: Vec<Face>,
    pub gaps: Vec<Gap>,
    pub processed: usize,
    pub interval_excluded: usize,
    pub space: Space,
}

impl Mesh {
    fn new(space: Space) -> Self {
        Self {
            faces: Vec::new(),
            gaps: Vec::new(),
            processed: 0,
            interval_excluded: 0,
            space,
        }
    }

    fn issue(&mut self, domain: &[Range], issue: Issue) {
        self.gaps.push(Gap {
            domain: domain.to_vec(),
            reason: issue.to_string(),
            excluded: matches!(issue, Issue::Excluded(_)),
        });
    }

    fn face(&mut self, vertices: [Vec3; 3], normal_direction: Option<Vec3>) -> bool {
        let a = subtract(vertices[1], vertices[0]);
        let b = subtract(vertices[2], vertices[0]);
        let scale = a.iter().chain(&b).map(|x| x.abs()).fold(0.0, f64::max);
        if scale == 0.0 || !scale.is_finite() {
            return false;
        }
        let a = a.map(|v| v / scale);
        let b = b.map(|v| v / scale);
        let normal = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        let norm = normal.into_iter().fold(0.0_f64, f64::hypot);
        if !norm.is_finite() || norm < 1e-13 {
            return false;
        }
        let mut face = Face {
            vertices,
            normal: normal.map(|v| v / norm),
        };
        if normal_direction.is_some_and(|other| dot(other, face.normal) < 0.0) {
            face.vertices.swap(1, 2);
            face.normal = face.normal.map(|v| -v);
        }
        self.faces.push(face);
        true
    }
}

pub enum Surface<'a> {
    Height(&'a Expr),
    Parametric([&'a Expr; 3]),
}

impl Surface<'_> {
    fn bounds(&self, ctx: &Context, [u, v]: [Range; 2]) -> Result<[Range; 3]> {
        let bits = ctx.control.bits;
        match self {
            Self::Height(height) => {
                let z = ctx
                    .real(height, &[("x", u), ("y", v)])?
                    .enclose(bits)?
                    .bounds()?;
                Ok([u, v, z])
            }
            Self::Parametric([x, y, z]) => {
                let bounds = |expr| {
                    ctx.real(expr, &[("u", u), ("v", v)])?
                        .enclose(bits)?
                        .bounds()
                };
                Ok([bounds(x)?, bounds(y)?, bounds(z)?])
            }
        }
    }

    fn sample(&self, ctx: &Context, u: f64, v: f64) -> Result<Vec3> {
        Ok(self.bounds(ctx, [[u; 2], [v; 2]])?.map(middle))
    }
}

pub fn surface(
    ctx: &Context,
    kind: &Surface,
    domain: [Range; 2],
    space: Space,
    depth: u8,
) -> Result<Mesh> {
    let space = space.validate()?;
    let control = ctx.control.check()?;
    if depth == 0 || depth > 8 || !domain.into_iter().all(valid_domain) {
        return Err(Issue::Unsupported("Invalid parameter domain or mesh depth"));
    }
    let span = space.bounds().map(|[a, b]| b - a);
    let mut mesh = Mesh::new(space);
    let mut stack = vec![(domain, 0)];
    while let Some((uv, level)) = stack.pop() {
        if mesh.processed == control.budget || mesh.faces.len() >= 48_000 {
            mesh.issue(&uv, Issue::Budget);
            continue;
        }
        mesh.processed += 1;
        let [u, v] = uv;
        let split = |queue: &mut Vec<_>| -> bool {
            let [m, n] = uv.map(middle);
            if level >= depth || u.contains(&m) || v.contains(&n) {
                return false;
            }
            for uu in [[u[0], m], [m, u[1]]] {
                for vv in [[v[0], n], [n, v[1]]] {
                    queue.push(([uu, vv], level + 1));
                }
            }
            true
        };
        let ranges = match kind.bounds(ctx, uv) {
            Ok(range) => range,
            Err(issue) => {
                if matches!(issue, Issue::Excluded(_)) || !split(&mut stack) {
                    mesh.issue(&uv, issue);
                }
                continue;
            }
        };
        if ranges
            .into_iter()
            .zip(space.bounds())
            .any(|(r, s)| r[0] > s[1] || r[1] < s[0])
        {
            mesh.interval_excluded += 1;
            continue;
        }
        if level < 3.min(depth) && split(&mut stack) {
            continue;
        }
        let stencil = [
            [u[0], v[0]],
            [u[1], v[0]],
            [u[1], v[1]],
            [u[0], v[1]],
            [middle(u), middle(v)],
            [middle(u), v[0]],
            [u[1], middle(v)],
            [middle(u), v[1]],
            [u[0], middle(v)],
        ];
        let points: Result<Vec<_>> = stencil
            .into_iter()
            .map(|[a, b]| kind.sample(ctx, a, b))
            .collect();
        let points = match points {
            Ok(points) => points,
            Err(issue) => {
                if !split(&mut stack) {
                    mesh.issue(&uv, issue);
                }
                continue;
            }
        };
        let acceptable = (0..3).all(|i| {
            let average = 0.25 * (points[0][i] + points[1][i] + points[2][i] + points[3][i]);
            let centre_error = (points[4][i] - average).abs();
            let edge_error = (0..4)
                .map(|k| (points[5 + k][i] - points[k][i].midpoint(points[(k + 1) % 4][i])).abs())
                .fold(0.0, f64::max);
            centre_error.max(edge_error) <= 0.004 * span[i]
                && ranges[i][1] - ranges[i][0] <= span[i] / 2.0
        });
        if !acceptable {
            if !split(&mut stack) {
                mesh.issue(
                    &uv,
                    Issue::Split("Surface stencil or interval width exceeds the mesh tolerance"),
                );
            }
            continue;
        }
        let mut degenerate = false;
        for j in 0..4 {
            degenerate |= !mesh.face([points[j], points[(j + 1) % 4], points[4]], None);
        }
        if degenerate {
            mesh.issue(&uv, Issue::Split("Degenerate surface triangle"));
        }
    }
    Ok(mesh)
}

pub fn iso(ctx: &Context, expr: &Expr, space: Space, depth: u8) -> Result<Mesh> {
    let space = space.validate()?;
    let controls = ctx.control.check()?;
    if !(2..=6).contains(&depth) {
        return Err(Issue::Unsupported("Isosurface depth must be 2 through 6"));
    }
    let mut mesh = Mesh::new(space);
    let mut work = vec![(space.bounds(), 0)];
    while let Some((bounds, level)) = work.pop() {
        if mesh.processed == controls.budget || mesh.faces.len() >= 48_000 {
            mesh.issue(&bounds, Issue::Budget);
            continue;
        }
        mesh.processed += 1;
        let result = residual(ctx, expr, bounds);
        if let Ok(range) = &result {
            if !range.contains_zero() {
                mesh.interval_excluded += 1;
                continue;
            }
        }
        if let Err(issue @ Issue::Excluded(_)) = &result {
            mesh.issue(&bounds, *issue);
            continue;
        }
        if level < depth && divide_octants(bounds, |cube| work.push((cube, level + 1))) {
            continue;
        }
        if let Err(issue) = result {
            mesh.issue(&bounds, issue);
            continue;
        }
        let corners: [Vec3; 8] =
            std::array::from_fn(|i| std::array::from_fn(|axis| bounds[axis][(i >> axis) & 1]));
        let values: Result<Vec<f64>> = corners
            .iter()
            .map(|point| {
                let r = residual(ctx, expr, point.map(|x| [x; 2]))?;
                if r.sign().is_none() {
                    return Err(Issue::Split("Undecided isosurface corner sign"));
                }
                Ok(middle(r.bounds()?))
            })
            .collect();
        let values = match values {
            Ok(values) => values,
            Err(issue) => {
                mesh.issue(&bounds, issue);
                continue;
            }
        };
        // Every cube uses the same body diagonal, so adjacent cubes split their shared faces consistently.
        const TETS: [[usize; 4]; 6] = [
            [0, 1, 3, 7],
            [0, 1, 5, 7],
            [0, 2, 3, 7],
            [0, 2, 6, 7],
            [0, 4, 5, 7],
            [0, 4, 6, 7],
        ];
        let before = mesh.faces.len();
        let mut ambiguous = false;
        for tetra in TETS {
            ambiguous |= !triangulate(&mut mesh, tetra, &corners, &values);
        }
        if ambiguous || before == mesh.faces.len() {
            mesh.issue(
                &bounds,
                Issue::Split("Zero is enclosed, but corner signs do not resolve local topology"),
            );
        }
    }
    Ok(mesh)
}

fn residual(ctx: &Context, expr: &Expr, [x, y, z]: [Range; 3]) -> Result<Interval> {
    ctx.real(expr, &[("x", x), ("y", y), ("z", z)])?
        .enclose(ctx.control.bits)
}

fn divide_octants(bounds: [Range; 3], mut add: impl FnMut([Range; 3])) -> bool {
    let center = bounds.map(middle);
    if (0..3).any(|axis| bounds[axis].contains(&center[axis])) {
        return false;
    }
    for i in 0..8 {
        add(std::array::from_fn(|axis| {
            if (i >> axis) & 1 == 0 {
                [bounds[axis][0], center[axis]]
            } else {
                [center[axis], bounds[axis][1]]
            }
        }));
    }
    true
}

fn triangulate(mesh: &mut Mesh, tetra: [usize; 4], points: &[Vec3; 8], values: &[f64]) -> bool {
    let mut positive = Vec::new();
    let mut negative = Vec::new();
    for i in tetra {
        match values[i].partial_cmp(&0.0) {
            Some(Ordering::Less) => negative.push(i),
            Some(_) => positive.push(i),
            _ => return false,
        }
    }
    if negative.is_empty() || positive.is_empty() {
        return true;
    }
    let point = |a: usize, b: usize| -> Vec3 {
        // Normalization avoids overflow when both residuals are close to the largest finite double.
        let da = values[a].abs();
        let db = values[b].abs();
        let scale = da.max(db);
        let t = (da / scale) / (da / scale + db / scale);
        std::array::from_fn(|i| points[a][i] + (points[b][i] - points[a][i]) * t)
    };
    let centroid = |group: &[usize]| -> Vec3 {
        std::array::from_fn(|i| {
            group.iter().map(|j| points[*j][i]).sum::<f64>() / group.len() as f64
        })
    };
    let direction = subtract(centroid(&positive), centroid(&negative));
    match (negative.as_slice(), positive.as_slice()) {
        ([a], [b, c, d]) | ([b, c, d], [a]) => mesh.face(
            [point(*a, *b), point(*a, *c), point(*a, *d)],
            Some(direction),
        ),
        ([a, b], [c, d]) => {
            let p = point(*a, *c);
            let q = point(*a, *d);
            let r = point(*b, *c);
            let s = point(*b, *d);
            let first = mesh.face([p, q, s], Some(direction));
            let second = mesh.face([p, s, r], Some(direction));
            first && second
        }
        _ => false,
    }
}

fn subtract(a: Vec3, b: Vec3) -> Vec3 {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: Vec3, b: Vec3) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}
