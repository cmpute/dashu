
use dashu_int::IBig;
use core::num::NonZeroIsize;
use crate::utils::get_precision;

// FIXME: this should be a enum when enum const is supported in generic argument
/// Defines rounding modes of the floating numbers.
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
pub mod RoundingMode {
    /// Round toward 0 (default mode for binary float)
    pub const Zero: u8 = 0;

    /// Round toward +infinity
    pub const Up: u8 = 1;

    /// Round toward -infinity
    pub const Down: u8 = 2;
    
    /// Round to the nearest value, ties are rounded to an even value. (default mode for decimal float)
    pub const HalfEven: u8 = 3;

    /// Round to the nearest value, ties away from zero
    pub const HalfAway: u8 = 4;
}

/// Represent an calculation result with possible error.
enum Approximation<T, E> {
    /// The result is exact, contains the result value
    Exact(T),

    /// The result is inexact, contains the result value and error
    InExact(T, E)
}

impl<T, E> Approximation<T, E> {
    /// Get the value of the calculation regardless of error
    #[inline]
    pub fn unwrap(self) -> T {
        match self {
            Self::Exact(v) => v,
            Self::InExact(v, _) => v
        }
    }
}

impl<T> Approximation<T, NonZeroIsize> {
    /// Get the error of the calculation. 0 is returned if the result is exact.
    #[inline]
    pub fn error(&self) -> isize {
        match self {
            Self::Exact(_) => 0,
            Self::InExact(_, e) => e.get()
        }
    }
}

// TODO: add standalone basic arith methods (add, sub, mul, div) for FloatRepr, such that it returns a Approximation struct

/// An arbitrary precision floating number represented as `mantissa * radix^exponent`, with a precision
/// such that `|mantissa| < radix^precision`. The mantissa is also called significant. `Radix` should be
/// in range \[2, isize::MAX\]. The representation is always normalized (mantissa is not divisible by radix). 
/// 
/// The rounding mode of operations between the float numbers is defined by `Rounding`, its value has to
/// be one of [RoundingMode]. Operations are permitted only between float numbers with the same radix and
/// rounding mode. Note that the rounding is only for operations, it's not "associated" with the value.
/// For example, for correct subtraction, the two operands should have reverse rounding direction.
///
/// The const generic parameters will be abbreviated as Radix -> X, Rounding -> R.
/// Radix should be in range \[2, isize::MAX\], and Rounding value has to be one of [RoundingMode]
#[allow(non_upper_case_globals)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FloatRepr<const Radix: usize, const Rounding: u8> {
    pub(crate) mantissa: IBig,
    pub(crate) exponent: isize,
    pub(crate) precision: usize, // TODO: let precision = 0 implies no precision bound, but when no-precision number operates with another has-precision number, the precision will be set as the other one's
}

impl<const X: usize, const R: u8> FloatRepr<X, R> {
    /// Get the maximum precision set for the float number.
    #[inline]
    pub fn precision(&self) -> usize {
        self.precision
    }

    /// Get the actual precision needed for the float number.
    /// 
    /// Shrink to this value using [Self::with_precision] will not cause loss of float precision.
    #[inline]
    pub fn actual_precision(&self) -> usize {
        get_precision::<X>(&self.mantissa)
    }

    fn ceil(&self) -> Self {
        unimplemented!()
    }

    fn floor(&self) -> Self {
        unimplemented!()
    }

    fn trunc(&self) -> Self {
        unimplemented!()
    }

    fn fract(&self) -> Self {
        unimplemented!()
    }
}

/// Multi-precision float number with binary exponent
#[allow(non_upper_case_globals)]
pub type BinaryRepr<const Rounding: u8> = FloatRepr<2, Rounding>;
/// Multi-precision decimal number with decimal exponent
#[allow(non_upper_case_globals)]
pub type DecimalRepr<const Rounding: u8> = FloatRepr<10, Rounding>;
