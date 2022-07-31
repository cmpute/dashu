//! Conversions between types.

use crate::{
    arch::word::Word,
    buffer::Buffer,
    error::{OutOfBoundsError, panic_negative_ubig},
    ibig::IBig,
    primitive::{self, PrimitiveSigned, PrimitiveUnsigned, DWORD_BYTES, WORD_BITS, WORD_BYTES, double_word},
    repr::{Repr, TypedReprRef::*},
    sign::Sign::{self, *},
    ubig::UBig,
};
use alloc::vec::Vec;
use core::convert::{TryFrom, TryInto};

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

impl UBig {
    /// Construct from little-endian bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from_le_bytes(&[3, 2, 1]), 0x010203);
    /// ```
    #[inline]
    pub fn from_le_bytes(bytes: &[u8]) -> UBig {
        let repr = if bytes.len() <= WORD_BYTES {
            // fast path
            Repr::from_word(primitive::word_from_le_bytes_partial(bytes))
        } else if bytes.len() <= DWORD_BYTES {
            Repr::from_dword(primitive::dword_from_le_bytes_partial(bytes))
        } else {
            // slow path
            Self::from_le_bytes_large(bytes)
        };
        UBig(repr)
    }

    fn from_le_bytes_large(bytes: &[u8]) -> Repr {
        debug_assert!(bytes.len() > WORD_BYTES);
        let mut buffer = Buffer::allocate((bytes.len() - 1) / WORD_BYTES + 1);
        let mut chunks = bytes.chunks_exact(WORD_BYTES);
        for chunk in &mut chunks {
            buffer.push(Word::from_le_bytes(chunk.try_into().unwrap()));
        }
        if !chunks.remainder().is_empty() {
            buffer.push(primitive::word_from_le_bytes_partial(chunks.remainder()));
        }
        Repr::from_buffer(buffer)
    }

    /// Construct from big-endian bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from_be_bytes(&[1, 2, 3]), 0x010203);
    /// ```
    #[inline]
    pub fn from_be_bytes(bytes: &[u8]) -> UBig {
        let repr = if bytes.len() <= WORD_BYTES {
            // fast path
            Repr::from_word(primitive::word_from_be_bytes_partial(bytes))
        } else if bytes.len() <= DWORD_BYTES {
            Repr::from_dword(primitive::dword_from_be_bytes_partial(bytes))
        } else {
            // slow path
            Self::from_be_bytes_large(bytes)
        };
        UBig(repr)
    }

    fn from_be_bytes_large(bytes: &[u8]) -> Repr {
        debug_assert!(bytes.len() > WORD_BYTES);
        let mut buffer = Buffer::allocate((bytes.len() - 1) / WORD_BYTES + 1);
        let mut chunks = bytes.rchunks_exact(WORD_BYTES);
        for chunk in &mut chunks {
            buffer.push(Word::from_be_bytes(chunk.try_into().unwrap()));
        }
        if !chunks.remainder().is_empty() {
            buffer.push(primitive::word_from_be_bytes_partial(chunks.remainder()));
        }
        Repr::from_buffer(buffer)
    }

    /// Return little-endian bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert!(UBig::ZERO.to_le_bytes().is_empty());
    /// assert_eq!(UBig::from(0x010203u32).to_le_bytes(), [3, 2, 1]);
    /// ```
    pub fn to_le_bytes(&self) -> Vec<u8> {
        match self.repr() {
            RefSmall(x) => {
                let bytes = x.to_le_bytes();
                let skip_bytes = x.leading_zeros() as usize / 8;
                bytes[..DWORD_BYTES - skip_bytes].to_vec()
            }
            RefLarge(buffer) => {
                let n = buffer.len();
                let last = buffer[n - 1];
                let skip_last_bytes = last.leading_zeros() as usize / 8;
                let mut bytes = Vec::with_capacity(n * WORD_BYTES - skip_last_bytes);
                for word in &buffer[..n - 1] {
                    bytes.extend_from_slice(&word.to_le_bytes());
                }
                let last_bytes = last.to_le_bytes();
                bytes.extend_from_slice(&last_bytes[..WORD_BYTES - skip_last_bytes]);
                bytes
            }
        }
    }

