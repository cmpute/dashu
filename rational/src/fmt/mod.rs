//! Formatting support for [`RBig`] and [`Relaxed`]: the standard `Display`/`Debug` traits
//! plus the radix/positional helpers ([`InRadix`] and [`InExpanded`]).
//!
//! [`InExpanded`]: expanded::InExpanded

pub mod expanded;

use crate::{
    rbig::{RBig, Relaxed},
    repr::Repr,
};
use alloc::string::ToString;
use core::fmt::{self, Write};
use dashu_base::Sign;

/// Write a fraction whose numerator and denominator have been rendered into `body`
/// (including the `-` of a negative numerator) as a single item: the `+` flag adds one sign
/// in front of the fraction, and the width, fill and alignment apply to the whole of it.
fn fmt_fraction(
    f: &mut fmt::Formatter<'_>,
    is_negative: bool,
    body: fmt::Arguments<'_>,
) -> fmt::Result {
    if f.width().is_none() {
        // no padding: write the parts directly, without an intermediate buffer
        if f.sign_plus() && !is_negative {
            f.write_char('+')?;
        }
        f.write_fmt(body)
    } else {
        let body = body.to_string();
        let magnitude = body.strip_prefix('-').unwrap_or(&body);
        f.pad_integral(!is_negative, "", magnitude)
    }
}

// Generate Display, Binary, Octal, LowerHex, UpperHex impls for Repr, RBig, and Relaxed.
// Pattern: if denominator is 1, print only the numerator; otherwise print
// "numerator/denominator" with each component in the target radix. The alternate flag
// (radix prefix) applies to both components, every other flag to the fraction as a whole.
macro_rules! impl_radix_fmt {
    ($trait:ident, $plain:literal, $alternate:literal) => {
        impl fmt::$trait for Repr {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if self.denominator.is_one() {
                    fmt::$trait::fmt(&self.numerator, f)
                } else {
                    let is_negative = self.numerator.sign() == Sign::Negative;
                    if f.alternate() {
                        fmt_fraction(
                            f,
                            is_negative,
                            format_args!($alternate, self.numerator, self.denominator),
                        )
                    } else {
                        fmt_fraction(
                            f,
                            is_negative,
                            format_args!($plain, self.numerator, self.denominator),
                        )
                    }
                }
            }
        }

        impl fmt::$trait for RBig {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::$trait::fmt(&self.0, f)
            }
        }

        impl fmt::$trait for Relaxed {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::$trait::fmt(&self.0, f)
            }
        }
    };
}

impl_radix_fmt!(Display, "{}/{}", "{:#}/{:#}");
impl_radix_fmt!(Binary, "{:b}/{:b}", "{:#b}/{:#b}");
impl_radix_fmt!(Octal, "{:o}/{:o}", "{:#o}/{:#o}");
impl_radix_fmt!(LowerHex, "{:x}/{:x}", "{:#x}/{:#x}");
impl_radix_fmt!(UpperHex, "{:X}/{:X}", "{:#X}/{:#X}");

impl fmt::Debug for Repr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Repr")
                .field("numerator", &format_args!("{:#?}", self.numerator))
                .field("denominator", &format_args!("{:#?}", self.denominator))
                .finish()
        } else {
            f.write_fmt(format_args!("{:?} / {:?}", self.numerator, self.denominator))
        }
    }
}

impl fmt::Debug for RBig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("RBig")
                .field("numerator", &format_args!("{:#?}", self.numerator()))
                .field("denominator", &format_args!("{:#?}", self.denominator()))
                .finish()
        } else {
            f.write_fmt(format_args!("{:?} / {:?}", self.numerator(), self.denominator()))
        }
    }
}

impl fmt::Debug for Relaxed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Relaxed")
                .field("numerator", &format_args!("{:#?}", self.numerator()))
                .field("denominator", &format_args!("{:#?}", self.denominator()))
                .finish()
        } else {
            f.write_fmt(format_args!("{:?} / {:?}", self.numerator(), self.denominator()))
        }
    }
}

// ---- InRadix: arbitrary radix formatting for rationals ----

/// Representation of a rational number in a given radix, returned by
/// [`RBig::in_radix`] and [`Relaxed::in_radix`].
///
/// Implements [`Display`](core::fmt::Display). The alternate flag (`{:#}`) toggles uppercase
/// letters for radices above 10.
pub struct InRadix<'a> {
    numerator: &'a dashu_int::IBig,
    denominator: &'a dashu_int::UBig,
    radix: u8,
}

impl fmt::Display for InRadix<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let r = self.radix;
        let num = self.numerator.in_radix(r);
        if self.denominator.is_one() {
            fmt::Display::fmt(&num, f)
        } else {
            let den = self.denominator.in_radix(r);
            let is_negative = self.numerator.sign() == Sign::Negative;
            if f.alternate() {
                fmt_fraction(f, is_negative, format_args!("{:#}/{:#}", num, den))
            } else {
                fmt_fraction(f, is_negative, format_args!("{}/{}", num, den))
            }
        }
    }
}

impl RBig {
    /// Representation in a given radix.
    ///
    /// The `radix` parameter is `u8`. Valid radices are 2 through 36.
    ///
    /// # Panics
    /// Panics if `radix` is not between 2 and 36 inclusive.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// assert_eq!(format!("{}", RBig::from_parts(83.into(), 16u8.into()).in_radix(16)), "53/10");
    /// ```
    #[inline]
    pub fn in_radix(&self, radix: u8) -> InRadix<'_> {
        InRadix {
            numerator: self.numerator(),
            denominator: self.denominator(),
            radix,
        }
    }
}

impl Relaxed {
    /// Representation in a given radix.
    ///
    /// The `radix` parameter is `u8`. Valid radices are 2 through 36.
    ///
    /// # Panics
    /// Panics if `radix` is not between 2 and 36 inclusive.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::Relaxed;
    /// assert_eq!(format!("{}", Relaxed::from_parts(83.into(), 16u8.into()).in_radix(16)), "53/10");
    /// ```
    #[inline]
    pub fn in_radix(&self, radix: u8) -> InRadix<'_> {
        InRadix {
            numerator: self.numerator(),
            denominator: self.denominator(),
            radix,
        }
    }
}
