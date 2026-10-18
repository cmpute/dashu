use dashu_float::DBig;

fn main() {
    let (a, b, c) = (DBig::from(2), DBig::from(3), DBig::from(4));
    // The semantics-changing FMA advice is opt-in, so this file has no warnings.
    let _ = a * b + c;
    // Macro-generated patterns are excluded as well.
    macro_rules! square {
        ($value:ident) => {
            &$value * &$value
        };
    }
    let n = dashu_int::UBig::from(12u8);
    let _ = square!(n);
}
