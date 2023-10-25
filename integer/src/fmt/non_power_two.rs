//! Format in a non-power-of-two radix.

use super::{digit_writer::DigitWriter, DoubleEnd, InRadixWriter, PreparedForFormatting};
use crate::{
    arch::word::{DoubleWord, Word},
    buffer::Buffer,
    div,
    helper_macros::debug_assert_zero,
    log,
    math::shl_dword,
    ops::DivRem,
    primitive::{double_word, shrink_dword, split_dword},
    radix::{self, Digit},
    repr::{
        Repr,
        TypedReprRef::{self, *},
    },
    shift,
};
use alloc::vec::Vec;
use core::{
    fmt::{self, Formatter},
    mem,
};

/// Format in chunks of CHUNK_LEN * digits_per_word.
const CHUNK_LEN: usize = 16;

impl InRadixWriter<'_> {
    pub fn fmt_non_power_two(&self, f: &mut Formatter) -> fmt::Result {
        debug_assert!(radix::is_radix_valid(self.radix) && !self.radix.is_power_of_two());

        if let RefSmall(dword) = self.magnitude {
            if let Some(word) = shrink_dword(dword) {
                let mut prepared = PreparedWord::new(word, self.radix, 1);
                return self.format_prepared(f, &mut prepared);
            } else {
                let mut prepared = PreparedDword::new(dword, self.radix);
                return self.format_prepared(f, &mut prepared);
            }
        }

        let radix_info = radix::radix_info(self.radix);
        let max_digits = self.magnitude.len() * (radix_info.digits_per_word + 1);
        if max_digits <= CHUNK_LEN * radix_info.digits_per_word {
            let mut prepared = PreparedMedium::new(self.magnitude, self.radix);
            self.format_prepared(f, &mut prepared)
        } else {
            let mut prepared = PreparedLarge::new(self.magnitude, self.radix);
            self.format_prepared(f, &mut prepared)
        }
    }
}

impl DoubleEnd<'_> {
    pub fn fmt_non_power_two(&self, f: &mut Formatter) -> fmt::Result {
        match self.magnitude {
            RefSmall(dword) => {
                // if the number is small enough, we directly prepare all digits
                if let Some(word) = shrink_dword(dword) {
                    let mut prepared = PreparedWord::new(word, 10, 1);
                    let digits = match word {
                        0 => 0,
                        _ => prepared.width(),
                    };
                    self.format_prepared(f, digits, &mut prepared, None)
                } else {
                    let mut prepared = PreparedDword::new(dword, 10);
                    self.format_prepared(f, prepared.width(), &mut prepared, None)
                }
            }
            RefLarge(words) => {
                // otherwise, find the least and most significant digits that fit in a word.
                // for the least significant digits, use the normal remainder algorithm
                // for most significant digits, use the logarithm to find the top bits
                let low_digits = div::rem_by_word(words, radix::RADIX10_INFO.range_per_word);
                let mut prepared_low =
                    PreparedWord::new(low_digits, 10, radix::RADIX10_INFO.digits_per_word);

                let (exp, pow) = log::repr::log_word_base(words, 10);
                let mut pow = pow.into_buffer();
                // there are exp + 1 digits, pow = 10^exp, we need to divide the target number
                // by 10^(exp + 1 - digits_per_word) = pow / 10^(digits_per_word-1) to get the high digits
                debug_assert_zero!(div::div_by_word_in_place(
                    &mut pow,
                    radix::RADIX10_INFO.range_per_word / 10
                ));
                pow.pop_zeros();
                debug_assert!(pow.len() > 1);

                let mut words = Buffer::from(words);
                let (shift, fast_div_pow) = div::normalize(&mut pow);
                let words_top = shift::shl_in_place(&mut words, shift);
                let (words_top, words_lo) = if words_top == 0 {
                    let (words_top, words_lo) = words.split_last_mut().unwrap();
                    (*words_top, words_lo)
                } else {
                    (words_top, &mut words[..])
                };
                let high_digits =
                    div::div_rem_highest_word(words_top, words_lo, &pow, fast_div_pow);
                let mut prepared_high =
                    PreparedWord::new(high_digits, 10, radix::RADIX10_INFO.digits_per_word);

                self.format_prepared(f, exp + 1, &mut prepared_high, Some(&mut prepared_low))
            }
        }
    }
}