    /// Return big-endian bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert!(UBig::ZERO.to_be_bytes().is_empty());
    /// assert_eq!(UBig::from(0x010203u32).to_be_bytes(), [1, 2, 3]);
    /// ```
    pub fn to_be_bytes(&self) -> Vec<u8> {
        match self.repr() {
            RefSmall(x) => {
                let bytes = x.to_be_bytes();
                let skip_bytes = x.leading_zeros() as usize / 8;
                bytes[skip_bytes..].to_vec()
            }
            RefLarge(buffer) => {
                let n = buffer.len();
                let last = buffer[n - 1];
                let skip_last_bytes = last.leading_zeros() as usize / 8;
                let mut bytes = Vec::with_capacity(n * WORD_BYTES - skip_last_bytes);
                let last_bytes = last.to_be_bytes();
                bytes.extend_from_slice(&last_bytes[skip_last_bytes..]);
                for word in buffer[..n - 1].iter().rev() {
                    bytes.extend_from_slice(&word.to_be_bytes());
                }
                bytes
            }
        }
    }

    /// Get the raw representation in [Word][crate::Word]s.
    /// 
    /// If the number is zero, then empty slice will be returned.
    #[inline]
    pub fn as_words(&self) -> &[crate::Word] {
        let (sign, words) = self.0.as_sign_slice();
        debug_assert!(matches!(sign, crate::sign::Sign::Positive));
        words
    }

    /// Create a UBig from a single [Word][crate::Word].
    #[inline]
    pub const fn from_word(word: crate::Word) -> Self {
        Self(Repr::from_word(word))
    }

    /// Create a UBig from a double [Word][crate::Word].
    #[inline]
    pub const fn from_dword(low: crate::Word, high: crate::Word) -> Self {
        Self(Repr::from_dword(double_word(low, high)))
    }

    /// Convert a sequence of [Word][crate::Word]s into a UBig
    #[inline]
    pub fn from_words(words: &[crate::Word]) -> Self {
        Self(Repr::from_buffer(words.into()))
    }

    // TODO(v0.2): return an Approximation struct

    /// Convert to f32.
    ///
    /// Round to nearest, breaking ties to even last bit.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from(134u8).to_f32(), 134.0f32);
    /// ```
    #[inline]
    pub fn to_f32(&self) -> f32 {
        self.repr().to_f32()
    }

    /// Convert to f64.
    ///
    /// Round to nearest, breaking ties to even last bit.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from(134u8).to_f64(), 134.0f64);
    /// ```
    #[inline]
    pub fn to_f64(&self) -> f64 {
        self.repr().to_f64()
    }
}

impl IBig {
    /// Get the raw representation in [Word][crate::Word]s.
    /// 
    /// If the number is zero, then empty slice will be returned.
    #[inline]
    pub fn as_sign_words(&self) -> (Sign, &[crate::Word]) {
        self.0.as_sign_slice()
    }

    /// Create an IBig from a [Sign] and a double [Word][crate::Word]
    pub const fn from_parts_const(sign: Sign, low: crate::Word, high: crate::Word) -> Self {
        Self(Repr::from_dword(double_word(low, high)).with_sign(sign))
    }

    /// Convert to f32.
    ///
    /// Round to nearest, breaking ties to even last bit.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// assert_eq!(IBig::from(-134).to_f32(), -134.0f32);
    /// ```
    #[inline]
    pub fn to_f32(&self) -> f32 {
        let (sign, mag) = self.as_sign_repr();
        let val = mag.to_f32();
        match sign {
            Positive => val,
            Negative => -val,
        }
    }

    /// Convert to f64.
    ///
    /// Round to nearest, breaking ties to even last bit.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// assert_eq!(IBig::from(-134).to_f64(), -134.0f64);
    /// ```
    #[inline]
    pub fn to_f64(&self) -> f64 {
        let (sign, mag) = self.as_sign_repr();
        let val = mag.to_f64();
        match sign {
            Positive => val,
            Negative => -val,
        }
    }
}

