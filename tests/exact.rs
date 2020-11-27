use lucent::exact::{decimal, power, ratio};
use lucent::interval::Issue;

#[test]
fn decimal_provenance_is_exact() {
    assert_eq!(decimal("1.250e-2").unwrap().to_string(), "1/80");
    assert_eq!(decimal(".0002").unwrap().to_string(), "1/5000");
    assert_eq!(
        decimal("1e100").unwrap().to_string(),
        format!("1{}", "0".repeat(100))
    );
    assert_eq!(
        power(&ratio("2/3").unwrap(), -4).unwrap().to_string(),
        "81/16"
    );
    assert_eq!(decimal("1e1000000"), Err(Issue::Budget));
    assert!(ratio("0/0").is_err());
}
