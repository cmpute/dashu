//! Trait definitions for bitwise operations

/// Bitwise AND NOT operation.
///
/// `x.and_not(y)` is equivalent to `x & !y` for primitive integers.
///
/// # Examples
///
/// ```
/// use dashu_base::bits::AndNot;
/// assert_eq!((0xff as u32).and_not(0x1111 as u32), 0xee);
/// ```
pub trait AndNot<Rhs = Self> {
    type Output;

    fn and_not(self, rhs: Rhs) -> Self::Output;
}

/// This trait support bit testing for integers
pub trait BitTest {
    /// Get the minimum required number of bits to represent this integer
    fn bit_len(&self) -> usize;

    /// Get the n-th bit of the integer
    fn bit(&self, n: usize) -> bool;

    /// Get the number of trailing zeros in the integer
    fn trailing_zeros(&self) -> Option<usize>;
}

macro_rules! impl_bit_ops_prim {
    ($($T:ty)*) => {$(
        impl AndNot for $T {
            type Output = $T;
            #[inline]
            fn and_not(self, rhs: $T) -> Self::Output {
                self & !rhs
            }
        }

        impl BitTest for $T {
            #[inline]
            fn bit_len(&self) -> usize {
                (<$T>::BITS - self.leading_zeros()) as usize
            }
            #[inline]
            fn bit(&self, position: usize) -> bool {
                self & (1 << position) > 0
            }
            #[inline]
            fn trailing_zeros(&self) -> Option<usize> {
                if *self == 0 {
                    None
                } else {
                    Some(<$T>::trailing_zeros(*self) as usize)
                }
            }
        }
    )*}
}
impl_bit_ops_prim!(u8 u16 u32 u64 u128 usize);