/// A `Word` prepared for formatting.
struct PreparedWord {
    // digits[start_index..] actually used.
    digits: [u8; radix::MAX_WORD_DIGITS_NON_POW_2],
    start_index: usize,
}

impl PreparedWord {
    /// Prepare a `Word` for formatting.
    ///
    /// If the input has less digits than min_digits, then zero padding will be appended.
    fn new(mut word: Word, radix: Digit, min_digits: usize) -> PreparedWord {
        debug_assert!(radix::is_radix_valid(radix) && !radix.is_power_of_two());
        let radix_info = radix::radix_info(radix);

        let mut prepared = PreparedWord {
            digits: [0; radix::MAX_WORD_DIGITS_NON_POW_2],
            start_index: radix::MAX_WORD_DIGITS_NON_POW_2,
        };

        let max_start = radix::MAX_WORD_DIGITS_NON_POW_2 - min_digits;
        while prepared.start_index > max_start || word != 0 {
            let (new_word, d) = radix_info.fast_div_radix.div_rem(word, radix as _);
            word = new_word;
            prepared.start_index -= 1;
            prepared.digits[prepared.start_index] = d as u8;
        }

        prepared
    }
}

impl PreparedForFormatting for PreparedWord {
    fn width(&self) -> usize {
        radix::MAX_WORD_DIGITS_NON_POW_2 - self.start_index
    }

    fn write(&mut self, digit_writer: &mut DigitWriter) -> fmt::Result {
        digit_writer.write(&self.digits[self.start_index..])
    }
}

/// A `DoubleWord` prepared for formatting.
struct PreparedDword {
    // digits[start_index..] actually used.
    digits: [u8; radix::MAX_DWORD_DIGITS_NON_POW_2],
    start_index: usize,
}

impl PreparedDword {
    /// Prepare a `DoubleWord` for formatting.
    fn new(dword: DoubleWord, radix: Digit) -> PreparedDword {
        debug_assert!(radix::is_radix_valid(radix) && !radix.is_power_of_two());
        debug_assert!(dword > Word::MAX as DoubleWord);
        let radix_info = radix::radix_info(radix);

        let mut prepared = PreparedDword {
            digits: [0; radix::MAX_DWORD_DIGITS_NON_POW_2],
            start_index: radix::MAX_DWORD_DIGITS_NON_POW_2,
        };

        // extract digits from three parts separated by range_per_word
        let shift = radix_info.range_per_word.leading_zeros();
        let range_div = &radix_info.fast_div_range_per_word;

        let (lo, mid, hi) = shl_dword(dword, shift);
        let (q1, r) = range_div.div_rem_2by1(double_word(mid, hi));
        let (q0, mut p0) = range_div.div_rem_2by1(double_word(lo, r));
        p0 >>= shift;

        // since: hi < 2^shift, range_per_word < 2^(WORD_BITS - shift),
        // we have: q1 = [hi, mid] / range_per_word < 2^(2*shift)
        // meanwhile, for radix 2~36 it can be verified that: shift <= 4 for WORD_BITS = 16 or 32 or 64
        // so q1 * 2^shift < 2^(3*shift) < 2^16, the shifting below won't overflow
        let q = double_word(q0, q1) << shift;
        let (mut p2, mut p1) = range_div.div_rem_2by1(q);
        p1 >>= shift;

        // extract digits from each part
        let mut get_digit = |p: &mut Word| {
            let (new_p, d) = radix_info.fast_div_radix.div_rem(*p, radix as _);
            *p = new_p;
            prepared.start_index -= 1;
            prepared.digits[prepared.start_index] = d as u8;
        };
        for _ in 0..radix_info.digits_per_word {
            get_digit(&mut p0);
        }
        for _ in 0..radix_info.digits_per_word {
            if p1 == 0 && p2 == 0 {
                break;
            }
            get_digit(&mut p1);
        }
        while p2 != 0 {
            get_digit(&mut p2);
        }

        prepared
    }
}

