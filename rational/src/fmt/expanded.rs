use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::{self, Display, Formatter, LowerExp, UpperExp, Write};
use dashu_base::{DivRem, Sign, UnsignedAbs};
use dashu_int::{UBig, Word};

use crate::rbig::{RBig, Relaxed};

/// Returned by [`RBig::in_expanded`] and [`Relaxed::in_expanded`].
///
/// Implements [`Display`], [`LowerExp`], and [`UpperExp`] for printing a
/// rational number in expanded positional notation (e.g., `0.3333` for 1/3).
///
/// # Format options
///
/// - `{}` — default expansion with a fixed number of fractional digits.
/// - `{:.N}` — exactly N digits after the radix point.
/// - `{:#}` — detect and display the repetend in parentheses (e.g., `0.(3)`).
/// - `{:e}` / `{:E}` — scientific notation. `#` has no effect in this mode.
/// - `{:.Ne}` / `{:.NE}` — scientific notation with N digits of precision.
pub struct InExpanded<'a> {
    sign: Sign,
    num_abs: UBig,
    denominator: &'a UBig,
    radix: u8,
}

/// Returned by `expand`.
struct Expanded {
    int_digits: Vec<u8>,
    frac_prefix: Vec<u8>, // non-repeating fractional digits
    repetend: Vec<u8>,    // repeating part (empty = terminating)
}

/// Perform long division and record digits.
///
/// If `track_repetend` is true, uses a `BTreeMap` to detect cycles. Stops when
/// `max_digits` fractional digits have been produced or when a terminating
/// condition is reached.
///
/// TODO(v0.5): use `RadixInfo` fast dividers from `dashu_int` for batched
/// digit extraction instead of one-digit-at-a-time `rem * radix / den`.
fn expand(num: &UBig, den: &UBig, radix: u8, max_digits: usize, track_repetend: bool) -> Expanded {
    let (int_part, mut rem) = num.div_rem(den);
    let int_digits: Vec<u8> = int_part
        .to_digits(radix as Word)
        .into_iter()
        .map(|d| d as u8)
        .collect();
    let mut frac_digits: Vec<u8> = Vec::with_capacity(max_digits);
    let mut seen: Option<BTreeMap<UBig, usize>> = if track_repetend {
        Some(BTreeMap::new())
    } else {
        None
    };
    let mut repetend_start: Option<usize> = None;

    while frac_digits.len() < max_digits {
        if rem.is_zero() {
            break;
        }
        if let Some(ref mut map) = seen {
            if let Some(&pos) = map.get(&rem) {
                repetend_start = Some(pos);
                break;
            }
            map.insert(rem.clone(), frac_digits.len());
        }

        // rem = rem * radix
        let scaled = &rem * radix;
        let (digit, new_rem) = scaled.div_rem(den);
        rem = new_rem;

        // digit is guaranteed 0..radix-1 (radix <= 36), so fits in u8
        frac_digits.push(u8::try_from(&digit).unwrap());
    }

    let (frac_prefix, repetend) = if let Some(start) = repetend_start {
        let repetend = frac_digits.split_off(start);
        (frac_digits, repetend)
    } else {
        (frac_digits, Vec::new())
    };

    Expanded {
        int_digits,
        frac_prefix,
        repetend,
    }
}

/// Default number of fractional digits for a given radix.
///
/// Returns `Word::BITS` digits for all radices. Use `{:.N}` to control
/// precision explicitly.
fn default_precision(_radix: u8) -> usize {
    Word::BITS as usize
}

/// Propagate a carry of 1 through a mutable slice of radix digits.
/// Returns `true` if the carry overflowed all digits (all wrapped to 0).
fn propagate_carry(digits: &mut [u8], radix: u8) -> bool {
    let carry = 1u8;
    for d in digits.iter_mut().rev() {
        *d += carry;
        if *d >= radix {
            *d -= radix;
        } else {
            return false;
        }
    }
    true
}

/// Check the guard digit at position `keep`, truncate, and apply half-up rounding.
/// Returns `true` if the carry propagated through all `keep` digits (rollover).
/// Returns `false` immediately if there are not enough digits for a guard.
fn round_and_carry(digits: &mut Vec<u8>, radix: u8, keep: usize) -> bool {
    if digits.len() <= keep {
        return false;
    }
    let extra = digits[keep];
    digits.truncate(keep);
    extra * 2 >= radix && propagate_carry(digits, radix)
}

