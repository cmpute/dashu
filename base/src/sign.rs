//! Trait definitions for sign related operations.

use core::{
    cmp::Ordering,
    ops::{Mul, MulAssign, Neg},
};

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
/// # use dashu_base::UnsignedAbs;
/// assert_eq!((-5i8).unsigned_abs(), 5u8);
/// ```
pub trait UnsignedAbs {
    type Output;

    fn unsigned_abs(self) -> Self::Output;
}

// TODO(next): implement abs_eq, abs_cmp among UBig/IBig/FBig/RBig

/// Check whether the magnitude of this number is equal the magnitude of the other number
///
/// # Examples
///
/// ```
/// # use dashu_base::AbsEq;
/// assert!(5.abs_eq(&-5));
/// assert!(12.3.abs_eq(&-12.3));
/// ```
pub trait AbsEq<Rhs = Self> {
    fn abs_eq(&self, rhs: &Rhs) -> bool;
}

/// Compare the magnitude of this number to the magnitude of the other number
///
/// Note that this function will panic if either of the numbers is NaN.
///
/// # Examples
///
/// ```
/// # use dashu_base::AbsOrd;
/// assert!(5.abs_cmp(&-6).is_le());
/// assert!(12.3.abs_cmp(&-12.3).is_eq());
/// ```
pub trait AbsOrd<Rhs = Self> {
    fn abs_cmp(&self, rhs: &Rhs) -> Ordering;
}

/// This trait marks the number is signed.
///
/// Notice that the negative zeros (of [f32] and [f64]) are still considered
/// to have a positive sign.
///
/// # Examples
///
/// ```
/// # use dashu_base::{Signed, Sign};
/// assert_eq!((-2).sign(), Sign::Negative);
/// assert_eq!((-2.4).sign(), Sign::Negative);
/// assert_eq!((0.).sign(), Sign::Positive);
///
/// assert!(2.is_positive());
/// assert!((-2.4).is_negative());
/// assert!((0.).is_positive());
/// ```
pub trait Signed {
    fn sign(&self) -> Sign;

    #[inline]
    fn is_positive(&self) -> bool {
        self.sign() == Sign::Positive
    }
    #[inline]
    fn is_negative(&self) -> bool {
        self.sign() == Sign::Negative
    }
}

macro_rules! impl_abs_ops_prim {
    ($($signed:ty;)*) => {$( // this branch is only for float
        impl Abs for $signed {
            type Output = $signed;
            #[inline]
            fn abs(self) -> Self::Output {
                if self.is_nan() || self >= 0. {
                    self
                } else {
                    -self
                }
            }
        }
    )*};
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
impl_abs_ops_prim!(f32; f64;);

/// An enum representing the sign of a number
///
/// A sign can be converted to or from a boolean value, assuming `true` is [Negative].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Sign {
    Positive,
    Negative,
}

use Sign::*;

impl From<bool> for Sign {
    /// Convert boolean value to [Sign], returns [Negative] for `true`
    #[inline]
    fn from(v: bool) -> Self {
        match v {
            true => Self::Negative,
            false => Self::Positive,
        }
    }
}

impl From<Sign> for bool {
    /// Convert [Sign] to boolean value, returns `true` for [Negative]
    #[inline]
    fn from(v: Sign) -> Self {
        match v {
            Sign::Negative => true,
            Sign::Positive => false,
        }
    }
}

impl Neg for Sign {
    type Output = Sign;

    #[inline]
    fn neg(self) -> Sign {
        match self {
            Positive => Negative,
            Negative => Positive,
        }
    }
}

impl Mul<Sign> for Sign {
    type Output = Sign;

    #[inline]
    fn mul(self, rhs: Sign) -> Sign {
        match (self, rhs) {
            (Positive, Positive) => Positive,
            (Positive, Negative) => Negative,
            (Negative, Positive) => Negative,
            (Negative, Negative) => Positive,
        }
    }
}

