//! Conversions between types.

use crate::{
    add, arch::word::{DoubleWord, Word}, buffer::Buffer, helper_macros::debug_assert_zero, ibig::IBig, math, primitive::{self, PrimitiveSigned, PrimitiveUnsigned, DWORD_BITS_USIZE, DWORD_BYTES, WORD_BITS, WORD_BITS_USIZE, WORD_BYTES}, repr::{
        Repr,
        TypedReprRef::{self, *},
    }, shift, ubig::UBig, Sign::*
};
use alloc::{boxed::Box, vec::Vec, vec};
use static_assertions::const_assert;
use core::convert::{TryFrom, TryInto};
use dashu_base::{
    Approximation::{self, *},
    BitTest, ConversionError, FloatEncoding, PowerOfTwo, Sign,
};

impl Default for UBig {
    /// Default value: 0.
    #[inline]
    fn default() -> UBig {
        UBig::ZERO
    }
}

impl Default for IBig {
    /// Default value: 0.
    #[inline]
    fn default() -> IBig {
        IBig::ZERO
    }
}

pub(crate) fn words_to_le_bytes<const FLIP: bool>(words: &[Word]) -> Vec<u8> {
    debug_assert!(!words.is_empty());

    let n = words.len();
    let last = words[n - 1];
    let skip_last_bytes = last.leading_zeros() as usize / 8;
    let mut bytes = Vec::with_capacity(n * WORD_BYTES - skip_last_bytes);
    for word in &words[..n - 1] {
        let word = if FLIP { !*word } else { *word };
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    let last = if FLIP { !last } else { last };
    let last_bytes = last.to_le_bytes();
    bytes.extend_from_slice(&last_bytes[..WORD_BYTES - skip_last_bytes]);
    bytes
}

fn words_to_be_bytes<const FLIP: bool>(words: &[Word]) -> Vec<u8> {
    debug_assert!(!words.is_empty());

    let n = words.len();
    let last = words[n - 1];
    let skip_last_bytes = last.leading_zeros() as usize / 8;
    let mut bytes = Vec::with_capacity(n * WORD_BYTES - skip_last_bytes);
    let last = if FLIP { !last } else { last };
    let last_bytes = last.to_be_bytes();
    bytes.extend_from_slice(&last_bytes[skip_last_bytes..]);
    for word in words[..n - 1].iter().rev() {
        let word = if FLIP { !*word } else { *word };
        bytes.extend_from_slice(&word.to_be_bytes());
    }
    bytes
}

/// Convert a integer into an array of chunks by bit chunking
/// 
/// Requirements:
/// - chunks_out.len() tightly fits all chunks.
/// - All words in chunks_out must has enough length (i.e. ceil(chunk_bits / WORD_BITS))
fn words_to_chunks(words: &[Word], chunks_out: &mut [&mut [Word]], chunk_bits: usize) {
    assert!(words.len() > 0);

    if chunk_bits % WORD_BITS_USIZE == 0 {
        // shortcut for word aligned chunks
        let words_per_chunk = chunk_bits / WORD_BITS_USIZE;
        for (i, chunk_out) in chunks_out.iter_mut().enumerate() {
            let start_pos = i * words_per_chunk;
            let end_pos = start_pos + words_per_chunk;
            chunk_out[..end_pos-start_pos].copy_from_slice(&words[start_pos..end_pos]);
        }
    } else {
        let bit_len = words.len() * WORD_BITS_USIZE - words.last().unwrap().leading_zeros() as usize;
        for (i, chunk_out) in chunks_out.iter_mut().enumerate() {
            let start = i * chunk_bits;
            let end = bit_len.min(start + chunk_bits);
            debug_assert!(start < end); // make sure that there is no empty chunk
    
            let (start_pos, end_pos) = (start / WORD_BITS_USIZE, end / WORD_BITS_USIZE);
            let end_bits = (end % WORD_BITS_USIZE) as u32;
            let len;
            if end_bits != 0 {
                len = end_pos - start_pos;
                chunk_out[..=len].copy_from_slice(&words[start_pos..=end_pos]);
                chunk_out[len] &= math::ones_word(end_bits);
            } else {
                len = end_pos - start_pos - 1;
                chunk_out[..=len].copy_from_slice(&words[start_pos..end_pos]);
            }
            shift::shr_in_place(&mut chunk_out[..=len], (start % WORD_BITS_USIZE) as u32);
        }
    }
}

/// Convert chunks to a single integer by shifting and adding.
/// 
/// Requirements:
/// - words_out must have enough length
/// - buffer must have enough length: buffer.len() > max(chunk.len()) for chunk in chunks
fn chunks_to_words(words_out: &mut [Word], chunks: &[&[Word]], chunk_bits: usize, buffer: &mut [Word]) {
    assert!(chunks.len() > 0);
    for (i, chunk) in chunks.iter().enumerate() {
        let shift = i * chunk_bits;
        buffer[..chunk.len()].copy_from_slice(chunk);
        buffer[chunk.len()] = 0;
        shift::shl_in_place(&mut buffer[..=chunk.len()], (shift % WORD_BITS_USIZE) as u32);
        debug_assert_zero!(add::add_in_place(&mut words_out[shift / WORD_BITS_USIZE..], &buffer[..=chunk.len()]));
    }
}

impl TypedReprRef<'_> {
    fn to_le_bytes(self) -> Vec<u8> {
        match self {
            RefSmall(x) => {
                let bytes = x.to_le_bytes();
                let skip_bytes = x.leading_zeros() as usize / 8;
                bytes[..DWORD_BYTES - skip_bytes].into()
            }
            RefLarge(words) => words_to_le_bytes::<false>(words),
        }
    }

    fn to_signed_le_bytes(self, negate: bool) -> Vec<u8> {
        // make sure to return empty for zero
        if let RefSmall(v) = self {
            if v == 0 {
                return Vec::new();
            }
        }

        let mut bytes = if negate {
            match self {
                RefSmall(x) => {
                    let bytes = (!x + 1).to_le_bytes();
                    let skip_bytes = x.leading_zeros() as usize / 8;
                    bytes[..DWORD_BYTES - skip_bytes].into()
                }
                RefLarge(words) => {
                    let mut buffer = Buffer::from(words);
                    debug_assert_zero!(add::sub_one_in_place(&mut buffer));
                    words_to_le_bytes::<true>(&buffer)
                }
            }
        } else {
            self.to_le_bytes()
        };

        let leading_zeros = match self {
            RefSmall(x) => x.leading_zeros(),
            RefLarge(words) => words.last().unwrap().leading_zeros(),
        };
        if leading_zeros % 8 == 0 {
            // add extra byte representing the sign, because the top bit is used
            bytes.push(if negate { 0xff } else { 0 });
        }

        bytes
    }

    fn to_be_bytes(self) -> Vec<u8> {
        match self {
            RefSmall(x) => {
                let bytes = x.to_be_bytes();
                let skip_bytes = x.leading_zeros() as usize / 8;
                bytes[skip_bytes..].into()
            }
            RefLarge(words) => words_to_be_bytes::<false>(words),
        }
    }

    fn to_signed_be_bytes(self, negate: bool) -> Vec<u8> {
        // make sure to return empty for zero
        if let RefSmall(v) = self {
            if v == 0 {
                return Vec::new();
            }
        }

        let mut bytes = if negate {
            match self {
                RefSmall(x) => {
                    let bytes = (!x + 1).to_be_bytes();
                    let skip_bytes = x.leading_zeros() as usize / 8;
                    bytes[skip_bytes..].into()
                }
                RefLarge(words) => {
                    let mut buffer = Buffer::from(words);
                    debug_assert_zero!(add::sub_one_in_place(&mut buffer));
                    words_to_be_bytes::<true>(&buffer)
                }
            }
        } else {
            self.to_be_bytes()
        };

        let leading_zeros = match self {
            RefSmall(x) => x.leading_zeros(),
            RefLarge(words) => words.last().unwrap().leading_zeros(),
        };
        if leading_zeros % 8 == 0 {
            // add extra byte representing the sign, because the top bit is used
            bytes.insert(0, if negate { 0xff } else { 0 });
        }

        bytes
    }

    fn to_chunks(self, chunk_bits: usize) -> Vec<Repr> {
        assert!(chunk_bits > 0);
        let chunk_count = math::ceil_div(self.bit_len(), chunk_bits);

        match self {
            RefSmall(x) => match chunk_count {
                0 => Vec::new(),
                1 => vec![Repr::from_dword(x)],
                n => {
                    const_assert!(u8::MAX as usize > DWORD_BITS_USIZE);
                    let mut buffers = Vec::with_capacity(n);
                    let chunk_bits = chunk_bits as u8; // chunk has at most DWORD_BITS bits, otherwise n <= 1
                    for i in 0..n as u8 {
                        let chunk = (x >> (i * chunk_bits)) & math::ones_dword(chunk_bits as _);
                        buffers.push(Repr::from_dword(chunk));
                    }
                    buffers
                }
            },
            RefLarge(words) =>{                
                let mut buffers = Vec::<Buffer>::new();
                let word_per_chunk = math::ceil_div(chunk_bits, WORD_BITS_USIZE);
                buffers.resize_with(chunk_count, || {
                    // allocate an extra word for shifting
                    let mut buf: Buffer = Buffer::allocate(word_per_chunk + 1);
                    buf.push_zeros(word_per_chunk + 1);
                    buf
                });
                let mut buffer_refs: Box<[&mut [Word]]> = buffers.iter_mut().map(|buf| buf.as_mut()).collect();
                words_to_chunks(words, &mut buffer_refs, chunk_bits);
                buffers.into_iter().map(|buf| Repr::from_buffer(buf)).collect()
            } 
        }
    }
}