impl Display for InExpanded<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.sign.as_sign_str(f.sign_plus()))?;

        // Zero shortcut
        if self.num_abs.is_zero() {
            f.write_char('0')?;
            if let Some(prec) = f.precision() {
                if prec > 0 {
                    f.write_char('.')?;
                    for _ in 0..prec {
                        f.write_char('0')?;
                    }
                }
            }
            return Ok(());
        }

        let prec = f
            .precision()
            .unwrap_or_else(|| default_precision(self.radix));
        let show_repetend = f.alternate();

        // When detecting repetends we need extra digits to find the cycle.
        let max_digits = if show_repetend {
            (prec + 1).max(128)
        } else {
            prec + 1
        };

        let expanded =
            expand(&self.num_abs, self.denominator, self.radix, max_digits, show_repetend);

        let mut int_digits = expanded.int_digits;

        // When repetend display is active, show the exact pattern without rounding.
        if show_repetend && !expanded.repetend.is_empty() {
            write_digits(f, &int_digits, self.radix, false)?;
            if !expanded.frac_prefix.is_empty() || !expanded.repetend.is_empty() {
                f.write_char('.')?;
            }
            write_digits(f, &expanded.frac_prefix, self.radix, false)?;
            f.write_char('(')?;
            write_digits(f, &expanded.repetend, self.radix, false)?;
            f.write_char(')')?;
        } else {
            let total_frac: Vec<u8> = if !expanded.repetend.is_empty() {
                [&expanded.frac_prefix[..], &expanded.repetend[..]].concat()
            } else {
                expanded.frac_prefix.clone()
            };

            let mut frac_digits = total_frac;

            if round_and_carry(&mut frac_digits, self.radix, prec)
                && propagate_carry(&mut int_digits, self.radix)
            {
                int_digits.insert(0, 1);
            }

            // Print integer part
            write_digits(f, &int_digits, self.radix, false)?;

            // Print fractional part if needed
            if !frac_digits.is_empty() || prec > 0 {
                f.write_char('.')?;
                let printed = frac_digits.len().min(prec);
                write_digits(f, &frac_digits[..printed], self.radix, false)?;
                for _ in printed..prec {
                    f.write_char('0')?;
                }
            }
        }

        Ok(())
    }
}

impl LowerExp for InExpanded<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt_scientific(f, 'e')
    }
}

impl UpperExp for InExpanded<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt_scientific(f, 'E')
    }
}

impl InExpanded<'_> {
    fn fmt_scientific(&self, f: &mut Formatter<'_>, exp_char: char) -> fmt::Result {
        f.write_str(self.sign.as_sign_str(f.sign_plus()))?;

        if self.num_abs.is_zero() {
            f.write_char('0')?;
            let prec = f.precision().unwrap_or(0);
            if prec > 0 {
                f.write_char('.')?;
                for _ in 0..prec {
                    f.write_char('0')?;
                }
            }
            return write!(f, "{}0", exp_char);
        }

        let prec = f
            .precision()
            .unwrap_or_else(|| default_precision(self.radix));
        let exp_marker = if self.radix == 10 { exp_char } else { '@' };

        // Compute integer part and remainder
        let (int_part, rem) = (&self.num_abs).div_rem(self.denominator);

        let exp: isize;
        let mut significand_digits: Vec<u8>;

        if !int_part.is_zero() {
            // Integer part >= 1: exponent = number of int digits - 1
            let int_digits: Vec<u8> = int_part
                .to_digits(self.radix as Word)
                .into_iter()
                .map(|d| d as u8)
                .collect();
            exp = int_digits.len() as isize - 1;
            significand_digits = int_digits;
            // Compute fractional digits to reach prec + 2 total (1 before point, prec+1 after)
            let need_frac = (prec + 2).saturating_sub(significand_digits.len());
            let more = expand_fraction(rem, self.denominator, self.radix, need_frac);
            significand_digits.extend_from_slice(&more);
        } else {
            // Integer part == 0: find first non-zero fractional digit
            let mut cur_rem = rem.clone();
            let mut leading_zeros: isize = 0;
            loop {
                if cur_rem.is_zero() {
                    // The number is exactly zero — should have been caught above
                    exp = 0;
                    significand_digits = vec![0];
                    break;
                }
                let scaled = &cur_rem * self.radix;
                let (d, new_rem) = scaled.div_rem(self.denominator);
                cur_rem = new_rem;
                if !d.is_zero() {
                    leading_zeros += 1;
                    exp = -leading_zeros;
                    significand_digits = vec![u8::try_from(&d).unwrap()];
                    // Compute remaining significand digits
                    let more = expand_fraction(cur_rem, self.denominator, self.radix, prec + 1);
                    significand_digits.extend_from_slice(&more);
                    break;
                }
                leading_zeros += 1;
            }
        };

        // Round the significand
        if round_and_carry(&mut significand_digits, self.radix, prec + 1) {
            significand_digits.insert(0, 1);
        }

        // Re-check for rollover
        let actual_exp = if significand_digits.len() > prec + 1 {
            // Rollover happened
            significand_digits.truncate(prec + 1);
            exp + 1
        } else {
            exp
        };

        // Print: first digit, '.', remaining digits, exp marker, exponent
        let upper = exp_char == 'E';
        let first = significand_digits.first().copied().unwrap_or(0);
        write_digit_char(f, first, self.radix, upper)?;

        let rest = &significand_digits[1..];
        if !rest.is_empty() || prec > 0 {
            f.write_char('.')?;
            let end = prec.min(rest.len());
            write_digits(f, &rest[..end], self.radix, upper)?;
            // Pad with zeros if needed
            for _ in end..prec {
                f.write_char('0')?;
            }
        }

        write!(f, "{}{}", exp_marker, actual_exp)
    }
}

