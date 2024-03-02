//! # Integer formatting.
//!
//! Both [UBig] and [IBig] support rust formatter traits ([Display], [LowerHex], etc.). The sign,
//! width, filling and padding options of the formatter are supported for all formatter traits except
//! [Debug]. Different from other formatters, [Debug] will display the least and the most significant
//! digits of the integer, but omitting the middle digits when it's too large. This helps to improve the
//! readability of debug info, and the printing speed. The digit length and bit length will also be displayed
//! when the alternate flag of the formatter is set. (pretty printing)
//!
//! The struct [InRadix] can be used to print the integer in a given radix, which also supporting
//! the common formatter options. But the [Debug] trait is not implemented for [InRadix] yet.
//!

use crate::{
    error::panic_invalid_radix,
    ibig::IBig,
    radix::{self, Digit, DigitCase},
    repr::TypedReprRef,
    ubig::UBig,
    Sign::{self, *},
};
use core::fmt::{
    self, Alignment, Binary, Debug, Display, Formatter, LowerHex, Octal, UpperHex, Write,
};
use digit_writer::DigitWriter;

mod digit_writer;
mod non_power_two;
mod power_two;

pub use radix::{MAX_RADIX, MIN_RADIX};

impl Display for UBig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        InRadixWriter {
            sign: Positive,
            magnitude: self.repr(),
            radix: 10,
            prefix: "",
            digit_case: DigitCase::NoLetters,
        }
        .fmt(f)
    }
}

impl Debug for UBig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        DoubleEnd {
            sign: Positive,
            magnitude: self.repr(),
            verbose: f.alternate(),
        }
        .fmt(f)
    }
}

impl Binary for UBig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        InRadixWriter {
            sign: Positive,
            magnitude: self.repr(),
            radix: 2,
            prefix: if f.alternate() { "0b" } else { "" },
            digit_case: DigitCase::NoLetters,
        }
        .fmt(f)
    }
}

impl Octal for UBig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        InRadixWriter {
            sign: Positive,
            magnitude: self.repr(),
            radix: 8,
            prefix: if f.alternate() { "0o" } else { "" },
            digit_case: DigitCase::NoLetters,
        }
        .fmt(f)
    }
}

impl LowerHex for UBig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        InRadixWriter {
            sign: Positive,
            magnitude: self.repr(),
            radix: 16,
            prefix: if f.alternate() { "0x" } else { "" },
            digit_case: DigitCase::Lower,
        }
        .fmt(f)
    }
}

impl UpperHex for UBig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        InRadixWriter {
            sign: Positive,
            magnitude: self.repr(),
            radix: 16,
            prefix: if f.alternate() { "0x" } else { "" },
            digit_case: DigitCase::Upper,
        }
        .fmt(f)
    }
}

impl Display for IBig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let (sign, magnitude) = self.as_sign_repr();
        InRadixWriter {
            sign,
            magnitude,
            radix: 10,
            prefix: "",
            digit_case: DigitCase::NoLetters,
        }
        .fmt(f)
    }
}

impl Debug for IBig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let (sign, magnitude) = self.as_sign_repr();
        DoubleEnd {
            sign,
            magnitude,
            verbose: f.alternate(),
        }
        .fmt(f)
    }
}

impl Binary for IBig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let (sign, magnitude) = self.as_sign_repr();
        InRadixWriter {
            sign,
            magnitude,
            radix: 2,
            prefix: if f.alternate() { "0b" } else { "" },
            digit_case: DigitCase::NoLetters,
        }
        .fmt(f)
    }
}

impl Octal for IBig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let (sign, magnitude) = self.as_sign_repr();
        InRadixWriter {
            sign,
            magnitude,
            radix: 8,
            prefix: if f.alternate() { "0o" } else { "" },
            digit_case: DigitCase::NoLetters,
        }
        .fmt(f)
    }
}

impl LowerHex for IBig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let (sign, magnitude) = self.as_sign_repr();
        InRadixWriter {
            sign,
            magnitude,
            radix: 16,
            prefix: if f.alternate() { "0x" } else { "" },
            digit_case: DigitCase::Lower,
        }
        .fmt(f)
    }
}

impl UpperHex for IBig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let (sign, magnitude) = self.as_sign_repr();
        InRadixWriter {
            sign,
            magnitude,
            radix: 16,
            prefix: if f.alternate() { "0x" } else { "" },
            digit_case: DigitCase::Upper,
        }
        .fmt(f)
    }
}

impl UBig {
    /// Representation in a given radix.
    ///
    /// # Panics
    ///
    /// Panics if `radix` is not between 2 and 36 inclusive.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(format!("{}", UBig::from(83u8).in_radix(3)), "10002");
    /// assert_eq!(format!("{:+010}", UBig::from(35u8).in_radix(36)), "+00000000z");
    /// ```
    #[inline]
    pub fn in_radix(&self, radix: u32) -> InRadix {
        if !radix::is_radix_valid(radix) {
            panic_invalid_radix(radix);
        }

        InRadix {
            sign: Positive,
            magnitude: self.repr(),
            radix,
        }
    }
}

impl IBig {
    /// Representation in a given radix.
    ///
    /// # Panics
    ///
    /// Panics if `radix` is not between 2 and 36 inclusive.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// assert_eq!(format!("{}", IBig::from(-83).in_radix(3)), "-10002");
    /// assert_eq!(format!("{:010}", IBig::from(-35).in_radix(36)), "-00000000z");
    /// ```
    #[inline]
    pub fn in_radix(&self, radix: u32) -> InRadix {
        if !radix::is_radix_valid(radix) {
            panic_invalid_radix(radix);
        }

        let (sign, magnitude) = self.as_sign_repr();
        InRadix {
            sign,
            magnitude,
            radix,
        }
    }
}