impl Repr {
    fn from_le_bytes(bytes: &[u8]) -> Self {
        if bytes.len() <= DWORD_BYTES {
            // fast path
            Self::from_dword(primitive::dword_from_le_bytes_partial::<false>(bytes))
        } else {
            // slow path
            Self::from_le_bytes_large::<false>(bytes)
        }
    }

    fn from_signed_le_bytes(bytes: &[u8]) -> Self {
        if let Some(v) = bytes.last() {
            if *v < 0x80 {
                return Self::from_le_bytes(bytes);
            }
        } else {
            return Self::zero();
        }

        // negative
        let repr = if bytes.len() <= DWORD_BYTES {
            // fast path
            let dword = primitive::dword_from_le_bytes_partial::<true>(bytes);
            Self::from_dword(!dword + 1)
        } else {
            // slow path
            Self::from_le_bytes_large::<true>(bytes)
        };
        repr.with_sign(Sign::Negative)
    }

    fn from_le_bytes_large<const NEG: bool>(bytes: &[u8]) -> Self {
        debug_assert!(bytes.len() >= DWORD_BYTES);
        let mut buffer = Buffer::allocate((bytes.len() - 1) / WORD_BYTES + 1);
        let mut chunks = bytes.chunks_exact(WORD_BYTES);
        for chunk in &mut chunks {
            let word = Word::from_le_bytes(chunk.try_into().unwrap());
            buffer.push(if NEG { !word } else { word });
        }
        if !chunks.remainder().is_empty() {
            let word = primitive::word_from_le_bytes_partial::<NEG>(chunks.remainder());
            buffer.push(if NEG { !word } else { word });
        }
        if NEG {
            debug_assert_zero!(add::add_one_in_place(&mut buffer));
        }
        Self::from_buffer(buffer)
    }

