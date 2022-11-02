//! Error types.

use core::fmt::{self, Display, Formatter};

/// Number out of bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutOfBoundsError;

impl Display for OutOfBoundsError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("number out of bounds")
    }
}

// TODO(v0.3): Create a new error type called ConversionError, which includes OutOfBounds and PrecisionLoss

#[cfg(feature = "std")]
impl std::error::Error for OutOfBoundsError {}

/// Error parsing a number.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseError {
    /// No digits in the string.
    NoDigits,
    /// Invalid digit for a given radix.
    InvalidDigit,
    /// The radix is not supported.
    UnsupportedRadix,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ParseError::NoDigits => f.write_str("no digits"),
            ParseError::InvalidDigit => f.write_str("invalid digit"),
            ParseError::UnsupportedRadix => f.write_str("unsupported radix"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseError {}

/// Panics when division by 0 is happening
pub(crate) const fn panic_divide_by_0() -> ! {
    panic!("divisor must not be 0")
}

/// Panics when the range input for the random generator in empty
#[cfg(feature = "rand")]
pub(crate) const fn panic_empty_range() -> ! {
    panic!("empty range for random generation")
}

/// Panics when try to allocate memory with size exceeding usize range
pub(crate) const fn panic_allocate_too_much() -> ! {
    panic!("try to allocate too much memory")
}

/// Panics when allocation failed
pub(crate) const fn panic_out_of_memory() -> ! {
    panic!("out of memory")
}

/// Panics when the `UBig` result is negative
pub(crate) const fn panic_negative_ubig() -> ! {
    panic!("UBig result must not be negative")
}

/// Panics when trying to do operations on `Modulo` values from different rings.
pub(crate) const fn panic_different_rings() -> ! {
    panic!("Modulo values from different rings")
}

/// Panics when the radix is not supported
pub(crate) fn panic_invalid_radix(radix: u32) -> ! {
    panic!("invalid radix: {}, only radix 2-36 are supported", radix);
}

/// Panics when the base is 0 or 1 in logarithm
pub(crate) fn panic_invalid_log_oprand() -> ! {
    panic!("logarithm is not defined for 0, base 0 and base 1!");
}

/// Panics when taking the zeroth root of an integer
pub(crate) fn panic_root_zeroth() -> ! {
    panic!("finding 0th root is not allowed!")
}

/// Panics when taking an even order root of an negative integer
pub(crate) fn panic_root_negative() -> ! {
    panic!("the root is a complex number!")
}
