//! Trait definitions for operations on general numbers.

/// Absolute value.
///
/// # Examples
/// ```
/// use dashu_base::Abs;
/// assert_eq!((-5).abs(), 5);
/// ```
pub trait Abs {
    type Output;

    fn abs(self) -> Self::Output;
}

/// Unsigned absolute value.
///
/// # Examples
/// ```
/// use dashu_base::UnsignedAbs;
/// assert_eq!((-5i8).unsigned_abs(), 5u8);
/// ```
pub trait UnsignedAbs {
    type Output;

    fn unsigned_abs(self) -> Self::Output;
}

macro_rules! impl_abs_ops_prim {
    ($($signed:ty => $unsigned:ty;)*) => {$(
        impl Abs for $signed {
            type Output = $signed;
            #[inline]
            fn abs(self) -> Self::Output {
                <$signed>::abs(self)
            }
        }

        impl UnsignedAbs for $signed {
            type Output = $unsigned;
            #[inline]
            fn unsigned_abs(self) -> Self::Output {
                <$signed>::unsigned_abs(self)
            }
        }
    )*}
}
impl_abs_ops_prim!(i8 => u8; i16 => u16; i32 => u32; i64 => u64; i128 => u128; isize => usize;);