impl PreparedForFormatting for PreparedDword {
    fn width(&self) -> usize {
        radix::MAX_DWORD_DIGITS_NON_POW_2 - self.start_index
    }

    fn write(&mut self, digit_writer: &mut DigitWriter) -> fmt::Result {
        digit_writer.write(&self.digits[self.start_index..])
    }
}

/// A medium number prepared for formatting.
/// Must have no more than CHUNK_LEN * digits_per_word digits.
struct PreparedMedium {
    top_group: PreparedWord,
    // Little endian in groups of digits_per_word.
    low_groups: [Word; CHUNK_LEN],
    num_low_groups: usize,
    radix: Digit,
}

impl PreparedMedium {
    /// Prepare a medium number for formatting.
    fn new(number: TypedReprRef<'_>, radix: Digit) -> PreparedMedium {
        debug_assert!(radix::is_radix_valid(radix) && !radix.is_power_of_two());
        let radix_info = radix::radix_info(radix);

        let (mut buffer, mut buffer_len) = repr_to_chunk_buffer(number);

        let mut low_groups = [0; CHUNK_LEN];
        let mut num_low_groups = 0;

        let shift = radix_info.range_per_word.leading_zeros();
        while buffer_len > 1 {
            let rem = div::fast_div_by_word_in_place(
                &mut buffer[..buffer_len],
                shift,
                radix_info.fast_div_range_per_word,
            );
            low_groups[num_low_groups] = rem;
            num_low_groups += 1;

            while buffer[buffer_len - 1] == 0 {
                buffer_len -= 1;
            }
        }
        debug_assert!(buffer_len == 1);
        PreparedMedium {
            top_group: PreparedWord::new(buffer[0], radix, 1),
            low_groups,
            num_low_groups,
            radix,
        }
    }
}

impl PreparedForFormatting for PreparedMedium {
    fn width(&self) -> usize {
        let radix_info = radix::radix_info(self.radix);
        self.top_group.width() + self.num_low_groups * radix_info.digits_per_word
    }

    fn write(&mut self, digit_writer: &mut DigitWriter) -> fmt::Result {
        let radix_info = radix::radix_info(self.radix);

        self.top_group.write(digit_writer)?;

        for group_word in self.low_groups[..self.num_low_groups].iter().rev() {
            let mut prepared =
                PreparedWord::new(*group_word, self.radix, radix_info.digits_per_word);
            prepared.write(digit_writer)?;
        }
        Ok(())
    }
}

/// A large number prepared for formatting.
struct PreparedLarge {
    top_chunk: PreparedMedium,
    // radix^((digits_per_word * CHUNK_LEN) << i)
    radix_powers: Vec<Repr>,
    // little endian chunks: (i, (digits_per_word * CHUNK_LEN)<<i digit number)
    // decreasing in size, so there is a logarithmic number of them
    big_chunks: Vec<(usize, Repr)>,
    radix: Digit,
}

impl PreparedLarge {
    /// Prepare a medium number for formatting in a non-power-of-2 radix.
    fn new(number: TypedReprRef<'_>, radix: Digit) -> PreparedLarge {
        debug_assert!(radix::is_radix_valid(radix) && !radix.is_power_of_two());
        let radix_info = radix::radix_info(radix);

        let mut radix_powers = Vec::new();
        let mut big_chunks = Vec::new();
        let chunk_power = Repr::from_word(radix_info.range_per_word)
            .as_typed()
            .pow(CHUNK_LEN);
        if chunk_power.as_typed() > number {
            return PreparedLarge {
                top_chunk: PreparedMedium::new(number, radix),
                radix_powers,
                big_chunks,
                radix,
            };
        }

        radix_powers.push(chunk_power);
        loop {
            let prev = radix_powers.last().unwrap();
            // Avoid multiplication if we know prev * prev > number just by looking at lengths.
            if 2 * prev.len() - 1 > number.len() {
                break;
            }

            // 2 * prev.len() is at most 1 larger than number.len().
            let new = prev.as_typed().sqr();
            if new.as_typed() > number {
                break;
            }
            radix_powers.push(new);
        }

        let mut power_iter = radix_powers.iter().enumerate().rev();
        let mut x = {
            let (i, p) = power_iter.next().unwrap();
            let (q, r) = number.div_rem(p.as_typed());
            big_chunks.push((i, r));
            q
        };
        for (i, p) in power_iter {
            if x.as_typed() >= p.as_typed() {
                let (q, r) = x.into_typed().div_rem(p.as_typed());
                big_chunks.push((i, r));
                x = q;
            }
        }

        PreparedLarge {
            top_chunk: PreparedMedium::new(x.as_typed(), radix),
            radix_powers,
            big_chunks,
            radix,
        }
    }