    fn from_be_bytes(bytes: &[u8]) -> Self {
        if bytes.len() <= DWORD_BYTES {
            Self::from_dword(primitive::dword_from_be_bytes_partial::<false>(bytes))
        } else {
            // slow path
            Self::from_be_bytes_large::<false>(bytes)
        }
    }

    fn from_signed_be_bytes(bytes: &[u8]) -> Self {
        if let Some(v) = bytes.first() {
            if *v < 0x80 {
                return Self::from_be_bytes(bytes);
            }
        } else {
            return Self::zero();
        }

        // negative
        let repr = if bytes.len() <= DWORD_BYTES {
            // fast path
            let dword = primitive::dword_from_be_bytes_partial::<true>(bytes);
            Self::from_dword(!dword + 1)
        } else {
            // slow path
            Self::from_be_bytes_large::<true>(bytes)
        };
        repr.with_sign(Sign::Negative)
    }

    fn from_be_bytes_large<const NEG: bool>(bytes: &[u8]) -> Self {
        debug_assert!(bytes.len() >= DWORD_BYTES);
        let mut buffer = Buffer::allocate((bytes.len() - 1) / WORD_BYTES + 1);
        let mut chunks = bytes.rchunks_exact(WORD_BYTES);
        for chunk in &mut chunks {
            let word = Word::from_be_bytes(chunk.try_into().unwrap());
            buffer.push(if NEG { !word } else { word });
        }
        if !chunks.remainder().is_empty() {
            let word = primitive::word_from_be_bytes_partial::<NEG>(chunks.remainder());
            buffer.push(if NEG { !word } else { word });
        }
        if NEG {
            debug_assert_zero!(add::add_one_in_place(&mut buffer));
        }
        Self::from_buffer(buffer)
    }

