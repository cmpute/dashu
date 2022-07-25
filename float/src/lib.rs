/// - Rounding is ensured in type level
/// - Precision is stored inside the numbers
/// - The higher precision will be used if two oprands have different precision
/// - Conversion from f32 and f64 is only implemented for BinaryRepr
/// - Conversion from and to str is limited to native radix. To print or parse with different
///   radix, use FloatRepr::with_radix() to convert. (printing with certain radices is permitted,
///   but need to specify explicitly, to print decimal numbers, one can use scientific representation
///   or use the alternate flag)
// TODO: reference crates: twofloat, num-bigfloat, rust_decimal, bigdecimal
mod add;
mod convert;
mod div;
mod fmt;
mod ibig_ext;
mod mul;
mod macros;
mod parse;
mod repr;
mod round;
mod sign;
mod utils;

pub use repr::{BinaryRepr, DecimalRepr, FloatRepr};
pub use round::{Rounding, RoundingMode};

/// Multi-precision float number with binary exponent and [RoundingMode::Zero] rounding mode
#[allow(non_upper_case_globals)]
pub type FBig = BinaryRepr<{ RoundingMode::Zero }>;
/// Multi-precision decimal number with decimal exponent and [RoundingMode::HalfEven] rounding mode
#[allow(non_upper_case_globals)]
pub type DBig = DecimalRepr<{ RoundingMode::HalfEven }>;

// TODO: make no_std
// TODO: add macro fbig!, dbig!, support parsing scientific repr, and set rounding mode
//       ref: scientific-macro crate, https://www.exploringbinary.com/hexadecimal-floating-point-constants/,
