//! Buffered raw digit -> ASCII conversion.

use crate::{arch, math, radix::DigitCase};
use core::{convert::TryInto, fmt, str};

/// Minimum buffer length.
const BUFFER_LEN_MIN: usize = 32;

const BUFFER_LEN: usize = math::round_up_usize(BUFFER_LEN_MIN, arch::digits::DIGIT_CHUNK_LEN);

/// DigitWriter allows writing raw digits and turns them into ASCII.
pub struct DigitWriter<'a> {
    buffer: [u8; BUFFER_LEN],
    buffer_len: usize,
    digit_case: DigitCase,
    writer: &'a mut dyn fmt::Write,
}

impl<'a> DigitWriter<'a> {
    pub fn new(writer: &'a mut dyn fmt::Write, digit_case: DigitCase) -> DigitWriter {
        DigitWriter {
            buffer: [0; BUFFER_LEN],
            buffer_len: 0,
            digit_case,
            writer,
        }
    }

    /// buf must contain values 0-35, or 0-9 if digit_case is NoLetters.
    pub fn write(&mut self, mut buf: &[u8]) -> fmt::Result {
        while !buf.is_empty() {
            let len = buf.len().min(BUFFER_LEN - self.buffer_len);
            let (buf_chunk, buf_remainder) = buf.split_at(len);
            buf = buf_remainder;
            self.buffer[self.buffer_len..self.buffer_len + len].copy_from_slice(buf_chunk);
            self.buffer_len += len;
            if self.buffer_len == BUFFER_LEN {
                self.flush()?;
            }
        }
        Ok(())
    }

    pub fn write_digit(&mut self, digit: u8) -> fmt::Result {
        self.buffer[self.buffer_len] = digit;
        self.buffer_len += 1;
        if self.buffer_len == BUFFER_LEN {
            self.flush()?;
        }
        Ok(())
    }

    /// Must call flush to make sure all the data is written.
    pub fn flush(&mut self) -> fmt::Result {
        let buffer_len_rounded = math::round_up(self.buffer_len, arch::digits::DIGIT_CHUNK_LEN);
        self.buffer[self.buffer_len..buffer_len_rounded].fill(0);
        for chunk in
            self.buffer[..buffer_len_rounded].chunks_exact_mut(arch::digits::DIGIT_CHUNK_LEN)
        {
            arch::digits::digit_chunk_raw_to_ascii(chunk.try_into().unwrap(), self.digit_case);
        }
        let b = &self.buffer[..self.buffer_len];
        // Safe because the buffer contains only ASCII characters 0-9, a-z, A-Z.
        let s = unsafe { str::from_utf8_unchecked(b) };
        self.writer.write_str(s)?;
        self.buffer_len = 0;
        Ok(())
    }
}