    fn from_chunks(chunks: &[&[Word]], chunk_bits: usize) -> Self {
        if let Some(max_len) = chunks.iter().map(|words| words.len()).max() {
            // allocate an extra word for shifting
            let result_len = max_len + (chunks.len() - 1) * chunk_bits + 1;
            let mut result = Buffer::allocate(result_len);
            result.push_zeros(result_len);
            let mut buffer = Buffer::allocate_exact(max_len + 1);
            buffer.push_zeros(max_len + 1);

            chunks_to_words(&mut result, chunks, chunk_bits, &mut buffer);
            Self::from_buffer(result)
        } else {
            Self::zero()
        }
    }
}

impl UBig {
    /// Construct from little-endian bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from_le_bytes(&[3, 2, 1]), UBig::from(0x010203u32));
    /// ```
    #[inline]
    pub fn from_le_bytes(bytes: &[u8]) -> UBig {
        UBig(Repr::from_le_bytes(bytes))
    }

    /// Construct from big-endian bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from_be_bytes(&[1, 2, 3]), UBig::from(0x010203u32));
    /// ```
    #[inline]
    pub fn from_be_bytes(bytes: &[u8]) -> UBig {
        UBig(Repr::from_be_bytes(bytes))
    }

    /// Return little-endian bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert!(UBig::ZERO.to_le_bytes().is_empty());
    /// assert_eq!(*UBig::from(0x010203u32).to_le_bytes(), [3, 2, 1]);
    /// ```
    pub fn to_le_bytes(&self) -> Box<[u8]> {
        self.repr().to_le_bytes().into_boxed_slice()
    }

    /// Return big-endian bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert!(UBig::ZERO.to_be_bytes().is_empty());
    /// assert_eq!(*UBig::from(0x010203u32).to_be_bytes(), [1, 2, 3]);
    /// ```
    pub fn to_be_bytes(&self) -> Box<[u8]> {
        self.repr().to_be_bytes().into_boxed_slice()
    }

    /// Reconstruct an integer from a group of bit chunks.
    /// 
    /// Denote the chunks as C_i, then this function calculates sum(C_i * 2^(i * chunk_bits))
    /// for i from 0 to len(chunks) - 1.
    /// 
    /// Note that it's allowed for each chunk to have more bits than chunk_bits, which is different
    /// from the [UBig::to_chunks] method.
    /// 
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(
    ///     UBig::from_chunks([0x3u8.into(), 0x2u8.into(), 0x1u8.into()].iter(), 8),
    ///     UBig::from(0x010203u32)
    /// );
    /// ```
    /// 
    /// # Panics
    /// 
    /// Panics if chunk_bits is zero.
    #[inline]
    pub fn from_chunks<'a, I: Iterator<Item = &'a UBig>>(chunks: I, chunk_bits: usize) -> Self {
        let chunks: Box<_> = chunks.into_iter().map(|u| u.as_words()).collect();
        Self(Repr::from_chunks(&chunks, chunk_bits))
    }

    /// Slice the integer into group of bit chunks from the least significant end to the most
    /// significant end. Each chunk is represented as an integer, containing a subset of the
    /// bits in the source integer.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert!(UBig::ZERO.to_chunks(1).is_empty());
    /// assert_eq!(*UBig::from(0x010203u32).to_chunks(8),
    ///     [0x3u8.into(), 0x2u8.into(), 0x1u8.into()]);
    /// ```
    /// 
    /// # Panics
    /// 
    /// Panics if chunk_bits is zero.
    #[inline]
    pub fn to_chunks(&self, chunk_bits: usize) -> Box<[UBig]> {
        self.repr().to_chunks(chunk_bits).into_iter().map(|r| UBig(r)).collect()
    }

    /// Convert to f32.
    ///
    /// Round to nearest, breaking ties to even last bit. The returned approximation
    /// is exact if the integer is exactly representable by f32, otherwise the error
    /// field of the approximation contains the sign of `result - self`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from(134u8).to_f32().value(), 134.0f32);
    /// ```
    #[inline]
    pub fn to_f32(&self) -> Approximation<f32, Sign> {
        self.repr().to_f32()
    }

    /// Convert to f64.
    ///
    /// Round to nearest, breaking ties to even last bit. The returned approximation
    /// is exact if the integer is exactly representable by f64, otherwise the error
    /// field of the approximation contains the sign of `result - self`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from(134u8).to_f64().value(), 134.0f64);
    /// ```
    #[inline]
    pub fn to_f64(&self) -> Approximation<f64, Sign> {
        self.repr().to_f64()
    }

    /// Regard the number as a [IBig] number and return a reference of [IBig] type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{IBig, UBig};
    /// assert_eq!(UBig::from(123u8).as_ibig(), &IBig::from(123));
    /// ```
    #[inline]
    pub const fn as_ibig(&self) -> &IBig {
        // SAFETY: UBig and IBig are both transparent wrapper around the Repr type.
        //         This conversion is only available for immutable references, so that
        //         the sign will not be messed up.
        unsafe { core::mem::transmute(self) }
    }
}

impl IBig {
    /// Construct from signed little-endian bytes.
    ///
    /// The negative number must be represented in a two's complement format, assuming
    /// the top bits are all ones. The number is assumed negative when the top bit of
    /// the top byte is set.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// assert_eq!(IBig::from_le_bytes(&[1, 2, 0xf3]), IBig::from(0xfff30201u32 as i32));
    /// ```
    #[inline]
    pub fn from_le_bytes(bytes: &[u8]) -> IBig {
        IBig(Repr::from_signed_le_bytes(bytes))
    }

