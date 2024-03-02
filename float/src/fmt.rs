//! Implementation of formatters

use crate::{
    fbig::FBig,
    repr::{Context, Repr},
    round::{mode::Zero, Round},
    utils::{digit_len, split_digits_ref},
};
use alloc::string::String;
use core::fmt::{self, Alignment, Display, Formatter, Write};
use dashu_base::Sign;
use dashu_int::{IBig, Word};

trait DebugStructHelper {
    /// Print the full debug info for the significand
    fn field_significand<const B: Word>(&mut self, signif: &IBig) -> &mut Self;
}

impl<'a, 'b> DebugStructHelper for fmt::DebugStruct<'a, 'b> {
    fn field_significand<const B: Word>(&mut self, signif: &IBig) -> &mut Self {
        match B {
            2 => self.field(
                "significand",
                &format_args!("{:?} ({} bits)", signif, digit_len::<B>(signif)),
            ),
            10 => self.field("significand", &format_args!("{:#?}", signif)),
            _ => self.field(
                "significand",
                &format_args!("{:?} ({} digits)", signif, digit_len::<B>(signif)),
            ),
        }
    }
}

impl<const B: Word> fmt::Debug for Repr<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // shortcut for infinities
        if self.is_infinite() {
            return match self.sign() {
                Sign::Positive => f.write_str("inf"),
                Sign::Negative => f.write_str("-inf"),
            };
        }

        if f.alternate() {
            f.debug_struct("Repr")
                .field_significand::<B>(&self.significand)
                .field("exponent", &format_args!("{} ^ {}", &B, &self.exponent))
                .finish()
        } else {
            f.write_fmt(format_args!("{:?} * {} ^ {}", &self.significand, &B, &self.exponent))
        }
    }
}

impl<R: Round> fmt::Debug for Context<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let rnd_name = core::any::type_name::<R>();
        let rnd_name = rnd_name
            .rfind("::")
            .map(|pos| &rnd_name[pos + 2..])
            .unwrap_or(rnd_name);
        f.debug_struct("Context")
            .field("precision", &self.precision)
            .field("rounding", &format_args!("{}", rnd_name))
            .finish()
    }
}

