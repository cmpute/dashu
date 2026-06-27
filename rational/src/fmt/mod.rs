pub mod expanded;

use crate::{
    rbig::{RBig, Relaxed},
    repr::Repr,
};
use core::fmt::{self, Write};

// Generate Binary, Octal, LowerHex, UpperHex impls for Repr, RBig, and Relaxed.
// Pattern: if denominator is 1, print only the numerator; otherwise print
// "numerator/denominator" with each component in the target radix.
macro_rules! impl_radix_fmt {
    ($trait:ident) => {
        impl fmt::$trait for Repr {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if self.denominator.is_one() {
                    fmt::$trait::fmt(&self.numerator, f)
                } else {
                    fmt::$trait::fmt(&self.numerator, f)?;
                    f.write_char('/')?;
                    fmt::$trait::fmt(&self.denominator, f)
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

impl_radix_fmt!(Binary);
impl_radix_fmt!(Octal);
impl_radix_fmt!(LowerHex);
impl_radix_fmt!(UpperHex);

impl fmt::Debug for Repr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Repr")
                .field("numerator", &format_args!("{:#?}", &self.numerator))
                .field("denominator", &format_args!("{:#?}", &self.denominator))
                .finish()
        } else {
            f.write_fmt(format_args!("{:?} / {:?}", &self.numerator, &self.denominator))
        }
    }
}

impl fmt::Display for Repr {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator.is_one() {
            fmt::Display::fmt(&self.numerator, f)
        } else {
            fmt::Display::fmt(&self.numerator, f)?;
            f.write_char('/')?;
            fmt::Display::fmt(&self.denominator, f)
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

impl fmt::Display for RBig {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
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

impl fmt::Display for Relaxed {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

// ---- InRadix: arbitrary radix formatting for rationals ----

/// Representation of a rational number in a given radix, returned by
/// [`RBig::in_radix`] and [`Relaxed::in_radix`].
///
/// Implements [`Display`]. The alternate flag (`{:#}`) toggles uppercase
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
            if f.alternate() {
                write!(f, "{:#}", num)
            } else {
                write!(f, "{}", num)
            }
        } else {
            let den = self.denominator.in_radix(r);
            if f.alternate() {
                write!(f, "{:#}/{:#}", num, den)
            } else {
                write!(f, "{}/{}", num, den)
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