    /// Construct from big-endian bytes.
    ///
    /// The negative number must be represented in a two's complement format, assuming
    /// the top bits are all ones. The number is assumed negative when the top bit of
    /// the top byte is set.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// assert_eq!(IBig::from_be_bytes(&[0xf3, 2, 1]), IBig::from(0xfff30201u32 as i32));
    /// ```
    #[inline]
    pub fn from_be_bytes(bytes: &[u8]) -> IBig {
        IBig(Repr::from_signed_be_bytes(bytes))
    }

    /// Return little-endian bytes.
    ///
    /// The negative number will be represented in a two's complement format
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// assert!(IBig::ZERO.to_le_bytes().is_empty());
    /// assert_eq!(*IBig::from(0xfff30201u32 as i32).to_le_bytes(), [1, 2, 0xf3]);
    /// ```
    #[inline]
    pub fn to_le_bytes(&self) -> Box<[u8]> {
        let (sign, repr) = self.as_sign_repr();
        repr.to_signed_le_bytes(sign.into()).into_boxed_slice()
    }

    /// Return big-endian bytes.
    ///
    /// The negative number will be represented in a two's complement format
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// assert!(IBig::ZERO.to_be_bytes().is_empty());
    /// assert_eq!(*IBig::from(0xfff30201u32 as i32).to_be_bytes(), [0xf3, 2, 1]);
    /// ```
    pub fn to_be_bytes(&self) -> Box<[u8]> {
        let (sign, repr) = self.as_sign_repr();
        repr.to_signed_be_bytes(sign.into()).into_boxed_slice()
    }

    /// Convert to f32.
    ///
    /// Round to nearest, breaking ties to even last bit. The returned approximation
    /// is exact if the integer is exactly representable by f32, otherwise the error
    /// field of the approximation contains the sign of `result - self`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// assert_eq!(IBig::from(-134).to_f32().value(), -134.0f32);
    /// ```
    #[inline]
    pub fn to_f32(&self) -> Approximation<f32, Sign> {
        let (sign, mag) = self.as_sign_repr();
        match mag.to_f32() {
            Exact(val) => Exact(sign * val),
            Inexact(val, diff) => Inexact(sign * val, sign * diff),
        }
    }

    /// Convert to f64.
    ///
    /// Round to nearest, breaking ties to even last bit. The returned approximation
    /// is exact if the integer is exactly representable by f64, otherwise the error
    /// field of the approximation contains the sign of `result - self`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// assert_eq!(IBig::from(-134).to_f64().value(), -134.0f64);
    /// ```
    #[inline]
    pub fn to_f64(&self) -> Approximation<f64, Sign> {
        let (sign, mag) = self.as_sign_repr();
        match mag.to_f64() {
            Exact(val) => Exact(sign * val),
            Inexact(val, diff) => Inexact(sign * val, sign * diff),
        }
    }

