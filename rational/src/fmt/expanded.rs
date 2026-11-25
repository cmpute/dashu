//! Positional-expansion formatting for rationals: render the infinite decimal (or other-base)
//! expansion of an [`RBig`]/[`Relaxed`], exposing the non-repeating and repeating parts.
//!
//! The [`InExpanded`] helper drives this output and supports precision control, scientific
//! notation, and repetend display.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{self, Display, Formatter, LowerExp, UpperExp, Write};
use dashu_base::{DivRem, Gcd, Sign, UnsignedAbs};
use dashu_int::{fast_div::ConstDivisor, UBig, Word};

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
/// - `{:#}` — exact display: a finite expansion is printed in full (padded with zeros up to
///   the precision), and a repeating one is printed with its repetend in parentheses
///   (e.g., `0.(3)`). The repetend is found when the non-repeating digits and the period
///   together are at most `max(N + 1, 128)` digits long, where `N` is the precision;
///   otherwise the output falls back to the rounded form of `{:.N}`.
/// - `{:e}` / `{:E}` — scientific notation. `#` has no effect in this mode. The exponent
///   marker is `e` / `E` for radix 10 and `@` for every other radix (where `e` can be a digit).
/// - `{:.Ne}` / `{:.NE}` — scientific notation with N digits of precision.
///
/// Rounded output is rounded to the nearest value, with ties away from zero.
///
/// The `+` flag, the width, the fill, the alignment and the `0` flag apply to the number as a
/// whole, as they do for the primitive numbers (`{:>8.2}` of 1/2 is `"    0.50"` and
/// `{:08.2}` of -1/2 is `"-0000.50"`).
pub struct InExpanded<'a> {
    sign: Sign,
    num_abs: UBig,
    denominator: &'a UBig,
    radix: u8,
}

/// Convert an integer into its base-`radix` digits, most-significant first.
fn to_radix_digits(n: &UBig, radix: u8) -> Vec<u8> {
    n.to_digits(radix as Word)
        .into_iter()
        .map(|d| d as u8)
        .collect()
}

/// Exact expansion of the fraction `rem / den` (with `rem < den`), produced one digit at a
/// time so that a cycle can be detected at single-digit granularity (e.g. `1/6 -> 0.1(6)`).
///
/// Returns the non-repeating prefix and the repetend (empty for a terminating expansion)
/// if the expansion is resolved within `budget` digits, i.e. if the number of prefix digits
/// plus the period is at most `budget`. Returns `None` otherwise.
fn expand_exact(rem: &UBig, den: &UBig, radix: u8, budget: usize) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut rem = rem.clone();
    let mut digits: Vec<u8> = Vec::new();
    let mut seen: BTreeMap<UBig, usize> = BTreeMap::new();
    loop {
        // The state after `digits.len()` digits is checked before the budget, so that an
        // expansion using exactly `budget` digits is still recognized.
        if rem.is_zero() {
            return Some((digits, Vec::new()));
        }
        if let Some(&pos) = seen.get(&rem) {
            let repetend = digits.split_off(pos);
            return Some((digits, repetend));
        }
        if digits.len() >= budget {
            return None;
        }
        seen.insert(rem.clone(), digits.len());

        let (digit, new_rem) = (&rem * radix).div_rem(den);
        rem = new_rem;
        // digit is guaranteed 0..radix-1 (radix <= 36), so fits in u8
        digits.push(u8::try_from(&digit).unwrap());
    }
}

/// Whether the expansion of `rem / den` in the given radix is finite, that is whether every
/// prime factor of the reduced denominator divides the radix.
fn expansion_terminates(rem: &UBig, den: &UBig, radix: u8) -> bool {
    if rem.is_zero() {
        return true;
    }
    let mut d = den / rem.gcd(den);
    // strip the factors shared with the radix, a word-sized power of the radix at a time
    let (_, radix_k) = digits_per_word(radix as Word);
    let radix_k = UBig::from(radix_k);
    loop {
        let g = (&d).gcd(&radix_k);
        if g.is_one() {
            break;
        }
        d /= g;
    }
    d.is_one()
}

