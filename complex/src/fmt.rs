//! [`Display`] / [`Debug`] for [`CBig`] in the algebraic `a+bi` notation.
//!
//! This diverges from MPC's parenthesized `"(re im)"` form: `dashu-cmplx` uses the human-readable
//! algebraic notation (the `num-complex` idiom). The parenthesized form is **not** accepted on input.

use crate::cbig::{is_numeric_zero, CBig};
use core::fmt::{self, Debug, Display, Formatter, Write};
use dashu_float::round::Round;
use dashu_float::{FBig, Repr};
use dashu_int::{IBig, Word};

/// A part is a unit (value exactly `±1`) iff its normalized significand is `±1` at exponent `0`.
fn is_unit<const B: Word>(repr: &Repr<B>) -> bool {
    repr.exponent() == 0 && {
        let s = repr.significand();
        *s == IBig::from(1) || *s == IBig::from(-1)
    }
}

impl<R: Round, const B: Word> Display for CBig<R, B> {
    /// Format in algebraic `a+bi` form: `"1+2i"`, `"-3-4i"`, `"5"` (pure real), `"-7i"` (pure
    /// imaginary), `"i"` (`0+1i`), `"-i"` (`0-1i`). The imaginary term always carries an explicit
    /// sign, a unit coefficient is elided, and a zero imaginary is omitted. Each coefficient uses
    /// [`FBig`]'s native `Display` (specials render as `inf` / `-inf`).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let fctx = self.context.float();
        let re_zero = is_numeric_zero(&self.re);
        let im_zero = is_numeric_zero(&self.im);
        let im_neg = self.im.sign() == dashu_base::Sign::Negative;
        let im_unit = is_unit(&self.im);

        if im_zero {
            // pure real (incl. 0+0i → "0")
            let re = FBig::from_repr(self.re.clone(), fctx);
            return Display::fmt(&re, f);
        }

        // imaginary part is nonzero
        if !re_zero {
            let re = FBig::from_repr(self.re.clone(), fctx);
            Display::fmt(&re, f)?;
            f.write_char(if im_neg { '-' } else { '+' })?;
        } else if im_neg {
            f.write_char('-')?;
        }
        if !im_unit {
            let im_abs_repr = if im_neg {
                -self.im.clone()
            } else {
                self.im.clone()
            };
            let im_abs = FBig::from_repr(im_abs_repr, fctx);
            Display::fmt(&im_abs, f)?;
        }
        f.write_char('i')
    }
}

impl<R: Round, const B: Word> Debug for CBig<R, B> {
    /// Structured form `"re:<re> im:<im> (prec: <p>)"` — e.g. `"re:1.5 im:-2.0 (prec: 53)"` — for
    /// quick inspection (mirrors `FBig`'s `Debug` style). The alternate `#` form exposes the raw
    /// significands and exponent scaling.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let fctx = self.context.float();
        let re = FBig::from_repr(self.re.clone(), fctx);
        let im = FBig::from_repr(self.im.clone(), fctx);
        if f.alternate() {
            f.debug_struct("CBig")
                .field("re", &re)
                .field("im", &im)
                .field("precision", &self.context.precision())
                .finish()
        } else {
            f.write_str("re:")?;
            Display::fmt(&re, f)?;
            f.write_str(" im:")?;
            Display::fmt(&im, f)?;
            f.write_fmt(format_args!(" (prec: {})", self.context.precision()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use dashu_float::round::mode;

    type C = CBig<mode::HalfAway, 10>;

    fn c(re: i32, im: i32) -> C {
        C::from_parts(re.into(), im.into())
    }

    #[test]
    fn display_algebraic() {
        assert_eq!(format!("{}", c(0, 0)), "0");
        assert_eq!(format!("{}", c(5, 0)), "5");
        assert_eq!(format!("{}", c(0, 1)), "i");
        assert_eq!(format!("{}", c(0, -1)), "-i");
        assert_eq!(format!("{}", c(0, 4)), "4i");
        assert_eq!(format!("{}", c(0, -4)), "-4i");
        assert_eq!(format!("{}", c(1, 2)), "1+2i");
        assert_eq!(format!("{}", c(-3, -4)), "-3-4i");
        assert_eq!(format!("{}", c(1, 1)), "1+i");
        assert_eq!(format!("{}", c(2, -1)), "2-i");
    }

    #[test]
    fn display_constants() {
        assert_eq!(format!("{}", C::I), "i");
        assert_eq!(format!("{}", C::ONE), "1");
        assert_eq!(format!("{}", C::ZERO), "0");
    }

    #[test]
    fn debug_structured() {
        let z = C::from_parts(FBig::from(3), FBig::from(4));
        let s = format!("{:?}", z);
        assert!(s.starts_with("re:3 im:4 (prec:"));
    }
}