impl Mul<Ordering> for Sign {
    type Output = Ordering;
    #[inline]
    fn mul(self, rhs: Ordering) -> Self::Output {
        match self {
            Positive => rhs,
            Negative => rhs.reverse(),
        }
    }
}

impl Mul<Sign> for Ordering {
    type Output = Ordering;
    #[inline]
    fn mul(self, rhs: Sign) -> Self::Output {
        match rhs {
            Positive => self,
            Negative => self.reverse(),
        }
    }
}

impl MulAssign<Sign> for Sign {
    #[inline]
    fn mul_assign(&mut self, rhs: Sign) {
        *self = *self * rhs;
    }
}

impl PartialOrd for Sign {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Sign {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Positive, Negative) => Ordering::Greater,
            (Negative, Positive) => Ordering::Less,
            _ => Ordering::Equal,
        }
    }
}

macro_rules! impl_sign_ops_for_primitives {
    ($($t:ty)*) => {$(
        impl Mul<$t> for Sign {
            type Output = $t;

            #[inline]
            fn mul(self, rhs: $t) -> Self::Output {
                match self {
                    Positive => rhs,
                    Negative => -rhs
                }
            }
        }

        impl Mul<Sign> for $t {
            type Output = $t;

            #[inline]
            fn mul(self, rhs: Sign) -> Self::Output {
                match rhs {
                    Positive => self,
                    Negative => -self
                }
            }
        }
    )*};
}
impl_sign_ops_for_primitives!(i8 i16 i32 i64 i128 isize f32 f64);

macro_rules! impl_signed_for_int {
    ($($t:ty)*) => {$(
        impl Signed for $t {
            #[inline]
            fn sign(&self) -> Sign {
                Sign::from(*self < 0)
            }
        }

        impl AbsEq for $t {
            #[inline]
            fn abs_eq(&self, rhs: &Self) -> bool {
                self.abs() == rhs.abs()
            }
        }

        impl AbsOrd for $t {
            #[inline]
            fn abs_cmp(&self, rhs: &Self) -> Ordering {
                self.abs().cmp(&rhs.abs())
            }
        }
    )*};
}
impl_signed_for_int!(i8 i16 i32 i64 i128 isize);

macro_rules! impl_signed_for_float {
    ($t:ty, $shift:literal) => {
        impl Signed for $t {
            #[inline]
            fn sign(&self) -> Sign {
                if self.is_nan() {
                    panic!("nan doesn't have a sign")
                } else if *self == -0. {
                    return Sign::Positive;
                }
                Sign::from(self.to_bits() >> $shift > 0)
            }
        }

        impl AbsEq for $t {
            #[inline]
            fn abs_eq(&self, rhs: &Self) -> bool {
                self.abs() == rhs.abs()
            }
        }

        impl AbsOrd for $t {
            #[inline]
            fn abs_cmp(&self, rhs: &Self) -> Ordering {
                self.abs()
                    .partial_cmp(&rhs.abs())
                    .expect("abs_cmp is not allowed on NaNs!")
            }
        }
    };
}
impl_signed_for_float!(f32, 31);
impl_signed_for_float!(f64, 63);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signed() {
        assert_eq!(0i32.sign(), Sign::Positive);
        assert_eq!(1i32.sign(), Sign::Positive);
        assert_eq!((-1i32).sign(), Sign::Negative);

        assert_eq!(0f32.sign(), Sign::Positive);
        assert_eq!((-0f32).sign(), Sign::Positive);
        assert_eq!(1f32.sign(), Sign::Positive);
        assert_eq!((-1f32).sign(), Sign::Negative);
    }

    #[test]
    #[should_panic]
    fn test_signed_nan() {
        let _ = f32::NAN.sign();
    }

    #[test]
    #[should_panic]
    fn test_abs_cmp_nan() {
        let _ = f32::NAN.abs_cmp(&f32::NAN);
    }
}