    /// Write (digits_per_word * CHUNK_LEN) << i digits.
    fn write_big_chunk(&self, digit_writer: &mut DigitWriter, i: usize, x: Repr) -> fmt::Result {
        if i == 0 {
            self.write_chunk(digit_writer, x)
        } else {
            let (q, r) = x.into_typed().div_rem(self.radix_powers[i - 1].as_typed());
            self.write_big_chunk(digit_writer, i - 1, q)?;
            self.write_big_chunk(digit_writer, i - 1, r)
        }
    }

    /// Write digits_per_word * CHUNK_LEN digits.
    fn write_chunk(&self, digit_writer: &mut DigitWriter, x: Repr) -> fmt::Result {
        let radix_info = radix::radix_info(self.radix);
        let (mut buffer, mut buffer_len) = repr_to_chunk_buffer(x.as_typed());

        let mut groups = [0; CHUNK_LEN];

        let shift = radix_info.range_per_word.leading_zeros();
        for group in groups.iter_mut() {
            *group = div::fast_div_by_word_in_place(
                &mut buffer[..buffer_len],
                shift,
                radix_info.fast_div_range_per_word,
            );
            while buffer_len != 0 && buffer[buffer_len - 1] == 0 {
                buffer_len -= 1;
            }
        }
        assert_eq!(buffer_len, 0);

        for group in groups.iter().rev() {
            let mut prepared = PreparedWord::new(*group, self.radix, radix_info.digits_per_word);
            prepared.write(digit_writer)?;
        }

        Ok(())
    }
}

impl PreparedForFormatting for PreparedLarge {
    fn width(&self) -> usize {
        let mut num_digits = self.top_chunk.width();
        let radix_info = radix::radix_info(self.radix);
        for (i, _) in &self.big_chunks {
            num_digits += (radix_info.digits_per_word * CHUNK_LEN) << i;
        }
        num_digits
    }

    fn write(&mut self, digit_writer: &mut DigitWriter) -> fmt::Result {
        self.top_chunk.write(digit_writer)?;

        let mut big_chunks = mem::take(&mut self.big_chunks);
        for (i, val) in big_chunks.drain(..).rev() {
            self.write_big_chunk(digit_writer, i, val)?;
        }
        Ok(())
    }
}

fn repr_to_chunk_buffer(x: TypedReprRef<'_>) -> ([Word; CHUNK_LEN], usize) {
    let mut buffer = [0; CHUNK_LEN];

    match x {
        TypedReprRef::RefSmall(dword) => {
            let (lo, hi) = split_dword(dword);
            buffer[0] = lo;
            if hi != 0 {
                buffer[1] = hi;
                (buffer, 2)
            } else {
                (buffer, 1)
            }
        }
        TypedReprRef::RefLarge(words) => {
            let buffer_len = words.len();
            buffer[..buffer_len].copy_from_slice(words);
            (buffer, buffer_len)
        }
    }
}

/// Utility function to print usize in decimal digits
pub fn write_usize_decimals(f: &mut fmt::Formatter, u: usize) -> fmt::Result {
    let mut prepared = PreparedWord::new(u as Word, 10, 1);
    let mut writer = DigitWriter::new(f, radix::DigitCase::NoLetters);
    prepared.write(&mut writer)?;
    writer.flush()
}
