//! Tests of the NUMERIC wire encoding used by the `postgres-types` integration. Unlike
//! `postgres.rs` these need no database server: they check the encoded bytes directly.

use core::str::FromStr;
use dashu_float::{DBig, Repr};
use postgres_types_v02::{private::BytesMut, FromSql, ToSql, Type};

/// A decoded NUMERIC header and its base-10000 digits.
#[derive(Debug, PartialEq)]
struct Wire {
    weight: i16,
    sign: u16,
    dscale: u16,
    digits: Vec<u16>,
}

fn parse_wire(raw: &[u8]) -> Wire {
    let words: Vec<u16> = raw
        .chunks(2)
        .map(|b| u16::from_be_bytes([b[0], b[1]]))
        .collect();
    assert_eq!(words[0] as usize, words.len() - 4, "digit count in the header");
    Wire {
        weight: words[1] as i16,
        sign: words[2],
        dscale: words[3],
        digits: words[4..].to_vec(),
    }
}

/// Build the wire bytes from `[ndigits, weight, sign, dscale, digits...]`.
fn wire_bytes(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_be_bytes()).collect()
}

fn to_sql<T: ToSql>(x: &T) -> Result<Vec<u8>, String> {
    let mut buf = BytesMut::new();
    x.to_sql(&Type::NUMERIC, &mut buf)
        .map(|_| buf.to_vec())
        .map_err(|e| e.to_string())
}

fn dbig_from_sql(raw: &[u8]) -> Result<DBig, String> {
    DBig::from_sql(&Type::NUMERIC, raw).map_err(|e| e.to_string())
}

fn repr_from_sql(raw: &[u8]) -> Result<Repr<10>, String> {
    <Repr<10> as FromSql>::from_sql(&Type::NUMERIC, raw).map_err(|e| e.to_string())
}

/// The value the server stores for these bytes: it truncates to `dscale` fractional digits.
fn stored_value(raw: &[u8]) -> DBig {
    let wire = parse_wire(raw);
    let value = dbig_from_sql(raw).unwrap();
    assert!(
        value.repr().exponent() + wire.dscale as isize >= 0,
        "{value} has digits beyond dscale {}",
        wire.dscale
    );
    value
}

const ZERO: [u8; 8] = [0; 8];

#[test]
fn test_negative_zero() {
    // NUMERIC has no negative zero: both zeros are stored as zero
    let neg_zeros = [
        DBig::from_str("-0").unwrap(),
        DBig::from_str("-0.0").unwrap(),
        DBig::from_str("-0").unwrap().with_precision(5).value(),
        DBig::from_str("-1").unwrap() * DBig::ZERO,
        -DBig::ZERO,
    ];
    for x in &neg_zeros {
        assert!(x.repr().is_neg_zero());
        let raw = to_sql(x).unwrap();
        assert_eq!(raw, ZERO, "{x:?}");
        assert!(dbig_from_sql(&raw).unwrap().repr().is_pos_zero());
    }
    for x in [
        DBig::ZERO,
        DBig::from_str("0.00").unwrap(),
        DBig::from_str("1.5").unwrap() - DBig::from_str("1.5").unwrap(),
        DBig::from_str("-1.5").unwrap() + DBig::from_str("1.5").unwrap(),
    ] {
        assert_eq!(to_sql(&x).unwrap(), ZERO, "{x:?}");
    }

    assert_eq!(to_sql(&Repr::<10>::neg_zero()).unwrap(), ZERO);
    assert_eq!(to_sql(&Repr::<10>::zero()).unwrap(), ZERO);
    assert!(repr_from_sql(&ZERO).unwrap().is_pos_zero());

    // a zero written with explicit zero digits (what `Repr::neg_zero()` used to encode to),
    // with a negative sign, or with a display scale is a zero as well
    for words in [
        vec![1, 0xffff, 0, 4, 0],
        vec![2, 0, 0, 0, 0, 0],
        vec![0, 0, 0x4000, 0],
        vec![0, 0, 0, 2],
    ] {
        let raw = wire_bytes(&words);
        assert!(repr_from_sql(&raw).unwrap().is_pos_zero(), "{words:?}");
        assert!(dbig_from_sql(&raw).unwrap().repr().is_pos_zero(), "{words:?}");
    }
}

