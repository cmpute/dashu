use dashu_int::{IBig, UBig};

// A function of this crate that happens to be called like a conversion is not one.
fn from(n: u64) -> IBig {
    IBig::from(n * 2)
}

fn patterns(value: &IBig, other: &UBig, n: u64, k: u128, neg: i64) {
    let _ = value * IBig::from(n);
    let _ = value + IBig::from(neg);
    let _ = value / IBig::from(neg);
    let _ = other * UBig::from_word(n);
    let _ = other - UBig::from_dword(k);
    // A computed primitive and a negated variable.
    let _ = value * IBig::from(n as usize + 1);
    let _ = value * IBig::from(-neg);
    // A chain of conversions starts at the primitive too.
    let _ = value * IBig::from(UBig::from_dword(k));
    let mut a = value.clone();
    a *= IBig::from(n);
    let mut b = other.clone();
    b /= UBig::from(n);
    b += UBig::from_dword(k);
}

fn negative_cases(value: &IBig, again: &IBig, other: &UBig, n: u64) {
    // A literal, suffixed or not: the operators cannot infer the type of a bare one.
    let _ = value * IBig::from(2);
    let _ = value / IBig::from(-7);
    let _ = other * UBig::from(2u8);
    let _ = other - UBig::from_word(1);
    // `%` and the bit operators return the primitive: the converted operand sets the result type.
    let _ = value % IBig::from(n);
    let _ = other & UBig::from(n);
    let _ = other | UBig::from_word(n);
    // No operator takes a Dashu integer on the right of a primitive.
    let _ = IBig::from(n) * value;
    // A different Dashu integer on the left: the operator mixes the types.
    let _ = other * IBig::from(n);
    // Not a primitive being converted.
    let _ = value * again;
    let _ = value * IBig::from(other.clone());
    // Not a conversion of `dashu-int`.
    let _ = value * from(n);
    // Not an operator.
    let _ = IBig::from(n);
}

fn main() {
    let (value, other) = (IBig::from(3), UBig::from(4u8));
    patterns(&value, &other, 5, 6, -5);
    negative_cases(&value, &value, &other, 5);
}
