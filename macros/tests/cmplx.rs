use dashu_cmplx::CBig;
use dashu_macros::{cbig, static_cbig};

#[test]
fn test_cbig_decimal_default() {
    // unprefixed coefficients are decimal (3 + 4i), the same value as the
    // explicit hex / binary / octal forms
    assert_eq!(cbig!(3 + 4i), cbig!(0x3 + 0x4i));
    assert_eq!(cbig!(3 + 4i), cbig!(0b11 + 0b100i));
    assert_eq!(cbig!(3 + 4i), cbig!(0o3 + 0o4i));

    // values are correct (CBig<Zero, 2> displays coefficients in binary)
    assert_eq!(cbig!(3 + 4i).re().to_string(), "11"); // 3
    assert_eq!(cbig!(3 + 4i).im().to_string(), "100"); // 4
    assert_eq!(cbig!(3 + 4i).to_string(), "11+100i");

    // pure real / pure imaginary / signs
    assert_eq!(cbig!(3).re().to_string(), "11");
    assert_eq!(cbig!(4i).im().to_string(), "100");
    assert_eq!(cbig!(i).im().to_string(), "1");
    assert_eq!(cbig!(-i).im().to_string(), "-1");
    assert_eq!(cbig!(-3 - 4i).to_string(), "-11-100i");
    assert_eq!(cbig!(1 + i).to_string(), "1+i");
    assert_eq!(cbig!(1 - i).to_string(), "1-i");

    // pair form
    assert_eq!(cbig!(3, 4), cbig!(3 + 4i));
    assert_eq!(cbig!(-3, -4), cbig!(-3 - 4i));

    // decimal precision: derived from the digits, converted to bits
    assert_eq!(cbig!(3 + 4i).precision(), 3);
}

#[test]
fn test_cbig_hex() {
    // 0x prefix selects hexadecimal
    assert_eq!(cbig!(0x3 + 0x4i), cbig!(3 + 4i));
    // hex float coefficients with the C++ `p` exponent
    assert_eq!(cbig!(0x1p4 + 0x1p3i).to_string(), "10000+1000i"); // 16 + 8i

    // one hex digit carries 4 bits of precision
    assert_eq!(cbig!(0x3 + 0x4i).precision(), 4);
}

#[test]
fn test_cbig_binary_octal() {
    // 0b / 0o prefixes select binary / octal
    assert_eq!(cbig!(0b11 + 0b100i), cbig!(3 + 4i));
    assert_eq!(cbig!(0o17).re().to_string(), "1111"); // 15
    assert_eq!(cbig!(0o17 + 0o20i), cbig!(15 + 16i));

    // binary float coefficients (the `_` escape dodges the tokenizer, as in `fbig!`)
    assert_eq!(cbig!(_0b11.1).re().to_string(), "11.1"); // 3.5
}

#[test]
fn test_cbig_fractional_decimal() {
    // exactly-representable decimals are exact in binary
    assert_eq!(cbig!(0.5).re().to_string(), "0.1"); // 0.5 = 0.1₂
    assert_eq!(cbig!(0.25).re().to_string(), "0.01"); // 0.25 = 0.01₂
    assert_eq!(cbig!(3.5 + 2.25i), cbig!(_0b11.1 + _0b10.01i));
}

#[test]
fn test_cbig_const() {
    const Z: CBig = cbig!(3 + 4i);
    assert_eq!(Z, cbig!(3 + 4i));
    const ZH: CBig = cbig!(0x3 + 0x4i);
    assert_eq!(ZH, Z);
    const P: CBig = cbig!(3, 4);
    assert_eq!(P, Z);
}

#[rustversion::since(1.64)]
#[rustversion::attr(since(1.64), test)]
fn test_static_cbig() {
    let z: &'static CBig = static_cbig!(3 + 4i);
    assert_eq!(*z, cbig!(3 + 4i));

    // large coefficients that `cbig!` can't lift into a `const`
    let big: &'static CBig = static_cbig!(
        0xffffffffffffffffffffffffffffffffffffffffffffffff,
        0xffffffffffffffffffffffffffffffffffffffffffffffff
    );
    assert_eq!(
        *big,
        cbig!(
            0xffffffffffffffffffffffffffffffffffffffffffffffff,
            0xffffffffffffffffffffffffffffffffffffffffffffffff
        )
    );
}