impl<const B: Word> Repr<B> {
    /// Print the float number with given rounding mode.
    /// The rounding may happen if the precision option of the formatter is set.
    fn fmt_round<R: Round>(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // shortcut for infinities
        if self.is_infinite() {
            return match self.sign() {
                Sign::Positive => f.write_str("inf"),
                Sign::Negative => f.write_str("-inf"),
            };
        }

        // first perform rounding before actual printing if necessary
        let negative = self.significand.sign() == Sign::Negative;
        let rounded_signif;
        let (signif, exp) = if let Some(prec) = f.precision() {
            let diff = prec as isize + self.exponent;
            if diff < 0 {
                let shift = -diff as usize;
                let (signif, rem) = split_digits_ref::<B>(&self.significand, shift);
                let adjust = R::round_fract::<B>(&signif, rem, shift);
                rounded_signif = signif + adjust;
                (&rounded_signif, self.exponent - diff)
            } else {
                (&self.significand, self.exponent)
            }
        } else {
            (&self.significand, self.exponent)
        };

        // then print the digits to a buffer, without the sign
        let mut signif_str = String::new();
        write!(&mut signif_str, "{}", signif.in_radix(B as _))?;
        let signif_str = if negative {
            &signif_str[1..]
        } else {
            signif_str.as_str()
        };

        // calculate padding if necessary
        let (left_pad, right_pad) = if let Some(min_width) = f.width() {
            let mut signif_digits = signif_str.len();
            // the leading zeros needs to be printed (when the exponent of the number is very small).
            let leading_zeros = -(exp + signif_str.len() as isize - 1).min(0) as usize;
            // the trailing zeros needs to be printed (when the exponent of the number is very large)
            let mut trailing_zeros = exp.max(0) as usize;

            // if the precision option is set, there might be extra trailing zeros
            if let Some(prec) = f.precision() {
                let diff = prec as isize + exp.min(0);
                if diff > 0 {
                    trailing_zeros += diff as usize;
                }
            }
            if leading_zeros == 0 {
                // there is at least one digit to print (0)
                signif_digits = signif_digits.max(1);
            }

            let has_sign = (negative || f.sign_plus()) as usize;
            let has_radix_point = if exp > 0 {
                // if there's no fractional part, the result has the floating point
                // only if the precision is set to be non-zero
                f.precision().unwrap_or(0) > 0
            } else {
                // if there is fractional part, the result has the floating point
                // if the precision is not set, or set to be non-zero
                f.precision() != Some(0) // non-zero or none
            } as usize;

            let width = signif_digits + has_sign + has_radix_point + leading_zeros + trailing_zeros;

            // check alignment and calculate padding
            if width >= min_width {
                (0, 0)
            } else if f.sign_aware_zero_pad() {
                (min_width - width, 0)
            } else {
                match f.align() {
                    Some(Alignment::Left) => (0, min_width - width),
                    Some(Alignment::Right) | None => (min_width - width, 0),
                    Some(Alignment::Center) => {
                        let diff = min_width - width;
                        (diff / 2, diff - diff / 2)
                    }
                }
            }
        } else {
            (0, 0)
        };

        // print sign and left padding
        if !f.sign_aware_zero_pad() {
            for _ in 0..left_pad {
                f.write_char(f.fill())?;
            }
        }
        if negative {
            f.write_char('-')?;
        } else if f.sign_plus() {
            f.write_char('+')?;
        }
        if f.sign_aware_zero_pad() {
            for _ in 0..left_pad {
                f.write_char('0')?;
            }
        }

        // print the actual digits
        if exp < 0 {
            // If the exponent is negative, then the float number has fractional part
            let exp = -exp as usize;
            let (int, fract) = signif_str.split_at(signif_str.len().saturating_sub(exp));

            let frac_digits = fract.len();
            debug_assert!(frac_digits <= exp);

            // print the integral part, at least print a zero.
            if int.is_empty() {
                f.write_char('0')?;
            } else {
                f.write_str(int)?;
            }

            // print the fractional part, it has exactly `exp` digits (with left zero padding)
            if let Some(prec) = f.precision() {
                // don't print any fractional part if precision is zero
                if prec != 0 {
                    f.write_char('.')?;
                    if exp >= prec {
                        // the fractional part should be already rounded at the beginning
                        debug_assert!(exp == prec);

                        // print padding zeros
                        if prec > frac_digits {
                            for _ in 0..prec - frac_digits {
                                f.write_char('0')?;
                            }
                        }
                        if frac_digits > 0 {
                            f.write_str(fract)?;
                        }
                    } else {
                        // append zeros if the required precision is larger
                        for _ in 0..exp - frac_digits {
                            f.write_char('0')?;
                        }
                        f.write_str(fract)?;
                        for _ in 0..prec - exp {
                            f.write_char('0')?;
                        }
                    }
                }
            } else if frac_digits > 0 {
                f.write_char('.')?;
                for _ in 0..(exp - frac_digits) {
                    f.write_char('0')?;
                }
                f.write_str(fract)?;
            }
        } else {
            // In this case, the number is actually an integer and it can be trivially formatted.
            // However, when the precision option is set, we need to append zeros.

            // print the significand and append zeros if needed
            if signif_str.is_empty() {
                // this branch can happend when a negative float is rounded to zero.
                f.write_char('0')?;
            } else {
                f.write_str(signif_str)?;
            }
            for _ in 0..exp {
                f.write_char('0')?;
            }

            // print trailing zeros after the float point if the precision is set to be nonzero
            if let Some(prec) = f.precision() {
                if prec > 0 {
                    f.write_char('.')?;
                    for _ in 0..prec {
                        f.write_char('0')?;
                    }
                }
            }
        };

        // print right padding
        for _ in 0..right_pad {
            f.write_char(f.fill())?;
        }

        Ok(())
    }

