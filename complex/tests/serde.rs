use dashu_cmplx::CBig;
use dashu_float::round::mode::HalfEven;
use postcard::{from_bytes, to_allocvec};
use serde_json::{from_str, to_string};

type C = CBig<HalfEven, 10>;

/// Build a `CBig` with both parts at precision `prec`.
fn c(re: &str, im: &str, prec: usize) -> C {
    let re = re.parse::<dashu_float::FBig<HalfEven, 10>>().unwrap();
    let im = im.parse::<dashu_float::FBig<HalfEven, 10>>().unwrap();
    CBig::from_parts(re.with_precision(prec).value(), im.with_precision(prec).value())
}

#[test]
fn test_cbig_serde() {
    // Normalized values: each part's digit count ≤ the context precision, so the padded string
    // recovers the precision from the digit count.
    let test_numbers = [
        c("0", "0", 10),
        c("1", "2", 10),
        c("-3.5", "4.25", 10),
        c("123456789.0123456789", "-0.0001", 20),
        c("3.14159", "-2.71828", 30),
    ];
    for z in &test_numbers {
        // binary (non-human-readable) round-trip
        let output = to_allocvec(z).unwrap();
        let parsed: C = from_bytes(&output).unwrap();
        assert_eq!(&parsed, z);
        assert_eq!(parsed.precision(), z.precision(), "binary precision round-trip");

        // string (human-readable) round-trip: a single `a+bi` string whose coefficients are
        // padded to the precision, so `CBig::from_str` recovers it from the digit count.
        let output = to_string(z).unwrap();
        assert!(output.starts_with('"'), "human-readable must be a string, got {output}");
        let parsed: C = from_str(&output).unwrap();
        assert_eq!(&parsed, z);
        assert_eq!(parsed.precision(), z.precision(), "string precision round-trip");
    }
}

#[test]
fn cbig_serde_over_precision_value() {
    // A part whose digit count exceeds the context precision cannot recover the precision from
    // the string (the digit count becomes the precision) — the same limitation as FBig's serde.
    // The value itself still round-trips exactly.
    let z = c("1e100", "-1e-100", 30);
    let output = to_string(&z).unwrap();
    let parsed: C = from_str(&output).unwrap();
    assert_eq!(parsed, z);
}
