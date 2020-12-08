use lucent::{
    eval::{Angle, Eval, Scope},
    lang::parse,
    program::Program,
    value::{Dimension, Real, Value},
};

fn evaluate(input: &str) -> Value {
    let definitions = vec![
        "offset := 3".into(),
        "f(x) := x^2 + offset".into(),
        "g(offset) := f(offset)".into(),
    ];
    let program = Program::compile(&definitions, std::iter::empty::<&str>()).unwrap();
    Eval::new(&program, &Scope::new(), 128, Angle::Radians)
        .run(&parse(input).unwrap().expression().unwrap())
        .unwrap()
}

#[test]
fn exact_values_flow_through_definitions_and_units() {
    let quantity = evaluate("g(2) + sqrt(4/9) + if(1/3 = 0.333, 1/0, 3/7)")
        .quantity()
        .unwrap();
    let Real::Exact(number) = quantity.real else {
        panic!()
    };
    assert_eq!(number.to_string(), "170/21");
    let quantity = evaluate("2m + 38cm").quantity().unwrap();
    assert_eq!(quantity.dimension, Dimension::LENGTH);
    assert_eq!(
        quantity.real.enclose(128).unwrap().bounds().unwrap()[0],
        2.38
    );
}

#[test]
fn exact_linear_and_complex_arithmetic() {
    let solution = evaluate("solve([[2, 1], [1, 1]], [8, 5])").list().unwrap();
    let real = |value: Value| {
        value
            .quantity()
            .unwrap()
            .real
            .enclose(128)
            .unwrap()
            .bounds()
            .unwrap()[0]
    };
    assert_eq!(
        solution.into_iter().map(real).collect::<Vec<_>>(),
        [3.0, 2.0]
    );
    let (re, im) = evaluate("(1+i)^3").complex().unwrap();
    assert_eq!(re.enclose(128).unwrap().bounds().unwrap(), [-2.0, -2.0]);
    assert_eq!(im.enclose(128).unwrap().bounds().unwrap(), [2.0, 2.0]);
}

#[test]
fn zero_exclusions_and_dimensions_survive() {
    let program = Program::default();
    let scope = Scope::new();
    for expr in [
        "0*(1/0)",
        "2m + 3s",
        "sin(3kg)",
        "if(interval(-1,1) > 0, 1, 2)",
    ] {
        assert!(
            Eval::new(&program, &scope, 96, Angle::Radians)
                .run(&parse(expr).unwrap().expression().unwrap())
                .is_err()
        );
    }
}
