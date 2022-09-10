//! Implementation of formatters

use crate::{
    fbig::FBig,
    repr::{Context, Repr},
    round::Round,
    utils::{digit_len, shr_digits_in_place, split_digits_ref},
};
use core::fmt::{self, Display, Formatter, Write};
use dashu_base::{Abs, Sign};
use dashu_int::Word;

impl<const B: Word> fmt::Debug for Repr<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.significand, f)?;
        f.write_str(" * ")?;
        fmt::Debug::fmt(&B, f)?;
        f.write_str(" ^ ")?;
        fmt::Debug::fmt(&self.exponent, f)
    }
}

impl<R: Round> fmt::Debug for Context<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(" (prec: ")?;
        fmt::Debug::fmt(&self.precision, f)?;
        f.write_str(", rnd: ")?;
        f.write_str(core::any::type_name::<R>())?;
        f.write_str(")")
    }
}

impl<R: Round, const B: Word> fmt::Debug for FBig<R, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.repr, f)?;
        fmt::Debug::fmt(&self.context, f)
    }
}

// FIXME: sign, width and fill options are not yet correctly handled

impl<R: Round, const B: Word> Display for FBig<R, B> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // print in decimal if the alternate flag is set
        if f.alternate() && B != 10 {
            return self.to_decimal().value().fmt(f);
        }

        if self.repr.exponent < 0 {
            let exp = -self.repr.exponent as usize;
            let (int, frac) = split_digits_ref::<B>(&self.repr.significand, exp);
            let frac_prec = digit_len::<B>(&frac);
            assert!(frac_prec <= exp);
            let mut frac = frac.abs(); // don't print sign for fractional part

            // print integral part
            if int.is_zero() && self.repr.sign() == Sign::Negative {
                f.write_char('-')?;
            }
            int.in_radix(B as u32).fmt(f)?;

            // print fractional part
            // note that the fractional part has actually exp digits (with left zero padding)
            if let Some(v) = f.precision() {
                if v != 0 {
                    f.write_char('.')?;
                    if exp >= v {
                        // shrink fractional part if it exceeds the required precision
                        // there could be one more digit in the fractional part after rounding
                        let new_prec = if exp == v {
                            frac_prec
                        } else if frac_prec > exp - v {
                            let (shifted, mut rem) = split_digits_ref::<B>(&frac, exp - v);
                            frac = shifted;
                            shr_digits_in_place::<B>(&mut rem, exp - v - 1);
                            frac += R::round_fract::<B>(&frac, rem, exp - v);
                            digit_len::<B>(&frac)
                        } else {
                            0
                        };

                        if v > new_prec {
                            for _ in 0..v - new_prec {
                                f.write_char('0')?;
                            }
                        }
                        if frac_prec > exp - v {
                            frac.in_radix(B as u32).fmt(f)?;
                        }
                    } else {
                        // append zeros if the required precision is larger
                        for _ in 0..exp - frac_prec {
                            f.write_char('0')?;
                        }
                        frac.in_radix(B as u32).fmt(f)?;
                        for _ in 0..v - exp {
                            f.write_char('0')?; // TODO: padding handling is not correct here
                        }
                    }
                }
                // don't print any fractional part if precision is zero
            } else {
                if frac_prec > 0 {
                    f.write_char('.')?;
                    for _ in 0..(exp - frac_prec) {
                        f.write_char('0')?;
                    }
                    frac.in_radix(B as u32).fmt(f)?;
                }
            }
        } else {
            // directly print the significand and append zeros if needed
            // precision doesn't make a difference since we force printing in native radix
            self.repr.significand.in_radix(B as u32).fmt(f)?;
            for _ in 0..self.repr.exponent {
                f.write_char('0')?;
            }
        };

        Ok(())
    }
}

// TODO: impl LowerHex and UpperHex for FBig with base 2, printing the "0xabcp-n" format.
