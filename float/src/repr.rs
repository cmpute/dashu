use crate::utils::get_precision;
use dashu_int::IBig;

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

    /// Convert raw parts into a float number, the precision will be inferred from mantissa
    /// (the lowest k such that `mantissa < radix^k`)
    ///
    /// # Panics
    /// If the mantissa is larger than `radix^usize::MAX`
    #[inline]
    pub fn from_parts(mantissa: IBig, exponent: isize) -> Self {
        // TODO: prevent using this function internally because we enforce normalized representation
        let (mantissa, exponent) = Self::normalize(mantissa, exponent);
        let precision = get_precision::<X>(&mantissa);
        Self {
            mantissa,
            exponent,
            precision,
        }
    }

    /// Convert raw parts into a float number, with given precision.
    #[inline]
    pub fn from_parts_with_precision(mantissa: IBig, exponent: isize, precision: usize) -> Self {
        Self::from_parts(mantissa, exponent).with_precision(precision)
    }

    /// Convert the float number into raw (mantissa, exponent) parts
    #[inline]
    pub fn into_parts(self) -> (IBig, isize) {
        (self.mantissa, self.exponent)
    }

    fn ceil(&self) -> IBig {
        unimplemented!()
    }

    fn floor(&self) -> IBig {
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