/// Largest `k` such that `radix^k <= Word::MAX`, together with the value `radix^k`, used to batch
/// fractional digit extraction.
const fn digits_per_word(radix: Word) -> (usize, Word) {
    let mut k = 0usize;
    let mut power: Word = 1;
    while power <= Word::MAX / radix {
        power *= radix;
        k += 1;
    }
    (k, power)
}

/// Extract up to `limit` fractional base-`radix` digits of the value `rem / den`
/// (with `rem < den`, i.e. the value is in `[0, 1)`), stopping early if the expansion
/// terminates. Returns the digits together with the remainder left after them: the
/// part of the value not covered by the returned digits is `remainder / den` units of the
/// last digit (zero iff the returned digits are the complete expansion).
///
/// Each iteration produces up to `k` digits (`radix^k` being the largest power of the radix
/// fitting a word): multiply the remainder by `radix^k`, divide once by the precomputed
/// divisor, then split the word-sized quotient chunk into `k` digits (most-significant first).
/// This trades `k` big-int divisions for one.
fn expand_frac(mut rem: UBig, den: &UBig, radix: u8, limit: usize) -> (Vec<u8>, UBig) {
    let radix = radix as Word;
    let (k, radix_k) = digits_per_word(radix);
    let den_div = ConstDivisor::new(den.clone());

    let mut digits: Vec<u8> = Vec::new();
    while digits.len() < limit && !rem.is_zero() {
        // The last chunk may be shorter, so that the remainder corresponds to exactly
        // `limit` digits (it decides the rounding of the last digit).
        let take = (limit - digits.len()).min(k);
        let scale = if take == k {
            radix_k
        } else {
            radix.pow(take as u32)
        };
        let (quot, new_rem) = (&rem * scale).div_rem(&den_div);
        rem = new_rem;

        // quot < radix^take <= Word::MAX, so it always fits in a single Word.
        let mut word: Word = quot.try_into().unwrap();

        // split `word` into `take` base-radix digits, filling the chunk from its end
        let start = digits.len();
        digits.resize(start + take, 0);
        for d in digits[start..].iter_mut().rev() {
            *d = (word % radix) as u8;
            word /= radix;
        }
    }
    (digits, rem)
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

/// Whether a value whose discarded part is `rem / den` units of the last kept digit is
/// rounded up (round half up on the magnitude).
///
/// The decision is made on the exact remainder rather than on a guard digit: in an odd radix
/// a single guard digit cannot tell which side of the half-way point the value is on.
fn rounds_up(rem: &UBig, den: &UBig) -> bool {
    (rem << 1usize) >= *den
}

impl Display for InExpanded<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let (precision, alternate) = (f.precision(), f.alternate());
        self.fmt_padded(f, |this, w| this.write_fixed(w, precision, alternate))
    }
}

impl LowerExp for InExpanded<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let precision = f.precision();
        self.fmt_padded(f, |this, w| this.write_scientific(w, precision, 'e'))
    }
}

impl UpperExp for InExpanded<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let precision = f.precision();
        self.fmt_padded(f, |this, w| this.write_scientific(w, precision, 'E'))
    }
}

