//! Format in a power-of-two radix.

use super::{digit_writer::DigitWriter, InRadixWriter, PreparedForFormatting};
use crate::{
    arch::word::{DoubleWord, Word},
    math,
    primitive::{shrink_dword, DWORD_BITS_USIZE, WORD_BITS, WORD_BITS_USIZE},
    radix::{self, Digit},
    repr::TypedReprRef::*,
};
use core::fmt::{self, Formatter};

impl InRadixWriter<'_> {
    /// Radix must be a power of 2.
    pub fn fmt_power_two(&self, f: &mut Formatter) -> fmt::Result {
        debug_assert!(radix::is_radix_valid(self.radix) && self.radix.is_power_of_two());

        match self.magnitude {
            RefSmall(dword) => {
                if let Some(word) = shrink_dword(dword) {
                    let mut prepared = PreparedWord::new(word, self.radix);
                    self.format_prepared(f, &mut prepared)
                } else {
                    let mut prepared = PreparedDword::new(dword, self.radix);
                    self.format_prepared(f, &mut prepared)
                }
            }
            RefLarge(words) => {
                let mut prepared = PreparedLarge::new(words, self.radix);
                self.format_prepared(f, &mut prepared)
            }
        }
    }
}

/// A `Word` prepared for formatting.
struct PreparedWord {
    word: Word,
    log_radix: u32,
    width: usize,
}

impl PreparedWord {
    /// Prepare a `Word` for formatting.
    fn new(word: Word, radix: Digit) -> PreparedWord {
        debug_assert!(radix::is_radix_valid(radix) && radix.is_power_of_two());
        let log_radix = radix.trailing_zeros();
        let width = math::ceil_div(math::bit_len(word), log_radix).max(1) as usize;

        PreparedWord {
            word,
            log_radix,
            width,
        }
    }
}

impl PreparedForFormatting for PreparedWord {
    fn width(&self) -> usize {
        self.width
    }

    fn write(&mut self, digit_writer: &mut DigitWriter) -> fmt::Result {
        let mask: Word = math::ones_word(self.log_radix);
        let mut digits = [0; WORD_BITS_USIZE];
        for idx in 0..self.width {
            let digit = ((self.word >> (idx as u32 * self.log_radix)) & mask) as u8;
            digits[self.width - 1 - idx] = digit;
        }
        digit_writer.write(&digits[..self.width])
    }
}

/// A large number prepared for formatting.
struct PreparedDword {
    dword: DoubleWord,
    log_radix: u32,
    width: usize,
}

impl PreparedDword {
    /// Prepare a `DoubleWord` for formatting.
    fn new(dword: DoubleWord, radix: Digit) -> PreparedDword {
        debug_assert!(dword > Word::MAX as DoubleWord);
        debug_assert!(radix::is_radix_valid(radix) && radix.is_power_of_two());
        let log_radix = radix.trailing_zeros();
        let width = math::ceil_div(math::bit_len(dword), log_radix).max(1) as usize;

        PreparedDword {
            dword,
            log_radix,
            width,
        }
    }
}

impl PreparedForFormatting for PreparedDword {
    fn width(&self) -> usize {
        self.width
    }

    fn write(&mut self, digit_writer: &mut DigitWriter) -> fmt::Result {
        let mask: DoubleWord = math::ones_dword(self.log_radix);
        let mut digits = [0; DWORD_BITS_USIZE];
        for idx in 0..self.width {
            let digit = ((self.dword >> (idx as u32 * self.log_radix)) & mask) as u8;
            digits[self.width - 1 - idx] = digit;
        }
        digit_writer.write(&digits[..self.width])
    }
}

/// A large number prepared for formatting.
struct PreparedLarge<'a> {
    words: &'a [Word],
    log_radix: u32,
    width: usize,
}

impl PreparedLarge<'_> {
    /// Prepare a large number for formatting.
    fn new(words: &[Word], radix: Digit) -> PreparedLarge {
        debug_assert!(radix::is_radix_valid(radix) && radix.is_power_of_two());
        let log_radix = radix.trailing_zeros();

        // No overflow because words.len() * WORD_BITS <= usize::MAX for
        // words.len() <= Buffer::MAX_CAPACITY.
        let width = math::ceil_div(
            words.len() * WORD_BITS_USIZE - words.last().unwrap().leading_zeros() as usize,
            log_radix as usize,
        )
        .max(1);

        PreparedLarge {
            words,
            log_radix,
            width,
        }
    }
}

impl PreparedForFormatting for PreparedLarge<'_> {
    fn width(&self) -> usize {
        self.width
    }

    fn write(&mut self, digit_writer: &mut DigitWriter) -> fmt::Result {
        let mask: Word = math::ones_word(self.log_radix);

        let mut it = self.words.iter().rev();
        let mut word = it.next().unwrap();
        let mut bits = (self.width * self.log_radix as usize
            - (self.words.len() - 1) * WORD_BITS_USIZE) as u32;

        loop {
            let digit;
            if bits < self.log_radix {
                match it.next() {
                    Some(w) => {
                        let extra_bits = self.log_radix - bits;
                        bits = WORD_BITS - extra_bits;
                        digit = ((word << extra_bits | w >> bits) & mask) as u8;
                        word = w;
                    }
                    None => break,
                }
            } else {
                bits -= self.log_radix;
                digit = ((word >> bits) & mask) as u8;
            }
            // digit_writer.write(&[digit])?;
            digit_writer.write_digit(digit)?;
        }
        debug_assert_eq!(bits, 0);
        Ok(())
    }
}
