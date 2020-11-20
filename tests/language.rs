use lucent::lang::parse;

#[test]
fn precedence_and_formatting() {
    for (text, canonical) in [
        ("-2^2 + 3x", "((-(2 ^ 2)) + (3 * x))"),
        ("2^3^2", "(2 ^ (3 ^ 2))"),
        ("2^-3", "(2 ^ (-3))"),
        ("1/2x", "((1 / 2) * x)"),
        ("2π + 2(x + 1)", "((2 * pi) + (2 * (x + 1)))"),
        ("if(x ≤ 0, [.5, 1e-3], 7)", "if((x <= 0), [.5, 1e-3], 7)"),
    ] {
        let expression = parse(text).unwrap().expression().unwrap();
        assert_eq!(expression.to_string(), canonical);
        let second = parse(canonical).unwrap().expression().unwrap();
        assert_eq!(second.to_string(), canonical);
    }
}

#[test]
fn errors_name_the_range() {
    let diagnostic = parse("π + ?").unwrap_err();
    assert_eq!((diagnostic.span.start, diagnostic.span.end), (5, 6));
    for text in [
        "3.2.1",
        "1e+",
        "x < y < z",
        "f(x, x) := x",
        "[1,]",
        "(x",
        "1 2",
    ] {
        assert!(parse(text).is_err(), "{text}");
    }
}