#[test]
fn test_dscale_covers_fractional_digits() {
    let d = |s: &str| DBig::from_str(s).unwrap();

    // an inexact difference carries a guard digit beyond its precision; the fraction must
    // still be covered by dscale, or the server would truncate it away
    let x = d("0.1") - d("11");
    assert_eq!((x.precision(), x.repr().digits()), (2, 3));
    let raw = to_sql(&x).unwrap();
    assert_eq!(
        parse_wire(&raw),
        Wire {
            weight: 0,
            sign: 0x4000,
            dscale: 1,
            digits: vec![10, 9000]
        }
    );
    assert_eq!(stored_value(&raw), d("-10.9"));

    let x = d("11") - d("0.1");
    let raw = to_sql(&x).unwrap();
    assert_eq!(
        parse_wire(&raw),
        Wire {
            weight: 0,
            sign: 0,
            dscale: 1,
            digits: vec![10, 9000]
        }
    );
    assert_eq!(stored_value(&raw), d("10.9"));

    // the same value at other precisions: exact digits, then trailing zeros
    assert_eq!(parse_wire(&to_sql(&d("-10.9")).unwrap()).dscale, 1);
    assert_eq!(parse_wire(&to_sql(&d("-10.900")).unwrap()).dscale, 3);
    assert_eq!(parse_wire(&to_sql(&(d("0.1") - d("10"))).unwrap()).dscale, 1);
    assert_eq!(parse_wire(&to_sql(&d("1e4")).unwrap()).dscale, 0);
    assert_eq!(parse_wire(&to_sql(&d("1e-6")).unwrap()).dscale, 6);
    assert_eq!(parse_wire(&to_sql(&d("-00000.000001")).unwrap()).dscale, 16);

    // whatever the precision, no digit of the value lies beyond dscale
    for a in ["0.1", "0.01", "9.99", "11", "123.456", "1e3", "0.00007"] {
        for b in ["11", "0.1", "99.5", "1e-9", "12345.6789", "1e5"] {
            for x in [d(a) - d(b), d(a) + d(b), d(a) * d(b)] {
                let raw = to_sql(&x).unwrap();
                assert_eq!(stored_value(&raw), x, "{a} and {b}");
                for p in [1, 2, 5, 20, 50, 100, 500] {
                    let y = x.clone().with_precision(p).value();
                    let raw = to_sql(&y).unwrap();
                    assert_eq!(stored_value(&raw), y, "{a} and {b} at precision {p}");
                    let back = dbig_from_sql(&raw).unwrap();
                    assert_eq!(back, y);
                    assert!(back.precision() >= y.repr().digits());
                }
            }
        }
    }
}

#[test]
fn test_from_sql_malformed() {
    // [ndigits, weight, sign, dscale, digits...]
    assert_eq!(dbig_from_sql(&wire_bytes(&[0, 0, 0x0000, 0])).unwrap(), DBig::ZERO);
    assert_eq!(dbig_from_sql(&wire_bytes(&[0, 0, 0xD000, 0])).unwrap(), DBig::INFINITY);
    assert_eq!(dbig_from_sql(&wire_bytes(&[0, 0, 0xF000, 0])).unwrap(), DBig::NEG_INFINITY);

    let malformed: [&[u16]; 8] = [
        &[0, 0, 0xC000, 0],       // NaN
        &[0, 0, 0x0001, 0],       // unknown sign words
        &[1, 0, 0x8000, 0, 1],    //
        &[0, 0, 0xFFFF, 0],       //
        &[2, 0, 0, 0, 1],         // fewer digits than announced
        &[1, 0, 0, 0, 1, 2],      // more digits than announced
        &[1, 0, 0, 0, 10000],     // digit out of range
        &[2, 0, 0, 0, 1, 0xFFFF], //
    ];
    for words in malformed {
        let raw = wire_bytes(words);
        assert!(dbig_from_sql(&raw).is_err(), "{words:?}");
        assert!(repr_from_sql(&raw).is_err(), "{words:?}");
    }
    // truncated header, and a dangling half digit
    for len in 0..8 {
        assert!(dbig_from_sql(&ZERO[..len]).is_err());
    }
    let mut raw = wire_bytes(&[1, 0, 0, 0, 1]);
    raw.pop();
    assert!(dbig_from_sql(&raw).is_err());

    // leading zero digits and a dscale short of the digits are unusual but decodable
    let x = dbig_from_sql(&wire_bytes(&[2, 1, 0, 0, 0, 15])).unwrap();
    assert_eq!(x, DBig::from_str("15").unwrap());
    assert_eq!(x.precision(), 2);
    let x = dbig_from_sql(&wire_bytes(&[1, 0xffff, 0x4000, 0, 1500])).unwrap();
    assert_eq!(x, DBig::from_str("-0.15").unwrap());
    assert_eq!(x.precision(), 2);
}

#[test]
fn test_large_weight_and_precision() {
    // 1e32764, 1e32768, and 1e30764 with 2002 / 2003 decimal places (precision 32767 / 32768)
    let cases: [[u16; 5]; 5] = [
        [1, 8191, 0, 0, 1],
        [1, 8192, 0, 0, 1],
        [1, 7691, 0, 2002, 1],
        [1, 7691, 0, 2003, 1],
        [1, 32767, 0x4000, 16383, 9999],
    ];
    for words in cases {
        let raw = wire_bytes(&words);
        let x = dbig_from_sql(&raw).unwrap();
        assert_eq!(to_sql(&x).unwrap(), raw, "{words:?}");
    }

    let d = |s: &str| DBig::from_str(s).unwrap();
    assert_eq!(to_sql(&d("1e32768")).unwrap(), wire_bytes(&[1, 8192, 0, 0, 1]));
    assert_eq!(to_sql(&d("1e131071")).unwrap(), wire_bytes(&[1, 32767, 0, 0, 1000]));
    assert_eq!(
        to_sql(&d("1e-16383")).unwrap(),
        wire_bytes(&[1, (-4096i16) as u16, 0, 16383, 10])
    );
    assert_eq!(
        to_sql(&d("5").with_precision(40000).value()).unwrap(),
        wire_bytes(&[1, 0, 0, 39999, 5])
    );
    // the padding implied by the precision stops at the largest scale of the format
    assert_eq!(
        to_sql(&d("5").with_precision(100_000).value()).unwrap(),
        wire_bytes(&[1, 0, 0, 65535, 5])
    );

    // beyond what the format can hold
    for s in [
        "1e131072",
        "1e-65536",
        "1e1000000",
        "1e-1000000",
        "1e9223372036854775806",
        "1e-9223372036854775807",
    ] {
        assert!(to_sql(&d(s)).is_err(), "{s}");
        assert!(to_sql(d(s).repr()).is_err(), "{s}");
    }
}