/// Representation of a [UBig] or [IBig] in any radix between [MIN_RADIX] and [MAX_RADIX] inclusive.
///
/// This can be used to format a number in a non-standard radix, by calling [UBig::in_radix] or [IBig::in_radix].
///
/// The default format uses lower-case letters a-z for digits 10-35.
/// The "alternative" format (`{:#}`) uses upper-case letters.
///
/// # Examples
///
/// ```
/// # use dashu_int::{IBig, UBig};
/// assert_eq!(format!("{}", UBig::from(83u8).in_radix(3)), "10002");
/// assert_eq!(format!("{:+010}", UBig::from(35u8).in_radix(36)), "+00000000z");
/// // For bases 2, 8, 10, 16 we don't have to use `InRadix`:
/// assert_eq!(format!("{:x}", UBig::from(3000u32)), "bb8");
/// assert_eq!(format!("{:#X}", IBig::from(-3000)), "-0xBB8");
/// ```
pub struct InRadix<'a> {
    sign: Sign,
    magnitude: TypedReprRef<'a>,
    radix: Digit,
}

impl Display for InRadix<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let digit_case = if self.radix <= 10 {
            DigitCase::NoLetters
        } else if f.alternate() {
            DigitCase::Upper
        } else {
            DigitCase::Lower
        };

        InRadixWriter {
            sign: self.sign,
            magnitude: self.magnitude,
            radix: self.radix,
            prefix: "",
            digit_case,
        }
        .fmt(f)
    }
}

/// Representation in a given radix with a prefix and digit case.
struct InRadixWriter<'a> {
    sign: Sign,
    magnitude: TypedReprRef<'a>,
    radix: Digit,
    prefix: &'static str,
    digit_case: DigitCase,
}

/// Representation for printing only head and tail of the number, only decimal is supported
struct DoubleEnd<'a> {
    sign: Sign,
    magnitude: TypedReprRef<'a>,
    verbose: bool,
}

impl InRadixWriter<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.radix.is_power_of_two() {
            self.fmt_power_two(f)
        } else {
            self.fmt_non_power_two(f)
        }
    }

    /// Format using a `PreparedForFormatting`.
    fn format_prepared(
        &self,
        f: &mut Formatter,
        prepared: &mut dyn PreparedForFormatting,
    ) -> fmt::Result {
        let mut width = prepared.width();

        // Adding sign and prefix to width will not overflow, because Buffer::MAX_CAPACITY leaves
        // (WORD_BITS - 1) spare bits before we would hit overflow.
        let sign = if self.sign == Negative {
            "-"
        } else if f.sign_plus() {
            "+"
        } else {
            ""
        };
        // In bytes, but it's OK because everything is ASCII.
        width += sign.len() + self.prefix.len();

        let mut write_digits = |f| {
            let mut digit_writer = DigitWriter::new(f, self.digit_case);
            prepared.write(&mut digit_writer)?;
            digit_writer.flush()
        };

        match f.width() {
            None => {
                f.write_str(sign)?;
                f.write_str(self.prefix)?;
                write_digits(f)?
            }
            Some(min_width) => {
                if width >= min_width {
                    f.write_str(sign)?;
                    f.write_str(self.prefix)?;
                    write_digits(f)?;
                } else if f.sign_aware_zero_pad() {
                    f.write_str(sign)?;
                    f.write_str(self.prefix)?;
                    for _ in 0..min_width - width {
                        f.write_char('0')?;
                    }
                    write_digits(f)?;
                } else {
                    let left_pad = match f.align() {
                        Some(Alignment::Left) => 0,
                        Some(Alignment::Right) | None => min_width - width,
                        Some(Alignment::Center) => (min_width - width) / 2,
                    };
                    let fill = f.fill();
                    for _ in 0..left_pad {
                        f.write_char(fill)?;
                    }
                    f.write_str(sign)?;
                    f.write_str(self.prefix)?;
                    write_digits(f)?;
                    for _ in left_pad..min_width - width {
                        f.write_char(fill)?;
                    }
                }
            }
        }

        Ok(())
    }
}

impl DoubleEnd<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.fmt_non_power_two(f)
    }

    /// Format using a `PreparedForFormatting`.
    fn format_prepared(
        &self,
        f: &mut Formatter,
        digits: usize,
        prepared_high: &mut dyn PreparedForFormatting,
        prepared_low: Option<&mut dyn PreparedForFormatting>,
    ) -> fmt::Result {
        let sign = if self.sign == Negative {
            "-"
        } else if f.sign_plus() {
            "+"
        } else {
            ""
        };
        f.write_str(sign)?;

        let mut digit_writer = DigitWriter::new(f, DigitCase::NoLetters);
        prepared_high.write(&mut digit_writer)?;
        digit_writer.flush()?;

        if let Some(low) = prepared_low {
            f.write_str("..")?;

            let mut digit_writer = DigitWriter::new(f, DigitCase::NoLetters);
            low.write(&mut digit_writer)?;
            digit_writer.flush()?;
        }

        if self.verbose {
            f.write_str(" (digits: ")?;
            non_power_two::write_usize_decimals(f, digits)?;
            f.write_str(", bits: ")?;
            non_power_two::write_usize_decimals(f, self.magnitude.bit_len())?;
            f.write_str(")")?;
        }

        Ok(())
    }
}

/// Trait for state of a partially-formatted [UBig].
///
/// The state must be such the width (number of digits) is already known.
trait PreparedForFormatting {
    /// Returns the number of characters that will be written.
    fn width(&self) -> usize;

    /// Write to a stream.
    fn write(&mut self, digit_writer: &mut DigitWriter) -> fmt::Result;
}