impl InExpanded<'_> {
    /// Write the sign followed by the magnitude produced by `write_magnitude`, applying the
    /// `+` flag, the width, the fill, the alignment and the `0` flag of the formatter to the
    /// number as a whole.
    fn fmt_padded(
        &self,
        f: &mut Formatter<'_>,
        write_magnitude: impl Fn(&Self, &mut dyn Write) -> fmt::Result,
    ) -> fmt::Result {
        if f.width().is_none() {
            // no padding: write the parts directly, without an intermediate buffer
            f.write_str(self.sign.as_sign_str(f.sign_plus()))?;
            write_magnitude(self, f)
        } else {
            let mut magnitude = String::new();
            write_magnitude(self, &mut magnitude)?;
            f.pad_integral(self.sign == Sign::Positive, "", &magnitude)
        }
    }

    /// Write the magnitude in the positional notation, with the given precision and
    /// alternate flag of the format specification.
    fn write_fixed(
        &self,
        f: &mut dyn Write,
        precision: Option<usize>,
        alternate: bool,
    ) -> fmt::Result {
        // Zero shortcut
        if self.num_abs.is_zero() {
            f.write_char('0')?;
            if let Some(prec) = precision {
                if prec > 0 {
                    f.write_char('.')?;
                    for _ in 0..prec {
                        f.write_char('0')?;
                    }
                }
            }
            return Ok(());
        }

        let prec = precision.unwrap_or_else(|| default_precision(self.radix));
        let (int_part, rem) = (&self.num_abs).div_rem(self.denominator);
        let mut int_digits = to_radix_digits(&int_part, self.radix);

        // With the alternate flag the value is displayed exactly whenever its expansion can
        // be resolved: either it is finite, or the repetend is found within the budget.
        if alternate {
            // When detecting repetends we need extra digits to find the cycle.
            let budget = (prec + 1).max(128);
            let exact = match expand_exact(&rem, self.denominator, self.radix, budget) {
                // A finite expansion is exact however long it is.
                None if expansion_terminates(&rem, self.denominator, self.radix) => {
                    let (mut digits, _) =
                        expand_frac(rem.clone(), self.denominator, self.radix, usize::MAX);
                    // the last chunk of digits may extend past the end of the expansion
                    while digits.last() == Some(&0) {
                        digits.pop();
                    }
                    Some((digits, Vec::new()))
                }
                resolved => resolved,
            };

            if let Some((frac_prefix, repetend)) = exact {
                write_digits(f, &int_digits, self.radix, false)?;
                if !repetend.is_empty() {
                    f.write_char('.')?;
                    write_digits(f, &frac_prefix, self.radix, false)?;
                    f.write_char('(')?;
                    write_digits(f, &repetend, self.radix, false)?;
                    f.write_char(')')?;
                } else if !frac_prefix.is_empty() || prec > 0 {
                    // all the digits of the finite expansion, padded up to the precision
                    f.write_char('.')?;
                    write_digits(f, &frac_prefix, self.radix, false)?;
                    for _ in frac_prefix.len()..prec {
                        f.write_char('0')?;
                    }
                }
                return Ok(());
            }
        }

        // Otherwise round the expansion to `prec` fractional digits.
        let (mut frac_digits, rem) = expand_frac(rem, self.denominator, self.radix, prec);
        if rounds_up(&rem, self.denominator)
            && propagate_carry(&mut frac_digits, self.radix)
            && propagate_carry(&mut int_digits, self.radix)
        {
            int_digits.insert(0, 1);
        }

        // Print integer part
        write_digits(f, &int_digits, self.radix, false)?;

        // Print fractional part if needed
        if prec > 0 {
            f.write_char('.')?;
            write_digits(f, &frac_digits, self.radix, false)?;
            // the expansion may have terminated early
            for _ in frac_digits.len()..prec {
                f.write_char('0')?;
            }
        }

        Ok(())
    }

    /// `floor(log_radix(num_abs / denominator))` for a nonzero value.
    fn floor_log(&self) -> isize {
        let base = UBig::from(self.radix);
        let (num, den) = (&self.num_abs, self.denominator);
        // radix^exp <= value  <=>  num >= den * radix^exp
        let reaches = |exp: isize| {
            if exp >= 0 {
                *num >= den * base.pow(exp as usize)
            } else {
                num * base.pow(exp.unsigned_abs()) >= *den
            }
        };

        // the difference of the integer logarithms is off by at most one
        let mut exp = num.ilog(&base) as isize - den.ilog(&base) as isize;
        while !reaches(exp) {
            exp -= 1;
        }
        while reaches(exp + 1) {
            exp += 1;
        }
        exp
    }

    /// Write the magnitude in the scientific notation, with the given precision of the
    /// format specification.
    fn write_scientific(
        &self,
        f: &mut dyn Write,
        precision: Option<usize>,
        exp_char: char,
    ) -> fmt::Result {
        // `e`/`E` are digits in large radices, so the marker is `@` except in decimal
        let exp_marker = if self.radix == 10 { exp_char } else { '@' };

        if self.num_abs.is_zero() {
            f.write_char('0')?;
            let prec = precision.unwrap_or(0);
            if prec > 0 {
                f.write_char('.')?;
                for _ in 0..prec {
                    f.write_char('0')?;
                }
            }
            return write!(f, "{}0", exp_marker);
        }

        let prec = precision.unwrap_or_else(|| default_precision(self.radix));

        // Scale the value by a power of the radix so that its integer part holds exactly
        // the `prec + 1` significant digits, then round on the exact remainder.
        let base = UBig::from(self.radix);
        let mut exp = self.floor_log();
        let shift = exp - prec as isize;
        let (num, den) = if shift >= 0 {
            (self.num_abs.clone(), self.denominator * base.pow(shift as usize))
        } else {
            (&self.num_abs * base.pow(shift.unsigned_abs()), self.denominator.clone())
        };
        let (mut significand, rem) = num.div_rem(&den);
        if rounds_up(&rem, &den) {
            significand += UBig::ONE;
        }
        let mut significand_digits = to_radix_digits(&significand, self.radix);
        if significand_digits.len() > prec + 1 {
            // the rounding carried into a new leading digit (e.g. 9.99 -> 10.0)
            significand_digits.truncate(prec + 1);
            exp += 1;
        }

        // Print: first digit, '.', remaining digits, exp marker, exponent
        let upper = exp_char == 'E';
        write_digit_char(f, significand_digits[0], self.radix, upper)?;
        if prec > 0 {
            f.write_char('.')?;
            write_digits(f, &significand_digits[1..], self.radix, upper)?;
        }

        write!(f, "{}{}", exp_marker, exp)
    }
}

