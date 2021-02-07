use lucent::{exact::ratio, floating, statistics};

#[test]
fn pivoted_qr_and_jacobi_svd_match_simple_linear_models() {
    let a = [vec![1.0, 1.0], vec![0.0, 1.0], vec![1.0, 0.0]];
    let qr = floating::qr(&a, Some(&[5.0, 3.0, 2.0])).unwrap();
    let x = qr.solution.unwrap();
    assert!((x[0] - 2.0).abs() < 1e-12 && (x[1] - 3.0).abs() < 1e-12);
    assert!(qr.residual_norm.unwrap() < 1e-12);
    let svd = floating::svd(&a).unwrap();
    assert_eq!(svd.rank, 2);
    assert!(svd.converged);
    assert!(svd.relative_residual < 1e-12);
    assert!((svd.singular_values[0] - 3.0_f64.sqrt()).abs() < 1e-12);
    assert!((svd.singular_values[1] - 1.0).abs() < 1e-12);
    let deficient = floating::svd(&[vec![2.0, 4.0], vec![1.0, 2.0]]).unwrap();
    assert_eq!(deficient.rank, 1);
    assert!(deficient.condition.is_none());
}

#[test]
fn regression_keeps_residuals_and_uncertainty_visible() {
    let points = [[0.0, 0.9], [1.0, 3.2], [2.0, 4.8], [3.0, 7.2], [4.0, 8.9]];
    let fit = floating::regress(&points, 1).unwrap();
    let weights = fit.coefficients.unwrap();
    assert!((weights[0] - 5.0).abs() < 1e-12 && (weights[1] - 4.0).abs() < 1e-12);
    assert_eq!(fit.degrees_of_freedom, 3);
    assert!(fit.residual_norm.unwrap() > 0.0);
    assert!(fit.coefficient_std_error.unwrap().iter().all(|s| *s > 0.0));
    assert!((fit.points.iter().map(|p| p.residual).sum::<f64>()).abs() < 1e-12);
}

#[test]
fn quantile_uses_the_small_tail_and_reports_endpoint_limits() {
    let p = ratio("1/1000000000000").unwrap();
    let q = statistics::normal_quantile(&p, 96, 384, 1e-13, 1024).unwrap();
    let [a, b] = q.enclosure.unwrap();
    assert!(a <= -7.034483825301131 && -7.034483825301131 <= b);
    assert!(q.target_met);
    let near_one =
        statistics::normal_quantile(&(ratio("1").unwrap() - p), 96, 384, 1e-13, 1024).unwrap();
    assert_eq!(near_one.enclosure.unwrap(), [-b, -a]);
    let endpoint = statistics::normal_quantile(&ratio("1").unwrap(), 96, 384, 1e-13, 1024).unwrap();
    assert_eq!(endpoint.limit, Some("+infinity"));
}
