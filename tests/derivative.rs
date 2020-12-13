use lucent::{
    differentiate::derivative,
    eval::{Angle, Eval, Scope},
    lang::parse,
    program::Program,
    value::{Real, Value},
};

#[test]
fn derivative_uses_supported_chain_and_power_rules() {
    let expr = parse("exp(x^2) + 3/x").unwrap().expression().unwrap();
    let symbolic = derivative(&expr, "x", Angle::Radians).unwrap();
    let scope = Scope::from([("x".into(), Value::real(Real::integer(1)))]);
    let program = Program::default();
    let [lo, hi] = Eval::new(&program, &scope, 128, Angle::Radians)
        .real(&symbolic)
        .unwrap()
        .enclose(128)
        .unwrap()
        .bounds()
        .unwrap();
    let reference = 2.4365636569180905_f64;
    assert!(lo <= reference && reference <= hi);
    assert!(hi - lo < 1e-14);
}