    /// Print the float number in scientific notation with given rounding mode.
    /// The rounding may happen if the precision option of the formatter is set.
    ///
    /// When `use_hexadecimal` is True and base B is 2, the output will be represented
    /// in the hexadecimal format 0xaaa.bbbpcc.
    fn fmt_round_scientific<R: Round>(
        &self,
        f: &mut Formatter<'_>,
        upper: bool,
        use_hexadecimal: bool,
        exp_marker: Option<char>,
    ) -> fmt::Result {
        assert!(!(B != 2 && use_hexadecimal), "hexadecimal is only relevant for base 2");

        // shortcut for infinities
        if self.is_infinite() {
            return match self.sign() {
                Sign::Positive => f.write_str("inf"),
                Sign::Negative => f.write_str("-inf"),
            };
        }

        // first perform rounding before actual printing if necessary
        let negative = self.significand.sign() == Sign::Negative;
        let rounded_signif;
        let (signif, exp) = if let Some(prec) = f.precision() {
            // add one because always have one extra digit before the radix point
            let prec = if use_hexadecimal {
                (prec * 4 + 4) as isize
            } else {
                (prec + 1) as isize
            };
            let diff = prec - self.digits() as isize;
            if diff < 0 {
                let shift = -diff as usize;
                let (signif, rem) = split_digits_ref::<B>(&self.significand, shift);
                let adjust = R::round_fract::<B>(&signif, rem, shift);
                rounded_signif = signif + adjust;
                (&rounded_signif, self.exponent - diff)
            } else {
                (&self.significand, self.exponent)
            }
        } else {
            (&self.significand, self.exponent)
        };

        // then print the digits to a buffer, without the prefix or sign
        let (mut signif_str, mut exp_str) = (String::new(), String::new());
        match (upper, use_hexadecimal) {
            (false, false) => write!(&mut signif_str, "{}", signif.in_radix(B as _)),
            (true, false) => write!(&mut signif_str, "{:#}", signif.in_radix(B as _)),
            (false, true) => write!(&mut signif_str, "{:}", signif.in_radix(16)),
            (true, true) => write!(&mut signif_str, "{:#}", signif.in_radix(16)),
        }?;
        let signif_str = if negative {
            &signif_str[1..]
        } else {
            signif_str.as_str()
        };
        // adjust exp because the radix point is put after the first digit
        let exp_adjust = if use_hexadecimal {
            exp + (signif_str.len() as isize - 1) * 4
        } else {
            exp + signif_str.len() as isize - 1
        };
        write!(&mut exp_str, "{}", exp_adjust)?;
        let exp_str = exp_str.as_str();

        // calculate padding if necessary
        let (left_pad, right_pad) = if let Some(min_width) = f.width() {
            let prec = f.precision().unwrap_or(0);
            let has_point = signif_str.len() > 1 || prec > 0; // whether print the radix point
            let has_sign = negative || f.sign_plus();

            // if the precision option is set, there might be extra trailing zeros
            let trailing_zeros = if prec > signif_str.len() - 1 {
                prec - (signif_str.len() - 1)
            } else {
                0
            };

            let width = signif_str.len() + exp_str.len()
                + /* exponent marker */ 1
                + has_sign as usize
                + has_point as usize
                + use_hexadecimal as usize * 2
                + trailing_zeros;

            if width >= min_width {
                (0, 0)
            } else {
                match f.align() {
                    Some(Alignment::Left) => (0, min_width - width),
                    Some(Alignment::Right) | None => (min_width - width, 0),
                    Some(Alignment::Center) => {
                        let diff = min_width - width;
                        (diff / 2, diff - diff / 2)
                    }
                }
            }
        } else {
            (0, 0)
        };

        // print sign and left padding
        if !f.sign_aware_zero_pad() {
            for _ in 0..left_pad {
                f.write_char(f.fill())?;
            }
        }
        if negative {
            f.write_char('-')?;
        } else if f.sign_plus() {
            f.write_char('+')?;
        }
        if use_hexadecimal {
            f.write_str("0x")?;
        }
        if f.sign_aware_zero_pad() {
            for _ in 0..left_pad {
                f.write_char('0')?;
            }
        }

        // print the body
        let (int, fract) = signif_str.split_at(1);
        f.write_str(int)?;
        if !fract.is_empty() {
            f.write_char('.')?;
            f.write_str(fract)?;
        }
        let prec = f.precision().unwrap_or(0);
        if prec > 0 {
            if fract.is_empty() {
                f.write_char('.')?
            }
            for _ in fract.len()..prec {
                f.write_char('0')?;
            }
        }

        f.write_char(exp_marker.unwrap_or('@'))?;
        f.write_str(exp_str)?;

        // print right padding
        for _ in 0..right_pad {
            f.write_char(f.fill())?;
        }

        Ok(())
    }
}

