//! Implementation of formatters

use crate::{
    repr::FloatRepr,
    utils::{get_precision, shr_radix, shr_rem_radix},
    Rounding,
};
use core::fmt::{self, Display, Formatter, Write};
use dashu_base::Abs;
use dashu_int::Sign;

// TODO: implement Debug using mantissa * radix ^ exponent (prec: xxx),
// FIXME: sign, width and fill options are not yet correctly handled
// TODO: print decimal by default, and print in native radix

impl<const X: usize, const R: u8> Display for FloatRepr<X, R> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // print in decimal if the alternate flag is set
        if f.alternate() && X != 10 {
            return self.clone().into_decimal().fmt(f);
        }

        if self.exponent < 0 {
            let exp = -self.exponent as usize;
            let (trunc, frac) = shr_rem_radix::<X>(&self.mantissa, exp);
            let frac_prec = get_precision::<X>(&frac);
            assert!(frac_prec <= exp);
            let mut frac = frac.abs(); // don't print sign for fractional part

            // print integral part
            if trunc.is_zero() && self.mantissa.sign() == Sign::Negative {
                f.write_char('-')?;
            }
            trunc.in_radix(X as u32).fmt(f)?;

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
                            let (shifted, mut rem) = shr_rem_radix::<X>(&frac, exp - v);
                            frac = shifted;
                            shr_radix::<X>(&mut rem, exp - v - 1);
                            frac += Rounding::from_fract::<X, R>(&frac, rem, exp - v);
                            get_precision::<X>(&frac)
                        } else {
                            0
                        };

                        if v > new_prec {
                            for _ in 0..v - new_prec {
                                f.write_char('0')?;
                            }
                        }
                        if frac_prec > exp - v {
                            frac.in_radix(X as u32).fmt(f)?;
                        }
                    } else {
                        // append zeros if the required precision is larger
                        for _ in 0..exp - frac_prec {
                            f.write_char('0')?;
                        }
                        frac.in_radix(X as u32).fmt(f)?;
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
                    frac.in_radix(X as u32).fmt(f)?;
                }
            }
        } else {
            // directly print the mantissa and append zeros if needed
            // precision doesn't make a difference since we force printing in native radix
            self.mantissa.in_radix(X as u32).fmt(f)?;
            for _ in 0..self.exponent {
                f.write_char('0')?;
            }
        };

        Ok(())
    }
}