/// Round to even floating point adjustment, based on the bottom
/// bit of mantissa and additional 2 bits (i.e. 3 bits in units of ULP/4).
#[inline]
fn round_to_even_adjustment(bits: u32) -> bool {
    bits >= 0b110 || bits == 0b011
}

macro_rules! ubig_unsigned_conversions {
    ($t:ty) => {
        impl From<$t> for UBig {
            #[inline]
            fn from(value: $t) -> UBig {
                UBig::from_unsigned(value)
            }
        }

        impl TryFrom<UBig> for $t {
            type Error = OutOfBoundsError;

            #[inline]
            fn try_from(value: UBig) -> Result<$t, OutOfBoundsError> {
                value.try_to_unsigned()
            }
        }

        impl TryFrom<&UBig> for $t {
            type Error = OutOfBoundsError;

            #[inline]
            fn try_from(value: &UBig) -> Result<$t, OutOfBoundsError> {
                value.try_to_unsigned()
            }
        }
    };
}

ubig_unsigned_conversions!(u8);
ubig_unsigned_conversions!(u16);
ubig_unsigned_conversions!(u32);
ubig_unsigned_conversions!(u64);
ubig_unsigned_conversions!(u128);
ubig_unsigned_conversions!(usize);

impl From<bool> for UBig {
    #[inline]
    fn from(b: bool) -> UBig {
        u8::from(b).into()
    }
}

macro_rules! ubig_signed_conversions {
    ($t:ty) => {
        impl TryFrom<$t> for UBig {
            type Error = OutOfBoundsError;

            #[inline]
            fn try_from(value: $t) -> Result<UBig, OutOfBoundsError> {
                UBig::try_from_signed(value)
            }
        }

        impl TryFrom<UBig> for $t {
            type Error = OutOfBoundsError;

            #[inline]
            fn try_from(value: UBig) -> Result<$t, OutOfBoundsError> {
                value.try_to_signed()
            }
        }

        impl TryFrom<&UBig> for $t {
            type Error = OutOfBoundsError;

            #[inline]
            fn try_from(value: &UBig) -> Result<$t, OutOfBoundsError> {
                value.try_to_signed()
            }
        }
    };
}

ubig_signed_conversions!(i8);
ubig_signed_conversions!(i16);
ubig_signed_conversions!(i32);
ubig_signed_conversions!(i64);
ubig_signed_conversions!(i128);
ubig_signed_conversions!(isize);

macro_rules! ibig_unsigned_conversions {
    ($t:ty) => {
        impl From<$t> for IBig {
            #[inline]
            fn from(value: $t) -> IBig {
                IBig::from_unsigned(value)
            }
        }

        impl TryFrom<IBig> for $t {
            type Error = OutOfBoundsError;

            #[inline]
            fn try_from(value: IBig) -> Result<$t, OutOfBoundsError> {
                value.try_to_unsigned()
            }
        }

        impl TryFrom<&IBig> for $t {
            type Error = OutOfBoundsError;

            #[inline]
            fn try_from(value: &IBig) -> Result<$t, OutOfBoundsError> {
                value.try_to_unsigned()
            }
        }
    };
}

ibig_unsigned_conversions!(u8);
ibig_unsigned_conversions!(u16);
ibig_unsigned_conversions!(u32);
ibig_unsigned_conversions!(u64);
ibig_unsigned_conversions!(u128);
ibig_unsigned_conversions!(usize);

impl From<bool> for IBig {
    #[inline]
    fn from(b: bool) -> IBig {
        u8::from(b).into()
    }
}

macro_rules! ibig_signed_conversions {
    ($t:ty) => {
        impl From<$t> for IBig {
            #[inline]
            fn from(value: $t) -> IBig {
                IBig::from_signed(value)
            }
        }

        impl TryFrom<IBig> for $t {
            type Error = OutOfBoundsError;

            #[inline]
            fn try_from(value: IBig) -> Result<$t, OutOfBoundsError> {
                value.try_to_signed()
            }
        }

        impl TryFrom<&IBig> for $t {
            type Error = OutOfBoundsError;

            #[inline]
            fn try_from(value: &IBig) -> Result<$t, OutOfBoundsError> {
                value.try_to_signed()
            }
        }
    };
}

