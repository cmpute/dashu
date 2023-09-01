//! Trait definitions for math operations

/// Fast estimation of the binary logarithm of a number
///
/// # Panics
///
/// Panics if the number is 0
///
/// # Examples
///
/// ```
/// use dashu_base::EstimatedLog2;
///
/// let lb3 = 1.584962500721156f32;
/// let (lb3_lb, lb3_ub) = 3u8.log2_bounds();
/// assert!(lb3_lb <= lb3 && lb3 <= lb3_ub);
/// assert!((lb3 - lb3_lb) / lb3 < 1. / 256.);
/// assert!((lb3_ub - lb3) / lb3 <= 1. / 256.);
///
/// let lb3_est = 3u8.log2_est();
/// assert!((lb3 - lb3_est).abs() < 1e-3);
/// ```
pub trait EstimatedLog2 {
    /// Estimate the bounds of the binary logarithm.
    ///
    /// The result is `(lower bound, upper bound)` such that `lower bound ≤ log2(self) ≤ upper bound`.
    /// The precision of the bounds must be at least 8 bits (relative error < 2^-8).
    ///
    /// With `std` disabled, the precision is about 13 bits. With `std` enabled, the precision
    /// can be full 24 bits. But the exact precision is not guaranteed and should not be not
    /// relied on.
    ///
    /// For negative values, the logarithm is calculated based on its absolute value. If the number
    /// is zero, then negative infinity will be returned.
    ///
    fn log2_bounds(&self) -> (f32, f32);

    /// Estimate the value of the binary logarithm. It's calculated as the
    /// average of [log2_bounds][EstimatedLog2::log2_bounds] by default.
    #[inline]
    fn log2_est(&self) -> f32 {
        let (lb, ub) = self.log2_bounds();
        (lb + ub) / 2.
    }
}

/// Compute the multiplicative inverse (aka. reciprocal) of the number.
///
/// # Examples
///
/// ```
/// # use dashu_base::Inverse;
/// assert_eq!(0.1234.inv(), 8.103727714748784);
/// assert_eq!(f32::INFINITY.inv(), 0f32);
/// ```
pub trait Inverse {
    type Output;
    fn inv(self) -> Self::Output;
}

/// Compute the square root of the number.
///
/// The result should be rounded towards zero by default.
///
/// # Examples
///
/// ```
/// # use dashu_base::SquareRoot;
/// assert_eq!(256u32.sqrt(), 16);
/// assert_eq!(257u32.sqrt(), 16);
/// ```
pub trait SquareRoot {
    type Output;

    fn sqrt(&self) -> Self::Output;
}

/// Compute the cubic root of the number.
///
/// The result should be rounded towards zero by default.
///
/// # Examples
///
/// ```
/// # use dashu_base::CubicRoot;
/// assert_eq!(216u32.cbrt(), 6);
/// assert_eq!(217u32.cbrt(), 6);
/// ```
pub trait CubicRoot {
    type Output;

    fn cbrt(&self) -> Self::Output;
}

mod inv;
mod log;
mod root;