/// Write a slice of digit values in the given radix to the formatter.
fn write_digits(f: &mut dyn Write, digits: &[u8], radix: u8, upper: bool) -> fmt::Result {
    for &d in digits {
        write_digit_char(f, d, radix, upper)?;
    }
    Ok(())
}

/// Write a single digit value (0..radix-1) as a character.
fn write_digit_char(f: &mut dyn Write, digit: u8, _radix: u8, upper: bool) -> fmt::Result {
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

#[cfg(test)]
mod tests {
    use crate::RBig;
    use alloc::format;
    use core::str::FromStr;

    #[test]
    fn test_expanded_terminating() {
        // terminating fractions: the batched path must not change the digits
        let r = RBig::from_str("1/8").unwrap(); // 0.125
        assert_eq!(format!("{:.4}", r.in_expanded(10)), "0.1250");
        assert_eq!(format!("{:.10}", r.in_expanded(10)), "0.1250000000");
        let r = RBig::from_str("1/4").unwrap(); // 0.25
        assert_eq!(format!("{:.3}", r.in_expanded(10)), "0.250");
    }

    #[test]
    fn test_expanded_repeating() {
        let third = RBig::from_str("1/3").unwrap();
        assert_eq!(format!("{:.6}", third.in_expanded(10)), "0.333333");
        assert_eq!(format!("{:#}", third.in_expanded(10)), "0.(3)");
        assert_eq!(format!("{:#}", RBig::from_str("1/7").unwrap().in_expanded(10)), "0.(142857)");
        assert_eq!(format!("{:#}", RBig::from_str("1/6").unwrap().in_expanded(10)), "0.1(6)");
    }

    #[test]
    fn test_expanded_scientific() {
        let third = RBig::from_str("1/3").unwrap();
        assert_eq!(format!("{:.4e}", third.in_expanded(10)), "3.3333e-1");
        let eighth = RBig::from_str("1/8").unwrap();
        assert_eq!(format!("{:.2e}", eighth.in_expanded(10)), "1.25e-1");
    }

    #[test]
    fn test_expanded_binary() {
        // a non-decimal radix exercises batching with a different digits_per_word
        let eighth = RBig::from_str("1/8").unwrap(); // 0.001 binary
        assert_eq!(format!("{:.5}", eighth.in_expanded(2)), "0.00100");
        assert_eq!(format!("{:#}", RBig::from_str("1/3").unwrap().in_expanded(2)), "0.(01)");
    }
}
