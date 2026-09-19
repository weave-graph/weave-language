use weave_language::decimal::{Decimal, DecimalError::*};
fn d(text: &str) -> Decimal {
    text.parse().unwrap()
}

#[test]
fn decimal_canonical_encoding_preserves_values_beyond_binary64_integer_precision() {
    for (input, expected) in [
        ("+0012.3400e-1", "1.234"),
        ("-0.000e+128", "0"),
        ("100e-2", "1"),
        ("1e-18", "0.000000000000000001"),
        ("9007199254740993", "9007199254740993"),
        ("-999999999999999999", "-999999999999999999"),
    ] {
        let value = d(input);
        assert_eq!(value.to_string(), expected);
        let wire = serde_json::to_string(&value).unwrap();
        assert_eq!(wire, format!("\"{expected}\""));
        assert_eq!(serde_json::from_str::<Decimal>(&wire).unwrap(), value);
    }
    for wire in ["1.2", "\"1.20\"", "\"1e0\"", "\"-0\"", "\"+1\""] {
        assert!(serde_json::from_str::<Decimal>(wire).is_err());
    }
}
#[test]
fn decimal_parser_rejects_malformed_unbounded_and_unrepresentable_values() {
    for input in [
        "", " ", "1 ", ".1", "1.", "--1", "1e", "1e-", "NaN", "Infinity", "1_000", "１２", "1.2.3",
    ] {
        assert_eq!(input.parse::<Decimal>(), Err(Syntax), "{input}");
    }
    assert_eq!("1e129".parse::<Decimal>(), Err(InputLimit));
    assert_eq!("0".repeat(129).parse::<Decimal>(), Err(InputLimit));
    assert_eq!("1e-19".parse::<Decimal>(), Err(Scale));
    assert_eq!("1000000000000000000".parse::<Decimal>(), Err(Precision));
    assert_eq!("0.1234567890123456789".parse::<Decimal>(), Err(Precision));
}
#[test]
fn exact_arithmetic_never_implicitly_rounds() {
    assert_eq!(d("0.1").checked_add(d("0.2")), Ok(d("0.3")));
    assert_eq!(
        d("0.999999999999999999").checked_add(d("0.000000000000000001")),
        Ok(d("1"))
    );
    assert_eq!(
        d("999999999999999999").checked_sub(d("999999999999999999")),
        Ok(Decimal::ZERO)
    );
    assert_eq!(d("1.25").checked_mul(d("0.8")), Ok(d("1")));
    assert_eq!(
        d("999999999999999999").checked_mul(d("0.1")),
        Ok(d("99999999999999999.9"))
    );
    assert_eq!(
        d("999999999999999999").checked_add(d("0.1")),
        Err(Precision)
    );
    assert_eq!(d("999999999999999999").checked_mul(d("2")), Err(Precision));
    assert_eq!(d("0.000000000000000001").checked_mul(d("0.1")), Err(Scale));
}
#[test]
fn exact_division_reduces_before_testing_decimal_termination() {
    for (a, b, result) in [
        ("1", "8", "0.125"),
        ("3", "6", "0.5"),
        ("-10", "0.1", "-100"),
        ("-1", "-2", "0.5"),
        ("0.000000000000000001", "0.000000000000000001", "1"),
        ("100000000000000000", "100000000000000000", "1"),
        ("0", "3", "0"),
    ] {
        assert_eq!(d(a).checked_div(d(b)), Ok(d(result)));
    }
    assert_eq!(d("1").checked_div(d("3")), Err(NonTerminating));
    assert_eq!(d("0").checked_div(d("0")), Err(DivisionByZero));
    assert_eq!(d("1e-18").checked_div(d("2")), Err(Scale));
    assert_eq!(
        d("999999999999999999").checked_div(d("1e-18")),
        Err(Precision)
    );
}
#[test]
fn bounded_rational_oracle_covers_arithmetic_and_finite_division() {
    // Independent integer oracle over a two-decimal input grid.
    for a in -40_i128..=40 {
        for b in -40_i128..=40 {
            let x = d(&format!("{a}e-2"));
            let y = d(&format!("{b}e-2"));
            assert_eq!(x.checked_add(y), Ok(d(&format!("{}e-2", a + b))));
            assert_eq!(x.checked_sub(y), Ok(d(&format!("{}e-2", a - b))));
            assert_eq!(x.checked_mul(y), Ok(d(&format!("{}e-4", a * b))));
            if b != 0 {
                match x.checked_div(y) {
                    Ok(value) => assert_eq!(value.checked_mul(y), Ok(x)),
                    Err(NonTerminating) => {
                        let mut n = a.abs();
                        let mut q = b.abs();
                        while q != 0 {
                            (n, q) = (q, n % q);
                        }
                        let mut denominator = b.abs() / n;
                        while denominator % 2 == 0 {
                            denominator /= 2;
                        }
                        while denominator % 5 == 0 {
                            denominator /= 5;
                        }
                        assert_ne!(denominator, 1);
                    }
                    other => panic!("Unexpected division {a}/{b}: {other:?}"),
                }
            }
        }
    }
}
