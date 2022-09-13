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

/// An enum representing the sign of a number
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Sign {
    Positive,
    Negative,
}

use Sign::*;

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

macro_rules! impl_mul_sign_for_primitives {
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

impl_mul_sign_for_primitives!(i8 i16 i32 i64 i128 isize f32 f64);