    /// Regard the number as a [UBig] number and return a reference of [UBig] type.
    ///
    /// The conversion is only successful when the number is positive
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{IBig, UBig};
    /// assert_eq!(IBig::from(123).as_ubig(), Some(&UBig::from(123u8)));
    /// assert_eq!(IBig::from(-123).as_ubig(), None);
    /// ```
    #[inline]
    pub const fn as_ubig(&self) -> Option<&UBig> {
        match self.sign() {
            Sign::Positive => {
                // SAFETY: UBig and IBig are both transparent wrapper around the Repr type.
                //         This conversion is only available for immutable references and
                //         positive numbers, so that the sign will not be messed up.
                unsafe { Some(core::mem::transmute(self)) }
            }
            Sign::Negative => None,
        }
    }
}

macro_rules! ubig_unsigned_conversions {
    ($($t:ty)*) => {$(
        impl From<$t> for UBig {
            #[inline]
            fn from(value: $t) -> UBig {
                UBig::from_unsigned(value)
            }
        }

        impl TryFrom<UBig> for $t {
            type Error = ConversionError;

            #[inline]
            fn try_from(value: UBig) -> Result<$t, ConversionError> {
                value.try_to_unsigned()
            }
        }

        impl TryFrom<&UBig> for $t {
            type Error = ConversionError;

            #[inline]
            fn try_from(value: &UBig) -> Result<$t, ConversionError> {
                value.try_to_unsigned()
            }
        }
    )*};
}
ubig_unsigned_conversions!(u8 u16 u32 u64 u128 usize);

impl From<bool> for UBig {
    #[inline]
    fn from(b: bool) -> UBig {
        u8::from(b).into()
    }
}

macro_rules! ubig_signed_conversions {
    ($($t:ty)*) => {$(
        impl TryFrom<$t> for UBig {
            type Error = ConversionError;

            #[inline]
            fn try_from(value: $t) -> Result<UBig, ConversionError> {
                UBig::try_from_signed(value)
            }
        }

        impl TryFrom<UBig> for $t {
            type Error = ConversionError;

            #[inline]
            fn try_from(value: UBig) -> Result<$t, ConversionError> {
                value.try_to_signed()
            }
        }

        impl TryFrom<&UBig> for $t {
            type Error = ConversionError;

            #[inline]
            fn try_from(value: &UBig) -> Result<$t, ConversionError> {
                value.try_to_signed()
            }
        }
    )*};
}
ubig_signed_conversions!(i8 i16 i32 i64 i128 isize);

macro_rules! ibig_unsigned_conversions {
    ($($t:ty)*) => {$(
        impl From<$t> for IBig {
            #[inline]
            fn from(value: $t) -> IBig {
                IBig::from_unsigned(value)
            }
        }

        impl TryFrom<IBig> for $t {
            type Error = ConversionError;

            #[inline]
            fn try_from(value: IBig) -> Result<$t, ConversionError> {
                value.try_to_unsigned()
            }
        }

        impl TryFrom<&IBig> for $t {
            type Error = ConversionError;

            #[inline]
            fn try_from(value: &IBig) -> Result<$t, ConversionError> {
                value.try_to_unsigned()
            }
        }
    )*};
}

ibig_unsigned_conversions!(u8 u16 u32 u64 u128 usize);

impl From<bool> for IBig {
    #[inline]
    fn from(b: bool) -> IBig {
        u8::from(b).into()
    }
}

macro_rules! ibig_signed_conversions {
    ($($t:ty)*) => {$(
        impl From<$t> for IBig {
            #[inline]
            fn from(value: $t) -> IBig {
                IBig::from_signed(value)
            }
        }

        impl TryFrom<IBig> for $t {
            type Error = ConversionError;

            #[inline]
            fn try_from(value: IBig) -> Result<$t, ConversionError> {
                value.try_to_signed()
            }
        }

        impl TryFrom<&IBig> for $t {
            type Error = ConversionError;

            #[inline]
            fn try_from(value: &IBig) -> Result<$t, ConversionError> {
                value.try_to_signed()
            }
        }
    )*};
}
ibig_signed_conversions!(i8 i16 i32 i64 i128 isize);

macro_rules! ubig_float_conversions {
    ($($t:ty => $i:ty;)*) => {$(
        impl TryFrom<$t> for UBig {
            type Error = ConversionError;

            fn try_from(value: $t) -> Result<Self, Self::Error> {
                let (man, exp) = value.decode().map_err(|_| ConversionError::OutOfBounds)?;
                let mut result: UBig = man.try_into()?;
                if exp >= 0 {
                    result <<= exp as usize;
                } else {
                    result >>= (-exp) as usize;
                }
                Ok(result)
            }
        }

        impl TryFrom<UBig> for $t {
            type Error = ConversionError;

            fn try_from(value: UBig) -> Result<Self, Self::Error> {
                const MAX_BIT_LEN: usize = (<$t>::MANTISSA_DIGITS + 1) as usize;
                if value.bit_len() > MAX_BIT_LEN
                    || (value.bit_len() == MAX_BIT_LEN && !value.is_power_of_two())
                {
                    // precision loss occurs when the integer has more digits than what the mantissa can store
                    Err(ConversionError::LossOfPrecision)
                } else {
                    Ok(<$i>::try_from(value).unwrap() as $t)
                }
            }
        }
    )*};
}
ubig_float_conversions!(f32 => u32; f64 => u64;);

macro_rules! ibig_float_conversions {
    ($($t:ty => $i:ty;)*) => {$(
        impl TryFrom<$t> for IBig {
            type Error = ConversionError;

            fn try_from(value: $t) -> Result<Self, Self::Error> {
                let (man, exp) = value.decode().map_err(|_| ConversionError::OutOfBounds)?;
                let mut result: IBig = man.into();
                if exp >= 0 {
                    result <<= exp as usize;
                } else {
                    result >>= (-exp) as usize;
                }
                Ok(result)
            }
        }

        impl TryFrom<IBig> for $t {
            type Error = ConversionError;

            fn try_from(value: IBig) -> Result<Self, Self::Error> {
                const MAX_BIT_LEN: usize = (<$t>::MANTISSA_DIGITS + 1) as usize;
                let (sign, value) = value.into_parts();
                if value.bit_len() > MAX_BIT_LEN
                    || (value.bit_len() == MAX_BIT_LEN && !value.is_power_of_two())
                {
                    // precision loss occurs when the integer has more digits than what the mantissa can store
                    Err(ConversionError::LossOfPrecision)
                } else {
                    let float = <$i>::try_from(value).unwrap() as $t;
                    Ok(sign * float)
                }
            }
        }
    )*};
}
ibig_float_conversions!(f32 => u32; f64 => u64;);

impl From<UBig> for IBig {
    #[inline]
    fn from(x: UBig) -> IBig {
        IBig(x.0.with_sign(Positive))
    }
}

impl TryFrom<IBig> for UBig {
    type Error = ConversionError;

