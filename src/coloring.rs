use serde::Serialize;

use crate::{
    eval::Eval,
    graph::{Context, View, middle},
    interval::{Interval, Result},
    lang::Expr,
    value::{Real, Value},
};

#[derive(Serialize)]
pub struct Raster {
    pub size: [usize; 2],
    pub rgba: Vec<u8>,
    pub undefined_samples: usize,
    pub view: View,
}

pub fn sample(ctx: &Context, expr: &Expr, view: View) -> Result<Raster> {
    let view = view.validate()?;
    let bits = ctx.control.bits;
    let work = ctx.control.budget.clamp(16, 24_576);
    let aspect = (f64::from(view.pixels[0]) / f64::from(view.pixels[1])).clamp(0.25, 4.0);
    let width = ((work as f64 * aspect).sqrt() as usize).max(1);
    let height = (work / width).max(1);
    let mut output = Raster {
        size: [width, height],
        rgba: Vec::with_capacity(width * height * 4),
        undefined_samples: 0,
        view,
    };
    let mut scope = ctx.scope.clone();
    for j in 0..height {
        let t = (j as f64 + 0.5) / height as f64;
        let y = view.y[1] - t * (view.y[1] - view.y[0]);
        for i in 0..width {
            let t = (i as f64 + 0.5) / width as f64;
            let x = view.x[0] + t * (view.x[1] - view.x[0]);
            let input = Value::Complex(
                Real::Bounded(Interval::point(x, bits)?),
                Real::Bounded(Interval::point(y, bits)?),
            );
            scope.insert("z".into(), input);
            let value = Eval::new(ctx.program, &scope, bits, ctx.control.angle)
                .run(expr)
                .ok()
                .and_then(|v| v.complex().ok());
            let [r, g, b, a] = value
                .and_then(|(real, imag)| {
                    let re = middle(real.enclose(bits).ok()?.bounds().ok()?);
                    let im = middle(imag.enclose(bits).ok()?.bounds().ok()?);
                    pigment(re, im)
                })
                .unwrap_or_else(|| {
                    output.undefined_samples += 1;
                    [43, 37, 32, 255]
                });
            output.rgba.extend([r, g, b, a]);
        }
    }
    Ok(output)
}

fn pigment(re: f64, im: f64) -> Option<[u8; 4]> {
    let radius = re.hypot(im);
    if !radius.is_finite() {
        return None;
    }
    if radius == 0.0 {
        return Some([18, 28, 30, 255]);
    }
    let phase = (im.atan2(re) / std::f64::consts::TAU).rem_euclid(1.0) * 6.0;
    let ring = (radius.log2().rem_euclid(1.0) - 0.5).abs();
    let luminance = 0.57 + 0.24 * (2.0 * ring) + 0.10 * (radius / (1.0 + radius));
    let chroma = luminance * 0.72;
    let x = chroma * (1.0 - (phase.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match phase as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let channel = |v| ((luminance - chroma + v) * 255.0) as u8;
    Some([channel(r), channel(g), channel(b), 255])
}