/// Compute `n` fractional digits via long division starting from `rem`.
/// Does not track repetends.
fn expand_fraction(mut rem: UBig, den: &UBig, radix: u8, n: usize) -> Vec<u8> {
    let mut digits = Vec::with_capacity(n);
    for _ in 0..n {
        if rem.is_zero() {
            digits.push(0);
        } else {
            let scaled = &rem * radix;
            let (digit, new_rem) = scaled.div_rem(den);
            rem = new_rem;
            digits.push(u8::try_from(&digit).unwrap());
        }
    }
    digits
}

/// Write a slice of digit values in the given radix to the formatter.
///
/// TODO(v0.5): replace with `DigitWriter` from `dashu_int::fmt` for buffered,
/// SIMD-accelerated digit-to-ASCII conversion.
fn write_digits(f: &mut Formatter<'_>, digits: &[u8], radix: u8, upper: bool) -> fmt::Result {
    for &d in digits {
        write_digit_char(f, d, radix, upper)?;
    }
    Ok(())
}

/// Write a single digit value (0..radix-1) as a character.
fn write_digit_char(f: &mut Formatter<'_>, digit: u8, _radix: u8, upper: bool) -> fmt::Result {
    let ch = if digit < 10 {
        (b'0' + digit) as char
    } else if upper {
        (b'A' + (digit - 10)) as char
    } else {
        (b'a' + (digit - 10)) as char
    };
    f.write_char(ch)
}

impl RBig {
    /// Representation in expanded positional notation.
    ///
    /// Returns a wrapper that implements [`Display`], [`LowerExp`], and
    /// [`UpperExp`] for printing the rational number in fractional form.
    ///
    /// The `radix` parameter is `u8`. Valid radices are 2 through 36.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// let one_third = RBig::from_parts(1.into(), 3u8.into());
    /// assert_eq!(format!("{:.4}", one_third.in_expanded(10)), "0.3333");
    /// assert_eq!(format!("{:#.4}", one_third.in_expanded(10)), "0.(3)");
    /// assert_eq!(format!("{:.4e}", one_third.in_expanded(10)), "3.3333e-1");
    /// ```
    #[inline]
    pub fn in_expanded(&self, radix: u8) -> InExpanded<'_> {
        assert!((2..=36).contains(&radix), "radix must be between 2 and 36");
        InExpanded {
            sign: self.0.numerator.sign(),
            num_abs: self.0.numerator.clone().unsigned_abs(),
            denominator: self.denominator(),
            radix,
        }
    }
}

impl Relaxed {
    /// Representation in expanded positional notation.
    ///
    /// The `radix` parameter is `u8`. See [`RBig::in_expanded`] for details.
    #[inline]
    pub fn in_expanded(&self, radix: u8) -> InExpanded<'_> {
        assert!((2..=36).contains(&radix), "radix must be between 2 and 36");
        InExpanded {
            sign: self.0.numerator.sign(),
            num_abs: self.0.numerator.clone().unsigned_abs(),
            denominator: self.denominator(),
            radix,
        }
    }
}