    #[inline]
    fn try_from(x: IBig) -> Result<UBig, ConversionError> {
        match x.sign() {
            Positive => Ok(UBig(x.0)),
            Negative => Err(ConversionError::OutOfBounds),
        }
    }
}

impl UBig {
    /// Convert an unsigned primitive to [UBig].
    #[inline]
    pub(crate) fn from_unsigned<T>(x: T) -> UBig
    where
        T: PrimitiveUnsigned,
    {
        UBig(Repr::from_unsigned(x))
    }

    /// Try to convert a signed primitive to [UBig].
    #[inline]
    fn try_from_signed<T>(x: T) -> Result<UBig, ConversionError>
    where
        T: PrimitiveSigned,
    {
        let (sign, mag) = x.to_sign_magnitude();
        match sign {
            Sign::Positive => Ok(UBig(Repr::from_unsigned(mag))),
            Sign::Negative => Err(ConversionError::OutOfBounds),
        }
    }

    /// Try to convert [UBig] to an unsigned primitive.
    #[inline]
    pub(crate) fn try_to_unsigned<T>(&self) -> Result<T, ConversionError>
    where
        T: PrimitiveUnsigned,
    {
        self.repr().try_to_unsigned()
    }

    /// Try to convert [UBig] to a signed primitive.
    #[inline]
    fn try_to_signed<T>(&self) -> Result<T, ConversionError>
    where
        T: PrimitiveSigned,
    {
        T::try_from_sign_magnitude(Sign::Positive, self.repr().try_to_unsigned()?)
    }
}

impl IBig {
    /// Convert an unsigned primitive to [IBig].
    #[inline]
    pub(crate) fn from_unsigned<T: PrimitiveUnsigned>(x: T) -> IBig {
        IBig(Repr::from_unsigned(x))
    }

    /// Convert a signed primitive to [IBig].
    #[inline]
    pub(crate) fn from_signed<T: PrimitiveSigned>(x: T) -> IBig {
        let (sign, mag) = x.to_sign_magnitude();
        IBig(Repr::from_unsigned(mag).with_sign(sign))
    }

    /// Try to convert [IBig] to an unsigned primitive.
    #[inline]
    pub(crate) fn try_to_unsigned<T: PrimitiveUnsigned>(&self) -> Result<T, ConversionError> {
        let (sign, mag) = self.as_sign_repr();
        match sign {
            Positive => mag.try_to_unsigned(),
            Negative => Err(ConversionError::OutOfBounds),
        }
    }

    /// Try to convert [IBig] to an signed primitive.
    #[inline]
    pub(crate) fn try_to_signed<T: PrimitiveSigned>(&self) -> Result<T, ConversionError> {
        let (sign, mag) = self.as_sign_repr();
        T::try_from_sign_magnitude(sign, mag.try_to_unsigned()?)
    }
}

mod repr {
    use core::cmp::Ordering;

