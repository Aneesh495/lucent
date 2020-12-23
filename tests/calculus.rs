use lucent::{
    calculus,
    eval::{Angle, Scope},
    graph::{Context, Controls},
    lang::{Expr, parse},
    polynomial,
    program::Program,
};

fn expr(source: &str) -> Expr {
    parse(source).unwrap().expression().unwrap()
}

#[test]
fn unique_roots_and_regular_quadrature() {
    let ctx = Context {
        program: &Program::default(),
        scope: &Scope::new(),
        control: Controls {
            bits: 128,
            angle: Angle::Radians,
            budget: 4096,
            tolerance: 0.45,
        },
    };
    let quadratic = expr("x^2-2");
    let roots = calculus::roots(&ctx, &quadratic, [1.0, 2.0], 1e-12).unwrap();
    assert_eq!(roots.unique.len(), 1);
    let root = &roots.unique[0];
    assert!(root.bracket[0] <= 2.0_f64.sqrt() && 2.0_f64.sqrt() <= root.bracket[1]);
    assert!(root.bracket[1] - root.bracket[0] < 1e-12);
    calculus::verify_root(&ctx, &quadratic, root).unwrap();

    let sine = expr("sin(x)");
    let integral = calculus::integrate(&ctx, &sine, [0.0, 1.0], 1e-12).unwrap();
    assert!(integral.target_met);
    let [a, b] = integral.enclosure.unwrap();
    let exact_rounded = 0.4596976941318603;
    assert!(a <= exact_rounded && exact_rounded <= b);
    calculus::verify_integral(&ctx, &sine, [0.0, 1.0], [a, b], &integral.panels).unwrap();

    let pole = calculus::integrate(&ctx, &expr("1/x"), [-1.0, 1.0], 1e-7).unwrap();
    assert!(pole.enclosure.is_none());
}

#[test]
fn exact_sturm_handles_endpoints_and_multiplicity() {
    let polynomial = expr("(x+1)^4*(x-1/3)^2*(x-2)^3*(x^2+1)");
    let result = polynomial::isolate(&polynomial, ["-1", "2"], "1/1000000000000", 1024).unwrap();
    assert_eq!(result.distinct_count, 3);
    assert_eq!(result.multiplicity_count, 9);
    assert!(result.unresolved.is_empty());
    assert_eq!(
        result
            .roots
            .iter()
            .map(|r| r.multiplicity)
            .collect::<Vec<_>>(),
        [4, 2, 3]
    );
    assert_eq!(result.roots[0].low, "-1");
    assert_eq!(result.roots[2].high, "2");
}