ibig_signed_conversions!(i8);
ibig_signed_conversions!(i16);
ibig_signed_conversions!(i32);
ibig_signed_conversions!(i64);
ibig_signed_conversions!(i128);
ibig_signed_conversions!(isize);

impl From<UBig> for IBig {
    #[inline]
    fn from(x: UBig) -> IBig {
        IBig(x.0.with_sign(Positive))
    }
}

impl From<&UBig> for IBig {
    #[inline]
    fn from(x: &UBig) -> IBig {
        IBig::from(x.clone())
    }
}

impl TryFrom<IBig> for UBig {
    type Error = OutOfBoundsError;

    #[inline]
    fn try_from(x: IBig) -> Result<UBig, OutOfBoundsError> {
        match x.sign() {
            Positive => Ok(UBig(x.0)),
            Negative => Err(OutOfBoundsError),
        }
    }
}

impl TryFrom<&IBig> for UBig {
    type Error = OutOfBoundsError;

    #[inline]
    fn try_from(x: &IBig) -> Result<UBig, OutOfBoundsError> {
        match x.sign() {
            Positive => Ok(UBig(x.0.clone())),
            Negative => Err(OutOfBoundsError),
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
        if let Ok(w) = x.try_into() {
            UBig(Repr::from_word(w))
        } else if let Ok(dw) = x.try_into() {
            UBig(Repr::from_dword(dw))
        } else {
            let repr = x.to_le_bytes();
            UBig::from_le_bytes(repr.as_ref())
        }
    }

    /// Try to convert a signed primitive to [UBig].
    #[inline]
    fn try_from_signed<T>(x: T) -> Result<UBig, OutOfBoundsError>
    where
        T: PrimitiveSigned,
    {
        match T::Unsigned::try_from(x) {
            Ok(u) => Ok(UBig::from_unsigned(u)),
            Err(_) => Err(OutOfBoundsError),
        }
    }

    /// Try to convert [UBig] to an unsigned primitive.
    #[inline]
    pub(crate) fn try_to_unsigned<T>(&self) -> Result<T, OutOfBoundsError>
    where
        T: PrimitiveUnsigned,
    {
        self.repr().try_to_unsigned()
    }

    /// Try to convert [UBig] to a signed primitive.
    #[inline]
    fn try_to_signed<T>(&self) -> Result<T, OutOfBoundsError>
    where
        T: PrimitiveSigned,
    {
        match self.repr() {
            RefSmall(dw) => T::try_from(dw).map_err(|_| OutOfBoundsError),
            RefLarge(buffer) => {
                let u: T::Unsigned = unsigned_from_words(buffer)?;
                u.try_into().map_err(|_| OutOfBoundsError)
            }
        }
    }

    /// This method will panic if the signed integer input is negative,
    /// therefore it should not be exposed to public.
    #[inline]
    pub(crate) fn from_ibig(x: IBig) -> UBig {
        match UBig::try_from(x) {
            Ok(v) => v,
            Err(_) => panic_negative_ubig(),
        }
    }
}

/// Try to convert `Word`s to an unsigned primitive.
fn unsigned_from_words<T>(words: &[Word]) -> Result<T, OutOfBoundsError>
where
    T: PrimitiveUnsigned,
{
    debug_assert!(words.len() >= 2);
    let t_words = T::BYTE_SIZE / WORD_BYTES;
    if t_words <= 1 || words.len() > t_words {
        Err(OutOfBoundsError)
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

impl IBig {
    /// Convert an unsigned primitive to [IBig].
    #[inline]
    pub(crate) fn from_unsigned<T: PrimitiveUnsigned>(x: T) -> IBig {
        IBig(UBig::from_unsigned(x).0)
    }

    /// Convert a signed primitive to [IBig].
    #[inline]
    pub(crate) fn from_signed<T: PrimitiveSigned>(x: T) -> IBig {
        let (sign, mag) = x.to_sign_magnitude();
        IBig(UBig::from_unsigned(mag).0.with_sign(sign))
    }

    /// Try to convert [IBig] to an unsigned primitive.
    #[inline]
    pub(crate) fn try_to_unsigned<T: PrimitiveUnsigned>(&self) -> Result<T, OutOfBoundsError> {
        let (sign, mag) = self.as_sign_repr();
        match sign {
            Positive => mag.try_to_unsigned(),
            Negative => Err(OutOfBoundsError),
        }
    }

    /// Try to convert [IBig] to an signed primitive.
    #[inline]
    pub(crate) fn try_to_signed<T: PrimitiveSigned>(&self) -> Result<T, OutOfBoundsError> {
        let (sign, mag) = self.as_sign_repr();
        T::try_from_sign_magnitude(sign, mag.try_to_unsigned()?)
    }
}

mod repr {
    use super::*;
    use crate::repr::TypedReprRef;

    impl<'a> TypedReprRef<'a> {
        #[inline]
        pub fn try_to_unsigned<T>(self) -> Result<T, OutOfBoundsError>
        where
            T: PrimitiveUnsigned,
        {
            match self {
                RefSmall(dw) => T::try_from(dw).map_err(|_| OutOfBoundsError),
                RefLarge(buffer) => unsigned_from_words(buffer),
            }
        }

        #[inline]
        pub fn to_f32(self) -> f32 {
            match self {
                RefSmall(dword) => dword as f32,
                RefLarge(_) => match self.try_to_unsigned::<u32>() {
                    Ok(val) => val as f32,
                    Err(_) => self.to_f32_nontrivial(),
                },
            }
        }

        fn to_f32_nontrivial(self) -> f32 {
            let n = self.bit_len();
            debug_assert!(n > 32);

            if n > 128 {
                f32::INFINITY
            } else {
                let exponent = (n - 1) as u32;
                debug_assert!((32..128).contains(&exponent));
                let mantissa25: u32 = self.high_bits(25).as_typed().try_to_unsigned().unwrap();
                let mantissa = mantissa25 >> 1;

                // value = [8 bits: exponent + 127][23 bits: mantissa without the top bit]
                let value = ((exponent + 126) << 23) + mantissa;

                // Calculate round-to-even adjustment.
                let extra_bit = self.are_low_bits_nonzero(n - 25);
                // low bit of mantissa and two extra bits
                let low_bits = ((mantissa25 & 0b11) << 1) | u32::from(extra_bit);
                let adjustment = round_to_even_adjustment(low_bits);

                // If adjustment is true, increase the mantissa.
                // If the mantissa overflows, this correctly increases the exponent and
                // sets the mantissa to 0.
                // If the exponent overflows, we correctly get the representation of infinity.
                let value = value + u32::from(adjustment);
                f32::from_bits(value)
            }
        }

        #[inline]
        pub fn to_f64(self) -> f64 {
            match self {
                RefSmall(dword) => dword as f64,
                RefLarge(_) => match self.try_to_unsigned::<u64>() {
                    Ok(val) => val as f64,
                    Err(_) => self.to_f64_nontrivial(),
                },
            }
        }

        fn to_f64_nontrivial(self) -> f64 {
            let n = self.bit_len();
            debug_assert!(n > 64);

            if n > 1024 {
                f64::INFINITY
            } else {
                let exponent = (n - 1) as u64;
                debug_assert!((64..1024).contains(&exponent));
                let mantissa54: u64 = self.high_bits(54).as_typed().try_to_unsigned().unwrap();
                let mantissa = mantissa54 >> 1;

                // value = [11-bits: exponent + 1023][52 bit: mantissa without the top bit]
                let value = ((exponent + 1022) << 52) + mantissa;

                // Calculate round-to-even adjustment.
                let extra_bit = self.are_low_bits_nonzero(n - 54);
                // low bit of mantissa and two extra bits
                let low_bits = (((mantissa54 & 0b11) as u32) << 1) | u32::from(extra_bit);
                let adjustment = round_to_even_adjustment(low_bits);

                // If adjustment is true, increase the mantissa.
                // If the mantissa overflows, this correctly increases the exponent and
                // sets the mantissa to 0.
                // If the exponent overflows, we correctly get the representation of infinity.
                let value = value + u64::from(adjustment);
                f64::from_bits(value)
            }
        }
    }
}