    use static_assertions::const_assert;

    use super::*;
    use crate::repr::TypedReprRef;

    /// Try to convert `Word`s to an unsigned primitive.
    fn unsigned_from_words<T>(words: &[Word]) -> Result<T, ConversionError>
    where
        T: PrimitiveUnsigned,
    {
        debug_assert!(words.len() >= 2);
        let t_words = T::BYTE_SIZE / WORD_BYTES;
        if t_words <= 1 || words.len() > t_words {
            Err(ConversionError::OutOfBounds)
        } else {
            assert!(
                T::BIT_SIZE % WORD_BITS == 0,
                "A large primitive type not a multiple of word size."
            );
            let mut repr = T::default().to_le_bytes();
            let bytes: &mut [u8] = repr.as_mut();
            for (idx, w) in words.iter().enumerate() {
                let pos = idx * WORD_BYTES;
                bytes[pos..pos + WORD_BYTES].copy_from_slice(&w.to_le_bytes());
            }
            Ok(T::from_le_bytes(repr))
        }
    }

    impl Repr {
        #[inline]
        pub fn from_unsigned<T>(x: T) -> Self
        where
            T: PrimitiveUnsigned,
        {
            if let Ok(dw) = x.try_into() {
                Self::from_dword(dw)
            } else {
                let repr = x.to_le_bytes();
                Self::from_le_bytes_large::<false>(repr.as_ref())
            }
        }
    }

    impl<'a> TypedReprRef<'a> {
        #[inline]
        pub fn try_to_unsigned<T>(self) -> Result<T, ConversionError>
        where
            T: PrimitiveUnsigned,
        {
            match self {
                RefSmall(dw) => T::try_from(dw).map_err(|_| ConversionError::OutOfBounds),
                RefLarge(words) => unsigned_from_words(words),
            }
        }

        #[inline]
        pub fn to_f32(self) -> Approximation<f32, Sign> {
            match self {
                RefSmall(dword) => to_f32_small(dword),
                RefLarge(_) => self.to_f32_nontrivial(),
            }
        }

        #[inline]
        fn to_f32_nontrivial(self) -> Approximation<f32, Sign> {
            let n = self.bit_len();
            debug_assert!(n > 32);

            if n > 128 {
                Inexact(f32::INFINITY, Positive)
            } else {
                let top_u31: u32 = (self >> (n - 31)).as_typed().try_to_unsigned().unwrap();
                let extra_bit = self.are_low_bits_nonzero(n - 31) as u32;
                f32::encode((top_u31 | extra_bit) as i32, (n - 31) as i16)
            }
        }

        #[inline]
        pub fn to_f64(self) -> Approximation<f64, Sign> {
            match self {
                RefSmall(dword) => to_f64_small(dword as DoubleWord),
                RefLarge(_) => self.to_f64_nontrivial(),
            }
        }

        #[inline]
        fn to_f64_nontrivial(self) -> Approximation<f64, Sign> {
            let n = self.bit_len();
            debug_assert!(n > 64);

            if n > 1024 {
                Inexact(f64::INFINITY, Positive)
            } else {
                let top_u63: u64 = (self >> (n - 63)).as_typed().try_to_unsigned().unwrap();
                let extra_bit = self.are_low_bits_nonzero(n - 63) as u64;
                f64::encode((top_u63 | extra_bit) as i64, (n - 63) as i16)
            }
        }
    }

    fn to_f32_small(dword: DoubleWord) -> Approximation<f32, Sign> {
        let f = dword as f32;
        if f.is_infinite() {
            return Inexact(f, Sign::Positive);
        }

        let back = f as DoubleWord;
        match back.partial_cmp(&dword).unwrap() {
            Ordering::Greater => Inexact(f, Sign::Positive),
            Ordering::Equal => Exact(f),
            Ordering::Less => Inexact(f, Sign::Negative),
        }
    }

    fn to_f64_small(dword: DoubleWord) -> Approximation<f64, Sign> {
        const_assert!((DoubleWord::MAX as f64) < f64::MAX);
        let f = dword as f64;
        let back = f as DoubleWord;

        match back.partial_cmp(&dword).unwrap() {
            Ordering::Greater => Inexact(f, Sign::Positive),
            Ordering::Equal => Exact(f),
            Ordering::Less => Inexact(f, Sign::Negative),
        }
    }
}

// TODO(next): implement `to_digits` and `from_digits`, that supports base up to Word::MAX.
//             This method won't be optimized as much as the InRadix formatter,
//             because InRadix has a limit on the radix.