impl<const B: Word> Display for Repr<B> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt_round::<Zero>(f)
    }
}

impl<R: Round, const B: Word> fmt::Debug for FBig<R, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // shortcut for infinities
        if self.repr.is_infinite() {
            return match self.repr.sign() {
                Sign::Positive => f.write_str("inf"),
                Sign::Negative => f.write_str("-inf"),
            };
        }

        let rnd_name = core::any::type_name::<R>();
        let rnd_name = rnd_name
            .rfind("::")
            .map(|pos| &rnd_name[pos + 2..])
            .unwrap_or(rnd_name);

        if f.alternate() {
            f.debug_struct("FBig")
                .field_significand::<B>(&self.repr.significand)
                .field("exponent", &format_args!("{} ^ {}", &B, &self.repr.exponent))
                .field("precision", &self.context.precision)
                .field("rounding", &format_args!("{}", rnd_name))
                .finish()
        } else {
            f.write_fmt(format_args!("{:?} (prec: {})", &self.repr, &self.context.precision))
        }
    }
}

impl<R: Round, const B: Word> Display for FBig<R, B> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.repr.fmt_round::<R>(f)
    }
}

macro_rules! impl_fmt_with_base {
    ($base:literal, $trait:ident, $upper: literal, $hex:literal, $marker:literal) => {
        impl fmt::$trait for Repr<$base> {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.fmt_round_scientific::<Zero>(f, $upper, $hex, Some($marker))
            }
        }

        impl<R: Round> fmt::$trait for FBig<R, $base> {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.repr
                    .fmt_round_scientific::<R>(f, $upper, $hex, Some($marker))
            }
        }
    };
}

// TODO(v1.0): Alternate flags can be used to print upper separator, for example 'p' -> 'P'.
//             In case of base ten, it can be used to switch between '@' and 'e'/'E'.
//             Need to investigate what is the best way to utilize the alternate flag before implementing.
impl_fmt_with_base!(2, LowerHex, false, true, 'p');
impl_fmt_with_base!(2, UpperHex, true, true, 'p');
impl_fmt_with_base!(2, Binary, false, false, 'b');
impl_fmt_with_base!(8, Octal, false, false, 'o');
impl_fmt_with_base!(16, LowerHex, false, false, 'h');
impl_fmt_with_base!(16, UpperHex, true, false, 'h');

impl<const B: Word> fmt::LowerExp for Repr<B> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let marker = match B {
            10 => Some('e'),
            _ => None,
        };
        self.fmt_round_scientific::<Zero>(f, false, false, marker)
    }
}
impl<const B: Word> fmt::UpperExp for Repr<B> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let marker = match B {
            10 => Some('E'),
            _ => None,
        };
        self.fmt_round_scientific::<Zero>(f, true, false, marker)
    }
}
impl<R: Round, const B: Word> fmt::LowerExp for FBig<R, B> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let marker = match B {
            10 => Some('e'),
            _ => None,
        };
        self.repr.fmt_round_scientific::<R>(f, false, false, marker)
    }
}
impl<R: Round, const B: Word> fmt::UpperExp for FBig<R, B> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let marker = match B {
            10 => Some('E'),
            _ => None,
        };
        self.repr.fmt_round_scientific::<R>(f, true, false, marker)
    }
}
