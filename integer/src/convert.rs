//! Conversions between types.

use crate::{
    arch::word::{DoubleWord, Word},
    buffer::Buffer,
    ibig::IBig,
    primitive::{self, PrimitiveSigned, PrimitiveUnsigned, DWORD_BYTES, WORD_BITS, WORD_BYTES},
    repr::{Repr, TypedReprRef::*},
    ubig::UBig,
    Sign::*,
};
use alloc::{boxed::Box, vec::Vec};
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

pub(crate) fn words_to_le_bytes(words: &[Word]) -> Vec<u8> {
    debug_assert!(!words.is_empty());

    let n = words.len();
    let last = words[n - 1];
    let skip_last_bytes = last.leading_zeros() as usize / 8;
    let mut bytes = Vec::with_capacity(n * WORD_BYTES - skip_last_bytes);
    for word in &words[..n - 1] {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    let last_bytes = last.to_le_bytes();
    bytes.extend_from_slice(&last_bytes[..WORD_BYTES - skip_last_bytes]);
    bytes
}

pub(crate) fn words_to_be_bytes(words: &[Word]) -> Vec<u8> {
    debug_assert!(!words.is_empty());

    let n = words.len();
    let last = words[n - 1];
    let skip_last_bytes = last.leading_zeros() as usize / 8;
    let mut bytes = Vec::with_capacity(n * WORD_BYTES - skip_last_bytes);
    let last_bytes = last.to_be_bytes();
    bytes.extend_from_slice(&last_bytes[skip_last_bytes..]);
    for word in words[..n - 1].iter().rev() {
        bytes.extend_from_slice(&word.to_be_bytes());
    }
    bytes
}

impl Repr {
    #[inline]
    pub fn from_le_bytes(bytes: &[u8]) -> Repr {
        if bytes.len() <= WORD_BYTES {
            // fast path
            Self::from_word(primitive::word_from_le_bytes_partial(bytes))
        } else if bytes.len() <= DWORD_BYTES {
            Self::from_dword(primitive::dword_from_le_bytes_partial(bytes))
        } else {
            // slow path
            Self::from_le_bytes_large(bytes)
        }
    }

    pub fn from_le_bytes_large(bytes: &[u8]) -> Repr {
        debug_assert!(bytes.len() >= DWORD_BYTES);
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

    pub fn from_be_bytes(bytes: &[u8]) -> Repr {
        if bytes.len() <= WORD_BYTES {
            // fast path
            Repr::from_word(primitive::word_from_be_bytes_partial(bytes))
        } else if bytes.len() <= DWORD_BYTES {
            Repr::from_dword(primitive::dword_from_be_bytes_partial(bytes))
        } else {
            // slow path
            Self::from_be_bytes_large(bytes)
        }
    }

    pub fn from_be_bytes_large(bytes: &[u8]) -> Repr {
        debug_assert!(bytes.len() >= DWORD_BYTES);
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
        match self.repr() {
            RefSmall(x) => {
                let bytes = x.to_le_bytes();
                let skip_bytes = x.leading_zeros() as usize / 8;
                bytes[..DWORD_BYTES - skip_bytes].into()
            }
            RefLarge(words) => words_to_le_bytes(words).into_boxed_slice(),
        }
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
        match self.repr() {
            RefSmall(x) => {
                let bytes = x.to_be_bytes();
                let skip_bytes = x.leading_zeros() as usize / 8;
                bytes[skip_bytes..].into()
            }
            RefLarge(words) => words_to_be_bytes(words).into_boxed_slice(),
        }
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
            if let Ok(w) = x.try_into() {
                Self::from_word(w)
            } else if let Ok(dw) = x.try_into() {
                Self::from_dword(dw)
            } else {
                let repr = x.to_le_bytes();
                Self::from_le_bytes_large(repr.as_ref())
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
