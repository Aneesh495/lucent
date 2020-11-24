use lucent::{
    interval::{Interval, Issue},
    mp::Unary,
};

fn interval(a: f64, b: f64) -> Interval {
    Interval::new(a, b, 96).unwrap()
}

#[test]
fn mixed_sign_arithmetic() {
    assert_eq!(
        interval(-3.0, 1.0)
            .mul(&interval(-2.0, 4.0))
            .unwrap()
            .bounds()
            .unwrap(),
        [-12.0, 6.0]
    );
    assert_eq!(
        interval(-3.0, 1.0).pow(2).unwrap().bounds().unwrap(),
        [0.0, 9.0]
    );
    assert_eq!(
        interval(-3.0, -2.0).pow(-1).unwrap().bounds().unwrap()[0],
        -0.5
    );
    assert!(matches!(
        interval(0.0, 1.0).div(&interval(-0.5, 0.25)),
        Err(Issue::Split(_))
    ));
    assert!(matches!(
        interval(-2.0, -1.0).unary(Unary::Sqrt),
        Err(Issue::Excluded(_))
    ));
}

#[test]
fn periodic_critical_points() {
    assert_eq!(
        interval(1.0, 2.0)
            .unary(Unary::Sin)
            .unwrap()
            .bounds()
            .unwrap()[1],
        1.0
    );
    assert_eq!(
        interval(-2.0, -1.0)
            .unary(Unary::Sin)
            .unwrap()
            .bounds()
            .unwrap()[0],
        -1.0
    );
    assert_eq!(
        interval(3.0, 3.3)
            .unary(Unary::Cos)
            .unwrap()
            .bounds()
            .unwrap()[0],
        -1.0
    );
    assert_eq!(
        interval(-0.1, 0.1)
            .unary(Unary::Cos)
            .unwrap()
            .bounds()
            .unwrap()[1],
        1.0
    );
    assert!(matches!(
        interval(-2.0, -1.0).unary(Unary::Tan),
        Err(Issue::Split(_))
    ));
    assert!(interval(2.0, 3.0).unary(Unary::Tan).is_ok());
}
